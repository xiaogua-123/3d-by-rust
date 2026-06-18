//! 音乐系统 — 可复用的 BGM 管理模块
//!
//! 自动扫描 `assets/music/` 注册曲目，支持播放列表管理（顺序/随机/单曲/列表循环）、
//! 渐入渐出 + 交叉淡化过渡、按类别/场景切换、独立音量控制。
//! 通过 `MusicCommand` 事件驱动（解耦方式，推荐）。

use bevy::prelude::*;
use bevy::audio::Volume;
use std::collections::HashMap;

// ─── 公开 API ───

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicManager>()
            .add_message::<MusicCommand>()
            .add_systems(Startup, register_music_tracks)
            .add_systems(First, handle_music_commands)
            .add_systems(Last, update_music_volume);
    }
}

// ─── 循环模式 ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// 不循环，播完即止
    None,
    /// 单曲循环
    One,
    /// 列表循环（默认）
    #[default]
    All,
    /// 随机播放
    Shuffle,
}

// ─── 音乐命令（跨系统解耦控制） ───

#[derive(Message)]
pub enum MusicCommand {
    /// 播放指定曲目（按注册名称）
    Play(String),
    /// 停止播放
    Stop,
    /// 暂停
    Pause,
    /// 恢复播放
    Resume,
    /// 下一首
    Next,
    /// 上一首
    Prev,
    /// 设置循环模式
    SetLoop(LoopMode),
    /// 设置音量 (0.0 ~ 1.0)
    SetVolume(f32),
    /// 清空播放列表
    ClearPlaylist,
    /// 添加到播放列表末尾
    Enqueue(String),
    /// 替换整个播放列表
    SetPlaylist(Vec<String>),
    /// 设置渐入时长（秒），0 = 立即
    SetFadeIn(f32),
    /// 设置交叉淡化时长（秒），0 = 立即切换
    SetCrossfade(f32),
}

// ─── 曲目注册信息 ───

#[derive(Clone, Debug)]
pub struct TrackInfo {
    pub name: String,
    pub path: String,
    pub category: String,
    pub handle: Handle<AudioSource>,
}

// ─── 渐出标记（内部组件） ───

#[derive(Component)]
struct FadeOut {
    start_volume: f32,
    duration: f32,
    elapsed: f32,
}

// ─── 音乐管理器（核心 Resource） ───

#[derive(Resource)]
pub struct MusicManager {
    /// 所有已注册的曲目（name → TrackInfo）
    pub tracks: HashMap<String, TrackInfo>,
    /// 当前正在播放的曲目名称
    pub current_track: Option<String>,
    /// 播放列表
    pub playlist: Vec<String>,
    /// 播放列表索引
    pub playlist_index: usize,
    /// 循环模式
    pub loop_mode: LoopMode,
    /// 独立音乐音量 (0.0 ~ 1.0)
    pub volume: f32,
    /// 是否暂停
    pub paused: bool,
    /// 渐入时长（秒），0 = 立即
    pub fade_in_duration: f32,
    /// 交叉淡化时长（秒），0 = 立即切换
    pub crossfade_duration: f32,

    // ── 内部状态 ──
    entity: Option<Entity>,
    fade_in_elapsed: f32,
}

impl Default for MusicManager {
    fn default() -> Self {
        Self {
            tracks: HashMap::new(),
            current_track: None,
            playlist: Vec::new(),
            playlist_index: 0,
            loop_mode: LoopMode::All,
            volume: 0.5,
            paused: false,
            fade_in_duration: 0.8,
            crossfade_duration: 0.0,
            entity: None,
            fade_in_elapsed: 0.0,
        }
    }
}

impl MusicManager {
    pub fn has_track(&self, name: &str) -> bool {
        self.tracks.contains_key(name)
    }

    pub fn current_track_info(&self) -> Option<&TrackInfo> {
        self.current_track.as_ref().and_then(|n| self.tracks.get(n))
    }

    pub fn tracks_by_category(&self, category: &str) -> Vec<&TrackInfo> {
        self.tracks.values().filter(|t| t.category == category).collect()
    }

    pub fn track_names(&self) -> Vec<&str> {
        self.tracks.keys().map(|s| s.as_str()).collect()
    }
}

// ═══════════════════════════════════════════
// 曲目自动注册
// ═══════════════════════════════════════════

const MUSIC_EXTENSIONS: &[&str] = &["wav", "mp3", "ogg", "flac"];

