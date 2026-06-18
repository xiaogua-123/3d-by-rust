//! 游戏 UI 系统 — 菜单 / HUD / 弹窗
//!
//! 布局参考 CSS Flex / Grid 模型映射到 egui：
//! - CSS Flex column → ui.vertical() / ui.vertical_centered()
//! - CSS Flex row     → ui.horizontal() / ui.with_layout(right_to_left)
//! - CSS Grid         → ui.columns(n) / egui::Grid
//! - CSS card         → Frame + shadow + rounding
//! - CSS sticky bar   → TopBottomPanel / SidePanel

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use crate::collectible::{Collectible, InteractionTarget};
use crate::core::save::{LoadGameEvent, SaveGameEvent};
use crate::game_state::*;
use crate::inventory::ItemBank;
use crate::level::{GameLevel, LevelConfig};
use crate::td;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, (
            setup_fonts,
            main_menu_ui.run_if(in_state(GamePhase::MainMenu)),
            hud_ui.run_if(in_state(GamePhase::Playing)),
            td_hud_ui.run_if(in_state(GamePhase::Playing)),
            pause_ui.run_if(in_state(GamePhase::Paused)),
            game_over_ui.run_if(in_state(GamePhase::GameOver)),
            level_complete_ui.run_if(in_state(GamePhase::LevelComplete)),
            td_victory_ui.run_if(in_state(GamePhase::LevelComplete)),
        ));
    }
}

// ═══════════════════════════════════════════
// 视觉主题系统 — CSS 自定义属性 等效
// ═══════════════════════════════════════════

/// 全局 UI 主题 — 类似 CSS :root 变量
pub(crate) mod theme {
    use super::egui::Color32;

    // 背景色
    pub const BG_DARK: Color32 = Color32::from_rgba_premultiplied(8, 8, 18, 235);
    pub const BG_PANEL: Color32 = Color32::from_rgba_premultiplied(14, 14, 30, 245);
    pub const BG_HUD: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 140);
    // 边框
    pub const BORDER: Color32 = Color32::from_rgb(60, 55, 90);
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(255, 200, 50);
    // 文字色
    pub const TEXT_PRIMARY: Color32 = Color32::WHITE;
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 180, 200);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(110, 110, 130);
    pub const TEXT_ACCENT: Color32 = Color32::from_rgb(255, 200, 50);
    pub const TEXT_DANGER: Color32 = Color32::from_rgb(255, 70, 70);
    pub const TEXT_SUCCESS: Color32 = Color32::from_rgb(70, 220, 100);
    // 按钮色
    pub const BTN_BG: Color32 = Color32::from_rgb(30, 30, 55);
    pub const BTN_HOVER: Color32 = Color32::from_rgb(50, 45, 80);
    pub const BTN_ACCENT_BG: Color32 = Color32::from_rgb(60, 45, 10);
    pub const BTN_ACCENT_HOVER: Color32 = Color32::from_rgb(90, 65, 15);
    // 尺寸
    pub const CORNER_RADIUS: u8 = 8;
    pub const PANEL_RADIUS: u8 = 12;
    pub const BTN_WIDTH: f32 = 220.0;
    pub const BTN_HEIGHT: f32 = 48.0;
    pub const BTN_SMALL_H: f32 = 40.0;
    pub const CARD_MAX_W: f32 = 420.0;

    // ── 创造模式专用 ──
    /// 工具栏/面板背景
    pub const TOOLBAR_BG: Color32 = Color32::from_rgba_premultiplied(10, 12, 22, 200);
    /// 物品槽默认背景
    pub const SLOT_BG: Color32 = Color32::from_rgba_premultiplied(22, 24, 40, 200);
    /// 物品槽悬停背景
    pub const SLOT_HOVER: Color32 = Color32::from_rgba_premultiplied(40, 42, 65, 220);
    /// 物品槽选中背景
    pub const SLOT_SELECTED: Color32 = Color32::from_rgba_premultiplied(55, 40, 12, 220);
    /// 标签页激活指示条颜色
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(255, 200, 50);
    /// 标签页悬停指示条颜色
    pub const TAB_HOVER: Color32 = Color32::from_rgb(120, 100, 40);
    /// 热键标签色
    pub const KEY_HINT: Color32 = Color32::from_rgba_premultiplied(180, 180, 200, 160);
}

