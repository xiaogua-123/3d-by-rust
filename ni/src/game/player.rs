//! 玩家系统
//!
//! 定义玩家相关的组件（`Player`、`Speed`、`Velocity`、`Flashlight`）、
//! 资源（`PlayerSettings`）和系统（输入处理、物理移动、旋转、手电筒循环）。
//! 使用新旧两套碰撞系统实现地面检测和水平推出。

use bevy::prelude::*;
use bevy::audio::Volume;
use std::time::Duration;

use crate::camera::LookState;
use crate::collision::{find_ground_y, push_out_horizontal};
use crate::colliders::{Collider, CollisionMask, CollisionResponse};
use crate::config::GameplayConfig;
use crate::entity_db::GlbCache;
use crate::grid::GameGridResource;
use crate::level::ResetPlayerEvent;
use crate::world_label::WorldLabel;
use crate::time_recorder::{PositionHistory, RewindTracked, TimedPosition, TrackedEntityKind};

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerSettings {
    pub move_speed: f32,
    pub sprint_speed: f32,
    pub gravity: f32,
    pub jump_force: f32,
    pub rotation_speed: f32,
    pub mouse_sensitivity: f32,
    pub flashlight_intensity: f32,
    pub flashlight_range: f32,
    pub flashlight_color: Color,
    pub coyote_time: f32,
    pub jump_buffer_time: f32,
    /// 模型在 GLB 中的默认朝向（Z 正方向为 Blender 默认）
    pub model_forward: ModelForward,
}

