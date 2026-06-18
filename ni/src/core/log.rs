//! 日志和窗口配置模块
//!
//! 提供 `configured_plugins()` 函数，封装 Bevy 的 `DefaultPlugins`，
//! 根据编译模式（debug/release）自动设置日志级别和过滤器。
//! 窗口模式默认关闭垂直同步以允许最高帧率。

use bevy::{
    app::PluginGroup,
    log::{Level, LogPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

/// 编译期确定的日志过滤器：只显示项目自身日志，第三方库仅输出 warning/error
const LOG_FILTER: &str = if cfg!(debug_assertions) {
    "ni=debug,bevy=warn,bevy_gltf=error,error"
} else {
    "ni=info,bevy=warn,bevy_gltf=error,error"
};

/// 编译期确定的日志级别
const LOG_LEVEL: Level = if cfg!(debug_assertions) {
    Level::DEBUG
} else {
    Level::INFO
};

/// 返回已配置的 DefaultPlugins，仅输出项目自身日志，屏蔽 Bevy 引擎冗余
pub fn configured_plugins() -> impl PluginGroup {
    DefaultPlugins
        .set(LogPlugin {
            level: LOG_LEVEL,
            filter: LOG_FILTER.into(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        })
}