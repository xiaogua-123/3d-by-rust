//! 碰撞管理器
//!
//! 统一管理所有碰撞检测、事件分发和物理响应

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::colliders::*;

/// 碰撞对标识
/// 用于跟踪当前帧的碰撞状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CollisionPair(Entity, Entity);

impl CollisionPair {
    fn new(a: Entity, b: Entity) -> Self {
        // 确保顺序一致
        if a < b {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }
}

/// 碰撞管理器资源
/// 存储碰撞检测的全局状态
#[derive(Resource, Default)]
pub struct CollisionManager {
    /// 上一帧的碰撞对（用于检测分离事件）
    previous_collisions: HashSet<CollisionPair>,
    /// 当前帧的碰撞对
    current_collisions: HashSet<CollisionPair>,
}

/// 碰撞管理器插件
pub struct CollisionManagerPlugin;

impl Plugin for CollisionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionManager>()
            .add_message::<CollisionEvent>()
            .add_message::<CollisionSeparationEvent>()
            .add_message::<TriggerEvent>()
            .add_systems(
                FixedUpdate,
                (
                    update_collisions,
                    resolve_collisions,
                    cleanup_collision_state,
                )
                    .chain(),
            );
    }
}

/// 更新碰撞状态
/// 检测所有碰撞对，生成碰撞事件
fn update_collisions(
    mut collision_manager: ResMut<CollisionManager>,
    mut collision_events: MessageWriter<CollisionEvent>,
    mut separation_events: MessageWriter<CollisionSeparationEvent>,
    mut trigger_events: MessageWriter<TriggerEvent>,
    collider_q: Query<(Entity, &Transform, &Collider)>,
) {
    // 清空当前帧碰撞
    collision_manager.current_collisions.clear();

    let entities: Vec<_> = collider_q.iter().collect();

    // 检测所有碰撞对
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (entity_a, transform_a, collider_a) = &entities[i];
            let (entity_b, transform_b, collider_b) = &entities[j];

            // 检查层过滤
            if !collider_a.mask.can_collide_with(&collider_b.mask) {
                continue;
            }

            let pos_a = transform_a.translation;
            let pos_b = transform_b.translation;

            // 检测碰撞
            if let Some((normal, depth)) = check_overlap(
                pos_a,
                &collider_a.shape,
                pos_b,
                &collider_b.shape,
            ) {
                let pair = CollisionPair::new(*entity_a, *entity_b);
                collision_manager.current_collisions.insert(pair);

                // 计算接触点（简化：使用两个位置的中点）
                let contact_point = (pos_a + pos_b) / 2.0;

                // 发送碰撞事件
                collision_events.write(CollisionEvent {
                    entity_a: *entity_a,
                    entity_b: *entity_b,
                    contact_point,
                    contact_normal: normal,
                    penetration_depth: depth,
                });

                // 处理触发器事件
                if collider_a.is_trigger || collider_b.is_trigger {
                    let (trigger_entity, other_entity) = if collider_a.is_trigger {
                        (*entity_a, *entity_b)
                    } else {
                        (*entity_b, *entity_a)
                    };

                    let trigger_type = if collision_manager.previous_collisions.contains(&pair) {
                        TriggerType::Stay
                    } else {
                        TriggerType::Enter
                    };

                    trigger_events.write(TriggerEvent {
                        trigger_entity,
                        other_entity,
                        trigger_type,
                    });
                }
            }
        }
    }

    // 检测分离事件
    for pair in &collision_manager.previous_collisions {
        if !collision_manager.current_collisions.contains(pair) {
            separation_events.write(CollisionSeparationEvent {
                entity_a: pair.0,
                entity_b: pair.1,
            });

            // 对于触发器，发送 Exit 事件
            // 这里简化处理，实际可能需要更复杂的逻辑
            if let Ok((_, _, collider_a)) = collider_q.get(pair.0) {
                if collider_a.is_trigger {
                    trigger_events.write(TriggerEvent {
                        trigger_entity: pair.0,
                        other_entity: pair.1,
                        trigger_type: TriggerType::Exit,
                    });
                }
            }
            if let Ok((_, _, collider_b)) = collider_q.get(pair.1) {
                if collider_b.is_trigger {
                    trigger_events.write(TriggerEvent {
                        trigger_entity: pair.1,
                        other_entity: pair.0,
                        trigger_type: TriggerType::Exit,
                    });
                }
            }
        }
    }
}

