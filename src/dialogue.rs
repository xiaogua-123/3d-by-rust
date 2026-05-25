use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use crate::game_state::GamePhase;

// ═══════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub text: String,
    pub next_id: String,
    #[serde(default)]
    pub condition: Option<DialogueCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueCondition {
    HasItem(String),
    NoItem(String),
    QuestComplete(String),
    QuestActive(String),
    Flag(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueEffect {
    GiveItem(String, u32),
    RemoveItem(String, u32),
    SetFlag(String),
    CompleteQuest(String),
    StartQuest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueConversation {
    pub id: String,
    pub nodes: HashMap<String, DialogueNode>,
}

// ═══════════════════════════════════════════
// 资源
// ═══════════════════════════════════════════

#[derive(Resource, Default)]
pub struct DialogueBank {
    pub conversations: HashMap<String, DialogueConversation>,
}

#[derive(Resource, Default)]
pub struct QuestTracker {
    pub completed_quests: Vec<String>,
    pub active_quests: Vec<String>,
    pub flags: Vec<String>,
}

#[derive(Resource, Default)]
pub struct DialogueManager {
    pub active_conversation_id: Option<String>,
    pub current_node_id: Option<String>,
    pub display_text: String,
    pub char_index: usize,
    pub text_timer: Timer,
    pub text_complete: bool,
    pub visible: bool,
    /// 关卡设计调试：点击文字切换显示节点内部状态
    pub debug_visible: bool,
}

// ═══════════════════════════════════════════
// 消息
// ═══════════════════════════════════════════

#[derive(Message)]
pub struct StartDialogueEvent {
    pub conversation_id: String,
    pub start_node: String,
}

#[derive(Message)]
pub struct DialogueChoiceEvent(pub usize);

#[derive(Message)]
pub struct DialogueAdvanceEvent;

// ═══════════════════════════════════════════
// 组件（供 NPC 使用）
// ═══════════════════════════════════════════

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct DialogueTrigger {
    pub conversation_id: String,
    pub start_node: String,
    pub radius: f32,
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DialogueTrigger>()
            .init_resource::<DialogueBank>()
            .init_resource::<QuestTracker>()
            .init_resource::<DialogueManager>()
            .add_message::<StartDialogueEvent>()
            .add_message::<DialogueChoiceEvent>()
            .add_message::<DialogueAdvanceEvent>()
            .add_systems(Startup, load_dialogues)
            .add_systems(
                Update,
                (
                    handle_start_dialogue,
                    handle_dialogue_choice,
                    handle_dialogue_advance,
                    dialogue_input,
                    dialogue_ui.run_if(dialogue_visible),
                    typewriter_tick.run_if(dialogue_visible),
                ),
            );
    }
}

// ═══════════════════════════════════════════
// 加载对话文件
// ═══════════════════════════════════════════

fn load_dialogues(mut bank: ResMut<DialogueBank>) {
    let dir = "assets/dialogue";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                match fs::File::open(&path) {
                    Ok(file) => match from_reader::<_, DialogueConversation>(file) {
                        Ok(conv) => {
                            info!("加载对话: {}", conv.id);
                            bank.conversations.insert(conv.id.clone(), conv);
                        }
                        Err(e) => {
                            error!("解析对话文件失败 {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        error!("打开对话文件失败 {:?}: {}", path, e);
                    }
                }
            }
        }
    } else {
        // 创建目录和示例对话
        let _ = fs::create_dir_all(dir);
        error!("对话目录不存在，已创建 assets/dialogue/，请放入 .ron 对话文件");
    }
    info!("共加载 {} 个对话", bank.conversations.len());
}

// ═══════════════════════════════════════════
// 条件/效果执行
// ═══════════════════════════════════════════

impl DialogueCondition {
    pub fn check(&self, quests: &QuestTracker) -> bool {
        match self {
            DialogueCondition::HasItem(_) => true,  // 等背包系统实现
            DialogueCondition::NoItem(_) => true,
            DialogueCondition::QuestComplete(id) => quests.completed_quests.contains(id),
            DialogueCondition::QuestActive(id) => quests.active_quests.contains(id),
            DialogueCondition::Flag(f) => quests.flags.contains(f),
        }
    }
}

fn apply_effects(effects: &[DialogueEffect], quests: &mut QuestTracker) {
    for effect in effects {
        match effect {
            DialogueEffect::StartQuest(id) => {
                if !quests.active_quests.contains(id) {
                    quests.active_quests.push(id.clone());
                    info!("任务开始: {}", id);
                }
            }
            DialogueEffect::CompleteQuest(id) => {
                quests.active_quests.retain(|q| q != id);
                if !quests.completed_quests.contains(id) {
                    quests.completed_quests.push(id.clone());
                }
                info!("任务完成: {}", id);
            }
            DialogueEffect::SetFlag(f) => {
                if !quests.flags.contains(f) {
                    quests.flags.push(f.clone());
                }
            }
            DialogueEffect::GiveItem(_, _) | DialogueEffect::RemoveItem(_, _) => {
                // 等背包系统实现
            }
        }
    }
}

// ═══════════════════════════════════════════
// 对话状态机
// ═══════════════════════════════════════════

fn dialogue_visible(manager: Res<DialogueManager>) -> bool {
    manager.visible
}

fn handle_start_dialogue(
    mut events: MessageReader<StartDialogueEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    for ev in events.read() {
        if let Some(conv) = bank.conversations.get(&ev.conversation_id) {
            if let Some(node) = conv.nodes.get(&ev.start_node) {
                manager.active_conversation_id = Some(ev.conversation_id.clone());
                manager.current_node_id = Some(ev.start_node.clone());
                manager.display_text = String::new();
                manager.char_index = 0;
                manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                manager.text_complete = false;
                manager.visible = true;
                apply_effects(&node.on_enter, &mut quests);
                next_phase.set(GamePhase::Dialoguing);
            }
        }
    }
}

fn handle_dialogue_choice(
    mut events: MessageReader<DialogueChoiceEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
) {
    for ev in events.read() {
        let Some(conv_id) = &manager.active_conversation_id else { continue };
        let Some(conv) = bank.conversations.get(conv_id) else { continue };
        let Some(current_id) = &manager.current_node_id else { continue };
        let Some(current_node) = conv.nodes.get(current_id) else { continue };

        let choice = &current_node.choices[ev.0];

        // 检查条件
        if let Some(cond) = &choice.condition {
            if !cond.check(&quests) {
                continue;
            }
        }

        // 跳转
        if let Some(next_node) = conv.nodes.get(&choice.next_id) {
            manager.current_node_id = Some(choice.next_id.clone());
            manager.display_text = String::new();
            manager.char_index = 0;
            manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
            manager.text_complete = false;
            apply_effects(&next_node.on_enter, &mut quests);
        }
    }
}

fn handle_dialogue_advance(
    mut events: MessageReader<DialogueAdvanceEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        let Some(conv_id) = &manager.active_conversation_id.clone() else { continue };
        let Some(conv) = bank.conversations.get(conv_id) else { continue };
        let Some(current_id) = &manager.current_node_id.clone() else { continue };
        let Some(current_node) = conv.nodes.get(current_id) else { continue };

        // 有选项时点击文字 → 切换调试视图（方便关卡设计）
        if manager.text_complete && !current_node.choices.is_empty() {
            manager.debug_visible = !manager.debug_visible;
            continue;
        }

        if manager.text_complete {
            // 文本完整 → 前进到下一节点或结束
            if let Some(next_id) = &current_node.next {
                if let Some(next_node) = conv.nodes.get(next_id) {
                    manager.current_node_id = Some(next_id.clone());
                    manager.display_text = String::new();
                    manager.char_index = 0;
                    manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                    manager.text_complete = false;
                    apply_effects(&next_node.on_enter, &mut quests);
                }
            } else {
                end_dialogue(&mut manager, &mut next_phase);
            }
        } else {
            // 跳过打字机
            if let Some(node) = conv.nodes.get(current_id) {
                manager.display_text = node.text.clone();
                manager.char_index = node.text.chars().count();
                manager.text_complete = true;
            }
        }
    }
}

