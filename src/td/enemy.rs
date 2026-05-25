// ═══════════════════════════════════════════
// TD 敌人系统：移动、攻击核心、死亡检测
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::combat::{self, AttackDamage, Health, MoveSpeed};
use super::data::*;

#[derive(Component)]
pub struct EnemyAttackTimer(pub Timer);

pub fn td_enemy_move(
    time: Res<Time>,
    core_q: Query<&Transform, (With<DefenseCore>, Without<TdEnemy>)>,
    mut enemy_q: Query<(&mut Transform, &MoveSpeed), Without<DefenseCore>>,
) {
    let Ok(core_t) = core_q.single() else {
        return;
    };
    let core_pos = core_t.translation;

    for (mut transform, speed) in enemy_q.iter_mut() {
        combat::move_toward_target(&mut transform, core_pos, speed.0, time.delta_secs(), 0.8);
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

    if total_damage > 0.0 {
        if let Ok(mut core) = core_q.single_mut() {
            core.current_health -= total_damage;
            if core.current_health <= 0.0 {
                core.current_health = 0.0;
            }
        }
    }
}

pub fn td_check_enemy_death(
    mut commands: Commands,
    enemy_q: Query<(Entity, &TdEnemy, &Health)>,
    mut gold: ResMut<TdGold>,
    mut state: ResMut<TdWaveState>,
) {
    for (entity, enemy, health) in enemy_q.iter() {
        if health.current <= 0.0 {
            gold.0 += enemy.gold_reward;
            state.enemies_alive = state.enemies_alive.saturating_sub(1);
            commands.entity(entity).despawn();
            info!(
                "击杀敌人! +{} 金币 (剩余金币: {}, 剩余敌人: {})",
                enemy.gold_reward, gold.0, state.enemies_alive
            );
        }
    }
}
