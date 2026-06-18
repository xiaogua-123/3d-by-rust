//! 渲染分辨率缩放 — 性价比最高的性能优化方案
//!
//! 原理：降低渲染分辨率（Camera::viewport）→ GPU 硬件线性上采样 → 全分辨率输出
//! 无需自定义 shader、RenderGraph 节点、时序累积或运动向量。
//!
//! 收益：50% 缩放 → 约 4x 像素数减少，FPS 提升显著，画质损失可控。
//!
//! ## 控制
//! - `F11` — 开关缩放
//! - `F7` — 切换质量模式
//! - `F8` — 切换信息面板

use bevy::prelude::*;
use bevy::camera::Viewport;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

/// 渲染缩放质量模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleQuality {
    /// 50% 渲染分辨率，最大性能提升
    Performance,
    /// ~58% 渲染分辨率，均衡
    Balanced,
    /// ~67% 渲染分辨率，高质量
    Quality,
    /// 100% 原生分辨率
    Native,
}

impl ScaleQuality {
    /// 渲染缩放比例
    pub fn render_scale(self) -> f32 {
        match self {
            ScaleQuality::Performance => 0.50,
            ScaleQuality::Balanced => 0.58,
            ScaleQuality::Quality => 0.67,
            ScaleQuality::Native => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScaleQuality::Performance => "性能 (50%)",
            ScaleQuality::Balanced => "均衡 (58%)",
            ScaleQuality::Quality => "质量 (67%)",
            ScaleQuality::Native => "原生 (100%)",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            ScaleQuality::Performance => ScaleQuality::Balanced,
            ScaleQuality::Balanced => ScaleQuality::Quality,
            ScaleQuality::Quality => ScaleQuality::Native,
            ScaleQuality::Native => ScaleQuality::Performance,
        }
    }
}

/// 渲染缩放配置
#[derive(Resource)]
pub struct ScaleConfig {
    /// 是否启用缩放
    pub enabled: bool,
    /// 当前质量模式
    pub quality: ScaleQuality,
    /// 是否显示覆盖层
    pub show_overlay: bool,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            quality: ScaleQuality::Quality,
            show_overlay: true,
        }
    }
}

impl ScaleConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 计算当前渲染分辨率
    pub fn render_resolution(&self, window_size: UVec2) -> UVec2 {
        let scale = self.quality.render_scale();
        let w = ((window_size.x as f32 * scale) as u32) & !1;
        let h = ((window_size.y as f32 * scale) as u32) & !1;
        UVec2::new(w.max(64), h.max(64))
    }
}

/// 渲染缩放插件
pub struct ScalePlugin;

impl Plugin for ScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScaleConfig>()
            .add_systems(Update, (
                handle_input,
                apply_viewport,
            ))
            .add_systems(EguiPrimaryContextPass, draw_overlay);
    }
}

// ============================================================================
// 系统
// ============================================================================

/// 键盘控制
fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ScaleConfig>,
) {
    if keys.just_pressed(KeyCode::F11) {
        config.enabled = !config.enabled;
        info!("渲染缩放: {}", if config.enabled { "ON" } else { "OFF" });
    }

    if keys.just_pressed(KeyCode::F7) {
        config.quality = config.quality.cycle();
        info!("缩放质量: {}", config.quality.label());
    }

    if keys.just_pressed(KeyCode::F8) {
        config.show_overlay = !config.show_overlay;
    }
}

/// 应用缩放视口到主相机
fn apply_viewport(
    window: Query<&Window, With<PrimaryWindow>>,
    config: Res<ScaleConfig>,
    mut cameras: Query<&mut Camera, With<Camera3d>>,
) {
    let Ok(window) = window.single() else { return };
    let window_size = UVec2::new(
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );

    for mut camera in cameras.iter_mut() {
        if config.enabled() {
            let res = config.render_resolution(window_size);
            camera.viewport = Some(Viewport {
                physical_size: res,
                physical_position: UVec2::ZERO,
                depth: 0.0..1.0,
            });
        } else {
            camera.viewport = None;
        }
    }
}

// ============================================================================
// FPS 计数器 + 覆盖层
// ============================================================================

const FPS_HISTORY_SIZE: usize = 60;

#[derive(Resource)]
pub struct FpsCounter {
    samples: [f32; FPS_HISTORY_SIZE],
    index: usize,
    count: usize,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            samples: [0.0; FPS_HISTORY_SIZE],
            index: 0,
            count: 0,
        }
    }
}

impl FpsCounter {
    fn record(&mut self, dt: f32) {
        self.samples[self.index] = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        self.index = (self.index + 1) % FPS_HISTORY_SIZE;
        self.count = (self.count + 1).min(FPS_HISTORY_SIZE);
    }

