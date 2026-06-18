//! 导航网格桥接模块
//!
//! 将 de_pathing 寻路库（Polyanya 算法）封装为 Bevy Resource，
//! 用于 TD 模式的导航网格寻路。

use bevy::prelude::*;
use de_pathing::{ExclusionArea, MapBounds, PathFinder, PathQueryProps, PathTarget, triangulate};
use glam::Vec2;
use parry2d::math::Point;
use parry2d::shape::ConvexPolygon;

/// 障碍物内边距（考虑实体半径 + 安全余量）
const OBSTACLE_PADDING: f32 = 0.8;

/// 导航网格资源 — 基于 de_pathing 的 Polyanya 寻路器
#[derive(Resource)]
pub struct TdNavMesh {
    finder: PathFinder,
}

impl TdNavMesh {
    /// 从关卡定义构建导航网格
    pub fn build(arena_size: f32, obstacles: &[crate::td::level_data::ObstacleDef]) -> Self {
        let half = arena_size / 2.0;

        let exclusions: Vec<ExclusionArea> = obstacles
            .iter()
            .filter_map(|obs| {
                let (px, _, pz) = obs.position;
                let (sx, _, sz) = obs.scale;
                let hx = sx / 2.0 + OBSTACLE_PADDING;
                let hz = sz / 2.0 + OBSTACLE_PADDING;

                let left = (px - hx).max(-half);
                let right = (px + hx).min(half);
                let bottom = (pz - hz).max(-half);
                let top = (pz + hz).min(half);

                if right <= left || top <= bottom {
                    return None;
                }

                ConvexPolygon::from_convex_hull(&[
                    Point::new(left, bottom),
                    Point::new(right, bottom),
                    Point::new(right, top),
                    Point::new(left, top),
                ])
                .map(ExclusionArea::new)
            })
            .collect();

        let bounds = MapBounds::new(Vec2::new(arena_size, arena_size));
        let triangles = triangulate(&bounds, &exclusions);

        info!(
            "TdNavMesh: {} 三角形, {} 排他区域",
            triangles.len(),
            exclusions.len(),
        );

        Self {
            finder: PathFinder::from_triangles(triangles, vec![]),
        }
    }

    /// 在 XZ 平面上寻路，返回 3D 路径点列表
    ///
    /// 返回的路径点顺序：起点 → ... → 终点。
    /// 如果两点间不可达，返回 `None`。
    pub fn find_path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let from_2d = Vec2::new(from.x, from.z);
        let to_2d = Vec2::new(to.x, to.z);

        let path = self
            .finder
            .find_path(from_2d, PathTarget::new(to_2d, PathQueryProps::exact(), false))?;

        // de_pathing 的 waypoints: [0]=终点, [last]=起点
        // 反转后: [0]=起点, [last]=终点
        let waypoints: Vec<Vec3> = path
            .waypoints()
            .iter()
            .rev()
            .map(|&wp| Vec3::new(wp.x, from.y, wp.y))
            .collect();

        Some(waypoints)
    }
}
