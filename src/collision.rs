use bevy::prelude::*;
use bevy::ecs::query::QueryFilter;
use crate::player::Player;

/// 场景中可碰撞物体的形状定义。
/// 支持无限水平面和轴对齐立方体（AABB）。
#[derive(Component, Clone, Reflect)]
pub enum CollisionShape {
    /// 无限水平面，所有实体不能穿过该 Y 坐标。
    Plane { y: f32 },
    /// 轴对齐立方体（考虑实体缩放），以实体位置为中心。
    Box { half_extents: Vec3 },
}

impl CollisionShape {
    /// 在给定世界变换和 XZ 平面点的情况下，返回该点正下方的地面高度。
    /// 如果该点在立方体水平范围内，返回立方体顶部 Y 坐标；否则返回 None。
    /// 平面总是返回其自身 Y 坐标。
    pub fn ground_height_at(&self, transform: &Transform, point_xz: Vec2) -> Option<f32> {
        match self {
            CollisionShape::Plane { y } => Some(*y),
            CollisionShape::Box { half_extents } => {
                let pos = transform.translation;
                let s = transform.scale;
                let hx = half_extents.x * s.x; // 考虑缩放后的半长
                let hz = half_extents.z * s.z; // 考虑缩放后的半宽
                let top_y = pos.y + half_extents.y * s.y; // 立方体顶部 Y 坐标

                // 检测点是否在立方体的 XZ 投影范围内
                if point_xz.x >= pos.x - hx
                    && point_xz.x <= pos.x + hx
                    && point_xz.y >= pos.z - hz
                    && point_xz.y <= pos.z + hz
                {
                    Some(top_y)
                } else {
                    None
                }
            }
        }
    }
}

/// 在给定的碰撞体集合中，查找玩家脚底正下方的最大地面高度。
/// 用于确定玩家是否着地，以及着地时应站立的 Y 坐标。
pub fn find_ground_y<F: QueryFilter>(
    collision_q: &Query<(&Transform, &CollisionShape), F>,
    player_xz: Vec2,
) -> f32 {
    let mut best = f32::NEG_INFINITY;
    // 遍历所有碰撞体，取该点正下方最高的地面高度
    for (t, shape) in collision_q.iter() {
        if let Some(h) = shape.ground_height_at(t, player_xz) {
            if h > best {
                best = h;
            }
        }
    }
    best
}

/// 将玩家实体推出任何重叠的立方体碰撞体。
/// 使用圆-vs-AABB 最近点算法处理水平碰撞。
/// `player_pos` 会被原地修改，`player_radius` 为玩家水平圆形半径，`player_height` 为玩家高度。
pub fn push_out_horizontal(
    collision_q: &Query<(&Transform, &CollisionShape), Without<Player>>,
    player_pos: &mut Vec3,
    player_radius: f32,
    player_height: f32,
) {
    for (t, shape) in collision_q.iter() {
        // 只处理立方体碰撞体，平面不参与水平推出
        let CollisionShape::Box { half_extents } = shape else { continue };
        let pos = t.translation;
        let s = t.scale;

        let hx = half_extents.x * s.x;
        let hz = half_extents.z * s.z;
        let hy = half_extents.y * s.y;

        // 先检查垂直方向是否重叠，若玩家完全在立方体上方或下方则跳过
        let player_bottom = player_pos.y;
        let player_top = player_pos.y + player_height;
        let box_bottom = pos.y - hy;
        let box_top = pos.y + hy;
        if player_top <= box_bottom || player_bottom >= box_top {
            continue;
        }

        // 计算立方体在 XZ 平面上的最小/最大坐标
        let box_min_x = pos.x - hx;
        let box_max_x = pos.x + hx;
        let box_min_z = pos.z - hz;
        let box_max_z = pos.z + hz;

        // 找到玩家圆心在 XZ 平面上的最近点（位于立方体矩形边界内）
        let closest_x = player_pos.x.clamp(box_min_x, box_max_x);
        let closest_z = player_pos.z.clamp(box_min_z, box_max_z);

        let dx = player_pos.x - closest_x;
        let dz = player_pos.z - closest_z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < player_radius * player_radius && dist_sq > f32::EPSILON {
            // 玩家圆与矩形边界相交，沿法线方向推出
            let dist = dist_sq.sqrt();
            let push = player_radius - dist;
            player_pos.x += dx / dist * push;
            player_pos.z += dz / dist * push;
        } else if dist_sq <= f32::EPSILON {
            // 玩家圆心在矩形内部，需要推到最近的边外面
            let overlap_x = hx - (player_pos.x - pos.x).abs();
            let overlap_z = hz - (player_pos.z - pos.z).abs();
            // 选择重叠较小的轴推出，并加上玩家半径
            if overlap_x < overlap_z {
                let sign = if player_pos.x > pos.x { 1.0 } else { -1.0 };
                player_pos.x += sign * (overlap_x + player_radius);
            } else {
                let sign = if player_pos.z > pos.z { 1.0 } else { -1.0 };
                player_pos.z += sign * (overlap_z + player_radius);
            }
        }
    }
}