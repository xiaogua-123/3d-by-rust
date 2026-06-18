//! Chunk RON 数据结构
//!
//! 定义大地图分块加载的配置格式。每个 ChunkDef 对应一个网格区块，
//! 包含该区块内的 NPC、收集品、ProximityModel 等内容。

use bevy::math::Vec3;
use serde::{Deserialize, Serialize};

/// 区块坐标（chunk 空间）
pub type ChunkPos = bevy::math::IVec2;

/// 区块状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChunkState {
    /// 未加载
    #[default]
    Unloaded,
    /// 加载中（带优先级分，用于排序）
    Loading { priority: u32 },
    /// 已加载
    Loaded,
    /// 卸载中
    Unloading,
}

/// 区块定义（从 RON 反序列化）
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChunkDef {
    /// 区块坐标
    pub coord: (i32, i32),
    /// 地形 GLB 场景路径（可选）
    pub terrain_glb: Option<String>,
    /// NPC 列表
    pub npcs: Vec<ChunkNpcDef>,
    /// 收集品列表
    pub collectibles: Vec<ChunkCollectibleDef>,
    /// 距离加载模型列表
    pub proximity_models: Vec<ChunkProximityDef>,
}

/// Chunk 中的 NPC 定义
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChunkNpcDef {
    pub name: String,
    pub model_path: String,
    pub position: (f32, f32, f32),
    pub rotation: f32,
    pub scale: f32,
    pub conversation_id: String,
    pub start_node: String,
    pub initial_action: String,
    pub patrol_route: Option<ChunkPatrolDef>,
    pub animations: std::collections::HashMap<String, String>,
    pub sounds: std::collections::HashMap<String, String>,
}

/// Chunk 中的巡逻路线定义
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChunkPatrolDef {
    pub speed: f32,
    pub points: Vec<(f32, f32, f32)>,
}

/// Chunk 中的收集品定义
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChunkCollectibleDef {
    pub name: String,
    pub model_path: String,
    pub position: (f32, f32, f32),
    pub pickup_sound: Option<String>,
    pub pickup_function: String,
    pub respawn_time: f32,
}

/// Chunk 中的距离加载模型定义
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChunkProximityDef {
    pub id: String,
    pub path: String,
    pub position: (f32, f32, f32),
    pub scale: f32,
    pub load_distance: f32,
    pub unload_distance: f32,
    pub label: Option<(String, f32)>,
}

/// 区块配置索引（maps 所有 ChunkPos → ChunkDef）
pub type ChunkDataMap = std::collections::HashMap<(i32, i32), ChunkDef>;

/// 将世界坐标转换为 chunk 坐标
pub fn world_to_chunk(pos: Vec3, chunk_size: f32) -> ChunkPos {
    ChunkPos::new(
        (pos.x / chunk_size).floor() as i32,
        (pos.z / chunk_size).floor() as i32,
    )
}
