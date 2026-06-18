# Phase 2: 系统解耦 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task.

**Goal:** 拆分超限文件（creative.rs 953行、dialogue.rs 855行），合并两套碰撞系统，事件化系统间通信

**Architecture:** 将大文件按功能提取为独立子模块，用 `mod.rs` 聚合导出；逐步淘汰旧 `CollisionShape` 并用新 `Collider` 系统替代；确保所有跨系统数据流通过 Bevy Events 而不是直接组件访问。

**Tech Stack:** Rust + Bevy 0.18, `cargo check` 验证每步

**前提条件:** Phase 1 模块重组已完成，旧文件已清理，`cargo check` 通过。

---

### 文件映射表

| 当前路径 | 新路径 |
|----------|--------|
| `src/tools/creative.rs` | `src/tools/creative/mod.rs` (插件+导出) + `src/tools/creative/state.rs` (组件+资源) + `src/tools/creative/systems.rs` (切换/放置/删除/保存) + `src/tools/creative/ui.rs` (Hotbar UI) |
| `src/game/dialogue.rs` | `src/game/dialogue/mod.rs` (插件+导出) + `src/game/dialogue/types.rs` (数据结构) + `src/game/dialogue/events.rs` (消息) + `src/game/dialogue/branch.rs` (条件/效果) + `src/game/dialogue/loader.rs` (加载) + `src/game/dialogue/quest.rs` (任务追踪) + `src/game/dialogue/systems.rs` (对话状态机) + `src/game/dialogue/ui.rs` (渲染) |
| `src/physics/collision/shape.rs` | 合并到 `collider.rs` + `manager.rs` 后删除 |

---

### Task 1: 拆分 creative.rs → creative/ 子模块

**说明：** 将 953 行的创造模式文件按功能拆分为 4 个文件。`creative/mod.rs` 作为插件入口和重导出，`state.rs` 存放组件和资源，`systems.rs` 存放所有系统函数，`ui.rs` 存放 UI 绘制代码。

**Files:**
- Create: `src/tools/creative/mod.rs`
- Create: `src/tools/creative/state.rs`
- Create: `src/tools/creative/systems.rs`
- Create: `src/tools/creative/ui.rs`
- Remove: `src/tools/creative.rs`

- [ ] **Step 1: 创建 `creative/state.rs`**

从原 `creative.rs` 提取组件和资源定义：

```rust
//! 创造模式 — 组件与资源定义

use bevy::prelude::*;
use crate::entity_db::EntityCategory;

/// 创造模式下放置的物体标记
#[derive(Component)]
pub struct CreativePlacedItem {
    pub template_id: String,
    #[allow(dead_code)]
    pub saved: bool,
}

/// 幽灵预览标记
#[derive(Component)]
pub struct CreativeGhost;

/// 创造模式状态
#[derive(Resource)]
pub struct CreativeState {
    pub selected_slot: usize,
    pub category_index: usize,
    pub current_items: Vec<String>,
    pub current_item_names: Vec<String>,
    pub current_item_categories: Vec<EntityCategory>,
    pub categories: Vec<String>,
    pub category_items: Vec<Vec<String>>,
    pub ghost_entity: Option<Entity>,
    pub ghost_material: Option<Handle<StandardMaterial>>,
    pub ghost_mesh: Option<Handle<Mesh>>,
    pub grid_snap: bool,
    pub show_labels: bool,
    pub show_level: bool,
    pub dirty: bool,
    pub camera_entity: Option<Entity>,
    pub next_id: u64,
}

impl Default for CreativeState {
    fn default() -> Self {
        Self {
            selected_slot: 0,
            category_index: 0,
            current_items: Vec::new(),
            current_item_names: Vec::new(),
            current_item_categories: Vec::new(),
            categories: vec!["道具".into(), "NPC".into(), "敌人".into(), "收集品".into()],
            category_items: vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            ghost_entity: None,
            ghost_material: None,
            ghost_mesh: None,
            grid_snap: false,
            show_labels: true,
            show_level: true,
            dirty: false,
            camera_entity: None,
            next_id: 0,
        }
    }
}
```

- [ ] **Step 2: 创建 `creative/systems.rs`**

从原 `creative.rs` 提取所有系统函数：

