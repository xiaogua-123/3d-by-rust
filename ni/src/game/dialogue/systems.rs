//! 对话系统 — 状态机与输入处理

use bevy::prelude::*;

use crate::game::dialogue::branch::apply_effects;
use crate::game::dialogue::events::*;
use crate::game::dialogue::quest::*;
use crate::game::dialogue::types::*;
use crate::game_state::{GamePhase, OverlayState};
use crate::inventory::{GiveItemEvent, Inventory, RemoveItemEvent};

/// 对话管理器资源
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

pub fn dialogue_visible(manager: Res<DialogueManager>) -> bool {
    manager.visible
}

pub fn handle_start_dialogue(
    mut events: MessageReader<StartDialogueEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut give_item_writer: MessageWriter<GiveItemEvent>,
    mut remove_item_writer: MessageWriter<RemoveItemEvent>,
) {
    for ev in events.read() {
        if let Some(conv) = bank.conversations.get(&ev.conversation_id)
            && let Some(node) = conv.nodes.get(&ev.start_node) {
                manager.active_conversation_id = Some(ev.conversation_id.clone());
                manager.current_node_id = Some(ev.start_node.clone());
                manager.display_text = String::new();
                manager.char_index = 0;
                manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                manager.text_complete = false;
                manager.visible = true;
                for cmd in apply_effects(&node.on_enter, &mut quests) {
                    match cmd {
                        PendingEffect::GiveItem(id, amount) => {
                            give_item_writer.write(GiveItemEvent {
                                item_id: id,
                                amount,
                            });
                        }
                        PendingEffect::RemoveItem(id, amount) => {
                            remove_item_writer.write(RemoveItemEvent {
                                item_id: id,
                                amount,
                            });
                        }
                    }
                }
                next_phase.set(GamePhase::Dialoguing);
            }
    }
}

pub fn handle_dialogue_choice(
    mut events: MessageReader<DialogueChoiceEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    inventory: Res<Inventory>,
    mut give_item_writer: MessageWriter<GiveItemEvent>,
    mut remove_item_writer: MessageWriter<RemoveItemEvent>,
) {
    for ev in events.read() {
        let Some(conv_id) = &manager.active_conversation_id.clone() else {
            continue;
        };
        let Some(conv) = bank.conversations.get(conv_id) else {
            continue;
        };
        let Some(current_id) = &manager.current_node_id.clone() else {
            continue;
        };
        let Some(current_node) = conv.nodes.get(current_id) else {
            continue;
        };
        let choice = &current_node.choices[ev.0];

        if let Some(cond) = &choice.condition
            && !cond.check(&quests, &inventory) {
                continue;
            }

        if let Some(next_node) = conv.nodes.get(&choice.next_id) {
            manager.current_node_id = Some(choice.next_id.clone());
            manager.display_text = String::new();
            manager.char_index = 0;
            manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
            manager.text_complete = false;
            for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                match cmd {
                    PendingEffect::GiveItem(id, amount) => {
                        give_item_writer.write(GiveItemEvent {
                            item_id: id,
                            amount,
                        });
                    }
                    PendingEffect::RemoveItem(id, amount) => {
                        remove_item_writer.write(RemoveItemEvent {
                            item_id: id,
                            amount,
                        });
                    }
                }
            }
        }
    }
}

