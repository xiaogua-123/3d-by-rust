//! 谜题系统 — 拉杆、钥匙门、压力板、连锁触发
//!
//! 定义三种谜题元素（`Lever` / `KeyDoor` / `PressurePlate`），
//! 通过 `PuzzleLink` 组件连接成触发链。支持 F 键交互、碰撞触发和状态传播。

use bevy::prelude::*;

use crate::colliders::{TriggerEvent, TriggerType};
use crate::inventory::{RemoveItemEvent, Inventory};

/// 谜题元素类型
#[derive(Clone, PartialEq, Eq, Debug, Reflect)]
pub enum PuzzleType {
    /// 拉杆 — F 键两态切换
    Lever,
    /// 钥匙门 — 检测背包后开启
    KeyDoor { required_item: String },
    /// 压力板 — 玩家站立触发
    PressurePlate,
}

/// 谜题元素标记
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PuzzleElement {
    pub puzzle_type: PuzzleType,
}

/// 谜题解决状态
#[derive(Component, Reflect)]
#[reflect(Component)]
#[derive(Default)]
pub struct PuzzleState {
    pub solved: bool,
}


/// 谜题连锁：source 解决时自动设置 target.solved = true
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PuzzleLink {
    pub targets: Vec<Entity>,
}

/// 可交互标记（F 键交互范围）
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Interactable {
    pub radius: f32,
}

pub struct PuzzlePlugin;

impl Plugin for PuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PuzzleElement>()
            .register_type::<PuzzleType>()
            .register_type::<PuzzleState>()
            .register_type::<PuzzleLink>()
            .register_type::<Interactable>()
            .add_systems(
                Update,
                (
                    check_pressure_plates,
                    interact_with_puzzle,
                    solve_linked_puzzles,
                )
                    .chain()
                    .run_if(in_state(crate::game_state::GamePhase::Playing)),
            );
    }
}

/// 压力板检测 — 读取 TriggerEvent，标记压力板为已解决
fn check_pressure_plates(
    mut events: MessageReader<TriggerEvent>,
    mut puzzle_q: Query<(&PuzzleElement, &mut PuzzleState)>,
    player_q: Query<Entity, With<crate::player::Player>>,
) {
    for event in events.read() {
        if event.trigger_type != TriggerType::Enter {
            continue;
        }
        if player_q.get(event.other_entity).is_err() {
            continue;
        }
        if let Ok((element, mut state)) = puzzle_q.get_mut(event.trigger_entity)
            && element.puzzle_type == PuzzleType::PressurePlate {
                state.solved = true;
            }
    }
}

/// 按 F 与最近的谜题元素交互
fn interact_with_puzzle(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<crate::player::Player>>,
    hiding_q: Query<(), With<crate::stealth::PlayerHiding>>,
    interact_q: Query<(Entity, &Interactable, &PuzzleElement, &Transform)>,
    mut state_q: Query<&mut PuzzleState>,
    inventory: Res<Inventory>,
    mut remove_item_writer: MessageWriter<RemoveItemEvent>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    // 躲藏时禁止交互
    if !hiding_q.is_empty() {
        return;
    }

    let Ok(player_t) = player_q.single() else { return };
    let player_pos = player_t.translation;

    let mut nearest: Option<(f32, Entity)> = None;

    for (entity, interactable, element, transform) in interact_q.iter() {
        if let Ok(state) = state_q.get(entity)
            && !matches!(element.puzzle_type, PuzzleType::Lever) && state.solved {
                continue;
            }

        let dist = transform.translation.distance(player_pos);
        if dist <= interactable.radius {
            let replace = nearest.map(|(d, _)| dist < d).unwrap_or(true);
            if replace {
                nearest = Some((dist, entity));
            }
        }
    }

    let Some((_, entity)) = nearest else { return };
    let Ok((_, _, element, _)) = interact_q.get(entity) else { return };
    let Ok(mut state) = state_q.get_mut(entity) else { return };

    match element.puzzle_type {
        PuzzleType::Lever => {
            state.solved = !state.solved;
        }
        PuzzleType::KeyDoor { ref required_item } => {
            if !state.solved && inventory.has(required_item) {
                state.solved = true;
                remove_item_writer.write(RemoveItemEvent {
                    item_id: required_item.clone(),
                    amount: 1,
                });
            }
        }
        PuzzleType::PressurePlate => {
            // 由 check_pressure_plates 处理
        }
    }
}

/// 连锁传播 — PuzzleState 改变时同步更新 PuzzleLink.targets
#[allow(clippy::type_complexity)]
fn solve_linked_puzzles(
    mut set: ParamSet<(
        Query<(&PuzzleLink, &PuzzleState), Changed<PuzzleState>>,
        Query<&mut PuzzleState>,
    )>,
) {
    // 先收集需要连锁解决的目标
    let mut targets_to_solve: Vec<Entity> = Vec::new();
    for (link, state) in set.p0().iter() {
        if state.solved {
            targets_to_solve.extend(link.targets.iter().copied());
        }
    }
    // 再用第二个查询应用解决状态（避免 B0001 冲突）
    for target in targets_to_solve {
        if let Ok(mut target_state) = set.p1().get_mut(target) {
            target_state.solved = true;
        }
    }
}
