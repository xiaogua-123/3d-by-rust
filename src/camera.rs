use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions};
use crate::game_state::GamePhase;
use crate::player::Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LookState {
    pub pitch: f32,
    pub yaw: f32,
}

pub struct CameraPlugin;

#[derive(Resource, Default)]
pub(crate) struct CursorToggleState {
    pub(crate) locked: bool,
    pub(crate) panel_override: bool,
}

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

    look.pitch = (look.pitch - delta.y * settings.mouse_sensitivity).clamp(-1.5, 1.5);
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
