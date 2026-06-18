//! 敌人 AI 状态机 — 感知、决策、行动管线
//!
//! 实现六状态 AI 行为树：`Idle → Patrol → Alert → Chase → Attack → Search`。
//! 使用空间网格（`GameGridResource`）进行视线检测，支持分离避障和回退检测机制。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::colliders::SmoothPush;
use crate::combat::MoveSpeed;
use crate::enemy::Enemy;
use crate::grid::GameGridResource;
use crate::player::Player;

/// AI 状态
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum AiState {
    Idle,
    Patrol,
    Alert,
    Chase,
    Attack,
    Search,
}

/// 敌人 AI 大脑组件
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct EnemyBrain {
    pub state: AiState,
    pub state_timer: Timer,
    pub vision_range: f32,
    pub vision_half_angle: f32,
    pub chase_speed_multiplier: f32,
    pub alert_duration: f32,
    pub search_duration: f32,
    pub idle_duration: f32,
    pub last_known_player_pos: Vec3,
    pub patrol_index: usize,
    /// false = XZ 平面定向（地面敌人，不倾斜）
    /// true = 全 3D 定向（飞行敌人等）
    pub use_3d_orientation: bool,
    /// slerp 转向速率，越大转动越快
    pub turn_speed: f32,
}

impl Default for EnemyBrain {
    fn default() -> Self {
        Self {
            state: AiState::Patrol,
            state_timer: Timer::from_seconds(0.0, TimerMode::Once),
            vision_range: 10.0,
            vision_half_angle: std::f32::consts::PI / 3.0,
            chase_speed_multiplier: 1.5,
            alert_duration: 1.0,
            search_duration: 5.0,
            idle_duration: 3.0,
            last_known_player_pos: Vec3::ZERO,
            patrol_index: 0,
            use_3d_orientation: false,
            turn_speed: 10.0,
        }
    }
}

impl EnemyBrain {
    #[allow(dead_code)]
    pub fn with_patrol(has_patrol: bool) -> Self {
        Self {
            state: if has_patrol { AiState::Patrol } else { AiState::Idle },
            ..Default::default()
        }
    }

    /// 创建支持全 3D 朝向的敌人（飞行单位等）
    #[allow(dead_code)]
    pub fn flying() -> Self {
        Self {
            use_3d_orientation: true,
            ..Default::default()
        }
    }
}

// ═══════════════════════════════════════════
// 分离力 — 水平排斥组件，防止敌人聚集叠罗汉
// ═══════════════════════════════════════════

/// 分离力组件
///
/// 使附近敌人之间产生水平排斥，避免堆叠。
/// `radius`: 排斥作用半径
/// `strength`: 排斥强度系数（0=无排斥，1=最大）
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Separation {
    pub radius: f32,
    pub strength: f32,
}

impl Default for Separation {
    fn default() -> Self {
        Self {
            radius: 1.5,
            strength: 0.3,
        }
    }
}

// ═══════════════════════════════════════════
// 巡逻偏移 — 目标点随机偏移，防止多敌人汇聚同一点
// ═══════════════════════════════════════════

/// 巡逻目标点偏移组件
///
/// 对每个敌人的巡逻目标添加一个固定的随机偏移量，
/// 使得多个使用相同巡逻点的敌人不会精确汇聚在同一点。
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PatrolOffset {
    pub offset: Vec3,
}

// ═══════════════════════════════════════════
// 兜底检测 — 卡住/飘起检测与恢复
// ═══════════════════════════════════════════

/// 兜底检测组件
///
/// 监控敌人的位置异常：
/// 1. Y 轴高度超过阈值 → 传送回落点
/// 2. 长时间位置未变化（卡住）→ 传送到回落点
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FallbackDetection {
    /// 地面的 Y 坐标（初始生成 Y）
    pub ground_y: f32,
    /// 卡住计时器
    pub stuck_timer: f32,
    /// 卡住判定阈值（秒）
    pub stuck_threshold: f32,
    /// 上一帧位置（用于判断是否移动了）
    pub last_position: Vec3,
    /// 兜底传送回落点（通常是生成位置或最近导航点）
    pub fallback_point: Vec3,
}

