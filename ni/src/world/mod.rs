//! 世界/场景 — 关卡、分块加载、网格、放置、导航网格、地形
pub mod level;
pub mod level_tool;
pub mod chunk;
pub mod grid;
pub mod placement;
pub mod nav_mesh;
pub mod terrain;
pub mod label;

// 向后兼容: 旧 world.rs 中的 WorldPlugin 现在在 terrain 模块中
pub use terrain::WorldPlugin;
