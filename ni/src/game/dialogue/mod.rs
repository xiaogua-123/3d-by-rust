//! 对话系统 — RON 驱动的分支对话与任务管理
//!
//! 基于 RON 文件定义的节点式对话树，支持：
//! - 对话分支、条件判断、事件触发
//! - 任务追踪（`QuestTracker`）与通知（`QuestNotification`）
//! - 打字机效果文本、NPC 对话触发器
//! - 完整的有限状态机控制对话流程

mod types;
mod events;
mod branch;
mod loader;
mod quest;
mod systems;
mod ui;

pub use types::*;
pub use events::*;
pub use quest::*;
pub use systems::*;

use bevy::prelude::*;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DialogueTrigger>()
            .init_resource::<DialogueBank>()
            .init_resource::<QuestTracker>()
            .init_resource::<QuestBank>()
            .init_resource::<QuestNotification>()
            .init_resource::<DialogueManager>()
            .add_message::<StartDialogueEvent>()
            .add_message::<DialogueChoiceEvent>()
            .add_message::<DialogueAdvanceEvent>()
            .add_systems(Startup, (loader::load_dialogues, loader::load_quests))
            .add_systems(
                Update,
                (
                    handle_start_dialogue,
                    handle_dialogue_choice,
                    handle_dialogue_advance,
                    dialogue_input,
                    ui::dialogue_ui.run_if(dialogue_visible),
                    typewriter_tick.run_if(dialogue_visible),
                    quest_notification_from_effects,
                    quest_notification_clear,
                ),
            );
    }
}
