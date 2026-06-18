//! 创造模式 — 类似 Minecraft 创造模式的 3D 关卡编辑器
//!
//! F6 切换进入/退出，支持飞行放置/删除物体、保存到 RON。
//! 复用 CameraController 做飞行，复用 EntityRegistry 做物品来源。

mod state;
mod systems;
mod ui;

pub use state::*;
pub use systems::*;

use bevy::prelude::*;
use crate::game_state::GamePhase;

pub struct CreativePlugin;

impl Plugin for CreativePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeState>()
            .add_systems(OnEnter(GamePhase::Creative), enter_creative)
            .add_systems(OnExit(GamePhase::Creative), exit_creative)
            .add_systems(Update, (
                creative_toggle,
                creative_scroll.run_if(in_state(GamePhase::Creative)),
                creative_ghost.run_if(in_state(GamePhase::Creative)),
                creative_placement.run_if(in_state(GamePhase::Creative)),
                creative_remove.run_if(in_state(GamePhase::Creative)),
                creative_save.run_if(in_state(GamePhase::Creative)),
                toggle_grid_snap.run_if(in_state(GamePhase::Creative)),
                toggle_labels.run_if(in_state(GamePhase::Creative)),
                toggle_level_visibility.run_if(in_state(GamePhase::Creative)),
            ))
            .add_systems(Update, (
                ui::creative_hotbar_ui.run_if(in_state(GamePhase::Creative)),
            ));
    }
}
