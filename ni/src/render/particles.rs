//! 粒子系统（Hanabi）
//!
//! 环境粒子特效：尘土、落叶、雨滴、火焰柱。`ParticleConfig` 控制开关，
//! 粒子发射器跟随玩家移动，按场景主题自动切换。

use bevy::prelude::*;
use bevy_hanabi::{
    AccelModifier, AlphaMode, Attribute, ColorBlendMask, ColorBlendMode,
    ColorOverLifetimeModifier, EffectAsset, Gradient, HanabiPlugin, LinearDragModifier,
    Module, OrientMode, OrientModifier, ParticleEffect, SetAttributeModifier,
    SetPositionCircleModifier, SetVelocityCircleModifier, SetVelocitySphereModifier,
    ShapeDimension, SimulationSpace, SizeOverLifetimeModifier, SpawnerSettings,
};

// 导入游戏状态模块
use crate::game_state::GamePhase;
// 导入玩家模块
use crate::player::Player;

// ═══════════════════════════════════════════
// 粒子配置资源：存储所有粒子效果的开关状态
// ═══════════════════════════════════════════

// 粒子配置资源，实现反射便于编辑器调试
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ParticleConfig {
    pub dust_enabled: bool,    // 灰尘粒子开关
    pub leaves_enabled: bool,  // 落叶粒子开关
    pub rain_enabled: bool,    // 雨滴粒子开关
}

// 默认配置：开启灰尘、落叶，关闭雨滴
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
// 粒子发射器标记组件：用于区分不同类型的粒子发射器
// ═══════════════════════════════════════════

/// 灰尘粒子发射器标记组件
#[derive(Component)]
struct DustEmitter;

/// 落叶粒子发射器标记组件
#[derive(Component)]
struct LeafEmitter;

/// 雨滴粒子发射器标记组件
#[derive(Component)]
struct RainEmitter;

// ═══════════════════════════════════════════
// 粒子插件：统一注册粒子系统相关功能
// ═══════════════════════════════════════════

/// 粒子系统总插件
pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)  // 添加Hanabi粒子插件
            .init_resource::<ParticleConfig>()  // 初始化粒子配置资源
            .register_type::<ParticleConfig>()  // 注册配置类型用于反射
            .add_systems(Startup, setup_particle_effects)  // 启动时生成粒子效果
            .add_systems(
                Update,
                update_particle_positions  // 每帧更新粒子发射器位置
                    .run_if(in_state(GamePhase::Playing))  // 仅在游戏进行中运行
                    .run_if(resource_exists::<ParticleConfig>),  // 配置存在时运行
            );
    }
}

// ═══════════════════════════════════════════
// 灰尘效果：创建地面灰尘漂浮粒子
// ═══════════════════════════════════════════

/// 创建灰尘粒子效果资产
fn create_dust_effect() -> EffectAsset {
    // 粒子效果模块
    let mut module = Module::default();

    // 定义粒子效果常量参数
    let center = module.lit(Vec3::new(0.0, 0.0, 0.0));  // 生成中心
    let axis = module.lit(Vec3::Y);                     // 圆形生成轴
    let zero = module.lit(Vec3::ZERO);                  // 零向量
    let radius = module.lit(3.0_f32);                   // 生成半径
    let lifetime = module.lit(3.0_f32);                 // 粒子生命周期
    let speed = module.lit(1.0_f32);                    // 粒子初速度
    let wind = module.lit(Vec3::new(0.0, 0.3, 0.0));    // 风力加速度
    let drag = module.lit(0.8_f32);                     // 空气阻力

    // 生成器设置：每秒生成50个粒子
    let spawner = SpawnerSettings::rate(50.0_f32.into());

    // 构建灰尘粒子效果
    EffectAsset::new(128, spawner, module)
        .with_name("Dust")  // 效果名称
        .with_simulation_space(SimulationSpace::Local)  // 局部坐标系模拟
        // 初始化：圆形区域生成位置
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        // 初始化：圆形方向初速度
        .init(SetVelocityCircleModifier {
            center: zero,
            axis,
            speed,
        })
        // 初始化：设置粒子生命周期
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        // 更新：施加风力加速度
        .update(AccelModifier::new(wind))
        // 更新：施加线性阻力
        .update(LinearDragModifier::new(drag))
        // 渲染：生命周期内大小变化（逐渐消失）
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::linear(Vec3::splat(0.06), Vec3::ZERO),
            screen_space_size: false,
        })
        // 渲染：生命周期内颜色渐变（半透明土黄色→完全透明）
        .render(ColorOverLifetimeModifier {
            gradient: Gradient::linear(
                Vec4::new(0.6, 0.55, 0.45, 0.25),
                Vec4::new(0.6, 0.55, 0.45, 0.0),
            ),
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        // 渲染：始终面向相机平面
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
}

// ═══════════════════════════════════════════
// 落叶效果：创建空中缓慢飘落的树叶
// ═══════════════════════════════════════════

/// 创建落叶粒子效果资产
fn create_leaf_effect() -> EffectAsset {
    let mut module = Module::default();

    // 落叶粒子参数
    let center = module.lit(Vec3::new(0.0, 7.0, 0.0));   // 高空生成
    let axis = module.lit(Vec3::Y);
    let zero = module.lit(Vec3::ZERO);
    let radius = module.lit(8.0_f32);                    // 大范围生成
    let lifetime = module.lit(6.0_f32);                  // 长生命周期
    let speed = module.lit(1.5_f32);
    let wind = module.lit(Vec3::new(1.5, -1.0, 0.8));    // 斜向风力
    let drag = module.lit(0.3_f32);                      // 低阻力

    // 低生成频率
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
        // 渲染：固定大小
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::constant(Vec3::new(0.12, 0.08, 1.0)),
            screen_space_size: false,
        })
        // 渲染：三段颜色渐变（黄→橙褐→透明）
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
// 雨滴效果：创建快速下落的雨水粒子
// ═══════════════════════════════════════════

