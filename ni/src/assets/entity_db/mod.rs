//! 实体生成数据库 — 管理所有可生成实体的模板
//!
//! 从 `assets/data/*.ron` 加载实体模板，通过 `EntityRegistry` 管理。
//! 支持按名称查找模板、获取 GLB 模型句柄、批量生成实体。
//! 启动时预加载所有 GLB 模型，加载完成后切换到主菜单。

use bevy::prelude::*;

mod registry;


pub use registry::*;

use crate::game_state::GamePhase;

pub struct EntityDbPlugin;

impl Plugin for EntityDbPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityRegistry>()
            .init_resource::<GlbCache>()
            .init_resource::<SpawnQueue>()
            .init_resource::<AssetLoadProgress>()
            .add_systems(Startup, load_entity_db)
            .add_systems(Update, check_startup_loading.run_if(in_state(GamePhase::Loading)));
    }
}

/// 启动资源加载进度
#[derive(Resource, Default)]
pub struct AssetLoadProgress {
    pub loaded: u32,
    pub total: u32,
}

/// 每帧检查 GLB 资源加载状态，全部就绪后切换到主菜单
fn check_startup_loading(
    glb_cache: Res<GlbCache>,
    asset_server: Res<AssetServer>,
    mut progress: ResMut<AssetLoadProgress>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if glb_cache.handles.is_empty() {
        next_state.set(GamePhase::MainMenu);
        return;
    }

    let total = glb_cache.handles.len() as u32;
    let loaded = glb_cache
        .handles
        .values()
        .filter(|h| {
            matches!(
                asset_server.get_load_state(h.id()),
                Some(bevy::asset::LoadState::Loaded)
            )
        })
        .count() as u32;

    progress.loaded = loaded;
    progress.total = total;

    if loaded == total {
        info!("[Startup] 所有 {} 个 GLB 资源加载完成", total);
        next_state.set(GamePhase::MainMenu);
    }
}

/// 启动时从 assets/data/*.ron 加载实体模板
fn load_entity_db(
    mut registry: ResMut<EntityRegistry>,
    mut glb_cache: ResMut<GlbCache>,
    asset_server: Res<AssetServer>,
) {
    let paths = ["assets/data/entities.ron"];

    for path in &paths {
        let Ok(content) = std::fs::read_to_string(path) else {
            warn!("[EntityDB] 数据文件未找到: {}", path);
            continue;
        };

        let templates: Vec<EntityTemplate> = match ron::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                warn!("[EntityDB] 文件解析失败 ({}): {}", path, e);
                continue;
            }
        };

        for template in templates {
            // 预热 GLB 缓存：预加载所有模型的场景句柄
            if let Some(ref model) = template.model {
                glb_cache.handles.entry(model.clone()).or_insert_with(|| {
                    info!("[EntityDB] 预热 GLB: {}", model);
                    asset_server.load(model)
                });
            }
            info!("[EntityDB] 注册实体: {}", template.id);
            registry.templates.insert(template.id.clone(), template);
        }
    }

    info!(
        "[EntityDB] 加载完成: {} 个模板, {} 个 GLB 缓存",
        registry.templates.len(),
        glb_cache.handles.len()
    );
}
