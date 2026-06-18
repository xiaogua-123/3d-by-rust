//! 动画系统
//!
//! 定义 `AnimationState`（Idle/Walking/Running/Jumping/Falling）和
//! `AnimationController` 组件，基于玩家移动意图驱动 GLTF 动画图。
//! 自动在空闲/行走/奔跑状态之间平滑过渡。

use bevy::prelude::*;
use bevy::animation::graph::{AnimationGraph, AnimationNodeIndex};

use crate::game_state::GamePhase;
use crate::player::{MoveIntent, Player, Velocity};

// ═══════════════════════════════════════════
// 动画状态
// ═══════════════════════════════════════════

#[derive(Component, Reflect, Clone, PartialEq, Default, Debug)]
#[reflect(Component)]
pub enum AnimationState {
    #[default]
    Idle,
    Walking,
    Running,
    Jumping,
    Falling,
}

// ═══════════════════════════════════════════
// 动画控制器组件
// ═══════════════════════════════════════════

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationController {
    pub state: AnimationState,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            state: AnimationState::Idle,
        }
    }
}

// ═══════════════════════════════════════════
// 动画节点索引资源
// ═══════════════════════════════════════════

#[derive(Resource)]
pub struct PlayerAnimationClips {
    pub graph_handle: Handle<AnimationGraph>,
    pub walk: AnimationNodeIndex,
    pub run: AnimationNodeIndex,
    pub jump: AnimationNodeIndex,
    pub fall: AnimationNodeIndex,
}

/// 构建动画图并插入资源。
/// 在 Startup 中运行，在 spawn_player 之前执行。
fn build_animation_graph(mut commands: Commands, asset_server: Res<AssetServer>, mut graphs: ResMut<Assets<AnimationGraph>>) {
    let walk_clip: Handle<AnimationClip> =
        asset_server.load("models/animations/Walk.glb#Animation0");
    let run_clip: Handle<AnimationClip> =
        asset_server.load("models/animations/Running.glb#Animation0");
    let jump_clip: Handle<AnimationClip> =
        asset_server.load("models/animations/Jumping.glb#Animation0");
    let fall_clip: Handle<AnimationClip> =
        asset_server.load("models/animations/Big_Jump.glb#Animation0");

    let mut graph = AnimationGraph::new();
    let walk = graph.add_clip(walk_clip, 1.0, graph.root);
    let run = graph.add_clip(run_clip, 1.0, graph.root);
    let jump = graph.add_clip(jump_clip, 1.0, graph.root);
    let fall = graph.add_clip(fall_clip, 1.0, graph.root);

    let handle = graphs.add(graph);

    commands.insert_resource(PlayerAnimationClips {
        graph_handle: handle,
        walk,
        run,
        jump,
        fall,
    });
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AnimationController>()
            .register_type::<AnimationState>()
            .add_systems(Startup, build_animation_graph)
            .add_systems(
                Update,
                (
                    setup_player_animation,
                    (update_animation_state, apply_animation)
                        .chain(),
                )
                    .run_if(in_state(GamePhase::Playing)),
            );
    }
}

// ═══════════════════════════════════════════
// 查找 glTF 加载器创建的 AnimationPlayer 实体，绑定动画图
// ═══════════════════════════════════════════

/// 找到 glTF 场景内部实际持有 AnimationPlayer 的实体，
/// 将我们的动画图句柄和控制器附加上去。
/// 每次运行直到找到为止（场景可能在 PostUpdate 中才生成）。
pub fn setup_player_animation(
    anim_clips: Res<PlayerAnimationClips>,
    player_q: Query<Entity, With<Player>>,
    children_q: Query<&Children>,
    anim_player_q: Query<Entity, (With<AnimationPlayer>, Without<AnimationGraphHandle>)>,
    mut commands: Commands,
    mut done: Local<bool>,
) {
    if *done { return; }
    let Ok(player_entity) = player_q.single() else { return };

    // 递归遍历 Player 的所有后代，找到 glTF 添加的 AnimationPlayer
    let mut to_visit: Vec<Entity> = children_q
        .get(player_entity)
        .map(|c| c.to_vec())
        .unwrap_or_default();

    while let Some(entity) = to_visit.pop() {
        if anim_player_q.contains(entity) {
            commands.entity(entity).insert((
                AnimationGraphHandle(anim_clips.graph_handle.clone()),
                AnimationController::default(),
            ));
            info!("已绑定动画图到实体 {entity:?}");
            *done = true;
            return;
        }
        if let Ok(children) = children_q.get(entity) {
            to_visit.extend_from_slice(children);
        }
    }
}

// ═══════════════════════════════════════════
// 动画状态机
// ═══════════════════════════════════════════

fn update_animation_state(
    player_q: Query<(&Velocity, &MoveIntent), With<Player>>,
    mut anim_q: Query<&mut AnimationController>,
) {
    let Ok((velocity, intent)) = player_q.single() else { return };
    let Ok(mut controller) = anim_q.single_mut() else { return };

    let new_state = if velocity.y > 1.0 {
        AnimationState::Jumping
    } else if velocity.y < -1.0 {
        AnimationState::Falling
    } else if intent.world_direction != Vec3::ZERO {
        AnimationState::Walking
    } else {
        AnimationState::Idle
    };

    if new_state != controller.state {
        controller.state = new_state;
    }
}

// ═══════════════════════════════════════════
// 动画播放
// ═══════════════════════════════════════════

fn apply_animation(
    mut player_q: Query<(&AnimationController, &mut AnimationPlayer)>,
    clips: Res<PlayerAnimationClips>,
) {
    let Ok((controller, mut player)) = player_q.single_mut() else { return };

    let target = match controller.state {
        AnimationState::Idle => None,
        AnimationState::Walking => Some(clips.walk),
        AnimationState::Running => Some(clips.run),
        AnimationState::Jumping => Some(clips.jump),
        AnimationState::Falling => Some(clips.fall),
    };

    match target {
        None => {
            if player.playing_animations().next().is_some() {
                player.stop_all();
            }
        }
        Some(node) => {
            if !player.is_playing_animation(node) {
                player.stop_all();
                player.play(node).repeat();
            }
        }
    }
}
