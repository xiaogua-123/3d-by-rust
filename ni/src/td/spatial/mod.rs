//! 空间网格系统 — 塔防模式的高效空间查询
//!
//! 基于网格哈希的空间分区，提供快速的范围查询、邻居查找和碰撞检测。
//! 用于炮塔索敌、敌人同步和射线检测。

pub mod core;
pub mod entry;
pub mod filter;
pub mod iter;

pub mod integration;

#[allow(unused_imports)]
pub mod prelude {
    pub use crate::td::spatial::core::SpatialGrid;
    pub use crate::td::spatial::entry::EntityCategory;
    pub use crate::td::spatial::filter::CategoryFilter;
}
