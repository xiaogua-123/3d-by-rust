//! 空间网格与 Bevy ECS 集成
//!
//! 提供 `TdGridObject` 组件、`TdGridResource` 资源和 `TdGridPlugin`，
//! 将 `SpatialGrid` 作为 Bevy 资源自动管理实体同步。

use std::collections::HashMap;
use bevy::prelude::*;
use bevy::math::Vec2;
use crate::td::spatial::core::SpatialGrid;
use crate::td::spatial::entry::{EntityCategory, GridEntry};
/// 标记为塔防网格中的实体
#[derive(Component)]
pub struct TdGridObject {
    #[allow(dead_code)]
    pub category: EntityCategory,
    pub radius: f32,
}

/// 塔防空间网格资源
///
/// 手动管理插入和删除（在生成/销毁实体时调用），
/// 避免自动同步带来的 1 帧延迟。
#[derive(Resource, Default)]
pub struct TdGridResource {
    pub grid: SpatialGrid<Entity>,
    /// 追踪数据：last_pos, radius（独立于 ECS 组件，销毁时仍可读取）
    data: HashMap<Entity, (Vec2, f32)>,
}

impl TdGridResource {
    /// 手动插入实体到网格（在生成敌人/炮塔时调用）
    pub fn insert_entity(&mut self, entity: Entity, category: EntityCategory, pos: Vec2, radius: f32) {
        self.grid.insert(GridEntry::new(entity, category, pos, radius));
        self.data.insert(entity, (pos, radius));
    }

    /// 手动更新实体位置（在敌人移动时调用）
    pub fn update_entity(&mut self, entity: Entity, category: EntityCategory, old_pos: Vec2, new_pos: Vec2, radius: f32) {
        if old_pos != new_pos {
            self.grid.update(&entity, old_pos, new_pos, radius, category);
            self.data.insert(entity, (new_pos, radius));
        }
    }

    /// 手动从网格移除实体（在敌人死亡时调用）
    pub fn remove_entity(&mut self, entity: &Entity) {
        if let Some((pos, radius)) = self.data.remove(entity) {
            self.grid.remove(entity, pos, radius);
        }
    }
}

pub struct TdGridPlugin;

impl Plugin for TdGridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TdGridResource>();
    }
}
