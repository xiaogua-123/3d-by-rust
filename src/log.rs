use bevy::{
    app::PluginGroup,
    log::{Level, LogPlugin, DEFAULT_FILTER},
    prelude::*,
};

/// 返回一个已配置的 DefaultPlugins，其中 LogPlugin 已根据编译模式设置
pub fn configured_plugins() -> impl PluginGroup {
    let log_plugin = {
        let level = if cfg!(debug_assertions) {
            Level::DEBUG
        } else {
            Level::INFO
        };

        // 过滤规则：Bevy 默认抑制 + 本项目 debug + 关闭 GPU 分配器的刷屏日志
        let filter = format!(
            "ni={},{},offset_allocator=error",
            if cfg!(debug_assertions) { "debug" } else { "info" },
            DEFAULT_FILTER,
        );

        LogPlugin {
            level,
            filter,
            ..default()
        }
    };

    DefaultPlugins.set(log_plugin)
}
