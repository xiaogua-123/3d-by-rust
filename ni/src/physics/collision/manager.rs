//! 碰撞管理器
//!
//! 统一管理所有碰撞检测、事件分发和物理响应

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::colliders::*;
use crate::npc::{Npc, NpcConfig, NpcPatrol};

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
                    npc_vertical_separation,
                    zero_ground_npc_vertical,
                    apply_smooth_push,
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
            if let Ok((_, _, collider_a)) = collider_q.get(pair.0)
                && collider_a.is_trigger {
                    trigger_events.write(TriggerEvent {
                        trigger_entity: pair.0,
                        other_entity: pair.1,
                        trigger_type: TriggerType::Exit,
                    });
                }
            if let Ok((_, _, collider_b)) = collider_q.get(pair.1)
                && collider_b.is_trigger {
                    trigger_events.write(TriggerEvent {
                        trigger_entity: pair.1,
                        other_entity: pair.0,
                        trigger_type: TriggerType::Exit,
                    });
                }
        }
    }
}

/// 解决碰撞
/// 对非触发器碰撞应用物理响应
fn resolve_collisions(
    mut collision_events: MessageReader<CollisionEvent>,
    mut collider_q: Query<(&mut Transform, &Collider, Option<&CollisionResponse>, Option<&mut SmoothPush>)>,
) {
    // 收集需要应用的推力
    let mut push_forces: HashMap<Entity, Vec3> = HashMap::new();

    // 读取碰撞事件
    for event in collision_events.read() {
        let Ok((_, collider_a, response_a, _)) = collider_q.get(event.entity_a) else {
            continue;
        };
        let Ok((_, collider_b, response_b, _)) = collider_q.get(event.entity_b) else {
            continue;
        };

        // 跳过触发器
        if collider_a.is_trigger || collider_b.is_trigger {
            continue;
        }

        // 跳过平面碰撞体（仅用于 find_ground_y 查询，不参与物理推动）
        if matches!(collider_a.shape, ColliderShape::Plane { .. })
            || matches!(collider_b.shape, ColliderShape::Plane { .. })
        {
            continue;
        }

        let respond_a = response_a.is_none_or(|r| r.respond);
        let respond_b = response_b.is_none_or(|r| r.respond);

        if !respond_a && !respond_b {
            continue;
        }

        let push_strength = event.penetration_depth;
        let push_direction = event.contact_normal;

        // 分配推力，使用各方的 push_force 系数（0=不动，1=全推）
        let force_a = response_a.map_or(1.0, |r| r.push_force);
        let force_b = response_b.map_or(1.0, |r| r.push_force);

        if respond_a && respond_b {
            *push_forces.entry(event.entity_a).or_insert(Vec3::ZERO) -=
                push_direction * push_strength * 0.5 * force_a;
            *push_forces.entry(event.entity_b).or_insert(Vec3::ZERO) +=
                push_direction * push_strength * 0.5 * force_b;
        } else if respond_a {
            *push_forces.entry(event.entity_a).or_insert(Vec3::ZERO) -=
                push_direction * push_strength * force_a;
        } else {
            *push_forces.entry(event.entity_b).or_insert(Vec3::ZERO) +=
                push_direction * push_strength * force_b;
        }
    }

    // 应用推力：带 SmoothPush 的实体使用速度积累（平滑），否则直接位移
    for (entity, force) in push_forces {
        let Ok((mut transform, _, _, mut smooth)) = collider_q.get_mut(entity) else {
            continue;
        };
        if let Some(ref mut s) = smooth {
            s.velocity += force;
        } else {
            transform.translation += force;
        }
    }
}

