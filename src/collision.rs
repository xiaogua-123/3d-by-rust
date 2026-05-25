use bevy::prelude::*;
use bevy::ecs::query::QueryFilter;
use crate::player::Player;

#[derive(Component, Clone, Reflect)]
pub enum CollisionShape {
    Plane { y: f32 },
    Box { half_extents: Vec3 },
}

impl CollisionShape {
    pub fn ground_height_at(&self, transform: &Transform, point_xz: Vec2) -> Option<f32> {
        match self {
            CollisionShape::Plane { y } => Some(*y),
            CollisionShape::Box { half_extents } => {
                let pos = transform.translation;
                let s = transform.scale;
                let hx = half_extents.x * s.x;
                let hz = half_extents.z * s.z;
                let top_y = pos.y + half_extents.y * s.y;

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

pub fn find_ground_y<F: QueryFilter>(
    collision_q: &Query<(&Transform, &CollisionShape), F>,
    player_xz: Vec2,
) -> f32 {
    let mut best = f32::NEG_INFINITY;
    for (t, shape) in collision_q.iter() {
        if let Some(h) = shape.ground_height_at(t, player_xz) {
            if h > best {
                best = h;
            }
        }
    }
    best
}

/// Pushes `player_pos` out of any overlapping box colliders in the XZ plane,
/// using circle-vs-AABB closest-point algorithm.
pub fn push_out_horizontal(
    collision_q: &Query<(&Transform, &CollisionShape), Without<Player>>,
    player_pos: &mut Vec3,
    player_radius: f32,
    player_height: f32,
) {
    for (t, shape) in collision_q.iter() {
        let CollisionShape::Box { half_extents } = shape else { continue };
        let pos = t.translation;
        let s = t.scale;

        let hx = half_extents.x * s.x;
        let hz = half_extents.z * s.z;
        let hy = half_extents.y * s.y;

        // Check vertical overlap
        let player_bottom = player_pos.y;
        let player_top = player_pos.y + player_height;
        let box_bottom = pos.y - hy;
        let box_top = pos.y + hy;
        if player_top <= box_bottom || player_bottom >= box_top {
            continue;
        }

        // Closest point on box XZ rectangle to player XZ center
        let box_min_x = pos.x - hx;
        let box_max_x = pos.x + hx;
        let box_min_z = pos.z - hz;
        let box_max_z = pos.z + hz;

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
            // Player center is inside the box in XZ — push to nearest edge
            let overlap_x = hx - (player_pos.x - pos.x).abs();
            let overlap_z = hz - (player_pos.z - pos.z).abs();
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