fn dialogue_input(
    keys: Res<ButtonInput<KeyCode>>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    if !manager.visible {
        return;
    }

    let Some(conv_id) = &manager.active_conversation_id.clone() else { return };
    let Some(conv) = bank.conversations.get(conv_id) else { return };
    let Some(current_id) = &manager.current_node_id.clone() else { return };
    let Some(current_node) = conv.nodes.get(current_id) else { return };

    let advance = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::KeyF);

    // 数字键快捷选择（仅在文本完整且有选项时）
    if manager.text_complete && !current_node.choices.is_empty() {
        let number_keys = [
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
            KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        ];
        for (i, key) in number_keys.iter().enumerate() {
            if keys.just_pressed(*key) {
                if i < current_node.choices.len() {
                    let choice = &current_node.choices[i];
                    let valid = choice.condition.as_ref().map_or(true, |cond| cond.check(&quests));
                    if valid {
                        if let Some(next_node) = conv.nodes.get(&choice.next_id) {
                            manager.current_node_id = Some(choice.next_id.clone());
                            manager.display_text = String::new();
                            manager.char_index = 0;
                            manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                            manager.text_complete = false;
                            manager.debug_visible = false;
                            apply_effects(&next_node.on_enter, &mut quests);
                        }
                        return;
                    }
                }
            }
        }
    }

    if advance && !current_node.choices.is_empty() {
        // 有选项时不自动前进，需要选择选项（点击或数字键）
        return;
    }

    if advance {
        if manager.text_complete {
            // 文本已完整显示，前进到下一节点
            if let Some(next_id) = &current_node.next {
                if let Some(next_node) = conv.nodes.get(next_id) {
                    manager.current_node_id = Some(next_id.clone());
                    manager.display_text = String::new();
                    manager.char_index = 0;
                    manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                    manager.text_complete = false;
                    apply_effects(&next_node.on_enter, &mut quests);
                }
            } else {
                // 对话结束
                end_dialogue(&mut manager, &mut next_phase);
            }
        } else {
            // 跳过打字机效果，直接显示全部文本
            if let Some(node) = conv.nodes.get(current_id) {
                manager.display_text = node.text.clone();
                manager.char_index = node.text.chars().count();
                manager.text_complete = true;
            }
        }
    }
}

