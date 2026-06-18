//! 物品栏系统
//!
//! 定义 `ItemDef`、`ItemBank`、`Inventory` 核心数据结构，
//! 提供物品加载（RON → 运行时）、拾取/移除事件处理、Tab 键物品栏 UI。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game_state::GamePhase;

// ═══════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct ItemId(pub String);

impl ItemId {
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn has(&self, item_id: &str) -> bool {
        self.items.get(item_id).is_some_and(|&count| count > 0)
    }

    #[allow(dead_code)]
    pub fn count(&self, item_id: &str) -> u32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn give(&mut self, item_id: &str, amount: u32) {
        *self.items.entry(item_id.to_string()).or_insert(0) += amount;
    }

    pub fn remove(&mut self, item_id: &str, amount: u32) -> bool {
        if let Some(count) = self.items.get_mut(item_id)
            && *count >= amount {
                *count -= amount;
                if *count == 0 {
                    self.items.remove(item_id);
                }
                return true;
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
                ),
            );
            super::inventory_ui::register_inventory_ui(app);
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
            id: "east_wing_key".into(),
            name: "东翼钥匙".into(),
            description: "通往东翼走廊的钥匙，在接待大厅找到。".into(),
            item_type: ItemType::QuestItem,
        },
        ItemDef {
            id: "courtyard_key".into(),
            name: "庭院钥匙".into(),
            description: "通往庭院的钥匙，在东翼走廊找到。".into(),
            item_type: ItemType::QuestItem,
        },
        ItemDef {
            id: "underground_key".into(),
            name: "地下层钥匙".into(),
            description: "通往地下层的钥匙，在西翼档案室找到。".into(),
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
        if inventory.remove(&ev.item_id, ev.amount)
            && let Some(def) = bank.items.get(&ev.item_id) {
                info!("失去物品: {} x{}", def.name, ev.amount);
            }
    }
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

