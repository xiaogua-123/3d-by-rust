//! 游戏空间网格 — 用于主游戏场景的空间查询和射线碰撞检测
//!
//! 自动同步带有 `LevelEntity` 标记的实体到 `SpatialGrid<Entity>`，
//! 供 AI 视线检测、NPC 障碍感知、玩家交互射线等模块使用。
//!
//! # 用法
//!
//! ```ignore
//! fn my_system(grid: Res<GameGridResource>) {
//!     // 查询某个点周围的所有实体
//!     let nearby = grid.query_radius(Vec2::new(0.0, 0.0), 5.0);
//!
//!     // 视线检测：从玩家到敌人是否有遮挡
//!     let visible = grid.has_line_of_sight(player_xz, enemy_xz, |id| *id == player_entity);
//! }
//! ```

use bevy::prelude::*;
use bevy::math::Vec2;
use std::collections::HashSet;
use std::collections::HashMap;

use crate::level::LevelEntity;
use crate::td::spatial::core::SpatialGrid;
use crate::td::spatial::entry::{EntityCategory, GridEntry};
use crate::td::spatial::filter::CategoryFilter;
use crate::ray_cast::{raycast_entities, sphere_hit_test, RayEntityHit};

/// 主游戏场景的空间网格资源
#[derive(Resource)]
pub struct GameGridResource {
    /// 底层空间哈希网格
    pub grid: SpatialGrid<Entity>,
    /// 追踪每个实体的位置，用于检测移动
    prev_positions: HashMap<Entity, Vec2>,
}

impl Default for GameGridResource {
    fn default() -> Self {
        Self {
            grid: SpatialGrid::new(4.0),
            prev_positions: HashMap::new(),
        }
    }
}

impl GameGridResource {
    /// 返回 tile 大小
    #[allow(dead_code)]
    pub fn tile_size(&self) -> f32 {
        self.grid.tile_size()
    }

    /// 按圆形范围查询实体
    #[allow(dead_code)]
    pub fn query_radius(&self, center: Vec2, radius: f32) -> Vec<&GridEntry<Entity>> {
        self.grid.query_radius(center, radius, CategoryFilter::All)
    }

    /// 从起点到终点是否有视线（无障碍物）
    ///
    /// `ignore` 回调返回 `true` 表示该实体不阻挡视线（如玩家自身）。
    pub fn has_line_of_sight<F>(&self, from: Vec2, to: Vec2, ignore: F) -> bool
    where
        F: Fn(&Entity) -> bool,
    {
        let dir = to - from;
        let max_dist = dir.length();
        if max_dist < f32::EPSILON {
            return true;
        }
        let dir = dir / max_dist;

        let mut seen = HashSet::new();
        let tile_size = self.grid.tile_size();

        for tile in crate::ray_cast::RayTileIter::new(from, dir, max_dist, tile_size) {
            let Some(entries) = self.grid.tiles.get(&tile) else {
                continue;
            };
            for entry in entries.iter() {
                if !seen.insert(&entry.id) || ignore(&entry.id) {
                    continue;
                }
                if sphere_hit_test(0.0)(entry, from, dir).is_some() {
                    return false;
                }
            }
        }
        true
    }

    /// 沿射线方向进行实体碰撞检测，返回最近命中
    #[allow(dead_code)]
    pub fn raycast(
        &self,
        origin: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Option<RayEntityHit<Entity>> {
        raycast_entities(&self.grid, origin, direction, max_distance, sphere_hit_test(0.0))
    }
}

/// 自动同步游戏实体到空间网格
fn sync_game_grid(
    mut game_grid: ResMut<GameGridResource>,
    q: Query<(Entity, &Transform), With<LevelEntity>>,
) {
    // 阶段 1：收集当前实体状态
    let mut current: HashMap<Entity, Vec2> = HashMap::new();
    let mut to_insert: Vec<(Entity, Vec2)> = Vec::new();
    let mut to_update: Vec<(Entity, Vec2, Vec2)> = Vec::new();

    for (entity, transform) in q.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        current.insert(entity, pos);

        match game_grid.prev_positions.get(&entity) {
            Some(old_pos) if *old_pos != pos => {
                to_update.push((entity, *old_pos, pos));
            }
            None => {
                to_insert.push((entity, pos));
            }
            _ => {}
        }
    }

    // 阶段 2：收集需要删除的实体
    let mut to_remove: Vec<(Entity, Vec2)> = Vec::new();
    for (entity, pos) in game_grid.prev_positions.iter() {
        if !current.contains_key(entity) {
            to_remove.push((*entity, *pos));
        }
    }

    // 阶段 3：批量执行网格变更（避免同时借用 game_grid 的多个字段）
    for (entity, pos) in to_insert {
        game_grid.grid.insert(GridEntry::new(
            entity,
            EntityCategory::Other,
            pos,
            0.5,
        ));
    }
    for (entity, old_pos, new_pos) in to_update {
        game_grid.grid.update(
            &entity,
            old_pos,
            new_pos,
            0.5,
            EntityCategory::Other,
        );
    }
    for (entity, pos) in to_remove {
        game_grid.grid.remove(&entity, pos, 0.5);
    }

    game_grid.prev_positions = current;
}

pub struct GameGridPlugin;

impl Plugin for GameGridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameGridResource>()
            .add_systems(Update, sync_game_grid);
    }
}
