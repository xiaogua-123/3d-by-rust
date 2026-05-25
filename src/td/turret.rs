// ═══════════════════════════════════════════
// 炮塔系统：瞄准、开火、购买
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::level::LevelEntity;
use super::balance::GameDatabase;
use super::data::*;
use super::events::PurchaseTurretEvent;

pub fn td_turret_target(
    mut turret_q: Query<(&Transform, &mut Turret)>,
    enemy_q: Query<(Entity, &Transform), With<TdEnemy>>,
) {
    for (turret_t, mut turret) in turret_q.iter_mut() {
        let mut closest: Option<(Entity, f32)> = None;

        for (enemy_entity, enemy_t) in enemy_q.iter() {
            let dist = turret_t.translation.distance(enemy_t.translation);
            if dist <= turret.range {
                if let Some((_, closest_dist)) = closest {
                    if dist < closest_dist {
                        closest = Some((enemy_entity, dist));
                    }
                } else {
                    closest = Some((enemy_entity, dist));
                }
            }
        }

        turret.target = closest.map(|(e, _)| e);
    }
}

pub fn td_turret_fire_tick(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut turret_q: Query<(Entity, &Transform, &mut Turret)>,
    enemy_q: Query<&Transform, With<TdEnemy>>,
    db: Res<GameDatabase>,
) {
    for (_turret_entity, turret_t, mut turret) in turret_q.iter_mut() {
        turret.fire_timer.tick(time.delta());

        let Some(target_entity) = turret.target else {
            continue;
        };
        let Ok(enemy_t) = enemy_q.get(target_entity) else {
            turret.target = None;
            continue;
        };

        if !turret.fire_timer.is_finished() {
            continue;
        }

        let muzzle_pos = turret_t.translation + Vec3::new(0.0, turret.barrel_y, 0.0);
        let target_pos = enemy_t.translation;

        let proj_color = db
            .find_turret(turret.turret_type.id())
            .map(|d| d.to_color())
            .unwrap_or_else(|| turret.turret_type.color());
        let proj_mat = materials.add(StandardMaterial {
            base_color: proj_color,
            emissive: proj_color.into(),
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.12).mesh())),
            MeshMaterial3d(proj_mat),
            Transform::from_translation(muzzle_pos),
            Projectile {
                damage: turret.damage,
                speed: 8.0,
                target_pos,
                lifetime: Timer::from_seconds(2.0, TimerMode::Once),
                color: proj_color,
            },
            LevelEntity,
            Name::new("Projectile"),
        ));

        turret.fire_timer.reset();
    }
}

pub fn td_handle_purchase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<PurchaseTurretEvent>,
    mut gold: ResMut<TdGold>,
    db: Res<GameDatabase>,
) {
    for ev in events.read() {
        let def = db.find_turret(ev.turret_type.id());
        let cost = def.map(|d| d.cost).unwrap_or_else(|| ev.turret_type.cost());
        if gold.0 < cost {
            info!("金币不足! 需要 {}, 当前 {}", cost, gold.0);
            continue;
        }

        gold.0 -= cost;

        let color = def
            .map(|d| d.to_color())
            .unwrap_or_else(|| ev.turret_type.color());
        let mat = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        });

        let barrel_h = 0.8;
        let range = def.map(|d| d.range).unwrap_or_else(|| ev.turret_type.range());
        let damage = def.map(|d| d.damage).unwrap_or_else(|| ev.turret_type.damage());
        let fire_rate = def.map(|d| d.fire_rate).unwrap_or_else(|| ev.turret_type.fire_rate());

        commands
            .spawn((
                Transform::from_xyz(ev.position.x, 0.0, ev.position.z),
                Turret {
                    turret_type: ev.turret_type,
                    range,
                    damage,
                    fire_timer: Timer::from_seconds(fire_rate, TimerMode::Once),
                    target: None,
                    barrel_y: barrel_h,
                },
                LevelEntity,
                Name::new(format!("Turret_{:?}", ev.turret_type)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.3, 0.15))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.3, 0.3, 0.3),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.075, 0.0),
                    Name::new("Base"),
                ));
                parent.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.08, barrel_h))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, barrel_h / 2.0 + 0.15, 0.0),
                    Name::new("Barrel"),
                ));
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.12).mesh())),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, barrel_h + 0.15, 0.0),
                    Name::new("Head"),
                ));
                parent.spawn((
                    Mesh3d(meshes.add(Torus::new(range, 0.03))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color.with_alpha(0.3),
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_rotation(Quat::from_rotation_x(
                        std::f32::consts::FRAC_PI_2,
                    )),
                    Name::new("RangeRing"),
                ));
            });

        info!(
            "购买 {:?} 成功! (-{} 金币, 剩余 {})",
            ev.turret_type, cost, gold.0
        );
    }
}
