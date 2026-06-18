//! 性能极限压力测试
//!
//! 模拟大量 NPC（最多 50000 个）的性能基准测试。
//! F9/F10/F11 控制启停，egui 面板显示帧率和统计数据。

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use rand::Rng;

use crate::colliders::{Collider, CollisionMask, CollisionResponse, SmoothPush};
use crate::collision_manager::CollisionManager;
use crate::entity_db::{EntityRegistry, GlbCache};
use crate::pathfinding::{Navigator, NavTarget, AiNavMesh};
use crate::pathfinding::create_rect_navmesh;
use vleue_navigator::NavMesh;

pub struct StressTestPlugin;

impl Plugin for StressTestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StressTestState>()
            .init_resource::<StressStats>()
            .add_systems(Update, (
                stress_npc_set_target,
                track_stress_stats,
            ).run_if(|state: Res<StressTestState>| state.active))
            .add_systems(EguiPrimaryContextPass, stress_test_ui);
    }
}

// ── 组件 ──

/// 压力测试 NPC 标记（寻路系统驱动移动）
#[derive(Component)]
struct StressNpc;

/// 标记所有属于压力测试的实体（方便清理）
#[derive(Component)]
struct StressEntity;

/// 随机巡逻模式：NPC 在玩家周围随机游走，到达后换新目标
/// 不附加此组件的 StressNpc 默认追逐玩家
#[derive(Component)]
struct StressRandomPatrol {
    target: Vec3,
    change_timer: f32,
}

// ── 资源 ──

#[derive(Resource)]
pub struct StressTestState {
    pub active: bool,
    pub grid_size: f32,
    pub npc_count: usize,
    pub spawned: bool,
    pub input_text: String,
    pub grid_text: String,
    pub show_panel: bool,
    /// 上次生成的导航图参数（缓存用）
    last_grid_size: f32,
    last_center: Vec3,
}

impl Default for StressTestState {
    fn default() -> Self {
        Self {
            active: false,
            grid_size: 100.0,
            npc_count: 0,
            spawned: false,
            input_text: "10000".to_string(),
            grid_text: "100".to_string(),
            show_panel: false,
            last_grid_size: 0.0,
            last_center: Vec3::ZERO,
        }
    }
}

/// 压力测试性能指标
#[derive(Resource, Default)]
struct StressStats {
    /// 指数移动平均帧耗时 (ms)
    frame_time_ms: f32,
    /// 当前帧最高 NPC Y 坐标
    current_max_y: f32,
    /// 自测试启动以来的最高 Y（衡量叠罗汉严重程度）
    peak_npc_y: f32,
    /// 当前帧碰撞对数量
    collision_pairs: usize,
    /// 存活 NPC 数量
    npc_alive: usize,
}

// ── 系统 ──

/// 每帧跟踪性能指标
fn track_stress_stats(
    time: Res<Time>,
    collision_manager: Res<CollisionManager>,
    stress_npc_q: Query<&Transform, With<StressNpc>>,
    mut stats: ResMut<StressStats>,
) {
    let dt = time.delta_secs();
    // 指数移动平均帧耗时（α=0.1），比瞬时值更平滑
    stats.frame_time_ms = stats.frame_time_ms * 0.9 + dt * 1000.0 * 0.1;

    stats.collision_pairs = collision_manager.current_collision_count();
    stats.npc_alive = stress_npc_q.iter().len();

    let max_y = stress_npc_q.iter()
        .map(|t| t.translation.y)
        .fold(0.0f32, f32::max);
    stats.current_max_y = max_y;
    if max_y > stats.peak_npc_y {
        stats.peak_npc_y = max_y;
    }
}

