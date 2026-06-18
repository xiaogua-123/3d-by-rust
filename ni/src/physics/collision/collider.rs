//! 统一碰撞体组件系统
//!
//! 定义所有碰撞相关的组件、形状和层过滤机制

use bevy::prelude::*;
use bevy::ecs::query::QueryFilter;
use crate::player::Player;

// ═══════════════════════════════════════════
// 碰撞形状定义
// ═══════════════════════════════════════════

/// 碰撞形状类型
/// 支持球形、胶囊、立方体三种基本形状
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub enum ColliderShape {
    /// 球形碰撞体，适用于圆形物体
    Sphere { radius: f32 },
    /// 胶囊碰撞体，适用于角色控制器
    Capsule { radius: f32, half_height: f32 },
    /// 立方体碰撞体，适用于建筑、障碍物
    Box { half_extents: Vec3 },
    /// 无限水平面，适用于地面
    Plane { normal: Vec3, distance: f32 },
}

impl ColliderShape {
    /// 获取形状的包围球半径（用于快速剔除）
    pub fn bounding_radius(&self) -> f32 {
        match self {
            ColliderShape::Sphere { radius } => *radius,
            ColliderShape::Capsule { radius, half_height } => {
                (radius * radius + half_height * half_height).sqrt()
            }
            ColliderShape::Box { half_extents } => half_extents.length(),
            ColliderShape::Plane { .. } => f32::INFINITY,
        }
    }

    /// 计算两个形状之间的距离（用于快速碰撞检测）
    #[allow(dead_code)]
    pub fn distance_to(&self, self_pos: Vec3, other: &ColliderShape, other_pos: Vec3) -> f32 {
        match (self, other) {
            (ColliderShape::Sphere { radius: r1 }, ColliderShape::Sphere { radius: r2 }) => {
                self_pos.distance(other_pos) - r1 - r2
            }
            _ => {
                // 通用情况：使用包围球近似
                let r1 = self.bounding_radius();
                let r2 = other.bounding_radius();
                self_pos.distance(other_pos) - r1 - r2
            }
        }
    }
}

// ═══════════════════════════════════════════
// 碰撞层系统
// ═══════════════════════════════════════════

/// 碰撞层位掩码
/// 使用位运算进行高效的层过滤
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct CollisionLayer(pub u32);

impl CollisionLayer {
    #[allow(dead_code)]
    pub const NONE: Self = Self(0);
    pub const PLAYER: Self = Self(1 << 0);
    pub const ENEMY: Self = Self(1 << 1);
    pub const COLLECTIBLE: Self = Self(1 << 2);
    pub const NPC: Self = Self(1 << 3);
    pub const TERRAIN: Self = Self(1 << 4);
    pub const PROJECTILE: Self = Self(1 << 5);
    pub const TRIGGER: Self = Self(1 << 6);
    #[allow(dead_code)]
    pub const ALL: Self = Self(u32::MAX);

    /// 检查是否包含指定层
    pub fn contains(self, layer: Self) -> bool {
        (self.0 & layer.0) != 0
    }

    /// 合并两个层
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// 碰撞掩码组件
/// 定义该实体可以与哪些层发生碰撞
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CollisionMask {
    /// 该实体所属的层
    pub layer: CollisionLayer,
    /// 该实体可以碰撞的层
    pub mask: CollisionLayer,
}

impl CollisionMask {
    pub fn new(layer: CollisionLayer, mask: CollisionLayer) -> Self {
        Self { layer, mask }
    }

    /// 检查两个实体是否可以碰撞
    pub fn can_collide_with(&self, other: &Self) -> bool {
        self.mask.contains(other.layer) && other.mask.contains(self.layer)
    }
}

// 预定义的碰撞掩码组合
impl CollisionMask {
    /// 玩家：与敌人、收集品、NPC、地形碰撞
    pub fn player() -> Self {
        Self::new(
            CollisionLayer::PLAYER,
            CollisionLayer::ENEMY
                .union(CollisionLayer::COLLECTIBLE)
                .union(CollisionLayer::NPC)
                .union(CollisionLayer::TERRAIN)
                .union(CollisionLayer::TRIGGER),
        )
    }