```rust
//! 创造模式 — 系统函数（切换、进入/退出、放置、删除、保存）

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseScroll;
use crate::camera::{CameraController, LookState};
use crate::entity_db::{EntityCategory, EntityRegistry, GlbCache};
use crate::game_state::GamePhase;
use crate::world::level_tool::{LevelToolConfig, ProximityModelDef};
use crate::player::Player;
use crate::tools::creative::state::*;

pub fn creative_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut state: ResMut<CreativeState>,
) {
    if !keys.just_pressed(KeyCode::F6) { return; }
    match phase.get() {
        GamePhase::Playing => {
            state.dirty = false;
            next_phase.set(GamePhase::Creative);
            info!("[Creative] 进入创造模式");
        }
        GamePhase::Creative => {
            next_phase.set(GamePhase::Playing);
            info!("[Creative] 退出创造模式");
        }
        _ => {}
    }
}

pub fn enter_creative(
    mut state: ResMut<CreativeState>,
    registry: Res<EntityRegistry>,
    asset_server: Res<AssetServer>,
    player_q: Query<&Transform, With<Player>>,
    player_cam_q: Query<Entity, (With<Camera3d>, With<LookState>)>,
    items_q: Query<Entity, With<CreativePlacedItem>>,
    mut commands: Commands,
    mut cursor: Single<&mut bevy::window::CursorOptions>,
) {
    for entity in &player_cam_q {
        commands.entity(entity).insert(Visibility::Hidden);
    }

    state.category_items = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for (id, template) in &registry.templates {
        let idx = match template.category {
            EntityCategory::Prop => 0,
            EntityCategory::Npc => 1,
            EntityCategory::Enemy => 2,
            EntityCategory::Collectible => 3,
            _ => continue,
        };
        state.category_items[idx].push(id.clone());
    }
    state.category_index = 0;
    state.current_items = state.category_items.get(0).cloned().unwrap_or_default();
    state.selected_slot = 0;
    state.current_item_names = state.current_items.iter()
        .map(|id| registry.templates.get(id.as_str())
            .map_or(id.clone(), |t| t.display_name.clone()))
        .collect();
    state.current_item_categories = state.current_items.iter()
        .map(|id| registry.templates.get(id.as_str())
            .map_or(EntityCategory::Prop, |t| t.category.clone()))
        .collect();

    cursor.grab_mode = bevy::window::CursorGrabMode::None;
    cursor.visible = true;

    let (player_pos, player_yaw, player_pitch) = player_q
        .single()
        .map(|t| {
            let (y, p, _) = t.rotation.to_euler(EulerRot::YXZ);
            (t.translation, y, p)
        })
        .unwrap_or((Vec3::ZERO, 0.0, 0.0));

    let mut ctl = CameraController::default();
    ctl.yaw = player_yaw;
    ctl.pitch = player_pitch;

    let cam_entity = commands.spawn((
        Camera3d::default(),
        Camera { order: 2, ..default() },
        Msaa::Off,
        ctl,
        Transform::from_translation(player_pos + Vec3::new(0.0, 2.0, 0.0))
            .with_rotation(Quat::from_euler(EulerRot::YXZ, player_yaw, player_pitch, 0.0)),
        Name::new("CreativeCamera"),
    )).id();
    state.camera_entity = Some(cam_entity);

    for entity in &items_q {
        commands.entity(entity).despawn();
    }
    creative_cleanup_and_load(&mut state, &registry, &asset_server, &mut commands);
}

pub fn exit_creative(
    mut state: ResMut<CreativeState>,
    player_cam_q: Query<Entity, (With<Camera3d>, With<LookState>)>,
    mut commands: Commands,
    mut cursor: Single<&mut bevy::window::CursorOptions>,
    phase: Res<State<GamePhase>>,
) {
    for entity in &player_cam_q {
        commands.entity(entity).insert(Visibility::Inherited);
    }
    if let Some(e) = state.ghost_entity.take() {
        commands.entity(e).despawn();
    }
    if let Some(e) = state.camera_entity.take() {
        commands.entity(e).despawn();
    }
    if phase.get() == &GamePhase::Playing {
        cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn creative_cleanup_and_load(
    _state: &mut CreativeState,
    _registry: &EntityRegistry,
    asset_server: &AssetServer,
    commands: &mut Commands,
) {
    let content = match std::fs::read_to_string("assets/level/level_config.ron") {
        Ok(c) => c,
        Err(_) => { info!("[Creative] 无可加载的已保存物体"); return; }
    };
    let config: LevelToolConfig = match ron::de::from_str(&content) {
        Ok(c) => c,
        Err(e) => { warn!("[Creative] 解析关卡配置失败: {}", e); return; }
    };
    let level_id = "Demo";
    let Some(level_def) = config.levels.get(level_id) else { return };
    for model in &level_def.proximity_models {
        let pos = Vec3::new(model.position.0, model.position.1, model.position.2);
        let scene_path = if model.path.contains('#') { model.path.clone() }
            else { format!("{}#Scene0", model.path) };
        commands.spawn((
            SceneRoot(asset_server.load::<Scene>(&scene_path)),
            Transform::from_translation(pos).with_scale(Vec3::splat(model.scale)),
            CreativePlacedItem { template_id: model.id.clone(), saved: true },
            Name::new(format!("creative_{}", model.id)),
        ));
    }
}

pub fn toggle_grid_snap(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CreativeState>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        state.grid_snap = !state.grid_snap;
    }
}

pub fn toggle_labels(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CreativeState>,
) {
    if keys.just_pressed(KeyCode::KeyH) {
        state.show_labels = !state.show_labels;
    }
}

pub fn toggle_level_visibility(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CreativeState>,
    mut level_q: Query<&mut Visibility, With<crate::level::LevelEntity>>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        state.show_level = !state.show_level;
        let vis = if state.show_level { Visibility::Inherited } else { Visibility::Hidden };
        for mut v in &mut level_q { *v = vis; }
    }
}

pub fn creative_scroll(
    mut state: ResMut<CreativeState>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if mouse_scroll.delta.y != 0.0 {
        let count = state.current_items.len().min(10);
        if count > 0 {
            state.selected_slot = if mouse_scroll.delta.y > 0.0 {
                (state.selected_slot + 1) % count
            } else {
                if state.selected_slot == 0 { count - 1 } else { state.selected_slot - 1 }
            };
        }
    }
}

fn creative_ground_hit(
    camera_q: &Query<&GlobalTransform, (With<Camera3d>, With<CameraController>)>,
) -> Option<Vec3> {
    let Ok(gt) = camera_q.single() else { return None };
    let origin = gt.translation();
    let fwd = gt.forward();
    if fwd.y >= -0.001 { return None; }
    let t = -origin.y / fwd.y;
    if t < 0.0 || t > 50.0 { return None; }
    Some(origin + fwd * t)
}

pub fn creative_ghost(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<CreativeState>,
    registry: Res<EntityRegistry>,
    camera_q: Query<&GlobalTransform, (With<Camera3d>, With<CameraController>)>,
    mut ghost_q: Query<&mut Transform, With<CreativeGhost>>,
) {
    let Some(hit_pos) = creative_ground_hit(&camera_q) else {
        if let Some(e) = state.ghost_entity {
            if let Ok(mut t) = ghost_q.get_mut(e) { t.translation.y = -9999.0; }
        }
        return;
    };
    let Some(template) = state.current_items.get(state.selected_slot)
        .and_then(|id| registry.templates.get(id.as_str()))
    else { return };

    let final_pos = if state.grid_snap {
        Vec3::new((hit_pos.x + 0.5).floor(), 0.0, (hit_pos.z + 0.5).floor())
    } else {
        Vec3::new(hit_pos.x, 0.0, hit_pos.z)
    };
    let target = Transform::from_translation(final_pos).with_scale(Vec3::splat(template.scale));

    let ghost_entity = state.ghost_entity;
    if let Some(entity) = ghost_entity {
        if let Ok(mut t) = ghost_q.get_mut(entity) { *t = target; return; }
        state.ghost_entity = None;
    }
    let ghost_mat = state.ghost_material.get_or_insert_with(|| {
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.3, 0.8, 0.3, 0.35),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })
    }).clone();
    let ghost_mesh = state.ghost_mesh.get_or_insert_with(|| {
        meshes.add(Cuboid::new(0.8, 0.8, 0.8))
    }).clone();
    let entity = commands.spawn((
        Mesh3d(ghost_mesh), MeshMaterial3d(ghost_mat), target,
        CreativeGhost, Name::new("creative_ghost"),
    )).id();
    state.ghost_entity = Some(entity);
}

pub fn creative_placement(
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut glb_cache: ResMut<GlbCache>,
    mut state: ResMut<CreativeState>,
    registry: Res<EntityRegistry>,
    camera_q: Query<&GlobalTransform, (With<Camera3d>, With<CameraController>)>,
    mut egui_ctx: bevy_egui::EguiContexts,
) {
    if !buttons.just_pressed(MouseButton::Left) { return; }
    if let Ok(ctx) = egui_ctx.ctx_mut() {
        if ctx.wants_pointer_input() { return; }
    }
    let Some(hit_pos) = creative_ground_hit(&camera_q) else { return };
    let Some(template) = state.current_items.get(state.selected_slot)
        .and_then(|id| registry.templates.get(id.as_str()))
    else { return };
    let final_pos = if state.grid_snap {
        Vec3::new((hit_pos.x + 0.5).floor(), 0.0, (hit_pos.z + 0.5).floor())
    } else {
        Vec3::new(hit_pos.x, 0.0, hit_pos.z)
    };
    let handle = template.model.as_ref().map(|p| {
        glb_cache.handles.get(p).cloned().unwrap_or_else(|| {
            let path = if p.contains('#') { p.clone() } else { format!("{}#Scene0", p) };
            let handle = asset_server.load::<Scene>(&path);
            glb_cache.handles.insert(p.clone(), handle.clone());
            handle
        })
    });
    if let Some(handle) = handle {
        let id = state.next_id;
        state.next_id += 1;
        commands.spawn((
            SceneRoot(handle),
            Transform::from_translation(final_pos).with_scale(Vec3::splat(template.scale)),
            CreativePlacedItem { template_id: template.id.clone(), saved: false },
            Name::new(format!("creative_{}_{}", template.id, id)),
        ));
        state.dirty = true;
    }
}

pub fn creative_remove(
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut state: ResMut<CreativeState>,
    camera_q: Query<&GlobalTransform, (With<Camera3d>, With<CameraController>)>,
    items_q: Query<(Entity, &GlobalTransform, &CreativePlacedItem)>,
    mut egui_ctx: bevy_egui::EguiContexts,
) {
    if !buttons.just_pressed(MouseButton::Right) { return; }
    if let Ok(ctx) = egui_ctx.ctx_mut() {
        if ctx.wants_pointer_input() { return; }
    }
    let Ok(cam_gt) = camera_q.single() else { return };
    let origin = cam_gt.translation();
    let fwd = cam_gt.forward();
    if fwd.y >= -0.001 { return; }
    let t = -origin.y / fwd.y;
    if t < 0.0 || t > 50.0 { return; }
    let hit_pos = origin + fwd * t;
    let mut best: Option<(Entity, f32)> = None;
    for (entity, gt, _item) in &items_q {
        let dist = gt.translation().xz().distance(hit_pos.xz());
        if dist < 2.0 {
            let is_better = match best {
                Some((_, best_dist)) => dist < best_dist,
                None => true,
            };
            if is_better { best = Some((entity, dist)); }
        }
    }
    if let Some((entity, _)) = best {
        commands.entity(entity).despawn();
        state.dirty = true;
    }
}

pub fn creative_save(
    keys: Res<ButtonInput<KeyCode>>,
    registry: Res<EntityRegistry>,
    mut state: ResMut<CreativeState>,
    items_q: Query<(&Transform, &CreativePlacedItem)>,
) {
    if !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)) { return; }
    if !keys.just_pressed(KeyCode::KeyS) { return; }
    let mut models: Vec<ProximityModelDef> = Vec::new();
    for (tf, item) in &items_q {
        let model_path = registry.templates.get(&item.template_id)
            .and_then(|t| t.model.clone());
        let Some(path) = model_path else {
            warn!("[Creative] 跳过保存 {}：模板无模型路径", item.template_id);
            continue;
        };
        models.push(ProximityModelDef {
            id: item.template_id.clone(), path,
            position: (tf.translation.x, tf.translation.y, tf.translation.z),
            scale: tf.scale.x, load_distance: 8.0, unload_distance: 16.0,
            label: Some((item.template_id.clone(), 4.0)),
        });
    }
    if models.is_empty() { info!("[Creative] 无物体可保存"); return; }
    let content = match std::fs::read_to_string("assets/level/level_config.ron") {
        Ok(c) => c,
        Err(e) => { error!("[Creative] 无法读取关卡配置: {}", e); return; }
    };
    let mut config: LevelToolConfig = match ron::de::from_str(&content) {
        Ok(c) => c,
        Err(e) => { error!("[Creative] 无法解析关卡配置: {}", e); return; }
    };
    if let Some(level) = config.levels.get_mut("Demo") {
        level.proximity_models = models;
    }
    let ron_str = ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default())
        .unwrap_or_default();
    match std::fs::write("assets/level/level_config.ron", &ron_str) {
        Ok(()) => { state.dirty = false; info!("[Creative] 已保存"); }
        Err(e) => error!("[Creative] 保存失败: {}", e),
    }
}
```