/// 配置面板 UI
#[allow(clippy::too_many_arguments)]
fn stress_test_ui(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<StressTestState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut nav_meshes: ResMut<Assets<NavMesh>>,
    registry: Res<EntityRegistry>,
    glb_cache: Res<GlbCache>,
    player_q: Query<&Transform, With<crate::player::Player>>,
    stress_q: Query<Entity, (With<StressEntity>, Without<crate::player::Player>)>,
    phase: Res<State<crate::game_state::GamePhase>>,
    mut stats: ResMut<StressStats>,
) {
    // F10 切换配置面板
    if keys.just_pressed(KeyCode::F10) {
        state.show_panel = !state.show_panel;
    }

    // F9 开关压力测试
    if keys.just_pressed(KeyCode::F9) {
        if state.active {
            despawn_all(&mut commands, &stress_q);
            state.active = false;
            state.spawned = false;
            info!("压力测试已关闭");
        } else {
            let count = state.input_text.trim().parse::<usize>().unwrap_or(10000);
            state.npc_count = count.clamp(1, 50000);
            let gs = state.grid_text.trim().parse::<f32>().unwrap_or(100.0);
            state.grid_size = gs.clamp(10.0, 1000.0);
            stats.peak_npc_y = 0.0;
            spawn_stress_test(
                &mut commands, &asset_server, &registry, &glb_cache,
                &mut meshes, &mut materials, &mut nav_meshes, &player_q, &mut state,
            );
            state.active = true;
            state.spawned = true;
        }
    }

    // F11 重置统计峰值
    if keys.just_pressed(KeyCode::F11) {
        stats.peak_npc_y = 0.0;
    }

    if *phase.get() != crate::game_state::GamePhase::Playing {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let panel = egui::Window::new("⚡ 性能测试")
        .fixed_pos(egui::pos2(12.0, 60.0))
        .collapsible(true)
        .default_open(true)
        .resizable(false)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x1a, 220),
            corner_radius: 8.0.into(),
            stroke: egui::epaint::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x4a)),
            ..Default::default()
        });

    panel.show(ctx, |ui| {
        ui.style_mut().override_font_id = Some(egui::FontId::proportional(14.0));

        if state.active {
            // ── 运行中状态 ──
            ui.label(egui::RichText::new(format!(
                "NPC: {} / {}",
                stats.npc_alive, state.npc_count,
            )).color(egui::Color32::from_rgb(0x00, 0xcc, 0x66)));

            ui.label(egui::RichText::new(format!(
                "网格: {:.0}×{:.0}", state.grid_size, state.grid_size,
            )).color(egui::Color32::GRAY));

            // 性能指标
            ui.separator();
            ui.label(egui::RichText::new(format!("帧耗时: {:.1} ms", stats.frame_time_ms))
                .color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)));
            ui.label(egui::RichText::new(format!("碰撞对: {}", stats.collision_pairs))
                .color(egui::Color32::GRAY));
            ui.label(egui::RichText::new(format!("当前最高 Y: {:.2}", stats.current_max_y))
                .color(egui::Color32::GRAY));
            let peak_color = if stats.peak_npc_y > 1.0 {
                egui::Color32::from_rgb(0xff, 0x66, 0x44)
            } else {
                egui::Color32::GRAY
            };
            ui.label(egui::RichText::new(format!("历史最高 Y: {:.2}", stats.peak_npc_y))
                .color(peak_color));

            ui.separator();
            if ui.add(
                egui::Button::new(egui::RichText::new("⏹ 停止 (F9)")
                    .size(13.0).color(egui::Color32::WHITE))
                    .fill(egui::Color32::from_rgb(0xcc, 0x33, 0x33))
                    .corner_radius(4.0)
            ).clicked() {
                despawn_all(&mut commands, &stress_q);
                state.active = false;
                state.spawned = false;
            }
        } else {
            // ── 配置状态 ──
            ui.label(egui::RichText::new("NPC 数量:").color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)));

            let mut count_str = state.input_text.clone();
            let resp = ui.add_sized(
                egui::vec2(140.0, 28.0),
                egui::TextEdit::singleline(&mut count_str)
                    .font(egui::TextStyle::Monospace)
                    .text_color_opt(Some(egui::Color32::from_rgb(0x00, 0xd4, 0xff)))
                    .background_color(egui::Color32::from_rgb(0x1a, 0x1a, 0x2e)),
            );
            if resp.changed() || resp.lost_focus() {
                state.input_text = count_str;
            }

            ui.add_space(4.0);
            ui.label(egui::RichText::new("范围 1 ~ 50000")
                .size(11.0).color(egui::Color32::GRAY));

            ui.add_space(8.0);
            ui.label(egui::RichText::new("网格大小:").color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)));

            let mut grid_str = state.grid_text.clone();
            let gresp = ui.add_sized(
                egui::vec2(140.0, 28.0),
                egui::TextEdit::singleline(&mut grid_str)
                    .font(egui::TextStyle::Monospace)
                    .text_color_opt(Some(egui::Color32::from_rgb(0x00, 0xd4, 0xff)))
                    .background_color(egui::Color32::from_rgb(0x1a, 0x1a, 0x2e)),
            );
            if gresp.changed() || gresp.lost_focus() {
                state.grid_text = grid_str;
            }

            ui.label(egui::RichText::new("范围 10 ~ 1000")
                .size(11.0).color(egui::Color32::GRAY));

            ui.add_space(8.0);
            if ui.add(
                egui::Button::new(egui::RichText::new("▶ 开始 (F9)")
                    .size(13.0).color(egui::Color32::BLACK))
                    .fill(egui::Color32::from_rgb(0x00, 0xcc, 0x66))
                    .corner_radius(4.0)
            ).clicked() {
                let count = state.input_text.trim().parse::<usize>().unwrap_or(10000);
                state.npc_count = count.clamp(1, 50000);
                let gs = state.grid_text.trim().parse::<f32>().unwrap_or(100.0);
                state.grid_size = gs.clamp(10.0, 1000.0);
                stats.peak_npc_y = 0.0;
                spawn_stress_test(
                    &mut commands, &asset_server, &registry, &glb_cache,
                    &mut meshes, &mut materials, &mut nav_meshes, &player_q, &mut state,
                );
                state.active = true;
                state.spawned = true;
            }

            ui.separator();
            ui.label(egui::RichText::new("F9 开关 | F10 面板 | F11 重置统计")
                .size(11.0).color(egui::Color32::GRAY));
        }

        if state.active {
            ui.label(egui::RichText::new(format!("存活实体: {}", stats.npc_alive + 1))
                .size(11.0).color(egui::Color32::GRAY));
        }
    });
}

