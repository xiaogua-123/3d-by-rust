//! 大地图分块加载系统
//!
//! 将大地图划分为网格区块，根据玩家位置和相机朝向动态加载/卸载。
//! 在 `GameLevel` 状态机之下作为第二层加载管理。
//!
//! # 用法
//!
//! ```ignore
//! // 注册到 App
//! app.add_plugins(ChunkPlugin);
//!
//! // 进入支持 chunk 的关卡时自动初始化
//! // 每帧自动管理 chunk 加载/卸载
//! ```

use bevy::prelude::*;

pub mod chunk_def;
pub mod priority;
pub mod systems;

use chunk_def::{ChunkDataMap, ChunkState};
use systems::{deinit_chunks, init_demo_chunks, update_chunks};

/// Chunk 管理器资源
///
/// 管理所有区块的状态、数据和生命周期。
/// 只有 `active` 为 true 时，`update_chunks` 系统才会运行。
#[derive(Resource)]
pub struct ChunkManager {
    /// 是否激活（只在支持 chunk 的关卡中为 true）
    pub active: bool,
    /// 每个 chunk 的边长（米）
    pub chunk_size: f32,
    /// 基础加载半径（chunk 数量）
    pub base_radius: u32,
    /// 扩展加载半径（朝向方向额外加载范围）
    pub extended_radius: u32,
    /// 所有区块的配置数据
    pub chunk_data: ChunkDataMap,
    /// 区块状态追踪
    pub states: std::collections::HashMap<chunk_def::ChunkPos, ChunkState>,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            active: false,
            chunk_size: 32.0,
            base_radius: 2,
            extended_radius: 3,
            chunk_data: std::collections::HashMap::new(),
            states: std::collections::HashMap::new(),
        }
    }
}

impl ChunkManager {
    /// 初始化 ChunkManager（在进入关卡时调用）
    pub fn init(
        &mut self,
        data: ChunkDataMap,
        chunk_size: f32,
        base_radius: u32,
        extended_radius: u32,
    ) {
        self.active = true;
        self.chunk_size = chunk_size;
        self.base_radius = base_radius;
        self.extended_radius = extended_radius;
        self.chunk_data = data;
        self.states.clear();
    }

    /// 停用 ChunkManager（退出关卡时调用）
    pub fn deinit(&mut self) {
        self.active = false;
        self.chunk_data.clear();
        self.states.clear();
    }
}

/// Chunk 插件
///
/// 注册 `ChunkManager` 资源和 chunk 管理系统。
/// 通过 `OnEnter(GameLevel::Demo)` 初始化区块数据。
pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            // Demo 关卡初始化 chunk 数据
            .add_systems(OnEnter(crate::level::GameLevel::Demo), init_demo_chunks)
            // 每帧更新 chunk 加载
            .add_systems(Update, update_chunks)
            // 退出关卡时清理
            .add_systems(OnExit(crate::level::GameLevel::Demo), deinit_chunks);
    }
}