/// 模型默认朝向 —— 对应 GLB 文件中模型正面指向的方向
#[derive(Clone, Copy, PartialEq, Debug, Reflect)]
pub enum ModelForward {
    NegZ,   // Blender 默认（-Z 为前），适合大多数 glTF
    PosZ,   // 如果模型正面朝向 +Z
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            move_speed: 2.5,
            sprint_speed: 5.0,
            gravity: -15.0,
            jump_force: 8.0,
            rotation_speed: std::f32::consts::PI * 4.0,
            mouse_sensitivity: 0.002,
            flashlight_intensity: 800_000.0,
            flashlight_range: 20.0,
            flashlight_color: Color::srgb(1.0, 0.95, 0.8),
            coyote_time: 0.08,
            jump_buffer_time: 0.1,
            model_forward: ModelForward::NegZ,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Speed(pub f32);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Velocity {
    pub y: f32,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct RotationState {
    pub target_direction: Vec3,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Flashlight;

/// 手电筒模式
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum FlashlightMode {
    /// 正常暖色光
    Normal,
    /// UV 冷色窄光束——可探测隐藏痕迹
    UV,
    /// 关闭
    Off,
}

impl FlashlightMode {
    /// 循环到下一个模式
    pub fn cycle(self) -> Self {
        match self {
            FlashlightMode::Normal => FlashlightMode::UV,
            FlashlightMode::UV => FlashlightMode::Off,
            FlashlightMode::Off => FlashlightMode::Normal,
        }
    }
}

/// 手电筒运行时状态
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct FlashlightState {
    pub mode: FlashlightMode,
}

impl Default for FlashlightState {
    fn default() -> Self {
        Self {
            mode: FlashlightMode::Normal,
        }
    }
}

/// Marker on the child entity that holds the 3D model (rotated to face movement direction).
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Model;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct CoyoteTimer {
    pub timer: Timer,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct JumpBuffer {
    pub timer: Timer,
}

/// Separates player input intent from physics execution.
/// Written by `player_input`, consumed by `player_physics`.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct MoveIntent {
    pub world_direction: Vec3,
    pub is_sprinting: bool,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Player>()
            .register_type::<Speed>()
            .register_type::<Velocity>()
            .register_type::<RotationState>()
            .register_type::<Flashlight>()
            .register_type::<FlashlightMode>()
            .register_type::<FlashlightState>()
            .register_type::<Model>()
            .register_type::<CoyoteTimer>()
            .register_type::<JumpBuffer>()
            .register_type::<MoveIntent>()
            .register_type::<PlayerSettings>()
            .register_type::<ModelForward>()
            .init_resource::<PlayerSettings>()
            .init_resource::<FlashlightState>()
            .add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (
                    (player_input, player_physics)
                        .chain()
                        .run_if(in_state(crate::game_state::GamePhase::Playing)),
                    apply_player_rotation.run_if(in_state(crate::game_state::GamePhase::Playing)),
                    cycle_flashlight_mode,
                    apply_flashlight_mode,
                    sync_flashlight_from_settings,
                    handle_player_reset,
                ),
            );
    }
}

pub(crate) fn spawn_player(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<PlayerSettings>,
    glb_cache: Res<GlbCache>,
) {
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::default(),
            Player,
            Speed(settings.move_speed),
            Velocity::default(),
            RotationState {
                target_direction: Vec3::NEG_Z,
            },
            CoyoteTimer {
                timer: Timer::from_seconds(settings.coyote_time, TimerMode::Once),
            },
            JumpBuffer {
                // Start finished so no spurious jump on first frame
                timer: {
                    let mut t = Timer::from_seconds(settings.jump_buffer_time, TimerMode::Once);
                    t.tick(Duration::from_secs(3600));
                    t
                },
            },
            MoveIntent::default(),
            // 统一碰撞体系统
            Collider::capsule(0.3, 0.9, CollisionMask::player()),
            CollisionResponse::default(),
            Name::new("Player"),
            WorldLabel::new("玩家").with_offset(3.0).with_font_size(14.0),
            // 时间回溯追踪
            RewindTracked,
            PositionHistory {
                stack: vec![TimedPosition {
                    timestamp: 0.0,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }],
                max_size: 1200,  // 1200 * 0.5s = 10分钟历史
                kind: TrackedEntityKind::Player,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneRoot(glb_cache.handles.get("models/animations/Walk.glb#Scene0").cloned().unwrap_or_else(|| assets.load("models/animations/Walk.glb#Scene0"))),
                Transform::IDENTITY,
                Model,
                Name::new("PlayerModel"),
            ));
            parent.spawn((
                Camera3d::default(),
                Msaa::Off,
                Camera { order: 1, ..default() },
                Transform::from_xyz(0.2, 1.5, 1.7),
                LookState { pitch: 0.0, yaw: 0.0 },
                Name::new("Camera"),
            ));
            parent.spawn((
                SpotLight {
                    color: settings.flashlight_color,
                    intensity: settings.flashlight_intensity,
                    range: settings.flashlight_range,
                    outer_angle: 0.5,
                    ..default()
                },
                Transform::from_xyz(0.0, 1.5, 0.5),
                Flashlight,
                Name::new("Flashlight"),
            ));
        });
}

/// Reads keyboard input and writes `MoveIntent`.
fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut MoveIntent, &Transform), With<Player>>,
    hiding_q: Query<(), With<crate::stealth::PlayerHiding>>,
) {
    let Ok((mut intent, transform)) = player_q.single_mut() else {
        warn!("player_input: Player 实体不存在");
        return;
    };

    // 躲藏时禁止移动
    if !hiding_q.is_empty() {
        intent.world_direction = Vec3::ZERO;
        return;
    }

    let mut input = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        input.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        input.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        input.x += 1.0;
    }

    intent.is_sprinting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if input != Vec3::ZERO {
        let (sin_yaw, cos_yaw) = transform.rotation.to_euler(EulerRot::YXZ).0.sin_cos();
        let flat_fwd = Vec3::new(-sin_yaw, 0.0, -cos_yaw);
        let flat_right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
        intent.world_direction = (flat_fwd * input.z + flat_right * input.x).normalize();

        // 调试：首次按键时打印确认
        static mut LAST_DEBUG: f32 = 0.0;
        let now = transform.translation.length_squared();
        // safety: 仅用于调试标志，非多线程环境
        unsafe {
            if (now - LAST_DEBUG).abs() > 100.0 {
                LAST_DEBUG = now;
                info!("player_input: WASD 已识别，方向={:?}, 位置={}", intent.world_direction, transform.translation);
            }
        }
    } else {
        intent.world_direction = Vec3::ZERO;
    }
}

