//! Chunk 管理系统
//!
//! 每帧检测玩家位置和相机朝向，动态加载/卸载区块。
//! 仅在 `ChunkManager` 激活时运行。

use bevy::prelude::*;
use bevy::math::Vec2;

use crate::colliders::{Collider, ColliderShape, CollisionMask};
use crate::collectible::{Collectible, pickup_function_to_item_id};
use crate::dialogue::DialogueTrigger;
use crate::level::LevelEntity;
use crate::npc::{Npc, NpcConfig, NpcPatrol};
use crate::player::Player;
use crate::proximity_loader::{ProximityModel, ProximityModels};

use super::chunk_def::{
    world_to_chunk, ChunkDataMap, ChunkDef, ChunkPos,
};
use super::priority::compute_chunk_priority;
use super::ChunkManager;

/// 每帧最大加载 chunk 数（防止帧率尖刺）
const MAX_LOADS_PER_FRAME: usize = 2;
/// 每帧最大卸载 chunk 数
const MAX_UNLOADS_PER_FRAME: usize = 4;

/// Chunk 实体标记组件（用于追踪清理）
#[derive(Component)]
pub struct ChunkEntity {
    pub chunk_pos: ChunkPos,
}

/// 初始化 Demo 关卡的 Chunk 数据
pub fn init_demo_chunks(mut manager: ResMut<ChunkManager>) {
    let ron_str = include_str!("../../../assets/chunks/demo.ron");
    let data: ChunkDataMap = match ron::de::from_str(ron_str) {
        Ok(d) => d,
        Err(e) => {
            error!("解析 demo chunk 数据失败: {}", e);
            return;
        }
    };
    info!("Demo Chunk 数据已加载: {} 个区块", data.len());
    manager.init(data, 32.0, 2, 3);
}

/// 清理 ChunkManager（关卡退出时）
pub fn deinit_chunks(mut manager: ResMut<ChunkManager>) {
    manager.deinit();
}

/// 核心系统：每帧检测玩家位置和朝向，动态管理 chunk 加载
#[allow(clippy::type_complexity)]
pub fn update_chunks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut proximity: ResMut<ProximityModels>,
    mut manager: ResMut<ChunkManager>,
    player_q: Query<&Transform, With<Player>>,
    camera_q: Query<&crate::camera::LookState, (With<Camera3d>, Without<Player>)>,
    existing_chunks: Query<(&ChunkEntity, Entity)>,
) {
    if !manager.active {
        return;
    }

    let Ok(player_t) = player_q.single() else { return };
    let Ok(look) = camera_q.single() else { return };

    let player_pos = player_t.translation;
    let player_chunk = world_to_chunk(player_pos, manager.chunk_size);

    // ── 相机朝向（XZ 平面） ──
    // yaw 是绕 Y 轴的旋转角，forward = (-sin(yaw), -cos(yaw))
    let (sin_yaw, cos_yaw) = look.yaw.sin_cos();
    let camera_fwd = Vec2::new(-sin_yaw, -cos_yaw);

    // ── 收集当前已存在的 chunk 实体 ──
    // 用于检测 chunk 是否已经被加载了
    let mut loaded_chunks: std::collections::HashSet<ChunkPos> =
        std::collections::HashSet::new();
    let mut chunk_entities: std::collections::HashMap<ChunkPos, Vec<Entity>> =
        std::collections::HashMap::new();
    for (chunk_entity, entity) in existing_chunks.iter() {
        loaded_chunks.insert(chunk_entity.chunk_pos);
        chunk_entities
            .entry(chunk_entity.chunk_pos)
            .or_default()
            .push(entity);
    }

    let radius = manager.extended_radius as i32;
    let base_r = manager.base_radius as i32;

    // ── 收集候选 chunk ──
    struct Candidate {
        pos: ChunkPos,
        priority: f32,
    }

    let mut to_load: Vec<Candidate> = Vec::new();
    let mut to_unload: Vec<ChunkPos> = Vec::new();

    for x in -radius..=radius {
        for z in -radius..=radius {
            let cp = ChunkPos::new(player_chunk.x + x, player_chunk.y + z);
            let is_loaded = loaded_chunks.contains(&cp);
            let has_data = manager.chunk_data.contains_key(&(cp.x, cp.y));
            let in_base = x.abs() <= base_r && z.abs() <= base_r;

            if !is_loaded && has_data {
                let priority = compute_chunk_priority(
                    player_chunk, camera_fwd, cp, manager.base_radius,
                );
                // base 半径内直接加载；扩展半径内只有优先级 > 0.3 才加载
                if in_base || priority > 0.3 {
                    to_load.push(Candidate { pos: cp, priority });
                }
            } else if is_loaded && !in_base {
                // 超出 base 半径：检查是否需要卸载
                let dist = ((x.abs() + z.abs()) as f32).max(1.0);
                if dist > radius as f32 * 1.5 {
                    to_unload.push(cp);
                }
            }
        }
    }

    // ── 按优先级排序 ──
    to_load.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

    // ── 加载（上限 MAX_LOADS_PER_FRAME） ──
    let mut loaded_count = 0;
    for candidate in &to_load {
        if loaded_count >= MAX_LOADS_PER_FRAME {
            break;
        }
        if let Some(def) = manager.chunk_data.get(&(candidate.pos.x, candidate.pos.y)) {
            load_chunk(
                &mut commands,
                &asset_server,
                &mut proximity,
                candidate.pos,
                def,
            );
            loaded_count += 1;
        }
    }

    // ── 卸载（上限 MAX_UNLOADS_PER_FRAME） ──
    let mut unloaded_count = 0;
    for cp in &to_unload {
        if unloaded_count >= MAX_UNLOADS_PER_FRAME {
            break;
        }
        if let Some(entities) = chunk_entities.remove(cp) {
            for entity in entities {
                commands.entity(entity).despawn();
            }
            unloaded_count += 1;
        }
    }

    // ── 更新管理器的 spawned 追踪 ──
    // 注意：实际 entity id 由 load_chunk 写入，
    // 但这里我们用 ChunkEntity 组件来追踪，避免双重管理
    if !to_load.is_empty() || !to_unload.is_empty() {
        // 更新 ChunkManager 的 active 状态
        manager.active = true;
    }

    // 每 N 帧打印一次调试信息
    #[cfg(debug_assertions)]
    {
        let _ = player_chunk; // 保持变量存活
    }
}