// ═══════════════════════════════════════════
// 布局组件 — CSS 卡片 / Flex / Grid 辅助
// ═══════════════════════════════════════════

/// 全屏遮罩 + 居中卡片容器（类比 CSS: .overlay { display:flex; justify-content:center; align-items:center }）
fn overlay_card(ctx: &egui::Context, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(egui::Frame { fill: theme::BG_DARK, ..Default::default() })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let av = ui.available_width();
                let cw = av.min(theme::CARD_MAX_W);
                let gap = (av - cw) * 0.5;

                ui.add_space(60.0);
                ui.horizontal(|ui| {
                    ui.add_space(gap);
                    // CSS card: background + border + border-radius + box-shadow
                    egui::Frame {
                        fill: theme::BG_PANEL,
                        stroke: egui::Stroke::new(1.0, theme::BORDER),
                        corner_radius: egui::CornerRadius::same(theme::PANEL_RADIUS),
                        inner_margin: egui::Margin::symmetric(48, 36),
                        shadow: egui::Shadow {
                            offset: [0, 8],
                            blur: 32,
                            spread: 0,
                            color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 80),
                        },
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        ui.set_max_width(cw);
                        // Flex column, centered (CSS: display:flex; flex-direction:column; align-items:center)
                        ui.vertical_centered(|ui| {
                            add_contents(ui);
                        });
                    });
                });
                ui.add_space(60.0);
            });
        });
}

/// 主题按钮 — display:inline-flex; align-items:center; justify-content:center
/// 支持响应式宽度（最多 BTN_WIDTH）
fn theme_button(ui: &mut egui::Ui, text: &str, font_size: f32, accent: bool) -> bool {
    let (bg, hover_bg) = if accent { (theme::BTN_ACCENT_BG, theme::BTN_ACCENT_HOVER) } else { (theme::BTN_BG, theme::BTN_HOVER) };
    let (text_color, txt_accent) = if accent { (theme::TEXT_ACCENT, theme::TEXT_ACCENT) } else { (theme::TEXT_PRIMARY, theme::BORDER_FOCUS) };
    let w = ui.available_rect_before_wrap().width().min(theme::BTN_WIDTH);
    let round = egui::CornerRadius::same(theme::CORNER_RADIUS);
    let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(w, theme::BTN_HEIGHT), egui::Sense::click());

    if ui.is_rect_visible(pos) {
        let fill = if resp.hovered() { hover_bg } else { bg };
        let border_c = if resp.hovered() { txt_accent } else { theme::BORDER };
        ui.painter().rect_filled(pos, round, fill);
        ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, border_c), egui::StrokeKind::Middle);
        let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), text_color);
        let tp = pos.center() - egui::Vec2::new(galley.size().x * 0.5, galley.size().y * 0.5);
        ui.painter().galley(tp, galley, text_color);
    }
    resp.clicked()
}

/// 小号主题按钮
fn theme_btn_sm(ui: &mut egui::Ui, text: &str, font_size: f32, accent: bool) -> bool {
    let (bg, hover_bg) = if accent { (theme::BTN_ACCENT_BG, theme::BTN_ACCENT_HOVER) } else { (theme::BTN_BG, theme::BTN_HOVER) };
    let (text_color, txt_accent) = if accent { (theme::TEXT_ACCENT, theme::TEXT_ACCENT) } else { (theme::TEXT_PRIMARY, theme::BORDER_FOCUS) };
    let w = ui.available_rect_before_wrap().width().min(theme::BTN_WIDTH);
    let round = egui::CornerRadius::same(theme::CORNER_RADIUS);
    let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(w, theme::BTN_SMALL_H), egui::Sense::click());

    if ui.is_rect_visible(pos) {
        let fill = if resp.hovered() { hover_bg } else { bg };
        let border_c = if resp.hovered() { txt_accent } else { theme::BORDER };
        ui.painter().rect_filled(pos, round, fill);
        ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, border_c), egui::StrokeKind::Middle);
        let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), text_color);
        let tp = pos.center() - egui::Vec2::new(galley.size().x * 0.5, galley.size().y * 0.5);
        ui.painter().galley(tp, galley, text_color);
    }
    resp.clicked()
}