// ── 辅助函数 ──

fn despawn_all(
    commands: &mut Commands,
    stress_q: &Query<Entity, (With<StressEntity>, Without<crate::player::Player>)>,
) {
    for entity in stress_q.iter() {
        commands.entity(entity).despawn();
    }
}

// ── 核心生成函数 ──

#[allow(clippy::too_many_arguments)]
fn spawn_stress_test(
    commands: &mut Commands,
    asset_server: &AssetServer,
    registry: &EntityRegistry,
    glb_cache: &GlbCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    nav_meshes: &mut Assets<NavMesh>,
    player_q: &Query<&Transform, With<crate::player::Player>>,
    state: &mut StressTestState,
) {
    let center = player_q.iter().next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let count = state.npc_count;
    if count == 0 {
        info!("压力测试: NPC 数量为 0，跳过生成");
        return;
    }

    let half = state.grid_size / 2.0;
    let per_side = (count as f32).sqrt().ceil() as usize;
    // 网格间距保证 NPC 均匀分布在 half × half 区域
    let spacing = if per_side > 1 {
        state.grid_size / (per_side as f32 - 1.0)
    } else {
        state.grid_size
    };

    if spacing < 0.6 {
        warn!(
            "网格间距 ({:.1}) 小于碰撞体直径 (0.6)，NPC 将密集堆叠",
            spacing
        );
    }

    // 从实体数据库查找模型
    let template = registry.get("stress.npc")
        .expect("[EntityDB] 缺失 stress.npc 模板 — 检查 assets/data/entities.ron");
    let model_path = template.model.as_ref()
        .expect("[EntityDB] stress.npc 模板未配置 model 字段");
    let npc_scene = glb_cache.handles.get(model_path)
        .cloned()
        .unwrap_or_else(|| {
            warn!("[EntityDB] GLB 缓存未命中，直接加载: {}", model_path);
            asset_server.load(model_path)
        });

    // 地板
    let floor_mesh = meshes.add(Plane3d::default().mesh().size(state.grid_size, state.grid_size));
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.2),
            ..default()
        })),
        Transform::from_xyz(center.x, 0.0, center.z),
        StressEntity,
        Name::new("StressFloor"),
    ));

    // ── NPC 批量生成 ──
    // 分两批 spawn_batch：3/4 追逐玩家，1/4 随机巡逻
    let chase_count = count * 3 / 4;
    let patrol_count = count - chase_count;

    let mut rng = rand::thread_rng();
    let mut chase_batch = Vec::with_capacity(chase_count);
    let mut patrol_batch = Vec::with_capacity(patrol_count);

    for i in 0..count {
        // 扁平索引 → 二维网格坐标
        let ix = i % per_side;
        let iz = i / per_side;
        let x = center.x - half + ix as f32 * spacing;
        let z = center.z - half + iz as f32 * spacing;

        let speed = rng.gen_range(1.0..4.0);
        // perturbation 控制寻路随机扰动幅度，使路径多样化
        let perturbation = rng.r#gen::<f32>() * 0.8 + 0.2;

        let common = (
            SceneRoot(npc_scene.clone()),
            Transform::from_xyz(x, 0.0, z),
            Navigator { perturbation, ..Navigator::with_recalc(speed, 2.0) },
            Collider::sphere(0.42, CollisionMask::npc()),
            CollisionResponse { push_force: 5.0, ..default() },
            SmoothPush { damping: 0.5, ..default() },
            StressNpc,
            StressEntity,
            Name::new(format!("StressNpc_{}", i)),
        );

        // 每 4 个中 1 个巡逻（25%），其余追逐玩家
        if i % 4 == 0 {
            let patrol_target = Vec3::new(
                center.x + rng.gen_range(-half..half),
                0.0,
                center.z + rng.gen_range(-half..half),
            );
            patrol_batch.push((
                common,
                NavTarget::new(patrol_target),
                StressRandomPatrol {
                    target: patrol_target,
                    change_timer: rng.gen_range(2.0..6.0),
                },
            ));
        } else {
            chase_batch.push((
                common,
                NavTarget::new(center),
            ));
        }
    }

    commands.spawn_batch(chase_batch);
    commands.spawn_batch(patrol_batch);

    // ── NavMesh（缓存：参数变化时才重建）──
    let needs_new_graph = (state.grid_size - state.last_grid_size).abs() > 0.1
        || center.distance(state.last_center) > 0.1;
    if needs_new_graph {
        let handle = create_rect_navmesh(nav_meshes, center, state.grid_size);
        commands.insert_resource(AiNavMesh { handle });
        state.last_grid_size = state.grid_size;
        state.last_center = center;
        info!(
            "NavMesh 已初始化 (网格 {:.0}×{:.0})",
            state.grid_size, state.grid_size,
        );
    }

    info!(
        "压力测试: {} 个 NPC 已生成 (网格 {:.0}×{:.0}, 追逐 {} / 巡逻 {})",
        count, state.grid_size, state.grid_size, chase_count, patrol_count,
    );
}

// ── 寻路目标系统 ──

/// 每帧更新 NPC 的 NavTarget
fn stress_npc_set_target(
    time: Res<Time>,
    player_q: Query<&Transform, With<crate::player::Player>>,
    mut chase_q: Query<&mut NavTarget, (With<StressNpc>, Without<StressRandomPatrol>)>,
    mut patrol_q: Query<(&mut NavTarget, &mut StressRandomPatrol), With<StressNpc>>,
) {
    let player_pos = player_q.iter().next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let dt = time.delta_secs();

    // 追逐玩家
    for mut target in chase_q.iter_mut() {
        target.position = player_pos;
    }

    // 随机巡逻
    for (mut target, mut patrol) in patrol_q.iter_mut() {
        patrol.change_timer -= dt;
        if patrol.change_timer <= 0.0 {
            patrol.target = Vec3::new(
                player_pos.x + rand::thread_rng().gen_range(-50.0..50.0),
                0.0,
                player_pos.z + rand::thread_rng().gen_range(-50.0..50.0),
            );
            patrol.change_timer = rand::thread_rng().gen_range(2.0..6.0);
        }
        target.position = patrol.target;
    }
}