- [ ] **Step 3: 创建 `creative/ui.rs`**

从原 `creative.rs` 提取所有 UI 函数：

```rust
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

    // 数字键 1-9,0 选择
    let show_count = state.current_items.len().min(10);
    if show_count > 0 {
        for n in 0..show_count {
            let key = if n < 9 { match n { 0 => KeyCode::Digit1, 1 => KeyCode::Digit2, 2 => KeyCode::Digit3, 3 => KeyCode::Digit4, 4 => KeyCode::Digit5, 5 => KeyCode::Digit6, 6 => KeyCode::Digit7, 7 => KeyCode::Digit8, 8 => KeyCode::Digit9, _ => unreachable!(), } } else { KeyCode::Digit0 };
            if keys.just_pressed(key) { state.selected_slot = n; }
        }
    }

    // 分类标签栏（顶部）
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

    // Hotbar 物品槽（底部居中）
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
                            for (i, item_id) in current_items.iter().enumerate().take(10) {
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

    // 左上角信息面板
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
                    if let Some(id) = &state.current_items.get(state.selected_slot).cloned() {
                        if let Some(t) = registry.templates.get(id.as_str()) {
                            ui.label(egui::RichText::new(format!("当前: {}", t.display_name)).size(12.0).color(theme::TEXT_PRIMARY));
                        }
                    }
                });
        });

    // 底部操作提示条
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
```

