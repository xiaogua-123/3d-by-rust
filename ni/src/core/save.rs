//! 存档系统 — JSON 存档保存/加载
//!
//! 支持 5 个槽位（槽位 0 为自动存档）。
//! 保存数据包括玩家位置、背包、分数、生命、收集品进度、任务、音量、手电筒模式。
//!
//! # 流程
//! - `SaveGameEvent` → 收集所有游戏状态 → 写入 `saves/save_{slot}.json`
//! - `LoadGameEvent` → 读取 JSON → `PendingLoadData` → 切换关卡 → 下一帧应用数据

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::game_state::{GamePhase, LevelCollectibles, PlayerHealth, Score, VolumeSettings};
use crate::inventory::Inventory;
use crate::level::{GameLevel, LevelConfig};
use crate::dialogue::QuestTracker;
use crate::player::{FlashlightMode, FlashlightState};

// ── 常量 ──

/// 存档槽位数量
const SAVE_SLOTS: usize = 5;
/// 自动存档槽位
const AUTO_SAVE_SLOT: usize = 0;
/// 存档文件名格式
const SAVE_FILE_PREFIX: &str = "save_";
const SAVE_FILE_SUFFIX: &str = ".json";

// ── 数据结构 ──

/// 存档数据（序列化为 JSON）
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    /// 存档名称
    pub save_name: String,
    /// 保存时间戳
    pub timestamp: f64,
    /// 存档版本号
    pub version: u32,
    /// 当前关卡 zone_id
    pub level: String,
    /// 玩家位置
    pub player_x: f32,
    pub player_y: f32,
    pub player_z: f32,
    /// 玩家旋转（四元数）
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    pub rot_w: f32,
    /// 背包物品 (item_id → 数量)
    pub inventory: HashMap<String, u32>,
    /// 分数
    pub score: u32,
    /// 生命值
    pub health_current: u32,
    pub health_max: u32,
    /// 收集品进度
    pub collectibles_total: u32,
    pub collectibles_collected: u32,
    /// 活跃任务
    pub active_quests: Vec<String>,
    /// 已完成任务
    pub completed_quests: Vec<String>,
    /// 游戏标记
    pub flags: Vec<String>,
    /// 音量设置
    pub volume_master: f32,
    pub volume_music: f32,
    pub volume_sfx: f32,
    /// 手电筒模式
    pub flashlight_mode: String,
}

impl SaveData {
    /// 从当前游戏状态收集数据
    #[allow(clippy::too_many_arguments)]
    fn collect(
        save_name: String,
        level: &str,
        player_t: &Transform,
        inventory: &Inventory,
        score: u32,
        health: &PlayerHealth,
        collectibles: &LevelCollectibles,
        quest_tracker: &QuestTracker,
        volume: &VolumeSettings,
        flashlight: &FlashlightMode,
    ) -> Self {
        Self {
            save_name,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            version: 1,
            level: level.to_string(),
            player_x: player_t.translation.x,
            player_y: player_t.translation.y,
            player_z: player_t.translation.z,
            rot_x: player_t.rotation.x,
            rot_y: player_t.rotation.y,
            rot_z: player_t.rotation.z,
            rot_w: player_t.rotation.w,
            inventory: inventory.items.clone(),
            score,
            health_current: health.current,
            health_max: health.max,
            collectibles_total: collectibles.total,
            collectibles_collected: collectibles.collected,
            active_quests: quest_tracker.active_quests.clone(),
            completed_quests: quest_tracker.completed_quests.clone(),
            flags: quest_tracker.flags.clone(),
            volume_master: volume.master,
            volume_music: volume.music,
            volume_sfx: volume.sfx,
            flashlight_mode: format!("{:?}", flashlight),
        }
    }

    /// 将存档数据应用到游戏状态
    fn apply(&self, inventory: &mut Inventory, quest_tracker: &mut QuestTracker) {
        // 背包
        inventory.items.clone_from(&self.inventory);

        // 任务
        quest_tracker.active_quests.clone_from(&self.active_quests);
        quest_tracker.completed_quests.clone_from(&self.completed_quests);
        quest_tracker.flags.clone_from(&self.flags);
    }

    /// 解析手电筒模式
    fn parse_flashlight_mode(&self) -> FlashlightMode {
        match self.flashlight_mode.as_str() {
            "UV" => FlashlightMode::UV,
            "Off" => FlashlightMode::Off,
            _ => FlashlightMode::Normal,
        }
    }
}

// ── 资源 ──

/// 待应用的存档数据（加载流程中间态）
///
/// 在 `handle_load_game` 中创建，下一帧由 `apply_pending_load_data` 消费。
#[derive(Resource, Clone)]
pub struct PendingLoadData {
    pub data: SaveData,
}

// ── 事件 ──

/// 保存游戏事件
#[derive(Message)]
pub struct SaveGameEvent {
    pub slot: usize,
    pub save_name: String,
}

/// 加载游戏事件
#[derive(Message)]
pub struct LoadGameEvent {
    pub slot: usize,
}

// ── 插件 ──

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveGameEvent>()
            .add_message::<LoadGameEvent>()
            .add_systems(Update, (
                handle_save_game,
                handle_load_game,
                apply_pending_load_data,
                auto_save_on_level_complete,
            ));
    }
}

