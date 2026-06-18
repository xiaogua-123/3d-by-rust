//! Level Tool 集成插件 — 从 RON 加载多关卡配置，按 GameLevel 状态生成/清理
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::colliders::{Collider, CollisionMask};
use crate::collectible::{Collectible, pickup_function_to_item_id};
use crate::dialogue::DialogueTrigger;
use crate::level::{GameLevel, LevelEntity};
use crate::npc::{Npc, NpcConfig, NpcPatrol};
use crate::proximity_loader::{ProximityModel, ProximityModels};

// ═══════════════════════════════════════════
// RON 数据结构
// ═══════════════════════════════════════════

/// 顶层配置 — 包含所有关卡的数据
#[derive(Deserialize, Serialize)]
pub struct LevelToolConfig {
    pub levels: HashMap<String, LevelDef>,
}

/// 单个关卡的全部内容
#[derive(Deserialize, Serialize)]
pub struct LevelDef {
    pub map: MapDef,
    pub npcs: Vec<NpcDef>,
    pub collectibles: Vec<CollectibleDef>,
    pub proximity_models: Vec<ProximityModelDef>,
    pub sound_triggers: Vec<SoundTriggerDef>,
    pub menu: Option<MenuDef>,
}

#[derive(Deserialize, Serialize)]
pub struct NpcDef {
    pub name: String,
    pub model_path: String,
    pub position: (f32, f32, f32),
    pub rotation: f32,
    pub scale: f32,
    pub conversation_id: String,
    pub start_node: String,
    pub initial_action: String,
    pub patrol_route: Option<PatrolRouteDef>,
    pub animations: HashMap<String, String>,
    pub sounds: HashMap<String, String>,
}

#[derive(Deserialize, Serialize)]
pub struct PatrolRouteDef {
    pub speed: f32,
    pub points: Vec<(f32, f32, f32)>,
}

#[derive(Deserialize, Serialize)]
pub struct CollectibleDef {
    pub name: String,
    pub model_path: String,
    pub position: (f32, f32, f32),
    pub pickup_sound: Option<String>,
    pub pickup_function: String,
    pub respawn_time: f32,
}

#[derive(Deserialize, Serialize)]
pub struct ProximityModelDef {
    pub id: String,
    pub path: String,
    pub position: (f32, f32, f32),
    pub scale: f32,
    pub load_distance: f32,
    pub unload_distance: f32,
    pub label: Option<(String, f32)>,
}

#[derive(Deserialize, Serialize)]
pub struct SoundTriggerDef {
    pub state: String,
    pub sound_file: String,
    pub trigger_type: String,
    pub volume: f32,
}

#[derive(Deserialize, Serialize)]
pub struct MapDef {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub grid_unit: f32,
    pub terrain_model: Option<String>,
    pub skybox: Option<String>,
    pub ambient_light: String,
    pub fog_color: String,
    pub fog_near: f32,
    pub fog_far: f32,
}

#[derive(Deserialize, Serialize)]
pub struct MenuDef {
    pub background: String,
    pub title: String,
    pub buttons: Vec<MenuButtonDef>,
}

#[derive(Deserialize, Serialize)]
pub struct MenuButtonDef {
    pub text: String,
    pub action: String,
    pub position: (f32, f32),
    pub size: (f32, f32),
}

// ═══════════════════════════════════════════
// 资源
// ═══════════════════════════════════════════

/// 存储所有关卡配置，供 OnEnter 系统查询
#[derive(Resource)]
pub struct LevelToolData {
    pub config: LevelToolConfig,
}

// ═══════════════════════════════════════════
// 插件
// ═══════════════════════════════════════════

pub struct LevelToolPlugin;

impl Plugin for LevelToolPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_level_tool_config)
            // ── 关卡进入：生成该关卡的内容 ──
            .add_systems(OnEnter(GameLevel::Demo), spawn_level_content)
            .add_systems(OnEnter(GameLevel::Reception), spawn_level_content)
            .add_systems(OnEnter(GameLevel::EastWing), spawn_level_content)
            .add_systems(OnEnter(GameLevel::Courtyard), spawn_level_content)
            .add_systems(OnEnter(GameLevel::WestWing), spawn_level_content)
            .add_systems(OnEnter(GameLevel::Underground), spawn_level_content)
            .add_systems(OnEnter(GameLevel::WalkTest), spawn_level_content)
            .add_systems(OnEnter(GameLevel::MusicTest), spawn_level_content)
            .add_systems(OnEnter(GameLevel::ParticleTest), spawn_level_content)
            .add_systems(OnEnter(GameLevel::CollisionTest), spawn_level_content);
        // 清理由 level.rs 的 cleanup_level(LevelEntity) 统一处理
    }
}

// ═══════════════════════════════════════════
// Startup: 加载所有关卡配置
// ═══════════════════════════════════════════

fn load_level_tool_config(mut commands: Commands) {
    let ron_str = include_str!("../../assets/level/level_config.ron");
    let config: LevelToolConfig = match ron::de::from_str::<LevelToolConfig>(ron_str) {
        Ok(c) => {
            let total: usize = c.levels.values().map(|l| l.npcs.len()).sum();
            info!("LevelTool 配置已加载: {} 个关卡, 共 {} NPC",
                c.levels.len(), total);
            c
        }
        Err(e) => {
            error!("加载 level_config.ron 失败: {}. 跳过所有生成。", e);
            return;
        }
    };
    commands.insert_resource(LevelToolData { config });
}