- [ ] **Step 4: 创建 `creative/mod.rs`**（新插件入口）

```rust
//! 创造模式 — 类似 Minecraft 创造模式的 3D 关卡编辑器
//!
//! F6 切换进入/退出，支持飞行放置/删除物体、保存到 RON。
//! 复用 CameraController 做飞行，复用 EntityRegistry 做物品来源。

mod state;
mod systems;
mod ui;

pub use state::*;
pub use systems::*;

use bevy::prelude::*;
use crate::game_state::GamePhase;

pub struct CreativePlugin;

impl Plugin for CreativePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeState>()
            .add_systems(OnEnter(GamePhase::Creative), enter_creative)
            .add_systems(OnExit(GamePhase::Creative), exit_creative)
            .add_systems(Update, (
                creative_toggle,
                creative_scroll.run_if(in_state(GamePhase::Creative)),
                creative_ghost.run_if(in_state(GamePhase::Creative)),
                creative_placement.run_if(in_state(GamePhase::Creative)),
                creative_remove.run_if(in_state(GamePhase::Creative)),
                creative_save.run_if(in_state(GamePhase::Creative)),
                toggle_grid_snap.run_if(in_state(GamePhase::Creative)),
                toggle_labels.run_if(in_state(GamePhase::Creative)),
                toggle_level_visibility.run_if(in_state(GamePhase::Creative)),
            ))
            .add_systems(Update, (
                ui::creative_hotbar_ui.run_if(in_state(GamePhase::Creative)),
            ));
    }
}
```

- [ ] **Step 5: 删除旧 `creative.rs`**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
rm src/tools/creative.rs
```

- [ ] **Step 6: 运行 cargo check**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

预期：0 errors。

---

### Task 2: 拆分 dialogue.rs → dialogue/ 子模块

**说明：** 将 855 行的对话系统按职责拆分为 7 个文件。`mod.rs` 作为插件入口，`types.rs` 存放数据结构，`events.rs` 存放消息，`branch.rs` 存放条件/效果逻辑，`loader.rs` 存放加载函数，`quest.rs` 存放任务追踪，`systems.rs` 存放对话状态机，`ui.rs` 存放对话 UI 渲染。

**Files:**
- Create: `src/game/dialogue/mod.rs`
- Create: `src/game/dialogue/types.rs`
- Create: `src/game/dialogue/events.rs`
- Create: `src/game/dialogue/branch.rs`
- Create: `src/game/dialogue/loader.rs`
- Create: `src/game/dialogue/quest.rs`
- Create: `src/game/dialogue/systems.rs`
- Create: `src/game/dialogue/ui.rs`
- Remove: `src/game/dialogue.rs`

- [ ] **Step 1: 创建 `dialogue/types.rs`**

```rust
//! 对话系统 — 数据结构定义

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 对话节点
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

/// 对话选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub text: String,
    pub next_id: String,
    #[serde(default)]
    pub condition: Option<DialogueCondition>,
}

/// 对话触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueCondition {
    HasItem(String),
    NoItem(String),
    QuestComplete(String),
    QuestActive(String),
    Flag(String),
    HasVisitedZone(String),
}

/// 对话效果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueEffect {
    GiveItem(String, u32),
    RemoveItem(String, u32),
    SetFlag(String),
    CompleteQuest(String),
    StartQuest(String),
    StartPuzzle(String),
    UnlockDoor(String),
    PlayCutscene(String),
}

/// apply_effects 返回的待处理效果（需要事件 writer）
#[derive(Debug)]
pub enum PendingEffect {
    GiveItem(String, u32),
    RemoveItem(String, u32),
}

/// 完整对话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueConversation {
    pub id: String,
    pub nodes: HashMap<String, DialogueNode>,
}

/// 对话触发器组件（供 NPC 使用）
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct DialogueTrigger {
    pub conversation_id: String,
    pub start_node: String,
    pub radius: f32,
}
```

- [ ] **Step 2: 创建 `dialogue/events.rs`**

```rust
//! 对话系统 — 消息定义

use bevy::prelude::*;

/// 开始对话事件
#[derive(Message)]
pub struct StartDialogueEvent {
    pub conversation_id: String,
    pub start_node: String,
}

/// 玩家选择选项
#[derive(Message)]
pub struct DialogueChoiceEvent(pub usize);

/// 玩家推进对话
#[derive(Message)]
pub struct DialogueAdvanceEvent;
```

- [ ] **Step 3: 创建 `dialogue/branch.rs`**

```rust
//! 对话系统 — 条件检查与效果执行

use crate::inventory::Inventory;
use crate::game::dialogue::types::*;

