//! 相机系统
//!
//! 提供两套相机控制器：
//! - `CameraControllerPlugin`：通用 FPS 相机，无游戏状态依赖
//! - `CameraPlugin`：游戏主相机，根据 `GamePhase` 自动切换响应模式
//!
//! 包含鼠标视角控制、俯仰限制、光标锁定等功能。

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions};
use crate::game_state::GamePhase;
use crate::player::{Flashlight, Player};

const PITCH_LIMIT: f32 = 1.5; // 俯仰角限制（弧度），防止垂直翻转

// ============================================================================
// 通用相机控制器 — 无游戏状态依赖，任何场景可复用
// ============================================================================

/// 通用第一人称相机控制器组件，存储移动速度、灵敏度及当前朝向。
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CameraController {
    pub walk_speed: f32,          // 步行速度
    pub run_speed: f32,           // 奔跑速度（按住 Shift）
    pub mouse_sensitivity: f32,   // 鼠标灵敏度
    pub pitch: f32,                   // 当前俯仰角（局部）
    pub yaw: f32,                     // 当前偏航角（局部）
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
/// 不依赖 GamePhase / Player 等游戏状态，可直接用于任何场景。
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraController>()
            .add_systems(Update, (
                camera_wasd.run_if(in_state(GamePhase::Playing).or(in_state(GamePhase::Creative))),
                camera_mouse_look.run_if(in_state(GamePhase::Playing).or(in_state(GamePhase::Creative))),
                camera_cursor_toggle.run_if(in_state(GamePhase::Playing).or(in_state(GamePhase::Creative))),
            ));
    }
}

/// 根据键盘 WASD/QE/Space/Shift 控制相机移动（世界空间）。
fn camera_wasd(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    phase: Res<State<GamePhase>>,
    mut query: Query<(&CameraController, &mut Transform)>,
) {
    let is_creative = phase.get() == &GamePhase::Creative;
    for (ctl, mut transform) in query.iter_mut() {
        // 按住 Shift 时使用奔跑速度（创造模式下 Shift 用于下降）
        let speed = if !is_creative && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)) {
            ctl.run_speed
        } else {
            ctl.walk_speed
        };

        let forward = *transform.forward(); // 相机本地前向（解引用 Dir3 -> Vec3）
        let right = *transform.right();     // 相机本地右向
        let mut direction = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) { direction += forward; }
        if keys.pressed(KeyCode::KeyS) { direction -= forward; }
        if keys.pressed(KeyCode::KeyD) { direction += right; }
        if keys.pressed(KeyCode::KeyA) { direction -= right; }
        if is_creative {
            if keys.pressed(KeyCode::Space) { direction += Vec3::Y; }
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) { direction -= Vec3::Y; }
        } else {
            if keys.pressed(KeyCode::KeyE) { direction += Vec3::Y; } // 上升
            if keys.pressed(KeyCode::KeyQ) { direction -= Vec3::Y; } // 下降
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}

/// 根据鼠标移动量更新相机朝向（欧拉角），并限制俯仰角。
fn camera_mouse_look(
    motion: Res<AccumulatedMouseMotion>, // 每帧累计鼠标移动
    mut query: Query<(&mut CameraController, &mut Transform)>,
) {
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    for (mut ctl, mut transform) in query.iter_mut() {
        // 更新偏航角（左右旋转）
        ctl.yaw -= delta.x * ctl.mouse_sensitivity;
        // 更新俯仰角（上下旋转），并限制在 PITCH_LIMIT 范围内
        ctl.pitch = (ctl.pitch - delta.y * ctl.mouse_sensitivity).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        // 将欧拉角（YXZ 顺序）转换为四元数并应用到 Transform
        transform.rotation = Quat::from_euler(EulerRot::YXZ, ctl.yaw, ctl.pitch, 0.0);
    }
}

