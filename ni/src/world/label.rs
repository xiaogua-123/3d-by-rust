//! 世界空间名字标签 — 在 3D 实体头顶绘制文字
//!
//! 将 WorldLabel 组件附加到需要标签的实体上，
//! 系统会自动将 3D 坐标投影到屏幕并用 egui 绘制名字。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

/// 世界空间标签组件（附加到要显示名字的实体）
#[derive(Component)]
pub struct WorldLabel {
    pub text: String,
    /// 实体头顶偏移高度
    pub offset: f32,
    /// egui 字体大小（点）
    pub font_size: f32,
    /// 文字颜色
    pub color: egui::Color32,
}

impl WorldLabel {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            offset: 2.5,
            font_size: 14.0,
            color: egui::Color32::WHITE,
        }
    }

    pub fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    #[allow(dead_code)]
    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
}

/// 在世界空间绘制名字标签的系统
/// 运行在 Update 中，使用 EguiContexts 在每帧绘制
pub fn draw_world_labels(
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    labels: Query<(Entity, &WorldLabel, &GlobalTransform), Without<Camera3d>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut contexts: EguiContexts,
) {
    let Ok((cam, cam_transform)) = cameras.single() else { return };
    let Ok(window) = window.single() else { return };

    let Some(viewport_size) = cam.logical_viewport_size() else { return };
    let cam_pos = cam_transform.translation();
    let cam_forward = cam_transform.forward();
    let scale = window.scale_factor();

    // 获取视口相对窗口的偏移（物理像素）
    let vp_offset_phys = match &cam.viewport {
        Some(vp) => Vec2::new(vp.physical_position.x as f32, vp.physical_position.y as f32),
        None => Vec2::ZERO,
    };
    // 转换到逻辑像素
    let vp_offset = vp_offset_phys / scale;

    let Ok(ctx) = contexts.ctx_mut() else { return };

    for (entity, label, transform) in labels.iter() {
        let world_pos = transform.translation() + Vec3::Y * label.offset;

        // 剔除相机背后的实体
        if (world_pos - cam_pos).dot(*cam_forward) <= 0.0 {
            continue;
        }

        // world_to_viewport 返回逻辑像素坐标
        let Ok(vp_coord) = cam.world_to_viewport(cam_transform, world_pos) else {
            continue;
        };

        // 剔除超出视口边界的标签
        if vp_coord.x < 0.0 || vp_coord.y < 0.0
            || vp_coord.x > viewport_size.x || vp_coord.y > viewport_size.y
        {
            continue;
        }

        // 视口逻辑坐标 + 窗口偏移逻辑坐标 → egui 逻辑坐标
        let egui_pos = egui::pos2(vp_offset.x + vp_coord.x, vp_offset.y + vp_coord.y);

        // 使用 Entity 的 bits 作为唯一 ID，防止同名实体 ID 冲突
        let area_id = egui::Id::new(entity);

        // 绘制带背景的标签文字
        egui::Area::new(area_id)
            .fixed_pos(egui_pos)
            .anchor(egui::Align2::CENTER_BOTTOM, (0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(160))
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&label.text)
                            .font(egui::FontId::proportional(label.font_size))
                            .color(label.color),
                    );
                });
            });
    }
}

/// 世界标签插件
pub struct WorldLabelPlugin;

impl Plugin for WorldLabelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, draw_world_labels);
    }
}