impl DialogueCondition {
    pub fn check(&self, quests: &QuestTracker, inventory: &Inventory) -> bool {
        match self {
            DialogueCondition::HasItem(id) => inventory.has(id),
            DialogueCondition::NoItem(id) => !inventory.has(id),
            DialogueCondition::QuestComplete(id) => quests.completed_quests.contains(id),
            DialogueCondition::QuestActive(id) => quests.active_quests.contains(id),
            DialogueCondition::Flag(f) => quests.flags.contains(f),
            DialogueCondition::HasVisitedZone(id) => quests.flags.contains(&format!("visited_{}", id)),
        }
    }
}

pub fn apply_effects(effects: &[DialogueEffect], quests: &mut QuestTracker) -> Vec<PendingEffect> {
    let mut pending = Vec::new();
    for effect in effects {
        match effect {
            DialogueEffect::StartQuest(id) => {
                if !quests.active_quests.contains(id) { quests.active_quests.push(id.clone()); }
            }
            DialogueEffect::CompleteQuest(id) => {
                quests.active_quests.retain(|q| q != id);
                if !quests.completed_quests.contains(id) { quests.completed_quests.push(id.clone()); }
            }
            DialogueEffect::SetFlag(f) => { if !quests.flags.contains(f) { quests.flags.push(f.clone()); } }
            DialogueEffect::GiveItem(id, amount) => pending.push(PendingEffect::GiveItem(id.clone(), *amount)),
            DialogueEffect::RemoveItem(id, amount) => pending.push(PendingEffect::RemoveItem(id.clone(), *amount)),
            DialogueEffect::StartPuzzle(id) => info!("对话效果: 启动谜题 {}", id),
            DialogueEffect::UnlockDoor(id) => info!("对话效果: 解锁门 {}", id),
            DialogueEffect::PlayCutscene(id) => info!("对话效果: 播放过场 {}", id),
        }
    }
    pending
}
```

- [ ] **Step 4: 创建 `dialogue/loader.rs`**

```rust
//! 对话系统 — 从 RON 文件加载对话和任务

use bevy::prelude::*;
use ron::de::from_reader;
use crate::game::dialogue::types::*;
use std::fs;

pub fn load_dialogues(mut bank: ResMut<DialogueBank>) {
    let dir = "assets/dialogue";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                match fs::File::open(&path) {
                    Ok(file) => match from_reader::<_, DialogueConversation>(file) {
                        Ok(conv) => { bank.conversations.insert(conv.id.clone(), conv); }
                        Err(e) => error!("解析对话文件失败 {:?}: {}", path, e),
                    },
                    Err(e) => error!("打开对话文件失败 {:?}: {}", path, e),
                }
            }
        }
    } else {
        let _ = fs::create_dir_all(dir);
        error!("对话目录不存在，已创建 assets/dialogue/，请放入 .ron 对话文件");
    }
}

pub fn load_quests(mut bank: ResMut<QuestBank>) {
    let quests = vec![
        QuestDef {
            id: "investigate_forest".into(),
            name: "调查森林".into(),
            description: "东边的森林出现了怪物，前往调查。".into(),
            subgoals: vec![
                SubgoalDef { description: "与村长交谈".into(), completion_flag: None },
                SubgoalDef { description: "前往东翼走廊".into(), completion_flag: Some("visited_east_wing".into()) },
                SubgoalDef { description: "调查森林中的异常".into(), completion_flag: None },
            ],
        },
    ];
    for q in quests { bank.quests.insert(q.id.clone(), q); }
}
```

- [ ] **Step 5: 创建 `dialogue/quest.rs`**

```rust
//! 对话系统 — 任务追踪与通知

use bevy::prelude::*;

/// 子目标定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgoalDef {
    pub description: String,
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

/// 任务通知
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
            let name = quest_bank.quests.get(q).map_or(q.as_str(), |d| d.name.as_str());
            notif.message = Some(format!("新任务: {}", name));
            notif.timer = 4.0;
        }
    }
    for q in &quests.completed_quests {
        if !last_completed.contains(q) {
            let name = quest_bank.quests.get(q).map_or(q.as_str(), |d| d.name.as_str());
            notif.message = Some(format!("任务完成: {}！", name));
            notif.timer = 4.0;
        }
    }
    *last_active = quests.active_quests.clone();
    *last_completed = quests.completed_quests.clone();
}

/// 自动清除任务通知
pub fn quest_notification_clear(
    time: Res<Time>,
    mut notif: ResMut<QuestNotification>,
) {
    if notif.message.is_some() {
        notif.timer -= time.delta_secs();
        if notif.timer <= 0.0 { notif.message = None; notif.timer = 0.0; }
    }
}
```

注意：`quest.rs` 需要使用 `use serde::{Deserialize, Serialize};` — 在文件顶部添加。

- [ ] **Step 6: 创建 `dialogue/systems.rs`**

包含对话状态机：`dialogue_visible`, `handle_start_dialogue`, `handle_dialogue_choice`, `handle_dialogue_advance`, `dialogue_input`, `typewriter_tick`, `end_dialogue`。

```rust
//! 对话系统 — 状态机与输入处理