impl FallbackDetection {
    /// 在生成位置创建兜底检测
    #[allow(dead_code)]
    pub fn new(spawn_pos: Vec3) -> Self {
        Self {
            ground_y: spawn_pos.y,
            stuck_timer: 0.0,
            stuck_threshold: 3.0,
            last_position: spawn_pos,
            fallback_point: spawn_pos,
        }
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AiState>()
            .register_type::<EnemyBrain>()
            .register_type::<Separation>()
            .register_type::<PatrolOffset>()
            .register_type::<FallbackDetection>()
            .add_systems(
                Update,
                (
                    ai_state_update,
                    ai_separation,
                    ai_movement,
                    ai_fallback_detection,
                )
                    .chain()
                    .run_if(in_state(crate::game_state::GamePhase::Playing)),
            );
    }
}

/// 感知 + 决策 — 检测玩家并驱动状态转换
#[allow(clippy::type_complexity)]
fn ai_state_update(
    time: Res<Time>,
    game_grid: Res<GameGridResource>,
    player_q: Query<(Entity, &Transform), (With<Player>, Without<Enemy>)>,
    mut brain_q: Query<(&Transform, &mut EnemyBrain), (With<Enemy>, Without<Player>)>,
) {
    let Ok((player_entity, player_t)) = player_q.single() else { return };
    let player_pos = player_t.translation;
    let player_pos_xz = Vec2::new(player_pos.x, player_pos.z);

    for (enemy_t, mut brain) in brain_q.iter_mut() {
        let dir_to_player = player_pos - enemy_t.translation;
        let dist_to_player = dir_to_player.length();
        let enemy_pos_xz = Vec2::new(enemy_t.translation.x, enemy_t.translation.z);

        // ── 感知：视锥 + 视线遮挡检测 ──
        let player_visible = dist_to_player <= brain.vision_range && {
            let enemy_fwd = enemy_t.rotation * Vec3::NEG_Z;
            let angle = dir_to_player.normalize_or_zero().dot(enemy_fwd).acos();
            angle <= brain.vision_half_angle
            // 视锥通过后再做遮挡检测
        } && game_grid.has_line_of_sight(
            enemy_pos_xz,
            player_pos_xz,
            |id| *id == player_entity,
        );

        if player_visible {
            brain.last_known_player_pos = player_pos;
        }

        // ── 状态机转换 ──
        brain.state_timer.tick(time.delta());

        match brain.state {
            AiState::Idle => {
                if brain.state_timer.just_finished() {
                    brain.state = AiState::Patrol;
                }
            }

            AiState::Patrol => {
                if player_visible {
                    brain.state = AiState::Alert;
                    brain.state_timer = Timer::from_seconds(brain.alert_duration, TimerMode::Once);
                }
            }

            AiState::Alert => {
                if !player_visible {
                    brain.state = AiState::Patrol;
                } else if brain.state_timer.just_finished() {
                    brain.state = AiState::Chase;
                }
            }

            AiState::Chase => {
                if dist_to_player <= 1.5 {
                    brain.state = AiState::Attack;
                } else if !player_visible && dist_to_player > brain.vision_range * 1.5 {
                    brain.state = AiState::Search;
                    brain.state_timer = Timer::from_seconds(brain.search_duration, TimerMode::Once);
                }
            }

            AiState::Attack => {
                if dist_to_player > 2.5 {
                    brain.state = AiState::Chase;
                }
            }

            AiState::Search => {
                if player_visible {
                    brain.state = AiState::Chase;
                } else if brain.state_timer.just_finished() {
                    brain.state = AiState::Patrol;
                }
            }
        }
    }
}

