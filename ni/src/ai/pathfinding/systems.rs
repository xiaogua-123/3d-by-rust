//! 寻路系统管线
//!
//! 执行顺序（每帧）：
//! 1. `detect_nav_target` — 检测目标变化，发送寻路请求
//! 2. `compute_path` — 处理请求，执行 NavMesh Polyanya 寻路
//! 3. `follow_nav_path` — 沿路径移动实体（含随机扰动）

use bevy::prelude::*;
use rand::Rng;
use vleue_navigator::NavMesh;

use super::components::*;
use super::nav_mesh::AiNavMesh;

/// ── System 1: 检测目标变化 ──
///
/// 对有 Navigator + NavTarget 的实体：
/// - 无 NavPath → 发出 RequestPathEvent
/// - NavPath.completed → 重新发出 RequestPathEvent
/// - 计时器触发 → 重新发出 RequestPathEvent（追逐动态目标）
pub fn detect_nav_target(
    mut q: Query<(Entity, &NavTarget, &mut Navigator, Option<&NavPath>)>,
    mut writer: MessageWriter<RequestPathEvent>,
    time: Res<Time>,
) {
    for (entity, target, mut nav, path) in q.iter_mut() {
        let needs_repath = match path {
            None => true,
            Some(p) if p.completed => true,
            Some(_) => {
                nav.timer.tick(time.delta());
                nav.timer.just_finished()
            }
        };

        if needs_repath {
            writer.write(RequestPathEvent {
                entity,
                to: target.position,
            });
        }
    }
}

/// ── System 2: 处理寻路请求 ──
///
/// 读取 RequestPathEvent，使用 NavMesh Polyanya 算法计算路径：
/// 1. 从 `AiNavMesh` 获取 NavMesh 实例
/// 2. 调用 `navmesh.path(from, to)`（2D XZ 平面）
/// 3. 将 2D 路径点转为 3D 坐标写入 NavPath
pub fn compute_path(
    mut events: MessageReader<RequestPathEvent>,
    q: Query<&Transform>,
    nav_q: Query<&Navigator>,
    mut path_q: Query<&mut NavPath>,
    mut cmd: Commands,
    nav_meshes: Option<Res<Assets<NavMesh>>>,
    ai_nav: Res<AiNavMesh>,
) {
    let Some(nav_meshes) = nav_meshes else {
        return; // Assets<NavMesh> 尚未创建
    };
    let Some(navmesh) = nav_meshes.get(&ai_nav.handle) else {
        return; // NavMesh 尚未初始化
    };

    for event in events.read() {
        let Ok(transform) = q.get(event.entity) else { continue };

        let from = Vec2::new(transform.translation.x, transform.translation.z);
        let to = Vec2::new(event.to.x, event.to.z);
        let y = transform.translation.y;

        let result = navmesh.path(from, to);

        if let Some(path_result) = result {
            // 2D 路径点 → 3D 世界坐标
            let mut waypoints: Vec<Vec3> = path_result
                .path
                .iter()
                .map(|p| Vec3::new(p.x, y, p.y))
                .collect();

            // 随机扰动：对中间路径点添加横向偏移，让路线更自然
            let perturbation = nav_q
                .get(event.entity)
                .map(|n| n.perturbation)
                .unwrap_or(0.0);
            if perturbation > 0.0 && waypoints.len() > 2 {
                let mut rng = rand::thread_rng();
                for i in 1..waypoints.len() - 1 {
                    let ox = (rng.r#gen::<f32>() - 0.5) * perturbation;
                    let oz = (rng.r#gen::<f32>() - 0.5) * perturbation;
                    waypoints[i].x += ox;
                    waypoints[i].z += oz;
                }
            }

            if let Ok(mut path) = path_q.get_mut(event.entity) {
                path.waypoints = waypoints;
                path.index = 0;
                path.completed = path.waypoints.is_empty();
            } else {
                cmd.entity(event.entity).insert(NavPath::new(waypoints));
            }
        }
    }
}

/// ── System 3: 沿路径移动 ──
///
/// 每帧沿 NavPath 移动实体：
/// 1. 取当前目标 waypoint
/// 2. 计算方向向量，normalize × speed × dt
/// 3. 距当前 waypoint < threshold → 前进到下一个
/// 4. 所有 waypoint 走完 → completed = true（停止移动）
///
/// 若 Navigator.perturbation > 0.0，移动方向会附加随机抖动，
/// 模拟自然行走的不完美步态。
pub fn follow_nav_path(
    mut q: Query<(&mut Transform, &mut NavPath, &Navigator)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut path, nav) in q.iter_mut() {
        if path.completed || path.waypoints.is_empty() {
            continue;
        }

        let target = path.waypoints[path.index];
        let dir = target - transform.translation;
        let dist = dir.length();

        if dist <= nav.threshold || !dist.is_finite() {
            path.advance();
            continue;
        }

        // 基础移动方向（已归一化）
        let mut move_dir = dir / dist;

        // 随机扰动：每帧给方向加点横向抖动
        if nav.perturbation > 0.0 {
            let mut rng = rand::thread_rng();
            let jitter_angle = (rng.r#gen::<f32>() - 0.5) * nav.perturbation * 0.3;
            let (sin, cos) = jitter_angle.sin_cos();
            let dx = move_dir.x * cos - move_dir.z * sin;
            let dz = move_dir.x * sin + move_dir.z * cos;
            move_dir.x = dx;
            move_dir.z = dz;
        }

        // 移动
        let step = move_dir * nav.speed * dt;
        transform.translation += step;

        // 旋转面向移动方向（使用扰动后的方向）
        if nav.rotate_to_face {
            let angle = move_dir.x.atan2(move_dir.z);
            transform.rotation = Quat::from_rotation_y(angle);
        }
    }
}
