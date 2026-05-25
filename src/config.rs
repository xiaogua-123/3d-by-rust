use bevy::prelude::*;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct GameplayConfig {
    pub pickup_radius: f32,
    pub enemy_damage_radius: f32,
    pub enemy_default_damage: u32,
    pub collectible_float_speed: f32,
    pub collectible_float_amplitude: f32,
    pub collectible_rotation_speed: f32,
    pub player_radius: f32,
    pub player_height: f32,
    // ═══ 塔防参数 ═══
    /// 炮塔放置距玩家的最小距离
    pub turret_place_min_dist: f32,
    /// 炮塔之间的最小间距
    pub turret_min_spacing: f32,
}

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
