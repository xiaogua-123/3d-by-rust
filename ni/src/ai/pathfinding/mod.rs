//! 寻路系统 — 基于 NavMesh (vleue_navigator) 的 NPC 自动寻路模块
//!
//! 实体挂载 `Navigator` + `NavTarget` 后自动计算并沿路径移动。
//! 底层使用 Polyanya 算法在导航网格上做任意角度寻路，
//! 支持路径扰动（自然行走效果）。

use bevy::prelude::*;
use vleue_navigator::VleueNavigatorPlugin;

mod components;
mod systems;
mod nav_mesh;

pub use components::*;
pub use systems::*;
pub use nav_mesh::*;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VleueNavigatorPlugin)
            .init_resource::<AiNavMesh>()
            .add_message::<RequestPathEvent>()
            .add_systems(Update, (
                detect_nav_target,
                compute_path.after(detect_nav_target),
                follow_nav_path.after(compute_path),
            ));
    }
}