fn register_music_tracks(
    mut manager: ResMut<MusicManager>,
    asset_server: Res<AssetServer>,
) {
    let music_dir = std::path::PathBuf::from("assets/music");
    if !music_dir.exists() {
        let _ = std::fs::create_dir_all(&music_dir);
        info!("音乐系统: 已创建 assets/music/ 目录");
        return;
    }

    let mut count = 0u32;

    // 扫描子目录（按类别归类）
    if let Ok(entries) = std::fs::read_dir(&music_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let category = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                count += scan_directory(&mut manager, &path, &category, &asset_server);
            }
        }
    }

    // 扫描根目录文件（归类为 "root"）
    count += scan_directory(&mut manager, &music_dir, "root", &asset_server);

    // 初始化播放列表为全部曲目
    if manager.playlist.is_empty() && !manager.tracks.is_empty() {
        let mut names: Vec<String> = manager.tracks.keys().cloned().collect();
        names.sort();
        manager.playlist = names;
    }

    if count > 0 {
        info!("音乐系统: 已注册 {} 首曲目", count);
    }
}

fn scan_directory(
    manager: &mut MusicManager,
    dir: &std::path::Path,
    category: &str,
    asset_server: &AssetServer,
) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut count = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !MUSIC_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 注册名称: "category_name"（root 类别直接用文件名）
        let name = if category == "root" { stem } else { format!("{}_{}", category, stem) };

        if manager.tracks.contains_key(&name) {
            continue;
        }

        // 去掉 assets/ 前缀，AssetServer 路径相对于 assets 目录
        let asset_path = path.strip_prefix("assets/").unwrap_or(&path);
        let path_str = asset_path.to_string_lossy().to_string();
        let handle = asset_server.load(&path_str);

        manager.tracks.insert(name.clone(), TrackInfo {
            name,
            path: path_str,
            category: category.to_string(),
            handle,
        });

        count += 1;
    }

    count
}

// ═══════════════════════════════════════════
// 命令处理（First 阶段）
// ═══════════════════════════════════════════

fn handle_music_commands(
    mut commands: Commands,
    mut events: MessageReader<MusicCommand>,
    mut manager: ResMut<MusicManager>,
    asset_server: Res<AssetServer>,
    bgm_q: Query<Entity, With<BgmMarker>>,
    sink_q: Query<&AudioSink, With<BgmMarker>>,
) {
    for cmd in events.read() {
        match cmd {
            MusicCommand::Play(name) => {
                play_track(&mut commands, &mut manager, &asset_server, &bgm_q, name);
            }
            MusicCommand::Stop => {
                stop_music(&mut commands, &mut manager, &bgm_q);
            }
            MusicCommand::Pause => {
                manager.paused = true;
                if let Ok(sink) = sink_q.single() {
                    sink.pause();
                }
            }
            MusicCommand::Resume => {
                manager.paused = false;
                if let Ok(sink) = sink_q.single() {
                    sink.play();
                }
            }
            MusicCommand::Next => {
                next_track(&mut commands, &mut manager, &asset_server, &bgm_q);
            }
            MusicCommand::Prev => {
                prev_track(&mut commands, &mut manager, &asset_server, &bgm_q);
            }
            MusicCommand::SetLoop(mode) => manager.loop_mode = *mode,
            MusicCommand::SetVolume(vol) => {
                manager.volume = vol.clamp(0.0, 1.0);
            }
            MusicCommand::ClearPlaylist => {
                manager.playlist.clear();
                manager.playlist_index = 0;
            }
            MusicCommand::Enqueue(name) => {
                if manager.tracks.contains_key(name) && !manager.playlist.contains(name) {
                    manager.playlist.push(name.clone());
                }
            }
            MusicCommand::SetPlaylist(names) => {
                manager.playlist = names.iter()
                    .filter(|n| manager.tracks.contains_key(*n))
                    .cloned()
                    .collect();
                manager.playlist_index = 0;
            }
            MusicCommand::SetFadeIn(dur) => {
                manager.fade_in_duration = dur.max(0.0);
            }
            MusicCommand::SetCrossfade(dur) => {
                manager.crossfade_duration = dur.max(0.0);
            }
        }
    }
}

// ═══════════════════════════════════════════
// 核心播放控制
// ═══════════════════════════════════════════

/// 背景音乐标记组件
#[derive(Component)]
pub struct BgmMarker;