use bevy::prelude::*;
use crate::game_state::{GamePhase, OverlayState};
use crate::inventory::{Inventory, GiveItemEvent, RemoveItemEvent};
use crate::game::dialogue::types::*;
use crate::game::dialogue::events::*;
use crate::game::dialogue::branch::apply_effects;
use crate::game::dialogue::loader::*;

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
        if let Some(conv) = bank.conversations.get(&ev.conversation_id) {
            if let Some(node) = conv.nodes.get(&ev.start_node) {
                manager.active_conversation_id = Some(ev.conversation_id.clone());
                manager.current_node_id = Some(ev.start_node.clone());
                manager.display_text = String::new();
                manager.char_index = 0;
                manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                manager.text_complete = false;
                manager.visible = true;
                for cmd in apply_effects(&node.on_enter, &mut quests) {
                    match cmd {
                        PendingEffect::GiveItem(id, amount) => give_item_writer.write(GiveItemEvent { item_id: id, amount }),
                        PendingEffect::RemoveItem(id, amount) => remove_item_writer.write(RemoveItemEvent { item_id: id, amount }),
                    }
                }
                next_phase.set(GamePhase::Dialoguing);
            }
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
        let Some(conv_id) = &manager.active_conversation_id.clone() else { continue };
        let Some(conv) = bank.conversations.get(conv_id) else { continue };
        let Some(current_id) = &manager.current_node_id.clone() else { continue };
        let Some(current_node) = conv.nodes.get(current_id) else { continue };
        let choice = &current_node.choices[ev.0];
        if let Some(cond) = &choice.condition { if !cond.check(&quests, &*inventory) { continue; } }
        if let Some(next_node) = conv.nodes.get(&choice.next_id) {
            manager.current_node_id = Some(choice.next_id.clone());
            manager.display_text = String::new();
            manager.char_index = 0;
            manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
            manager.text_complete = false;
            for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                match cmd {
                    PendingEffect::GiveItem(id, amount) => give_item_writer.write(GiveItemEvent { item_id: id, amount }),
                    PendingEffect::RemoveItem(id, amount) => remove_item_writer.write(RemoveItemEvent { item_id: id, amount }),
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
        let Some(conv_id) = &manager.active_conversation_id.clone() else { continue };
        let Some(conv) = bank.conversations.get(conv_id) else { continue };
        let Some(current_id) = &manager.current_node_id.clone() else { continue };
        let Some(current_node) = conv.nodes.get(current_id) else { continue };
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
                            PendingEffect::GiveItem(id, amount) => give_item_writer.write(GiveItemEvent { item_id: id, amount }),
                            PendingEffect::RemoveItem(id, amount) => remove_item_writer.write(RemoveItemEvent { item_id: id, amount }),
                        }
                    }
                }
            } else { end_dialogue(&mut manager, &mut next_phase); }
        } else {
            if let Some(node) = conv.nodes.get(current_id) {
                manager.display_text = node.text.clone();
                manager.char_index = node.text.chars().count();
                manager.text_complete = true;
            }
        }
    }
}

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
    if !manager.visible || overlay.active.is_some() { return; }
    let Some(conv_id) = &manager.active_conversation_id.clone() else { return };
    let Some(conv) = bank.conversations.get(conv_id) else { return };
    let Some(current_id) = &manager.current_node_id.clone() else { return };
    let Some(current_node) = conv.nodes.get(current_id) else { return };

    let advance = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::KeyF);
    let number_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    ];

    if manager.text_complete && !current_node.choices.is_empty() {
        for (i, key) in number_keys.iter().enumerate() {
            if keys.just_pressed(*key) && i < current_node.choices.len() {
                let choice = &current_node.choices[i];
                if choice.condition.as_ref().map_or(true, |cond| cond.check(&quests, &*inventory)) {
                    if let Some(next_node) = conv.nodes.get(&choice.next_id) {
                        manager.current_node_id = Some(choice.next_id.clone());
                        manager.display_text = String::new();
                        manager.char_index = 0;
                        manager.text_timer = Timer::from_seconds(0.03, TimerMode::Repeating);
                        manager.text_complete = false;
                        manager.debug_visible = false;
                        for cmd in apply_effects(&next_node.on_enter, &mut quests) {
                            match cmd {
                                PendingEffect::GiveItem(id, amount) => give_item_writer.write(GiveItemEvent { item_id: id, amount }),
                                PendingEffect::RemoveItem(id, amount) => remove_item_writer.write(RemoveItemEvent { item_id: id, amount }),
                            }
                        }
                    }
                    return;
                }
            }
        }
    }
    if advance && !current_node.choices.is_empty() { return; }
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
                            PendingEffect::GiveItem(id, amount) => give_item_writer.write(GiveItemEvent { item_id: id, amount }),
                            PendingEffect::RemoveItem(id, amount) => remove_item_writer.write(RemoveItemEvent { item_id: id, amount }),
                        }
                    }
                }
            } else { end_dialogue(&mut manager, &mut next_phase); }
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
    if manager.text_complete { return; }
    let Some(conv_id) = &manager.active_conversation_id else { return };
    let Some(conv) = bank.conversations.get(conv_id) else { return };
    let Some(current_id) = &manager.current_node_id else { return };
    let Some(current_node) = conv.nodes.get(current_id) else { return };
    manager.text_timer.tick(time.delta());
    let ticks = manager.text_timer.times_finished_this_tick() as usize;
    if ticks > 0 {
        let chars: Vec<char> = current_node.text.chars().collect();
        let target = (manager.char_index + ticks).min(chars.len());
        manager.display_text = chars[..target].iter().collect();
        manager.char_index = target;
        if manager.char_index >= chars.len() { manager.text_complete = true; }
    }
}

pub fn end_dialogue(manager: &mut DialogueManager, next_phase: &mut NextState<GamePhase>) {
    manager.visible = false;
    manager.active_conversation_id = None;
    manager.current_node_id = None;
    manager.display_text.clear();
    manager.char_index = 0;
    manager.text_complete = false;
    next_phase.set(GamePhase::Playing);
}
```

- [ ] **Step 7: 创建 `dialogue/ui.rs`**

```rust
//! 对话系统 — UI 渲染

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::inventory::Inventory;
use crate::game::dialogue::types::*;
use crate::game::dialogue::events::*;
use crate::game::dialogue::systems::DialogueManager;

