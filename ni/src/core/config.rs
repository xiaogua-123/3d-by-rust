//! 游戏核心配置资源
//!
//! 定义 `GameplayConfig` 资源，存储全局游戏参数（物理尺寸、拾取半径、
//! 塔防规则等）。可在运行时通过编辑器调整。

use bevy::prelude::*;

/// 游戏核心配置资源
/// 存储全局游戏参数、物理尺寸、塔防规则等配置，可在编辑器/运行时调整
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct GameplayConfig {
    /// 玩家拾取物品的有效半径
    pub pickup_radius: f32,
    /// 敌人造成伤害的检测半径
    pub enemy_damage_radius: f32,
    /// 敌人默认攻击力
    pub enemy_default_damage: u32,
    /// 可收集物品上下漂浮的速度
    pub collectible_float_speed: f32,
    /// 可收集物品上下漂浮的幅度（高度）
    pub collectible_float_amplitude: f32,
    /// 可收集物品自身旋转速度
    pub collectible_rotation_speed: f32,
    /// 玩家碰撞体半径
    pub player_radius: f32,
    /// 玩家碰撞体高度
    pub player_height: f32,
    // ═══ 塔防参数 ═══
    /// 炮塔放置距玩家的最小距离
    pub turret_place_min_dist: f32,
    /// 炮塔之间的最小间距（防止重叠）
    pub turret_min_spacing: f32,
}

/// 默认游戏配置
/// 提供一套平衡的初始参数，直接用于游戏运行
impl Default for GameplayConfig {
    fn default() -> Self {
        Self {
            pickup_radius: 1.0,
            enemy_damage_radius: 1.2,
            enemy_default_damage: 1,
            collectible_float_speed: 3.0,
            collectible_float_amplitude: 0.1,
            collectible_rotation_speed: 2.0,
            player_radius: 0.3,
            player_height: 0.5,
            turret_place_min_dist: 2.0,
            turret_min_spacing: 1.5,
        }
    }
}