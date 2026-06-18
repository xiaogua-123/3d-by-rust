//! 图片画廊 — 在游戏中加载本地图片并交互展示
//!
//! 自动扫描 `assets/images/` 目录，在 egui 面板中展示缩略图网格。
//! 支持全屏查看、背景选择、翻页浏览等功能。

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

pub struct ImageGalleryPlugin;

impl Plugin for ImageGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ImageManager>()
            .add_systems(EguiPrimaryContextPass, (
                load_gallery.run_if(in_state(crate::game_state::GamePhase::MainMenu)),
                gallery_toggle_key,
            ));
    }
}

/// 图片管理器 — 存储所有加载的图片和交互状态
#[derive(Resource)]
#[derive(Default)]
pub struct ImageManager {
    pub images: Vec<GalleryImage>,
    pub selected: Option<usize>,
    pub show_gallery: bool,
    /// 当前选中的背景图索引
    pub bg_index: Option<usize>,
}


/// 单张图片数据（持有 TextureHandle 防止被释放）
pub struct GalleryImage {
    pub name: String,
    pub handle: egui::TextureHandle,
    pub width: u32,
    pub height: u32,
}

impl GalleryImage {
    pub fn texture_id(&self) -> egui::TextureId {
        self.handle.id()
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 { return 1.0; }
        self.width as f32 / self.height as f32
    }
}

// ── 图片加载 ──

fn load_gallery(
    mut contexts: EguiContexts,
    mut manager: ResMut<ImageManager>,
    mut loaded: Local<bool>,
) {
    if *loaded {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };
    *loaded = true;

    // 从多个目录加载图片
    let dirs = [
        std::path::PathBuf::from("assets/images"),
        std::path::PathBuf::from("assets/textures"),
    ];

    let mut count = 0usize;

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { continue };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }

            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if !["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) {
                continue;
            }

            let name = path.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // 跳过已加载的（同名去重）
            if manager.images.iter().any(|img| img.name == name) {
                continue;
            }

            let Ok(reader) = image::ImageReader::open(&path) else {
                warn!("无法打开图片: {:?}", path);
                continue;
            };
            let Ok(rgba_img) = reader.decode() else {
                warn!("无法解码图片: {:?}", path);
                continue;
            };

            let rgba = rgba_img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let pixels = rgba.into_raw();

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &pixels,
            );

            let handle = ctx.load_texture(&name, color_image, egui::TextureOptions::LINEAR);

            manager.images.push(GalleryImage {
                name,
                handle,
                width: w,
                height: h,
            });
            count += 1;
        }
    }

    info!("图片画廊: 已加载 {} 张图片", count);
}

// ── 键盘快捷键 ──

fn gallery_toggle_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut manager: ResMut<ImageManager>,
) {
    if keys.just_pressed(KeyCode::KeyG) && !manager.images.is_empty() {
        manager.show_gallery = !manager.show_gallery;
    }
}

// ── 画廊 UI 组件 ──

/// 绘制图片缩略图网格，返回被点击的图片索引
pub fn gallery_grid(ui: &mut egui::Ui, manager: &mut ImageManager) -> Option<usize> {
    let mut clicked: Option<usize> = None;

    if manager.images.is_empty() {
        ui.label(egui::RichText::new("暂无图片").color(egui::Color32::GRAY));
        return None;
    }

    let thumb_size = 110.0;
    let padding = 10.0;
    let cols = ((ui.available_width() - padding) / (thumb_size + padding)).max(1.0) as usize;

    egui::Grid::new("gallery_grid")
        .spacing(egui::vec2(padding, padding))
        .show(ui, |ui| {
            for (i, img) in manager.images.iter().enumerate() {
                let is_selected = manager.selected == Some(i);

                let frame = egui::Frame {
                    fill: if is_selected {
                        egui::Color32::from_rgb(0x00, 0x66, 0xcc)
                    } else {
                        egui::Color32::from_rgb(0x1a, 0x1a, 0x2e)
                    },
                    inner_margin: egui::Margin::same(4),
                    corner_radius: 6.0.into(),
                    stroke: egui::epaint::Stroke::new(
                        1.0,
                        if is_selected {
                            egui::Color32::from_rgb(0x44, 0xaa, 0xff)
                        } else {
                            egui::Color32::from_rgb(0x33, 0x33, 0x55)
                        },
                    ),
                    ..Default::default()
                };

                frame.show(ui, |ui| {
                    let aspect = img.aspect_ratio();
                    let (tw, th) = if aspect >= 1.0 {
                        (thumb_size, thumb_size / aspect)
                    } else {
                        (thumb_size * aspect, thumb_size)
                    };

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(tw, th),
                        egui::Sense::click(),
                    );

                    if ui.is_rect_visible(rect) {
                        ui.painter().image(
                            img.texture_id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&img.name)
                            .size(10.0)
                            .color(egui::Color32::LIGHT_GRAY),
                    );

                    if response.clicked() {
                        clicked = Some(i);
                    }
                });

                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });

    clicked
}

