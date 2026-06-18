//! 射线碰撞检测模块
//!
//! 基于 DDA（Digital Differential Analyzer）网格遍历算法的高效射线检测。
//! 利用空间哈希网格作为 broad-phase，对命中实体做精确碰撞检测。
//!
//! # 来源
//!
//! 从 DigitalExtinction 项目的 `de_index::precise::segment`（DDA tile 遍历）
//! 和 `de_combat::sightline`（视线检测）适配而来。
//!
//! # 用法
//!
//! ```ignore
//! use crate::ray_cast::{raycast_entities, RayHit};
//!
//! fn my_system(grid: Res<TdGridResource>, ...) {
//!     let ray_origin = Vec2::new(0.0, 0.0);
//!     let ray_dir = Vec2::new(1.0, 0.0);
//!     let hit = raycast_entities(&grid.grid, ray_origin, ray_dir, 50.0, |entry, pos| {
//!         // 自定义碰撞检测回调：entry 是候选实体，pos 是实体位置
//!         // 返回 Some(toi) 或 None
//!         Some(entry.position.distance(ray_origin))
//!     });
//! }
//! ```

use bevy::math::{IVec2, Vec2};
use std::collections::HashSet;

use crate::td::spatial::core::SpatialGrid;
use crate::td::spatial::entry::GridEntry;

// ──────────────────────────────────────────────
// DDA 射线-网格遍历
// ──────────────────────────────────────────────

/// 沿射线方向步进遍历网格 tile 的迭代器
///
/// 使用 DDA（Digital Differential Analyzer）算法高效遍历射线经过的所有网格tile。
/// 源自 `de_index::precise::segment::TileIterator`。
///
/// 每次调用 `next()` 返回射线穿过的下一个 tile 坐标，按距离升序。
pub struct RayTileIter {
    /// 当前点在 2D 空间中的位置
    point: Vec2,
    /// 射线终点
    #[allow(dead_code)]
    stop: Vec2,
    /// 射线方向（单位向量）
    dir: Vec2,
    /// 上一个访问过的 tile 坐标
    last_tile: IVec2,
    /// 是否遍历完毕
    finished: bool,
    /// tile 边长
    tile_size: f32,
}

impl RayTileIter {
    /// 创建新的射线 tile 遍历迭代器
    ///
    /// # 参数
    ///
    /// * `origin` - 射线起点（2D 坐标）
    /// * `direction` - 射线方向（2D 向量，会被归一化）
    /// * `max_distance` - 最大射线长度
    /// * `tile_size` - 空间网格的 tile 边长
    pub fn new(origin: Vec2, direction: Vec2, max_distance: f32, tile_size: f32) -> Self {
        let dir = direction.normalize();
        let stop = origin + dir * max_distance;

        let mut point = origin;

        // 如果方向沿任何轴为负，第一个 tile 可能重复，修复此问题
        if origin != stop {
            let next = Self::next_tile_boundary(origin, dir, tile_size);
            if (next / tile_size).floor() == (origin / tile_size).floor() {
                point = next;
            }
        }

        Self {
            point,
            stop,
            dir,
            last_tile: (stop / tile_size).floor().as_ivec2(),
            finished: false,
            tile_size,
        }
    }

    /// 计算射线到达下一个 tile 边界的位置
    fn next_tile_boundary(point: Vec2, dir: Vec2, tile_size: f32) -> Vec2 {
        let current_tile = point / tile_size;

        // 计算下一个 tile 边界的坐标
        let next_tile_x = tile_size
            * if dir.x >= 0.0 {
                current_tile.x.floor() + 1.0
            } else {
                current_tile.x.ceil() - 1.0
            };
        let next_tile_y = tile_size
            * if dir.y >= 0.0 {
                current_tile.y.floor() + 1.0
            } else {
                current_tile.y.ceil() - 1.0
            };

        // 计算到达下一个 tile 边界的参数 t
        let factor_x = if dir.x == 0.0 {
            f32::INFINITY
        } else {
            (next_tile_x - point.x) / dir.x
        };
        let factor_y = if dir.y == 0.0 {
            f32::INFINITY
        } else {
            (next_tile_y - point.y) / dir.y
        };

        // 选择更近的边界
        if factor_x < factor_y {
            if factor_x >= 1.0 {
                point + dir
            } else {
                Vec2::new(next_tile_x, point.y + factor_x * dir.y)
            }
        } else if factor_y >= 1.0 {
            point + dir
        } else {
            Vec2::new(point.x + factor_y * dir.x, next_tile_y)
        }
    }
}

impl Iterator for RayTileIter {
    type Item = IVec2;

