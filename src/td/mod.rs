// ═══════════════════════════════════════════
// td 模块：塔防玩法
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::game_state::GamePhase;

pub mod data;
pub mod events;
pub mod balance;
pub mod level_data;
mod wave;
mod turret;
mod projectile;
mod enemy;
pub mod level;

pub use data::*;
pub use events::*;

pub struct TdPlugin;

impl Plugin for TdPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (balance::load_game_database, level_data::load_td_level))
            .init_resource::<TdGold>()
            .init_resource::<TdWaveConfig>()
            .init_resource::<TdWaveState>()
            .add_message::<PurchaseTurretEvent>()
            .add_message::<StartNextWaveEvent>()
            .add_message::<TdVictoryEvent>()
            .add_message::<TdDefeatEvent>()
            // ── 模拟层 (FixedUpdate) ──
            .add_systems(
                FixedUpdate,
                (
                    wave::td_wave_manager,
                    turret::td_handle_purchase,
                    level::td_check_victory,
                    level::td_check_defeat,
                    level::td_handle_victory,
                )
                    .chain()
                    .run_if(in_state(GamePhase::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    enemy::td_enemy_move,
                    enemy::td_enemy_attack_core,
                )
                    .chain()
                    .run_if(in_state(GamePhase::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    turret::td_turret_target,
                    turret::td_turret_fire_tick,
                )
                    .chain()
                    .run_if(in_state(GamePhase::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    projectile::td_projectile_move,
                    projectile::td_projectile_hit,
                    enemy::td_check_enemy_death,
                    level::td_check_game_end,
                )
                    .chain()
                    .run_if(in_state(GamePhase::Playing)),
            );
    }
}
