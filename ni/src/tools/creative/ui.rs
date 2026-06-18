//! 创造模式 — UI（物品栏、分类标签、信息面板）

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::entity_db::{EntityCategory, EntityRegistry};
use crate::game_state::GamePhase;
use crate::tools::creative::state::*;
use crate::ui::theme;

fn category_color(cat: &EntityCategory) -> egui::Color32 {
    match cat {
        EntityCategory::Prop => egui::Color32::from_rgb(76, 175, 80),
        EntityCategory::Npc => egui::Color32::from_rgb(33, 150, 243),
        EntityCategory::Enemy => egui::Color32::from_rgb(244, 67, 54),
        EntityCategory::Collectible => egui::Color32::from_rgb(255, 193, 7),
        EntityCategory::Projectile => egui::Color32::from_rgb(156, 39, 176),
        EntityCategory::StressNpc => egui::Color32::from_rgb(255, 87, 34),
    }
}

fn cat_tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let height = 32.0;
    let pad = 16.0;
    let w = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(14.0), theme::TEXT_PRIMARY).size().x + pad * 2.0;
    let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(w, height), egui::Sense::click());
    if ui.is_rect_visible(pos) {
        let bg = if resp.hovered() && !selected {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 8)
        } else { egui::Color32::TRANSPARENT };
        ui.painter().rect_filled(pos, egui::CornerRadius::ZERO, bg);
        let color = if selected { theme::TAB_ACTIVE } else if resp.hovered() { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };
        let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(14.0), color);
        ui.painter().galley(egui::pos2(pos.center().x - galley.size().x * 0.5, pos.center().y - galley.size().y * 0.5), galley, color);
        if selected {
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(pos.center().x - 12.0, pos.bottom() - 4.0),
                egui::vec2(24.0, 2.5),
            );
            ui.painter().rect_filled(bar_rect, egui::CornerRadius::same(2), theme::TAB_ACTIVE);
        } else if resp.hovered() {
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(pos.center().x - 8.0, pos.bottom() - 4.0),
                egui::vec2(16.0, 2.0),
            );
            ui.painter().rect_filled(bar_rect, egui::CornerRadius::same(1), theme::TAB_HOVER);
        }
    }
    resp
}