/// 全尺寸显示单张图片（自动缩放适应可用区域）
pub fn show_image_fit(ui: &mut egui::Ui, img: &GalleryImage) {
    let available = ui.available_size();
    let aspect = img.aspect_ratio();

    let (w, h) = if available.x / available.y > aspect {
        (available.y * aspect, available.y)
    } else {
        (available.x, available.x / aspect)
    };

    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        ui.painter().image(
            img.texture_id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

/// 菜单用可点击图片卡片 — 带名称标签、悬停高亮、圆角边框
pub fn image_card_menu(
    ui: &mut egui::Ui,
    img: &GalleryImage,
    max_size: egui::Vec2,
    bg_color: egui::Color32,
    accent_color: egui::Color32,
) -> bool {
    let aspect = img.aspect_ratio();
    let (w, h) = if max_size.x / max_size.y > aspect {
        (max_size.y * aspect, max_size.y)
    } else {
        (max_size.x, max_size.x / aspect)
    };

    let total_w = w.max(120.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(total_w, h + 22.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let img_rect = egui::Rect::from_min_size(rect.min, egui::vec2(w, h));
        let is_hover = response.hovered();

        // 图片背景框
        ui.painter().rect_filled(
            img_rect.expand(2.0),
            8.0,
            if is_hover { accent_color.linear_multiply(0.3) } else { bg_color },
        );
        ui.painter().rect_stroke(
            img_rect.expand(2.0),
            8.0,
            egui::epaint::Stroke::new(
                if is_hover { 2.0 } else { 1.0 },
                if is_hover { accent_color } else { egui::Color32::from_rgb(0x55, 0x55, 0x77) },
            ),
            egui::StrokeKind::Inside,
        );

        // 图片
        let tint = if is_hover {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 230)
        } else {
            egui::Color32::WHITE
        };
        ui.painter().image(img.texture_id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);

        // 名称标签
        let label_pos = egui::pos2(rect.min.x + 4.0, img_rect.max.y + 4.0);
        ui.painter().text(
            label_pos,
            egui::Align2::LEFT_TOP,
            &img.name,
            egui::FontId::proportional(11.0),
            if is_hover { accent_color } else { egui::Color32::from_rgb(0xaa, 0xaa, 0xcc) },
        );
    }

    response.clicked()
}

/// 菜单背景图选择条 — 用小缩略图切换菜单背景
pub fn background_selector(
    ui: &mut egui::Ui,
    manager: &ImageManager,
    current_bg: Option<usize>,
) -> Option<usize> {
    if manager.images.is_empty() {
        return None;
    }

    let mut clicked: Option<usize> = None;
    let thumb_size = 64.0;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("背景:")
                .size(12.0)
                .color(egui::Color32::from_rgb(0x88, 0x88, 0xbb)),
        );
        ui.add_space(4.0);

        let scroll = egui::ScrollArea::horizontal().id_salt("bg_selector");
        scroll.show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, img) in manager.images.iter().enumerate() {
                    let is_active = current_bg == Some(i);
                    let aspect = img.aspect_ratio();
                    let (tw, th) = if aspect >= 1.0 {
                        (thumb_size, thumb_size / aspect)
                    } else {
                        (thumb_size * aspect, thumb_size)
                    };

                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(tw.max(40.0) + 4.0, th + 4.0),
                        egui::Sense::click(),
                    );

                    if ui.is_rect_visible(rect) {
                        let img_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + 2.0, rect.min.y + 2.0),
                            egui::vec2(tw, th),
                        );

                        let border_color = if is_active {
                            egui::Color32::from_rgb(0x00, 0xd4, 0xff)
                        } else if resp.hovered() {
                            egui::Color32::from_rgb(0x66, 0x66, 0x99)
                        } else {
                            egui::Color32::from_rgb(0x33, 0x33, 0x55)
                        };

                        ui.painter().rect_stroke(
                            img_rect.expand(1.0),
                            4.0,
                            egui::epaint::Stroke::new(if is_active { 2.0 } else { 1.0 }, border_color),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().image(
                            img.texture_id(),
                            img_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            });
        });
    });

    clicked
}

/// 带边框和悬停效果的图片按钮
pub fn image_button(
    ui: &mut egui::Ui,
    img: &GalleryImage,
    max_size: egui::Vec2,
) -> bool {
    let aspect = img.aspect_ratio();
    let (w, h) = if max_size.x / max_size.y > aspect {
        (max_size.y * aspect, max_size.y)
    } else {
        (max_size.x, max_size.x / aspect)
    };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        // 边框
        ui.painter().rect_stroke(
            rect.expand(2.0),
            6.0,
            egui::epaint::Stroke::new(1.0, egui::Color32::from_rgb(0x55, 0x55, 0x77)),
            egui::StrokeKind::Inside,
        );

        // 图片本身（悬停时略微变暗 = 交互反馈）
        let tint = if response.hovered() {
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 210)
        } else {
            egui::Color32::WHITE
        };

        ui.painter().image(
            img.texture_id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }

    response.clicked()
}

/// 获取适合做菜单背景的图片（优先 bg_index，其次 name 含 menu/background/bg 的，否则用第一张）
pub fn find_menu_background(manager: &ImageManager) -> Option<&GalleryImage> {
    // 优先 bg_index
    if let Some(idx) = manager.bg_index
        && idx < manager.images.len() {
            return Some(&manager.images[idx]);
        }
    // 其次找名称带背景含义的
    for img in &manager.images {
        let lower = img.name.to_lowercase();
        if lower.contains("menu") || lower.contains("background") || lower == "bg" {
            return Some(img);
        }
    }
    // 否则取第一张
    manager.images.first()
}