/// 内联按钮组 — CSS display:flex; flex-direction:row; gap:10px
/// 两个按钮平分可用宽度
#[allow(dead_code)]
fn btn_pair(ui: &mut egui::Ui, left: (&str, f32, bool), right: (&str, f32, bool)) -> (bool, bool) {
    let total = ui.available_rect_before_wrap().width().min(theme::BTN_WIDTH);
    let half = (total - 10.0) * 0.5;
    let round = egui::CornerRadius::same(theme::CORNER_RADIUS);

    let mk = |ui: &mut egui::Ui, text: &str, font_size: f32, accent: bool, w: f32| -> bool {
        let (bg, hover_bg) = if accent { (theme::BTN_ACCENT_BG, theme::BTN_ACCENT_HOVER) } else { (theme::BTN_BG, theme::BTN_HOVER) };
        let (tc, ta) = if accent { (theme::TEXT_ACCENT, theme::TEXT_ACCENT) } else { (theme::TEXT_PRIMARY, theme::BORDER_FOCUS) };
        let h = theme::BTN_SMALL_H;
        let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(w, h), egui::Sense::click());
        if ui.is_rect_visible(pos) {
            let fill = if resp.hovered() { hover_bg } else { bg };
            let bc = if resp.hovered() { ta } else { theme::BORDER };
            ui.painter().rect_filled(pos, round, fill);
            ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, bc), egui::StrokeKind::Middle);
            let g = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), tc);
            ui.painter().galley(pos.center() - egui::Vec2::new(g.size().x * 0.5, g.size().y * 0.5), g, tc);
        }
        resp.clicked()
    };

    ui.horizontal(|ui| {
        let a = mk(ui, left.0, left.1, left.2, half);
        ui.add_space(10.0);
        let b = mk(ui, right.0, right.1, right.2, half);
        (a, b)
    }).inner
}

/// 分隔线 — CSS hr 效果
fn theme_hr(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    let r = ui.available_rect_before_wrap();
    let w = r.width().min(theme::BTN_WIDTH);
    let x = r.center().x - w * 0.5;
    let y = r.top() + 8.0;
    ui.painter().line_segment([egui::pos2(x, y), egui::pos2(x + w, y)], egui::Stroke::new(1.0, theme::BORDER));
    ui.add_space(20.0);
}

/// HUD 顶栏 — CSS position:sticky; top:0
fn hud_bar(ctx: &egui::Context, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::TopBottomPanel::top("hud_bar")
        .frame(egui::Frame {
            fill: theme::BG_HUD,
            inner_margin: egui::Margin::symmetric(16, 10),
            ..Default::default()
        })
        .show(ctx, |ui| { ui.horizontal(|ui| add_contents(ui)); });
}

// ═══════════════════════════════════════════
// 字体
// ═══════════════════════════════════════════

fn setup_fonts(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done { return; }
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("msyh".to_owned(), egui::FontData::from_static(include_bytes!("../../assets/fonts/msyh.ttf")).into());

    let prop = fonts.families.entry(egui::FontFamily::Proportional).or_insert_with(Vec::new);
    prop.insert(0, "msyh".to_owned());
    let mono = fonts.families.entry(egui::FontFamily::Monospace).or_insert_with(Vec::new);
    mono.insert(0, "msyh".to_owned());

    ctx.set_fonts(fonts);
    *done = true;
}

// ═══════════════════════════════════════════
// 主菜单 — CSS Flex Column Card Layout
// ═══════════════════════════════════════════
//
//  ┌───────────────────────────┐
//  │          NI               │  ← heading
//  │   从零开始的 3D 冒险       │  ← subtitle
//  │  ───────────────────────  │  ← hr
//  │     [  开始游戏  ]        │  ← primary btn
//  │     [  多人聊天  ]        │  ← secondary btn
//  │  ───────────────────────  │  ← hr
//  │  [ 设置 ]    [ 退出 ]     │  ← flex row pair
//  │  ───────────────────────  │  ← hr
//  │   WASD移动 · ESC暂停      │  ← footer text
//  └───────────────────────────┘

