//! 加载界面 — 关卡/区域过渡时的全屏加载动画
//!
//! `LoadingOverlay` 资源管理非阻塞场景加载检测。
//! 显示旋转图标 + 进度条 + 随机提示，等待 GLB 场景资源真正加载完成。
//! 自定义加载背景图：将命名为 `loading` 的图片放入 `assets/images/`。

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::game_state::{GamePhase, StartGameEvent, NextLevelEvent};
use crate::entity_db::AssetLoadProgress;
use crate::image_gallery::ImageManager;
use crate::level::{GameLevel, LevelConfig, LoadLevelEvent, ZoneBank};

/// 加载覆盖层状态
#[derive(Resource)]
pub struct LoadingOverlay {
    /// 是否正在加载
    pub active: bool,
    /// 加载开始时间（秒）
    pub start_time: f32,
    /// 最短显示时间（秒）
    pub min_duration: f32,
    /// 最大等待超时（秒），超时后强制关闭
    pub max_timeout: f32,
    /// 当前加载信息
    pub message: String,
    /// 加载进度（0.0 ~ 1.0）
    pub progress: f32,
    /// 当前加载的场景 Handle（用于检测实际加载状态）
    pub scene_handle: Option<Handle<Scene>>,
    /// 场景路径（用于显示）
    pub scene_path: Option<String>,
    /// 是否已完成场景加载检测
    pub scene_loaded: bool,
}

impl Default for LoadingOverlay {
    fn default() -> Self {
        Self {
            active: false,
            start_time: 0.0,
            min_duration: 0.5,
            max_timeout: 5.0,
            message: "加载中".into(),
            progress: 0.0,
            scene_handle: None,
            scene_path: None,
            scene_loaded: false,
        }
    }
}

/// 根据 GameLevel 查找对应 Zone 的 GLB 场景路径
fn get_zone_glb_path(level: GameLevel, bank: &ZoneBank) -> Option<String> {
    let zone_id = level.zone_id();
    if zone_id.is_empty() {
        return None;
    }
    bank.zones.get(zone_id).and_then(|z| {
        z.glb_scene.as_ref().map(|glb| {
            if glb.contains('#') {
                glb.clone()
            } else {
                format!("{glb}#Scene0")
            }
        })
    })
}

/// 检测关卡过渡事件并激活加载界面，同时预加载 GLB 场景
#[allow(clippy::too_many_arguments)]
fn start_loading(
    mut events: MessageReader<LoadLevelEvent>,
    mut start_events: MessageReader<StartGameEvent>,
    mut next_events: MessageReader<NextLevelEvent>,
    mut loading: ResMut<LoadingOverlay>,
    bank: Res<ZoneBank>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    level_state: Res<State<GameLevel>>,
) {
    // 优先处理 LoadLevelEvent（携带具体 level）
    if let Some(ev) = events.read().next() {
        loading.active = true;
        loading.start_time = time.elapsed_secs();
        loading.progress = 0.0;
        loading.scene_loaded = false;
        loading.message = "正在加载区域...".into();

        if let Some(path) = get_zone_glb_path(ev.level, &bank) {
            let handle = asset_server.load::<Scene>(&path);
            loading.scene_handle = Some(handle);
            loading.scene_path = Some(path);
            loading.message = format!("加载场景 {}", ev.level.display_name());
        } else {
            loading.scene_handle = None;
            loading.scene_path = None;
            loading.min_duration = 0.8;
        }
        return;
    }

    // StartGameEvent → 默认加载 Reception
    if start_events.read().next().is_some() {
        loading.active = true;
        loading.start_time = time.elapsed_secs();
        loading.progress = 0.0;
        loading.scene_loaded = false;
        loading.message = "准备开始游戏...".into();

        if let Some(path) = get_zone_glb_path(GameLevel::Reception, &bank) {
            let handle = asset_server.load::<Scene>(&path);
            loading.scene_handle = Some(handle);
            loading.scene_path = Some(path);
        } else {
            loading.scene_handle = None;
            loading.scene_path = None;
        }
        return;
    }

    // NextLevelEvent → 需要推断下一关
    if let Some(_ev) = next_events.read().next()
        && let Some(next_level) = level_state.get().next() {
            loading.active = true;
            loading.start_time = time.elapsed_secs();
            loading.progress = 0.0;
            loading.scene_loaded = false;
            loading.message = "进入下一关...".into();

            if let Some(path) = get_zone_glb_path(next_level, &bank) {
                let handle = asset_server.load::<Scene>(&path);
                loading.scene_handle = Some(handle);
                loading.scene_path = Some(path);
            } else {
                loading.scene_handle = None;
                loading.scene_path = None;
            }
        }
}

