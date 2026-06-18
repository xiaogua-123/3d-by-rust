//! 物品放置系统
//!
//! 允许玩家在游戏世界中放置 3D 模型道具（装饰物）。
//!
//! # 操作
//!
//! - **F4**: 切换放置模式
//! - **egui 面板**: 选择要放置的物品类别
//! - **鼠标移动**: 预览物品跟随地面上的准星位置
//! - **左键点击**: 确认放置物品

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::camera::LookState;
use crate::entity_db::{EntityCategory, EntityRegistry, GlbCache};
use crate::game_state::GamePhase;
use crate::level::LevelEntity;
use crate::player::Player;
use crate::ui::theme;

/// 预览幽灵标记 — 跟随鼠标的半透明预览
#[derive(Component)]
struct PlacementGhost;

/// 已放置物品标记 — 所有放置到世界的物品都带此组件
#[derive(Component)]
#[allow(dead_code)]
pub struct PlacedItem {
    pub template_id: String,
}

/// 放置模式状态
#[derive(Resource, Default)]
pub struct PlacementState {
    /// 是否处于放置模式
    pub active: bool,
    /// 当前选中的物品模板 ID
    pub selected_template_id: String,
    /// 预览实体的 Entity（用于更新位置）
    ghost_entity: Option<Entity>,
}

/// 放置最大距离（米）
const PLACEMENT_MAX_DIST: f32 = 20.0;

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementState>()
            .add_systems(Update, (
                placement_toggle,
                update_ghost.run_if(|s: Res<PlacementState>| s.active),
                confirm_placement.run_if(|s: Res<PlacementState>| s.active),
                cleanup_on_exit,
            ))
            .add_systems(EguiPrimaryContextPass, (
                placement_ui.run_if(|s: Res<PlacementState>| s.active),
            ));
    }
}

// ═══════════════════════════════════════════
// 系统实现
// ═══════════════════════════════════════════

/// F4 切换放置模式
fn placement_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut state: ResMut<PlacementState>,
    mut commands: Commands,
) {
    if !matches!(phase.get(), GamePhase::Playing) {
        return;
    }
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }

    state.active = !state.active;
    if state.active {
        info!("[Placement] 进入放置模式 — 选择物品后看向地面，左键放置");
    } else {
        if let Some(e) = state.ghost_entity.take() {
            commands.entity(e).despawn();
        }
        state.selected_template_id.clear();
        info!("[Placement] 退出放置模式");
    }
}

/// 从摄像机发射射线，计算与 y=0 地面的交点
fn ground_hit(
    player_q: &Query<&Transform, With<Player>>,
    camera_q: &Query<(&Transform, &LookState), With<Camera3d>>,
) -> Option<Vec3> {
    let p = player_q.single().ok()?;
    let (cl, _ls) = camera_q.single().ok()?;

    // 世界空间中的相机位置
    let cpos = p.translation + p.rotation * cl.translation;
    // 世界空间视线方向（玩家偏航 + 相机俯仰）
    let fwd = (p.rotation * cl.rotation) * Vec3::NEG_Z;

    // 看向天空 → 不放置
    if fwd.y >= -0.001 {
        return None;
    }

    let t = -cpos.y / fwd.y;
    if !(0.0..=PLACEMENT_MAX_DIST).contains(&t) {
        return None;
    }

    Some(cpos + fwd * t)
}

/// 获取可放置道具列表（有 3D 模型的 Prop 类实体）
fn placeable_props(registry: &EntityRegistry) -> Vec<&String> {
    registry
        .templates
        .iter()
        .filter(|(_, t)| matches!(t.category, EntityCategory::Prop) && t.model.is_some())
        .map(|(id, _)| id)
        .collect()
}

