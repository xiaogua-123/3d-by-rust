//! 塔防敌人系统：寻路移动、攻击核心、死亡检测
//!
//! 沿预计算路径移动、每帧同步到空间网格、到达核心时发起攻击、
//! 血量归零时死亡并发放金币奖励。

use bevy::prelude::*;
use crate::combat::{AttackDamage, Health, MoveSpeed};
use crate::pathfinding::NavPath;
use super::data::*;
use super::events::EnemyDeathEvent;
use super::spatial::entry::EntityCategory;
use super::spatial::integration::{TdGridObject, TdGridResource};

#[derive(Component)]
pub struct EnemyAttackTimer(pub Timer);

/// 记录敌人在空间网格中的最后位置，用于帧间增量更新
#[derive(Component)]
pub struct TdGridPos(pub Vec2);

/// 沿预计算的 NavPath 寻路移动，不触发重新寻路
pub fn td_enemy_pathfollow(
    time: Res<Time>,
    mut enemy_q: Query<(&mut Transform, &mut NavPath, &MoveSpeed)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut path, speed) in enemy_q.iter_mut() {
        if path.completed || path.waypoints.is_empty() {
            continue;
        }
        let target = path.waypoints[path.index];
        let dir = target - transform.translation;
        let dist = dir.length();
        if dist <= 0.5 || !dist.is_finite() {
            path.advance();
            continue;
        }
        let move_dir = dir / dist;
        transform.translation += move_dir * speed.0 * dt;
    }
}

/// 每帧将敌人位置同步到空间网格
pub fn td_enemy_grid_sync(
    mut enemy_q: Query<(Entity, &Transform, &TdGridObject, &mut TdGridPos)>,
    mut grid: ResMut<TdGridResource>,
) {
    for (entity, transform, grid_obj, mut grid_pos) in enemy_q.iter_mut() {
        let new_pos = Vec2::new(transform.translation.x, transform.translation.z);
        if grid_pos.0 != new_pos {
            grid.update_entity(
                entity,
                EntityCategory::Monster,
                grid_pos.0,
                new_pos,
                grid_obj.radius,
            );
            grid_pos.0 = new_pos;
        }
    }
}

pub fn td_enemy_attack_core(
    time: Res<Time>,
    mut commands: Commands,
    mut core_q: Query<&mut DefenseCore>,
    core_t_q: Query<&Transform, (With<DefenseCore>, Without<TdEnemy>)>,
    mut enemy_q: Query<
        (Entity, &Transform, &AttackDamage, Option<&mut EnemyAttackTimer>),
        Without<DefenseCore>,
    >,
) {
    let Ok(core_t) = core_t_q.single() else {
        return;
    };
    let core_pos = core_t.translation;

    let mut total_damage = 0.0;

    for (enemy_entity, enemy_t, damage, attack_timer) in enemy_q.iter_mut() {
        let dist = enemy_t.translation.distance(core_pos);

        if dist < 0.8 {
            if let Some(mut timer) = attack_timer {
                timer.0.tick(time.delta());
                if timer.0.is_finished() {
                    total_damage += damage.0;
                    timer.0.reset();
                }
            } else {
                commands.entity(enemy_entity).insert(EnemyAttackTimer(
                    Timer::from_seconds(1.0, TimerMode::Repeating),
                ));
            }
        } else if attack_timer.is_some() {
            commands.entity(enemy_entity).remove::<EnemyAttackTimer>();
        }
    }

    if total_damage > 0.0
        && let Ok(mut core) = core_q.single_mut() {
            core.current_health -= total_damage;
            if core.current_health <= 0.0 {
                core.current_health = 0.0;
            }
        }
}

pub fn td_check_enemy_death(
    mut commands: Commands,
    enemy_q: Query<(Entity, &TdEnemy, &Health)>,
    mut gold: ResMut<TdGold>,
    mut state: ResMut<TdWaveState>,
    mut grid: ResMut<TdGridResource>,
    mut death_writer: MessageWriter<EnemyDeathEvent>,
) {
    for (entity, enemy, health) in enemy_q.iter() {
        if health.current <= 0.0 {
            gold.0 += enemy.gold_reward;
            death_writer.write(EnemyDeathEvent {
                gold_reward: enemy.gold_reward as f32,
            });
            state.enemies_alive = state.enemies_alive.saturating_sub(1);
            grid.remove_entity(&entity);
            commands.entity(entity).despawn();
            info!(
                "击杀敌人! +{} 金币 (剩余金币: {}, 剩余敌人: {})",
                enemy.gold_reward, gold.0, state.enemies_alive
            );
        }
    }
}
