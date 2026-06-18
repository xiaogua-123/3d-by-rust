//! Solari 光追展厅 — PBR 材质展示
//!
//! 一个交互式 PBR 材质展厅，展示金属度 × 粗糙度的材质变化、
//! 玻璃透射、镜面反射、发光材质和点光源效果。
//! 玩家可在展厅中自由行走，从各个角度观察材质表现。

#![allow(dead_code)]

use bevy::prelude::*;
use crate::camera::CameraController;
use crate::level::LevelEntity;
use crate::render_utils::{self, Rotating};

// ═══════════════════════════════════════════
// 公开接口
// ═══════════════════════════════════════════

/// 生成 Solari PBR 材质展厅的全部内容
pub(crate) fn spawn_pbr_showcase(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    spawn_reflective_floor(commands, meshes, materials);
    spawn_material_grid(commands, meshes, materials);
    spawn_center_torus(commands, meshes, materials);
    spawn_glass_collection(commands, meshes, materials);
    spawn_mirror_collection(commands, meshes, materials);
    spawn_emissive_towers(commands, meshes, materials);
    spawn_point_lights(commands, meshes, materials);
    spawn_directional_light(commands);
    spawn_free_camera(commands);
    spawn_help_text(commands);
    spawn_glowing_cube(commands, meshes, materials);
}

// 注：旋转动画由 main.rs 全局注册的 render_utils::animate_rotation 统一处理

// ═══════════════════════════════════════════
// 内部辅助（材质委托给共享工具 render_utils）
// ═══════════════════════════════════════════

// 使用 render_utils::pbr_mat / glass_mat 替代本地版本

// ═══════════════════════════════════════════
// 场景各部件
// ═══════════════════════════════════════════

/// 1. 反光地面（深色微反光）
fn spawn_reflective_floor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0).build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.06, 0.06, 0.10),
            0.0,
            0.12,
            LinearRgba::BLACK,
        )),
        LevelEntity,
        Name::new("Solari_Floor"),
    ));
}

/// 2. 3×3 材质球矩阵
fn spawn_material_grid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let sphere_mesh = meshes.add(Sphere::new(0.4).mesh().build());

    let metalness_vals = [0.0, 0.5, 1.0];
    let roughness_vals = [0.1, 0.4, 0.8];
    let row_colors = [
        Color::srgb(0.25, 0.55, 0.90), // 非金属：蓝
        Color::srgb(0.70, 0.50, 0.15), // 半金属：铜
        Color::srgb(0.80, 0.80, 0.82), // 金属：银
    ];

    for (mi, &metalness) in metalness_vals.iter().enumerate() {
        for (ri, &roughness) in roughness_vals.iter().enumerate() {
            let x = -1.8 + ri as f32 * 1.8;
            let z = 4.0 - mi as f32 * 1.5;
            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(render_utils::pbr_mat(
                    materials,
                    row_colors[mi],
                    metalness,
                    roughness,
                    LinearRgba::BLACK,
                )),
                Transform::from_xyz(x, 0.4, z),
                LevelEntity,
                Name::new(format!("Solari_Mat_{}_{}", mi, ri)),
            ));
        }
    }
}

/// 3. 中心金色圆环（旋转）
fn spawn_center_torus(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(1.0, 0.25).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.85, 0.70, 0.20),
            1.0,
            0.15,
            LinearRgba::BLACK,
        )),
        Transform::from_xyz(0.0, 1.2, 0.0),
        Rotating(0.8),
        LevelEntity,
        Name::new("Solari_Torus"),
    ));

    // 圆环下方小底座
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.2)).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.3, 0.3, 0.35),
            0.8,
            0.3,
            LinearRgba::BLACK,
        )),
        Transform::from_xyz(0.0, 0.1, 0.0),
        LevelEntity,
        Name::new("Solari_TorusBase"),
    ));
}

/// 4. 玻璃系列（东侧）
fn spawn_glass_collection(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let base_x = 4.2;
    let base_z = -0.5;

    // 玻璃球
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().build())),
        MeshMaterial3d(render_utils::glass_mat(
            materials,
            Color::srgba(0.70, 0.85, 1.0, 0.35),
            0.05,
        )),
        Transform::from_xyz(base_x, 0.5, base_z),
        LevelEntity,
        Name::new("Solari_GlassSphere"),
    ));

    // 玻璃柱
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.3, 1.2).mesh().build())),
        MeshMaterial3d(render_utils::glass_mat(
            materials,
            Color::srgba(0.60, 0.80, 1.0, 0.30),
            0.08,
        )),
        Transform::from_xyz(base_x, 0.6, base_z - 1.5),
        LevelEntity,
        Name::new("Solari_GlassCylinder"),
    ));

    // 玻璃方块
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.7)).mesh().build())),
        MeshMaterial3d(render_utils::glass_mat(
            materials,
            Color::srgba(0.80, 0.90, 1.0, 0.25),
            0.10,
        )),
        Transform::from_xyz(base_x, 0.35, base_z + 1.5),
        LevelEntity,
        Name::new("Solari_GlassCube"),
    ));
}