pub fn dialogue_ui(
    mut contexts: EguiContexts,
    bank: Res<DialogueBank>,
    manager: Res<DialogueManager>,
    quests: Res<QuestTracker>,
    inventory: Res<Inventory>,
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

    egui::TopBottomPanel::bottom("dialogue_panel")
        .frame(panel_frame)
        .min_height(120.0)
        .max_height(180.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new(&current_node.speaker).size(16.0).strong().color(egui::Color32::from_rgb(255, 200, 80)));
            ui.add_space(6.0);

            let display = if manager.text_complete { current_node.text.clone() } else { format!("{}▌", manager.display_text) };
            let text_response = ui.add(egui::Label::new(egui::RichText::new(display).size(14.0).color(egui::Color32::WHITE)).sense(egui::Sense::click()));
            if text_response.clicked() { advance_writer.write(DialogueAdvanceEvent); }
            ui.add_space(8.0);

            if manager.text_complete {
                if !current_node.choices.is_empty() {
                    let valid_choices: Vec<(usize, &DialogueChoice)> = current_node.choices.iter().enumerate()
                        .filter(|(_, c)| c.condition.as_ref().map_or(true, |cond| cond.check(&quests, &*inventory))).collect();
                    if manager.debug_visible {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("[调试] 节点: {}", current_id)).size(11.0).color(egui::Color32::from_rgb(100, 200, 100)));
                        if let Some(next) = &current_node.next { ui.label(egui::RichText::new(format!("  next → {}", next)).size(11.0).color(egui::Color32::GRAY)); }
                        if !current_node.on_enter.is_empty() { ui.label(egui::RichText::new(format!("  on_enter: {:?}", current_node.on_enter)).size(11.0).color(egui::Color32::from_rgb(255, 200, 80))); }
                        ui.label(egui::RichText::new(format!("  有效选项: {}/{}", valid_choices.len(), current_node.choices.len())).size(11.0).color(egui::Color32::from_rgb(150, 200, 255)));
                        for (i, c) in current_node.choices.iter().enumerate() {
                            let valid = c.condition.as_ref().map_or(true, |cond| cond.check(&quests, &*inventory));
                            ui.label(egui::RichText::new(format!("  [{}] → {} {:?}", i, c.next_id, c.condition)).size(11.0).color(if valid { egui::Color32::from_rgb(200, 255, 200) } else { egui::Color32::from_rgb(150, 150, 150) }));
                        }
                    } else if valid_choices.is_empty() {
                        if current_node.next.is_some() { ui.label(egui::RichText::new("按 空格/Enter/F 或点击文字 继续").size(12.0).color(egui::Color32::GRAY)); }
                    } else {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("— 选择一个选项 —").size(11.0).color(egui::Color32::from_rgb(180, 180, 200)));
                        ui.add_space(4.0);
                        for (idx, choice) in &valid_choices {
                            if ui.add_sized(egui::vec2(ui.available_width(), 32.0),
                                egui::Button::new(egui::RichText::new(format!("[{}]  {}", idx + 1, choice.text)).size(13.0).color(egui::Color32::WHITE))
                            ).clicked() { choice_writer.write(DialogueChoiceEvent(*idx)); }
                            ui.add_space(4.0);
                        }
                    }
                } else if current_node.next.is_some() {
                    ui.label(egui::RichText::new("按 空格/Enter/F 或点击文字 继续").size(12.0).color(egui::Color32::GRAY));
                    if ui.add(egui::Button::new(egui::RichText::new("→ 继续").size(13.0).color(egui::Color32::from_rgb(100, 200, 255)))).clicked() {
                        advance_writer.write(DialogueAdvanceEvent);
                    }
                } else {
                    if ui.add(egui::Button::new(egui::RichText::new("关闭").size(13.0).color(egui::Color32::WHITE))).clicked() {
                        advance_writer.write(DialogueAdvanceEvent);
                    }
                }
            }
        });
}
```

- [ ] **Step 8: 创建 `dialogue/mod.rs`**（新插件入口）

```rust
//! 对话系统 — RON 驱动的分支对话与任务管理

mod types;
mod events;
mod branch;
mod loader;
mod quest;
mod systems;
mod ui;

pub use types::*;
pub use events::*;
pub use quest::*;
pub use systems::*;

use bevy::prelude::*;
use crate::game_state::GamePhase;
use crate::inventory::GiveItemEvent;
use crate::inventory::RemoveItemEvent;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DialogueTrigger>()
            .init_resource::<DialogueBank>()
            .init_resource::<QuestTracker>()
            .init_resource::<QuestBank>()
            .init_resource::<QuestNotification>()
            .init_resource::<DialogueManager>()
            .add_message::<StartDialogueEvent>()
            .add_message::<DialogueChoiceEvent>()
            .add_message::<DialogueAdvanceEvent>()
            .add_systems(Startup, (loader::load_dialogues, loader::load_quests))
            .add_systems(Update, (
                handle_start_dialogue,
                handle_dialogue_choice,
                handle_dialogue_advance,
                dialogue_input,
                ui::dialogue_ui.run_if(dialogue_visible),
                typewriter_tick.run_if(dialogue_visible),
                quest_notification_from_effects,
                quest_notification_clear,
            ));
    }
}
```

注意：`DialogueBank`（对话资源）、`DialogueManager` 等已在 `types.rs` 和 `systems.rs` 中定义，需要在 `mod.rs` 中 `pub use` 导出。

- [ ] **Step 9: 删除旧 `dialogue.rs`**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
rm src/game/dialogue.rs
```

- [ ] **Step 10: 运行 cargo check**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

预期：0 errors。

---

### Task 3: 合并碰撞系统（CollisionShape → Collider）

**说明：** 当前两套碰撞系统共存：旧 `CollisionShape`（shape.rs，用于地面检测）和新 `Collider` + `ColliderShape`（collider.rs，用于完整碰撞处理）。本任务将把所有旧 `CollisionShape` 使用迁移到新 `Collider` 系统，然后删除 `shape.rs`。

**Files:**
- Modify: `src/game/player.rs`
- Modify: `src/game/npc.rs`
- Modify: `src/world/level.rs`
- Modify: `src/td/level.rs`
- Modify: `src/physics/collision/collider.rs`（添加 Plane ground_height_at）
- Modify: `src/physics/collision/manager.rs`
- Modify: `src/physics/collision/debug.rs`
- Modify: `src/main.rs`（移除 CollisionShape 注册）
- Remove: `src/physics/collision/mod.rs`（移除 `pub mod shape;`）
- Remove: `src/physics/collision/shape.rs`

