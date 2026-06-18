//! 音频系统 — 音效、音乐
pub mod plugin;
pub mod music;

// 扁平化导出，避免 audio::plugin::GameAudioPlugin 这种冗余路径
#[allow(unused_imports)]
pub use plugin::*;
#[allow(unused_imports)]
pub use music::*;
