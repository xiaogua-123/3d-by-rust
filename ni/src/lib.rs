//! NI 3D 潜行恐怖游戏 — 库入口
//!
//! 领域模块化架构：按游戏功能分组到 core/game/world/physics/render/ai/audio/
//! ui/network/assets/tools 等目录下。lib.rs 管理模块声明、re-export 和 PluginGroup。

// ═══ 领域模块声明 ═══
pub mod core;
pub mod game;
pub mod world;
pub mod physics;
pub mod render;
pub mod ai;
pub mod audio;
pub mod ui;
pub mod network;
pub mod assets;
pub mod tools;
pub mod td;

// ═══ Use 导入 ═══
use bevy::prelude::*;
use render::animation::AnimationPlugin;
use audio::plugin::GameAudioPlugin;
use render::camera::{CameraControllerPlugin, CameraPlugin};
use render::camera_motion::CameraMotionPlugin;
use game::collectible::CollectiblePlugin;
use physics::collision::manager::CollisionManagerPlugin;
use game::combat::CombatPlugin;
use core::config::GameplayConfig;
use core::save::SavePlugin;
use game::dialogue::DialoguePlugin;
use game::enemy::EnemyPlugin;
use core::game_state::GameStatePlugin;
use game::inventory::InventoryPlugin;
use world::chunk::ChunkPlugin;
use world::level::LevelPlugin;
use game::npc::NpcPlugin;
use game::player::PlayerPlugin;
use td::TdPlugin;
use ui::plugin::GameUiPlugin;
use world::WorldPlugin;
use render::particles::ParticlePlugin;
use render::scale::ScalePlugin;
use world::label::WorldLabelPlugin;
use tools::time_recorder::TimeRecorderPlugin;
use ui::image_gallery::ImageGalleryPlugin;
use tools::stress_test::StressTestPlugin;
use assets::entity_db::EntityDbPlugin;
use ai::pathfinding::PathfindingPlugin;
use game::puzzle::PuzzlePlugin;
use world::grid::GameGridPlugin;
use ai::plugin::AiPlugin;
use game::stealth::StealthPlugin;
use physics::collision::debug::CollisionDebugPlugin;
use assets::loading::LoadingPlugin;
use audio::music::MusicPlugin;
use world::placement::PlacementPlugin;
use assets::proximity_loader::ProximityLoaderPlugin;
use tools::creative::CreativePlugin;
use world::level_tool::LevelToolPlugin;

// ═══ 向后兼容 re-exports（保持 crate::X 路径可供其他模块使用） ═══
pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, chunk, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;
pub use physics::collision;                                // crate::collision
pub use physics::collision::collider as colliders;          // crate::colliders
pub use physics::collision::manager as collision_manager;   // crate::collision_manager
pub use physics::collision::debug as collision_debug;       // crate::collision_debug
pub use physics::ray_cast;
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::pathfinding;
pub use audio::music;
pub use ui::image_gallery;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};

// ═══ 复合插件：核心游戏插件 + 类型注册 / 系统 ═══

pub struct GamePlugins;

impl Plugin for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameStatePlugin, DialoguePlugin, InventoryPlugin,
            NpcPlugin, AnimationPlugin, PlayerPlugin,
            CameraPlugin, WorldPlugin, LevelPlugin,
            GameAudioPlugin, CollectiblePlugin, CombatPlugin,
            EnemyPlugin, GameUiPlugin, CollisionManagerPlugin,
        ));
        app.add_plugins((
            ParticlePlugin, TdPlugin, PuzzlePlugin, AiPlugin,
            GameGridPlugin, StealthPlugin, CollisionDebugPlugin,
            LoadingPlugin, ChunkPlugin,
        ));
        app.add_plugins((
            CameraMotionPlugin, WorldLabelPlugin, ScalePlugin,
            CameraControllerPlugin, SavePlugin,
        ));

        // 类型注册和资源
        app.init_resource::<GameplayConfig>();
        app.register_type::<GameplayConfig>();
        app.register_type::<physics::collision::collider::Collider>();
        app.register_type::<physics::collision::collider::ColliderShape>();
        app.register_type::<physics::collision::collider::CollisionLayer>();
        app.register_type::<physics::collision::collider::CollisionMask>();
        app.register_type::<physics::collision::collider::SmoothPush>();
        app.init_resource::<render::debug_lighting::LightingDebug>();
        app.add_systems(PostUpdate, render::debug_lighting::sync_lighting_to_world);
        app.add_systems(Update, render::render_utils::animate_rotation);
    }
}

// ═══ 复合插件：调试 / 工具插件 ═══

pub struct ToolPlugins;

impl Plugin for ToolPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TimeRecorderPlugin, ImageGalleryPlugin,
            EntityDbPlugin, StressTestPlugin, PathfindingPlugin,
            MusicPlugin, PlacementPlugin, ProximityLoaderPlugin,
            LevelToolPlugin, CreativePlugin,
        ));
    }
}

// ═══ 调试快捷键：F5 → 跳转到 Demo 关卡 ═══

pub fn debug_switch_to_demo(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<crate::world::level::LoadLevelEvent>,
    mut phase: ResMut<NextState<crate::core::game_state::GamePhase>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        info!("[Debug] F5 → 跳转到 Demo 关卡");
        phase.set(crate::core::game_state::GamePhase::Playing);
        events.write(crate::world::level::LoadLevelEvent {
            level: crate::world::level::GameLevel::Demo,
        });
    }
}