// ── 保存系统 ──

#[allow(clippy::too_many_arguments)]
fn handle_save_game(
    mut events: MessageReader<SaveGameEvent>,
    player_q: Query<&Transform, With<crate::player::Player>>,
    inventory: Res<Inventory>,
    score: Res<Score>,
    health: Res<PlayerHealth>,
    collectibles: Res<LevelCollectibles>,
    quest_tracker: Res<QuestTracker>,
    volume: Res<VolumeSettings>,
    flashlight: Res<FlashlightState>,
    level_config: Res<LevelConfig>,
) {
    let Ok(player_t) = player_q.single() else {
        warn!("存档失败：找不到玩家实体");
        return;
    };

    for event in events.read() {
        let slot = event.slot.min(SAVE_SLOTS - 1);
        let path = save_path(slot);

        // 确保 saves 目录存在
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let data = SaveData::collect(
            event.save_name.clone(),
            level_config.current_level.zone_id(),
            player_t,
            &inventory,
            score.0,
            &health,
            &collectibles,
            &quest_tracker,
            &volume,
            &flashlight.mode,
        );

        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                match fs::write(&path, &json) {
                    Ok(()) => info!("存档已保存到: {:?} (槽位 {})", path, slot),
                    Err(e) => error!("写入存档文件失败: {}", e),
                }
            }
            Err(e) => error!("序列化存档数据失败: {}", e),
        }
    }
}

// ── 加载系统 ──

fn handle_load_game(
    mut events: MessageReader<LoadGameEvent>,
    mut commands: Commands,
    mut phase: ResMut<NextState<GamePhase>>,
    mut level_state: ResMut<NextState<GameLevel>>,
) {
    for event in events.read() {
        let slot = event.slot.min(SAVE_SLOTS - 1);
        let path = save_path(slot);

        let json = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                error!("读取存档文件失败 (槽位 {}): {}", slot, e);
                continue;
            }
        };

        let data: SaveData = match serde_json::from_str(&json) {
            Ok(d) => d,
            Err(e) => {
                error!("解析存档数据失败: {}", e);
                continue;
            }
        };

        info!("正在加载存档: {} (槽位 {})", data.save_name, slot);

        // 转换为 GameLevel
        let target_level = GameLevel::from_zone_id(&data.level)
            .unwrap_or(GameLevel::Demo);

        // 保存待应用数据
        commands.insert_resource(PendingLoadData { data });

        // 切换关卡（触发 cleanup → spawn）
        level_state.set(target_level);
        phase.set(GamePhase::Playing);
    }
}

/// 应用待加载的存档数据（在关卡生成后的下一帧）
#[allow(clippy::too_many_arguments)]
fn apply_pending_load_data(
    mut commands: Commands,
    pending: Option<ResMut<PendingLoadData>>,
    mut inventory: ResMut<Inventory>,
    mut quest_tracker: ResMut<QuestTracker>,
    mut score: ResMut<Score>,
    mut health: ResMut<PlayerHealth>,
    mut collectibles: ResMut<LevelCollectibles>,
    mut volume: ResMut<VolumeSettings>,
    mut flashlight: ResMut<FlashlightState>,
    mut player_q: Query<&mut Transform, With<crate::player::Player>>,
    level_config: Res<LevelConfig>,
) {
    let Some(pending) = pending else { return };
    // 等待玩家实体生成（关卡 OnEnter 系统完成后）
    let Ok(mut transform) = player_q.single_mut() else { return };

    let data = &pending.data;

    // 应用背包和任务数据
    data.apply(&mut inventory, &mut quest_tracker);

    // 分数
    score.0 = data.score;

    // 生命
    health.current = data.health_current;
    health.max = data.health_max;

    // 收集品进度（只恢复数据，不修改实际场景中的收集品实体）
    collectibles.total = data.collectibles_total;
    collectibles.collected = data.collectibles_collected;

    // 音量
    volume.master = data.volume_master;
    volume.music = data.volume_music;
    volume.sfx = data.volume_sfx;

    // 手电筒
    flashlight.mode = data.parse_flashlight_mode();

    // 重置玩家位置和旋转
    transform.translation = Vec3::new(data.player_x, data.player_y, data.player_z);
    transform.rotation = Quat::from_xyzw(data.rot_x, data.rot_y, data.rot_z, data.rot_w);

    info!("存档数据已应用到游戏（关卡: {}）", level_config.current_level.zone_id());

    // 清理待应用数据
    commands.remove_resource::<PendingLoadData>();
}

/// 关卡完成时自动存档
fn auto_save_on_level_complete(
    mut phase_listener: MessageReader<crate::game_state::LevelCompleteEvent>,
    mut save_writer: MessageWriter<SaveGameEvent>,
) {
    for _ in phase_listener.read() {
        save_writer.write(SaveGameEvent {
            slot: AUTO_SAVE_SLOT,
            save_name: "自动存档".to_string(),
        });
    }
}

// ── 辅助函数 ──

/// 获取存档文件路径
fn save_path(slot: usize) -> PathBuf {
    let mut path = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&p).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("saves");
    path.push(format!("{}{}{}", SAVE_FILE_PREFIX, slot, SAVE_FILE_SUFFIX));
    path
}
