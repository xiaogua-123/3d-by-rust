//! 可收集物品系统
//!
//! 定义 `Collectible` 组件，提供旋转动画 + 上下漂浮效果。
//! 支持两种拾取模式：
//! - 碰撞自动拾取（`auto_pickup: true`）：走近自动收集
//! - E 键交互拾取（`auto_pickup: false`）：靠近后按 E 键拾取
//!
//! 拾取后连接背包系统（`GiveItemEvent`）和计分系统（`CollectItemEvent`）。

use bevy::prelude::*;
use bevy::audio::Volume;
use crate::colliders::{TriggerEvent, TriggerType};
use crate::config::GameplayConfig;
use crate::game_state::{CollectItemEvent, GamePhase};
use crate::inventory::GiveItemEvent;
use crate::player::Player;

/// 可收集物品组件
///
/// 标记可拾取道具，记录初始高度用于漂浮动画，
/// 以及关联的背包物品 ID 和拾取模式。
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Collectible {
    pub base_y: f32,
    /// 对应的背包物品 ID（与 ItemBank 中的 key 一致）
    pub item_id: String,
    /// true = 碰撞自动拾取, false = 需按 E 键交互拾取
    pub auto_pickup: bool,
}

/// 最近的可交互收集品（用于 E 键拾取和 UI 提示）
///
/// 由 `detect_nearby_collectibles` 系统每帧更新，
/// 被 `e_key_interaction` 和 UI 系统读取。
#[derive(Resource, Default)]
pub struct InteractionTarget(pub Option<Entity>);

/// 收集品插件
#[derive(Default)]
pub struct CollectiblePlugin;

impl Plugin for CollectiblePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Collectible>()
            .init_resource::<InteractionTarget>()
            .add_systems(Update, (
                animate_collectibles,
                check_collectible_pickup,
                detect_nearby_collectibles,
                e_key_interaction,
            ).run_if(in_state(GamePhase::Playing)));
    }
}

/// 收集品动画系统
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

/// 检测玩家碰撞到收集品（自动拾取）
fn check_collectible_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut trigger_events: MessageReader<TriggerEvent>,
    collectible_q: Query<&Collectible>,
    mut collect_writer: MessageWriter<CollectItemEvent>,
    mut give_writer: MessageWriter<GiveItemEvent>,
) {
    for event in trigger_events.read() {
        if event.trigger_type != TriggerType::Enter {
            continue;
        }
        let Ok(collectible) = collectible_q.get(event.trigger_entity) else {
            continue;
        };
        if !collectible.auto_pickup {
            continue;
        }

        // 添加到背包
        give_writer.write(GiveItemEvent {
            item_id: collectible.item_id.clone(),
            amount: 1,
        });

        // 触发计分/计数事件
        collect_writer.write(CollectItemEvent);

        // 播放音效
        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/collect.wav")),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
        ));

        // 销毁收集品
        commands.entity(event.trigger_entity).despawn();
    }
}

/// 检测玩家附近的可交互收集品（用于 E 键拾取和 UI 提示）
const INTERACTION_RANGE: f32 = 3.0;

fn detect_nearby_collectibles(
    player_q: Query<&Transform, With<Player>>,
    collectible_q: Query<(Entity, &Collectible, &Transform)>,
    mut target: ResMut<InteractionTarget>,
) {
    let Ok(player_t) = player_q.single() else {
        target.0 = None;
        return;
    };

    target.0 = None;
    let mut nearest_dist = INTERACTION_RANGE;

    for (entity, collectible, transform) in collectible_q.iter() {
        if collectible.auto_pickup {
            continue;
        }
        let dist = player_t.translation.distance(transform.translation);
        if dist < nearest_dist {
            nearest_dist = dist;
            target.0 = Some(entity);
        }
    }
}

/// E 键交互拾取系统
fn e_key_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    target: Res<InteractionTarget>,
    collectible_q: Query<&Collectible>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut collect_writer: MessageWriter<CollectItemEvent>,
    mut give_writer: MessageWriter<GiveItemEvent>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Some(entity) = target.0 else {
        return;
    };

    let Ok(collectible) = collectible_q.get(entity) else {
        return;
    };

    give_writer.write(GiveItemEvent {
        item_id: collectible.item_id.clone(),
        amount: 1,
    });

    collect_writer.write(CollectItemEvent);

    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/collect.wav")),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
    ));

    commands.entity(entity).despawn();
}

/// 将 `pickup_function` 字符串映射为背包物品 ID
///
/// 在 RON 数据中使用 `pickup_function` 字段指定物品类型，
/// 此函数将其映射到 `ItemBank` 中定义的 `item_id`。
pub fn pickup_function_to_item_id(func: &str) -> &str {
    match func {
        "on_pickup_coin" => "gold_coin",
        "on_pickup_herb" => "herb",
        "on_pickup_key_old" => "old_key",
        "on_pickup_key_east" => "east_wing_key",
        "on_pickup_key_courtyard" => "courtyard_key",
        "on_pickup_key_underground" => "underground_key",
        "on_pickup_crystal" => "magic_crystal",
        other => other,
    }
}