fn typewriter_tick(
    time: Res<Time>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
) {
    if manager.text_complete {
        return;
    }

    let Some(conv_id) = &manager.active_conversation_id else { return };
    let Some(conv) = bank.conversations.get(conv_id) else { return };
    let Some(current_id) = &manager.current_node_id else { return };
    let Some(current_node) = conv.nodes.get(current_id) else { return };

    manager.text_timer.tick(time.delta());
    let ticks = manager.text_timer.times_finished_this_tick() as usize;
    if ticks > 0 {
        // 中文每字显示
        let chars: Vec<char> = current_node.text.chars().collect();
        let target = (manager.char_index + ticks).min(chars.len());
        manager.display_text = chars[..target].iter().collect();
        manager.char_index = target;
        if manager.char_index >= chars.len() {
            manager.text_complete = true;
        }
    }
}

fn end_dialogue(manager: &mut DialogueManager, next_phase: &mut NextState<GamePhase>) {
    manager.visible = false;
    manager.active_conversation_id = None;
    manager.current_node_id = None;
    manager.display_text.clear();
    manager.char_index = 0;
    manager.text_complete = false;
    next_phase.set(GamePhase::Playing);
}

// ═══════════════════════════════════════════
// 对话 UI
// ═══════════════════════════════════════════

