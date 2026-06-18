//! 碰撞检测系统
pub mod collider;
pub mod manager;
pub mod debug;

// 向后兼容 — 将子模块的所有 pub 项提升到 collision 命名空间
pub use collider::*;
pub use manager::*;
pub use debug::*;