/// egui 面板：选择要放置的物品（CSS Grid 三列布局）
fn placement_ui(
    mut ctx: EguiContexts,
    registry: Res<EntityRegistry>,
    mut state: ResMut<PlacementState>,
) {
    let Ok(ctx) = ctx.ctx_mut() else { return };

    // CSS: position:fixed; top:100px; left:10px; card
    egui::Area::new(egui::Id::new("placement_panel"))
        .fixed_pos(egui::pos2(10.0, 100.0))
        .show(ctx, |ui| {
            egui::Frame {
                fill: theme::BG_PANEL,
                stroke: egui::Stroke::new(1.0, theme::BORDER),
                corner_radius: egui::CornerRadius::same(theme::PANEL_RADIUS),
                inner_margin: egui::Margin::symmetric(16, 16),
                shadow: egui::Shadow {
                    offset: [0, 4],
                    blur: 20,
                    spread: 0,
                    color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 80),
                },
                ..Default::default()
            }
            .show(ui, |ui| {
                ui.set_min_width(260.0);

                // ── Header ──
                ui.label(
                    egui::RichText::new("📦 放置物品")
                        .size(16.0)
                        .color(theme::TEXT_PRIMARY),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("左键放置 · F4 退出")
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );

                // ── 分隔线 ──
                ui.add_space(8.0);
                let r = ui.available_rect_before_wrap();
                let y = r.top() + 4.0;
                ui.painter().line_segment(
                    [egui::pos2(r.left(), y), egui::pos2(r.right(), y)],
                    egui::Stroke::new(1.0, theme::BORDER),
                );
                ui.add_space(12.0);

                // ── 道具网格 ──
                let prop_ids = placeable_props(&registry);
                if prop_ids.is_empty() {
                    ui.label(egui::RichText::new("暂无可用道具").color(theme::TEXT_MUTED));
                    return;
                }

                // CSS Grid: 三列网格
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        egui::Grid::new("prop_grid")
                            .num_columns(3)
                            .min_col_width(72.0)
                            .max_col_width(88.0)
                            .spacing(egui::vec2(6.0, 6.0))
                            .show(ui, |ui| {
                                for (i, id) in prop_ids.iter().enumerate() {
                                    let t = &registry.templates[id.as_str()];
                                    let selected = state.selected_template_id == **id;
                                    if prop_card(ui, &t.display_name, selected) {
                                        state.selected_template_id = (*id).clone();
                                        info!(
                                            "[Placement] 选中物品: {} ({})",
                                            t.display_name, id
                                        );
                                    }
                                    if i % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                // ── 底部选中状态 ──
                ui.add_space(8.0);
                let r2 = ui.available_rect_before_wrap();
                let y2 = r2.top() + 4.0;
                ui.painter().line_segment(
                    [egui::pos2(r2.left(), y2), egui::pos2(r2.right(), y2)],
                    egui::Stroke::new(1.0, theme::BORDER),
                );
                ui.add_space(4.0);
                let selected_name = registry
                    .templates
                    .get(&state.selected_template_id)
                    .map_or("无".to_string(), |t| t.display_name.clone());
                ui.label(
                    egui::RichText::new(format!("已选: {}", selected_name))
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY),
                );
            });
        });
}

/// 道具卡片按钮 — CSS grid item，带选中态和高亮
fn prop_card(ui: &mut egui::Ui, name: &str, selected: bool) -> bool {
    let width = ui.available_width();
    let height = 64.0;
    let round = egui::CornerRadius::same(theme::CORNER_RADIUS);
    let (bg, border_c, text_c) = if selected {
        (theme::BTN_ACCENT_BG, theme::BORDER_FOCUS, theme::TEXT_ACCENT)
    } else {
        (theme::BTN_BG, theme::BORDER, theme::TEXT_SECONDARY)
    };
    let (pos, resp) = ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::click());

    if ui.is_rect_visible(pos) {
        let fill = if resp.hovered() && !selected {
            theme::BTN_HOVER
        } else {
            bg
        };
        let bc = if resp.hovered() { theme::BORDER_FOCUS } else { border_c };
        ui.painter().rect_filled(pos, round, fill);
        ui.painter().rect_stroke(pos, round, egui::Stroke::new(1.0, bc), egui::StrokeKind::Middle);

        let galley = ui.painter().layout_no_wrap(
            name.to_string(),
            egui::FontId::proportional(12.0),
            text_c,
        );
        let tp = pos.center() - egui::Vec2::new(galley.size().x * 0.5, galley.size().y * 0.5);
        ui.painter().galley(tp, galley, text_c);
    }

    resp.clicked()
}

