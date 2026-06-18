//! 碰撞调试工具 — 可视化碰撞体形状与参数
//!
//! 按 F4 开关，在场景中绘制所有碰撞体的线框轮廓：
//! - `Collider`：绿线
//!
//! 面板中提供 NPC 推力系数滑条，实时调整 NPC 间的碰撞推力

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::colliders::{Collider, CollisionResponse, SmoothPush};

/// 碰撞调试开关
#[derive(Resource, Default)]
pub struct CollisionDebug {
    pub enabled: bool,
}

/// NPC 碰撞参数调试
#[derive(Resource)]
pub struct NpcCollisionConfig {
    pub push_force: f32,
    /// 阻尼系数 (0~1)，越大滑动越远、越柔
    pub damping: f32,
}

impl Default for NpcCollisionConfig {
    fn default() -> Self {
        Self {
            push_force: 0.3,
            damping: 0.85,
        }
    }
}

pub struct CollisionDebugPlugin;

impl Plugin for CollisionDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionDebug>()
            .init_resource::<NpcCollisionConfig>()
            .add_systems(Update, (
                toggle_collision_debug,
                draw_collider_gizmos,
                collision_debug_ui,
                apply_npc_push_force,
            ));
    }
}

/// F4 切换碰撞调试可视化
fn toggle_collision_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CollisionDebug>,
) {
    if keys.just_pressed(KeyCode::F4) {
        state.enabled = !state.enabled;
        info!("碰撞调试可视化: {}", if state.enabled { "开启" } else { "关闭" });
    }
}

/// 绘制 Collider（新系统）的线框 — 绿色
fn draw_collider_gizmos(
    state: Res<CollisionDebug>,
    mut gizmos: Gizmos,
    q: Query<(&Transform, &Collider)>,
) {
    if !state.enabled {
        return;
    }

    let color = Color::srgb(0.0, 1.0, 0.3);
    for (transform, collider) in q.iter() {
        let pos = transform.translation;
        let rot = transform.rotation;

        match &collider.shape {
            crate::colliders::ColliderShape::Sphere { radius } => {
                gizmos.sphere(pos, *radius, color);
            }
            crate::colliders::ColliderShape::Capsule { radius, half_height } => {
                let up = rot * Vec3::Y;
                let top = pos + up * *half_height;
                let bottom = pos - up * *half_height;
                gizmos.sphere(top, *radius, color);
                gizmos.sphere(bottom, *radius, color);
                for (dx, dz) in &[(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let side = rot * Vec3::new(*dx, 0.0, *dz) * *radius;
                    gizmos.line(top + side, bottom + side, color);
                }
            }
            crate::colliders::ColliderShape::Box { half_extents } => {
                gizmos.cube(
                    Transform::from_translation(pos)
                        .with_scale(*half_extents * 2.0)
                        .with_rotation(rot),
                    color,
                );
            }
            crate::colliders::ColliderShape::Plane { normal: _, distance } => {
                let size = 10.0;
                let steps = 10;
                for i in 0..=steps {
                    let t = -size + i as f32 * (2.0 * size / steps as f32);
                    let c = Color::srgb(0.0, 1.0, 0.3).with_alpha(0.3);
                    gizmos.line(Vec3::new(t, *distance, -size), Vec3::new(t, *distance, size), c);
                    gizmos.line(Vec3::new(-size, *distance, t), Vec3::new(size, *distance, t), c);
                }
            }
        }
    }
}


/// 碰撞调试参数面板
fn collision_debug_ui(
    mut contexts: EguiContexts,
    state: Res<CollisionDebug>,
    mut npc_config: ResMut<NpcCollisionConfig>,
    npc_q: Query<Entity, With<crate::npc::Npc>>,
) {
    if !state.enabled {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let panel = egui::Window::new("碰撞调试")
        .fixed_pos(egui::pos2(12.0, 300.0))
        .collapsible(true)
        .default_open(false)
        .resizable(false)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x1a, 220),
            corner_radius: 8.0.into(),
            stroke: egui::epaint::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x4a)),
            ..Default::default()
        });

    panel.show(ctx, |ui| {
        ui.style_mut().override_font_id = Some(egui::FontId::proportional(14.0));

        // 可视化开关
        ui.label(egui::RichText::new(if state.enabled { "可视: 开启 (F4)" } else { "可视: 关闭 (F4)" })
            .color(if state.enabled { egui::Color32::from_rgb(0x00, 0xcc, 0x66) } else { egui::Color32::GRAY }));

        ui.separator();

        // NPC 推力系数
        ui.label(egui::RichText::new("NPC 推力系数").color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)));
        ui.label(egui::RichText::new(format!("当前值: {:.2}", npc_config.push_force))
            .size(11.0)
            .color(egui::Color32::from_rgb(0x00, 0xd4, 0xff)));

        let changed = ui.add(
            egui::Slider::new(&mut npc_config.push_force, 0.0..=1.0)
                .step_by(0.05)
                .show_value(false)
        ).changed();

        if changed {
            info!("NPC 推力系数已调整为: {:.2}", npc_config.push_force);
        }

        ui.add_space(2.0);
        ui.label(egui::RichText::new(format!("影响 {} 个 NPC", npc_q.iter().count()))
            .size(11.0)
            .color(egui::Color32::GRAY));

        ui.separator();

        // 阻尼系数（摩擦力）
        ui.label(egui::RichText::new("阻尼系数（摩擦力）").color(egui::Color32::from_rgb(0xe8, 0xe8, 0xf0)));
        ui.label(egui::RichText::new(format!("当前值: {:.2}", npc_config.damping))
            .size(11.0)
            .color(egui::Color32::from_rgb(0x00, 0xd4, 0xff)));

        let d_changed = ui.add(
            egui::Slider::new(&mut npc_config.damping, 0.5..=0.98)
                .step_by(0.01)
                .show_value(false)
        ).changed();

        if d_changed {
            info!("NPC 阻尼系数已调整为: {:.2}", npc_config.damping);
        }

        ui.add_space(2.0);
        ui.label(egui::RichText::new("0.50 = 很快停止（摩擦力大）")
            .size(11.0)
            .color(egui::Color32::GRAY));
        ui.label(egui::RichText::new("0.98 = 滑行很远（摩擦力小）")
            .size(11.0)
            .color(egui::Color32::GRAY));
    });
}

/// 将 NpcCollisionConfig.push_force 同步到所有 NPC 的 CollisionResponse
fn apply_npc_push_force(
    config: Res<NpcCollisionConfig>,
    mut npc_q: Query<(&mut CollisionResponse, &mut SmoothPush), With<crate::npc::Npc>>,
) {
    if !config.is_changed() {
        return;
    }

    for (mut response, mut smooth) in npc_q.iter_mut() {
        response.push_force = config.push_force;
        smooth.damping = config.damping;
    }
}