/// 加载单个 chunk：生成所有实体
fn load_chunk(
    commands: &mut Commands,
    asset_server: &AssetServer,
    proximity: &mut ProximityModels,
    pos: ChunkPos,
    def: &ChunkDef,
) {
    // 1. 地形 GLB
    if let Some(glb_path) = &def.terrain_glb {
        let path = if glb_path.contains('#') {
            glb_path.clone()
        } else {
            format!("{}#Scene0", glb_path)
        };
        commands.spawn((
            SceneRoot(asset_server.load::<Scene>(&path)),
            Transform::from_xyz(
                pos.x as f32 * 32.0 + 16.0,
                0.0,
                pos.y as f32 * 32.0 + 16.0,
            ),
            LevelEntity,
            ChunkEntity { chunk_pos: pos },
            Name::new(format!("ChunkTerrain_{}_{}", pos.x, pos.y)),
        ));
    }

    // 2. NPC
    for npc_def in &def.npcs {
        let world_pos = Vec3::new(
            npc_def.position.0,
            npc_def.position.1,
            npc_def.position.2,
        );
        let patrol_points: Vec<Vec3> = match &npc_def.patrol_route {
            Some(route) => route.points.iter().map(|p| Vec3::new(p.0, p.1, p.2)).collect(),
            None => Vec::new(),
        };
        let speed = npc_def.patrol_route.as_ref().map_or(0.0, |r| r.speed);

        let mut cmd = commands.spawn((
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
            NpcPatrol { current_target: 1, ground_y: world_pos.y, velocity: Vec3::ZERO },
            DialogueTrigger {
                conversation_id: npc_def.conversation_id.clone(),
                start_node: npc_def.start_node.clone(),
                radius: 2.5,
            },
            Transform::from_translation(world_pos)
                .with_scale(Vec3::splat(npc_def.scale)),
            Name::new(format!("ChunkNPC_{}", npc_def.name)),
            LevelEntity,
            ChunkEntity { chunk_pos: pos },
        ));
        cmd.insert(Collider::sphere(0.3, CollisionMask::npc()));

        let scene_path = if npc_def.model_path.contains('#') {
            npc_def.model_path.clone()
        } else {
            format!("{}#Scene0", npc_def.model_path)
        };
        cmd.insert(SceneRoot(asset_server.load::<Scene>(&scene_path)));
    }

    // 3. 收集品
    for item in &def.collectibles {
        let item_pos = Vec3::new(item.position.0, item.position.1, item.position.2);
        let item_id = pickup_function_to_item_id(&item.pickup_function).to_string();
        let mut cmd = commands.spawn((
            Collectible {
                base_y: item_pos.y,
                item_id,
                auto_pickup: true,
            },
            Transform::from_translation(item_pos),
            Collider::trigger(ColliderShape::Sphere { radius: 0.5 }, CollisionMask::collectible()),
            Name::new(format!("ChunkCollectible_{}", item.name)),
            LevelEntity,
            ChunkEntity { chunk_pos: pos },
        ));
        if !item.model_path.is_empty() {
            let scene_path = if item.model_path.contains('#') {
                item.model_path.clone()
            } else {
                format!("{}#Scene0", item.model_path)
            };
            cmd.insert(SceneRoot(asset_server.load::<Scene>(&scene_path)));
        }
    }

    // 4. ProximityModels（注册到 ProximityModels resource）
    for m in &def.proximity_models {
        let position = Vec3::new(m.position.0, m.position.1, m.position.2);
        let id = format!("chunk_{}_{}_{}", pos.x, pos.y, m.id);
        proximity.register(ProximityModel {
            id,
            path: m.path.clone(),
            position,
            scale: m.scale,
            load_distance: m.load_distance,
            unload_distance: m.unload_distance,
            label: m.label.clone(),
            collider: None,
        });
    }
}
