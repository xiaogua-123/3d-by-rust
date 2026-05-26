//! Solari 实时光追关卡

use bevy::prelude::*;
use rand::Rng;

use crate::camera::CameraController;
use crate::level::LevelEntity;

/// 生成 Solari 实时光追场景（由关卡系统调用）
pub(crate) fn spawn_solari_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // ---------- 地面 ----------
    let ground_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(20.0, 20.0)
            .build()
            .with_generated_tangents()
            .unwrap(),
    );
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            perceptual_roughness: 0.9,
            ..default()
        })),
        LevelEntity,
        Name::new("Solari_Ground"),
    ));

    // ---------- 几个彩色方块 ----------
    let cube_mesh = meshes.add(
        Cuboid::default()
            .mesh()
            .build()
            .with_generated_tangents()
            .unwrap(),
    );

    let colors = [
        (Color::srgb(0.9, 0.2, 0.2), Vec3::new(-3.0, 0.5, 0.0)),
        (Color::srgb(0.2, 0.7, 0.2), Vec3::new(0.0, 0.5, 0.0)),
        (Color::srgb(0.2, 0.2, 0.9), Vec3::new(3.0, 0.5, 0.0)),
        (Color::srgb(0.9, 0.9, 0.2), Vec3::new(-1.5, 1.5, 2.0)),
        (Color::srgb(0.7, 0.3, 0.7), Vec3::new(1.5, 1.5, 2.0)),
    ];

    for (i, (color, pos)) in colors.iter().enumerate() {
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: *color,
                perceptual_roughness: 0.3,
                metallic: 0.1,
                ..default()
            })),
            Transform::from_translation(*pos),
            LevelEntity,
            Name::new(format!("Solari_Cube_{i}")),
        ));
    }

    // ---------- 一个金属球 ----------
    let sphere_mesh = meshes.add(
        Sphere::new(0.6)
            .mesh()
            .build()
            .with_generated_tangents()
            .unwrap(),
    );
    commands.spawn((
        Mesh3d(sphere_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.7, 0.3),
            perceptual_roughness: 0.05,
            metallic: 1.0,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, 0.6, -2.0)),
        LevelEntity,
        Name::new("Solari_MetalSphere"),
    ));

    // ---------- 发光球体（作为光源）----------
    let light_sphere = meshes.add(
        Sphere::new(0.15)
            .mesh()
            .build()
            .with_generated_tangents()
            .unwrap(),
    );

    let mut rng = rand::thread_rng();
    let light_positions = [
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(-4.0, 2.0, -3.0),
        Vec3::new(4.0, 2.0, -3.0),
    ];

    for (i, &pos) in light_positions.iter().enumerate() {
        let emissive_color = Color::linear_rgb(
            rng.gen_range(0.5..1.0),
            rng.gen_range(0.5..1.0),
            rng.gen_range(0.5..1.0),
        );
        commands.spawn((
            Mesh3d(light_sphere.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                emissive: LinearRgba::from(emissive_color) * 500.0,
                ..default()
            })),
            Transform::from_translation(pos),
            LevelEntity,
            Name::new(format!("Solari_Light_{i}")),
        ));
    }

    // ---------- 方向光 ----------
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_xyzw(
            -0.13334629,
            -0.86597735,
            -0.3586996,
            0.3219264,
        )),
        LevelEntity,
        Name::new("Solari_DirLight"),
    ));

    // ---------- 相机 ----------
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.04)),
            ..default()
        },
        Transform::from_translation(Vec3::new(6.0, 4.0, 8.0))
            .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        CameraController::default(),
        LevelEntity,
        Name::new("Solari_Camera"),
    ));

    // ---------- 提示文字 ----------
    commands.spawn((
        Text::new("光追演示关卡 · WASD移动 · 鼠标旋转 · Backspace释放光标 · 6返回"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        LevelEntity,
        Name::new("Solari_HelpText"),
    ));
}
