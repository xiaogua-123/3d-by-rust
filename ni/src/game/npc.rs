//! NPC（非玩家角色）系统
//!
//! 定义 `Npc`、`NpcConfig`、`NpcPatrol` 等组件，提供 NPC 生成、
//! 交互提示、面朝玩家和巡逻移动等功能。交互通过 F 键触发对话系统。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::colliders::{Collider, ColliderShape, CollisionMask, CollisionResponse, SmoothPush};
use crate::dialogue::{DialogueTrigger, StartDialogueEvent};
use crate::game_state::GamePhase;
use crate::grid::GameGridResource;
use crate::player::Player;

// ═══════════════════════════════════════════
// 组件
// ═══════════════════════════════════════════

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Npc;

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct NpcConfig {
    pub display_name: String,
    pub conversation_id: String,
    pub start_node: String,
    pub patrol_points: Vec<Vec3>,
    pub speed: f32,
    /// false = XZ 平面定向（地面 NPC，不倾斜）
    /// true = 全 3D 定向（飞行 NPC 等）
    pub use_3d_orientation: bool,
    /// slerp 转向速率，越大转动越快
    pub turn_speed: f32,
    /// 碰撞响应开关：true = 会被推开（默认），false = 不响应碰撞
    pub collision_push: bool,
    /// NPC 之间互相推开：true = 会受到其他 NPC 碰撞推开（默认），false = 不与其他 NPC 碰撞
    pub push_npcs: bool,
    /// 3D 碰撞模式：true = 全 3D 碰撞（飞天 NPC），false = 仅地面 XZ 碰撞（默认）
    pub collision_3d: bool,
}

impl NpcConfig {
    pub fn stationary(display_name: &str, conversation_id: &str, start_node: &str) -> Self {
        Self {
            display_name: display_name.to_string(),
            conversation_id: conversation_id.to_string(),
            start_node: start_node.to_string(),
            patrol_points: Vec::new(),
            speed: 0.0,
            use_3d_orientation: false,
            turn_speed: 8.0,
            collision_push: true,
            push_npcs: true,
            collision_3d: false,
        }
    }

