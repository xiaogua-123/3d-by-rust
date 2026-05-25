use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashMap;

use crate::game_state::GamePhase;

// ═══════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId(pub String);

impl ItemId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone)]
pub enum ItemType {
    Collectible,
    QuestItem,
    Consumable,
}

impl ItemType {
    pub fn label(&self) -> &'static str {
        match self {
            ItemType::Collectible => "收集品",
            ItemType::QuestItem => "任务物品",
            ItemType::Consumable => "消耗品",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
}

// ═══════════════════════════════════════════
// 资源
// ═══════════════════════════════════════════

#[derive(Resource, Default)]
pub struct ItemBank {
    pub items: HashMap<String, ItemDef>,
}

#[derive(Resource, Default)]
pub struct Inventory {
    pub items: HashMap<String, u32>,
    pub visible: bool,
}

impl Inventory {
    pub fn has(&self, item_id: &str) -> bool {
        self.items.get(item_id).map_or(false, |&count| count > 0)
    }

    pub fn count(&self, item_id: &str) -> u32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn give(&mut self, item_id: &str, amount: u32) {
        *self.items.entry(item_id.to_string()).or_insert(0) += amount;
    }

    pub fn remove(&mut self, item_id: &str, amount: u32) -> bool {
        if let Some(count) = self.items.get_mut(item_id) {
            if *count >= amount {
                *count -= amount;
                if *count == 0 {
                    self.items.remove(item_id);
                }
                return true;
            }
        }
        false
    }
}

// ═══════════════════════════════════════════
// 消息
// ═══════════════════════════════════════════

#[derive(Message)]
pub struct GiveItemEvent {
    pub item_id: String,
    pub amount: u32,
}

#[derive(Message)]
pub struct RemoveItemEvent {
    pub item_id: String,
    pub amount: u32,
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemBank>()
            .init_resource::<Inventory>()
            .add_message::<GiveItemEvent>()
            .add_message::<RemoveItemEvent>()
            .add_systems(Startup, load_items)
            .add_systems(
                Update,
                (
                    handle_give_item,
                    handle_remove_item,
                    inventory_toggle,
                    inventory_ui.run_if(inventory_open),
                ),
            );
    }
}

// ═══════════════════════════════════════════
// 物品加载（硬编码 + 预留 RON 文件加载）
// ═══════════════════════════════════════════

fn load_items(mut bank: ResMut<ItemBank>) {
    // 基础物品定义（后续可改为从 assets/items.ron 加载）
    let items = vec![
        ItemDef {
            id: "herb".into(),
            name: "药草".into(),
            description: "可以恢复少量生命值的草药。".into(),
            item_type: ItemType::Consumable,
        },
        ItemDef {
            id: "gold_coin".into(),
            name: "金币".into(),
            description: "闪闪发光的金币，用来买东西。".into(),
            item_type: ItemType::Collectible,
        },
        ItemDef {
            id: "old_key".into(),
            name: "旧钥匙".into(),
            description: "一把生锈的铁钥匙，似乎能打开某个门。".into(),
            item_type: ItemType::QuestItem,
        },
        ItemDef {
            id: "magic_crystal".into(),
            name: "魔法水晶".into(),
            description: "散发着微光的水晶，能感受到魔法的力量。".into(),
            item_type: ItemType::QuestItem,
        },
    ];

    for item in items {
        bank.items.insert(item.id.clone(), item);
    }
    info!("加载了 {} 种物品定义", bank.items.len());
}

// ═══════════════════════════════════════════
// 事件处理
// ═══════════════════════════════════════════

fn handle_give_item(
    mut events: MessageReader<GiveItemEvent>,
    mut inventory: ResMut<Inventory>,
    bank: Res<ItemBank>,
) {
    for ev in events.read() {
        inventory.give(&ev.item_id, ev.amount);
        if let Some(def) = bank.items.get(&ev.item_id) {
            info!("获得物品: {} x{}", def.name, ev.amount);
        } else {
            info!("获得物品: {} x{}", ev.item_id, ev.amount);
        }
    }
}

fn handle_remove_item(
    mut events: MessageReader<RemoveItemEvent>,
    mut inventory: ResMut<Inventory>,
    bank: Res<ItemBank>,
) {
    for ev in events.read() {
        if inventory.remove(&ev.item_id, ev.amount) {
            if let Some(def) = bank.items.get(&ev.item_id) {
                info!("失去物品: {} x{}", def.name, ev.amount);
            }
        }
    }
}

fn inventory_open(inventory: Res<Inventory>) -> bool {
    inventory.visible
}

fn inventory_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
    phase: Res<State<GamePhase>>,
) {
    if !matches!(phase.get(), GamePhase::Playing | GamePhase::Dialoguing) {
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        inventory.visible = !inventory.visible;
    }
}

// ═══════════════════════════════════════════
// 背包 UI
// ═══════════════════════════════════════════

fn inventory_ui(
    mut contexts: EguiContexts,
    mut inventory: ResMut<Inventory>,
    bank: Res<ItemBank>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let panel_frame = egui::Frame {
        fill: egui::Color32::from_rgba_premultiplied(10, 15, 25, 230),
        inner_margin: egui::Margin::same(12),
        corner_radius: egui::CornerRadius::same(8),
        ..Default::default()
    };

    let items_empty = inventory.items.is_empty();
    let items_snapshot: Vec<(String, u32)> = inventory
        .items
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    egui::Window::new("🎒 背包")
        .resizable(true)
        .default_size(egui::vec2(320.0, 400.0))
        .frame(panel_frame)
        .open(&mut inventory.visible)
        .show(ctx, |ui| {
            if items_empty {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("背包空空如也")
                            .size(14.0)
                            .color(egui::Color32::GRAY),
                    );
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (item_id, count) in &items_snapshot {
                    let def = bank.items.get(item_id);

                    ui.horizontal(|ui| {
                        // 物品图标占位
                        let icon = match def.map(|d| &d.item_type) {
                            Some(ItemType::Consumable) => "🧪",
                            Some(ItemType::QuestItem) => "🔑",
                            Some(ItemType::Collectible) => "🪙",
                            None => "📦",
                        };

                        ui.label(egui::RichText::new(icon).size(24.0));

                        ui.vertical(|ui| {
                            let name = def.map_or(item_id.as_str(), |d| d.name.as_str());
                            ui.label(
                                egui::RichText::new(name)
                                    .size(13.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                            if let Some(def) = def {
                                ui.label(
                                    egui::RichText::new(&def.description)
                                        .size(11.0)
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        });

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!("x{}", count))
                                        .size(16.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(255, 200, 50)),
                                );
                            },
                        );
                    });
                    ui.separator();
                }
            });
        });
}
