//! 物品栏 Bevy 原生 UI
//!
//! 使用 Bevy UI (Node/Button/Text 组件) 替代原有的 egui 背包界面。
//! 格子布局、悬停高亮、物品描述提示框。
//!
//! 通过 InventoryState 状态机管理生命周期：
//! - Tab → sync_inventory_state 桥接 → OnEnter(Shown) spawn → Update(Shown) 交互 → OnExit(Hidden) despawn

use bevy::prelude::*;

use crate::core::game_state::{pause_toggle, GamePhase};
use crate::game::inventory::{Inventory, ItemBank, ItemType};

// ═══════════════════════════════════════════
// Marker Components
// ═══════════════════════════════════════════

#[derive(Component)]
pub struct InventoryUiRoot;

#[derive(Component)]
pub struct InventorySlot {
    pub item_id: String,
}

#[derive(Component)]
pub struct InventoryTooltip;

#[derive(Component)]
pub struct InventoryTooltipName;

#[derive(Component)]
pub struct InventoryTooltipDesc;

#[derive(Component)]
pub struct InventoryGridContainer;

#[derive(Component)]
pub struct InventoryCloseButton;

// ═══════════════════════════════════════════
// State & Resources
// ═══════════════════════════════════════════

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InventoryState {
    #[default]
    Hidden,
    Shown,
}

#[derive(Resource, Default)]
pub struct InventoryHoveredSlot(pub Option<String>);

/// 记录背包打开时的 GamePhase，Esc 关闭时恢复到此阶段
#[derive(Resource)]
pub struct InventoryReturnPhase(pub GamePhase);

impl Default for InventoryReturnPhase {
    fn default() -> Self {
        Self(GamePhase::Playing)
    }
}

// ═══════════════════════════════════════════
// 颜色常量
// ═══════════════════════════════════════════

const BG_OVERLAY: Color = Color::srgba(0.03, 0.05, 0.10, 0.90);
const BG_PANEL: Color = Color::srgba(0.08, 0.08, 0.18, 0.96);
const BG_SLOT: Color = Color::srgba(0.12, 0.12, 0.24, 0.80);
const BG_SLOT_HOVER: Color = Color::srgba(0.20, 0.20, 0.35, 0.85);
const COLOR_BORDER: Color = Color::srgb(0.24, 0.22, 0.35);
const COLOR_BORDER_HOVER: Color = Color::srgb(1.0, 0.78, 0.2);
const COLOR_TEXT_ACCENT: Color = Color::srgb(1.0, 0.78, 0.2);

// ═══════════════════════════════════════════
// 系统
// ═══════════════════════════════════════════

/// 桥接系统：Inventory.visible → InventoryState
fn sync_inventory_state(
    inventory: Res<Inventory>,
    mut next_state: ResMut<NextState<InventoryState>>,
    mut return_phase: ResMut<InventoryReturnPhase>,
    state: Res<State<InventoryState>>,
    phase: Res<State<GamePhase>>,
) {
    let allowed = matches!(phase.get(), GamePhase::Playing | GamePhase::Dialoguing);
    let was_shown = state.get() == &InventoryState::Shown;

    if allowed && inventory.visible && !was_shown {
        return_phase.0 = *phase.get();
        next_state.set(InventoryState::Shown);
    } else if !inventory.visible && was_shown {
        next_state.set(InventoryState::Hidden);
    }
}

/// 生成 UI 实体树
fn spawn_inventory_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_OVERLAY),
            InventoryUiRoot,
        ))
        .with_children(|parent| {
            // 面板
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(380.0),
                        height: Val::Auto,
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(BG_PANEL),
                    BorderColor::all(COLOR_BORDER),
                ))
                .with_children(|panel| {
                    // 标题栏
                    panel
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            width: Val::Percent(100.0),
                            margin: UiRect::bottom(Val::Px(12.0)),
                            ..default()
                        },))
                        .with_children(|title_bar| {
                            title_bar.spawn((
                                Text::new("背包"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                            title_bar
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(28.0),
                                        height: Val::Px(28.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
                                    InventoryCloseButton,
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new("✕"),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                    ));
                                });
                        });

                    // 网格容器
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::FlexStart,
                                width: Val::Percent(100.0),
                                min_height: Val::Px(100.0),
                                ..default()
                            },
                            InventoryGridContainer,
                        ))
                        .with_children(|grid| {
                            grid.spawn((
                                Text::new("背包空空如也"),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 0.5, 0.6)),
                                Node {
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    padding: UiRect::all(Val::Px(20.0)),
                                    ..default()
                                },
                            ));
                        });

                    // 提示框（默认隐藏）
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                margin: UiRect::top(Val::Px(8.0)),
                                padding: UiRect::all(Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.12, 0.24, 0.9)),
                            BorderColor::all(COLOR_BORDER),
                            Visibility::Hidden,
                            InventoryTooltip,
                        ))
                        .with_children(|tip| {
                            tip.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                InventoryTooltipName,
                            ));
                            tip.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.7, 0.78)),
                                InventoryTooltipDesc,
                            ));
                        });
                });
        });
}

/// 销毁 UI 实体树
fn despawn_inventory_ui(mut commands: Commands, root: Query<Entity, With<InventoryUiRoot>>) {
    for entity in &root {
        commands.entity(entity).despawn();
    }
}