    #[allow(dead_code)]
    pub fn patrol(
        display_name: &str,
        conversation_id: &str,
        start_node: &str,
        patrol_points: Vec<Vec3>,
        speed: f32,
    ) -> Self {
        Self {
            display_name: display_name.to_string(),
            conversation_id: conversation_id.to_string(),
            start_node: start_node.to_string(),
            patrol_points,
            speed,
            use_3d_orientation: false,
            turn_speed: 8.0,
            collision_push: true,
            push_npcs: true,
            collision_3d: false,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct NpcPatrol {
    pub current_target: usize,
    /// NPC 的初始 Y 坐标，用于地面 NPC 碰撞后恢复高度
    pub ground_y: f32,
    /// 当前速度向量（用于平滑加速/减速过渡）
    pub velocity: Vec3,
}

impl Default for NpcPatrol {
    fn default() -> Self {
        Self {
            current_target: 1,
            ground_y: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

// ═══════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════

#[allow(dead_code)]
pub fn spawn_npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    config: NpcConfig,
    pos: Vec3,
    color: Color,
) -> Entity {
    let interaction_radius = 2.5;
    // TODO: GLB替换 → SceneRoot(asset_server.load("models/characters/{npc_id}.glb#Scene0"))
    // 方案1: spawn_npc 接受 asset_server 参数，按 config.display_name 选择模型
    // 方案2: 在 NpcConfig 中增加 model_path 字段，支持 per-NPC 模型
    // 替换后保留 Npc/NpcConfig/DialogueTrigger 组件，移除 Mesh3d/MeshMaterial3d
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 1.6, 0.6))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(pos),
        Npc,
        config.clone(),
        NpcPatrol { current_target: 1, ground_y: pos.y, velocity: Vec3::ZERO },
        DialogueTrigger {
            conversation_id: config.conversation_id,
            start_node: config.start_node,
            radius: interaction_radius,
        },
        Collider::sphere(
            0.3,
            if config.push_npcs {
                CollisionMask::npc()
            } else {
                CollisionMask::npc_no_push()
            },
        ),
        if config.collision_push {
            CollisionResponse {
                push_force: 0.3,
                ..default()
            }
        } else {
            CollisionResponse::kinematic()
        },
        SmoothPush::default(),
        Name::new(format!("NPC_{}", config.display_name)),
    ));

    // 头顶名字标签 (spawn as child)
    // Note: no Text3D in simple form, skip for now
    // Can be added later with a proper name tag system

    entity.id()
}

// ═══════════════════════════════════════════
// 交互提示组件
// ═══════════════════════════════════════════

/// 交互提示组件 — 附加到需要显示 "按 F 交谈" 等提示的实体上
#[derive(Component)]
pub struct InteractionPrompt {
    pub text: String,
    /// 头顶偏移高度
    pub offset: f32,
    pub font_size: f32,
}

impl Default for InteractionPrompt {
    fn default() -> Self {
        Self {
            text: "按 F 交谈".into(),
            offset: 3.5,
            font_size: 13.0,
        }
    }
}

/// 在 NPC 头顶绘制交互提示（只在玩家靠近时显示）
fn npc_interaction_prompt(
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    player_q: Query<&Transform, With<Player>>,
    npc_q: Query<(Entity, &Transform, &DialogueTrigger, &InteractionPrompt), With<Npc>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut contexts: EguiContexts,
    hiding_q: Query<(), With<crate::stealth::PlayerHiding>>,
) {
    let Ok((cam, cam_transform)) = cameras.single() else { return };
    let Ok(window) = window.single() else { return };
    let Ok(player_t) = player_q.single() else { return };

    let Some(viewport_size) = cam.logical_viewport_size() else { return };
    let cam_pos = cam_transform.translation();
    let cam_forward = cam_transform.forward();
    let scale = window.scale_factor();

    let vp_offset_phys = match &cam.viewport {
        Some(vp) => Vec2::new(vp.physical_position.x as f32, vp.physical_position.y as f32),
        None => Vec2::ZERO,
    };
    let vp_offset = vp_offset_phys / scale;

    let Ok(ctx) = contexts.ctx_mut() else { return };

    for (entity, npc_t, trigger, prompt) in npc_q.iter() {
        // 检查玩家距离
        let dist = player_t.translation.distance(npc_t.translation);
        if dist > trigger.radius + 1.0 {
            continue;
        }

        // 躲藏时隐藏提示
        if !hiding_q.is_empty() {
            continue;
        }

        let world_pos = npc_t.translation + Vec3::Y * prompt.offset;

        // 剔除相机背后的实体
        if (world_pos - cam_pos).dot(*cam_forward) <= 0.0 {
            continue;
        }

        let Ok(vp_coord) = cam.world_to_viewport(cam_transform, world_pos) else {
            continue;
        };

        if vp_coord.x < 0.0 || vp_coord.y < 0.0
            || vp_coord.x > viewport_size.x || vp_coord.y > viewport_size.y
        {
            continue;
        }

        let egui_pos = egui::pos2(vp_offset.x + vp_coord.x, vp_offset.y + vp_coord.y);

        // 交互提示：F 键图标 + "交谈"
        egui::Area::new(egui::Id::new(("npc_interact", entity)))
            .fixed_pos(egui_pos)
            .anchor(egui::Align2::CENTER_BOTTOM, (0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame {
                    fill: egui::Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x1a, 220),
                    inner_margin: egui::Margin::symmetric(10, 4),
                    corner_radius: 6.0.into(),
                    stroke: egui::epaint::Stroke::new(1.0, egui::Color32::from_rgb(0x00, 0xd4, 0xff)),
                    ..Default::default()
                }.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // F 键高亮
                        egui::Frame {
                            fill: egui::Color32::from_rgb(0x00, 0xd4, 0xff),
                            inner_margin: egui::Margin::symmetric(6, 1),
                            corner_radius: 3.0.into(),
                            ..Default::default()
                        }.show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("F")
                                    .size(prompt.font_size)
                                    .color(egui::Color32::BLACK)
                                    .strong(),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&prompt.text)
                                .size(prompt.font_size - 1.0)
                                .color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)),
                        );
                    });
                });
            });
    }
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Npc>()
            .register_type::<NpcConfig>()
            .register_type::<NpcPatrol>()
            .add_systems(
                Update,
                (
                    npc_interact.run_if(in_state(GamePhase::Playing)),
                    npc_patrol.run_if(in_state(GamePhase::Playing)),
                    npc_face_player.run_if(in_state(GamePhase::Dialoguing)),
                ),
            )
            .add_systems(EguiPrimaryContextPass,
                npc_interaction_prompt.run_if(in_state(GamePhase::Playing)),
            );
    }
}