/// 检测加载完成 — 一旦关卡状态已切换且最短时间已过即关闭加载界面，
/// GLB 场景会在后台继续加载，加载界面不阻塞游戏。
fn check_loading_complete(
    mut loading: ResMut<LoadingOverlay>,
    time: Res<Time>,
    level_state: Res<State<GameLevel>>,
    scenes: Option<Res<Assets<Scene>>>,
    mut prev_level: Local<GameLevel>,
) {
    if !loading.active {
        *prev_level = *level_state.get();
        return;
    }

    let elapsed = time.elapsed_secs() - loading.start_time;

    // ── 非阻塞式检查场景资源加载（仅用于进度显示，不阻塞关闭） ──
    if !loading.scene_loaded {
        if let Some(ref handle) = loading.scene_handle {
            if let Some(ref scenes) = scenes
                && scenes.get(handle).is_some() {
                    loading.scene_loaded = true;
                    info!("场景资源加载完成 ({:.1}s)", elapsed);
                }
        } else {
            loading.scene_loaded = true;
        }
    }

    // 进度显示：加载完成后 100%，否则按时间模拟
    loading.progress = if loading.scene_loaded {
        1.0
    } else {
        (elapsed / loading.max_timeout).min(0.9)
    };

    // ── 检测 GameLevel 已切换 ──
    let current_level = level_state.get();
    let level_changed = *current_level != *prev_level && *current_level != GameLevel::None;

    // ── 判定条件 ──
    // 关卡已切换 + 最短时间已过 → 立即完成（不等待 GLB 场景加载）
    // 或：超过最大超时 → 强制关闭
    let timeout = elapsed >= loading.max_timeout;
    let ready = level_changed && elapsed >= loading.min_duration;

    if ready || timeout {
        if timeout {
            warn!("加载超时 ({:.0}s)，强制继续", elapsed);
        } else {
            info!("加载完成 ({:.1}s)", elapsed);
        }
        loading.active = false;
        loading.progress = 1.0;
        loading.scene_handle = None;
        loading.scene_path = None;
        *prev_level = *current_level;
    }
}

/// 获取加载界面背景图（优先找名为 loading 的图片）
fn find_loading_background(manager: &ImageManager) -> Option<&crate::image_gallery::GalleryImage> {
    manager.images.iter().find(|&img| img.name.to_lowercase() == "loading").map(|v| v as _)
}

/// 绘制加载界面（关卡过渡 + 启动加载）
fn loading_ui(
    mut contexts: EguiContexts,
    loading: Res<LoadingOverlay>,
    asset_progress: Res<AssetLoadProgress>,
    manager: Res<ImageManager>,
    level: Res<LevelConfig>,
    phase: Res<State<GamePhase>>,
) {
    let is_startup = matches!(phase.get(), GamePhase::Loading);
    let is_transition = loading.active;

    if !is_startup && !is_transition {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    // ── 启动加载画面 ──
    if is_startup {
        draw_startup_loading(ctx, &asset_progress);
        return;
    }

    // ── 关卡过渡加载（原有逻辑） ──
    let phase_name = match phase.get() {
        GamePhase::MainMenu => "准备开始游戏",
        GamePhase::Playing => "加载区域",
        _ => "加载中",
    };

    let zone_name = if level.current_level != GameLevel::None {
        level.current_level.display_name()
    } else {
        ""
    };

    let bg_img = find_loading_background(&manager);

    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0x0d, 0x0d, 0x1a),
            ..Default::default()
        })
        .show(ctx, |ui| {
            let screen_rect = ctx.viewport_rect();

            // ── 背景图（如果有） ──
            if let Some(img) = bg_img {
                let bg_aspect = img.aspect_ratio();
                let screen_aspect = screen_rect.width() / screen_rect.height();
                let (w, h) = if screen_aspect > bg_aspect {
                    (screen_rect.width(), screen_rect.width() / bg_aspect)
                } else {
                    (screen_rect.height() * bg_aspect, screen_rect.height())
                };
                let x = (screen_rect.width() - w) / 2.0;
                let y = (screen_rect.height() - h) / 2.0;
                let img_rect = egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(w, h),
                );
                ui.painter().image(
                    img.texture_id(),
                    img_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_rgba_premultiplied(255, 255, 255, 180),
                );
            }

            // ── 半透明暗色覆盖层 ──
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x1a, 160),
            );

            // ── 旋转加载图标 ──
            let spinner_size = 48.0;
            let center = egui::pos2(screen_rect.center().x, screen_rect.center().y - screen_rect.height() * 0.2);
            for i in 0..8 {
                let frac = i as f32 / 8.0;
                let theta = frac * std::f32::consts::TAU + loading.start_time * 2.0;
                let (s, c) = theta.sin_cos();
                let radius = spinner_size * 0.5;
                let dot_pos = egui::pos2(center.x + c * radius, center.y + s * radius);
                let alpha = (0.3 + 0.7 * (frac + loading.start_time * 1.5).sin().abs()) as u8;
                ui.painter().circle_filled(
                    dot_pos, 4.0,
                    egui::Color32::from_rgba_premultiplied(0x00, 0xd4, 0xff, alpha),
                );
            }

            // ── 进度条 ──
            let bar_width = 240.0;
            let bar_height = 4.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.center().x - bar_width / 2.0, center.y + spinner_size),
                egui::vec2(bar_width, bar_height),
            );
            ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x4a));
            if loading.progress > 0.0 {
                let fill_rect = egui::Rect::from_min_size(
                    bar_rect.min,
                    egui::vec2(bar_width * loading.progress, bar_height),
                );
                ui.painter().rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(0x00, 0xd4, 0xff));
            }

            // ── 布局文字 ──
            egui::Area::new("loading_text".into())
                .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.add_space(screen_rect.height() * 0.35);

                    ui.label(
                        egui::RichText::new(phase_name)
                            .size(24.0)
                            .color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0))
                            .strong(),
                    );

                    if !zone_name.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(zone_name)
                                .size(14.0)
                                .color(egui::Color32::from_rgb(0x88, 0x88, 0xbb)),
                        );
                    }

                    // 显示加载详情
                    if loading.progress < 1.0 {
                        ui.add_space(4.0);
                        let detail: String = if loading.scene_handle.is_some() && !loading.scene_loaded {
                            "正在加载场景资源...".into()
                        } else if loading.scene_loaded {
                            "就绪".into()
                        } else {
                            format!("{:.0}%", loading.progress * 100.0)
                        };
                        ui.label(
                            egui::RichText::new(detail)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(0x66, 0x66, 0x99)),
                        );
                    }

                    ui.add_space(40.0);

                    // ── 小提示 ──
                    let tips = [
                        "按 WASD 移动角色",
                        "按 F 键与 NPC 对话",
                        "按 ESC 暂停游戏",
                        "收集金色光球来完成关卡",
                        "按 G 键打开图片画廊",
                    ];
                    let tip_idx = (loading.start_time as usize) % tips.len();
                    ui.label(
                        egui::RichText::new(tips[tip_idx])
                            .size(12.0)
                            .color(egui::Color32::from_rgb(0x55, 0x55, 0x77)),
                    );

                    if bg_img.is_none() {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("提示: 放入 loading.png 到 assets/images/ 可自定义加载背景")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(0x44, 0x44, 0x66)),
                        );
                    }
                });
        });
}

