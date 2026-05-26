// ═══════════════════════════════════════════
// 后期 GLB 模型替换方案
// ═══════════════════════════════════════════
// 收集品程序化球体(Sphere) → 3D 模型:
//   - models/props/collectible_gem.glb     (宝石，默认)
//   - models/props/collectible_coin.glb    (金币)
//   - models/props/collectible_crystal.glb (水晶)
//   - models/props/collectible_star.glb    (星星)
//   - 替换时保留 Collectible 组件
//   - 动画由 animate_collectibles 系统驱动（旋转 + 上下浮动），使用默认 PBR 材质
//   - 可按 zone 主题选择不同模型 (如 blue_forest → crystal, city → coin)
// ═══════════════════════════════════════════

use bevy::prelude::*;
use bevy::audio::Volume;
use crate::config::GameplayConfig;
use crate::game_state::{CollectItemEvent, GamePhase};
use crate::player::Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Collectible {
    pub base_y: f32,
}

pub struct CollectiblePlugin;

impl Plugin for CollectiblePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Collectible>()
            .add_systems(Update, (animate_collectibles, check_collectible_pickup).run_if(in_state(GamePhase::Playing)));
    }
}

fn animate_collectibles(
    time: Res<Time>,
    mut anim_time: Local<f32>,
    config: Res<GameplayConfig>,
    mut query: Query<(&mut Transform, &Collectible)>,
) {
    *anim_time += time.delta_secs();
    let t = *anim_time;
    for (mut transform, collectible) in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(t * config.collectible_rotation_speed);
        transform.translation.y = collectible.base_y
            + (t * config.collectible_float_speed + transform.translation.x).sin()
                * config.collectible_float_amplitude;
    }
}

fn check_collectible_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<GameplayConfig>,
    player_q: Query<&Transform, (With<Player>, Without<Collectible>)>,
    collectible_q: Query<(Entity, &Transform), With<Collectible>>,
    mut collect_writer: MessageWriter<CollectItemEvent>,
) {
    let Ok(player_t) = player_q.single() else { return };
    let player_pos = player_t.translation;

    for (entity, collectible_t) in collectible_q.iter() {
        let dist = player_pos.distance(collectible_t.translation);
        if dist < config.pickup_radius {
            commands.entity(entity).despawn();
            commands.spawn((
                AudioPlayer::new(asset_server.load("sounds/112.wav")),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
            ));
            collect_writer.write(CollectItemEvent);
        }
    }
}
