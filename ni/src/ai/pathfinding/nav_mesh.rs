//! NavMesh 桥接模块 — 封装 vleue_navigator 的 NavMesh 为 Bevy 资源
//!
//! 提供 `AiNavMesh` 资源和 `create_rect_navmesh()` 辅助函数。

use bevy::prelude::*;
use vleue_navigator::NavMesh;

/// NavMesh 资源 — 持有 Handle 以便在系统中通过 `Assets<NavMesh>` 访问
#[derive(Resource, Default)]
pub struct AiNavMesh {
    pub handle: Handle<NavMesh>,
}

/// 创建一个矩形 NavMesh（无内部障碍物）
pub fn create_rect_navmesh(
    nav_meshes: &mut Assets<NavMesh>,
    center: Vec3,
    size: f32,
) -> Handle<NavMesh> {
    let half = size / 2.0;
    let edges = vec![
        Vec2::new(center.x - half, center.z - half),
        Vec2::new(center.x + half, center.z - half),
        Vec2::new(center.x + half, center.z + half),
        Vec2::new(center.x - half, center.z + half),
    ];
    let mesh = NavMesh::from_edge_and_obstacles(edges, vec![]);
    nav_meshes.add(mesh)
}
