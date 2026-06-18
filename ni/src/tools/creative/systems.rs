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

#[allow(clippy::too_many_arguments)]
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
    state.current_items = state.category_items.first().cloned().unwrap_or_default();
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

    let ctl = CameraController {
        yaw: player_yaw,
        pitch: player_pitch,
        ..Default::default()
    };

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
    if !(0.0..=50.0).contains(&t) { return None; }
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
        if let Some(e) = state.ghost_entity
            && let Ok(mut t) = ghost_q.get_mut(e) { t.translation.y = -9999.0; }
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

#[allow(clippy::too_many_arguments)]
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
    if let Ok(ctx) = egui_ctx.ctx_mut()
        && ctx.wants_pointer_input() { return; }
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
    if let Ok(ctx) = egui_ctx.ctx_mut()
        && ctx.wants_pointer_input() { return; }
    let Ok(cam_gt) = camera_q.single() else { return };
    let origin = cam_gt.translation();
    let fwd = cam_gt.forward();
    if fwd.y >= -0.001 { return; }
    let t = -origin.y / fwd.y;
    if !(0.0..=50.0).contains(&t) { return; }
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
    let ron_str = match ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default()) {
        Ok(s) => s,
        Err(e) => { error!("[Creative] 序列化关卡配置失败: {}", e); return; }
    };
    match std::fs::write("assets/level/level_config.ron", &ron_str) {
        Ok(()) => { state.dirty = false; info!("[Creative] 已保存"); }
        Err(e) => error!("[Creative] 保存失败: {}", e),
    }
}
