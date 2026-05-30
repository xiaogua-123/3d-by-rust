use bevy::prelude::*;

use bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod animation;
mod audio;
mod camera;
mod collectible;
mod collision;
mod colliders;
mod collision_manager;
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
mod ui;
mod network;
mod world;
mod debug_lighting;
mod particles;
mod solari_demo;
mod td;

use animation::AnimationPlugin;
use audio::GameAudioPlugin;
use camera::{CameraControllerPlugin, CameraPlugin};
use collectible::CollectiblePlugin;
use collision_manager::CollisionManagerPlugin;
use combat::CombatPlugin;
use config::GameplayConfig;
use dialogue::DialoguePlugin;
use enemy::EnemyPlugin;
use game_state::GameStatePlugin;
use inventory::InventoryPlugin;
use level::LevelPlugin;
use npc::NpcPlugin;
use player::PlayerPlugin;
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
        CollisionManagerPlugin,
    ));
    app.add_plugins((ParticlePlugin, TdPlugin));

    // Solari 光追渲染在进入 Solari 关卡时按需激活
    // CameraControllerPlugin 提供通用自由视角（WASD + 鼠标），关卡6使用
    app.add_plugins(CameraControllerPlugin);

    app.init_resource::<GameplayConfig>();
    app.register_type::<config::GameplayConfig>();
    app.register_type::<collision::CollisionShape>();
    app.register_type::<colliders::Collider>();
    app.register_type::<colliders::ColliderShape>();
    app.register_type::<colliders::CollisionLayer>();
    app.register_type::<colliders::CollisionMask>();
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
