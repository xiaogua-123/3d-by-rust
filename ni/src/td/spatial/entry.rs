//! 空间网格条目定义
//!
//! 定义 `EntityCategory`（分类）和 `GridEntry<T>`（带位置/半径的泛型条目）。

use bevy::math::{IVec2, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityCategory {
    Tower,
    Monster,
    #[allow(dead_code)]
    Projectile,
    Other,
}

#[derive(Clone, Debug)]
pub struct GridEntry<T> {
    pub id: T,
    pub category: EntityCategory,
    pub position: Vec2,
    pub radius: f32,
}

impl<T> GridEntry<T> {
    pub fn new(id: T, category: EntityCategory, position: Vec2, radius: f32) -> Self {
        Self { id, category, position, radius }
    }

    pub fn half_extent(&self) -> Vec2 {
        Vec2::splat(self.radius)
    }

    #[allow(dead_code)]
    pub fn tile_min(&self, tile_size: f32) -> IVec2 {
        ((self.position - self.half_extent()) / tile_size).floor().as_ivec2()
    }

    #[allow(dead_code)]
    pub fn tile_max(&self, tile_size: f32) -> IVec2 {
        ((self.position + self.half_extent()) / tile_size).floor().as_ivec2()
    }
}