    fn next(&mut self) -> Option<IVec2> {
        if self.finished {
            return None;
        }

        let current_tile = (self.point / self.tile_size).floor().as_ivec2();
        if current_tile == self.last_tile {
            self.finished = true;
        } else {
            self.point = Self::next_tile_boundary(self.point, self.dir, self.tile_size);
        }

        Some(current_tile)
    }
}

// ──────────────────────────────────────────────
// 射线-实体碰撞检测
// ──────────────────────────────────────────────

/// 射线命中结果
#[derive(Debug, Clone)]
pub struct RayEntityHit<T> {
    /// 命中的实体 ID
    pub entity: T,
    /// 命中点到射线起点的距离
    pub distance: f32,
    /// 命中点位置
    pub hit_point: Vec2,
}

/// 对空间网格执行射线检测，返回第一个命中的实体
///
/// 使用 DDA 算法遍历射线经过的 tile，对每个 tile 中的候选实体
/// 调用 `hit_test` 回调进行精确碰撞检测。
///
/// # 参数
///
/// * `grid` - 空间哈希网格
/// * `origin` - 射线起点
/// * `direction` - 射线方向（会被归一化）
/// * `max_distance` - 最大检测距离
/// * `hit_test` - 碰撞检测回调 `fn(&GridEntry<T>, ray_origin, ray_dir) -> Option<f32>`
///   返回 `Some(toi)` 表示命中，`None` 表示未命中
///
/// # 返回值
///
/// 返回 `Option<RayEntityHit<T>>`，按距离最近优先
pub fn raycast_entities<T, F>(
    grid: &SpatialGrid<T>,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
    hit_test: F,
) -> Option<RayEntityHit<T>>
where
    T: Clone + Eq + std::hash::Hash,
    F: Fn(&GridEntry<T>, Vec2, Vec2) -> Option<f32>,
{
    // 零向量 → 无方向，直接返回
    if direction.length_squared() < f32::EPSILON {
        return None;
    }
    let dir = direction.normalize();

    let mut closest: Option<RayEntityHit<T>> = None;
    let mut seen = HashSet::new();
    let tile_size = grid.tile_size();

    for tile in RayTileIter::new(origin, dir, max_distance, tile_size) {
        let Some(entries) = grid.tiles.get(&tile) else {
            continue;
        };

        for entry in entries.iter() {
            if !seen.insert(&entry.id) {
                continue; // 已检测过
            }

            // 粗筛：实体在射线方向前面吗？
            let to_entity = entry.position - origin;
            let projection = to_entity.dot(dir);
            if projection < 0.0 || projection > max_distance {
                continue;
            }

            // 调用用户提供的精确碰撞检测
            if let Some(toi) = hit_test(entry, origin, dir) {
                let hit_point = origin + dir * toi;
                match &closest {
                    Some(current) if toi >= current.distance => {}
                    _ => {
                        closest = Some(RayEntityHit {
                            entity: entry.id.clone(),
                            distance: toi,
                            hit_point,
                        });
                    }
                }
            }
        }
    }

    closest
}

/// 对空间网格执行射线检测，返回所有命中的实体（按距离排序）
pub fn raycast_all_entities<T, F>(
    grid: &SpatialGrid<T>,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
    hit_test: F,
) -> Vec<RayEntityHit<T>>
where
    T: Clone + Eq + std::hash::Hash,
    F: Fn(&GridEntry<T>, Vec2, Vec2) -> Option<f32>,
{
    // 零向量 → 无方向，直接返回
    if direction.length_squared() < f32::EPSILON {
        return Vec::new();
    }

    let dir = direction.normalize();
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let tile_size = grid.tile_size();

    for tile in RayTileIter::new(origin, dir, max_distance, tile_size) {
        let Some(entries) = grid.tiles.get(&tile) else {
            continue;
        };

        for entry in entries.iter() {
            if !seen.insert(&entry.id) {
                continue;
            }

            let to_entity = entry.position - origin;
            let projection = to_entity.dot(dir);
            if projection < 0.0 || projection > max_distance {
                continue;
            }

            if let Some(toi) = hit_test(entry, origin, dir) {
                results.push(RayEntityHit {
                    entity: entry.id.clone(),
                    distance: toi,
                    hit_point: origin + dir * toi,
                });
            }
        }
    }

    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Less));
    results
}