/// Reads `MoveIntent` and applies physics: gravity, ground detection, jump, movement.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn player_physics(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    config: Res<GameplayConfig>,
    asset_server: Res<AssetServer>,
    mut player_q: Query<
        (
            &mut Transform,
            &Speed,
            &mut Velocity,
            &mut RotationState,
            &mut CoyoteTimer,
            &mut JumpBuffer,
            &MoveIntent,
        ),
        With<Player>,
    >,
    collision_q: Query<(&Transform, &Collider), Without<Player>>,
) {
    let Ok((
        mut transform,
        speed,
        mut velocity,
        mut rot_state,
        mut coyote,
        mut jump_buffer,
        intent,
    )) = player_q.single_mut()
    else {
        return;
    };

    // --- Vertical physics: ground detection, gravity, jump ---
    let player_xz = Vec2::new(transform.translation.x, transform.translation.z);
    let ground_y = find_ground_y(&collision_q, player_xz);
    let is_grounded = transform.translation.y <= ground_y + 0.1;

    if is_grounded {
        coyote.timer.reset();
        transform.translation.y = ground_y;
        velocity.y = 0.0;
    } else {
        coyote.timer.tick(time.delta());
        velocity.y += settings.gravity * time.delta_secs();
    }

    if keys.just_pressed(KeyCode::Space) {
        jump_buffer.timer.reset();
    }
    jump_buffer.timer.tick(time.delta());

    let can_jump = is_grounded || !coyote.timer.is_finished();
    let jump_buffered = !jump_buffer.timer.is_finished();

    if can_jump && jump_buffered {
        velocity.y = settings.jump_force;
        transform.translation.y = ground_y + 0.2;
        jump_buffer.timer.tick(Duration::from_secs(3600));
        coyote.timer.tick(Duration::from_secs(3600));
        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/jump.wav")),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.4)),
        ));
    }

    transform.translation.y += velocity.y * time.delta_secs();

    // --- Horizontal movement from intent ---
    if intent.world_direction != Vec3::ZERO {
        let current_speed = if intent.is_sprinting {
            settings.sprint_speed
        } else {
            speed.0
        };
        transform.translation += intent.world_direction * current_speed * time.delta_secs();

        if intent.world_direction.dot(rot_state.target_direction) < 0.9999 {
            rot_state.target_direction = intent.world_direction;
        }
    }

    // --- Horizontal collision push-out ---
    push_out_horizontal(
        &collision_q,
        &mut transform.translation,
        config.player_radius,
        config.player_height,
    );
}

/// Smoothly rotate the model child entity to face movement direction.
fn apply_player_rotation(
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    player_q: Query<(&RotationState, &Transform), With<Player>>,
    mut model_q: Query<&mut Transform, (With<Model>, Without<Player>)>,
) {
    let Ok((rot, player_t)) = player_q.single() else { return };
    let Ok(mut model_t) = model_q.single_mut() else { return };

    if rot.target_direction.length_squared() == 0.0 {
        // Keeps the model facing the last direction when not moving
        return;
    }

    // World-space target for the model's facing direction
    let model_fwd = match settings.model_forward {
        ModelForward::NegZ => Vec3::NEG_Z,
        ModelForward::PosZ => Vec3::Z,
    };
    let target_rot_world = Quat::from_rotation_arc(model_fwd, rot.target_direction);
    // Convert to local space (relative to the parent's camera yaw rotation)
    let target_rot_local = player_t.rotation.inverse() * target_rot_world;

    // Exponential ease toward target for smooth rotation
    let t = (1.0 - (-settings.rotation_speed * time.delta_secs()).exp()).min(1.0);
    model_t.rotation = model_t.rotation.slerp(target_rot_local, t);
}

