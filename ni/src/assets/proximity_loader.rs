//! 近距离加载系统 — 玩家靠近时才加载 GLB 模型，远离时自动卸载
//!
//! 将模型注册到 `ProximityModels`，系统每帧检查玩家距离：
//! - 进入 `load_distance` → 开始加载并生成实体
//! - 超出 `unload_distance` → 销毁实体释放资源

use bevy::prelude::*;
use std::collections::HashMap;

use crate::colliders::{Collider, ColliderShape, CollisionMask};
use crate::level::LevelEntity;
use crate::player::Player;
use crate::world_label::WorldLabel;

/// 定义一个按玩家距离自动加载/卸载的模型
pub struct ProximityModel {
    /// 唯一标识符
    pub id: String,
    /// GLB 路径，如 "models/1.glb#Scene0"
    pub path: String,
    /// 生成位置
    pub position: Vec3,
    /// 统一缩放
    pub scale: f32,
    /// 进入该距离（米）时开始加载
    pub load_distance: f32,
    /// 超出该距离（米）时卸载，应 > load_distance 避免频繁闪烁
    pub unload_distance: f32,
    /// 头顶标签 (文字, 偏移高度)
    pub label: Option<(String, f32)>,
    /// 碰撞体：可选 (碰撞形状, 碰撞掩码, 是否为触发器, 局部偏移)
    pub collider: Option<(ColliderShape, CollisionMask, bool, Vec3)>,
}

/// 近距离加载资源 — 管理待加载模型和已生成实体的映射
#[derive(Resource, Default)]
pub struct ProximityModels {
    /// 所有待/已加载的模型定义
    pub models: Vec<ProximityModel>,
    /// 已生成的实体 <model_id, entity>
    pub spawned: HashMap<String, Entity>,
}

impl ProximityModels {
    /// 注册一个按距离加载的模型
    pub fn register(&mut self, model: ProximityModel) {
        self.models.push(model);
    }
}

pub struct ProximityLoaderPlugin;

impl Plugin for ProximityLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProximityModels>()
            .add_systems(Update, proximity_loader_system);
    }
}

/// 待生成模型（替代复杂元组，clippy type_complexity）
struct PendingSpawn {
    id: String,
    path: String,
    position: Vec3,
    scale: f32,
    label: Option<(String, f32)>,
    collider: Option<(ColliderShape, CollisionMask, bool, Vec3)>,
}

/// 核心系统：每帧检查玩家与所有注册模型的距离，管理加载和卸载
fn proximity_loader_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_q: Query<&Transform, With<Player>>,
    mut models: ResMut<ProximityModels>,
) {
    let Ok(player_t) = player_q.single() else {
        return;
    };

    let mut spawn_batch: Vec<PendingSpawn> = Vec::new();
    let mut despawn_batch: Vec<String> = Vec::new();

    // 第一遍：只收集操作，不修改 resource
    for model in models.models.iter() {
        let dist = player_t.translation.distance(model.position);
        let already_spawned = models.spawned.contains_key(&model.id);

        if !already_spawned && dist <= model.load_distance {
            spawn_batch.push(PendingSpawn {
                id: model.id.clone(),
                path: model.path.clone(),
                position: model.position,
                scale: model.scale,
                label: model.label.clone(),
                collider: model.collider.clone(),
            });
        } else if already_spawned && dist > model.unload_distance {
            despawn_batch.push(model.id.clone());
        }
    }

    // 第二遍：执行生成
    for item in spawn_batch {
        let handle = asset_server.load::<Scene>(&item.path);
        let mut entity_cmd = commands.spawn((
            SceneRoot(handle),
            Transform::from_translation(item.position).with_scale(Vec3::splat(item.scale)),
            LevelEntity,
            Name::new(format!("Proximity_{}", item.id)),
        ));
        if let Some((text, offset)) = item.label {
            entity_cmd.insert(WorldLabel::new(text).with_offset(offset));
        }
        // 添加碰撞体（如果有配置）
        if let Some((shape, mask, is_trigger, offset)) = item.collider {
            if offset == Vec3::ZERO {
                // 无偏移：碰撞体直接放在根实体
                if is_trigger {
                    entity_cmd.insert(Collider::trigger(shape, mask));
                } else {
                    entity_cmd.insert(Collider::new(shape, mask));
                }
            } else {
                // 有偏移：用子实体承载碰撞体
                entity_cmd.with_child((
                    if is_trigger {
                        Collider::trigger(shape, mask)
                    } else {
                        Collider::new(shape, mask)
                    },
                    Transform::from_translation(offset),
                ));
            }
        }
        models.spawned.insert(item.id, entity_cmd.id());
    }

    // 第三遍：执行卸载
    for id in despawn_batch {
        if let Some(entity) = models.spawned.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}
