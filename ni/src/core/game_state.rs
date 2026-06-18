//! 游戏状态管理
//!
//! 定义 `GamePhase` 状态机（主菜单/游戏中/暂停/对话/游戏结束等）、
//! `GameLevel` 关卡状态、游戏资源（分数/血量/收藏品/音量）和事件系统。
//! 处理游戏开始、重新开始、返回菜单等生命周期事件。

use bevy::prelude::*;

// ─────────────────────────── 游戏阶段 ───────────────────────────
/// 定义游戏当前所处的逻辑阶段，用于控制不同系统集合的运行。
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GamePhase {
    /// 加载资源中
    #[default]
    Loading,
    /// 主菜单
    MainMenu,
    /// 正常游玩
    Playing,
    /// 暂停
    Paused,
    /// 对话中
    Dialoguing,
    /// 游戏结束
    GameOver,
    /// 关卡完成
    LevelComplete,
    /// 多人聊天
    MultiplayerChat,
    /// 创造模式
    Creative,
}

// ─────────────────────────── 游戏资源 ───────────────────────────
/// 当前得分
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct Score(pub u32);

/// 玩家生命值
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerHealth {
    pub current: u32, // 当前生命
    pub max: u32,     // 最大生命
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self { current: 3, max: 3 }
    }
}

/// 关卡收集品计数
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct LevelCollectibles {
    pub total: u32,     // 本关收集品总数
    pub collected: u32, // 已收集数量
}

/// 标记设置页面是从主菜单进入的（用于暂停菜单导航）
#[derive(Resource, Default)]
pub struct SettingsFromMainMenu(pub bool);

/// 覆盖层菜单状态 — 在所有 GamePhase 之上浮动渲染
#[derive(Resource, Default)]
pub struct OverlayState {
    pub active: Option<OverlayType>,
}

/// 覆盖层类型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OverlayType {
    /// ESC 菜单（暂停、设置、返回主菜单）
    InGameMenu,
}

/// 音量设置
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct VolumeSettings {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            master: 0.8,
            music: 0.5,
            sfx: 0.7,
        }
    }
}

// ─────────────────────────── 游戏流程事件 ───────────────────────────
/// 开始游戏事件（MainMenu → Playing）
#[derive(Message)]
pub struct StartGameEvent;

/// 重新开始事件（GameOver → Playing）
#[derive(Message)]
pub struct RestartGameEvent;

/// 下一关事件（LevelComplete → next level）
#[derive(Message)]
pub struct NextLevelEvent;

/// 返回主菜单事件（any → MainMenu）
#[derive(Message)]
pub struct MainMenuEvent;

// ─────────────────────────── 游戏内事件 ───────────────────────────
/// 收集物品事件
#[derive(Message)]
pub struct CollectItemEvent;

/// 玩家受伤事件（携带伤害值）
#[derive(Message)]
pub struct DamagePlayerEvent(pub u32);

/// 关卡完成事件
#[derive(Message)]
pub struct LevelCompleteEvent;

// ─────────────────────────── 游戏状态插件 ───────────────────────────
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app
            // 注册资源类型以支持反射（便于 bevy-inspector-egui 查看/修改）
            .register_type::<Score>()
            .register_type::<PlayerHealth>()
            .register_type::<LevelCollectibles>()
            .register_type::<VolumeSettings>()
            // 初始化游戏阶段状态机
            .init_state::<GamePhase>()
            // 初始化资源
            .init_resource::<Score>()
            .init_resource::<PlayerHealth>()
            .init_resource::<LevelCollectibles>()
            .init_resource::<VolumeSettings>()
            .init_resource::<SettingsFromMainMenu>()
            .init_resource::<OverlayState>()
            // 注册消息事件
            .add_message::<StartGameEvent>()
            .add_message::<RestartGameEvent>()
            .add_message::<NextLevelEvent>()
            .add_message::<MainMenuEvent>()
            .add_message::<CollectItemEvent>()
            .add_message::<DamagePlayerEvent>()
            .add_message::<LevelCompleteEvent>()
            // 添加系统（部分仅在 Playing 阶段运行）
            .add_systems(
                Update,
                (
                    handle_collect_item.run_if(in_state(GamePhase::Playing)),
                    handle_damage_player.run_if(in_state(GamePhase::Playing)),
                    handle_level_complete.run_if(in_state(GamePhase::Playing)),
                    check_player_death.run_if(in_state(GamePhase::Playing)),
                    pause_toggle,
                    handle_start_game,
                    handle_restart_game,
                    handle_next_level,
                    handle_main_menu,
                ),
            );
    }
}

