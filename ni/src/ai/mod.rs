//! AI 系统 — 行为树、感知、寻路
pub mod plugin;
pub mod pathfinding;

// 扁平化导出，保持 crate::ai::AiState 等旧路径可用
pub use plugin::*;