/// 解决碰撞
/// 对非触发器碰撞应用物理响应
fn resolve_collisions(
    mut collision_events: MessageReader<CollisionEvent>,
    mut collider_q: Query<(&mut Transform, &Collider, Option<&CollisionResponse>)>,
) {
    // 收集需要应用的推力
    let mut push_forces: HashMap<Entity, Vec3> = HashMap::new();

    // 读取碰撞事件
    for event in collision_events.read() {
        let Ok((_, collider_a, response_a)) = collider_q.get(event.entity_a) else {
            continue;
        };
        let Ok((_, collider_b, response_b)) = collider_q.get(event.entity_b) else {
            continue;
        };

        // 跳过触发器
        if collider_a.is_trigger || collider_b.is_trigger {
            continue;
        }

        let respond_a = response_a.map_or(true, |r| r.respond);
        let respond_b = response_b.map_or(true, |r| r.respond);

        if !respond_a && !respond_b {
            continue;
        }

        let push_strength = event.penetration_depth;
        let push_direction = event.contact_normal;

        // 分配推力
        if respond_a && respond_b {
            // 双方各推一半
            *push_forces.entry(event.entity_a).or_insert(Vec3::ZERO) -=
                push_direction * push_strength * 0.5;
            *push_forces.entry(event.entity_b).or_insert(Vec3::ZERO) +=
                push_direction * push_strength * 0.5;
        } else if respond_a {
            // 只推动 A
            *push_forces.entry(event.entity_a).or_insert(Vec3::ZERO) -=
                push_direction * push_strength;
        } else {
            // 只推动 B
            *push_forces.entry(event.entity_b).or_insert(Vec3::ZERO) +=
                push_direction * push_strength;
        }
    }

    // 应用推力
    for (entity, force) in push_forces {
        if let Ok((mut transform, _, _)) = collider_q.get_mut(entity) {
            transform.translation += force;
        }
    }
}

/// 清理碰撞状态
/// 更新上一帧的碰撞记录
fn cleanup_collision_state(mut collision_manager: ResMut<CollisionManager>) {
    collision_manager.previous_collisions = collision_manager.current_collisions.clone();
}

// ═══════════════════════════════════════════
// 辅助查询函数
// ═══════════════════════════════════════════

/// 查找指定位置附近的实体
#[allow(dead_code)]
pub fn find_entities_in_radius(
    position: Vec3,
    radius: f32,
    collider_q: &Query<(Entity, &Transform, &Collider)>,
    target_layer: Option<CollisionLayer>,
) -> Vec<(Entity, f32)> {
    let mut results = Vec::new();

    for (entity, transform, collider) in collider_q.iter() {
        // 检查层过滤
        if let Some(layer) = target_layer {
            if !collider.mask.layer.contains(layer) {
                continue;
            }
        }

        let distance = position.distance(transform.translation);
        let combined_radius = radius + collider.shape.bounding_radius();

        if distance <= combined_radius {
            results.push((entity, distance));
        }
    }

    // 按距离排序
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// 检查点是否在碰撞体内
#[allow(dead_code)]
pub fn point_in_collider(
    point: Vec3,
    transform: &Transform,
    collider: &Collider,
) -> bool {
    let local_point = point - transform.translation;

    match &collider.shape {
        ColliderShape::Sphere { radius } => local_point.length_squared() <= radius * radius,
        ColliderShape::Capsule { radius, half_height } => {
            // 简化：检查是否在胶囊的包围盒内
            let clamped_y = local_point.y.clamp(-half_height, *half_height);
            let center = Vec3::new(0.0, clamped_y, 0.0);
            local_point.distance_squared(center) <= radius * radius
        }
        ColliderShape::Box { half_extents } => {
            local_point.x.abs() <= half_extents.x
                && local_point.y.abs() <= half_extents.y
                && local_point.z.abs() <= half_extents.z
        }
        ColliderShape::Plane { normal, distance } => {
            local_point.dot(*normal) <= *distance
        }
    }
}

/// 射线检测结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RaycastHit {
    pub entity: Entity,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// 简单的射线检测
#[allow(dead_code)]
pub fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    collider_q: &Query<(Entity, &Transform, &Collider)>,
    target_layer: Option<CollisionLayer>,
) -> Option<RaycastHit> {
    let mut closest_hit: Option<RaycastHit> = None;

    for (entity, transform, collider) in collider_q.iter() {
        // 检查层过滤
        if let Some(layer) = target_layer {
            if !collider.mask.layer.contains(layer) {
                continue;
            }
        }

        // 简化的射线-球体检测
        if let ColliderShape::Sphere { radius } = &collider.shape {
            let to_center = transform.translation - origin;
            let projection = to_center.dot(direction);

            if projection < 0.0 || projection > max_distance {
                continue;
            }

            let closest_point = origin + direction * projection;
            let distance_to_center = closest_point.distance(transform.translation);

            if distance_to_center <= *radius {
                let hit_distance = projection - (radius * radius - distance_to_center * distance_to_center).sqrt();

                if hit_distance >= 0.0 && hit_distance <= max_distance {
                    let hit_point = origin + direction * hit_distance;
                    let hit_normal = (hit_point - transform.translation).normalize();

                    if closest_hit.is_none() || hit_distance < closest_hit.as_ref().unwrap().distance {
                        closest_hit = Some(RaycastHit {
                            entity,
                            point: hit_point,
                            normal: hit_normal,
                            distance: hit_distance,
                        });
                    }
                }
            }
        }
    }

    closest_hit
}

// ═══════════════════════════════════════════
// 系统标签
// ═══════════════════════════════════════════

/// 碰撞检测系统集
/// 用于控制系统执行顺序
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct CollisionSet {
    /// 碰撞检测阶段
    pub detection: bool,
    /// 碰撞响应阶段
    pub response: bool,
}
