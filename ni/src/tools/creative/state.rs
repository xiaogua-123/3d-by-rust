//! 创造模式 — 组件与资源定义

use bevy::prelude::*;
use crate::entity_db::EntityCategory;

/// 创造模式下放置的物体标记
#[derive(Component)]
pub struct CreativePlacedItem {
    pub template_id: String,
    #[allow(dead_code)]
    pub saved: bool,
}

/// 幽灵预览标记
#[derive(Component)]
pub struct CreativeGhost;

/// 创造模式状态
#[derive(Resource)]
pub struct CreativeState {
    pub selected_slot: usize,
    pub category_index: usize,
    pub current_items: Vec<String>,
    pub current_item_names: Vec<String>,
    pub current_item_categories: Vec<EntityCategory>,
    pub categories: Vec<String>,
    pub category_items: Vec<Vec<String>>,
    pub ghost_entity: Option<Entity>,
    pub ghost_material: Option<Handle<StandardMaterial>>,
    pub ghost_mesh: Option<Handle<Mesh>>,
    pub grid_snap: bool,
    pub show_labels: bool,
    pub show_level: bool,
    pub dirty: bool,
    pub camera_entity: Option<Entity>,
    pub next_id: u64,
}

impl Default for CreativeState {
    fn default() -> Self {
        Self {
            selected_slot: 0,
            category_index: 0,
            current_items: Vec::new(),
            current_item_names: Vec::new(),
            current_item_categories: Vec::new(),
            categories: vec!["道具".into(), "NPC".into(), "敌人".into(), "收集品".into()],
            category_items: vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            ghost_entity: None,
            ghost_material: None,
            ghost_mesh: None,
            grid_snap: false,
            show_labels: true,
            show_level: true,
            dirty: false,
            camera_entity: None,
            next_id: 0,
        }
    }
}