fn play_track(
    commands: &mut Commands,
    manager: &mut MusicManager,
    _asset_server: &AssetServer,
    bgm_q: &Query<Entity, With<BgmMarker>>,
    name: &str,
) {
    let Some(track) = manager.tracks.get(name) else {
        warn!("音乐系统: 未找到曲目 '{}'", name);
        return;
    };

    if manager.current_track.as_deref() == Some(name) && !manager.paused {
        return;
    }

    info!("音乐系统: 播放 '{}' [{}]", name, track.category);

    // 淡出旧曲目
    if let Ok(entity) = bgm_q.single() {
        if manager.crossfade_duration > 0.0 {
            commands.entity(entity).insert(FadeOut {
                start_volume: manager.volume,
                duration: manager.crossfade_duration,
                elapsed: 0.0,
            });
        } else {
            commands.entity(entity).despawn();
        }
    }

    // 启动新曲目（带渐入）
    let start_vol = if manager.fade_in_duration > 0.0 { 0.0 } else { manager.volume };

    let entity = commands.spawn((
        AudioPlayer::new(track.handle.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(start_vol)),
        BgmMarker,
        Name::new(format!("BGM_{}", name)),
    )).id();

    manager.current_track = Some(name.to_string());
    manager.entity = Some(entity);
    manager.paused = false;
    manager.fade_in_elapsed = 0.0;
}

fn stop_music(
    commands: &mut Commands,
    manager: &mut MusicManager,
    bgm_q: &Query<Entity, With<BgmMarker>>,
) {
    if let Ok(entity) = bgm_q.single() {
        commands.entity(entity).despawn();
    }
    manager.current_track = None;
    manager.entity = None;
    manager.paused = false;
}

fn next_track(
    commands: &mut Commands,
    manager: &mut MusicManager,
    asset_server: &AssetServer,
    bgm_q: &Query<Entity, With<BgmMarker>>,
) {
    if manager.playlist.is_empty() { return; }

    let next_idx = match manager.loop_mode {
        LoopMode::Shuffle => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen_range(0..manager.playlist.len())
        }
        LoopMode::One | LoopMode::All => {
            (manager.playlist_index + 1) % manager.playlist.len()
        }
        LoopMode::None => {
            if manager.playlist_index + 1 < manager.playlist.len() {
                manager.playlist_index + 1
            } else {
                return;
            }
        }
    };

    manager.playlist_index = next_idx;
    let name = manager.playlist[next_idx].clone();
    play_track(commands, manager, asset_server, bgm_q, &name);
}

fn prev_track(
    commands: &mut Commands,
    manager: &mut MusicManager,
    asset_server: &AssetServer,
    bgm_q: &Query<Entity, With<BgmMarker>>,
) {
    if manager.playlist.is_empty() { return; }

    let prev_idx = if manager.playlist_index == 0 {
        manager.playlist.len() - 1
    } else {
        manager.playlist_index - 1
    };

    manager.playlist_index = prev_idx;
    let name = manager.playlist[prev_idx].clone();
    play_track(commands, manager, asset_server, bgm_q, &name);
}

// ═══════════════════════════════════════════
// 音量更新 + 渐入处理（Last 阶段）
// ═══════════════════════════════════════════

#[allow(clippy::type_complexity)]
fn update_music_volume(
    time: Res<Time>,
    mut manager: ResMut<MusicManager>,
    mut fadeout_q: Query<(Entity, &mut FadeOut, &mut PlaybackSettings)>,
    mut bgm_q: Query<(Entity, &mut PlaybackSettings), (With<BgmMarker>, Without<FadeOut>)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();

    // 处理淡出
    let mut to_despawn = Vec::new();
    for (entity, mut fade, mut settings) in fadeout_q.iter_mut() {
        fade.elapsed += dt;
        let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
        settings.volume = Volume::Linear(fade.start_volume * (1.0 - t));
        if t >= 1.0 {
            to_despawn.push(entity);
        }
    }
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // 处理淡入 + 音量同步
    if let Ok((_entity, mut settings)) = bgm_q.single_mut() {
        if manager.fade_in_duration > 0.0 && manager.fade_in_elapsed < manager.fade_in_duration {
            manager.fade_in_elapsed += dt;
            let t = (manager.fade_in_elapsed / manager.fade_in_duration).min(1.0);
            let eased = 1.0 - (1.0 - t).powi(3); // ease-out cubic
            settings.volume = Volume::Linear(manager.volume * eased);
        } else {
            settings.volume = Volume::Linear(manager.volume);
        }
    }
}

// ═══════════════════════════════════════════
// 便捷辅助函数
// ═══════════════════════════════════════════

/// 创建区域切换时自动播放对应音乐的 system
pub fn play_zone_music(track_name: &'static str) -> impl Fn(MessageWriter<MusicCommand>) {
    move |mut cmd: MessageWriter<MusicCommand>| {
        cmd.write(MusicCommand::Play(track_name.to_string()));
    }
}

/// 创建菜单场景自动播放音乐的 system
pub fn play_menu_music(track_name: &'static str) -> impl Fn(MessageWriter<MusicCommand>) {
    move |mut cmd: MessageWriter<MusicCommand>| {
        cmd.write(MusicCommand::Play(track_name.to_string()));
    }
}
