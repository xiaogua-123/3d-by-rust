// ═══════════════════════════════════════════
// 游戏数据库：从 RON 加载的静态数值表
// ═══════════════════════════════════════════

use bevy::prelude::*;
use serde::Deserialize;

/// 炮塔数值定义（从 assets/game_data.ron 加载）
#[derive(Deserialize, Clone, Debug)]
pub struct TurretDef {
    pub id: String,
    #[allow(dead_code)]
    pub name: String,
    pub cost: u32,
    pub range: f32,
    pub damage: f32,
    pub fire_rate: f32,
    pub color: (f32, f32, f32),
}

impl TurretDef {
    pub fn to_color(&self) -> Color {
        Color::srgb(self.color.0, self.color.1, self.color.2)
    }
}

/// 敌人数值定义（从 assets/game_data.ron 加载）
#[derive(Deserialize, Clone, Debug)]
pub struct EnemyDef {
    pub id: String,
    #[allow(dead_code)]
    pub name: String,
    pub health: f32,
    pub speed: f32,
    pub damage: f32,
    pub gold: u32,
    pub size: f32,
    pub color: (f32, f32, f32),
}

impl EnemyDef {
    pub fn to_color(&self) -> Color {
        Color::srgb(self.color.0, self.color.1, self.color.2)
    }
}

/// 游戏数值数据库
#[derive(Resource, Deserialize, Clone, Debug)]
pub struct GameDatabase {
    pub turrets: Vec<TurretDef>,
    pub enemies: Vec<EnemyDef>,
}

impl GameDatabase {
    pub fn find_turret(&self, id: &str) -> Option<&TurretDef> {
        self.turrets.iter().find(|t| t.id == id)
    }

    pub fn find_enemy(&self, id: &str) -> Option<&EnemyDef> {
        self.enemies.iter().find(|e| e.id == id)
    }
}

/// 在游戏启动时同步加载数据库（避免异步等待问题）
pub fn load_game_database(mut commands: Commands) {
    let ron_str = include_str!("../../assets/game_data.ron");
    match ron::de::from_str::<GameDatabase>(ron_str) {
        Ok(db) => {
            info!("游戏数据库已加载: {} 炮塔, {} 敌人", db.turrets.len(), db.enemies.len());
            commands.insert_resource(db);
        }
        Err(e) => {
            error!("加载 game_data.ron 失败: {}", e);
            // 插入空数据库作为后备，让游戏不至于崩溃
            commands.insert_resource(GameDatabase {
                turrets: Vec::new(),
                enemies: Vec::new(),
            });
        }
    }
}