fn main_menu_ui(
    mut contexts: EguiContexts,
    mut phase: ResMut<NextState<GamePhase>>,
    mut start_writer: MessageWriter<StartGameEvent>,
    mut exit_writer: MessageWriter<AppExit>,
    mut load_writer: MessageWriter<LoadGameEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_card(ctx, |ui| {
        // ── 标题区 (flex column, centered) ──
        ui.heading(egui::RichText::new("NI").size(72.0).color(theme::TEXT_ACCENT).strong());
        ui.add_space(4.0);
        ui.label(egui::RichText::new("从零开始的 3D 冒险").size(18.0).color(theme::TEXT_SECONDARY));

        theme_hr(ui);

        // ── 主操作区 (flex column) ──
        if theme_button(ui, "开始游戏", 22.0, true) { start_writer.write(StartGameEvent); }
        ui.add_space(10.0);
        if theme_button(ui, "继续游戏", 20.0, false) {
            load_writer.write(LoadGameEvent { slot: 0 });
        }
        ui.add_space(10.0);
        if theme_button(ui, "多人聊天", 20.0, false) { phase.set(GamePhase::MultiplayerChat); }

        theme_hr(ui);

        // ── 次要操作区 (flex row, two buttons) ──
        // 暂时只放退出，后续可以加设置
        if theme_btn_sm(ui, "退出", 18.0, false) { exit_writer.write(AppExit::Success); }

        ui.add_space(24.0);

        // ── 底部提示 (flex column, muted) ──
        ui.label(egui::RichText::new("WASD 移动 · 空格跳跃 · 鼠标视角 · ESC 暂停").size(13.0).color(theme::TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("收集所有金色光球来完成关卡！").size(13.0).color(theme::TEXT_MUTED));
    });
}

// ═══════════════════════════════════════════
// HUD — CSS Flex Row: [left] auto | [center] flex:1 | [right]
// ═══════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn hud_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    health: Res<PlayerHealth>,
    collectibles: Res<LevelCollectibles>,
    level: Res<LevelConfig>,
    interaction_target: Res<InteractionTarget>,
    collectible_q: Query<&Collectible>,
    bank: Res<ItemBank>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    hud_bar(ctx, |ui| {
        // ── 左区: 关卡名 + 血条 ──
        ui.label(egui::RichText::new(level.current_level.display_name()).size(16.0).strong().color(theme::TEXT_PRIMARY));
        ui.add_space(16.0);
        health_bar(ui, health.current, health.max);

        // ── 右区 (flex: justify-end) ──
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // 分数: value + label
            ui.label(egui::RichText::new(format!("{}", score.0)).size(18.0).color(theme::TEXT_ACCENT).strong());
            ui.label(egui::RichText::new("分数: ").size(14.0).color(theme::TEXT_MUTED));
            ui.add_space(8.0);

            // 收集品
            if collectibles.total > 0 {
                ui.label(egui::RichText::new(format!("{}/{}", collectibles.collected, collectibles.total)).size(16.0).color(theme::TEXT_ACCENT));
                ui.label(egui::RichText::new("收集: ").size(14.0).color(theme::TEXT_MUTED));
            }
        });
    });

    // ── 拾取提示（屏幕底部居中） ──
    if let Some(entity) = interaction_target.0
        && let Ok(collectible) = collectible_q.get(entity)
    {
        let item_name = bank.items.get(&collectible.item_id)
            .map(|d| d.name.as_str())
            .unwrap_or(&collectible.item_id);

        egui::Area::new("pickup_prompt".into())
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -60.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("[E] 拾取 {}", item_name))
                            .size(18.0)
                            .color(theme::TEXT_ACCENT)
                            .strong()
                    );
                });
            });
    }
}

