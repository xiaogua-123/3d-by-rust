// ═══════════════════════════════════════════
// 统一战斗系统：共享组件与工具函数
// ═══════════════════════════════════════════

use bevy::prelude::*;

// ═══ 共享组件 ═══

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            max: value,
        }
    }

    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            self.current / self.max
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AttackDamage(pub f32);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MoveSpeed(pub f32);

// ═══ 共享工具函数 ═══

/// 向目标移动实体，返回 true 表示仍在移动（距离 > stop_distance）
pub fn move_toward_target(
    transform: &mut Transform,
    target: Vec3,
    speed: f32,
    delta_secs: f32,
    stop_distance: f32,
) -> bool {
    let dir = target - transform.translation;
    let dist = dir.length();
    if dist < stop_distance {
        return false;
    }
    let dir_norm = dir / dist;
    transform.translation += dir_norm * speed * delta_secs;
    transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir_norm);
    true
}

// ═══ 插件 ═══

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Health>()
            .register_type::<AttackDamage>()
            .register_type::<MoveSpeed>();
    }
}
