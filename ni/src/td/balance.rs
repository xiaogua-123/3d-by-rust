//! 塔防数值平衡配置
//!
//! 从 RON 文件加载炮塔定义（`TurretDef`）、敌人定义（`EnemyDef`）、
//! 整体配置（`TdConfig`）和难度缩放规则。提供 `GameDatabase` 资源。

use bevy::prelude::*;
use serde::Deserialize;

// ── 炮塔/敌人数据库（game_data.ron） ──

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

// ═══════════════════════════════════════════
// 塔防数值调参数据库（td_config.ron）
// ═══════════════════════════════════════════

/// 波次配置
#[derive(Deserialize, Clone, Debug)]
pub struct WaveConfig {
    pub max_waves: u32,
    pub enemies_per_wave_base: u32,
    pub enemies_per_wave_growth: u32,
    pub spawn_interval: f32,
    pub wave_cooldown: f32,
    pub starting_gold: u32,
}

/// 炮塔全局参数
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct TurretGlobalConfig {
    pub rotation_speed: f32,
    pub projectile_speed: f32,
    pub projectile_lifetime: f32,
    pub target_search_interval: f32,
}

/// 经济系统
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct EconomyConfig {
    pub gold_interest_rate: f32,
    pub gold_per_wave_bonus: u32,
    pub turret_refund_ratio: f32,
}

/// 防御核心
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct CoreConfig {
    pub max_health: f32,
}

/// 难度缩放
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct DifficultyScaling {
    pub health_mult_per_wave: f32,
    pub speed_mult_per_wave: f32,
    pub damage_mult_per_wave: f32,
    pub gold_mult_per_wave: f32,
}

/// 完整塔防调参数据库（从 assets/td_config.ron 加载）
#[derive(Resource, Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct TdConfig {
    pub wave: WaveConfig,
    pub turret_global: TurretGlobalConfig,
    pub economy: EconomyConfig,
    pub core: CoreConfig,
    pub scaling: DifficultyScaling,
}

/// 加载塔防调参数据库
pub fn load_td_config(mut commands: Commands) {
    let ron_str = include_str!("../../assets/td_config.ron");
    match ron::de::from_str::<TdConfig>(ron_str) {
        Ok(config) => {
            info!("塔防调参数据库已加载");
            commands.insert_resource(config);
        }
        Err(e) => {
            error!("加载 td_config.ron 失败: {}", e);
        }
    }
}