/// 图形化健康条
fn health_bar(ui: &mut egui::Ui, current: u32, max: u32) {
    let ratio = if max > 0 { current as f32 / max as f32 } else { 0.0 };
    let bar_w = 120.0;
    let bar_h = 16.0;
    let (pos, _) = ui.allocate_exact_size(egui::Vec2::new(bar_w + 40.0, bar_h), egui::Sense::hover());

    let fill_c = if ratio > 0.5 { egui::Color32::from_rgb(70, 200, 80) }
    else if ratio > 0.25 { egui::Color32::from_rgb(220, 180, 40) }
    else { egui::Color32::from_rgb(220, 50, 50) };

    let r = egui::CornerRadius::same(4);
    let bg_rect = egui::Rect::from_min_size(egui::pos2(pos.left(), pos.top()), egui::Vec2::new(bar_w, bar_h));
    ui.painter().rect_filled(bg_rect, r, egui::Color32::from_rgb(40, 15, 15));
    if ratio > 0.0 {
        ui.painter().rect_filled(egui::Rect::from_min_size(bg_rect.min, egui::Vec2::new((bar_w * ratio).max(4.0), bar_h)), r, fill_c);
    }
    ui.painter().rect_stroke(bg_rect, r, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 40, 40)), egui::StrokeKind::Middle);

    let txt = format!("{} / {}", current, max);
    let g = ui.painter().layout_no_wrap(txt, egui::FontId::proportional(11.0), theme::TEXT_PRIMARY);
    ui.painter().galley(bg_rect.center() - egui::Vec2::new(g.size().x * 0.5, g.size().y * 0.5), g, theme::TEXT_PRIMARY);
}

// ═══════════════════════════════════════════
// 暂停菜单 — CSS Flex Column Card
// ═══════════════════════════════════════════

fn pause_ui(
    mut contexts: EguiContexts,
    mut phase: ResMut<NextState<GamePhase>>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
    mut save_writer: MessageWriter<SaveGameEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_card(ctx, |ui| {
        ui.heading(egui::RichText::new("暂停").size(48.0).color(theme::TEXT_PRIMARY).strong());
        theme_hr(ui);
        if theme_button(ui, "继续游戏", 22.0, true) { phase.set(GamePhase::Playing); }
        ui.add_space(10.0);
        if theme_btn_sm(ui, "保存游戏", 18.0, false) {
            save_writer.write(SaveGameEvent { slot: 0, save_name: "手动存档".to_string() });
        }
        ui.add_space(10.0);
        if theme_btn_sm(ui, "返回主菜单", 18.0, false) { main_menu_writer.write(MainMenuEvent); }
        ui.add_space(16.0);
        ui.label(egui::RichText::new("ESC 继续游戏").size(12.0).color(theme::TEXT_MUTED));
    });
}

// ═══════════════════════════════════════════
// 游戏结束 — CSS Flex Column Card
// ═══════════════════════════════════════════

fn game_over_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    mut restart_writer: MessageWriter<RestartGameEvent>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_card(ctx, |ui| {
        ui.heading(egui::RichText::new("游戏结束").size(48.0).color(theme::TEXT_DANGER).strong());
        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("最终分数: {}", score.0)).size(22.0).color(theme::TEXT_ACCENT));
        theme_hr(ui);
        if theme_button(ui, "重新开始", 22.0, true) { restart_writer.write(RestartGameEvent); }
        ui.add_space(10.0);
        if theme_btn_sm(ui, "返回主菜单", 18.0, false) { main_menu_writer.write(MainMenuEvent); }
    });
}

// ═══════════════════════════════════════════
// 关卡完成 — CSS Flex Column Card
// ═══════════════════════════════════════════

fn level_complete_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    level: Res<LevelConfig>,
    mut next_writer: MessageWriter<NextLevelEvent>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_card(ctx, |ui| {
        ui.heading(egui::RichText::new("关卡完成！").size(48.0).color(theme::TEXT_SUCCESS).strong());
        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("当前分数: {}", score.0)).size(22.0).color(theme::TEXT_ACCENT));
        theme_hr(ui);
        if level.current_level.next().is_some() {
            if theme_button(ui, "下一关", 22.0, true) { next_writer.write(NextLevelEvent); }
            ui.add_space(10.0);
        }
        if theme_btn_sm(ui, "返回主菜单", 18.0, false) { main_menu_writer.write(MainMenuEvent); }
    });
}

