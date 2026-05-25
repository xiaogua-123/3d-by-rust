// ═══════════════════════════════════════════
// 弹丸系统：移动、命中检测
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::combat::Health;
use super::data::*;

pub fn td_projectile_move(
    time: Res<Time>,
    mut commands: Commands,
    mut projectile_q: Query<(Entity, &mut Transform, &mut Projectile)>,
) {
    for (entity, mut transform, mut projectile) in projectile_q.iter_mut() {
        projectile.lifetime.tick(time.delta());
        if projectile.lifetime.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let dir = (projectile.target_pos - transform.translation).normalize();
        transform.translation += dir * projectile.speed * time.delta_secs();
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, dir);
    }
}

pub fn td_projectile_hit(
    mut commands: Commands,
    projectile_q: Query<(Entity, &Transform, &Projectile)>,
    mut enemy_q: Query<(Entity, &Transform, &TdEnemy, &mut Health)>,
) {
    for (proj_entity, proj_t, projectile) in projectile_q.iter() {
        for (_enemy_entity, enemy_t, td_enemy, mut health) in enemy_q.iter_mut() {
            let dist = proj_t.translation.distance(enemy_t.translation);
            if dist < td_enemy.enemy_type.size() + 0.3 {
                health.current -= projectile.damage;
                commands.entity(proj_entity).despawn();
                break;
            }
        }
    }
}
