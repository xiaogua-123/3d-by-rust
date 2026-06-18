//! 网络模块
pub mod plugin;

// 扁平化导出，避免 network::plugin::NetworkPlugin 这种冗余路径
#[allow(unused_imports)]
pub use plugin::*;
