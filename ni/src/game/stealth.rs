//! 潜行/躲藏系统
//!
//! 定义 `HidingSpot`（躲藏点）和 `PlayerHiding`（玩家躲藏标记）组件。
//! 提供 F 键进出躲藏点、自动关闭手电筒、敌人靠近时发现玩家的完整流程。

use bevy::prelude::*;

use crate::ai::{AiState, EnemyBrain};
use crate::enemy::Enemy;
use crate::player::{FlashlightMode, FlashlightState, Player};

/// 躲藏点组件
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct HidingSpot {
    pub name: String,
    pub enter_radius: f32,
}

/// 玩家躲藏标记（添加到玩家实体）
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PlayerHiding;

pub struct StealthPlugin;

impl Plugin for StealthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HidingSpot>()
            .register_type::<PlayerHiding>()
            .add_systems(
                Update,
                (hiding_interaction, apply_hiding_state, enemy_detect_hiding)
                    .chain()
                    .run_if(in_state(crate::game_state::GamePhase::Playing)),
            );
    }
}

/// F 键进出躲藏点
fn hiding_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<(Entity, &Transform), With<Player>>,
    hiding_spot_q: Query<(&Transform, &HidingSpot)>,
    is_hiding_q: Query<(), With<PlayerHiding>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Ok((player_entity, player_t)) = player_q.single() else {
        return;
    };

    // 已在躲藏 → 离开
    if is_hiding_q.get(player_entity).is_ok() {
        commands.entity(player_entity).remove::<PlayerHiding>();
        return;
    }

    // 找最近的躲藏点
    for (spot_t, spot) in hiding_spot_q.iter() {
        if spot_t.translation.distance(player_t.translation) <= spot.enter_radius {
            commands.entity(player_entity).insert(PlayerHiding);
            return;
        }
    }
}

/// 躲藏时自动关闭手电筒
fn apply_hiding_state(
    is_hiding_q: Query<(), With<PlayerHiding>>,
    mut flashlight_state: ResMut<FlashlightState>,
) {
    if !is_hiding_q.is_empty() {
        flashlight_state.mode = FlashlightMode::Off;
    }
}

/// 敌人靠近躲藏点时发现玩家
fn enemy_detect_hiding(
    player_q: Query<Entity, (With<Player>, With<PlayerHiding>)>,
    enemy_q: Query<(&Transform, &EnemyBrain), With<Enemy>>,
    mut commands: Commands,
) {
    let Ok(player) = player_q.single() else {
        return;
    };

    for (enemy_t, brain) in enemy_q.iter() {
        if brain.state != AiState::Search && brain.state != AiState::Chase {
            continue;
        }

        if enemy_t.translation.distance(brain.last_known_player_pos) < 2.0 {
            commands.entity(player).remove::<PlayerHiding>();
            return;
        }
    }
}
