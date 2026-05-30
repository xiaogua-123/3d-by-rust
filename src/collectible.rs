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
use crate::colliders::{TriggerEvent, TriggerType};
use crate::config::GameplayConfig;
use crate::game_state::{CollectItemEvent, GamePhase};

/// 可收集物品组件
/// 用于标记可拾取道具，并记录初始高度用于漂浮动画
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Collectible {
    pub base_y: f32,  // 收集品的基础高度（漂浮动画的基准Y坐标）
}

/// 收集品插件
/// 注册组件并添加收集品动画与拾取检测系统
pub struct CollectiblePlugin;

impl Plugin for CollectiblePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Collectible>()
            .add_systems(Update, (
                animate_collectibles,    // 收集品旋转+漂浮动画
                check_collectible_pickup // 玩家拾取检测
            ).run_if(in_state(GamePhase::Playing)));  // 仅游戏进行时运行
    }
}

/// 收集品动画系统
/// 让收集品持续旋转 + 上下正弦浮动
fn animate_collectibles(
    time: Res<Time>,                          // 时间资源
    mut anim_time: Local<f32>,                // 本地缓存动画累计时间
    config: Res<GameplayConfig>,              // 游戏配置（浮动速度/幅度）
    mut query: Query<(&mut Transform, &Collectible)>,  // 获取所有收集品
) {
    // 累计动画时间
    *anim_time += time.delta_secs();
    let t = *anim_time;

    // 遍历所有可收集物品，更新位置与旋转
    for (mut transform, collectible) in query.iter_mut() {
        // Y 轴旋转
        transform.rotation = Quat::from_rotation_y(t * config.collectible_rotation_speed);
        
        // 上下漂浮（基于正弦波）
        transform.translation.y = collectible.base_y
            + (t * config.collectible_float_speed + transform.translation.x).sin()
                * config.collectible_float_amplitude;
    }
}

/// 检测玩家是否拾取收集品
/// 使用触发器事件系统检测碰撞
fn check_collectible_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut trigger_events: MessageReader<TriggerEvent>,
    collectible_q: Query<&Collectible>,
    mut collect_writer: MessageWriter<CollectItemEvent>,
) {
    // 处理触发器事件
    for event in trigger_events.read() {
        // 只处理进入事件
        if event.trigger_type != TriggerType::Enter {
            continue;
        }

        // 检查是否是收集品触发器
        if collectible_q.get(event.trigger_entity).is_err() {
            continue;
        }

        // 检查触发的实体是否是玩家
        // 注意：这里简化处理，实际应该检查实体是否有 Player 组件
        // 由于触发器已经通过 CollisionMask 过滤，这里假设都是玩家

        // 销毁收集品
        commands.entity(event.trigger_entity).despawn();

        // 播放拾取音效
        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/112.wav")),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
        ));

        // 触发物品收集事件
        collect_writer.write(CollectItemEvent);
    }
}