// ═══════════════════════════════════════════
// 交互系统
// ═══════════════════════════════════════════

fn npc_interact(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    hiding_q: Query<(), With<crate::stealth::PlayerHiding>>,
    npc_q: Query<(&Transform, &DialogueTrigger), With<Npc>>,
    mut dialogue_writer: MessageWriter<StartDialogueEvent>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    // 躲藏时不能对话
    if !hiding_q.is_empty() {
        return;
    }

    let Ok(player_t) = player_q.single() else {
        return;
    };

    for (npc_t, trigger) in npc_q.iter() {
        let dist = player_t.translation.distance(npc_t.translation);
        if dist <= trigger.radius {
            info!("触发对话: {} (距离: {:.1})", trigger.conversation_id, dist);
            dialogue_writer.write(StartDialogueEvent {
                conversation_id: trigger.conversation_id.clone(),
                start_node: trigger.start_node.clone(),
            });
            return; // 一次只触发一个对话
        }
    }
}

// ═══════════════════════════════════════════
// 巡逻系统
// ═══════════════════════════════════════════

/// 检查从 NPC 当前位置到目标点的路径是否有遮挡
///
/// 返回 `true` 表示路径通畅，可以移动。
/// `self_entity` 用于忽略 NPC 自身的碰撞体。
#[allow(dead_code)]
pub fn npc_is_path_clear(
    game_grid: &GameGridResource,
    from: Vec3,
    to: Vec3,
    self_entity: Entity,
) -> bool {
    game_grid.has_line_of_sight(Vec2::new(from.x, from.z), Vec2::new(to.x, to.z), |id| {
        *id == self_entity
    })
}