// ═══════════════════════════════════════════
// 塔防 HUD — CSS Flex Row (top bar) + Flex Column (side panel)
// ═══════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn td_hud_ui(
    mut contexts: EguiContexts,
    level: Res<LevelConfig>,
    gold: Res<td::TdGold>,
    wave_state: Res<td::TdWaveState>,
    config: Res<td::TdWaveConfig>,
    core_q: Query<&td::DefenseCore>,
    mut start_wave_writer: MessageWriter<td::StartNextWaveEvent>,
    mut purchase_writer: MessageWriter<td::PurchaseTurretEvent>,
    player_q: Query<&Transform, With<crate::player::Player>>,
) {
    if level.current_level != GameLevel::Underground { return; }
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let player_pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // ── 顶栏: CSS Flex Row 水平分布 ──
    hud_bar(ctx, |ui| {
        ui.label(egui::RichText::new("塔防试炼").size(16.0).strong().color(theme::TEXT_ACCENT));
        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("金币: {}", gold.0)).size(16.0).color(theme::TEXT_ACCENT));
        ui.add_space(16.0);

        let phase_text = match wave_state.phase {
            td::WavePhase::Waiting => format!("下一波: {} ({}s)", wave_state.current_wave + 1, (config.wave_cooldown - wave_state.wave_timer.elapsed_secs()).max(0.0) as u32),
            td::WavePhase::Spawning => format!("第 {} 波 生成中... 剩余: {}", wave_state.current_wave, wave_state.enemies_to_spawn + wave_state.enemies_alive),
            td::WavePhase::Active => format!("第 {} 波 战斗中! 剩余敌人: {}", wave_state.current_wave, wave_state.enemies_alive),
            td::WavePhase::Complete => "全部波次完成!".to_string(),
        };
        ui.label(egui::RichText::new(phase_text).size(14.0).color(theme::TEXT_PRIMARY));
        ui.add_space(16.0);

        if let Ok(core) = core_q.single() {
            let ratio = core.current_health / core.max_health;
            let c = if ratio > 0.5 { theme::TEXT_SUCCESS } else if ratio > 0.25 { theme::TEXT_ACCENT } else { theme::TEXT_DANGER };
            ui.label(egui::RichText::new(format!("核心: {:.0}/{:.0}", core.current_health, core.max_health)).size(14.0).color(c));
        }

        // 右端: 开始下一波按钮
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if wave_state.phase == td::WavePhase::Waiting && wave_state.current_wave < config.max_waves {
                let round = egui::CornerRadius::same(theme::CORNER_RADIUS);
                let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(130.0, 30.0), egui::Sense::click());
                if ui.is_rect_visible(pos) {
                    let fill = if resp.hovered() { egui::Color32::from_rgb(70, 55, 20) } else { egui::Color32::from_rgb(50, 40, 15) };
                    ui.painter().rect_filled(pos, round, fill);
                    ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, theme::BORDER), egui::StrokeKind::Middle);
                    let g = ui.painter().layout_no_wrap("开始下一波".to_string(), egui::FontId::proportional(14.0), theme::TEXT_ACCENT);
                    ui.painter().galley(pos.center() - egui::Vec2::new(g.size().x * 0.5, g.size().y * 0.5), g, theme::TEXT_ACCENT);
                }
                if resp.clicked() { start_wave_writer.write(td::StartNextWaveEvent); }
            }
        });
    });

    // ── 右侧建造面板: CSS Flex Column + Card Grid ──
    egui::SidePanel::right("td_shop")
        .frame(egui::Frame {
            fill: theme::BG_PANEL,
            inner_margin: egui::Margin::same(14),
            stroke: egui::Stroke::new(1.0, theme::BORDER),
            ..Default::default()
        })
        .min_width(210.0)
        .show(ctx, |ui| {
            ui.heading(egui::RichText::new("建造炮台").size(18.0).color(theme::TEXT_PRIMARY).strong());
            ui.add_space(12.0);

            // 炮塔卡片 — 每个 card 内 flex row (名称+价格) + 描述行
            td_card(ui, "基础炮台", "伤害: 10 · 射速: 1s", 50, gold.0 >= 50, || {
                purchase_writer.write(td::PurchaseTurretEvent { turret_type: td::TurretType::Basic, position: player_pos });
            });
            ui.add_space(8.0);

            td_card(ui, "速射炮台", "伤害: 5 · 射速: 0.3s", 100, gold.0 >= 100, || {
                purchase_writer.write(td::PurchaseTurretEvent { turret_type: td::TurretType::Rapid, position: player_pos });
            });
            ui.add_space(8.0);

            td_card(ui, "重型炮台", "伤害: 30 · 射速: 2s", 150, gold.0 >= 150, || {
                purchase_writer.write(td::PurchaseTurretEvent { turret_type: td::TurretType::Heavy, position: player_pos });
            });

            // 底部说明区
            td_help(ui);
        });
}

