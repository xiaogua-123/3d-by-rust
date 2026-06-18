//! 对话系统 — 消息定义

use bevy::prelude::*;

/// 开始对话事件
#[derive(Message)]
pub struct StartDialogueEvent {
    pub conversation_id: String,
    pub start_node: String,
}

/// 玩家选择选项
#[derive(Message)]
pub struct DialogueChoiceEvent(pub usize);

/// 玩家推进对话
#[derive(Message)]
pub struct DialogueAdvanceEvent;
