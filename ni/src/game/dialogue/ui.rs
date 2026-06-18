//! 对话系统 — UI 渲染

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::game::dialogue::events::*;
use crate::game::dialogue::quest::*;
use crate::game::dialogue::systems::DialogueManager;
use crate::game::dialogue::types::*;
use crate::inventory::Inventory;

pub fn dialogue_ui(
    mut contexts: EguiContexts,
    bank: Res<DialogueBank>,
    manager: Res<DialogueManager>,
    quests: Res<QuestTracker>,
    inventory: Res<Inventory>,
    mut choice_writer: MessageWriter<DialogueChoiceEvent>,
    mut advance_writer: MessageWriter<DialogueAdvanceEvent>,
) {
    let Some(conv_id) = &manager.active_conversation_id else {
        return;
    };
    let Some(conv) = bank.conversations.get(conv_id) else {
        return;
    };
    let Some(current_id) = &manager.current_node_id else {
        return;
    };
    let Some(current_node) = conv.nodes.get(current_id) else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let panel_frame = egui::Frame {
        fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 220),
        inner_margin: egui::Margin::symmetric(20, 12),
        corner_radius: egui::CornerRadius::same(8),
        ..Default::default()
    };

    egui::TopBottomPanel::bottom("dialogue_panel")
        .frame(panel_frame)
        .min_height(120.0)
        .max_height(180.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(&current_node.speaker)
                    .size(16.0)
                    .strong()
                    .color(egui::Color32::from_rgb(255, 200, 80)),
            );
            ui.add_space(6.0);

            let display = if manager.text_complete {
                current_node.text.clone()
            } else {
                format!("{}▌", manager.display_text)
            };
            let text_response = ui
                .add(
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

            if manager.text_complete {
                if !current_node.choices.is_empty() {
                    let valid_choices: Vec<(usize, &DialogueChoice)> = current_node
                        .choices
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| {
                            c.condition
                                .as_ref()
                                .is_none_or(|cond| cond.check(&quests, &inventory))
                        })
                        .collect();

                    if manager.debug_visible {
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
                                egui::RichText::new(format!(
                                    "  on_enter: {:?}",
                                    current_node.on_enter
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(255, 200, 80)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "  有效选项: {}/{}",
                                valid_choices.len(),
                                current_node.choices.len()
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                        for (i, c) in current_node.choices.iter().enumerate() {
                            let valid = c.condition.as_ref().is_none_or(|cond| {
                                cond.check(&quests, &inventory)
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "  [{}] → {} {:?}",
                                    i, c.next_id, c.condition
                                ))
                                .size(11.0)
                                .color(if valid {
                                    egui::Color32::from_rgb(200, 255, 200)
                                } else {
                                    egui::Color32::from_rgb(150, 150, 150)
                                }),
                            );
                        }
                    } else if valid_choices.is_empty() {
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
                            if ui
                                .add_sized(
                                    egui::vec2(ui.available_width(), 32.0),
                                    egui::Button::new(
                                        egui::RichText::new(format!(
                                            "[{}]  {}",
                                            idx + 1,
                                            choice.text
                                        ))
                                        .size(13.0)
                                        .color(egui::Color32::WHITE),
                                    ),
                                )
                                .clicked()
                            {
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
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("→ 继续")
                                .size(13.0)
                                .color(egui::Color32::from_rgb(100, 200, 255)),
                        ))
                        .clicked()
                    {
                        advance_writer.write(DialogueAdvanceEvent);
                    }
                } else {
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
                }
            }
        });
}