    /// 敌人：与玩家、地形、敌人碰撞
    pub fn enemy() -> Self {
        Self::new(
            CollisionLayer::ENEMY,
            CollisionLayer::PLAYER
                .union(CollisionLayer::TERRAIN)
                .union(CollisionLayer::ENEMY),
        )
    }

    /// 收集品：只与玩家碰撞
    pub fn collectible() -> Self {
        Self::new(
            CollisionLayer::COLLECTIBLE,
            CollisionLayer::PLAYER,
        )
    }

    /// NPC：与玩家、其他 NPC、地形碰撞
    pub fn npc() -> Self {
        Self::new(
            CollisionLayer::NPC,
            CollisionLayer::PLAYER
                .union(CollisionLayer::NPC)
                .union(CollisionLayer::TERRAIN),
        )
    }

    /// NPC：只与玩家碰撞（不与其他 NPC 碰撞）
    pub fn npc_no_push() -> Self {
        Self::new(
            CollisionLayer::NPC,
            CollisionLayer::PLAYER,
        )
    }

    /// 地形：与所有物理实体碰撞
    pub fn terrain() -> Self {
        Self::new(
            CollisionLayer::TERRAIN,
            CollisionLayer::PLAYER
                .union(CollisionLayer::ENEMY)
                .union(CollisionLayer::NPC)
                .union(CollisionLayer::PROJECTILE),
        )
    }

    /// 触发器：只与玩家碰撞
    #[allow(dead_code)]
    pub fn trigger() -> Self {
        Self::new(
            CollisionLayer::TRIGGER,
            CollisionLayer::PLAYER,
        )
    }
}

// ═══════════════════════════════════════════
// 碰撞体组件
// ═══════════════════════════════════════════

/// 碰撞体主组件
/// 组合了形状、层信息和物理属性
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Collider {
    pub shape: ColliderShape,
    pub mask: CollisionMask,
    /// 是否为触发器（不产生物理响应，只触发事件）
    pub is_trigger: bool,
    /// 碰撞时的摩擦系数
    pub friction: f32,
    /// 碰撞时的弹性系数
    pub restitution: f32,
}

impl Collider {
    /// 创建新的碰撞体
    pub fn new(shape: ColliderShape, mask: CollisionMask) -> Self {
        Self {
            shape,
            mask,
            is_trigger: false,
            friction: 0.5,
            restitution: 0.3,
        }
    }

    /// 创建触发器碰撞体
    pub fn trigger(shape: ColliderShape, mask: CollisionMask) -> Self {
        Self {
            shape,
            mask,
            is_trigger: true,
            friction: 0.0,
            restitution: 0.0,
        }
    }

    /// 创建球形碰撞体
    pub fn sphere(radius: f32, mask: CollisionMask) -> Self {
        Self::new(ColliderShape::Sphere { radius }, mask)
    }

    /// 创建胶囊碰撞体
    pub fn capsule(radius: f32, half_height: f32, mask: CollisionMask) -> Self {
        Self::new(ColliderShape::Capsule { radius, half_height }, mask)
    }

    /// 创建立方体碰撞体
    #[allow(dead_code)]
    pub fn cuboid(half_extents: Vec3, mask: CollisionMask) -> Self {
        Self::new(ColliderShape::Box { half_extents }, mask)
    }

    /// 创建地面平面碰撞体
    #[allow(dead_code)]
    pub fn ground(y: f32) -> Self {
        Self::new(
            ColliderShape::Plane {
                normal: Vec3::Y,
                distance: y,
            },
            CollisionMask::terrain(),
        )
    }