/// 每帧更新幽灵预览的位置
#[allow(clippy::type_complexity)]
fn update_ghost(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registry: Res<EntityRegistry>,
    glb_cache: Res<GlbCache>,
    mut state: ResMut<PlacementState>,
    mut params: ParamSet<(
        Query<&Transform, With<Player>>,
        Query<&Transform, With<Camera3d>>,
        Query<&mut Transform, With<PlacementGhost>>,
    )>,
) {
    // ── 计算地面交点（从 p0 + p1 提取数据后释放引用） ──
    let (p_t, p_r) = {
        let q = params.p0();
        let Ok(p) = q.single() else { return };
        (p.translation, p.rotation)
    };
    let (cl_t, cl_r) = {
        let q = params.p1();
        let Ok(cl) = q.single() else { return };
        (cl.translation, cl.rotation)
    };

    let cpos = p_t + p_r * cl_t;
    let fwd = (p_r * cl_r) * Vec3::NEG_Z;

    let pos = if fwd.y >= -0.001 {
        None
    } else {
        let t = -cpos.y / fwd.y;
        if t <= 0.0 || t > PLACEMENT_MAX_DIST {
            None
        } else {
            Some(cpos + fwd * t)
        }
    };

    let Some(pos) = pos else {
        // 看向天空 → 将幽灵移到不可见位置
        if let Some(e) = state.ghost_entity
            && let Ok(mut t) = params.p2().get_mut(e) {
                t.translation.y = -9999.0;
            }
        return;
    };

    let Some(template) = registry.templates.get(&state.selected_template_id) else {
        return;
    };
    let target =
        Transform::from_translation(pos).with_scale(Vec3::splat(template.scale));

    // ── 已有幽灵 → 更新位置 ──
    if let Some(entity) = state.ghost_entity {
        if let Ok(mut t) = params.p2().get_mut(entity) {
            *t = target;
            return;
        }
        state.ghost_entity = None;
    }

    // ── 创建新幽灵 ──
    let handle = template
        .model
        .as_ref()
        .and_then(|p| glb_cache.handles.get(p))
        .cloned()
        .unwrap_or_else(|| asset_server.load("models/entity/1.glb#Scene0"));

    let entity = commands
        .spawn((
            SceneRoot(handle),
            target,
            PlacementGhost,
            Name::new(format!("ghost_{}", template.id)),
        ))
        .id();
    state.ghost_entity = Some(entity);
}

/// 左键确认放置
#[allow(clippy::too_many_arguments)]
fn confirm_placement(
    buttons: Res<ButtonInput<MouseButton>>,
    mut ctx: EguiContexts,
    asset_server: Res<AssetServer>,
    player_q: Query<&Transform, With<Player>>,
    camera_q: Query<(&Transform, &LookState), With<Camera3d>>,
    registry: Res<EntityRegistry>,
    glb_cache: Res<GlbCache>,
    state: Res<PlacementState>,
    mut commands: Commands,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    // egui 正在使用鼠标（点击面板选择物品）→ 不触发放置
    if let Ok(ctx) = ctx.ctx_mut()
        && ctx.wants_pointer_input() {
            return;
        }

    let Some(pos) = ground_hit(&player_q, &camera_q) else {
        return;
    };
    let Some(template) = registry.templates.get(&state.selected_template_id) else {
        return;
    };

    let handle = template
        .model
        .as_ref()
        .and_then(|p| glb_cache.handles.get(p))
        .cloned()
        .unwrap_or_else(|| asset_server.load("models/entity/1.glb#Scene0"));

    commands.spawn((
        SceneRoot(handle),
        Transform::from_translation(pos).with_scale(Vec3::splat(template.scale)),
        LevelEntity,
        PlacedItem {
            template_id: template.id.clone(),
        },
        Name::new(format!("placed_{}", template.id)),
    ));

    info!(
        "[Placement] 放置物品: {} 于 ({:.1}, {:.1}, {:.1})",
        template.display_name, pos.x, pos.y, pos.z
    );
}

/// 离开 Playing 状态时自动清理放置模式
fn cleanup_on_exit(
    phase: Res<State<GamePhase>>,
    mut state: ResMut<PlacementState>,
    mut commands: Commands,
) {
    if !matches!(phase.get(), GamePhase::Playing) && state.active {
        state.active = false;
        if let Some(e) = state.ghost_entity.take() {
            commands.entity(e).despawn();
        }
        state.selected_template_id.clear();
    }
}
