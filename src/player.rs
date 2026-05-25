use bevy::prelude::*;
use bevy::audio::Volume;
use std::time::Duration;
use crate::animation::AnimationController;
use crate::camera::LookState;
use crate::collision::{CollisionShape, find_ground_y, push_out_horizontal};
use crate::config::GameplayConfig;
use crate::level::ResetPlayerEvent;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerSettings {
    pub move_speed: f32,
    pub gravity: f32,
    pub jump_force: f32,
    pub rotation_speed: f32,
    pub mouse_sensitivity: f32,
    pub flashlight_intensity: f32,
    pub flashlight_range: f32,
    pub flashlight_color: Color,
    pub coyote_time: f32,
    pub jump_buffer_time: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            move_speed: 2.5,
            gravity: -15.0,
            jump_force: 8.0,
            rotation_speed: std::f32::consts::PI * 4.0,
            mouse_sensitivity: 0.002,
            flashlight_intensity: 800_000.0,
            flashlight_range: 20.0,
            flashlight_color: Color::srgb(1.0, 0.95, 0.8),
            coyote_time: 0.08,
            jump_buffer_time: 0.1,
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
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Player>()
            .register_type::<Speed>()
            .register_type::<Velocity>()
            .register_type::<RotationState>()
            .register_type::<Flashlight>()
            .register_type::<Model>()
            .register_type::<CoyoteTimer>()
            .register_type::<JumpBuffer>()
            .register_type::<MoveIntent>()
            .register_type::<PlayerSettings>()
            .init_resource::<PlayerSettings>()
            .add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (
                    (player_input, player_physics)
                        .chain()
                        .run_if(in_state(crate::game_state::GamePhase::Playing)),
                    apply_player_rotation.run_if(in_state(crate::game_state::GamePhase::Playing)),
                    sync_flashlight_from_settings,
                    handle_player_reset,
                ),
            );
    }
}

fn spawn_player(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<PlayerSettings>,
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
            Name::new("Player"),
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneRoot(assets.load("BrainStem.glb#Scene0")),
                Transform::IDENTITY,
                AnimationController::default(),
                Model,
                Name::new("PlayerModel"),
            ));
            parent.spawn((
                Camera3d::default(),
                Transform::from_xyz(0.0, 1.9, -1.0),
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
) {
    let Ok((mut intent, transform)) = player_q.single_mut() else { return };

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

    if input != Vec3::ZERO {
        let (sin_yaw, cos_yaw) = transform.rotation.to_euler(EulerRot::YXZ).0.sin_cos();
        let flat_fwd = Vec3::new(-sin_yaw, 0.0, -cos_yaw);
        let flat_right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
        intent.world_direction = (flat_fwd * input.z + flat_right * input.x).normalize();
    } else {
        intent.world_direction = Vec3::ZERO;
    }
}

/// Reads `MoveIntent` and applies physics: gravity, ground detection, jump, movement.
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
    collision_q: Query<(&Transform, &CollisionShape), Without<Player>>,
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
            AudioPlayer::new(asset_server.load("sounds/112.wav")),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.4)),
        ));
    }

    transform.translation.y += velocity.y * time.delta_secs();

    // --- Horizontal movement from intent ---
    if intent.world_direction != Vec3::ZERO {
        transform.translation += intent.world_direction * speed.0 * time.delta_secs();

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
    let target_rot_world = Quat::from_rotation_arc(Vec3::NEG_Z, rot.target_direction);
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
        }
    }
}
