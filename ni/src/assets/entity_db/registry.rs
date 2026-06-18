//! 实体数据库 — 核心类型定义
//!
//! 定义 `EntityTemplate`（实体模板）、`EntityCategory`（分类）、
//! `EntityRegistry`（注册表）、`GlbCache`（GLB 模型缓存）、
//! `SpawnQueue`（生成队列）和 `SpawnCommand`（生成指令）。

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 实体模板（从 RON 文件反序列化） ──

/// 实体模板 — 定义一种可生成实体的所有静态属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTemplate {
    pub id: String,
    /// 中文显示名，用于 UI、调试信息等
    #[serde(default)]
    pub display_name: String,
    pub category: EntityCategory,
    /// GLB 模型路径，如 "models/5.glb#Scene0"
    /// None 表示使用原始几何体（立方体/球体等）回退
    pub model: Option<String>,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 图标路径（PNG 格式），None 时用分类色块占位
    #[serde(default)]
    pub icon: Option<String>,
}

fn default_scale() -> f32 {
    1.0
}

/// 实体分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EntityCategory {
    Enemy,
    Npc,
    Collectible,
    Prop,
    Projectile,
    StressNpc,
}

// ── 运行时资源 ──

/// 实体注册表 — 存储所有已加载的实体模板
#[derive(Resource, Default)]
pub struct EntityRegistry {
    pub templates: HashMap<String, EntityTemplate>,
}

impl EntityRegistry {
    pub fn get(&self, id: &str) -> Option<&EntityTemplate> {
        self.templates.get(id)
    }

    /// 按标签筛选模板 ID 列表
    pub fn find_by_tag(&self, tag: &str) -> Vec<&EntityTemplate> {
        self.templates
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .collect()
    }
}

/// GLB 场景句柄缓存 — 避免重复加载相同模型
#[derive(Resource, Default)]
pub struct GlbCache {
    pub handles: HashMap<String, Handle<Scene>>,
}

/// 生成队列 — 解耦生成请求与实际生成
#[derive(Resource, Default)]
pub struct SpawnQueue {
    pub pending: Vec<SpawnCommand>,
}

/// 生成命令 — 描述一次实体生成请求
pub struct SpawnCommand {
    pub template_id: String,
    pub position: Vec3,
}