// ─────────────────────────── 游戏内系统 ───────────────────────────

/// 处理物品收集：增加收集计数并增加分数。
fn handle_collect_item(
    mut events: MessageReader<CollectItemEvent>,
    mut collectibles: ResMut<LevelCollectibles>,
    mut score: ResMut<Score>,
) {
    for _ in events.read() {
        collectibles.collected += 1;
        score.0 += 100;
        debug!(
            "收集物品! ({}/{})",
            collectibles.collected, collectibles.total
        );
    }
}

/// 处理玩家受伤：减少当前生命值。
fn handle_damage_player(
    mut events: MessageReader<DamagePlayerEvent>,
    mut health: ResMut<PlayerHealth>,
) {
    for ev in events.read() {
        // 使用 saturating_sub 防止下溢
        health.current = health.current.saturating_sub(ev.0);
        debug!("受到伤害! 生命值: {}/{}", health.current, health.max);
    }
}

/// 关卡完成：切换到 LevelComplete 阶段。
fn handle_level_complete(
    mut events: MessageReader<LevelCompleteEvent>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        info!("关卡完成!");
        next_state.set(GamePhase::LevelComplete);
    }
}

/// 每帧检查玩家是否死亡（生命值为零），若死亡则进入 GameOver 阶段。
fn check_player_death(health: Res<PlayerHealth>, mut next_state: ResMut<NextState<GamePhase>>) {
    if health.current == 0 {
        info!("游戏结束!");
        next_state.set(GamePhase::GameOver);
    }
}

/// 暂停切换：按下 Esc 切换覆盖层菜单（Playing/Dialoguing）或恢复游戏（Paused）。
pub(crate) fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut overlay: ResMut<OverlayState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match phase.get() {
            GamePhase::Playing | GamePhase::Dialoguing => {
                next_state.set(GamePhase::Paused);
                overlay.active = Some(OverlayType::InGameMenu);
            }
            GamePhase::Paused => {
                overlay.active = None;
                next_state.set(GamePhase::Playing);
            }
            GamePhase::Creative => {
                next_state.set(GamePhase::Playing);
            }
            _ => {}
        }
    }
}

// ─────────────────────────── 流程事件处理 ───────────────────────────

/// 开始游戏：重置生命、分数、收集品，然后进入 Playing 阶段。
fn handle_start_game(
    mut events: MessageReader<StartGameEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
    mut health: ResMut<PlayerHealth>,
    mut score: ResMut<Score>,
    mut collectibles: ResMut<LevelCollectibles>,
) {
    for _ in events.read() {
        health.current = health.max;
        score.0 = 0;
        collectibles.total = 0;
        collectibles.collected = 0;
        phase.set(GamePhase::Playing);
    }
}

/// 重新开始：转发为 StartGameEvent，复用重置逻辑。
fn handle_restart_game(
    mut events: MessageReader<RestartGameEvent>,
    mut start_writer: MessageWriter<StartGameEvent>,
) {
    for _ in events.read() {
        // 转发给开始游戏处理器，保持单一重置逻辑
        start_writer.write(StartGameEvent);
    }
}

/// 下一关：直接进入 Playing 阶段（关卡加载由其他系统负责）。
fn handle_next_level(
    mut events: MessageReader<NextLevelEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        phase.set(GamePhase::Playing);
    }
}

/// 返回主菜单：切换到 MainMenu 阶段。
fn handle_main_menu(
    mut events: MessageReader<MainMenuEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        phase.set(GamePhase::MainMenu);
    }
}
