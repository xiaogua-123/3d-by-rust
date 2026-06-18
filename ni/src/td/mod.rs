//! 塔防玩法（Tower Defense）子游戏模块
//!
//! 完整的塔防模式实现：炮塔建造/升级、敌人波次生成、寻路导航网格、
//! 空间网格碰撞检测、经济系统（金币）和游戏结束判定。
//! 作为 `GamePhase` 的子状态运行。

use bevy::prelude::*;
use crate::game_state::GamePhase;
use self::spatial::integration::TdGridPlugin;

pub mod data;
pub mod events;
pub mod balance;
pub mod level_data;
pub mod spatial;
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
        app.add_plugins(TdGridPlugin)
            .add_systems(Startup, (
                balance::load_game_database,
                balance::load_td_config,
                balance::sync_wave_config,
                level_data::load_td_level,
                level::td_build_navmesh,
            ).chain())
            .init_resource::<TdGold>()
            .init_resource::<TdWaveConfig>()
            .init_resource::<TdWaveState>()
            .add_message::<PurchaseTurretEvent>()
            .add_message::<StartNextWaveEvent>()
            .add_message::<TdVictoryEvent>()
            .add_message::<TdDefeatEvent>()
            .add_message::<TurretShootEvent>()
            .add_message::<EnemyDeathEvent>()
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
                    enemy::td_enemy_pathfollow,
                    enemy::td_enemy_grid_sync,
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