- [ ] **Step 1: 分析当前旧 CollisionShape 使用情况**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
grep -rn "CollisionShape\|find_ground_y\|push_out_horizontal\|collision::" src/ --include="*.rs" | grep -v ".swp"
```

记录所有使用旧系统的文件。

- [ ] **Step 2: 在 collider.rs 中添加地面检测功能**

在 `Collider` 上添加 `ground_height_at` 方法（等效于旧 `CollisionShape::ground_height_at`）：

```rust
// 在 collider.rs 的 impl Collider 块中添加
impl Collider {
    /// 查询碰撞体在给定 XZ 位置的地面高度（等效于旧 CollisionShape::ground_height_at）
    pub fn ground_height_at(&self, transform: &Transform, point_xz: Vec2) -> Option<f32> {
        match &self.shape {
            ColliderShape::Plane { normal: _, distance } => {
                // Plane 定义为 y = distance
                Some(*distance)
            }
            ColliderShape::Box { half_extents } => {
                let pos = transform.translation;
                let s = transform.scale;
                let hx = half_extents.x * s.x;
                let hz = half_extents.z * s.z;
                if point_xz.x >= pos.x - hx && point_xz.x <= pos.x + hx
                    && point_xz.y >= pos.z - hz && point_xz.y <= pos.z + hz
                {
                    Some(pos.y + half_extents.y * s.y)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 将玩家从碰撞体水平推出（等效于旧 push_out_horizontal）
    pub fn push_out_horizontal(
        &self,
        transform: &Transform,
        player_pos: &mut Vec3,
        player_radius: f32,
        player_height: f32,
    ) {
        let ColliderShape::Box { half_extents } = &self.shape else { return };
        let pos = transform.translation;
        let s = transform.scale;
        let hx = half_extents.x * s.x;
        let hz = half_extents.z * s.z;
        let hy = half_extents.y * s.y;

        let player_bottom = player_pos.y;
        let player_top = player_pos.y + player_height;
        let box_bottom = pos.y - hy;
        let box_top = pos.y + hy;
        if player_top <= box_bottom || player_bottom >= box_top { return; }

        let box_min_x = pos.x - hx; let box_max_x = pos.x + hx;
        let box_min_z = pos.z - hz; let box_max_z = pos.z + hz;
        let closest_x = player_pos.x.clamp(box_min_x, box_max_x);
        let closest_z = player_pos.z.clamp(box_min_z, box_max_z);
        let dx = player_pos.x - closest_x;
        let dz = player_pos.z - closest_z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < player_radius * player_radius && dist_sq > f32::EPSILON {
            let dist = dist_sq.sqrt();
            let push = player_radius - dist;
            player_pos.x += dx / dist * push;
            player_pos.z += dz / dist * push;
        } else if dist_sq <= f32::EPSILON {
            let overlap_x = hx - (player_pos.x - pos.x).abs();
            let overlap_z = hz - (player_pos.z - pos.z).abs();
            if overlap_x < overlap_z {
                player_pos.x += if player_pos.x > pos.x { 1.0 } else { -1.0 } * (overlap_x + player_radius);
            } else {
                player_pos.z += if player_pos.z > pos.z { 1.0 } else { -1.0 } * (overlap_z + player_radius);
            }
        }
    }
}
```

- [ ] **Step 3: 更新 player.rs 使用新 Collider**

将 `use crate::collision::{CollisionShape, find_ground_y, push_out_horizontal};` 改为：

```rust
use crate::collision::collider::{Collider, ColliderShape};
```

将玩家地面检测和水平推出查询从 `CollisionShape` 改为 `Collider`：

```rust
// 查询地面高度 — 使用新 Collider::ground_height_at
fn find_ground_y(
    collision_q: &Query<(&Transform, &Collider), Without<Player>>,
    player_xz: Vec2,
) -> f32 {
    let mut best = f32::NEG_INFINITY;
    for (t, collider) in collision_q.iter() {
        if let Some(h) = collider.ground_height_at(t, player_xz) {
            if h > best { best = h; }
        }
    }
    best
}

// 水平推出
fn push_out_horizontal(
    collision_q: &Query<(&Transform, &Collider), Without<Player>>,
    player_pos: &mut Vec3,
    player_radius: f32,
    player_height: f32,
) {
    for (t, collider) in collision_q.iter() {
        collider.push_out_horizontal(t, player_pos, player_radius, player_height);
    }
}
```

将 player.rs 中的碰撞查询 `Query<(&Transform, &CollisionShape), Without<Player>>` 改为 `Query<(&Transform, &Collider), Without<Player>>`。

- [ ] **Step 4: 更新 npc.rs 使用新 Collider**

类似 player.rs 的修改，将 `CollisionShape` 查询改为 `Collider` 查询。

- [ ] **Step 5: 更新 world/level.rs**

将 `CollisionShape::Plane { y: 0.0 }` 改为使用新 `Collider`：

```rust
// 在生成关卡地面时：
use crate::collision::collider::{Collider, ColliderShape};
// 替换:
// CollisionShape::Plane { y: 0.0 }
// 为:
Collider::new(ColliderShape::Plane { normal: Vec3::Y, distance: 0.0 })
```

- [ ] **Step 6: 更新 td/level.rs**

同样替换 `CollisionShape::Plane` 和 `CollisionShape::Box` 为新的 `Collider` + `ColliderShape`。

- [ ] **Step 7: 更新 collision/debug.rs**

切换调试绘制从 `CollisionShape` 到 `Collider` 加 `ColliderShape`。

- [ ] **Step 8: 更新 collision/mod.rs**

移除 `pub mod shape;` 和 `pub use shape::*;`：

```rust
//! 碰撞检测系统
pub mod collider;
pub mod manager;
pub mod debug;

// 向后兼容 — 将子模块的所有 pub 项提升到 collision 命名空间
pub use collider::*;
pub use manager::*;
pub use debug::*;
```

- [ ] **Step 9: 更新 main.rs**

移除 `app.register_type::<physics::collision::shape::CollisionShape>();`。

- [ ] **Step 10: 删除旧 shape.rs**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
rm src/physics/collision/shape.rs
```

- [ ] **Step 11: 运行 cargo check**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

---


