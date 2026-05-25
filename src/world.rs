use bevy::prelude::*;
use crate::debug_lighting::LightingDebug;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world);
    }
}

fn setup_world(
    mut commands: Commands,
    mut lighting: ResMut<LightingDebug>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let ld = &mut *lighting;

    // 主方向光
    let sun_id = commands.spawn((
        DirectionalLight {
            color: Color::srgb(ld.sun_r, ld.sun_g, ld.sun_b),
            illuminance: ld.sun_illuminance,
            shadows_enabled: ld.sun_shadows_enabled,
            ..default()
        },
        Transform::from_rotation(
            Quat::from_rotation_x(ld.sun_rotation_x)
                .mul_quat(Quat::from_rotation_y(ld.sun_rotation_y)),
        ),
        Name::new("SunLight"),
    )).id();
    ld.sun_entity = Some(sun_id);

    // 辅助点光源
    let fill_id = commands.spawn((
        PointLight {
            color: Color::srgb(ld.fill_r, ld.fill_g, ld.fill_b),
            intensity: ld.fill_intensity,
            range: ld.fill_range,
            shadows_enabled: ld.fill_shadows_enabled,
            ..default()
        },
        Transform::from_xyz(0.0, ld.fill_y, 0.0),
        Name::new("FillLight"),
    )).id();
    ld.fill_entity = Some(fill_id);

    // 环境光
    ambient.brightness = ld.ambient_brightness;
    ambient.color = Color::srgb(ld.ambient_r, ld.ambient_g, ld.ambient_b);
}
