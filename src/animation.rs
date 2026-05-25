use bevy::prelude::*;

use crate::game_state::GamePhase;
use crate::player::{Model, MoveIntent, Player, Velocity};

// ═══════════════════════════════════════════
// 组件
// ═══════════════════════════════════════════

#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub enum AnimationState {
    #[default]
    Idle,
    Walking,
    Running,
    Jumping,
    Falling,
}

impl AnimationState {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            AnimationState::Idle => "待机",
            AnimationState::Walking => "走路",
            AnimationState::Running => "跑步",
            AnimationState::Jumping => "跳跃",
            AnimationState::Falling => "下落",
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationController {
    pub state: AnimationState,
    pub bob_timer: f32,
    pub transition_timer: f32,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            state: AnimationState::Idle,
            bob_timer: 0.0,
            transition_timer: 0.0,
        }
    }
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AnimationController>()
            .register_type::<AnimationState>()
            .add_systems(
                Update,
                (
                    update_animation_state.run_if(in_state(GamePhase::Playing)),
                    apply_animation.run_if(in_state(GamePhase::Playing)),
                ),
            );
    }
}

// ═══════════════════════════════════════════
// 动画状态机
// ═══════════════════════════════════════════

fn update_animation_state(
    player_q: Query<(&Velocity, &MoveIntent), With<Player>>,
    mut anim_q: Query<&mut AnimationController, With<Model>>,
) {
    let Ok((velocity, intent)) = player_q.single() else { return };
    let Ok(mut controller) = anim_q.single_mut() else { return };

    let new_state = if velocity.y > 1.0 {
        AnimationState::Jumping
    } else if velocity.y < -1.0 {
        AnimationState::Falling
    } else if intent.world_direction != Vec3::ZERO {
        // 检查是否是跑步（Shift 加速，但当前没有 Shift 检测，简化处理）
        AnimationState::Walking
    } else {
        AnimationState::Idle
    };

    if new_state != controller.state {
        controller.state = new_state;
        controller.transition_timer = 0.0;
    }
}

// ═══════════════════════════════════════════
// 动画效果应用（程序化）
// ═══════════════════════════════════════════

fn apply_animation(
    time: Res<Time>,
    mut anim_time: Local<f32>,
    mut model_q: Query<
        (&AnimationController, &mut Transform),
        (With<Model>, Without<Player>),
    >,
) {
    let Ok((controller, mut transform)) = model_q.single_mut() else { return };

    *anim_time += time.delta_secs();
    let t = *anim_time;

    let (bob_speed, bob_amplitude) = match controller.state {
        AnimationState::Idle => (1.5, 0.05),
        AnimationState::Walking => (8.0, 0.10),
        AnimationState::Running => (12.0, 0.15),
        AnimationState::Jumping => (0.0, 0.06),
        AnimationState::Falling => (0.0, 0.03),
    };

    let bob = (t * bob_speed).sin() * bob_amplitude;

    // 只处理 Y 轴浮动，旋转由 apply_player_rotation 处理
    let base_y = -0.3; // Model 的基础 Y 偏移（眼睛高度以下）
    transform.translation.y = base_y + bob;
}