pub fn handle_dialogue_advance(
    mut events: MessageReader<DialogueAdvanceEvent>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut give_item_writer: MessageWriter<GiveItemEvent>,
    mut remove_item_writer: MessageWriter<RemoveItemEvent>,
) {
    for _ in events.read() {
        let Some(conv_id) = &manager.active_conversation_id.clone() else {
            continue;
        };
        let Some(conv) = bank.conversations.get(conv_id) else {
            continue;
        };
        let Some(current_id) = &manager.current_node_id.clone() else {
            continue;
        };
        let Some(current_node) = conv.nodes.get(current_id) else {
            continue;
        };

        if manager.text_complete && !current_node.choices.is_empty() {
            manager.debug_visible = !manager.debug_visible;
            continue;
        }

        if manager.text_complete {
            if let Some(next_id) = &current_node.next {
                if let Some(next_node) = conv.nodes.get(next_id) {
                    manager.current_node_id = Some(next_id.clone());
                    manager.display_text = String::new();
                    manager.char_index = 0;
                    manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                    manager.text_complete = false;
                    for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                        match cmd {
                            PendingEffect::GiveItem(id, amount) => {
                                give_item_writer.write(GiveItemEvent {
                                    item_id: id,
                                    amount,
                                });
                            }
                            PendingEffect::RemoveItem(id, amount) => {
                                remove_item_writer.write(RemoveItemEvent {
                                    item_id: id,
                                    amount,
                                });
                            }
                        }
                    }
                }
            } else {
                end_dialogue(&mut manager, &mut next_phase);
            }
        } else {
            if let Some(node) = conv.nodes.get(current_id) {
                manager.display_text = node.text.clone();
                manager.char_index = node.text.chars().count();
                manager.text_complete = true;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn dialogue_input(
    keys: Res<ButtonInput<KeyCode>>,
    overlay: Res<OverlayState>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
    mut quests: ResMut<QuestTracker>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    inventory: Res<Inventory>,
    mut give_item_writer: MessageWriter<GiveItemEvent>,
    mut remove_item_writer: MessageWriter<RemoveItemEvent>,
) {
    if !manager.visible || overlay.active.is_some() {
        return;
    }

    let Some(conv_id) = &manager.active_conversation_id.clone() else {
        return;
    };
    let Some(conv) = bank.conversations.get(conv_id) else {
        return;
    };
    let Some(current_id) = &manager.current_node_id.clone() else {
        return;
    };
    let Some(current_node) = conv.nodes.get(current_id) else {
        return;
    };

    let advance = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::KeyF);

    let number_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];

    if manager.text_complete && !current_node.choices.is_empty() {
        for (i, key) in number_keys.iter().enumerate() {
            if keys.just_pressed(*key) && i < current_node.choices.len() {
                let choice = &current_node.choices[i];
                if choice
                    .condition
                    .as_ref()
                    .is_none_or(|cond| cond.check(&quests, &inventory))
                {
                    if let Some(next_node) = conv.nodes.get(&choice.next_id) {
                        manager.current_node_id = Some(choice.next_id.clone());
                        manager.display_text = String::new();
                        manager.char_index = 0;
                        manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                        manager.text_complete = false;
                        manager.debug_visible = false;
                        for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                            match cmd {
                                PendingEffect::GiveItem(id, amount) => {
                                    give_item_writer.write(GiveItemEvent {
                                        item_id: id,
                                        amount,
                                    });
                                }
                                PendingEffect::RemoveItem(id, amount) => {
                                    remove_item_writer.write(RemoveItemEvent {
                                        item_id: id,
                                        amount,
                                    });
                                }
                            }
                        }
                    }
                    return;
                }
            }
        }
    }

    if advance && !current_node.choices.is_empty() {
        return;
    }

    if advance {
        if manager.text_complete {
            if let Some(next_id) = &current_node.next {
                if let Some(next_node) = conv.nodes.get(next_id) {
                    manager.current_node_id = Some(next_id.clone());
                    manager.display_text = String::new();
                    manager.char_index = 0;
                    manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                    manager.text_complete = false;
                    for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                        match cmd {
                            PendingEffect::GiveItem(id, amount) => {
                                give_item_writer.write(GiveItemEvent {
                                    item_id: id,
                                    amount,
                                });
                            }
                            PendingEffect::RemoveItem(id, amount) => {
                                remove_item_writer.write(RemoveItemEvent {
                                    item_id: id,
                                    amount,
                                });
                            }
                        }
                    }
                }
            } else {
                end_dialogue(&mut manager, &mut next_phase);
            }
        } else {
            if let Some(node) = conv.nodes.get(current_id) {
                manager.display_text = node.text.clone();
                manager.char_index = node.text.chars().count();
                manager.text_complete = true;
            }
        }
    }
}

pub fn typewriter_tick(
    time: Res<Time>,
    bank: Res<DialogueBank>,
    mut manager: ResMut<DialogueManager>,
) {
    if manager.text_complete {
        return;
    }
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
    manager.text_timer.tick(time.delta());
    let ticks = manager.text_timer.times_finished_this_tick() as usize;
    if ticks > 0 {
        let chars: Vec<char> = current_node.text.chars().collect();
        let target = (manager.char_index + ticks).min(chars.len());
        manager.display_text = chars[..target].iter().collect();
        manager.char_index = target;
        if manager.char_index >= chars.len() {
            manager.text_complete = true;
        }
    }
}

pub fn end_dialogue(
    manager: &mut DialogueManager,
    next_phase: &mut NextState<GamePhase>,
) {
    manager.visible = false;
    manager.active_conversation_id = None;
    manager.current_node_id = None;
    manager.display_text.clear();
    manager.char_index = 0;
    manager.text_complete = false;
    next_phase.set(GamePhase::Playing);
}