fn sync_flashlight_from_settings(
    settings: Res<PlayerSettings>,
    mut flashlight_q: Query<&mut SpotLight, With<Flashlight>>,
) {
    if settings.is_changed() {
        for mut light in flashlight_q.iter_mut() {
            light.intensity = settings.flashlight_intensity;
            light.range = settings.flashlight_range;
            light.color = settings.flashlight_color;
        }
    }
}

/// 按 Q 键循环切换手电筒模式
fn cycle_flashlight_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<FlashlightState>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        state.mode = state.mode.cycle();
    }
}

/// 根据当前手电筒模式更新灯光参数
fn apply_flashlight_mode(
    state: Res<FlashlightState>,
    mut flashlight_q: Query<&mut SpotLight, With<Flashlight>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut light in flashlight_q.iter_mut() {
        match state.mode {
            FlashlightMode::Normal => {
                light.color = Color::srgb(1.0, 0.95, 0.8);
                light.intensity = 800_000.0;
                light.range = 20.0;
                light.outer_angle = 0.5;
            }
            FlashlightMode::UV => {
                light.color = Color::srgb(0.3, 0.1, 0.9);
                light.intensity = 600_000.0;
                light.range = 25.0;
                light.outer_angle = 0.3;
            }
            FlashlightMode::Off => {
                light.intensity = 0.0;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_player_reset(
    mut events: MessageReader<ResetPlayerEvent>,
    mut player_q: Query<
        (&mut Transform, &mut Velocity, &mut CoyoteTimer, &mut JumpBuffer, &mut RotationState, &mut MoveIntent),
        With<Player>,
    >,
) {
    for ev in events.read() {
        if let Ok((mut transform, mut velocity, mut coyote, mut jump_buffer, mut rot_state, mut intent)) =
            player_q.single_mut()
        {
            transform.translation = ev.position;
            transform.rotation = ev.rotation;
            velocity.y = 0.0;
            coyote.timer.reset();
            jump_buffer.timer.reset();
            jump_buffer.timer.tick(Duration::from_secs(3600));
            rot_state.target_direction = Vec3::NEG_Z;
            intent.world_direction = Vec3::ZERO;
            intent.is_sprinting = false;
            intent.world_direction = Vec3::ZERO;
        }
    }
}

// ═══════════════════════════════════════════
// 玩家交互射线接口
// ═══════════════════════════════════════════

/// 检查玩家到目标点之间是否有视线（无障碍物遮挡）
///
/// 用于交互检测——能否看到 NPC、敌人、收集品等。
/// `player_entity` 用于忽略玩家自身的碰撞体。
#[allow(dead_code)]
pub fn player_can_see(
    game_grid: &GameGridResource,
    player_pos: Vec3,
    target: Vec3,
    player_entity: Entity,
) -> bool {
    game_grid.has_line_of_sight(
        Vec2::new(player_pos.x, player_pos.z),
        Vec2::new(target.x, target.z),
        |id| *id == player_entity,
    )
}

/// 从玩家位置沿方向发射射线，返回命中的第一个实体
///
/// 可用于玩家瞄准、拾取检测等。
/// 返回命中实体的 Entity 和距离。
#[allow(dead_code)]
pub fn player_raycast(
    game_grid: &GameGridResource,
    player_pos: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(Entity, f32)> {
    let origin = Vec2::new(player_pos.x, player_pos.z);
    let dir = Vec2::new(direction.x, direction.z).normalize();
    game_grid
        .raycast(origin, dir, max_distance)
        .map(|hit| (hit.entity, hit.distance))
}