/// 视线检测：判断从起点到终点是否有遮挡
///
/// 使用射线检测判断两点之间是否可见。
/// 如果射线命中了任何实体（排除 `ignore` 中的实体），返回 `false`。
///
/// # 参数
///
/// * `grid` - 空间哈希网格
/// * `from` - 起点
/// * `to` - 终点
/// * `ignore` - 要忽略的实体 ID 集合
/// * `hit_test` - 碰撞检测回调
pub fn has_line_of_sight<T, F>(
    grid: &SpatialGrid<T>,
    from: Vec2,
    to: Vec2,
    ignore: &HashSet<T>,
    hit_test: F,
) -> bool
where
    T: Clone + Eq + std::hash::Hash,
    F: Fn(&GridEntry<T>, Vec2, Vec2) -> Option<f32>,
{
    let direction = to - from;
    let max_distance = direction.length();
    if max_distance < f32::EPSILON {
        return true;
    }
    let dir = direction / max_distance;

    let mut seen = HashSet::new();
    let tile_size = grid.tile_size();

    for tile in RayTileIter::new(from, dir, max_distance, tile_size) {
        let Some(entries) = grid.tiles.get(&tile) else {
            continue;
        };

        for entry in entries.iter() {
            if !seen.insert(&entry.id) || ignore.contains(&entry.id) {
                continue;
            }

            if hit_test(entry, from, dir).is_some() {
                return false; // 有遮挡
            }
        }
    }

    true // 无遮挡
}

// ──────────────────────────────────────────────
// 辅助函数：球形碰撞体射线检测
// ──────────────────────────────────────────────

/// 球体射线检测回调工厂
///
/// 返回一个闭包，用于检测射线是否命中指定半径的球体。
pub fn sphere_hit_test<T: Clone>(radius: f32) -> impl Fn(&GridEntry<T>, Vec2, Vec2) -> Option<f32> {
    move |entry: &GridEntry<T>, origin: Vec2, dir: Vec2| {
        let to_center = entry.position - origin;
        let projection = to_center.dot(dir);

        // 最近点在射线上的位置
        let closest = origin + dir * projection;
        let dist_sq = closest.distance_squared(entry.position);

        let r = entry.radius + radius;
        if dist_sq <= r * r {
            // 计算实际命中距离
            let dx = (r * r - dist_sq).sqrt();
            let hit_dist = projection - dx;
            if hit_dist >= 0.0 {
                return Some(hit_dist);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::td::spatial::entry::EntityCategory;

    #[test]
    fn test_ray_tile_iter_straight() {
        let origin = Vec2::new(0.0, 0.0);
        let dir = Vec2::new(1.0, 0.0);
        let tiles: Vec<IVec2> = RayTileIter::new(origin, dir, 10.0, 4.0).collect();
        // 从 tile (0,0) 到 (2,0)
        assert!(!tiles.is_empty());
        assert_eq!(tiles[0], IVec2::new(0, 0));
    }

    #[test]
    fn test_ray_tile_iter_diagonal() {
        let origin = Vec2::new(0.0, 0.0);
        let dir = Vec2::new(1.0, 1.0);
        let tiles: Vec<IVec2> = RayTileIter::new(origin, dir, 8.0, 4.0).collect();
        // 沿对角线会穿过多个 tile
        assert!(tiles.len() >= 2);
    }

    #[test]
    fn test_raycast_hits_entity() {
        let mut grid = SpatialGrid::new(4.0);
        grid.insert(crate::td::spatial::entry::GridEntry::new(
            1u32,
            EntityCategory::Monster,
            Vec2::new(5.0, 0.0),
            0.5,
        ));
        grid.insert(crate::td::spatial::entry::GridEntry::new(
            2u32,
            EntityCategory::Monster,
            Vec2::new(100.0, 100.0),
            0.5,
        ));

        let hit = raycast_entities(
            &grid,
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            50.0,
            sphere_hit_test(0.0),
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().entity, 1);
    }

    #[test]
    fn test_raycast_misses() {
        let mut grid = SpatialGrid::new(4.0);
        grid.insert(crate::td::spatial::entry::GridEntry::new(
            1u32,
            EntityCategory::Monster,
            Vec2::new(100.0, 100.0),
            0.5,
        ));

        let hit = raycast_entities(
            &grid,
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            50.0,
            sphere_hit_test(0.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn test_line_of_sight_blocked() {
        let mut grid = SpatialGrid::new(4.0);
        grid.insert(crate::td::spatial::entry::GridEntry::new(
            1u32,
            EntityCategory::Tower,
            Vec2::new(5.0, 0.0),
            1.0,
        ));
        grid.insert(crate::td::spatial::entry::GridEntry::new(
            2u32,
            EntityCategory::Monster,
            Vec2::new(10.0, 0.0),
            0.5,
        ));

        let ignore = HashSet::from([1u32]);
        let visible = has_line_of_sight(
            &grid,
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            &ignore,
            sphere_hit_test(0.0),
        );
        assert!(!visible); // entity 1 blocks
    }
}
