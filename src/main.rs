use bevy::prelude::*;

use bevy_egui::EguiPlugin;
use bevy::input::common_conditions::input_toggle_active;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod animation;
mod audio;
mod camera;
mod collectible;
mod collision;
mod combat;
mod config;
mod dialogue;
mod enemy;
mod inventory;
mod npc;
mod game_state;
mod level;
mod log;
mod player;
mod toon;
mod ui;
mod network;
mod world;
mod debug_lighting;
mod particles;
mod td;

use animation::AnimationPlugin;
use audio::GameAudioPlugin;
use camera::CameraPlugin;
use collectible::CollectiblePlugin;
use combat::CombatPlugin;
use config::GameplayConfig;
use dialogue::DialoguePlugin;
use enemy::EnemyPlugin;
use game_state::GameStatePlugin;
use inventory::InventoryPlugin;
use level::LevelPlugin;
use npc::NpcPlugin;
use player::PlayerPlugin;
use toon::ToonPlugin;
use td::TdPlugin;
use ui::GameUiPlugin;
use world::WorldPlugin;
use particles::ParticlePlugin;

use log::configured_plugins;
use network::NetworkPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((
        ToonPlugin,
        GameStatePlugin,
        DialoguePlugin,
        InventoryPlugin,
        NpcPlugin,
        PlayerPlugin,
        AnimationPlugin,
        CameraPlugin,
        WorldPlugin,
        LevelPlugin,
        GameAudioPlugin,
        CollectiblePlugin,
        CombatPlugin,
        EnemyPlugin,
        GameUiPlugin,
    ));
    app.add_plugins((ParticlePlugin, TdPlugin));

    app.init_resource::<GameplayConfig>();
    app.register_type::<config::GameplayConfig>();
    app.register_type::<collision::CollisionShape>();
    app.init_resource::<debug_lighting::LightingDebug>();
    app.add_systems(PostUpdate, debug_lighting::sync_lighting_to_world);

    // Always add EguiPlugin since UI depends on it
    app.add_plugins(EguiPlugin::default());

    #[cfg(debug_assertions)]
    app.add_plugins(
        WorldInspectorPlugin::new()
            .run_if(input_toggle_active(true, KeyCode::F3)),
    );

    app.run();
}
