// ═══════════════════════════════════════════
// 关卡数据定义：从 assets/td_level.ron 加载
// ═══════════════════════════════════════════

use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct CoreDef {
    pub position: (f32, f32, f32),
    pub height: f32,
    pub radius: f32,
    pub max_health: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SpawnPointDef {
    pub position: (f32, f32, f32),
    pub direction: (f32, f32, f32),
}

#[derive(Deserialize, Clone, Debug)]
pub struct ObstacleDef {
    pub position: (f32, f32, f32),
    pub scale: (f32, f32, f32),
}

#[derive(Resource, Deserialize, Clone, Debug)]
pub struct TdLevelDef {
    pub arena_size: f32,
    pub core: CoreDef,
    pub spawn_points: Vec<SpawnPointDef>,
    pub obstacles: Vec<ObstacleDef>,
}

pub fn load_td_level(mut commands: Commands) {
    let ron_str = include_str!("../../assets/td_level.ron");
    match ron::de::from_str::<TdLevelDef>(ron_str) {
        Ok(def) => {
            info!(
                "关卡配置已加载: arena={}, {} 生成点, {} 障碍物",
                def.arena_size,
                def.spawn_points.len(),
                def.obstacles.len()
            );
            commands.insert_resource(def);
        }
        Err(e) => {
            error!("加载 td_level.ron 失败: {}", e);
            commands.insert_resource(TdLevelDef::default());
        }
    }
}

impl Default for TdLevelDef {
    fn default() -> Self {
        Self {
            arena_size: 50.0,
            core: CoreDef {
                position: (0.0, 0.75, 0.0),
                height: 1.5,
                radius: 0.5,
                max_health: 100.0,
            },
            spawn_points: vec![
                SpawnPointDef { position: (23.0, 0.2, 0.0), direction: (-1.0, 0.0, 0.0) },
                SpawnPointDef { position: (-23.0, 0.2, 0.0), direction: (1.0, 0.0, 0.0) },
                SpawnPointDef { position: (0.0, 0.2, 23.0), direction: (0.0, 0.0, -1.0) },
                SpawnPointDef { position: (0.0, 0.2, -23.0), direction: (0.0, 0.0, 1.0) },
                SpawnPointDef { position: (23.0, 0.2, 23.0), direction: (-0.707, 0.0, -0.707) },
                SpawnPointDef { position: (-23.0, 0.2, 23.0), direction: (0.707, 0.0, -0.707) },
                SpawnPointDef { position: (23.0, 0.2, -23.0), direction: (-0.707, 0.0, 0.707) },
                SpawnPointDef { position: (-23.0, 0.2, -23.0), direction: (0.707, 0.0, 0.707) },
            ],
            obstacles: vec![
                ObstacleDef { position: (8.0, 0.8, 8.0), scale: (3.0, 1.6, 0.3) },
                ObstacleDef { position: (-8.0, 0.8, -8.0), scale: (3.0, 1.6, 0.3) },
                ObstacleDef { position: (8.0, 0.8, -8.0), scale: (3.0, 1.6, 0.3) },
                ObstacleDef { position: (-8.0, 0.8, 8.0), scale: (3.0, 1.6, 0.3) },
                ObstacleDef { position: (16.0, 0.6, 0.0), scale: (0.3, 1.2, 4.0) },
                ObstacleDef { position: (-16.0, 0.6, 0.0), scale: (0.3, 1.2, 4.0) },
                ObstacleDef { position: (0.0, 0.6, 16.0), scale: (4.0, 1.2, 0.3) },
                ObstacleDef { position: (0.0, 0.6, -16.0), scale: (4.0, 1.2, 0.3) },
            ],
        }
    }
}
