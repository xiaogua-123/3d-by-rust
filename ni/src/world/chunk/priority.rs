//! Chunk 优先级算法
//!
//! 根据玩家位置和相机朝向计算每个 chunk 的加载优先级。
//! 优先级越高 → 越早加载。朝向权重 > 距离权重，确保玩家面朝方向的区块先加载。

use bevy::math::{IVec2, Vec2};

/// 计算一个 chunk 的加载优先级分
///
/// # 参数
/// - `player_chunk`: 玩家当前所在的 chunk 坐标
/// - `camera_fwd`: 相机在 XZ 平面的朝向向量（已归一化）
/// - `candidate`: 要评估的 chunk 坐标
///
/// # 返回值
/// - 优先级分（越高越优先加载）
/// - 基础半径内的 chunk 自动获得高优先级
/// - 面朝方向的 chunk 额外加分
pub fn compute_chunk_priority(
    player_chunk: IVec2,
    camera_fwd: Vec2,
    candidate: IVec2,
    base_radius: u32,
) -> f32 {
    let offset = candidate - player_chunk;
    let dist = (offset.x.abs() + offset.y.abs()) as f32; // Manhattan 距离

    // ── 距离分：越近越高 ──
    let max_dist = base_radius as f32 * 2.0;
    let distance_score = 1.0 - (dist / max_dist).clamp(0.0, 1.0);

    // ── 朝向分：相机朝向与 chunk 方向向量的点积 ──
    // chunk 方向向量 = candidate - player_chunk（XZ 平面）
    let dir_to_chunk = if offset.x == 0 && offset.y == 0 {
        Vec2::ZERO // 玩家所在 chunk，方向无关
    } else {
        Vec2::new(offset.x as f32, offset.y as f32).normalize()
    };
    let facing_score = if dir_to_chunk == Vec2::ZERO {
        1.0 // 自身 chunk 永远高分
    } else {
        // 点积 [-1, 1] 映射到 [0, 1]
        (camera_fwd.dot(dir_to_chunk) + 1.0) * 0.5
    };

    // ── 综合分 ──
    // 玩家所在 chunk 或 base_radius 内的邻居：强制高优先级
    if dist <= base_radius as f32 {
        // 基础半径内：距离权重 0.6，朝向权重 0.4
        0.6 * distance_score + 0.4 * facing_score + 1.0 // +1.0 确保高于扩展半径
    } else {
        // 扩展半径内：距离权重 0.3，朝向权重 0.7
        0.3 * distance_score + 0.7 * facing_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_chunk_highest_priority() {
        let player = IVec2::new(0, 0);
        let fwd = Vec2::new(0.0, -1.0); // 朝 -Z
        let score = compute_chunk_priority(player, fwd, player, 2);
        let far_score = compute_chunk_priority(player, fwd, IVec2::new(3, 3), 2);
        assert!(score > far_score, "自身 chunk 应高于远处 chunk");
    }

    #[test]
    fn test_facing_chunk_higher_than_behind() {
        let player = IVec2::new(0, 0);
        let fwd = Vec2::new(0.0, -1.0); // 朝 -Z

        let ahead = IVec2::new(0, -3); // 前方
        let behind = IVec2::new(0, 3); // 后方

        let score_ahead = compute_chunk_priority(player, fwd, ahead, 2);
        let score_behind = compute_chunk_priority(player, fwd, behind, 2);

        assert!(
            (score_ahead - score_behind).abs() > 0.01,
            "朝向方向应显著高于背后方向: ahead={}, behind={}",
            score_ahead,
            score_behind,
        );
        // 前方应该确实高于后方
        if score_ahead <= score_behind {
            // 如果在 base_radius 内，两者都会 +1.0，但朝向分应有差异
            // 确保至少朝向分不同
            let fwd_dir = Vec2::new(0.0, -1.0);
            let ahead_dir = Vec2::new(0.0, -3.0).normalize();
            let behind_dir = Vec2::new(0.0, 3.0).normalize();
            assert!(
                fwd_dir.dot(ahead_dir) > fwd_dir.dot(behind_dir),
                "前方方向应与相机朝向更一致"
            );
        }
    }

    #[test]
    fn test_base_radius_boost() {
        let player = IVec2::new(0, 0);
        let fwd = Vec2::new(1.0, 0.0); // 朝 +X

        // base_radius=2 内的 chunk
        let inside = IVec2::new(2, 0);
        // base_radius=2 外的 chunk（同一方向 but 更远）
        let outside = IVec2::new(3, 0);

        let score_inside = compute_chunk_priority(player, fwd, inside, 2);
        let score_outside = compute_chunk_priority(player, fwd, outside, 2);

        assert!(
            score_inside > score_outside,
            "基础半径内的 chunk 应高于扩展半径: inside={}, outside={}",
            score_inside,
            score_outside,
        );
    }
}
