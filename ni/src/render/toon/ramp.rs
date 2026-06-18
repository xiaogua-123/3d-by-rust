//! Toon Ramp 纹理生成
//!
//! 程序化生成卡通着色所需的 Ramp 纹理贴图，提供运行时淡入/淡出过渡效果。

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// 启动时生成的Ramp贴图句柄
#[derive(Resource, Clone)]
pub struct RampTextureHandle(pub Handle<Image>);

/// 程序化生成三渲二的Ramp贴图 (256x1, RGBA)
/// 3阶色阶: 暗色(阴影) | 中间色(固有色) | 亮色(亮面)
pub fn generate_ramp_texture(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let width = 256u32;
    let height = 1u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for x in 0..width {
        let idx = (x * 4) as usize;

        // 将 [0, 255] 映射到 [0.0, 1.0]
        let t = x as f32 / (width - 1) as f32;

        // 3阶硬边: 暗 / 中 / 亮
        // 边界位置: 0.33 (暗→中), 0.66 (中→亮)
        let (r, g, b) = if t < 0.33 {
            // 暗面 — 深色
            (0.15, 0.12, 0.18)
        } else if t < 0.66 {
            // 中间面 — 固有色区域（中等亮度）
            (0.55, 0.50, 0.55)
        } else {
            // 亮面 — 接近白色
            (0.95, 0.92, 0.90)
        };

        pixels[idx] = (r * 255.0) as u8;
        pixels[idx + 1] = (g * 255.0) as u8;
        pixels[idx + 2] = (b * 255.0) as u8;
        pixels[idx + 3] = 255u8;
    }

    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);
    commands.insert_resource(RampTextureHandle(handle));
}
