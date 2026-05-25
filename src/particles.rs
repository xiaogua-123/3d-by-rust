use bevy::prelude::*;
use bevy_hanabi::{
    AccelModifier, AlphaMode, Attribute, ColorBlendMask, ColorBlendMode,
    ColorOverLifetimeModifier, EffectAsset, Gradient, HanabiPlugin, LinearDragModifier,
    Module, OrientMode, OrientModifier, ParticleEffect, SetAttributeModifier,
    SetPositionCircleModifier, SetVelocityCircleModifier, SetVelocitySphereModifier,
    ShapeDimension, SimulationSpace, SizeOverLifetimeModifier, SpawnerSettings,
};

use crate::game_state::GamePhase;
use crate::player::Player;

// ═══════════════════════════════════════════
// 粒子配置
// ═══════════════════════════════════════════

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ParticleConfig {
    pub dust_enabled: bool,
    pub leaves_enabled: bool,
    pub rain_enabled: bool,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            dust_enabled: true,
            leaves_enabled: true,
            rain_enabled: false,
        }
    }
}

// ═══════════════════════════════════════════
// 发射器标记组件
// ═══════════════════════════════════════════

#[derive(Component)]
struct DustEmitter;

#[derive(Component)]
struct LeafEmitter;

#[derive(Component)]
struct RainEmitter;

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .init_resource::<ParticleConfig>()
            .register_type::<ParticleConfig>()
            .add_systems(Startup, setup_particle_effects)
            .add_systems(
                Update,
                update_particle_positions
                    .run_if(in_state(GamePhase::Playing))
                    .run_if(resource_exists::<ParticleConfig>),
            );
    }
}

// ═══════════════════════════════════════════
// 灰尘效果
// ═══════════════════════════════════════════

fn create_dust_effect() -> EffectAsset {
    let mut module = Module::default();

    let center = module.lit(Vec3::new(0.0, 0.0, 0.0));
    let axis = module.lit(Vec3::Y);
    let zero = module.lit(Vec3::ZERO);
    let radius = module.lit(3.0_f32);
    let lifetime = module.lit(3.0_f32);
    let speed = module.lit(1.0_f32);
    let wind = module.lit(Vec3::new(0.0, 0.3, 0.0));
    let drag = module.lit(0.8_f32);

    let spawner = SpawnerSettings::rate(50.0_f32.into());

    EffectAsset::new(128, spawner, module)
        .with_name("Dust")
        .with_simulation_space(SimulationSpace::Local)
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        .init(SetVelocityCircleModifier {
            center: zero,
            axis,
            speed,
        })
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        .update(AccelModifier::new(wind))
        .update(LinearDragModifier::new(drag))
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::linear(Vec3::splat(0.06), Vec3::ZERO),
            screen_space_size: false,
        })
        .render(ColorOverLifetimeModifier {
            gradient: Gradient::linear(
                Vec4::new(0.6, 0.55, 0.45, 0.25),
                Vec4::new(0.6, 0.55, 0.45, 0.0),
            ),
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
}

// ═══════════════════════════════════════════
// 落叶效果
// ═══════════════════════════════════════════

fn create_leaf_effect() -> EffectAsset {
    let mut module = Module::default();

    let center = module.lit(Vec3::new(0.0, 7.0, 0.0));
    let axis = module.lit(Vec3::Y);
    let zero = module.lit(Vec3::ZERO);
    let radius = module.lit(8.0_f32);
    let lifetime = module.lit(6.0_f32);
    let speed = module.lit(1.5_f32);
    let wind = module.lit(Vec3::new(1.5, -1.0, 0.8));
    let drag = module.lit(0.3_f32);

    let spawner = SpawnerSettings::rate(12.0_f32.into());

    EffectAsset::new(64, spawner, module)
        .with_name("Falling Leaves")
        .with_simulation_space(SimulationSpace::Local)
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        .init(SetVelocityCircleModifier {
            center: zero,
            axis,
            speed,
        })
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        .update(AccelModifier::new(wind))
        .update(LinearDragModifier::new(drag))
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::constant(Vec3::new(0.12, 0.08, 1.0)),
            screen_space_size: false,
        })
        .render(ColorOverLifetimeModifier {
            gradient: {
                let mut g = Gradient::new();
                g.add_key(0.0, Vec4::new(0.9, 0.7, 0.3, 0.7));
                g.add_key(0.7, Vec4::new(0.7, 0.4, 0.15, 0.5));
                g.add_key(1.0, Vec4::new(0.4, 0.2, 0.05, 0.0));
                g
            },
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
}

// ═══════════════════════════════════════════
// 雨滴效果
// ═══════════════════════════════════════════

fn create_rain_effect() -> EffectAsset {
    let mut module = Module::default();

    let center = module.lit(Vec3::new(0.0, 10.0, 0.0));
    let axis = module.lit(Vec3::Y);
    let zero = module.lit(Vec3::ZERO);
    let radius = module.lit(10.0_f32);
    let lifetime = module.lit(1.0_f32);
    let speed = module.lit(20.0_f32);
    let gravity = module.lit(Vec3::new(0.0, -5.0, 0.0));
    let drag = module.lit(0.1_f32);

    let spawner = SpawnerSettings::rate(256.0_f32.into());

    EffectAsset::new(256, spawner, module)
        .with_name("Rain")
        .with_simulation_space(SimulationSpace::Local)
        .with_alpha_mode(AlphaMode::Add)
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        .init(SetVelocitySphereModifier {
            center: zero,
            speed,
        })
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        .update(AccelModifier::new(gravity))
        .update(LinearDragModifier::new(drag))
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::constant(Vec3::new(0.015, 0.12, 1.0)),
            screen_space_size: false,
        })
        .render(ColorOverLifetimeModifier {
            gradient: Gradient::linear(
                Vec4::new(0.6, 0.7, 0.9, 0.35),
                Vec4::new(0.5, 0.6, 0.8, 0.0),
            ),
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(OrientModifier::new(OrientMode::AlongVelocity))
}

// ═══════════════════════════════════════════
// 生成粒子效果实体
// ═══════════════════════════════════════════

fn setup_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    config: Res<ParticleConfig>,
) {
    let dust_handle = effects.add(create_dust_effect());
    let leaf_handle = effects.add(create_leaf_effect());
    let rain_handle = effects.add(create_rain_effect());

    // 灰尘发射器
    if config.dust_enabled {
        commands.spawn((
            ParticleEffect::new(dust_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            DustEmitter,
        ));
    }

    // 落叶发射器
    if config.leaves_enabled {
        commands.spawn((
            ParticleEffect::new(leaf_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            LeafEmitter,
        ));
    }

    // 雨滴发射器
    if config.rain_enabled {
        commands.spawn((
            ParticleEffect::new(rain_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            RainEmitter,
        ));
    }
}

// ═══════════════════════════════════════════
// 粒子发射器跟随玩家
// ═══════════════════════════════════════════

fn update_particle_positions(
    player_query: Query<&GlobalTransform, With<Player>>,
    mut emitter_query: Query<
        &mut Transform,
        Or<(With<DustEmitter>, With<LeafEmitter>, With<RainEmitter>)>,
    >,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation();

    for mut transform in emitter_query.iter_mut() {
        *transform = Transform::from_xyz(player_pos.x, player_pos.y + 0.1, player_pos.z);
    }
}
