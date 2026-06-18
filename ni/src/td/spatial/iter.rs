//! 空间网格迭代器
//!
//! 提供 `TileRange`（网格瓦片坐标迭代）、`AabbIter`（AABB 范围迭代）、
//! `RadiusIter`（圆形范围迭代）三种遍历方式。

use std::collections::HashSet;
use bevy::math::{IVec2, Vec2};
use crate::td::spatial::core::SpatialGrid;
use crate::td::spatial::entry::GridEntry;

pub(super) struct TileRange {
    start: IVec2,
    end: IVec2,
    current_x: i32,
    current_y: i32,
    exhausted: bool,
}

impl TileRange {
    pub fn from_entry(entry: &GridEntry<impl Clone>, tile_size: f32) -> Self {
        let half = entry.half_extent();
        let min = ((entry.position - half) / tile_size).floor().as_ivec2();
        let max = ((entry.position + half) / tile_size).floor().as_ivec2();
        Self::new(min, max)
    }

    #[allow(dead_code)]
    pub fn from_aabb(min: Vec2, max: Vec2, tile_size: f32) -> Self {
        let start = (min / tile_size).floor().as_ivec2();
        let end = (max / tile_size).floor().as_ivec2();
        Self::new(start, end)
    }

    pub fn new(start: IVec2, end: IVec2) -> Self {
        Self {
            start,
            end,
            current_x: start.x,
            current_y: start.y,
            exhausted: start.x > end.x || start.y > end.y,
        }
    }

    pub fn from_circle(center: Vec2, radius: f32, tile_size: f32) -> Self {
        let min = (center - Vec2::splat(radius)) / tile_size;
        let max = (center + Vec2::splat(radius)) / tile_size;
        let start = min.floor().as_ivec2();
        let end = max.floor().as_ivec2();
        Self::new(start, end)
    }
}

impl Iterator for TileRange {
    type Item = IVec2;

    fn next(&mut self) -> Option<IVec2> {
        if self.exhausted {
            return None;
        }
        let current = IVec2::new(self.current_x, self.current_y);
        if self.current_x == self.end.x {
            if self.current_y == self.end.y {
                self.exhausted = true;
            } else {
                self.current_x = self.start.x;
                self.current_y += 1;
            }
        } else {
            self.current_x += 1;
        }
        Some(current)
    }
}

#[allow(dead_code)]
pub struct AabbIter<'a, T> {
    grid: &'a SpatialGrid<T>,
    tile_range: TileRange,
    seen: HashSet<&'a T>,
    current_tile_entities: std::vec::IntoIter<&'a GridEntry<T>>,
}

#[allow(dead_code)]
impl<'a, T: Clone + Eq + std::hash::Hash> AabbIter<'a, T> {
    pub fn new(grid: &'a SpatialGrid<T>, min: Vec2, max: Vec2) -> Self {
        let tile_range = TileRange::from_aabb(min, max, grid.tile_size);
        Self {
            grid,
            tile_range,
            seen: HashSet::new(),
            current_tile_entities: vec![].into_iter(),
        }
    }
}

impl<'a, T: Clone + Eq + std::hash::Hash> Iterator for AabbIter<'a, T> {
    type Item = &'a GridEntry<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(entry) = self.current_tile_entities.next() {
                if self.seen.insert(&entry.id) {
                    return Some(entry);
                }
                continue;
            }
            match self.tile_range.next() {
                Some(tile) => {
                    let entries: Vec<&'a GridEntry<T>> = self
                        .grid
                        .tiles
                        .get(&tile)
                        .map(|v| v.iter().collect())
                        .unwrap_or_default();
                    self.current_tile_entities = entries.into_iter();
                }
                None => return None,
            }
        }
    }
}

pub struct RadiusIter<'a, T> {
    grid: &'a SpatialGrid<T>,
    tile_range: TileRange,
    seen: HashSet<&'a T>,
    current_tile_entities: std::vec::IntoIter<&'a GridEntry<T>>,
    center: Vec2,
    radius_sq: f32,
}

impl<'a, T: Clone + Eq + std::hash::Hash> RadiusIter<'a, T> {
    pub fn new(grid: &'a SpatialGrid<T>, center: Vec2, radius: f32) -> Self {
        let tile_range = TileRange::from_circle(center, radius, grid.tile_size);
        Self {
            grid,
            tile_range,
            seen: HashSet::new(),
            current_tile_entities: vec![].into_iter(),
            center,
            radius_sq: radius * radius,
        }
    }
}

impl<'a, T: Clone + Eq + std::hash::Hash> Iterator for RadiusIter<'a, T> {
    type Item = &'a GridEntry<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(entry) = self.current_tile_entities.next() {
                if self.seen.insert(&entry.id)
                    && entry.position.distance_squared(self.center) <= self.radius_sq {
                        return Some(entry);
                    }
                continue;
            }
            match self.tile_range.next() {
                Some(tile) => {
                    let entries: Vec<&'a GridEntry<T>> = self
                        .grid
                        .tiles
                        .get(&tile)
                        .map(|v| v.iter().collect())
                        .unwrap_or_default();
                    self.current_tile_entities = entries.into_iter();
                }
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::td::spatial::entry::EntityCategory;

    #[test]
    fn test_tile_range_single_tile() {
        let range = TileRange::new(IVec2::new(0, 0), IVec2::new(0, 0));
        let tiles: Vec<IVec2> = range.collect();
        assert_eq!(tiles, vec![IVec2::new(0, 0)]);
    }

    #[test]
    fn test_tile_range_2x2() {
        let range = TileRange::new(IVec2::new(0, 0), IVec2::new(1, 1));
        let tiles: Vec<IVec2> = range.collect();
        assert_eq!(tiles.len(), 4);
        assert!(tiles.contains(&IVec2::new(0, 0)));
        assert!(tiles.contains(&IVec2::new(1, 0)));
        assert!(tiles.contains(&IVec2::new(0, 1)));
        assert!(tiles.contains(&IVec2::new(1, 1)));
    }

    #[test]
    fn test_tile_range_empty() {
        let range = TileRange::new(IVec2::new(1, 0), IVec2::new(0, 0));
        let tiles: Vec<IVec2> = range.collect();
        assert!(tiles.is_empty());
    }

    #[test]
    fn test_from_circle() {
        let range = TileRange::from_circle(Vec2::new(0.0, 0.0), 4.0, 4.0);
        let tiles: Vec<IVec2> = range.collect();
        assert_eq!(tiles.len(), 9);
    }

    #[test]
    fn test_from_entry() {
        let entry = GridEntry::new(1u32, EntityCategory::Monster, Vec2::new(0.0, 0.0), 1.0);
        let range = TileRange::from_entry(&entry, 4.0);
        let tiles: Vec<IVec2> = range.collect();
        assert!(!tiles.is_empty());
    }
}
