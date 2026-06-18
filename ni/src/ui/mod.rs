//! UI 系统 — HUD、画廊
pub mod plugin;
pub mod image_gallery;

// 扁平化导出，避免 ui::plugin::GameUiPlugin 这种冗余路径
#[allow(unused_imports)]
pub use plugin::*;
#[allow(unused_imports)]
pub use image_gallery::*;