/// 5. 镜面系列（西侧）
fn spawn_mirror_collection(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let base_x = -4.2;
    let base_z = -0.5;

    // 镜面球
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.85, 0.85, 0.88),
            1.0,
            0.05,
            LinearRgba::BLACK,
        )),
        Transform::from_xyz(base_x, 0.5, base_z),
        LevelEntity,
        Name::new("Solari_MirrorSphere"),
    ));

    // 镜面柱
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.3, 1.2).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.80, 0.82, 0.85),
            1.0,
            0.08,
            LinearRgba::BLACK,
        )),
        Transform::from_xyz(base_x, 0.6, base_z - 1.5),
        LevelEntity,
        Name::new("Solari_MirrorCylinder"),
    ));

    // 镜面方块
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.7)).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            Color::srgb(0.82, 0.82, 0.85),
            1.0,
            0.10,
            LinearRgba::BLACK,
        )),
        Transform::from_xyz(base_x, 0.35, base_z + 1.5),
        LevelEntity,
        Name::new("Solari_MirrorCube"),
    ));
}

/// 6. 发光塔（北侧）
fn spawn_emissive_towers(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let tower_positions = [
        (Vec3::new(-2.5, 0.0, -4.0), Color::srgb(1.0, 0.2, 0.1)),
        (Vec3::new(0.0, 0.0, -4.5), Color::srgb(0.1, 0.8, 0.3)),
        (Vec3::new(2.5, 0.0, -4.0), Color::srgb(0.2, 0.4, 1.0)),
    ];

    for (i, (pos, color)) in tower_positions.iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.15, 2.0).mesh().build())),
            MeshMaterial3d(render_utils::pbr_mat(
                materials,
                *color,
                0.0,
                0.4,
                LinearRgba::from(*color) * 3.0,
            )),
            Transform::from_translation(*pos),
            LevelEntity,
            Name::new(format!("Solari_Tower_{}", i)),
        ));
    }
}

/// 7. 彩色点光源（带可见光球）
fn spawn_point_lights(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let light_configs = [
        (Vec3::new(-5.0, 3.5, 2.0), Color::srgb(1.0, 0.6, 0.2), 3000.0),
        (Vec3::new(5.0, 3.0, -2.0), Color::srgb(0.2, 0.5, 1.0), 3000.0),
        (Vec3::new(0.0, 4.0, -3.0), Color::srgb(0.8, 0.3, 1.0), 2500.0),
    ];

    let glow_mesh = meshes.add(Sphere::new(0.12).mesh().build());

    for (i, (pos, color, intensity)) in light_configs.iter().enumerate() {
        commands.spawn((
            PointLight {
                color: *color,
                intensity: *intensity,
                range: 12.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos),
            LevelEntity,
            Name::new(format!("Solari_PointLight_{}", i)),
        ));

        commands.spawn((
            Mesh3d(glow_mesh.clone()),
            MeshMaterial3d(render_utils::pbr_mat(
                materials,
                *color,
                0.0,
                0.2,
                LinearRgba::from(*color) * 3.0,
            )),
            Transform::from_translation(*pos),
            LevelEntity,
            Name::new(format!("Solari_LightGlow_{}", i)),
        ));
    }
}

/// 8. 方向光（微弱环境照明）
fn spawn_directional_light(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_xyzw(-0.13, -0.87, -0.36, 0.32)),
        LevelEntity,
        Name::new("Solari_DirLight"),
    ));
}

/// 9. 自由相机（第三人称环绕）
fn spawn_free_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.04)),
            ..default()
        },
        Transform::from_translation(Vec3::new(7.0, 4.5, 9.0))
            .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        CameraController::default(),
        LevelEntity,
        Name::new("Solari_Camera"),
    ));
}

/// 10. 操作提示文字
fn spawn_help_text(commands: &mut Commands) {
    commands.spawn((
        Text::new("光追展厅 · WASD 移动 · 鼠标环视 · Backspace 释放/锁定光标 · 6 返回"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node::default(),
        LevelEntity,
        Name::new("Solari_HelpText"),
    ));
}

// 新增：发光浮动立方体
fn spawn_glowing_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let color = Color::srgb(1.0, 0.8, 0.2);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.6)).mesh().build())),
        MeshMaterial3d(render_utils::pbr_mat(
            materials,
            color,
            0.0,
            0.3,
            LinearRgba::from(color) * 2.0,
        )),
        Transform::from_xyz(0.0, 2.5, -2.0),
        Rotating(0.5),
        LevelEntity,
        Name::new("Solari_GlowingCube"),
    ));
}