/// 行动 — 根据当前状态驱动万向移动
#[allow(clippy::type_complexity)]
fn ai_movement(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<(&mut Transform, &mut EnemyBrain, &MoveSpeed, &Enemy, Option<&PatrolOffset>), Without<Player>>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut brain, speed, enemy, patrol_offset) in enemy_q.iter_mut() {
        // 统一计算移动方向和速度
        let move_cmd = match brain.state {
            AiState::Patrol => {
                if enemy.patrol_points.is_empty() {
                    None
                } else {
                    if brain.patrol_index >= enemy.patrol_points.len() {
                        brain.patrol_index = 0;
                    }
                    let base_target = enemy.patrol_points[brain.patrol_index];
                    // 应用巡逻偏移（如果有），防止多敌人汇聚同一点
                    let target = if let Some(po) = patrol_offset {
                        base_target + po.offset
                    } else {
                        base_target
                    };
                    let dir = target - transform.translation;
                    let dist = dir.length();
                    if dist < 0.3 {
                        brain.patrol_index = (brain.patrol_index + 1) % enemy.patrol_points.len();
                        None // 到达目标，下帧切到下一个点
                    } else {
                        Some((dir / dist, speed.0))
                    }
                }
            }

            AiState::Chase => {
                let target = brain.last_known_player_pos;
                let dir = target - transform.translation;
                let dist = dir.length();
                if dist > 0.5 {
                    Some((dir / dist, speed.0 * brain.chase_speed_multiplier))
                } else {
                    None
                }
            }

            AiState::Search => {
                let target = brain.last_known_player_pos;
                let dir = target - transform.translation;
                let dist = dir.length();
                if dist > 0.5 {
                    Some((dir / dist, speed.0))
                } else {
                    None
                }
            }

            AiState::Attack => {
                // 攻击时面朝玩家但不移动
                if let Ok(player_t) = player_q.single() {
                    let dir = player_t.translation - transform.translation;
                    if dir.length_squared() > 0.01 {
                        let orient_dir = if brain.use_3d_orientation {
                            dir.normalize()
                        } else {
                            Vec3::new(dir.x, 0.0, dir.z).normalize()
                        };
                        if orient_dir.length_squared() > 0.001 {
                            let target_rot =
                                Quat::from_rotation_arc(Vec3::NEG_Z, orient_dir);
                            transform.rotation =
                                transform.rotation.slerp(target_rot, brain.turn_speed * dt);
                        }
                    }
                }
                continue;
            }

            AiState::Idle | AiState::Alert => continue,
        };

        if let Some((dir_norm, current_speed)) = move_cmd {
            // 万向移动：按连续方向向量平滑移动（支持任意角度）
            transform.translation += dir_norm * current_speed * dt;

            // 平滑旋转：根据 use_3d_orientation 选择 XZ 平面或全 3D 朝向
            let orient_dir = if brain.use_3d_orientation {
                dir_norm
            } else {
                Vec3::new(dir_norm.x, 0.0, dir_norm.z)
            };
            if orient_dir.length_squared() > 0.001 {
                let target_rot = Quat::from_rotation_arc(Vec3::NEG_Z, orient_dir.normalize());
                transform.rotation = transform.rotation.slerp(target_rot, brain.turn_speed * dt);
            }
        }
    }
}

// ═══════════════════════════════════════════
// 分离力系统 — 水平排斥防止敌人堆叠
// ═══════════════════════════════════════════