fn slot_widget(
    ui: &mut egui::Ui, size: egui::Vec2, icon_rect: egui::Rect,
    icon_color: egui::Color32, name: &str, key_label: &str, selected: bool,
) -> egui::Response {
    let (pos, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if !ui.is_rect_visible(pos) { return resp; }
    let round = egui::CornerRadius::same(6);
    let bg = if selected { theme::SLOT_SELECTED } else if resp.hovered() { theme::SLOT_HOVER } else { theme::SLOT_BG };
    ui.painter().rect_filled(pos, round, bg);
    let border_c = if selected { theme::BORDER_FOCUS } else if resp.hovered() { theme::TEXT_SECONDARY }
        else { egui::Color32::from_rgba_premultiplied(60, 60, 90, 120) };
    ui.painter().rect_stroke(pos, round, egui::Stroke::new(if selected { 2.0 } else { 1.0 }, border_c), egui::StrokeKind::Middle);
    ui.painter().rect_filled(icon_rect, egui::CornerRadius::same(4), icon_color.linear_multiply(0.7));
    let short: String = name.chars().take(4).collect();
    let name_g = ui.painter().layout_no_wrap(short, egui::FontId::proportional(10.0), theme::TEXT_SECONDARY);
    ui.painter().galley(egui::pos2(pos.center().x - name_g.size().x * 0.5, icon_rect.bottom() + 3.0), name_g, theme::TEXT_SECONDARY);
    let key_g = ui.painter().layout_no_wrap(key_label.to_string(), egui::FontId::proportional(10.0), theme::KEY_HINT);
    ui.painter().galley(egui::pos2(pos.left() + 4.0, pos.top() + 3.0), key_g, theme::KEY_HINT);
    resp
}

pub fn creative_hotbar_ui(
    mut contexts: EguiContexts, keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>, registry: Res<EntityRegistry>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut state: ResMut<CreativeState>, mut rendered: Local<bool>,
) {
    if phase.get() != &GamePhase::Creative { return; }
    if !*rendered { info!("[CreativeUI] 进入 Creative 阶段，开始渲染物品栏"); *rendered = true; }
    let ctx = match contexts.ctx_mut() { Ok(c) => c, Err(e) => { warn!("[CreativeUI] ctx_mut() 失败: {:?}", e); return; } };
    let win = match window.single() { Ok(w) => w, Err(e) => { warn!("[CreativeUI] 窗口查询失败: {:?}", e); return; } };
    let screen_w = win.resolution.physical_width() as f32;
    let screen_h = win.resolution.physical_height() as f32;

    let show_count = state.current_items.len().min(10);
    if show_count > 0 {
        for n in 0..show_count {
            let key = if n < 9 { match n { 0 => KeyCode::Digit1, 1 => KeyCode::Digit2, 2 => KeyCode::Digit3, 3 => KeyCode::Digit4, 4 => KeyCode::Digit5, 5 => KeyCode::Digit6, 6 => KeyCode::Digit7, 7 => KeyCode::Digit8, 8 => KeyCode::Digit9, _ => unreachable!(), } } else { KeyCode::Digit0 };
            if keys.just_pressed(key) { state.selected_slot = n; }
        }
    }

    let categories = state.categories.clone();
    let cat_index = state.category_index;
    let mut clicked_cat: Option<usize> = None;
    egui::Area::new(egui::Id::new("creative_cat_bar"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame { fill: theme::TOOLBAR_BG, inner_margin: egui::Margin::symmetric(12, 4), ..Default::default() }
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, cat_name) in categories.iter().enumerate() {
                            let resp = cat_tab_button(ui, cat_name, cat_index == i);
                            if resp.clicked() { clicked_cat = Some(i); }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if state.dirty { ui.label(egui::RichText::new("● 未保存").size(12.0).color(theme::TEXT_DANGER)); }
                        });
                    });
                });
        });
    if let Some(i) = clicked_cat {
        state.category_index = i;
        state.current_items = state.category_items.get(i).cloned().unwrap_or_default();
        state.selected_slot = 0;
        state.current_item_names = state.current_items.iter()
            .map(|id| registry.templates.get(id.as_str()).map_or(id.clone(), |t| t.display_name.clone())).collect();
        state.current_item_categories = state.current_items.iter()
            .map(|id| registry.templates.get(id.as_str()).map_or(EntityCategory::Prop, |t| t.category.clone())).collect();
    }

    let current_items = state.current_items.clone();
    let current_item_names = state.current_item_names.clone();
    let current_item_categories = state.current_item_categories.clone();
    let selected_slot = state.selected_slot;
    let mut clicked_slot: Option<usize> = None;
    if show_count > 0 {
        let slot_size = egui::Vec2::new(64.0, 70.0);
        let gap = 4.0;
        let total_w = show_count as f32 * (slot_size.x + gap) - gap;
        let start_x = (screen_w - total_w) * 0.5;
        egui::Area::new(egui::Id::new("creative_hotbar"))
            .fixed_pos(egui::pos2(start_x, screen_h - 90.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame { fill: theme::TOOLBAR_BG, inner_margin: egui::Margin::symmetric(10, 8), corner_radius: egui::CornerRadius::same(8), ..Default::default() }
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, _item_id) in current_items.iter().enumerate().take(10) {
                                let sel = selected_slot == i;
                                let name = current_item_names.get(i).map_or("?", |s| s.as_str());
                                let cat = current_item_categories.get(i).cloned().unwrap_or(EntityCategory::Prop);
                                let icon_color = category_color(&cat);
                                let icon_pos = egui::Rect::from_center_size(egui::pos2(32.0, 18.0), egui::vec2(28.0, 28.0));
                                let key_label = if i < 9 { format!("{}", i + 1) } else { "0".into() };
                                let resp = slot_widget(ui, slot_size, icon_pos, icon_color, name, &key_label, sel);
                                if resp.clicked() { clicked_slot = Some(i); }
                                let tip_name = name.to_string();
                                let _ = resp.on_hover_ui(|ui| { ui.label(egui::RichText::new(tip_name).size(13.0).color(theme::TEXT_PRIMARY)); });
                            }
                        });
                    });
            });
    }
    if let Some(i) = clicked_slot { state.selected_slot = i; }

    egui::Area::new(egui::Id::new("creative_info"))
        .fixed_pos(egui::pos2(10.0, 44.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame { fill: theme::TOOLBAR_BG, inner_margin: egui::Margin::symmetric(10, 8), corner_radius: egui::CornerRadius::same(6), ..Default::default() }
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("创造模式").size(14.0).color(theme::TAB_ACTIVE).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("网格吸附: {}", if state.grid_snap { "开启" } else { "关闭" })).size(12.0).color(theme::TEXT_SECONDARY));
                    if state.dirty { ui.label(egui::RichText::new("* 未保存").size(12.0).color(theme::TEXT_DANGER)); }
                    if let Some(id) = &state.current_items.get(state.selected_slot).cloned()
                        && let Some(t) = registry.templates.get(id.as_str()) {
                            ui.label(egui::RichText::new(format!("当前: {}", t.display_name)).size(12.0).color(theme::TEXT_PRIMARY));
                        }
                });
        });

    egui::Area::new(egui::Id::new("creative_help"))
        .fixed_pos(egui::pos2(0.0, screen_h - 26.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame { fill: theme::TOOLBAR_BG, inner_margin: egui::Margin::symmetric(12, 3), ..Default::default() }
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("左键放置 · 右键删除 · G 网格 · H 标签 · L 关卡 · Ctrl+S 保存").size(11.0).color(theme::TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("{:.0}×{:.0}", screen_w, screen_h)).size(10.0).color(theme::TEXT_MUTED));
                        });
                    });
                });
        });
}
