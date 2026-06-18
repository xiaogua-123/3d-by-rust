//! 敌人系统（关卡敌人）
//!
//! 定义 `Enemy` 组件标记，提供敌人对玩家的碰撞伤害检测。
//! 敌人 AI 状态机在 `ai.rs` 中实现，巡逻寻路使用 `pathfinding` 模块。
//!
//! # 模型替换计划
//!
//! 程序化方块 → GLB 模型：
//! - 关卡敌人 → `models/characters/enemy_{type}.glb`
//! - 分类：slime, skeleton, ghost, golem
//! - 塔防敌人 → `models/td/enemy_{basic,fast,tank}.glb`
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::colliders::CollisionEvent;
use crate::combat::AttackDamage;
use crate::game_state::{DamagePlayerEvent, GamePhase};
use crate::player::Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Enemy {
    pub patrol_points: Vec<Vec3>,
    pub attack_cooldown: Timer,
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Enemy>()
            .add_systems(FixedUpdate, enemy_damage_player.run_if(in_state(GamePhase::Playing)));
    }
}

fn enemy_damage_player(
    time: Res<Time>,
    mut enemy_q: Query<(&mut Enemy, &AttackDamage)>,
    player_q: Query<Entity, With<Player>>,
    mut collision_events: MessageReader<CollisionEvent>,
    mut damage_writer: MessageWriter<DamagePlayerEvent>,
) {
    let Ok(player_entity) = player_q.single() else { return };

    // 处理碰撞事件
    for event in collision_events.read() {
        // 检查是否是玩家与敌人的碰撞
        let (enemy_entity, _) = if event.entity_a == player_entity {
            (event.entity_b, event.entity_a)
        } else if event.entity_b == player_entity {
            (event.entity_a, event.entity_b)
        } else {
            continue;
        };

        // 获取敌人组件
        if let Ok((mut enemy, damage)) = enemy_q.get_mut(enemy_entity) {
            enemy.attack_cooldown.tick(time.delta());
            if !enemy.attack_cooldown.is_finished() {
                continue;
            }

            // 造成伤害
            damage_writer.write(DamagePlayerEvent(damage.0 as u32));
            enemy.attack_cooldown.reset();
        }
    }

    // 更新所有敌人的冷却时间
    for (mut enemy, _) in enemy_q.iter_mut() {
        enemy.attack_cooldown.tick(time.delta());
    }
}
