//! 光照调试模块 — 运行时可调节的光照参数
//!
//! F3 面板 → 资源标签 → 🔆 光照调试 折叠区域

use bevy::prelude::*;

/// 运行时可调节的光照参数（F3 面板控制）
#[derive(Resource, Clone)]
pub struct LightingDebug {
    // ── 环境光 ──
    pub ambient_brightness: f32,
    pub ambient_r: f32,
    pub ambient_g: f32,
    pub ambient_b: f32,

    // ── 方向光（太阳） ──
    pub sun_illuminance: f32,
    pub sun_r: f32,
    pub sun_g: f32,
    pub sun_b: f32,
    pub sun_rotation_x: f32,
    pub sun_rotation_y: f32,
    pub sun_shadows_enabled: bool,

    // ── 点光源（补光） ──
    pub fill_intensity: f32,
    pub fill_range: f32,
    pub fill_r: f32,
    pub fill_g: f32,
    pub fill_b: f32,
    pub fill_y: f32,
    pub fill_shadows_enabled: bool,

    // ── 手电筒 ──
    pub flashlight_intensity: f32,
    pub flashlight_range: f32,
    pub flashlight_outer_angle: f32,

    // ── 内部实体追踪（由 world.rs 设置，不用手动修改） ──
    #[doc(hidden)]
    pub sun_entity: Option<Entity>,
    #[doc(hidden)]
    pub fill_entity: Option<Entity>,

    // ── 测试预设 ──
    pub current_preset: LightingPreset,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LightingPreset {
    /// 当前自定义值
    Custom,
    /// 默认（三渲二优化）
    Default,
    /// 极暗 — 测试手电筒
    Dark,
    /// 极亮 — 测试过曝
    Bright,
    /// 黄昏 — 低角度光
    Sunset,
    /// 阴天 — 仅环境光
    Overcast,
}

impl LightingPreset {
    pub fn all() -> [LightingPreset; 6] {
        [
            LightingPreset::Custom,
            LightingPreset::Default,
            LightingPreset::Dark,
            LightingPreset::Bright,
            LightingPreset::Sunset,
            LightingPreset::Overcast,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            LightingPreset::Custom => "自定义",
            LightingPreset::Default => "默认",
            LightingPreset::Dark => "极暗",
            LightingPreset::Bright => "极亮",
            LightingPreset::Sunset => "黄昏",
            LightingPreset::Overcast => "阴天",
        }
    }
}

impl Default for LightingDebug {
    fn default() -> Self {
        Self {
            // 环境光（保持低值配合三渲二）
            ambient_brightness: 15.0,
            ambient_r: 0.04,
            ambient_g: 0.04,
            ambient_b: 0.06,

            // 方向光（暖白阳光）
            sun_illuminance: 8000.0,
            sun_r: 1.0,
            sun_g: 0.95,
            sun_b: 0.85,
            sun_rotation_x: -0.8,
            sun_rotation_y: 0.5,
            sun_shadows_enabled: true,

            // 点光源（冷色补光）
            fill_intensity: 500_000.0,
            fill_range: 200.0,
            fill_r: 0.6,
            fill_g: 0.7,
            fill_b: 0.9,
            fill_y: 5.0,
            fill_shadows_enabled: false,

            // 手电筒
            flashlight_intensity: 800_000.0,
            flashlight_range: 20.0,
            flashlight_outer_angle: 0.5,

            sun_entity: None,
            fill_entity: None,
            current_preset: LightingPreset::Default,
        }
    }
}

impl LightingDebug {
    /// 应用预设
    pub fn apply_preset(&mut self, preset: LightingPreset) {
        match preset {
            LightingPreset::Default => {
                let sun = self.sun_entity;
                let fill = self.fill_entity;
                *self = Self::default();
                self.sun_entity = sun;
                self.fill_entity = fill;
                self.current_preset = LightingPreset::Default;
            }
            LightingPreset::Dark => {
                self.ambient_brightness = 5.0;
                self.sun_illuminance = 2000.0;
                self.fill_intensity = 100_000.0;
                self.flashlight_intensity = 1_200_000.0;
                self.flashlight_range = 30.0;
                self.sun_shadows_enabled = true;
                self.current_preset = LightingPreset::Dark;
            }
            LightingPreset::Bright => {
                self.ambient_brightness = 50.0;
                self.sun_illuminance = 20000.0;
                self.fill_intensity = 800_000.0;
                self.current_preset = LightingPreset::Bright;
            }
            LightingPreset::Sunset => {
                self.ambient_brightness = 10.0;
                self.sun_illuminance = 5000.0;
                self.sun_rotation_x = -1.3;
                self.sun_rotation_y = 0.8;
                self.sun_r = 1.0;
                self.sun_g = 0.5;
                self.sun_b = 0.2;
                self.sun_shadows_enabled = true;
                self.fill_intensity = 200_000.0;
                self.fill_r = 0.8;
                self.fill_g = 0.4;
                self.fill_b = 0.2;
                self.current_preset = LightingPreset::Sunset;
            }
            LightingPreset::Overcast => {
                self.ambient_brightness = 30.0;
                self.ambient_r = 0.06;
                self.ambient_g = 0.06;
                self.ambient_b = 0.07;
                self.sun_illuminance = 1000.0;
                self.fill_intensity = 300_000.0;
                self.fill_range = 300.0;
                self.sun_shadows_enabled = false;
                self.current_preset = LightingPreset::Overcast;
            }
            LightingPreset::Custom => {
                self.current_preset = LightingPreset::Custom;
            }
        }
    }
}

/// 将 LightingDebug 资源同步到场景中的光照实体（使用 Commands 避免 Transform 查询冲突）
pub fn sync_lighting_to_world(
    debug: Res<LightingDebug>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut commands: Commands,
    mut flashlight_q: Query<&mut SpotLight, With<crate::player::Flashlight>>,
) {
    if !debug.is_changed() {
        return;
    }

    // ── 环境光 ──
    ambient.brightness = debug.ambient_brightness;
    ambient.color = Color::srgb(debug.ambient_r, debug.ambient_g, debug.ambient_b);

    // ── 方向光 ──
    if let Some(entity) = debug.sun_entity {
        commands.entity(entity).insert((
            DirectionalLight {
                color: Color::srgb(debug.sun_r, debug.sun_g, debug.sun_b),
                illuminance: debug.sun_illuminance,
                shadows_enabled: debug.sun_shadows_enabled,
                ..default()
            },
            Transform::from_rotation(
                Quat::from_rotation_x(debug.sun_rotation_x)
                    .mul_quat(Quat::from_rotation_y(debug.sun_rotation_y)),
            ),
        ));
    }

    // ── 补光 ──
    if let Some(entity) = debug.fill_entity {
        commands.entity(entity).insert((
            PointLight {
                color: Color::srgb(debug.fill_r, debug.fill_g, debug.fill_b),
                intensity: debug.fill_intensity,
                range: debug.fill_range,
                shadows_enabled: debug.fill_shadows_enabled,
                ..default()
            },
            Transform::from_xyz(0.0, debug.fill_y, 0.0),
        ));
    }

    // ── 手电筒（只改 SpotLight，不涉及 Transform，无冲突） ──
    for mut flashlight in flashlight_q.iter_mut() {
        flashlight.intensity = debug.flashlight_intensity;
        flashlight.range = debug.flashlight_range;
        flashlight.outer_angle = debug.flashlight_outer_angle;
    }
}
