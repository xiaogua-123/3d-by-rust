//! 游戏音效系统
//!
//! 管理 19 种游戏音效（`SfxAssets`），包括脚步（`FootstepState`）、
//! 拾取、伤害、UI 交互等。事件驱动播放，与音乐系统（`music.rs`）独立。

use bevy::prelude::*;
use bevy::audio::Volume;
use crate::game_state::*;
use crate::player::{Player, MoveIntent, Velocity};
use crate::td;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SfxAssets>();
        app.init_resource::<FootstepState>();
        app.add_systems(Update, (
            sfx_on_collect,
            sfx_on_damage,
            sfx_on_level_complete,
            sfx_on_game_over,
            sfx_on_turret_shoot,
            sfx_on_enemy_death,
            sfx_on_wave_start,
            sfx_on_footstep.run_if(in_state(GamePhase::Playing)),
        ));
    }
}

/// 所有游戏音效的资源句柄
#[derive(Resource)]
#[allow(dead_code)]
pub struct SfxAssets {
    pub ui_click: Handle<AudioSource>,
    pub ui_confirm: Handle<AudioSource>,
    pub ui_back: Handle<AudioSource>,
    pub ui_error: Handle<AudioSource>,
    pub ui_toggle: Handle<AudioSource>,
    pub ui_open: Handle<AudioSource>,
    pub ui_close: Handle<AudioSource>,
    pub ui_select: Handle<AudioSource>,
    pub collect: Handle<AudioSource>,
    pub damage: Handle<AudioSource>,
    pub jump: Handle<AudioSource>,
    pub game_over: Handle<AudioSource>,
    pub level_complete: Handle<AudioSource>,
    pub turret_shoot: Handle<AudioSource>,
    pub enemy_death: Handle<AudioSource>,
    pub wave_start: Handle<AudioSource>,
    pub gold_earn: Handle<AudioSource>,
    pub footstep_1: Handle<AudioSource>,
    pub footstep_2: Handle<AudioSource>,
}

impl FromWorld for SfxAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        SfxAssets {
            ui_click: asset_server.load("sounds/ui/click_001.wav"),
            ui_confirm: asset_server.load("sounds/ui/confirmation_001.wav"),
            ui_back: asset_server.load("sounds/ui/back_001.wav"),
            ui_error: asset_server.load("sounds/ui/error_001.wav"),
            ui_toggle: asset_server.load("sounds/ui/toggle_001.wav"),
            ui_open: asset_server.load("sounds/ui/open_001.wav"),
            ui_close: asset_server.load("sounds/ui/close_001.wav"),
            ui_select: asset_server.load("sounds/ui/select_001.wav"),
            collect: asset_server.load("sounds/collect.wav"),
            damage: asset_server.load("sounds/damage.wav"),
            jump: asset_server.load("sounds/jump.wav"),
            game_over: asset_server.load("sounds/game_over.wav"),
            level_complete: asset_server.load("sounds/level_complete.wav"),
            turret_shoot: asset_server.load("sounds/turret_shoot.wav"),
            enemy_death: asset_server.load("sounds/enemy_death.wav"),
            wave_start: asset_server.load("sounds/wave_start.wav"),
            gold_earn: asset_server.load("sounds/gold_earn.wav"),
            footstep_1: asset_server.load("sounds/footstep_1.wav"),
            footstep_2: asset_server.load("sounds/footstep_2.wav"),
        }
    }
}

// ─── 辅助函数：播放一次性音效 ───

fn play_sfx(commands: &mut Commands, handle: &Handle<AudioSource>, base_volume: f32, settings: &VolumeSettings) {
    let final_volume = base_volume * settings.sfx * settings.master;
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(final_volume)),
    ));
}

// ─── 游戏音效系统 ───

fn sfx_on_collect(
    mut events: MessageReader<CollectItemEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.collect, 0.5, &settings);
    }
}

fn sfx_on_damage(
    mut events: MessageReader<DamagePlayerEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.damage, 0.6, &settings);
    }
}

fn sfx_on_level_complete(
    mut events: MessageReader<LevelCompleteEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.level_complete, 0.6, &settings);
    }
}

fn sfx_on_game_over(
    mut events: MessageReader<RestartGameEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    phase: Res<State<GamePhase>>,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        if *phase.get() == GamePhase::GameOver {
            play_sfx(&mut commands, &sfx.game_over, 0.6, &settings);
        }
    }
}

// ─── 塔防音效系统 ───

fn sfx_on_turret_shoot(
    mut events: MessageReader<td::TurretShootEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.turret_shoot, 0.4, &settings);
    }
}

fn sfx_on_enemy_death(
    mut events: MessageReader<td::EnemyDeathEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.enemy_death, 0.5, &settings);
        play_sfx(&mut commands, &sfx.gold_earn, 0.3, &settings);
    }
}

fn sfx_on_wave_start(
    mut events: MessageReader<td::StartNextWaveEvent>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    settings: Res<VolumeSettings>,
) {
    for _ in events.read() {
        play_sfx(&mut commands, &sfx.wave_start, 0.5, &settings);
    }
}

// ─── 脚步声系统 ───

#[derive(Resource)]
pub struct FootstepState {
    pub timer: Timer,
    pub step_index: usize,
}

impl Default for FootstepState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.45, TimerMode::Repeating),
            step_index: 0,
        }
    }
}

/// 根据玩家移动状态播放脚步声，仅在地面且移动时触发。
fn sfx_on_footstep(
    time: Res<Time>,
    player_q: Query<(&MoveIntent, &Velocity), With<Player>>,
    sfx: Res<SfxAssets>,
    mut commands: Commands,
    mut state: ResMut<FootstepState>,
    settings: Res<VolumeSettings>,
) {
    let Ok((intent, velocity)) = player_q.single() else { return };

    let is_moving = intent.world_direction != Vec3::ZERO;
    let is_grounded = velocity.y == 0.0;

    if !is_moving || !is_grounded {
        state.timer.reset();
        return;
    }

    if state.timer.tick(time.delta()).just_finished() {
        let sound = if state.step_index.is_multiple_of(2) {
            &sfx.footstep_1
        } else {
            &sfx.footstep_2
        };
        state.step_index = state.step_index.wrapping_add(1);

        play_sfx(&mut commands, sound, 0.4, &settings);
    }
}
