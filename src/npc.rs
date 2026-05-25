use bevy::prelude::*;

use crate::dialogue::{DialogueTrigger, StartDialogueEvent};
use crate::game_state::GamePhase;
use crate::player::Player;

// ═══════════════════════════════════════════
// 组件
// ═══════════════════════════════════════════

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Npc;

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct NpcConfig {
    pub display_name: String,
    pub conversation_id: String,
    pub start_node: String,
    pub patrol_points: Vec<Vec3>,
    pub speed: f32,
}

impl NpcConfig {
    pub fn stationary(display_name: &str, conversation_id: &str, start_node: &str) -> Self {
        Self {
            display_name: display_name.to_string(),
            conversation_id: conversation_id.to_string(),
            start_node: start_node.to_string(),
            patrol_points: Vec::new(),
            speed: 0.0,
        }
    }

    pub fn patrol(
        display_name: &str,
        conversation_id: &str,
        start_node: &str,
        patrol_points: Vec<Vec3>,
        speed: f32,
    ) -> Self {
        Self {
            display_name: display_name.to_string(),
            conversation_id: conversation_id.to_string(),
            start_node: start_node.to_string(),
            patrol_points,
            speed,
        }
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct NpcPatrol {
    pub current_target: usize,
}

// ═══════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════

pub fn spawn_npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    config: NpcConfig,
    pos: Vec3,
    color: Color,
) -> Entity {
    let interaction_radius = 2.5;
    // TODO: GLB替换 → SceneRoot(asset_server.load("models/characters/{npc_id}.glb#Scene0"))
    // 方案1: spawn_npc 接受 asset_server 参数，按 config.display_name 选择模型
    // 方案2: 在 NpcConfig 中增加 model_path 字段，支持 per-NPC 模型
    // 替换后保留 Npc/NpcConfig/DialogueTrigger 组件，移除 Mesh3d/MeshMaterial3d
    let entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 1.6, 0.6))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        Transform::from_translation(pos),
        Npc,
        config.clone(),
        NpcPatrol {
            current_target: 1,
        },
        DialogueTrigger {
            conversation_id: config.conversation_id,
            start_node: config.start_node,
            radius: interaction_radius,
        },
        Name::new(format!("NPC_{}", config.display_name)),
    ));

    // 头顶名字标签 (spawn as child)
    // Note: no Text3D in simple form, skip for now
    // Can be added later with a proper name tag system

    entity.id()
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Npc>()
            .register_type::<NpcConfig>()
            .register_type::<NpcPatrol>()
            .add_systems(
                Update,
                (
                    npc_interact.run_if(in_state(GamePhase::Playing)),
                    npc_patrol.run_if(in_state(GamePhase::Playing)),
                    npc_face_player.run_if(in_state(GamePhase::Dialoguing)),
                ),
            );
    }
}

// ═══════════════════════════════════════════
// 交互系统
// ═══════════════════════════════════════════

fn npc_interact(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    npc_q: Query<(&Transform, &DialogueTrigger), With<Npc>>,
    mut dialogue_writer: MessageWriter<StartDialogueEvent>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Ok(player_t) = player_q.single() else { return };

    for (npc_t, trigger) in npc_q.iter() {
        let dist = player_t.translation.distance(npc_t.translation);
        if dist <= trigger.radius {
            info!("触发对话: {} (距离: {:.1})", trigger.conversation_id, dist);
            dialogue_writer.write(StartDialogueEvent {
                conversation_id: trigger.conversation_id.clone(),
                start_node: trigger.start_node.clone(),
            });
            return; // 一次只触发一个对话
        }
    }
}

// ═══════════════════════════════════════════
// 巡逻系统
// ═══════════════════════════════════════════

fn npc_patrol(
    time: Res<Time>,
    mut npc_q: Query<(&mut Transform, &NpcConfig, &mut NpcPatrol), With<Npc>>,
) {
    for (mut transform, config, mut patrol) in npc_q.iter_mut() {
        if config.patrol_points.len() < 2 {
            continue;
        }

        let target = config.patrol_points[patrol.current_target];
        let current = transform.translation;
        let dir = target - current;

        if dir.length() < 0.3 {
            patrol.current_target = (patrol.current_target + 1) % config.patrol_points.len();
            continue;
        }

        let direction = dir.normalize();
        transform.translation += direction * config.speed * time.delta_secs();

        // 面向移动方向
        if direction != Vec3::ZERO {
            transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);
        }
    }
}

// ═══════════════════════════════════════════
// 面向玩家
// ═══════════════════════════════════════════

fn npc_face_player(
    player_q: Query<&Transform, With<Player>>,
    mut npc_q: Query<&mut Transform, (With<Npc>, Without<Player>)>,
) {
    let Ok(player_t) = player_q.single() else { return };

    for mut npc_t in npc_q.iter_mut() {
        let dir = player_t.translation - npc_t.translation;
        let flat_dir = Vec3::new(dir.x, 0.0, dir.z);
        if flat_dir.length_squared() > 0.001 {
            npc_t.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat_dir.normalize());
        }
    }
}
