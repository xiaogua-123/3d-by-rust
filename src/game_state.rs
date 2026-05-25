use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GamePhase {
    #[default]
    MainMenu,
    Playing,
    Paused,
    Dialoguing,
    GameOver,
    LevelComplete,
    MultiplayerChat,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct Score(pub u32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerHealth {
    pub current: u32,
    pub max: u32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self { current: 3, max: 3 }
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct LevelCollectibles {
    pub total: u32,
    pub collected: u32,
}

// --- Game flow events ---
#[derive(Message)]
pub struct StartGameEvent;     // MainMenu → Playing

#[derive(Message)]
pub struct RestartGameEvent;  // GameOver → Playing

#[derive(Message)]
pub struct NextLevelEvent;    // LevelComplete → next level

#[derive(Message)]
pub struct MainMenuEvent;     // any → MainMenu

// --- In-game events ---
#[derive(Message)]
pub struct CollectItemEvent;

#[derive(Message)]
pub struct DamagePlayerEvent(pub u32);

#[derive(Message)]
pub struct LevelCompleteEvent;

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Score>()
            .register_type::<PlayerHealth>()
            .register_type::<LevelCollectibles>()
            .init_state::<GamePhase>()
            .init_resource::<Score>()
            .init_resource::<PlayerHealth>()
            .init_resource::<LevelCollectibles>()
            .add_message::<StartGameEvent>()
            .add_message::<RestartGameEvent>()
            .add_message::<NextLevelEvent>()
            .add_message::<MainMenuEvent>()
            .add_message::<CollectItemEvent>()
            .add_message::<DamagePlayerEvent>()
            .add_message::<LevelCompleteEvent>()
            .add_systems(Update, (
                handle_collect_item.run_if(in_state(GamePhase::Playing)),
                handle_damage_player.run_if(in_state(GamePhase::Playing)),
                handle_level_complete.run_if(in_state(GamePhase::Playing)),
                check_player_death.run_if(in_state(GamePhase::Playing)),
                pause_toggle,
                handle_start_game,
                handle_restart_game,
                handle_next_level,
                handle_main_menu,
            ));
    }
}

fn handle_collect_item(
    mut events: MessageReader<CollectItemEvent>,
    mut collectibles: ResMut<LevelCollectibles>,
    mut score: ResMut<Score>,
) {
    for _ in events.read() {
        collectibles.collected += 1;
        score.0 += 100;
        debug!("收集物品! ({}/{})", collectibles.collected, collectibles.total);
    }
}

fn handle_damage_player(
    mut events: MessageReader<DamagePlayerEvent>,
    mut health: ResMut<PlayerHealth>,
) {
    for ev in events.read() {
        health.current = health.current.saturating_sub(ev.0);
        debug!("受到伤害! 生命值: {}/{}", health.current, health.max);
    }
}

fn handle_level_complete(
    mut events: MessageReader<LevelCompleteEvent>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        info!("关卡完成!");
        next_state.set(GamePhase::LevelComplete);
    }
}

fn check_player_death(
    health: Res<PlayerHealth>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if health.current == 0 {
        info!("游戏结束!");
        next_state.set(GamePhase::GameOver);
    }
}

fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match phase.get() {
            GamePhase::Playing => next_state.set(GamePhase::Paused),
            GamePhase::Paused => next_state.set(GamePhase::Playing),
            GamePhase::Dialoguing => next_state.set(GamePhase::Playing),
            _ => {}
        }
    }
}

// --- Flow event handlers ---

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

fn handle_restart_game(
    mut events: MessageReader<RestartGameEvent>,
    mut start_writer: MessageWriter<StartGameEvent>,
) {
    for _ in events.read() {
        // Forward to start_game handler (single source of truth)
        start_writer.write(StartGameEvent);
    }
}

fn handle_next_level(
    mut events: MessageReader<NextLevelEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        phase.set(GamePhase::Playing);
    }
}

fn handle_main_menu(
    mut events: MessageReader<MainMenuEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        phase.set(GamePhase::MainMenu);
    }
}
