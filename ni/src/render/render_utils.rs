//! 共享渲染工具 — 提取自 Solari PBR 展厅
//!
//! 提供材质工厂函数、旋转动画组件和装饰物辅助，供所有关卡复用。

use bevy::prelude::*;

// ═══════════════════════════════════════════
// 材质工厂
// ═══════════════════════════════════════════

/// 创建标准 PBR 材质（金属/粗糙度/自发光可控）
pub fn pbr_mat(
    materials: &mut Assets<StandardMaterial>,
    base_color: Color,
    metallic: f32,
    roughness: f32,
    emissive: LinearRgba,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color,
        perceptual_roughness: roughness,
        metallic,
        emissive,
        ..default()
    })
}

/// 创建半透明玻璃材质
pub fn glass_mat(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    roughness: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: roughness,
        metallic: 0.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    })
}

/// 创建自发光材质（color * intensity 倍乘 emissive）
pub fn emissive_mat(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    intensity: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.3,
        metallic: 0.0,
        emissive: LinearRgba::from(color) * intensity,
        ..default()
    })
}

// ═══════════════════════════════════════════
// 旋转动画
// ═══════════════════════════════════════════

/// 旋转动画标记，值为角速度（弧度/秒）
#[derive(Component)]
pub struct Rotating(pub f32);

/// 绕 Y 轴旋转所有带 Rotating 标记的实体
pub fn animate_rotation(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &Rotating)>,
) {
    for (mut transform, rotating) in q.iter_mut() {
        transform.rotate_y(rotating.0 * time.delta_secs());
    }
}

// ═══════════════════════════════════════════
// 装饰物辅助
// ═══════════════════════════════════════════

/// 发光球配置参数
pub struct GlowOrbConfig {
    pub position: Vec3,
    pub color: Color,
    pub glow_intensity: f32,
    pub size: f32,
    pub name: String,
}

/// 生成发光装饰小球（常用于标记光源位置或做氛围装饰）
pub fn spawn_glow_orb(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    config: GlowOrbConfig,
) {
    let glow_mesh = meshes.add(Sphere::new(config.size).mesh().build());
    commands.spawn((
        Mesh3d(glow_mesh),
        MeshMaterial3d(emissive_mat(materials, config.color, config.glow_intensity)),
        Transform::from_translation(config.position),
        Name::new(config.name),
    ));
}

/// 用于地面或墙壁的深色微反光材质（通用环境装饰）
#[allow(dead_code)]
pub fn dark_reflective_mat(
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    pbr_mat(
        materials,
        Color::srgb(0.06, 0.06, 0.10),
        0.0,
        0.12,
        LinearRgba::BLACK,
    )
}
