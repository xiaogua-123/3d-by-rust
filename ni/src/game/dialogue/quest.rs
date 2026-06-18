//! 对话系统 — 任务追踪与通知

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 子目标定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgoalDef {
    pub description: String,
    /// 当此 flag 被设置时，子目标视为完成
    pub completion_flag: Option<String>,
}

/// 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub subgoals: Vec<SubgoalDef>,
}

/// 任务定义库
#[derive(Resource, Default)]
pub struct QuestBank {
    pub quests: std::collections::HashMap<String, QuestDef>,
}

/// 任务追踪器
#[derive(Resource, Default)]
pub struct QuestTracker {
    pub completed_quests: Vec<String>,
    pub active_quests: Vec<String>,
    pub flags: Vec<String>,
}

/// 任务通知（任务开始/完成时弹窗）
#[derive(Resource, Default)]
pub struct QuestNotification {
    pub message: Option<String>,
    pub timer: f32,
}

/// 检测 QuestTracker 变化并生成通知
pub fn quest_notification_from_effects(
    mut notif: ResMut<QuestNotification>,
    quests: Res<QuestTracker>,
    quest_bank: Res<QuestBank>,
    mut last_active: Local<Vec<String>>,
    mut last_completed: Local<Vec<String>>,
) {
    for q in &quests.active_quests {
        if !last_active.contains(q) {
            let name = quest_bank
                .quests
                .get(q)
                .map_or(q.as_str(), |d| d.name.as_str());
            notif.message = Some(format!("新任务: {}", name));
            notif.timer = 4.0;
        }
    }
    for q in &quests.completed_quests {
        if !last_completed.contains(q) {
            let name = quest_bank
                .quests
                .get(q)
                .map_or(q.as_str(), |d| d.name.as_str());
            notif.message = Some(format!("任务完成: {}！", name));
            notif.timer = 4.0;
        }
    }
    *last_active = quests.active_quests.clone();
    *last_completed = quests.completed_quests.clone();
}

/// 自动清除任务通知
pub fn quest_notification_clear(time: Res<Time>, mut notif: ResMut<QuestNotification>) {
    if notif.message.is_some() {
        notif.timer -= time.delta_secs();
        if notif.timer <= 0.0 {
            notif.message = None;
            notif.timer = 0.0;
        }
    }
}
