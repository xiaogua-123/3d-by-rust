//! 统一碰撞体组件系统
//!
//! 定义所有碰撞相关的组件、形状和层过滤机制

use bevy::prelude::*;

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

    /// NPC：与玩家碰撞
    pub fn npc() -> Self {
        Self::new(
            CollisionLayer::NPC,
            CollisionLayer::PLAYER,
        )
    }

    /// 地形：与所有物理实体碰撞
    #[allow(dead_code)]
    pub fn terrain() -> Self {
        Self::new(
            CollisionLayer::TERRAIN,
            CollisionLayer::PLAYER
                .union(CollisionLayer::ENEMY)
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

/// 计算点到线段的最近点
fn closest_point_on_segment(point: Vec3, seg_a: Vec3, seg_b: Vec3) -> Vec3 {
    let ab = seg_b - seg_a;
    let ap = point - seg_a;
    let t = ap.dot(ab) / ab.dot(ab);
    let t = t.clamp(0.0, 1.0);
    seg_a + ab * t
}