/// 创建雨滴粒子效果资产
fn create_rain_effect() -> EffectAsset {
    let mut module = Module::default();

    // 雨滴粒子参数
    let center = module.lit(Vec3::new(0.0, 10.0, 0.0));  // 极高空生成
    let axis = module.lit(Vec3::Y);
    let zero = module.lit(Vec3::ZERO);
    let radius = module.lit(10.0_f32);                   // 超大范围
    let lifetime = module.lit(1.0_f32);                 // 短生命周期
    let speed = module.lit(20.0_f32);                   // 极高速度
    let gravity = module.lit(Vec3::new(0.0, -5.0, 0.0));// 重力加速度
    let drag = module.lit(0.1_f32);                     // 极低阻力

    // 超高生成频率
    let spawner = SpawnerSettings::rate(256.0_f32.into());

    EffectAsset::new(256, spawner, module)
        .with_name("Rain")
        .with_simulation_space(SimulationSpace::Local)
        .with_alpha_mode(AlphaMode::Add)  // 加法混合（发光效果）
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        // 初始化：球形方向初速度
        .init(SetVelocitySphereModifier {
            center: zero,
            speed,
        })
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        .update(AccelModifier::new(gravity))
        .update(LinearDragModifier::new(drag))
        // 渲染：细长雨滴形状
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::constant(Vec3::new(0.015, 0.12, 1.0)),
            screen_space_size: false,
        })
        // 渲染：浅蓝色→透明
        .render(ColorOverLifetimeModifier {
            gradient: Gradient::linear(
                Vec4::new(0.6, 0.7, 0.9, 0.35),
                Vec4::new(0.5, 0.6, 0.8, 0.0),
            ),
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        // 渲染：沿速度方向朝向（模拟雨线）
        .render(OrientModifier::new(OrientMode::AlongVelocity))
}

// ═══════════════════════════════════════════
// 初始化：生成所有粒子发射器实体
// ═══════════════════════════════════════════

/// 启动时创建粒子效果并生成发射器实体
fn setup_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    config: Res<ParticleConfig>,
) {
    // 加载三种粒子效果资源
    let dust_handle = effects.add(create_dust_effect());
    let leaf_handle = effects.add(create_leaf_effect());
    let rain_handle = effects.add(create_rain_effect());

    // 根据配置生成灰尘发射器
    if config.dust_enabled {
        commands.spawn((
            ParticleEffect::new(dust_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            DustEmitter,
        ));
    }

    // 根据配置生成落叶发射器
    if config.leaves_enabled {
        commands.spawn((
            ParticleEffect::new(leaf_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            LeafEmitter,
        ));
    }

    // 根据配置生成雨滴发射器
    if config.rain_enabled {
        commands.spawn((
            ParticleEffect::new(rain_handle),
            Transform::from_xyz(0.0, 0.0, 0.0),
            RainEmitter,
        ));
    }
}

// ═══════════════════════════════════════════
// 运行时：粒子发射器跟随玩家位置移动
// ═══════════════════════════════════════════

/// 每帧更新粒子发射器位置，使其跟随玩家
#[allow(clippy::type_complexity)]
fn update_particle_positions(
    player_query: Query<&GlobalTransform, With<Player>>,  // 获取玩家全局坐标
    mut emitter_query: Query<
        &mut Transform,
        Or<(With<DustEmitter>, With<LeafEmitter>, With<RainEmitter>)>,  // 匹配所有粒子发射器
    >,
) {
    // 获取玩家位置，失败则直接返回
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation();

    // 所有发射器都移动到玩家位置（轻微抬高）
    for mut transform in emitter_query.iter_mut() {
        *transform = Transform::from_xyz(player_pos.x, player_pos.y + 0.1, player_pos.z);
    }
}

// ═══════════════════════════════════════════
// 火焰柱效果：粒子测试关卡专用
// ═══════════════════════════════════════════

/// 火焰柱粒子发射器标记
#[derive(Component)]
pub struct FirePillarEmitter;

/// 创建火焰柱效果
pub fn create_fire_pillar_effect() -> EffectAsset {
    let mut module = Module::default();

    let center = module.lit(Vec3::ZERO);
    let axis = module.lit(Vec3::Y);
    let radius = module.lit(0.4_f32);
    let lifetime = module.lit(2.0_f32);
    let speed = module.lit(2.5_f32);
    let gravity = module.lit(Vec3::new(0.0, 2.0, 0.0));
    let vel_center = module.lit(Vec3::ZERO);

    let spawner = SpawnerSettings::rate(40.0_f32.into());

    EffectAsset::new(256, spawner, module)
        .with_name("FirePillar")
        .with_simulation_space(SimulationSpace::Local)
        .init(SetPositionCircleModifier {
            center,
            axis,
            radius,
            dimension: ShapeDimension::Volume,
        })
        .init(SetVelocityCircleModifier {
            center: vel_center,
            axis,
            speed,
        })
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
        .update(AccelModifier::new(gravity))
        .render(SizeOverLifetimeModifier {
            gradient: Gradient::linear(Vec3::splat(0.3), Vec3::ZERO),
            screen_space_size: false,
        })
        .render(ColorOverLifetimeModifier {
            gradient: Gradient::linear(
                Vec4::new(1.0, 0.5, 0.0, 1.0),
                Vec4::new(0.8, 0.1, 0.0, 0.0),
            ),
            blend: ColorBlendMode::Overwrite,
            mask: ColorBlendMask::RGBA,
        })
        .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
}

/// 在粒子测试关卡中生成火焰柱特效
pub fn spawn_particle_test_fx(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let fire_handle = effects.add(create_fire_pillar_effect());

    // 在场景中央周围生成一圈小火柱
    let count = 6;
    for i in 0..count {
        let angle = i as f32 * std::f32::consts::TAU / count as f32;
        commands.spawn((
            ParticleEffect::new(fire_handle.clone()),
            Transform::from_xyz(angle.cos() * 3.0, 0.5, angle.sin() * 3.0),
            FirePillarEmitter,
            crate::level::LevelEntity,
        ));
    }

    // 中心大火柱（使用正确的 Module 生命周期模式）
    let mut module = Module::default();
    let center = module.lit(Vec3::ZERO);
    let axis = module.lit(Vec3::Y);
    let radius = module.lit(0.8_f32);
    let lifetime = module.lit(3.0_f32);
    let speed = module.lit(3.0_f32);
    let gravity = module.lit(Vec3::new(0.0, 2.5, 0.0));
    let vel_center = module.lit(Vec3::ZERO);

    let big_fire = effects.add(
        EffectAsset::new(512, SpawnerSettings::rate(60.0_f32.into()), module)
            .with_name("FirePillar_Big")
            .with_simulation_space(SimulationSpace::Local)
            .init(SetPositionCircleModifier { center, axis, radius, dimension: ShapeDimension::Volume })
            .init(SetVelocityCircleModifier { center: vel_center, axis, speed })
            .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
            .update(AccelModifier::new(gravity))
            .render(SizeOverLifetimeModifier {
                gradient: Gradient::linear(Vec3::splat(0.5), Vec3::ZERO),
                screen_space_size: false,
            })
            .render(ColorOverLifetimeModifier {
                gradient: Gradient::linear(
                    Vec4::new(1.0, 0.6, 0.0, 1.0),
                    Vec4::new(0.8, 0.0, 0.0, 0.0),
                ),
                blend: ColorBlendMode::Overwrite,
                mask: ColorBlendMask::RGBA,
            })
            .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane)),
    );
    commands.spawn((
        ParticleEffect::new(big_fire),
        Transform::from_xyz(0.0, 0.5, 0.0),
        FirePillarEmitter,
        crate::level::LevelEntity,
    ));
}