    /// 查询碰撞体在给定 XZ 位置的地面高度（等效于旧 CollisionShape::ground_height_at）
    pub fn ground_height_at(&self, transform: &Transform, point_xz: Vec2) -> Option<f32> {
        match &self.shape {
            ColliderShape::Plane { normal: _, distance } => {
                Some(*distance)
            }
            ColliderShape::Box { half_extents } => {
                let pos = transform.translation;
                let s = transform.scale;
                let hx = half_extents.x * s.x;
                let hz = half_extents.z * s.z;
                if point_xz.x >= pos.x - hx && point_xz.x <= pos.x + hx
                    && point_xz.y >= pos.z - hz && point_xz.y <= pos.z + hz
                {
                    Some(pos.y + half_extents.y * s.y)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 将玩家从碰撞体水平推出（等效于旧 push_out_horizontal）
    pub fn push_out_horizontal(
        &self,
        transform: &Transform,
        player_pos: &mut Vec3,
        player_radius: f32,
        player_height: f32,
    ) {
        let ColliderShape::Box { half_extents } = &self.shape else { return };
        let pos = transform.translation;
        let s = transform.scale;
        let hx = half_extents.x * s.x;
        let hz = half_extents.z * s.z;
        let hy = half_extents.y * s.y;

        let player_bottom = player_pos.y;
        let player_top = player_pos.y + player_height;
        let box_bottom = pos.y - hy;
        let box_top = pos.y + hy;
        if player_top <= box_bottom || player_bottom >= box_top { return; }

        let box_min_x = pos.x - hx; let box_max_x = pos.x + hx;
        let box_min_z = pos.z - hz; let box_max_z = pos.z + hz;
        let closest_x = player_pos.x.clamp(box_min_x, box_max_x);
        let closest_z = player_pos.z.clamp(box_min_z, box_max_z);
        let dx = player_pos.x - closest_x;
        let dz = player_pos.z - closest_z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < player_radius * player_radius && dist_sq > f32::EPSILON {
            let dist = dist_sq.sqrt();
            let push = player_radius - dist;
            player_pos.x += dx / dist * push;
            player_pos.z += dz / dist * push;
        } else if dist_sq <= f32::EPSILON {
            let overlap_x = hx - (player_pos.x - pos.x).abs();
            let overlap_z = hz - (player_pos.z - pos.z).abs();
            if overlap_x < overlap_z {
                player_pos.x += if player_pos.x > pos.x { 1.0 } else { -1.0 } * (overlap_x + player_radius);
            } else {
                player_pos.z += if player_pos.z > pos.z { 1.0 } else { -1.0 } * (overlap_z + player_radius);
            }
        }
    }
}

// ═══════════════════════════════════════════
// 辅助函数（从旧 shape.rs 迁移）
// ═══════════════════════════════════════════

/// 在给定的碰撞体集合中，查找玩家脚底正下方的最大地面高度。
/// 用于确定玩家是否着地，以及着地时应站立的 Y 坐标。
pub fn find_ground_y<F: QueryFilter>(
    collision_q: &Query<(&Transform, &Collider), F>,
    player_xz: Vec2,
) -> f32 {
    let mut best = f32::NEG_INFINITY;
    for (t, collider) in collision_q.iter() {
        if let Some(h) = collider.ground_height_at(t, player_xz)
            && h > best {
                best = h;
            }
    }
    best
}

/// 将玩家实体推出任何重叠的立方体碰撞体。
/// 使用圆-vs-AABB 最近点算法处理水平碰撞。
/// `player_pos` 会被原地修改，`player_radius` 为玩家水平圆形半径，`player_height` 为玩家高度。
pub fn push_out_horizontal(
    collision_q: &Query<(&Transform, &Collider), Without<Player>>,
    player_pos: &mut Vec3,
    player_radius: f32,
    player_height: f32,
) {
    for (t, collider) in collision_q.iter() {
        collider.push_out_horizontal(t, player_pos, player_radius, player_height);
    }
}

// ═══════════════════════════════════════════
// 碰撞事件
// ═══════════════════════════════════════════

/// 碰撞事件
/// 当两个碰撞体开始接触时触发
#[derive(Message, Debug, Clone)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    #[allow(dead_code)]
    pub contact_point: Vec3,
    pub contact_normal: Vec3,
    pub penetration_depth: f32,
}

/// 碰撞分离事件
/// 当两个碰撞体分离时触发
#[derive(Message, Debug, Clone)]
pub struct CollisionSeparationEvent {
    #[allow(dead_code)]
    pub entity_a: Entity,
    #[allow(dead_code)]
    pub entity_b: Entity,
}

/// 触发器事件
/// 当实体进入或离开触发器时触发
#[derive(Message, Debug, Clone)]
pub struct TriggerEvent {
    pub trigger_entity: Entity,
    #[allow(dead_code)]
    pub other_entity: Entity,
    pub trigger_type: TriggerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    Enter,
    Stay,
    Exit,
}

// ═══════════════════════════════════════════
// 碰撞响应组件
// ═══════════════════════════════════════════

/// 碰撞响应配置
/// 定义实体在碰撞时的行为
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CollisionResponse {
    /// 是否响应碰撞（推力）
    pub respond: bool,
    /// 碰撞时的推力强度
    pub push_force: f32,
}

impl Default for CollisionResponse {
    fn default() -> Self {
        Self {
            respond: true,
            push_force: 1.0,
        }
    }
}

impl CollisionResponse {
    pub fn kinematic() -> Self {
        Self {
            respond: false,
            push_force: 0.0,
        }
    }
}

/// 平滑推送组件
/// 使碰撞推力以速度 + 阻尼的方式逐渐生效，而非瞬间位移
/// 适合 NPC 等需要柔和碰撞响应的实体
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct SmoothPush {
    /// 当前累积的推送速度
    pub velocity: Vec3,
    /// 每帧阻尼系数 (0~1)，越大摩擦力越小，滑得越远
    /// 0.85 = 每帧衰减 15%，约 30 帧后几乎停止
    pub damping: f32,
}

impl Default for SmoothPush {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            damping: 0.85,
        }
    }
}

// ═══════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════

/// 检查两个碰撞体是否重叠
pub fn check_overlap(
    pos_a: Vec3,
    shape_a: &ColliderShape,
    pos_b: Vec3,
    shape_b: &ColliderShape,
) -> Option<(Vec3, f32)> {
    match (shape_a, shape_b) {
        // 球 vs 球
        (ColliderShape::Sphere { radius: r1 }, ColliderShape::Sphere { radius: r2 }) => {
            let diff = pos_b - pos_a;
            let dist = diff.length();
            let min_dist = r1 + r2;

            if dist < min_dist && dist > 0.0 {
                let normal = diff / dist;
                let penetration = min_dist - dist;
                Some((normal, penetration))
            } else {
                None
            }
        }

        // 球 vs 立方体
        (ColliderShape::Sphere { radius }, ColliderShape::Box { half_extents }) => {
            check_sphere_box_collision(pos_a, *radius, pos_b, *half_extents)
        }

        // 立方体 vs 球
        (ColliderShape::Box { half_extents }, ColliderShape::Sphere { radius }) => {
            check_sphere_box_collision(pos_b, *radius, pos_a, *half_extents)
                .map(|(normal, depth)| (-normal, depth))
        }

        // 胶囊 vs 球（简化为球 vs 球）
        (ColliderShape::Capsule { radius, half_height }, ColliderShape::Sphere { radius: r2 }) => {
            let capsule_top = pos_a + Vec3::Y * half_height;
            let capsule_bottom = pos_a - Vec3::Y * half_height;
            let closest_point = closest_point_on_segment(pos_b, capsule_top, capsule_bottom);
            let diff = pos_b - closest_point;
            let dist = diff.length();
            let min_dist = radius + r2;

            if dist < min_dist && dist > 0.0 {
                let normal = diff / dist;
                let penetration = min_dist - dist;
                Some((normal, penetration))
            } else {
                None
            }
        }

        // 球 vs 胶囊
        (ColliderShape::Sphere { radius }, ColliderShape::Capsule { radius: r2, half_height }) => {
            let capsule_top = pos_b + Vec3::Y * half_height;
            let capsule_bottom = pos_b - Vec3::Y * half_height;
            let closest_point = closest_point_on_segment(pos_a, capsule_top, capsule_bottom);
            let diff = pos_a - closest_point;
            let dist = diff.length();
            let min_dist = radius + r2;

            if dist < min_dist && dist > 0.0 {
                let normal = diff / dist;
                let penetration = min_dist - dist;
                Some((-normal, penetration))
            } else {
                None
            }
        }

        // 胶囊 vs 立方体 — 将胶囊简化为线段上的球，复用球 vs 立方体检测
        (ColliderShape::Capsule { radius, half_height }, ColliderShape::Box { half_extents }) => {
            let capsule_top = pos_a + Vec3::Y * half_height;
            let capsule_bottom = pos_a - Vec3::Y * half_height;
            let closest_on_seg = closest_point_on_segment(pos_b, capsule_top, capsule_bottom);
            check_sphere_box_collision(closest_on_seg, *radius, pos_b, *half_extents)
        }

        // 立方体 vs 胶囊
        (ColliderShape::Box { half_extents }, ColliderShape::Capsule { radius, half_height }) => {
            let capsule_top = pos_b + Vec3::Y * half_height;
            let capsule_bottom = pos_b - Vec3::Y * half_height;
            let closest_on_seg = closest_point_on_segment(pos_a, capsule_top, capsule_bottom);
            check_sphere_box_collision(closest_on_seg, *radius, pos_a, *half_extents)
        }

        // 平面 vs 球 — 球底部低于平面则推回
        (ColliderShape::Plane { normal, distance }, ColliderShape::Sphere { radius }) => {
            plane_sphere_overlap(pos_a, *normal, *distance, pos_b, *radius)
        }

        // 球 vs 平面
        (ColliderShape::Sphere { radius }, ColliderShape::Plane { normal, distance }) => {
            plane_sphere_overlap(pos_b, *normal, *distance, pos_a, *radius)
                .map(|(normal, depth)| (-normal, depth))
        }

        // 平面 vs 胶囊 — 胶囊底部低于平面则推回
        (ColliderShape::Plane { normal, distance }, ColliderShape::Capsule { radius, half_height }) => {
            let capsule_bottom = pos_b - Vec3::Y * half_height;
            let signed_dist = capsule_bottom.dot(*normal) - distance;
            let min_dist = -radius; // 胶囊底部允许在平面距离之上半径范围内
            if signed_dist < min_dist {
                Some((*normal, min_dist - signed_dist))
            } else {
                None
            }
        }

        // 胶囊 vs 平面
        (ColliderShape::Capsule { radius, half_height }, ColliderShape::Plane { normal, distance }) => {
            let capsule_bottom = pos_a - Vec3::Y * half_height;
            let signed_dist = capsule_bottom.dot(*normal) - distance;
            let min_dist = -radius;
            if signed_dist < min_dist {
                Some((-*normal, min_dist - signed_dist))
            } else {
                None
            }
        }

        // 其他情况：使用包围球近似
        _ => {
            let r1 = shape_a.bounding_radius();
            let r2 = shape_b.bounding_radius();
            let diff = pos_b - pos_a;
            let dist = diff.length();
            let min_dist = r1 + r2;

            if dist < min_dist && dist > 0.0 {
                let normal = diff / dist;
                let penetration = min_dist - dist;
                Some((normal, penetration))
            } else {
                None
            }
        }
    }
}

/// 球 vs 立方体碰撞检测
fn check_sphere_box_collision(
    sphere_pos: Vec3,
    sphere_radius: f32,
    box_pos: Vec3,
    box_half_extents: Vec3,
) -> Option<(Vec3, f32)> {
    let local_sphere = sphere_pos - box_pos;
    let clamped = local_sphere.clamp(-box_half_extents, box_half_extents);
    let closest_point = box_pos + clamped;
    let diff = sphere_pos - closest_point;
    let dist = diff.length();

    if dist < sphere_radius && dist > 0.0 {
        let normal = diff / dist;
        let penetration = sphere_radius - dist;
        Some((normal, penetration))
    } else if dist <= 0.0 {
        // 球心在立方体内部
        let to_center = sphere_pos - box_pos;
        let abs_x = to_center.x.abs() / box_half_extents.x;
        let abs_y = to_center.y.abs() / box_half_extents.y;
        let abs_z = to_center.z.abs() / box_half_extents.z;

        let (normal, penetration) = if abs_x >= abs_y && abs_x >= abs_z {
            (Vec3::X * to_center.x.signum(), box_half_extents.x - to_center.x.abs() + sphere_radius)
        } else if abs_y >= abs_x && abs_y >= abs_z {
            (Vec3::Y * to_center.y.signum(), box_half_extents.y - to_center.y.abs() + sphere_radius)
        } else {
            (Vec3::Z * to_center.z.signum(), box_half_extents.z - to_center.z.abs() + sphere_radius)
        };

        Some((normal, penetration))
    } else {
        None
    }
}

/// 平面 vs 球碰撞检测
/// 球的底部低于平面则该方向推力
fn plane_sphere_overlap(
    plane_pos: Vec3,
    plane_normal: Vec3,
    plane_distance: f32,
    sphere_pos: Vec3,
    sphere_radius: f32,
) -> Option<(Vec3, f32)> {
    let sphere_center = sphere_pos - plane_pos;
    let signed_dist = sphere_center.dot(plane_normal) - plane_distance;
    // 球心到平面的有符号距离小于半径 → 穿透
    if signed_dist < sphere_radius {
        Some((plane_normal, sphere_radius - signed_dist))
    } else {
        None
    }
}

/// 计算点到线段的最近点
fn closest_point_on_segment(point: Vec3, seg_a: Vec3, seg_b: Vec3) -> Vec3 {
    let ab = seg_b - seg_a;
    let ap = point - seg_a;
    let t = ap.dot(ab) / ab.dot(ab);
    let t = t.clamp(0.0, 1.0);
    seg_a + ab * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_ground_height() {
        let collider = Collider::new(ColliderShape::Plane { normal: Vec3::Y, distance: 0.0 }, CollisionMask::new(CollisionLayer::PLAYER, CollisionLayer::PLAYER.union(CollisionLayer::TERRAIN)));
        let transform = Transform::IDENTITY;
        assert_eq!(collider.ground_height_at(&transform, Vec2::ZERO), Some(0.0));
        assert_eq!(collider.ground_height_at(&transform, Vec2::new(100.0, 100.0)), Some(0.0));
    }

    #[test]
    fn box_ground_height_inside() {
        let collider = Collider::new(ColliderShape::Box { half_extents: Vec3::splat(1.0) }, CollisionMask::new(CollisionLayer::PLAYER, CollisionLayer::PLAYER.union(CollisionLayer::TERRAIN)));
        let transform = Transform::from_xyz(0.0, 5.0, 0.0);
        assert_eq!(collider.ground_height_at(&transform, Vec2::ZERO), Some(6.0));
    }

    #[test]
    fn box_ground_height_outside() {
        let collider = Collider::new(ColliderShape::Box { half_extents: Vec3::splat(1.0) }, CollisionMask::new(CollisionLayer::PLAYER, CollisionLayer::PLAYER.union(CollisionLayer::TERRAIN)));
        let transform = Transform::from_xyz(0.0, 5.0, 0.0);
        assert_eq!(collider.ground_height_at(&transform, Vec2::new(10.0, 0.0)), None);
    }

    #[test]
    fn sphere_bounding_radius() {
        let shape = ColliderShape::Sphere { radius: 2.0 };
        assert!((shape.bounding_radius() - 2.0).abs() < 0.01);
    }

    #[test]
    fn capsule_bounding_radius() {
        let shape = ColliderShape::Capsule { radius: 1.0, half_height: 2.0 };
        let expected = (1.0f32 * 1.0 + 2.0 * 2.0).sqrt();
        assert!((shape.bounding_radius() - expected).abs() < 0.01);
    }

    #[test]
    fn sphere_sphere_collision_inside_range() {
        let pos_a = Vec3::ZERO;
        let shape_a = ColliderShape::Sphere { radius: 2.0 };
        let pos_b = Vec3::new(3.0, 0.0, 0.0);
        let shape_b = ColliderShape::Sphere { radius: 2.0 };
        let result = check_overlap(pos_a, &shape_a, pos_b, &shape_b);
        assert!(result.is_some());
        let (normal, depth) = result.unwrap();
        assert!((normal - Vec3::X).length() < 0.01);
        assert!((depth - 1.0).abs() < 0.01);
    }

    #[test]
    fn sphere_sphere_no_collision() {
        let pos_a = Vec3::ZERO;
        let shape_a = ColliderShape::Sphere { radius: 1.0 };
        let pos_b = Vec3::new(5.0, 0.0, 0.0);
        let shape_b = ColliderShape::Sphere { radius: 1.0 };
        assert!(check_overlap(pos_a, &shape_a, pos_b, &shape_b).is_none());
    }
}
