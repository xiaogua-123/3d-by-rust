use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions};
use crate::game_state::GamePhase;
use crate::player::Player;

const PITCH_LIMIT: f32 = 1.5;

// ============================================================================
// 通用相机控制器 — 无游戏状态依赖，任何场景可复用
// ============================================================================

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CameraController {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub mouse_sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            run_speed: 10.0,
            mouse_sensitivity: 0.002,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/// 通用第一人称相机插件，提供 WASD 移动 + 鼠标视角 + 光标管理。
/// 不依赖 GamePhase / Player 等游戏状态。
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraController>()
            .add_systems(Update, (
                camera_wasd,
                camera_mouse_look,
                camera_cursor_toggle,
            ));
    }
}

fn camera_wasd(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&CameraController, &mut Transform)>,
) {
    for (ctl, mut transform) in query.iter_mut() {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            ctl.run_speed
        } else {
            ctl.walk_speed
        };

        let forward = *transform.forward();
        let right = *transform.right();
        let mut direction = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) { direction += forward; }
        if keys.pressed(KeyCode::KeyS) { direction -= forward; }
        if keys.pressed(KeyCode::KeyD) { direction += right; }
        if keys.pressed(KeyCode::KeyA) { direction -= right; }
        if keys.pressed(KeyCode::KeyE) { direction += Vec3::Y; }
        if keys.pressed(KeyCode::KeyQ) { direction -= Vec3::Y; }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}

fn camera_mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut CameraController, &mut Transform)>,
) {
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    for (mut ctl, mut transform) in query.iter_mut() {
        ctl.yaw -= delta.x * ctl.mouse_sensitivity;
        ctl.pitch = (ctl.pitch - delta.y * ctl.mouse_sensitivity).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, ctl.yaw, ctl.pitch, 0.0);
    }
}

fn camera_cursor_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if !keys.just_pressed(KeyCode::Backspace) {
        return;
    }
    match cursor.grab_mode {
        CursorGrabMode::None => {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
        CursorGrabMode::Locked | CursorGrabMode::Confined => {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

// ============================================================================
// 游戏专用相机层 — 玩家跟随 + GamePhase 光标管理
// ============================================================================

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LookState {
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Resource, Default)]
pub(crate) struct CursorToggleState {
    pub(crate) locked: bool,
    pub(crate) panel_override: bool,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LookState>()
            .init_resource::<CursorToggleState>()
            .add_systems(OnEnter(GamePhase::Playing), lock_cursor)
            .add_systems(OnExit(GamePhase::Playing), unlock_cursor)
            .add_systems(OnEnter(GamePhase::Dialoguing), unlock_cursor)
            .add_systems(OnExit(GamePhase::Dialoguing), lock_cursor)
            .add_systems(PreUpdate, sync_cursor_override)
            .add_systems(Update, (
                cursor_toggle,
                mouse_look.run_if(in_state(GamePhase::Playing)),
            ));
    }
}

/// PreUpdate 中处理调试面板的光标覆盖请求 — 在 bevy_egui 处理输入之前运行
fn sync_cursor_override(
    state: Res<CursorToggleState>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if state.panel_override {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn cursor_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut cursor: Single<&mut CursorOptions>,
    mut toggle: ResMut<CursorToggleState>,
) {
    // 面板覆盖期间忽略 O 键切换
    if toggle.panel_override {
        return;
    }
    // 只在游戏中或对话中允许切换光标
    if !matches!(phase.get(), GamePhase::Playing | GamePhase::Dialoguing) {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyO) {
        return;
    }
    toggle.locked = !toggle.locked;
    if toggle.locked {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<crate::player::PlayerSettings>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<Camera3d>)>,
    mut camera_q: Query<(&mut Transform, &mut LookState), With<Camera3d>>,
) {
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let Ok(mut player_t) = player_q.single_mut() else { return };
    let Ok((mut cam_t, mut look)) = camera_q.single_mut() else { return };

    look.yaw -= delta.x * settings.mouse_sensitivity;
    player_t.rotation = Quat::from_rotation_y(look.yaw);

    look.pitch = (look.pitch - delta.y * settings.mouse_sensitivity).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    cam_t.rotation = Quat::from_rotation_x(look.pitch);
}

fn lock_cursor(mut cursor: Single<&mut CursorOptions>, mut toggle: ResMut<CursorToggleState>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    toggle.locked = true;
}

fn unlock_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}