/// 炮塔购买卡片 — 内部分两行: [名称 价格] / [描述]
fn td_card(ui: &mut egui::Ui, name: &str, desc: &str, cost: u32, affordable: bool, on_click: impl FnOnce()) {
    let round = egui::CornerRadius::same(theme::CORNER_RADIUS);
    let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(180.0, 60.0), egui::Sense::click());

    let bg = if affordable {
        if resp.hovered() { egui::Color32::from_rgb(40, 100, 60) } else { egui::Color32::from_rgb(30, 80, 45) }
    } else {
        if resp.hovered() { egui::Color32::from_rgb(80, 40, 40) } else { egui::Color32::from_rgb(60, 30, 30) }
    };

    if ui.is_rect_visible(pos) {
        ui.painter().rect_filled(pos, round, bg);
        let bc = if resp.hovered() { theme::BORDER_FOCUS } else { theme::BORDER };
        ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, bc), egui::StrokeKind::Middle);

        // 第一行 flex: [名称 left] [价格 right]
        let nc = if affordable { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };
        let ng = ui.painter().layout_no_wrap(name.to_string(), egui::FontId::proportional(14.0), nc);
        ui.painter().galley(egui::pos2(pos.left() + 8.0, pos.top() + 6.0), ng, nc);

        let cc = if affordable { theme::TEXT_ACCENT } else { theme::TEXT_DANGER };
        let ct = format!("{} 金", cost);
        let cg = ui.painter().layout_no_wrap(ct, egui::FontId::proportional(13.0), cc);
        ui.painter().galley(egui::pos2(pos.right() - cg.size().x - 8.0, pos.top() + 6.0), cg, cc);

        // 第二行: 描述
        let dc = if affordable { theme::TEXT_SECONDARY } else { theme::TEXT_MUTED };
        let dg = ui.painter().layout_no_wrap(desc.to_string(), egui::FontId::proportional(11.0), dc);
        ui.painter().galley(egui::pos2(pos.left() + 8.0, pos.top() + 30.0), dg, dc);
    }

    if resp.clicked() && affordable { on_click(); }
}

/// 炮台面板底部帮助区
fn td_help(ui: &mut egui::Ui) {
    ui.add_space(20.0);
    let r = ui.available_rect_before_wrap();
    let w = r.width().min(180.0);
    let x = r.center().x - w * 0.5;
    ui.painter().line_segment([egui::pos2(x, r.top() + 4.0), egui::pos2(x + w, r.top() + 4.0)], egui::Stroke::new(1.0, theme::BORDER));
    ui.add_space(12.0);
    ui.label(egui::RichText::new("操作说明").size(13.0).color(theme::TEXT_MUTED));
    ui.add_space(4.0);
    for line in &["点击建造按钮放置炮台", "炮台放在当前位置", "保护中央的水晶核心！"] {
        ui.label(egui::RichText::new(*line).size(11.0).color(theme::TEXT_MUTED));
        ui.add_space(2.0);
    }
}

// ═══════════════════════════════════════════
// 塔防胜利 — CSS Flex Column Card
// ═══════════════════════════════════════════

fn td_victory_ui(
    mut contexts: EguiContexts,
    level: Res<LevelConfig>,
    score: Res<Score>,
    gold: Res<td::TdGold>,
    wave_state: Res<td::TdWaveState>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    if level.current_level != GameLevel::Underground { return; }
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_card(ctx, |ui| {
        ui.heading(egui::RichText::new("塔防胜利！").size(48.0).color(egui::Color32::from_rgb(50, 200, 255)).strong());
        ui.add_space(12.0);
        ui.label(egui::RichText::new(format!("成功抵御了所有 {} 波攻击！", wave_state.current_wave)).size(18.0).color(theme::TEXT_PRIMARY));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(format!("最终分数: {}  |  剩余金币: {}", score.0, gold.0)).size(16.0).color(theme::TEXT_ACCENT));
        theme_hr(ui);
        if theme_button(ui, "返回主菜单", 20.0, false) { main_menu_writer.write(MainMenuEvent); }
    });
}
