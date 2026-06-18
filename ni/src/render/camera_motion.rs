//! 运镜系统 — 平滑跟随、镜头过渡、震屏、轨道环绕
//!
//! 通过组件驱动，将对应组件附加到 Camera3d 实体上即可生效。
//! 组件移除后对应效果自动停止，不干扰现有相机控制。

use bevy::prelude::*;

// ============================================================================
// 缓动函数
// ============================================================================

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum Easing {
    Linear,
    SmoothStep,
    EaseOutExp,
    EaseInOutQuad,
}

impl Easing {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::SmoothStep => t * t * (3.0 - 2.0 * t),
            Easing::EaseOutExp => 1.0 - (1.0 - t).powi(4),
            Easing::EaseInOutQuad => {
                if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
            }
        }
    }
}

// ============================================================================
// 镜头预设
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) struct CameraShot {
    pub position: Vec3,
    pub look_at: Vec3,
}

impl CameraShot {
    pub fn new(position: Vec3, look_at: Vec3) -> Self {
        Self { position, look_at }
    }

    /// 从当前 Transform 提取镜头预设
    #[allow(dead_code)]
    pub fn from_transform(transform: &Transform) -> Self {
        let position = transform.translation;
        let forward = transform.forward();
        Self {
            position,
            look_at: position + *forward,
        }
    }

    /// 插值两个镜头预设
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            look_at: self.look_at.lerp(other.look_at, t),
        }
    }
}

/// 常用运镜预设
#[allow(dead_code)]
pub(crate) mod shots {
    use super::CameraShot;
    use bevy::math::Vec3;

    /// 竞技场全景俯视
    pub fn arena_overview(arena_size: f32) -> CameraShot {
        let dist = arena_size * 0.8;
        CameraShot::new(
            Vec3::new(0.0, dist, dist * 0.7),
            Vec3::new(0.0, 0.0, 0.0),
        )
    }

    /// 核心特写
    pub fn core_closeup(core_pos: Vec3) -> CameraShot {
        CameraShot::new(
            core_pos + Vec3::new(2.0, 1.5, 2.0),
            core_pos,
        )
    }

    /// 生成点巡视
    pub fn spawn_point_view(spawn_pos: Vec3) -> CameraShot {
        CameraShot::new(
            spawn_pos + Vec3::new(1.5, 1.0, 1.5),
            spawn_pos,
        )
    }

    /// 胜利镜头
    pub fn victory() -> CameraShot {
        CameraShot::new(
            Vec3::new(0.0, 8.0, 12.0),
            Vec3::new(0.0, 0.0, 0.0),
        )
    }

    /// 失败镜头（核心坠毁视角）
    pub fn defeat(core_pos: Vec3) -> CameraShot {
        CameraShot::new(
            core_pos + Vec3::new(0.0, 1.0, 3.0),
            core_pos + Vec3::new(0.0, 0.5, 0.0),
        )
    }
}

// ============================================================================
// 组件
// ============================================================================

/// 平滑跟随目标实体
#[derive(Component)]
pub(crate) struct CameraTarget {
    pub target: Entity,
    pub offset: Vec3,
    pub follow_speed: f32,
    pub look_at: bool,
}

/// 震屏效果
#[derive(Component)]
pub(crate) struct CameraShake {
    pub intensity: f32,
    pub decay: f32,
    elapsed: f32,
    duration: f32,
}

impl CameraShake {
    #[allow(dead_code)]
    pub fn new(intensity: f32, duration: f32, decay: f32) -> Self {
        Self {
            intensity,
            decay,
            elapsed: 0.0,
            duration,
        }
    }

    fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }
}

/// 镜头过渡动画（附加到相机实体后自动插值）
#[derive(Component)]
pub(crate) struct CameraTransition {
    pub from: CameraShot,
    pub to: CameraShot,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
}

impl CameraTransition {
    #[allow(dead_code)]
    pub fn new(from: CameraShot, to: CameraShot, duration: f32) -> Self {
        Self {
            from,
            to,
            duration,
            elapsed: 0.0,
            easing: Easing::EaseOutExp,
        }
    }

    #[allow(dead_code)]
    /// 从当前位置过渡到目标镜头
    pub fn from_current(to: CameraShot, duration: f32, transform: &Transform) -> Self {
        Self::new(CameraShot::from_transform(transform), to, duration)
    }

    /// 插值进度
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }
}

/// 轨道环绕相机（自动绕目标点旋转）
#[derive(Component)]
pub(crate) struct OrbitCamera {
    pub target: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub rotate_speed: f32,
}

// ============================================================================
// 系统
// ============================================================================

