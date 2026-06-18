//! 对话系统 — 数据结构定义

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 对话节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    pub speaker: String,
    pub text: String,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub choices: Vec<DialogueChoice>,
    #[serde(default)]
    pub on_enter: Vec<DialogueEffect>,
}

/// 对话选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub text: String,
    pub next_id: String,
    #[serde(default)]
    pub condition: Option<DialogueCondition>,
}

/// 对话触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueCondition {
    HasItem(String),
    NoItem(String),
    QuestComplete(String),
    QuestActive(String),
    Flag(String),
    HasVisitedZone(String),
}

/// 对话效果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueEffect {
    GiveItem(String, u32),
    RemoveItem(String, u32),
    SetFlag(String),
    CompleteQuest(String),
    StartQuest(String),
    StartPuzzle(String),
    UnlockDoor(String),
    PlayCutscene(String),
}

/// apply_effects 返回的待处理效果（需要事件 writer）
#[derive(Debug)]
pub enum PendingEffect {
    GiveItem(String, u32),
    RemoveItem(String, u32),
}

/// 完整对话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueConversation {
    pub id: String,
    pub nodes: HashMap<String, DialogueNode>,
}

/// 对话触发器组件（供 NPC 使用）
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct DialogueTrigger {
    pub conversation_id: String,
    pub start_node: String,
    pub radius: f32,
}

/// 对话库资源
#[derive(Resource, Default)]
pub struct DialogueBank {
    pub conversations: HashMap<String, DialogueConversation>,
}