fn npc_patrol(
    time: Res<Time>,
    game_grid: Res<GameGridResource>,
    wall_q: Query<(&Transform, &Collider), Without<Npc>>,
    mut npc_q: Query<(Entity, &mut Transform, &NpcConfig, &mut NpcPatrol), With<Npc>>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, config, mut patrol) in npc_q.iter_mut() {
        if config.patrol_points.len() < 2 {
            continue;
        }

        let target = config.patrol_points[patrol.current_target];
        let dir = target - transform.translation;

        // 检测路径是否有遮挡，有则跳到下一个巡逻点
        if dir.length() > 0.5 {
            let from_xz = Vec2::new(transform.translation.x, transform.translation.z);
            let to_xz = Vec2::new(target.x, target.z);
            let blocked = !game_grid.has_line_of_sight(from_xz, to_xz, |id| *id == entity);
            if blocked {
                patrol.current_target = (patrol.current_target + 1) % config.patrol_points.len();
                continue;
            }
        }

        if dir.length() < 0.3 {
            patrol.current_target = (patrol.current_target + 1) % config.patrol_points.len();
            continue;
        }

        let direction = dir.normalize();

        // ── 加速度平滑移动（取代直接 speed * dt） ──
        // 目标速度：朝目标方向全速前进
        let desired_velocity = direction * config.speed;

        // 接近目标时减速，防止"急停"
        let dist = dir.length();
        let slowdown_radius = config.speed * 0.6; // 减速半径随速度变化
        let speed_mult = (dist / slowdown_radius).min(1.0);

        // 带平滑过渡的目标速度（含减速）
        let target_vel = desired_velocity * speed_mult;

        // 加速度：向目标速度靠拢（数值越大响应越快）
        let acceleration = 4.0;
        patrol.velocity = patrol.velocity.lerp(target_vel, (acceleration * dt).min(1.0));

        // 应用速度
        transform.translation += patrol.velocity * dt;

        // ── 碰撞推离：将 NPC 推出重叠的墙壁 ──
        // 飞天 NPC (collision_3d) 跳过地面墙推，用 CollisionManager 做全 3D 碰撞
        if !config.collision_3d {
            let npc_radius = 0.3;
            let npc_height = 1.6;
            for (t, collider) in wall_q.iter() {
                let ColliderShape::Box { half_extents } = &collider.shape else {
                    continue;
                };
                let pos = t.translation;
                let s = t.scale;

                let hx = half_extents.x * s.x;
                let hz = half_extents.z * s.z;
                let hy = half_extents.y * s.y;

                // 垂直无重叠则跳过
                let npc_bottom = transform.translation.y;
                let npc_top = transform.translation.y + npc_height;
                let box_bottom = pos.y - hy;
                let box_top = pos.y + hy;
                if npc_top <= box_bottom || npc_bottom >= box_top {
                    continue;
                }

                // 计算 NPC 圆与 AABB 矩形之间的最近距离
                let box_min_x = pos.x - hx;
                let box_max_x = pos.x + hx;
                let box_min_z = pos.z - hz;
                let box_max_z = pos.z + hz;

                let closest_x = transform.translation.x.clamp(box_min_x, box_max_x);
                let closest_z = transform.translation.z.clamp(box_min_z, box_max_z);

                let dx = transform.translation.x - closest_x;
                let dz = transform.translation.z - closest_z;
                let dist_sq = dx * dx + dz * dz;

                if dist_sq < npc_radius * npc_radius && dist_sq > f32::EPSILON {
                    let dist = dist_sq.sqrt();
                    let push = npc_radius - dist;
                    transform.translation.x += dx / dist * push;
                    transform.translation.z += dz / dist * push;
                } else if dist_sq <= f32::EPSILON {
                    // NPC 圆心在矩形内部，推到最近的边
                    let overlap_x = hx - (transform.translation.x - pos.x).abs();
                    let overlap_z = hz - (transform.translation.z - pos.z).abs();
                    if overlap_x < overlap_z {
                        let sign = if transform.translation.x > pos.x {
                            1.0
                        } else {
                            -1.0
                        };
                        transform.translation.x += sign * (overlap_x + npc_radius);
                    } else {
                        let sign = if transform.translation.z > pos.z {
                            1.0
                        } else {
                            -1.0
                        };
                        transform.translation.z += sign * (overlap_z + npc_radius);
                    }
                }
            }
        } // if !collision_3d

        // 平滑旋转：根据 use_3d_orientation 选择 XZ 平面或全 3D 朝向
        let orient_dir = if config.use_3d_orientation {
            direction
        } else {
            Vec3::new(direction.x, 0.0, direction.z)
        };
        if orient_dir.length_squared() > 0.001 {
            let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, orient_dir.normalize());
            transform.rotation = transform.rotation.slerp(target_rot, config.turn_speed * dt);
        }
    }
}

// ═══════════════════════════════════════════
// 面向玩家
// ═══════════════════════════════════════════

#[allow(clippy::type_complexity)]
fn npc_face_player(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut npc_q: Query<(&mut Transform, &NpcConfig), (With<Npc>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let Ok(player_t) = player_q.single() else {
        return;
    };

    for (mut npc_t, config) in npc_q.iter_mut() {
        let dir = player_t.translation - npc_t.translation;
        let orient_dir = if config.use_3d_orientation {
            dir.normalize()
        } else {
            Vec3::new(dir.x, 0.0, dir.z)
        };
        if orient_dir.length_squared() > 0.001 {
            let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, orient_dir);
            npc_t.rotation = npc_t.rotation.slerp(target_rot, config.turn_speed * dt);
        }
    }
}