/// 启动加载画面 — 游戏启动时预加载 GLB 资源
fn draw_startup_loading(ctx: &egui::Context, progress: &AssetLoadProgress) {
    let screen_rect = ctx.viewport_rect();

    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0x0d, 0x0d, 0x1a),
            ..Default::default()
        })
        .show(ctx, |ui| {
            // ── 标题 ──
            let _title = egui::RichText::new("NI")
                .size(48.0)
                .color(egui::Color32::from_rgb(0x00, 0xd4, 0xff))
                .strong();
            let title_pos = egui::pos2(screen_rect.center().x, screen_rect.center().y - 80.0);
            let title_galley = ui.painter().layout_no_wrap(
                "NI".to_string(),
                egui::FontId::proportional(48.0),
                egui::Color32::from_rgb(0x00, 0xd4, 0xff),
            );
            ui.painter().galley(
                egui::pos2(title_pos.x - title_galley.size().x * 0.5, title_pos.y),
                title_galley,
                egui::Color32::from_rgb(0x00, 0xd4, 0xff),
            );

            // ── 进度条 ──
            let bar_width = 200.0;
            let bar_height = 3.0;
            let bar_x = screen_rect.center().x - bar_width / 2.0;
            let bar_y = title_pos.y + 60.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(bar_x, bar_y),
                egui::vec2(bar_width, bar_height),
            );
            ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x4a));

            let pct = if progress.total > 0 {
                progress.loaded as f32 / progress.total as f32
            } else {
                0.0
            };
            if pct > 0.0 {
                let fill_rect = egui::Rect::from_min_size(
                    bar_rect.min,
                    egui::vec2(bar_width * pct, bar_height),
                );
                ui.painter().rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(0x00, 0xd4, 0xff));
            }

            // ── 进度文字 ──
            let text = if progress.total > 0 {
                format!("加载资源  {}/{}", progress.loaded, progress.total)
            } else {
                "初始化...".to_string()
            };
            let text_galley = ui.painter().layout_no_wrap(
                text,
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(0x88, 0x88, 0xbb),
            );
            ui.painter().galley(
                egui::pos2(screen_rect.center().x - text_galley.size().x * 0.5, bar_y + 10.0),
                text_galley,
                egui::Color32::from_rgb(0x88, 0x88, 0xbb),
            );
        });
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingOverlay>()
            .add_systems(Update, (
                start_loading,
                check_loading_complete,
            ))
            .add_systems(EguiPrimaryContextPass, loading_ui);
    }
}