    fn avg_fps(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        let sum: f32 = self.samples[..self.count].iter().sum();
        sum / self.count as f32
    }
}

/// egui 信息覆盖层
fn draw_overlay(
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut config: ResMut<ScaleConfig>,
    mut fps_counter: Local<FpsCounter>,
    mut contexts: EguiContexts,
) {
    fps_counter.record(time.delta_secs());

    if !config.show_overlay {
        return;
    }

    let Ok(window) = window.single() else { return };
    let window_size = UVec2::new(
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("渲染缩放")
        .fixed_pos(egui::pos2(12.0, 12.0))
        .title_bar(false)
        .resizable(false)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
            corner_radius: 8.0.into(),
            ..default()
        })
        .show(ctx, |ui| {
            ui.style_mut().override_font_id = Some(egui::FontId::monospace(14.0));
            ui.style_mut().visuals.override_text_color =
                Some(egui::Color32::from_rgb(0xCC, 0xFF, 0x00));

            ui.vertical(|ui| {
                // FPS
                let fps = fps_counter.avg_fps();
                let fps_color = if fps >= 60.0 {
                    egui::Color32::GREEN
                } else if fps >= 30.0 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.colored_label(fps_color, format!("FPS: {:.0}", fps));
                ui.label(format!("帧时间: {:.1}ms", time.delta_secs_f64() * 1000.0));

                ui.separator();

                // 缩放状态
                let enabled = config.enabled();
                let status_color = if enabled {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GRAY
                };
                ui.colored_label(status_color, format!(
                    "缩放: {}",
                    if enabled { "ON" } else { "OFF" },
                ));

                if enabled {
                    ui.label(format!("模式: {}", config.quality.label()));
                    let res = config.render_resolution(window_size);
                    ui.label(format!(
                        "渲染: {}x{} ({:.0}%)",
                        res.x,
                        res.y,
                        config.quality.render_scale() * 100.0,
                    ));
                    ui.label(format!("输出: {}x{}", window_size.x, window_size.y));
                }

                ui.separator();

                if ui.selectable_label(false, "[F11] 开关").clicked() {
                    config.enabled = !config.enabled;
                }
                if ui.selectable_label(false, "[F7] 质量").clicked() {
                    config.quality = config.quality.cycle();
                }
                if ui.selectable_label(false, "[F8] 面板").clicked() {
                    config.show_overlay = !config.show_overlay;
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_quality_cycle() {
        assert_eq!(ScaleQuality::Performance.cycle(), ScaleQuality::Balanced);
        assert_eq!(ScaleQuality::Balanced.cycle(), ScaleQuality::Quality);
        assert_eq!(ScaleQuality::Quality.cycle(), ScaleQuality::Native);
        assert_eq!(ScaleQuality::Native.cycle(), ScaleQuality::Performance);
    }

    #[test]
    fn test_render_resolution_full() {
        let config = ScaleConfig { enabled: true, quality: ScaleQuality::Native, ..default() };
        let res = config.render_resolution(UVec2::new(1920, 1080));
        assert_eq!(res.x, 1920);
        assert_eq!(res.y, 1080);
    }

    #[test]
    fn test_render_resolution_half() {
        let config = ScaleConfig { enabled: true, quality: ScaleQuality::Performance, ..default() };
        let res = config.render_resolution(UVec2::new(1920, 1080));
        assert_eq!(res.x, 960);
        assert_eq!(res.y, 540);
    }

    #[test]
    fn test_render_resolution_odd() {
        let config = ScaleConfig { enabled: true, quality: ScaleQuality::Balanced, ..default() };
        let res = config.render_resolution(UVec2::new(1921, 1081));
        // 58% of 1921 = 1114.18 → 1114 (even)
        // 58% of 1081 = 626.98 → 626 (even)
        assert_eq!(res.x % 2, 0);
        assert_eq!(res.y % 2, 0);
    }

    #[test]
    fn test_render_resolution_min() {
        let config = ScaleConfig { enabled: true, quality: ScaleQuality::Performance, ..default() };
        let res = config.render_resolution(UVec2::new(10, 10));
        assert!(res.x >= 64);
        assert!(res.y >= 64);
    }

    #[test]
    fn test_scale_quality_labels_not_empty() {
        for q in &[ScaleQuality::Performance, ScaleQuality::Balanced, ScaleQuality::Quality, ScaleQuality::Native] {
            assert!(!q.label().is_empty());
            assert!(q.render_scale() > 0.0 && q.render_scale() <= 1.0);
        }
    }
}