/// 分离力系统
///
/// 对所有带有 `Separation` 的敌人计算 XZ 平面上的水平排斥力，
/// 并在 `ai_movement` 之前直接位移，防止敌人汇聚到同一点。
/// 作为 CollisionManager 物理碰撞的"软"前置补充。
#[allow(clippy::type_complexity)]
fn ai_separation(
    mut q: ParamSet<(
        Query<(Entity, &Transform, &Separation), With<Enemy>>,
        Query<(&mut Transform, &mut SmoothPush), With<Enemy>>,
    )>,
) {
    // Phase 1: 读取所有敌人生理数据
    let items: Vec<(Entity, Vec3, f32, f32)> = q
        .p0()
        .iter()
        .map(|(e, t, s)| (e, t.translation, s.radius, s.strength))
        .collect();

    if items.len() < 2 {
        return;
    }

    // O(N²) 配对排斥计算 — 对 <50 敌人数量级足够快
    let mut pushes: HashMap<Entity, Vec2> = HashMap::new();

    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let (e_i, pos_i, rad_i, str_i) = &items[i];
            let (e_j, pos_j, rad_j, str_j) = &items[j];

            let dx = pos_i.x - pos_j.x;
            let dz = pos_i.z - pos_j.z;
            let dist_sq = dx * dx + dz * dz;
            let combined_radius = rad_i + rad_j;

            if dist_sq < combined_radius * combined_radius && dist_sq > 0.0001 {
                let dist = dist_sq.sqrt();
                let overlap = combined_radius - dist;
                // 强度 = 重叠比例 × 较小强度 × 分配系数
                let strength = (overlap / combined_radius) * str_i.min(*str_j) * 0.5;
                let nx = dx / dist;
                let nz = dz / dist;

                *pushes.entry(*e_i).or_default() += Vec2::new(nx * strength, nz * strength);
                *pushes.entry(*e_j).or_default() -= Vec2::new(nx * strength, nz * strength);
            }
        }
    }

    // Phase 2: 写入 SmoothPush（因在 Update 没有 apply_smooth_push，写入后立即积分+阻尼）
    for (entity, push) in pushes {
        if push.length_squared() < 0.0001 {
            continue;
        }
        if let Ok((mut transform, mut smooth)) = q.p1().get_mut(entity) {
            smooth.velocity.x += push.x;
            smooth.velocity.z += push.y;
            // 立即积分到位置（Update 无 apply_smooth_push）
            transform.translation += smooth.velocity;
            // 阻尼衰减，避免 FixedUpdate 双重积分
            let damping = smooth.damping;
            smooth.velocity *= damping;
        }
    }
}

// ═══════════════════════════════════════════
// 兜底检测系统 — 卡住/飘起异常恢复
// ═══════════════════════════════════════════

/// 兜底检测系统
///
/// 在 ai_movement 之后运行，检查：
/// 1. Y 轴高度远超地面 Y → 判定飘起，传送到回落点
/// 2. 长时间位置无明显变化 → 判定卡住，传送到回落点
///
/// 回落点通常是敌人的生成位置（或最近导航点）。
fn ai_fallback_detection(
    time: Res<Time>,
    mut enemy_q: Query<(&mut Transform, &mut FallbackDetection), With<Enemy>>,
) {
    let dt = time.delta_secs();
    let height_threshold = 2.0; // Y 偏离超过此值视为飘起

    for (mut transform, mut detection) in enemy_q.iter_mut() {
        let pos = transform.translation;
        let mut needs_fallback = false;

        // 条件 1: Y 轴高度异常（飘起或掉落地底）
        if (pos.y - detection.ground_y).abs() > height_threshold {
            warn!(
                "Fallback: 高度异常 (y={:.1}, ground={:.1})，传送回落点",
                pos.y, detection.ground_y
            );
            needs_fallback = true;
        }

        // 条件 2: 卡住检测 — 位置几乎没变
        let moved = pos.distance_squared(detection.last_position);
        if moved < 0.0001 {
            detection.stuck_timer += dt;
            if detection.stuck_timer >= detection.stuck_threshold {
                warn!(
                    "Fallback: 卡住 {:.1}s，传送回落点",
                    detection.stuck_timer
                );
                needs_fallback = true;
            }
        } else {
            // 移动了，重置卡住计时
            detection.stuck_timer = 0.0;
        }

        if needs_fallback {
            transform.translation = detection.fallback_point;
            detection.stuck_timer = 0.0;
            detection.last_position = detection.fallback_point;
        } else {
            detection.last_position = pos;
        }
    }
}