fn dialogue_ui(
    mut contexts: EguiContexts,
    bank: Res<DialogueBank>,
    manager: Res<DialogueManager>,
    quests: Res<QuestTracker>,
    mut choice_writer: MessageWriter<DialogueChoiceEvent>,
    mut advance_writer: MessageWriter<DialogueAdvanceEvent>,
) {
    let Some(conv_id) = &manager.active_conversation_id else { return };
    let Some(conv) = bank.conversations.get(conv_id) else { return };
    let Some(current_id) = &manager.current_node_id else { return };
    let Some(current_node) = conv.nodes.get(current_id) else { return };

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let panel_frame = egui::Frame {
        fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 220),
        inner_margin: egui::Margin::symmetric(20, 12),
        corner_radius: egui::CornerRadius::same(8),
        ..Default::default()
    };

    // 底部居中对话框
    egui::TopBottomPanel::bottom("dialogue_panel")
        .frame(panel_frame)
        .min_height(120.0)
        .max_height(180.0)
        .resizable(false)
        .show(ctx, |ui| {
            // 说话人
            ui.label(
                egui::RichText::new(&current_node.speaker)
                    .size(16.0)
                    .strong()
                    .color(egui::Color32::from_rgb(255, 200, 80)),
            );
            ui.add_space(6.0);

            // 对话文本（可点击区域）
            let display = if manager.text_complete {
                current_node.text.clone()
            } else {
                format!("{}▌", manager.display_text)
            };
            let text_response = ui.add(
                egui::Label::new(
                    egui::RichText::new(display)
                        .size(14.0)
                        .color(egui::Color32::WHITE),
                )
                .sense(egui::Sense::click()),
            );
            if text_response.clicked() {
                advance_writer.write(DialogueAdvanceEvent);
            }

            ui.add_space(8.0);

            // 选项或继续提示
            if manager.text_complete {
                if !current_node.choices.is_empty() {
                    // 过滤掉条件不满足的选项
                    let valid_choices: Vec<(usize, &DialogueChoice)> = current_node
                        .choices
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.condition.as_ref().map_or(true, |cond| cond.check(&quests)))
                        .collect();

                    if manager.debug_visible {
                        // 调试视图：显示节点内部状态（方便关卡设计）
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("[调试] 节点: {}", current_id))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 200, 100)),
                        );
                        if let Some(next) = &current_node.next {
                            ui.label(
                                egui::RichText::new(format!("  next → {}", next))
                                    .size(11.0)
                                    .color(egui::Color32::GRAY),
                            );
                        }
                        if !current_node.on_enter.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("  on_enter: {:?}", current_node.on_enter))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 200, 80)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!("  有效选项: {}/{}", valid_choices.len(), current_node.choices.len()))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                        for (i, c) in current_node.choices.iter().enumerate() {
                            let cond_str = match &c.condition {
                                Some(cond) => format!("[{:?}]", cond),
                                None => String::new(),
                            };
                            let valid = c.condition.as_ref().map_or(true, |cond| cond.check(&quests));
                            let color = if valid {
                                egui::Color32::from_rgb(200, 255, 200)
                            } else {
                                egui::Color32::from_rgb(150, 150, 150)
                            };
                            ui.label(
                                egui::RichText::new(format!("  [{}] → {} {}", i, c.next_id, cond_str))
                                    .size(11.0)
                                    .color(color),
                            );
                        }
                        ui.label(
                            egui::RichText::new("点击文字切换回选项视图")
                                .size(10.0)
                                .color(egui::Color32::DARK_GRAY),
                        );
                    } else if valid_choices.is_empty() {
                        // 所有选项都不满足 → 自动前进或结束
                        if current_node.next.is_some() {
                            ui.label(
                                egui::RichText::new("按 空格/Enter/F 或点击文字 继续")
                                    .size(12.0)
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    } else {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("— 选择一个选项 —")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 180, 200)),
                        );
                        ui.add_space(4.0);
                        for (idx, choice) in &valid_choices {
                            let key_label = format!("[{}]", idx + 1);
                            let full_text = format!("{}  {}", key_label, choice.text);
                            let response = ui.add_sized(
                                egui::vec2(ui.available_width(), 32.0),
                                egui::Button::new(
                                    egui::RichText::new(full_text)
                                        .size(13.0)
                                        .color(egui::Color32::WHITE),
                                ),
                            );
                            if response.clicked() {
                                choice_writer.write(DialogueChoiceEvent(*idx));
                            }
                            ui.add_space(4.0);
                        }
                    }
                } else if current_node.next.is_some() {
                    ui.label(
                        egui::RichText::new("按 空格/Enter/F 或点击文字 继续")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    // 点击文字区域继续
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("→ 继续")
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(100, 200, 255)),
                            ),
                        )
                        .clicked()
                    {
                        advance_writer.write(DialogueAdvanceEvent);
                    }
                } else {
                    // 对话结束 — 点击关闭
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("关闭")
                                .size(13.0)
                                .color(egui::Color32::WHITE),
                        ))
                        .clicked()
                    {
                        advance_writer.write(DialogueAdvanceEvent);
                    }
                    ui.label(
                        egui::RichText::new("按 空格/Enter 或点击上方文字 关闭")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                }
            }
        });
}
