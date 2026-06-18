use bevy::math::{IVec2, Vec2};
use std::collections::HashMap;
use crate::td::spatial::entry::{EntityCategory, GridEntry};
use crate::td::spatial::filter::CategoryFilter;
use crate::td::spatial::iter::{AabbIter, RadiusIter, TileRange};

#[derive(Clone)]
pub struct SpatialGrid<T> {
    pub(crate) tile_size: f32,
    pub(crate) tiles: HashMap<IVec2, Vec<GridEntry<T>>>,
    count: usize,
}

impl<T: Clone + Eq + std::hash::Hash> SpatialGrid<T> {
    pub fn new(tile_size: f32) -> Self {
        assert!(tile_size > 0.0, "tile_size 必须大于 0");
        Self { tile_size, tiles: HashMap::new(), count: 0 }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize { self.count }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool { self.count == 0 }
    #[allow(dead_code)]
    pub fn tile_size(&self) -> f32 { self.tile_size }

    pub fn insert(&mut self, entry: GridEntry<T>) {
        for tile in TileRange::from_entry(&entry, self.tile_size) {
            self.tiles.entry(tile).or_default().push(entry.clone());
        }
        self.count += 1;
    }

    pub fn remove(&mut self, id: &T, position: Vec2, radius: f32) {
        let entry = GridEntry::new(id.clone(), EntityCategory::Other, position, radius);
        for tile in TileRange::from_entry(&entry, self.tile_size) {
            self.remove_from_tile(id, tile);
        }
        self.count -= 1;
    }

    pub fn update(
        &mut self,
        id: &T,
        old_pos: Vec2,
        new_pos: Vec2,
        radius: f32,
        category: EntityCategory,
    ) {
        let old_entry = GridEntry::new(id.clone(), category, old_pos, radius);
        let new_entry = GridEntry::new(id.clone(), category, new_pos, radius);
        let old_tiles: Vec<IVec2> = TileRange::from_entry(&old_entry, self.tile_size).collect();
        let new_tiles: Vec<IVec2> = TileRange::from_entry(&new_entry, self.tile_size).collect();

        if old_tiles == new_tiles {
            return;
        }

        for tile in &old_tiles {
            if !new_tiles.contains(tile) {
                self.remove_from_tile(id, *tile);
            }
        }
        for tile in &new_tiles {
            if !old_tiles.contains(tile) {
                self.tiles.entry(*tile).or_default().push(new_entry.clone());
            }
        }
    }

    #[allow(dead_code)]
    pub fn batch_update(&mut self, updates: &[(T, Vec2, Vec2, f32, EntityCategory)]) {
        for (id, old_pos, new_pos, radius, category) in updates {
            self.update(id, *old_pos, *new_pos, *radius, *category);
        }
    }

    #[allow(dead_code)]
    pub fn query_radius(
        &self,
        center: Vec2,
        radius: f32,
        filter: CategoryFilter,
    ) -> Vec<&GridEntry<T>> {
        let radius_sq = radius * radius;
        let mut results: Vec<&GridEntry<T>> = RadiusIter::new(self, center, radius)
            .filter(|entry| {
                filter.matches(&entry.category)
                    && entry.position.distance_squared(center) <= radius_sq
            })
            .collect();
        results.sort_by(|a, b| {
            a.position
                .distance_squared(center)
                .partial_cmp(&b.position.distance_squared(center))
                .unwrap_or(std::cmp::Ordering::Less)
        });
        results
    }

    #[allow(dead_code)]
    pub fn query_radius_ids(&self, center: Vec2, radius: f32, filter: CategoryFilter) -> Vec<T>
    where T: Copy {
        let radius_sq = radius * radius;
        let mut results = Vec::new();
        for entry in RadiusIter::new(self, center, radius) {
            if filter.matches(&entry.category)
                && entry.position.distance_squared(center) <= radius_sq
            {
                results.push(entry.id);
            }
        }
        results
    }

    #[allow(dead_code)]
    pub fn query_aabb(&self, min: Vec2, max: Vec2, filter: CategoryFilter) -> Vec<&GridEntry<T>> {
        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();
        for entry in AabbIter::new(self, min, max) {
            if filter.matches(&entry.category) && seen.insert(&entry.id as *const T) {
                results.push(entry);
            }
        }
        results
    }

    #[allow(dead_code)]
    pub fn query_neighbors(
        &self,
        id: &T,
        position: Vec2,
        range: f32,
        filter: CategoryFilter,
    ) -> Vec<&GridEntry<T>> {
        let radius_sq = range * range;
        let mut results: Vec<&GridEntry<T>> = RadiusIter::new(self, position, range)
            .filter(|entry| {
                entry.id != *id
                    && filter.matches(&entry.category)
                    && entry.position.distance_squared(position) <= radius_sq
            })
            .collect();
        results.sort_by(|a, b| {
            a.position
                .distance_squared(position)
                .partial_cmp(&b.position.distance_squared(position))
                .unwrap_or(std::cmp::Ordering::Less)
        });
        results
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tiles.clear();
        self.count = 0;
    }

    fn remove_from_tile(&mut self, id: &T, tile: IVec2) {
        let Some(entries) = self.tiles.get_mut(&tile) else {
            // tile 不存在说明实体跟踪已不同步，优雅跳过而非 panic
            return;
        };
        if let Some(pos) = entries.iter().position(|e| e.id == *id) {
            entries.swap_remove(pos);
        }
        if entries.is_empty() {
            self.tiles.remove(&tile);
        }
    }
}

impl<T: Clone + Eq + std::hash::Hash> Default for SpatialGrid<T> {
    fn default() -> Self {
        Self::new(4.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid_is_empty() {
        let grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        assert_eq!(grid.count, 0);
    }

    #[test]
    fn test_insert_single() {
        let mut grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        let entry = GridEntry::new(1, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0);
        grid.insert(entry);
        assert_eq!(grid.count, 1);
    }

    #[test]
    fn test_insert_and_remove() {
        let mut grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        grid.insert(GridEntry::new(1, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0));
        assert_eq!(grid.count, 1);
        grid.remove(&1, Vec2::new(0.0, 0.0), 1.0);
        assert_eq!(grid.count, 0);
    }

    #[test]
    fn test_update_moves_tile() {
        let mut grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        grid.insert(GridEntry::new(1, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0));
        grid.update(&1, Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0), 1.0, EntityCategory::Monster);
        assert_eq!(grid.count, 1);
    }

    #[test]
    fn test_query_radius_finds_nearby() {
        let mut grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        grid.insert(GridEntry::new(1, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0));
        grid.insert(GridEntry::new(2, EntityCategory::Monster, Vec2::new(3.0, 0.0), 1.0));
        grid.insert(GridEntry::new(3, EntityCategory::Tower, Vec2::new(0.0, 5.0), 1.0));
        let results = grid.query_radius(Vec2::ZERO, 5.0, CategoryFilter::monster_only());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_radius_excludes_out_of_range() {
        let mut grid: SpatialGrid<u32> = SpatialGrid::new(4.0);
        grid.insert(GridEntry::new(1, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0));
        grid.insert(GridEntry::new(2, EntityCategory::Monster, Vec2::new(100.0, 100.0), 1.0));
        let results = grid.query_radius(Vec2::ZERO, 5.0, CategoryFilter::monster_only());
        assert_eq!(results.len(), 1);
    }
}