/// Backspace 键切换鼠标锁定状态。
fn camera_cursor_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions>, // Single 确保只有一个窗口光标资源
) {
    if !keys.just_pressed(KeyCode::Backspace) {
        return;
    }
    match cursor.grab_mode {
        CursorGrabMode::None => {
            // 当前未锁定 → 锁定光标并隐藏
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
        CursorGrabMode::Locked | CursorGrabMode::Confined => {
            // 当前锁定或受限 → 释放光标并显示
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

// ============================================================================
// 游戏专用相机层 — 玩家跟随 + GamePhase 光标管理
// ============================================================================

/// 玩家第一人称视角的朝向状态（用于子实体相机）。
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LookState {
    pub pitch: f32, // 俯仰角
    pub yaw: f32,   // 偏航角
}

/// 光标切换状态（记录锁定状态、调试面板覆盖）。
#[derive(Resource, Default)]
pub(crate) struct CursorToggleState {
    pub(crate) locked: bool,         // 当前是否锁定
    pub(crate) panel_override: bool, // 调试面板是否强制解锁光标
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LookState>()
            .init_resource::<CursorToggleState>()
            // 进入游戏时锁定光标
            .add_systems(OnEnter(GamePhase::Playing), lock_cursor)
            // 退出游戏时释放光标
            .add_systems(OnExit(GamePhase::Playing), unlock_cursor)
            // 进入对话时释放光标
            .add_systems(OnEnter(GamePhase::Dialoguing), unlock_cursor)
            // 退出对话时锁定光标
            .add_systems(OnExit(GamePhase::Dialoguing), lock_cursor)
            // PreUpdate：处理调试面板光标覆盖请求（在 egui 前执行）
            .add_systems(PreUpdate, sync_cursor_override)
            .add_systems(Update, (
                cursor_toggle,                                          // O 键切换光标
                mouse_look.run_if(in_state(GamePhase::Playing)),       // 仅游戏中鼠标视角
                sync_flashlight_look.after(mouse_look),                // 手电筒跟随相机俯仰
            ));
    }
}

/// 当调试面板请求时，强制解锁光标并显示（最高优先级）。
/// 在 PreUpdate 中执行，确保在 bevy_egui 处理输入之前设置好。
fn sync_cursor_override(
    state: Res<CursorToggleState>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if state.panel_override {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// O 键切换光标锁定状态，仅在 Playing 或 Dialoguing 阶段有效，
/// 且不受调试面板覆盖影响。
fn cursor_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<GamePhase>>,
    mut cursor: Single<&mut CursorOptions>,
    mut toggle: ResMut<CursorToggleState>,
) {
    // 调试面板覆盖期间忽略手动切换
    if toggle.panel_override {
        return;
    }
    // 只在游戏或对话中允许切换光标
    if !matches!(phase.get(), GamePhase::Playing | GamePhase::Dialoguing) {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyO) {
        return;
    }
    // 切换锁定状态
    toggle.locked = !toggle.locked;
    if toggle.locked {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// 鼠标视角控制：旋转玩家实体（偏航）和相机子实体（俯仰）。
/// 仅在 Playing 阶段运行。
fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<crate::player::PlayerSettings>, // 玩家设置中的鼠标灵敏度
    mut player_q: Query<&mut Transform, (With<Player>, Without<Camera3d>)>, // 玩家本体 Transform
    mut camera_q: Query<(&mut Transform, &mut LookState), With<Camera3d>>, // 相机子实体
) {
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let Ok(mut player_t) = player_q.single_mut() else { return };
    let Ok((mut cam_t, mut look)) = camera_q.single_mut() else { return };

    // 偏航：旋转玩家实体，使模型整体转向
    look.yaw -= delta.x * settings.mouse_sensitivity;
    player_t.rotation = Quat::from_rotation_y(look.yaw);

    // 俯仰：只旋转相机子实体（相对于玩家），不影响玩家身体朝向
    look.pitch = (look.pitch - delta.y * settings.mouse_sensitivity).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    cam_t.rotation = Quat::from_rotation_x(look.pitch);
}

/// 手电筒跟随相机俯仰，实现上下瞄准
fn sync_flashlight_look(
    camera_q: Query<&LookState, With<Camera3d>>,
    mut flashlight_q: Query<&mut Transform, (With<Flashlight>, Without<Camera3d>)>,
) {
    let Ok(look) = camera_q.single() else { return };
    let Ok(mut transform) = flashlight_q.single_mut() else { return };
    transform.rotation = Quat::from_rotation_x(look.pitch);
}

/// 锁定光标（游戏开始时）
fn lock_cursor(mut cursor: Single<&mut CursorOptions>, mut toggle: ResMut<CursorToggleState>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    toggle.locked = true;
}

/// 释放光标（退出游戏时）
fn unlock_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}