/// NPC 力场分离 — 检测垂直堆叠并水平推离
///
/// 分级推力策略：
/// - 同层重叠（Y 中心差 < 0.3）：轻推，协助碰撞系统防止卡死
/// - 叠层重叠（Y 中心差 ≥ 0.3）：重推，强行炸开叠罗汉
#[allow(clippy::type_complexity)]
fn npc_vertical_separation(
    mut q: ParamSet<(
        Query<(Entity, &Transform, &NpcConfig, &NpcPatrol), With<Npc>>,
        Query<(&mut Transform, &mut SmoothPush), With<Npc>>,
    )>,
) {
    const NPC_HEIGHT: f32 = 1.6;
    const HORIZ_THRESHOLD: f32 = 0.6;
    const Y_STACK_THRESHOLD: f32 = 0.3; // 超过此值判定为叠层

    let items: Vec<(Entity, Vec3, bool, f32)> = q
        .p0()
        .iter()
        .map(|(e, t, config, patrol)| {
            (e, t.translation, config.collision_3d, patrol.ground_y)
        })
        .collect();

    if items.len() < 2 {
        return;
    }

    let mut pushes: HashMap<Entity, Vec2> = HashMap::new();

    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let (e_i, pos_i, is_3d_i, ground_i) = &items[i];
            let (e_j, pos_j, is_3d_j, ground_j) = &items[j];

            if *is_3d_i || *is_3d_j {
                continue;
            }

            // Y 区间重叠检测
            let y_bottom_i = pos_i.y;
            let y_top_i = pos_i.y + NPC_HEIGHT;
            let y_bottom_j = pos_j.y;
            let y_top_j = pos_j.y + NPC_HEIGHT;

            if y_top_i <= y_bottom_j || y_top_j <= y_bottom_i {
                continue;
            }

            let y_overlap = (y_top_i.min(y_top_j) - y_bottom_i.max(y_bottom_j)).max(0.0);

            // 水平距离检测
            let dx = pos_i.x - pos_j.x;
            let dz = pos_i.z - pos_j.z;
            let dist_h_sq = dx * dx + dz * dz;

            if dist_h_sq >= HORIZ_THRESHOLD * HORIZ_THRESHOLD {
                continue;
            }

            // 不同平台跳过
            if (ground_i - ground_j).abs() > 0.1 && y_overlap < NPC_HEIGHT * 0.3 {
                continue;
            }

            // 分级推力计算
            let y_diff = (pos_i.y - pos_j.y).abs();
            let dist_h = if dist_h_sq > 0.0001 { dist_h_sq.sqrt() } else { 0.0001 };

            let (push_strength, _label) = if y_diff >= Y_STACK_THRESHOLD {
                // 叠层：重推，强行炸开
                ((HORIZ_THRESHOLD - dist_h) * 1.5 + y_overlap * 1.0, "stack")
            } else {
                // 同层：轻推，协助碰撞系统防卡死
                ((HORIZ_THRESHOLD - dist_h) * 0.3 + y_overlap * 0.1, "同层")
            };

            let nx = dx / dist_h;
            let nz = dz / dist_h;

            *pushes.entry(*e_i).or_default() += Vec2::new(nx * push_strength, nz * push_strength);
            *pushes.entry(*e_j).or_default() -= Vec2::new(nx * push_strength, nz * push_strength);
        }
    }

    for (entity, push) in pushes {
        if push.length_squared() < 0.0001 {
            continue;
        }
        if let Ok((_transform, mut smooth)) = q.p1().get_mut(entity) {
            smooth.velocity.x += push.x;
            smooth.velocity.z += push.y;
        }
    }
}

/// 应用平滑推送
/// 将 SmoothPush.velocity 集成到位置，并施加阻尼（摩擦力）
fn apply_smooth_push(
    mut q: Query<(&mut Transform, &mut SmoothPush)>,
) {
    for (mut transform, mut push) in q.iter_mut() {
        if push.velocity.length_squared() > 0.0001 {
            transform.translation += push.velocity;
            let damping = push.damping;
            push.velocity *= damping;
        } else if push.velocity.length_squared() > 0.0 {
            push.velocity = Vec3::ZERO;
        }
    }
}

/// 地面 NPC 垂直约束 — 防止碰撞推力将 NPC 抬离地面
///
/// 地面 NPC (collision_3d = false) 不应接收 Y 轴推力：
/// 1. 清零 SmoothPush.velocity.y（如果有 SmoothPush）
/// 2. 直接将 Y 位置恢复到初始 spawn 高度（NpcPatrol.ground_y），
///    覆盖 resolve_collisions 中无 SmoothPush NPC 的
///    直接 transform.translation += force 导致的偏移
///
/// 必须在 apply_smooth_push 之前运行，否则 velocity 中的 Y 分量
/// 会被应用到位置导致 NPC 浮空。
fn zero_ground_npc_vertical(
    mut q: Query<(&mut Transform, Option<&mut SmoothPush>, &NpcConfig, &NpcPatrol), With<Npc>>,
) {
    for (mut transform, mut push, config, patrol) in q.iter_mut() {
        if config.collision_3d {
            continue;
        }
        // 清零垂直速度分量（SmoothPush 实体用 velocity 积累）
        if let Some(ref mut p) = push
            && p.velocity.y != 0.0 {
                p.velocity.y = 0.0;
            }
        // 硬恢复 Y 位置到初始 spawn 高度 — 覆盖 resolve_collisions 或
        // 无 SmoothPush NPC 的直接位移，防止 NPC 被推离地面
        if (transform.translation.y - patrol.ground_y).abs() > 0.01 {
            transform.translation.y = patrol.ground_y;
        }
    }
}

impl CollisionManager {
    /// 当前帧碰撞对数量
    pub fn current_collision_count(&self) -> usize {
        self.current_collisions.len()
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
        if let Some(layer) = target_layer
            && !collider.mask.layer.contains(layer) {
                continue;
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
        if let Some(layer) = target_layer
            && !collider.mask.layer.contains(layer) {
                continue;
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

                    if closest_hit.as_ref().is_none_or(|h| hit_distance < h.distance) {
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