/// 每帧同步物品数据：despawn 旧格子 + respawn 新格子
fn sync_inventory_items(
    mut commands: Commands,
    inventory: Res<Inventory>,
    bank: Res<ItemBank>,
    grid_query: Query<Entity, With<InventoryGridContainer>>,
    slot_query: Query<Entity, With<InventorySlot>>,
) {
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };

    // 清除旧格子
    for entity in &slot_query {
        commands.entity(entity).despawn();
    }

    // 空状态
    if inventory.items.is_empty() {
        commands.entity(grid_entity).with_children(|grid| {
            grid.spawn((
                Text::new("背包空空如也"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.6)),
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
            ));
        });
        return;
    }

    // 生成物品格子
    let mut sorted: Vec<(&String, &u32)> = inventory.items.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    commands.entity(grid_entity).with_children(|grid| {
        for (item_id, count) in &sorted {
            let def = bank.items.get(*item_id);
            let emoji = match def.map(|d| &d.item_type) {
                Some(ItemType::Consumable) => "🧪",
                Some(ItemType::QuestItem) => "🔑",
                Some(ItemType::Collectible) => "🪙",
                None => "📦",
            };
            let name = def.map_or(item_id.as_str(), |d| d.name.as_str());

            grid.spawn((
                Button,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: Val::Px(85.0),
                    height: Val::Px(90.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    margin: UiRect::all(Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(COLOR_BORDER),
                BackgroundColor(BG_SLOT),
                InventorySlot {
                    item_id: (*item_id).clone(),
                },
            ))
            .with_children(|slot| {
                slot.spawn((
                    Text::new(emoji),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                ));
                slot.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ));
                slot.spawn((
                    Text::new(format!("x{}", count)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_ACCENT),
                ));
            });
        }
    });
}

/// 检测悬停状态
fn inventory_hover_system(
    slots: Query<(&Interaction, &InventorySlot)>,
    mut hovered: ResMut<InventoryHoveredSlot>,
) {
    hovered.0 = None;
    for (interaction, slot) in &slots {
        if *interaction == Interaction::Hovered {
            hovered.0 = Some(slot.item_id.clone());
        }
    }
}

/// 更新格子边框高亮
fn inventory_slot_highlight_system(
    slots: Query<(&Interaction, Entity), With<InventorySlot>>,
    mut borders: Query<&mut BorderColor>,
    mut backgrounds: Query<&mut BackgroundColor>,
) {
    for (interaction, entity) in &slots {
        if let Ok(mut border) = borders.get_mut(entity) {
            if *interaction == Interaction::Hovered {
                *border = BorderColor::all(COLOR_BORDER_HOVER);
                if let Ok(mut bg) = backgrounds.get_mut(entity) {
                    bg.0 = BG_SLOT_HOVER;
                }
            } else {
                *border = BorderColor::all(COLOR_BORDER);
                if let Ok(mut bg) = backgrounds.get_mut(entity) {
                    bg.0 = BG_SLOT;
                }
            }
        }
    }
}

/// 更新提示框
#[allow(clippy::type_complexity)]
fn inventory_tooltip_system(
    hovered: Res<InventoryHoveredSlot>,
    bank: Res<ItemBank>,
    mut tooltip: Query<&mut Visibility, With<InventoryTooltip>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<InventoryTooltipName>>,
        Query<&mut Text, With<InventoryTooltipDesc>>,
    )>,
) {
    let Ok(mut vis) = tooltip.single_mut() else {
        return;
    };

    if let Some(item_id) = &hovered.0 {
        if let Some(def) = bank.items.get(item_id) {
            if let Ok(mut t) = texts.p0().single_mut() {
                t.0.clone_from(&def.name);
            }
            if let Ok(mut t) = texts.p1().single_mut() {
                t.0.clone_from(&def.description);
            }
        }
        *vis = Visibility::Visible;
    } else {
        *vis = Visibility::Hidden;
    }
}

/// 关闭按钮点击检测
fn inventory_close_click_system(
    mut inventory: ResMut<Inventory>,
    close_buttons: Query<&Interaction, (With<InventoryCloseButton>, Changed<Interaction>)>,
) {
    for interaction in &close_buttons {
        if *interaction == Interaction::Pressed {
            inventory.visible = false;
        }
    }
}

/// Esc 关闭背包 + 恢复 GamePhase
/// 必须在 pause_toggle 之后运行，以撤销暂停
fn inventory_esc_handler(
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
    phase: Res<State<GamePhase>>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    return_phase: Res<InventoryReturnPhase>,
) {
    if keys.just_pressed(KeyCode::Escape) && inventory.visible {
        inventory.visible = false;
        if *phase.get() == GamePhase::Paused {
            next_phase.set(return_phase.0);
        }
    }
}

// ═══════════════════════════════════════════
// 注册入口
// ═══════════════════════════════════════════

pub fn register_inventory_ui(app: &mut App) {
    app.init_resource::<InventoryHoveredSlot>()
        .init_resource::<InventoryReturnPhase>()
        .init_state::<InventoryState>()
        .add_systems(Update, sync_inventory_state)
        .add_systems(OnEnter(InventoryState::Shown), spawn_inventory_ui)
        .add_systems(OnExit(InventoryState::Shown), despawn_inventory_ui)
        .add_systems(
            Update,
            (
                sync_inventory_items,
                inventory_hover_system,
                inventory_slot_highlight_system,
                inventory_tooltip_system,
                inventory_close_click_system,
            )
                .run_if(in_state(InventoryState::Shown)),
        )
        .add_systems(Update, inventory_esc_handler.after(pause_toggle));
}