// ═══════════════════════════════════════════
// OnEnter: 生成当前关卡的内容
// ═══════════════════════════════════════════

fn spawn_level_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut proximity: ResMut<ProximityModels>,
    level_data: Option<Res<LevelToolData>>,
    level_state: Res<State<GameLevel>>,
) {
    let Some(level_data) = level_data else {
        warn!("LevelToolData 未加载，跳过关卡内容生成");
        return;
    };

    let level_id = level_state.get().zone_id();
    let Some(level_def) = level_data.config.levels.get(level_id) else {
        info!("关卡 '{}' 无 LevelTool 配置，跳过", level_id);
        return;
    };

    info!("正在生成关卡 '{}' 的内容: {} NPC, {} 收集品, {} 距离加载模型",
        level_id, level_def.npcs.len(), level_def.collectibles.len(),
        level_def.proximity_models.len());

    // 清空上一关的距离加载模型，注册当前关卡的新模型
    proximity.models.clear();
    proximity.spawned.clear();

    spawn_npcs(&mut commands, &asset_server, level_def);
    spawn_collectibles(&mut commands, &asset_server, level_def);
    register_proximity_models(&mut proximity, level_def);
}

// ═══════════════════════════════════════════
// NPC 生成
// ═══════════════════════════════════════════

fn spawn_npcs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    level: &LevelDef,
) {
    for npc_def in &level.npcs {
        let pos = Vec3::new(npc_def.position.0, npc_def.position.1, npc_def.position.2);

        let patrol_points: Vec<Vec3> = match &npc_def.patrol_route {
            Some(route) => route.points.iter().map(|p| Vec3::new(p.0, p.1, p.2)).collect(),
            None => Vec::new(),
        };
        let speed = npc_def.patrol_route.as_ref().map_or(0.0, |r| r.speed);

        let has_model = !npc_def.model_path.is_empty();

        let mut entity_cmd = commands.spawn((
            Npc,
            NpcConfig {
                display_name: npc_def.name.clone(),
                conversation_id: npc_def.conversation_id.clone(),
                start_node: npc_def.start_node.clone(),
                patrol_points,
                speed,
                use_3d_orientation: false,
                turn_speed: 8.0,
                collision_push: true,
                push_npcs: true,
                collision_3d: false,
            },
            NpcPatrol { current_target: 1, ground_y: pos.y, velocity: Vec3::ZERO },
            DialogueTrigger {
                conversation_id: npc_def.conversation_id.clone(),
                start_node: npc_def.start_node.clone(),
                radius: 2.5,
            },
            Transform::from_translation(pos).with_scale(Vec3::splat(npc_def.scale)),
            Name::new(format!("NPC_{}", npc_def.name)),
            LevelEntity,
        ));

        entity_cmd.insert(Collider::sphere(0.3, CollisionMask::npc()));

        if has_model {
            let scene_path = if npc_def.model_path.contains('#') {
                npc_def.model_path.clone()
            } else {
                format!("{}#Scene0", npc_def.model_path)
            };
            entity_cmd.insert(SceneRoot(asset_server.load::<Scene>(&scene_path)));
        }
    }
    info!("关卡已生成 {} 个 NPC", level.npcs.len());
}

// ═══════════════════════════════════════════
// 收集品生成
// ═══════════════════════════════════════════

fn spawn_collectibles(
    commands: &mut Commands,
    asset_server: &AssetServer,
    level: &LevelDef,
) {
    for item in &level.collectibles {
        let pos = Vec3::new(item.position.0, item.position.1, item.position.2);
        let base_y = pos.y;

        let mut entity_cmd = commands.spawn((
            Collectible {
                base_y,
                item_id: pickup_function_to_item_id(&item.pickup_function).to_string(),
                auto_pickup: true,
            },
            Transform::from_translation(pos),
            Name::new(format!("Collectible_{}", item.name)),
            LevelEntity,
        ));

        if !item.model_path.is_empty() {
            let scene_path = if item.model_path.contains('#') {
                item.model_path.clone()
            } else {
                format!("{}#Scene0", item.model_path)
            };
            entity_cmd.insert(SceneRoot(asset_server.load::<Scene>(&scene_path)));
        }
    }
    info!("关卡已生成 {} 个收集品", level.collectibles.len());
}

// ═══════════════════════════════════════════
// 距离加载模型注册
// ═══════════════════════════════════════════

fn register_proximity_models(
    proximity: &mut ProximityModels,
    level: &LevelDef,
) {
    for m in &level.proximity_models {
        let position = Vec3::new(m.position.0, m.position.1, m.position.2);
        proximity.register(ProximityModel {
            id: m.id.clone(),
            path: m.path.clone(),
            position,
            scale: m.scale,
            load_distance: m.load_distance,
            unload_distance: m.unload_distance,
            label: m.label.clone(),
            collider: None,
        });
    }
    info!("关卡已注册 {} 个距离加载模型", level.proximity_models.len());
}