/// 平滑跟随目标实体
fn apply_smooth_follow(
    targets: Query<&GlobalTransform, Without<CameraTarget>>,
    mut cameras: Query<(&CameraTarget, &mut Transform)>,
    time: Res<Time>,
) {
    for (target, mut transform) in cameras.iter_mut() {
        let Ok(target_transform) = targets.get(target.target) else { continue };
        let target_pos = target_transform.translation() + target.offset;
        let t = (1.0 - (-target.follow_speed * time.delta_secs()).exp())
            .clamp(0.0, 1.0);
        transform.translation = transform.translation.lerp(target_pos, t);
        if target.look_at {
            let dir = target_transform.translation() - transform.translation;
            if dir.length_squared() > 0.001 {
                transform.look_to(dir, Vec3::Y);
            }
        }
    }
}

/// 震屏效果
fn apply_camera_shake(
    time: Res<Time>,
    mut cameras: Query<(&mut CameraShake, &mut Transform)>,
) {
    for (mut shake, mut transform) in cameras.iter_mut() {
        shake.elapsed += time.delta_secs();
        if shake.finished() {
            continue;
        }
        let current_intensity = shake.intensity * (1.0 - shake.progress() * shake.decay);
        if current_intensity < 0.01 {
            continue;
        }
        // 在相机世界空间叠加随机偏移
        let offset = Vec3::new(
            (rand::random::<f32>() - 0.5) * 2.0,
            (rand::random::<f32>() - 0.5) * 2.0,
            (rand::random::<f32>() - 0.5) * 2.0,
        ) * current_intensity;
        transform.translation += offset;
    }
}

/// 镜头过渡动画
fn handle_camera_transition(
    time: Res<Time>,
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut CameraTransition, &mut Transform)>,
) {
    for (entity, mut transition, mut transform) in cameras.iter_mut() {
        transition.elapsed += time.delta_secs();
        let t = transition.easing.apply(transition.progress());
        let shot = transition.from.lerp(&transition.to, t);
        transform.translation = shot.position;
        transform.look_at(shot.look_at, Vec3::Y);
        if transition.elapsed >= transition.duration {
            commands.entity(entity).remove::<CameraTransition>();
        }
    }
}

/// 轨道环绕相机
fn orbit_camera(
    mut cameras: Query<(&mut OrbitCamera, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut orbit, mut transform) in cameras.iter_mut() {
        orbit.yaw += orbit.rotate_speed * time.delta_secs();
        let x = orbit.radius * orbit.yaw.cos() * orbit.pitch.cos();
        let y = orbit.radius * orbit.pitch.sin();
        let z = orbit.radius * orbit.yaw.sin() * orbit.pitch.cos();
        transform.translation = orbit.target + Vec3::new(x, y, z);
        transform.look_at(orbit.target, Vec3::Y);
    }
}

// ============================================================================
// 插件
// ============================================================================

pub(crate) struct CameraMotionPlugin;

impl Plugin for CameraMotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            apply_smooth_follow,
            apply_camera_shake,
            handle_camera_transition,
            orbit_camera,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        let easing = Easing::Linear;
        assert!((easing.apply(0.0)).abs() < 0.001);
        assert!((easing.apply(0.5) - 0.5).abs() < 0.001);
        assert!((easing.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_smoothstep() {
        let easing = Easing::SmoothStep;
        assert!((easing.apply(0.0)).abs() < 0.001);
        assert!((easing.apply(1.0) - 1.0).abs() < 0.001);
        // smoothstep 在 t=0.5 时 = 0.5^2 * (3 - 2*0.5) = 0.25 * 2 = 0.5
        assert!((easing.apply(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_easing_clamp() {
        let easing = Easing::EaseOutExp;
        assert!((easing.apply(-0.1)).abs() < 0.001);
        assert!((easing.apply(1.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_shot_lerp() {
        let a = CameraShot::new(Vec3::ZERO, Vec3::Y);
        let b = CameraShot::new(Vec3::ONE, Vec3::ZERO);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.position - Vec3::splat(0.5)).length() < 0.001);
        assert!((mid.look_at - Vec3::new(0.0, 0.5, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_from_transform() {
        let transform = Transform::from_xyz(1.0, 2.0, 3.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
        let shot = CameraShot::from_transform(&transform);
        assert!((shot.position - Vec3::new(1.0, 2.0, 3.0)).length() < 0.001);
    }

    #[test]
    fn test_transition_progress() {
        let from = CameraShot::new(Vec3::ZERO, Vec3::Y);
        let to = CameraShot::new(Vec3::ONE, Vec3::ZERO);
        let mut transition = CameraTransition::new(from, to, 2.0);
        assert!((transition.progress()).abs() < 0.001);
        transition.elapsed = 1.0;
        assert!((transition.progress() - 0.5).abs() < 0.001);
        transition.elapsed = 2.0;
        assert!((transition.progress() - 1.0).abs() < 0.001);
    }
}
