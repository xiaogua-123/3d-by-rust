//! NI 3D 潜行恐怖游戏 — 精简入口

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ni::core::log::configured_plugins;
use ni::network::plugin::NetworkPlugin;
use ni::{GamePlugins, ToolPlugins};

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((GamePlugins, ToolPlugins));
    app.add_plugins(EguiPlugin::default());
    app.add_systems(Update, ni::debug_switch_to_demo);
    #[cfg(debug_assertions)]
    app.add_plugins(
        WorldInspectorPlugin::new()
            .run_if(input_toggle_active(true, KeyCode::F3)),
    );
    app.run();
}