/// 将 TdConfig 中的波次参数同步到 TdWaveConfig
pub fn sync_wave_config(
    td_config: Res<TdConfig>,
    mut wave_config: ResMut<crate::td::data::TdWaveConfig>,
) {
    wave_config.max_waves = td_config.wave.max_waves;
    wave_config.enemies_per_wave_base = td_config.wave.enemies_per_wave_base;
    wave_config.enemies_per_wave_growth = td_config.wave.enemies_per_wave_growth;
    wave_config.spawn_interval = td_config.wave.spawn_interval;
    wave_config.wave_cooldown = td_config.wave.wave_cooldown;
    wave_config.starting_gold = td_config.wave.starting_gold;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turret_def_to_color() {
        let turret = TurretDef {
            id: "test".into(),
            name: "Test".into(),
            cost: 100,
            range: 5.0,
            damage: 10.0,
            fire_rate: 1.0,
            color: (0.5, 0.5, 0.5),
        };
        let color = turret.to_color();
        assert!((color.to_srgba().red - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_enemy_def_to_color() {
        let enemy = EnemyDef {
            id: "test".into(),
            name: "Test".into(),
            health: 100.0,
            speed: 2.0,
            damage: 5.0,
            gold: 10,
            size: 1.0,
            color: (1.0, 0.0, 0.0),
        };
        let color = enemy.to_color();
        assert!((color.to_srgba().red - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_find_turret_nonexistent() {
        let db = GameDatabase {
            turrets: vec![],
            enemies: vec![],
        };
        assert!(db.find_turret("nonexistent").is_none());
    }

    #[test]
    fn test_find_turret_exists() {
        let db = GameDatabase {
            turrets: vec![TurretDef {
                id: "basic".into(),
                name: "Basic".into(),
                cost: 100,
                range: 5.0,
                damage: 10.0,
                fire_rate: 1.0,
                color: (0.0, 0.0, 1.0),
            }],
            enemies: vec![],
        };
        assert!(db.find_turret("basic").is_some());
        assert_eq!(db.find_turret("basic").unwrap().cost, 100);
    }

    #[test]
    fn test_find_enemy() {
        let db = GameDatabase {
            turrets: vec![],
            enemies: vec![EnemyDef {
                id: "zombie".into(),
                name: "Zombie".into(),
                health: 50.0,
                speed: 1.0,
                damage: 5.0,
                gold: 5,
                size: 1.0,
                color: (0.5, 0.5, 0.5),
            }],
        };
        assert!(db.find_enemy("zombie").is_some());
        assert!(db.find_enemy("nonexistent").is_none());
    }

    #[test]
    fn test_game_data_ron_parses() {
        let ron_str = include_str!("../../assets/game_data.ron");
        let result = ron::de::from_str::<GameDatabase>(ron_str);
        assert!(result.is_ok(), "game_data.ron 解析失败: {:?}", result.err());
        let db = result.unwrap();
        assert!(!db.turrets.is_empty(), "应至少有一个炮塔");
        assert!(!db.enemies.is_empty(), "应至少有一个敌人");
    }

    #[test]
    fn test_sync_wave_config() {
        use bevy::app::App;
        let td_config = TdConfig {
            wave: WaveConfig {
                max_waves: 20,
                enemies_per_wave_base: 5,
                enemies_per_wave_growth: 2,
                spawn_interval: 1.5,
                wave_cooldown: 30.0,
                starting_gold: 200,
            },
            turret_global: TurretGlobalConfig {
                rotation_speed: 2.0,
                projectile_speed: 10.0,
                projectile_lifetime: 3.0,
                target_search_interval: 0.5,
            },
            economy: EconomyConfig {
                gold_interest_rate: 0.05,
                gold_per_wave_bonus: 50,
                turret_refund_ratio: 0.7,
            },
            core: CoreConfig { max_health: 100.0 },
            scaling: DifficultyScaling {
                health_mult_per_wave: 1.1,
                speed_mult_per_wave: 1.05,
                damage_mult_per_wave: 1.08,
                gold_mult_per_wave: 1.15,
            },
        };
        let mut app = App::new();
        app.insert_resource(td_config);
        app.insert_resource(crate::td::data::TdWaveConfig::default());
        app.add_systems(Update, sync_wave_config);
        app.update();

        let wave_config = app.world().resource::<crate::td::data::TdWaveConfig>();
        assert_eq!(wave_config.max_waves, 20);
        assert_eq!(wave_config.enemies_per_wave_base, 5);
        assert_eq!(wave_config.enemies_per_wave_growth, 2);
        assert!((wave_config.spawn_interval - 1.5).abs() < 0.01);
        assert!((wave_config.wave_cooldown - 30.0).abs() < 0.01);
        assert_eq!(wave_config.starting_gold, 200);
    }

    #[test]
    fn test_td_config_ron_parses() {
        let ron_str = include_str!("../../assets/td_config.ron");
        let result = ron::de::from_str::<TdConfig>(ron_str);
        assert!(result.is_ok(), "td_config.ron 解析失败: {:?}", result.err());
        let config = result.unwrap();
        assert!(config.wave.max_waves > 0, "最大波次应大于0");
        assert!(config.wave.starting_gold > 0, "初始金币应大于0");
        assert!(config.turret_global.projectile_speed > 0.0, "子弹速度应大于0");
        assert!(config.core.max_health > 0.0, "核心血量应大于0");
    }
}
