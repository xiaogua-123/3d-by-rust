//! 塔防数据定义：枚举、组件、资源
//!
//! 定义塔防模式的核心类型：`TurretType`、`TdEnemyType`、`WavePhase` 枚举，
//! `Turret`、`Projectile`、`TdEnemy`、`DefenseCore`、`BuildSpot` 等组件，
//! 以及 `TdGold`、`TdWaveConfig`、`TdWaveState` 等资源。

use bevy::prelude::*;

// ═══ 枚举 ═══

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurretType {
    Basic,
    Rapid,
    Heavy,
}

impl TurretType {
    pub fn id(self) -> &'static str {
        match self {
            TurretType::Basic => "basic",
            TurretType::Rapid => "rapid",
            TurretType::Heavy => "heavy",
        }
    }

    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            TurretType::Basic => "基础炮台",
            TurretType::Rapid => "速射炮台",
            TurretType::Heavy => "重型炮台",
        }
    }

    pub fn cost(self) -> u32 {
        match self {
            TurretType::Basic => 50,
            TurretType::Rapid => 100,
            TurretType::Heavy => 150,
        }
    }

    pub fn range(self) -> f32 {
        match self {
            TurretType::Basic => 5.0,
            TurretType::Rapid => 4.0,
            TurretType::Heavy => 6.0,
        }
    }

    pub fn damage(self) -> f32 {
        match self {
            TurretType::Basic => 10.0,
            TurretType::Rapid => 5.0,
            TurretType::Heavy => 30.0,
        }
    }

    pub fn fire_rate(self) -> f32 {
        match self {
            TurretType::Basic => 1.0,
            TurretType::Rapid => 0.3,
            TurretType::Heavy => 2.0,
        }
    }

    pub fn color(self) -> Color {
        match self {
            TurretType::Basic => Color::srgb(0.3, 0.7, 0.9),
            TurretType::Rapid => Color::srgb(0.9, 0.7, 0.2),
            TurretType::Heavy => Color::srgb(0.9, 0.2, 0.2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TdEnemyType {
    Basic,
    Fast,
    Tank,
}

impl TdEnemyType {
    pub fn id(self) -> &'static str {
        match self {
            TdEnemyType::Basic => "basic",
            TdEnemyType::Fast => "fast",
            TdEnemyType::Tank => "tank",
        }
    }

    pub fn health(self) -> f32 {
        match self {
            TdEnemyType::Basic => 30.0,
            TdEnemyType::Fast => 15.0,
            TdEnemyType::Tank => 80.0,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            TdEnemyType::Basic => 1.5,
            TdEnemyType::Fast => 3.0,
            TdEnemyType::Tank => 1.0,
        }
    }

    pub fn gold_reward(self) -> u32 {
        match self {
            TdEnemyType::Basic => 10,
            TdEnemyType::Fast => 15,
            TdEnemyType::Tank => 25,
        }
    }

    pub fn damage(self) -> f32 {
        match self {
            TdEnemyType::Basic => 5.0,
            TdEnemyType::Fast => 3.0,
            TdEnemyType::Tank => 10.0,
        }
    }

    pub fn color(self) -> Color {
        match self {
            TdEnemyType::Basic => Color::srgb(0.8, 0.3, 0.3),
            TdEnemyType::Fast => Color::srgb(0.9, 0.5, 0.1),
            TdEnemyType::Tank => Color::srgb(0.5, 0.1, 0.5),
        }
    }

    pub fn size(self) -> f32 {
        match self {
            TdEnemyType::Basic => 0.5,
            TdEnemyType::Fast => 0.35,
            TdEnemyType::Tank => 0.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavePhase {
    Waiting,
    Spawning,
    Active,
    Complete,
}

// ═══ 组件 ═══

#[derive(Component)]
pub struct Turret {
    pub turret_type: TurretType,
    pub range: f32,
    pub damage: f32,
    pub fire_timer: Timer,
    pub target: Option<Entity>,
    pub barrel_y: f32,
}

#[derive(Component)]
pub struct Projectile {
    pub damage: f32,
    pub speed: f32,
    pub target_pos: Vec3,
    pub lifetime: Timer,
    #[allow(dead_code)]
    pub color: Color,
}

#[derive(Component)]
pub struct TdEnemy {
    pub enemy_type: TdEnemyType,
    pub gold_reward: u32,
}

#[derive(Component)]
pub struct DefenseCore {
    pub max_health: f32,
    pub current_health: f32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct SpawnPoint {
    pub direction: Vec3,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct BuildSpot;

// ═══ 资源 ═══

#[derive(Resource, Default)]
pub struct TdGold(pub u32);

#[derive(Resource)]
pub struct TdWaveConfig {
    pub max_waves: u32,
    pub enemies_per_wave_base: u32,
    pub enemies_per_wave_growth: u32,
    pub spawn_interval: f32,
    pub wave_cooldown: f32,
    pub starting_gold: u32,
}

impl Default for TdWaveConfig {
    fn default() -> Self {
        Self {
            max_waves: 10,
            enemies_per_wave_base: 5,
            enemies_per_wave_growth: 2,
            spawn_interval: 0.8,
            wave_cooldown: 8.0,
            starting_gold: 300,
        }
    }
}

#[derive(Resource)]
pub struct TdWaveState {
    pub current_wave: u32,
    pub phase: WavePhase,
    pub enemies_to_spawn: u32,
    pub enemies_alive: u32,
    pub spawn_timer: Timer,
    pub wave_timer: Timer,
}

impl Default for TdWaveState {
    fn default() -> Self {
        Self {
            current_wave: 0,
            phase: WavePhase::Waiting,
            enemies_to_spawn: 0,
            enemies_alive: 0,
            spawn_timer: Timer::from_seconds(0.8, TimerMode::Repeating),
            wave_timer: Timer::from_seconds(8.0, TimerMode::Once),
        }
    }
}
