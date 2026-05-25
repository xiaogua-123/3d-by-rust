use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use ron::de::from_reader;
use crate::collision::CollisionShape;
use crate::collectible::Collectible;
use crate::combat::{AttackDamage, MoveSpeed};
use crate::config::GameplayConfig;
use crate::enemy::Enemy;
use crate::game_state::{LevelCollectibles, NextLevelEvent, StartGameEvent, GamePhase};
use crate::npc::{Npc, NpcConfig, NpcPatrol};
use crate::dialogue::DialogueTrigger;
use crate::player::Player;
use crate::toon::{ToonExtension, ToonOutline, ToonSettings};
use crate::td;

// ═══════════════════════════════════════════
// Zone 数据定义（RON 驱动）
// ═══════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDef {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub glb_scene: Option<String>,
    pub floor_size: f32,
    pub floor_color: (f32, f32, f32),
    pub spawn_point: (f32, f32, f32),
    #[serde(default)]
    pub npcs: Vec<ZoneNpcDef>,
    #[serde(default)]
    pub collectibles: Vec<(f32, f32, f32)>,
    #[serde(default)]
    pub enemies: Vec<ZoneEnemyDef>,
    #[serde(default)]
    pub transitions: Vec<ZoneTransitionDef>,
    #[serde(default)]
    pub walls: Vec<ZoneWallDef>,
    #[serde(default)]
    pub platforms: Vec<(f32, f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneNpcDef {
    pub display_name: String,
    pub conversation_id: String,
    pub start_node: String,
    pub position: (f32, f32, f32),
    pub color: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneEnemyDef {
    pub position: (f32, f32, f32),
    pub patrol: Vec<(f32, f32, f32)>,
    pub speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneTransitionDef {
    pub target_zone: String,
    pub trigger_pos: (f32, f32, f32),
    pub trigger_size: (f32, f32, f32),
    pub spawn_point: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneWallDef {
    pub position: (f32, f32, f32),
    pub scale: (f32, f32, f32),
}

#[derive(Resource, Default)]
pub struct ZoneBank {
    pub zones: HashMap<String, ZoneDef>,
}

type ToonMaterial = ExtendedMaterial<StandardMaterial, ToonExtension>;

/// 创建默认的 ToonMaterial
fn make_toon_material(
    materials: &mut Assets<ToonMaterial>,
    settings: &ToonSettings,
    base_color: Color,
    emissive: Option<Color>,
) -> Handle<ToonMaterial> {
    let ramp = settings.ramp_handle.clone().unwrap_or_default();
    materials.add(ToonMaterial {
        base: StandardMaterial {
            base_color,
            emissive: emissive.unwrap_or(Color::BLACK).into(),
            ..default()
        },
        extension: ToonExtension {
            ramp_texture: ramp,
            spec_threshold: settings.default_spec_threshold,
            spec_smoothness: settings.default_spec_smoothness,
            spec_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            toon_enabled: 1,
        },
    })
}


// --- Zone transition trigger component ---

#[derive(Component)]
struct ZoneTrigger {
    target_zone: GameLevel,
    spawn_point: Vec3,
}

/// 检测玩家进入 Zone 过渡区域
fn check_zone_transition(
    player_q: Query<&Transform, With<Player>>,
    trigger_q: Query<(&Transform, &CollisionShape, &ZoneTrigger)>,
    mut level_writer: MessageWriter<LoadLevelEvent>,
    config: Res<LevelConfig>,
) {
    let Ok(player_t) = player_q.single() else { return };

    for (trigger_t, shape, trigger) in trigger_q.iter() {
        if player_in_box(player_t.translation, trigger_t.translation, shape) {
            if trigger.target_zone != config.current_level {
                info!("区域过渡: {:?} → {:?}", config.current_level, trigger.target_zone);
                level_writer.write(LoadLevelEvent {
                    level: trigger.target_zone,
                    spawn_point: Some(trigger.spawn_point),
                });
                return;
            }
        }
    }
}

fn player_in_box(player_pos: Vec3, box_pos: Vec3, shape: &CollisionShape) -> bool {
    match shape {
        CollisionShape::Box { half_extents } => {
            let min = box_pos - *half_extents;
            let max = box_pos + *half_extents;
            player_pos.x >= min.x
                && player_pos.x <= max.x
                && player_pos.y >= min.y
                && player_pos.y <= max.y
                && player_pos.z >= min.z
                && player_pos.z <= max.z
        }
        _ => false,
    }
}

// --- Level state management ---

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameLevel {
    #[default]
    None,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

impl GameLevel {
    pub fn next(self) -> Option<GameLevel> {
        match self {
            GameLevel::None => Some(GameLevel::Level1),
            GameLevel::Level1 => Some(GameLevel::Level2),
            GameLevel::Level2 => Some(GameLevel::Level3),
            GameLevel::Level3 => Some(GameLevel::Level4),
            GameLevel::Level4 => Some(GameLevel::Level5),
            GameLevel::Level5 => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GameLevel::None => "",
            GameLevel::Level1 => "草原村",
            GameLevel::Level2 => "蓝色森林",
            GameLevel::Level3 => "黑暗废墟",
            GameLevel::Level4 => "城市",
            GameLevel::Level5 => "塔防试炼",
        }
    }

    pub fn zone_id(self) -> &'static str {
        match self {
            GameLevel::None => "",
            GameLevel::Level1 => "grassland",
            GameLevel::Level2 => "blue_forest",
            GameLevel::Level3 => "dark_ruins",
            GameLevel::Level4 => "city",
            GameLevel::Level5 => "tower_defense",
        }
    }

    pub fn from_zone_id(id: &str) -> Option<GameLevel> {
        match id {
            "grassland" => Some(GameLevel::Level1),
            "blue_forest" => Some(GameLevel::Level2),
            "dark_ruins" => Some(GameLevel::Level3),
            "city" => Some(GameLevel::Level4),
            "tower_defense" => Some(GameLevel::Level5),
            _ => None,
        }
    }
}

#[derive(Resource)]
pub struct LevelConfig {
    pub current_level: GameLevel,
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            current_level: GameLevel::None,
        }
    }
}

#[derive(Message)]
pub struct LoadLevelEvent {
    pub level: GameLevel,
    pub spawn_point: Option<Vec3>,
}

#[derive(Message)]
pub struct ResetPlayerEvent {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Default for ResetPlayerEvent {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        }
    }
}

#[derive(Component)]
pub struct LevelEntity;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLevel>()
            .init_resource::<LevelConfig>()
            .init_resource::<ZoneBank>()
            .add_message::<LoadLevelEvent>()
            .add_message::<ResetPlayerEvent>()
            .add_systems(Startup, load_zones)
            .add_systems(OnEnter(GameLevel::Level1), spawn_level1)
            .add_systems(OnEnter(GameLevel::Level2), spawn_level2)
            .add_systems(OnEnter(GameLevel::Level3), spawn_level3)
            .add_systems(OnEnter(GameLevel::Level4), spawn_level4)
            .add_systems(OnEnter(GameLevel::Level5), spawn_level5)
            .add_systems(OnExit(GameLevel::Level1), cleanup_level)
            .add_systems(OnExit(GameLevel::Level2), cleanup_level)
            .add_systems(OnExit(GameLevel::Level3), cleanup_level)
            .add_systems(OnExit(GameLevel::Level4), cleanup_level)
            .add_systems(OnExit(GameLevel::Level5), cleanup_level)
            .add_systems(OnExit(GameLevel::None), cleanup_level)
            // Clean up level when entering non-playing states
            .add_systems(OnEnter(GamePhase::GameOver), clear_level_state)
            .add_systems(OnEnter(GamePhase::MainMenu), clear_level_state)
            .add_systems(
                Update,
                (
                    handle_level_transition,
                    debug_level_switch,
                    check_collectibles_for_level_complete.run_if(in_state(crate::game_state::GamePhase::Playing)),
                    check_zone_transition.run_if(in_state(crate::game_state::GamePhase::Playing)),
                    handle_start_game_level,
                    handle_next_level_transition,
                ),
            );
    }
}

fn cleanup_level(mut commands: Commands, query: Query<Entity, With<LevelEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ═══════════════════════════════════════════
// Zone 加载系统
// ═══════════════════════════════════════════

fn load_zones(mut bank: ResMut<ZoneBank>) {
    let dir = "assets/zones";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                match fs::File::open(&path) {
                    Ok(file) => match from_reader::<_, ZoneDef>(file) {
                        Ok(zone) => {
                            info!("加载区域: {} ({})", zone.id, zone.display_name);
                            if let Some(glb) = &zone.glb_scene {
                                info!("  GLB 场景: {}", glb);
                            }
                            bank.zones.insert(zone.id.clone(), zone);
                        }
                        Err(e) => {
                            error!("解析区域文件失败 {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        error!("打开区域文件失败 {:?}: {}", path, e);
                    }
                }
            }
        }
    } else {
        let _ = fs::create_dir_all(dir);
        error!("区域目录不存在，已创建 assets/zones/，请放入 .ron 区域定义文件");
    }
    info!("共加载 {} 个区域", bank.zones.len());
}

fn spawn_zone(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_materials: &mut Assets<StandardMaterial>,
    materials: &mut Assets<ToonMaterial>,
    settings: &ToonSettings,
    config: &GameplayConfig,
    asset_server: &AssetServer,
    zone: &ZoneDef,
    collectibles: &mut LevelCollectibles,
) {
    // GLB 场景 或 程序化地面
    if let Some(glb_path) = &zone.glb_scene {
        // Bevy 0.18 要求 GLB 路径必须带 #Scene0 标签
        let path = if glb_path.contains('#') {
            glb_path.clone()
        } else {
            format!("{glb_path}#Scene0")
        };
        commands.spawn((
            SceneRoot(asset_server.load(path)),
            Transform::from_xyz(0.0, 200.0, 0.0).with_scale(Vec3::new(2.0, 2.0, 2.0)),
            LevelEntity,
            Name::new(format!("{}_GLB", zone.id)),
        ));
        // 地面碰撞
        commands.spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            CollisionShape::Plane { y: 0.0 },
            LevelEntity,
            Name::new(format!("{}_CollisionPlane", zone.id)),
        ));
    } else {
        let mat = make_toon_material(
            materials,
            settings,
            Color::srgb(zone.floor_color.0, zone.floor_color.1, zone.floor_color.2),
            None,
        );
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/terrain/{zone_id}_floor.glb#Scene0"))
        // 程序化 Plane 替换为带纹理的地面模型，保留 CollisionShape::Plane 碰撞
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(zone.floor_size, zone.floor_size))),
            MeshMaterial3d(mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
            CollisionShape::Plane { y: 0.0 },
            LevelEntity,
            Name::new(format!("{}_Floor", zone.id)),
        ));
    }

    // 平台
    for (i, (px, py, pz)) in zone.platforms.iter().enumerate() {
        let mat = make_toon_material(materials, settings, Color::srgb(0.6, 0.4, 0.2), None);
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/props/platform_{style}.glb#Scene0"))
        // 碰撞体保留 CollisionShape::Box，scale 从模型尺寸推导
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.0, 0.3, 3.0))),
            MeshMaterial3d(mat),
            Transform::from_xyz(*px, *py, *pz),
            CollisionShape::Box { half_extents: Vec3::new(1.5, 0.15, 1.5) },
            LevelEntity,
            Name::new(format!("{}_Platform_{}", zone.id, i)),
        ));
    }

    // 墙壁
    for (i, wall) in zone.walls.iter().enumerate() {
        let mat = make_toon_material(materials, settings, Color::srgb(0.5, 0.3, 0.1), None);
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/props/wall_{style}.glb#Scene0"))
        // 根据 zone 主题选择不同风格的墙壁模型
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(mat),
            Transform::from_xyz(wall.position.0, wall.position.1, wall.position.2)
                .with_scale(Vec3::new(wall.scale.0, wall.scale.1, wall.scale.2)),
            CollisionShape::Box { half_extents: Vec3::new(0.5, 0.5, 0.5) },
            LevelEntity,
            Name::new(format!("{}_Wall_{}", zone.id, i)),
        ));
    }

    // NPC
    for npc_def in &zone.npcs {
        let radius = 2.5;
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/characters/{npc_id}.glb#Scene0"))
        // 替换后 Npc/NpcConfig/DialogueTrigger 组件保留在 GLB 根实体上
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.6, 1.6, 0.6))),
            MeshMaterial3d(std_materials.add(StandardMaterial {
                base_color: Color::srgb(npc_def.color.0, npc_def.color.1, npc_def.color.2),
                ..default()
            })),
            Transform::from_xyz(npc_def.position.0, npc_def.position.1, npc_def.position.2),
            Npc,
            NpcConfig::stationary(&npc_def.display_name, &npc_def.conversation_id, &npc_def.start_node),
            NpcPatrol::default(),
            DialogueTrigger {
                conversation_id: npc_def.conversation_id.clone(),
                start_node: npc_def.start_node.clone(),
                radius,
            },
            LevelEntity,
            Name::new(format!("NPC_{}", npc_def.display_name)),
        ));
    }

    // 收集品
    collectibles.total = zone.collectibles.len() as u32;
    for (i, (px, py, pz)) in zone.collectibles.iter().enumerate() {
        let mat = make_toon_material(
            materials, settings,
            Color::srgb(1.0, 0.8, 0.0),
            Some(Color::srgb(0.5, 0.4, 0.0)),
        );
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/props/collectible_gem.glb#Scene0"))
        // 可选多种模型按 zone 主题区分: coin.glb, crystal.glb, star.glb
        // Collectible 组件和 ToonOutline 保留；动画由 animate_collectibles 系统驱动
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.3).mesh())),
            MeshMaterial3d(mat),
            Transform::from_xyz(*px, *py, *pz),
            Collectible { base_y: *py },
            ToonOutline,
            LevelEntity,
            Name::new(format!("{}_Collectible_{}", zone.id, i)),
        ));
    }

    // 敌人
    for (i, enemy_def) in zone.enemies.iter().enumerate() {
        let mat = make_toon_material(
            materials, settings,
            Color::srgb(1.0, 0.2, 0.2),
            Some(Color::srgb(0.3, 0.0, 0.0)),
        );
        let patrol: Vec<Vec3> = enemy_def.patrol.iter()
            .map(|(x, y, z)| Vec3::new(*x, *y, *z))
            .collect();
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/characters/enemy_{type}.glb#Scene0"))
        // 敌人类型: slime.glb / skeleton.glb / ghost.glb 等按 zone 主题
        // Enemy 组件和 ToonOutline 保留；巡逻由 enemy_movement 系统驱动
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
            MeshMaterial3d(mat),
            Transform::from_xyz(enemy_def.position.0, enemy_def.position.1, enemy_def.position.2),
            Enemy {
                patrol_points: patrol,
                current_target: 1,
                attack_cooldown: Timer::from_seconds(1.0, TimerMode::Once),
            },
            MoveSpeed(enemy_def.speed),
            AttackDamage(config.enemy_default_damage as f32),
            ToonOutline,
            LevelEntity,
            Name::new(format!("{}_Enemy_{}", zone.id, i)),
        ));
    }

    // Zone 过渡触发器
    for (_i, trans) in zone.transitions.iter().enumerate() {
        if let Some(target_level) = GameLevel::from_zone_id(&trans.target_zone) {
            commands.spawn((
                Transform::from_xyz(trans.trigger_pos.0, trans.trigger_pos.1, trans.trigger_pos.2),
                CollisionShape::Box {
                    half_extents: Vec3::new(
                        trans.trigger_size.0 / 2.0,
                        trans.trigger_size.1 / 2.0,
                        trans.trigger_size.2 / 2.0,
                    ),
                },
                ZoneTrigger {
                    target_zone: target_level,
                    spawn_point: Vec3::new(trans.spawn_point.0, trans.spawn_point.1, trans.spawn_point.2),
                },
                LevelEntity,
                Name::new(format!("{}_to_{}", zone.id, trans.target_zone)),
            ));
        } else {
            error!("未知目标区域: {}", trans.target_zone);
        }
    }

    info!("区域 {} ({}) 已加载 (收集品: {}, NPC: {}, 敌人: {})",
        zone.display_name, zone.id,
        zone.collectibles.len(),
        zone.npcs.len(),
        zone.enemies.len(),
    );
}

fn clear_level_state(
    mut level_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
) {
    config.current_level = GameLevel::None;
    level_state.set(GameLevel::None);
}

fn handle_level_transition(
    mut events: MessageReader<LoadLevelEvent>,
    mut next_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
    mut reset_writer: MessageWriter<ResetPlayerEvent>,
) {
    for ev in events.read() {
        config.current_level = ev.level;
        next_state.set(ev.level);
        reset_writer.write(ResetPlayerEvent {
            position: ev.spawn_point.unwrap_or(Vec3::new(0.0, 0.0, 0.0)),
            rotation: Quat::IDENTITY,
        });
    }
}

fn debug_level_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<LoadLevelEvent>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        events.write(LoadLevelEvent {
            level: GameLevel::Level1,
            spawn_point: None,
        });
        debug!("切换到关卡 1");
    }
    if keys.just_pressed(KeyCode::Digit2) {
        events.write(LoadLevelEvent {
            level: GameLevel::Level2,
            spawn_point: None,
        });
        debug!("切换到关卡 2");
    }
    if keys.just_pressed(KeyCode::Digit3) {
        events.write(LoadLevelEvent {
            level: GameLevel::Level3,
            spawn_point: None,
        });
        debug!("切换到关卡 3");
    }
    if keys.just_pressed(KeyCode::Digit4) {
        events.write(LoadLevelEvent {
            level: GameLevel::Level4,
            spawn_point: None,
        });
        debug!("切换到关卡 4");
    }
    if keys.just_pressed(KeyCode::Digit5) {
        events.write(LoadLevelEvent {
            level: GameLevel::Level5,
            spawn_point: None,
        });
        debug!("切换到关卡 5 (塔防试炼)");
    }
}

/// Check if all collectibles gathered → level complete
fn check_collectibles_for_level_complete(
    collectibles: Res<LevelCollectibles>,
    mut writer: MessageWriter<crate::game_state::LevelCompleteEvent>,
) {
    if collectibles.total > 0 && collectibles.collected >= collectibles.total {
        writer.write(crate::game_state::LevelCompleteEvent);
    }
}

/// StartGameEvent → set GameLevel to Level1
fn handle_start_game_level(
    mut events: MessageReader<StartGameEvent>,
    mut level_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
    mut reset_writer: MessageWriter<ResetPlayerEvent>,
) {
    for _ in events.read() {
        config.current_level = GameLevel::Level1;
        level_state.set(GameLevel::Level1);
        reset_writer.write(ResetPlayerEvent::default());
        info!("开始新游戏 - 加载关卡 1");
    }
}


/// NextLevelEvent → advance to next level
fn handle_next_level_transition(
    mut events: MessageReader<NextLevelEvent>,
    mut level_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
    mut reset_writer: MessageWriter<ResetPlayerEvent>,
    mut collectibles: ResMut<LevelCollectibles>,
) {
    for _ in events.read() {
        if let Some(next) = config.current_level.next() {
            config.current_level = next;
            level_state.set(next);
            collectibles.collected = 0;
            collectibles.total = 0;
            reset_writer.write(ResetPlayerEvent::default());
            info!("进入下一关: {:?}", next);
        } else {
            info!("已通关所有关卡! 返回主菜单");
        }
    }
}

// ========== Level 1 ==========

fn spawn_level1(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
    settings: Res<ToonSettings>,
    mut collectibles: ResMut<LevelCollectibles>,
    config: Res<GameplayConfig>,
    bank: Res<ZoneBank>,
    asset_server: Res<AssetServer>,
) {
    let Some(zone) = bank.zones.get(GameLevel::Level1.zone_id()) else {
        error!("找不到区域定义: grassland");
        return;
    };
    spawn_zone(
        &mut commands, &mut *meshes, &mut *std_materials, &mut *materials,
        &settings, &config, &asset_server, zone, &mut collectibles,
    );
}

// ========== Level 2 ==========

fn spawn_level2(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
    settings: Res<ToonSettings>,
    mut collectibles: ResMut<LevelCollectibles>,
    config: Res<GameplayConfig>,
    bank: Res<ZoneBank>,
    asset_server: Res<AssetServer>,
) {
    let Some(zone) = bank.zones.get(GameLevel::Level2.zone_id()) else {
        error!("找不到区域定义: blue_forest");
        return;
    };
    spawn_zone(
        &mut commands, &mut *meshes, &mut *std_materials, &mut *materials,
        &settings, &config, &asset_server, zone, &mut collectibles,
    );
}

// ========== Level 3 ==========

fn spawn_level3(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
    settings: Res<ToonSettings>,
    mut collectibles: ResMut<LevelCollectibles>,
    config: Res<GameplayConfig>,
    bank: Res<ZoneBank>,
    asset_server: Res<AssetServer>,
) {
    let Some(zone) = bank.zones.get(GameLevel::Level3.zone_id()) else {
        error!("找不到区域定义: dark_ruins");
        return;
    };
    spawn_zone(
        &mut commands, &mut *meshes, &mut *std_materials, &mut *materials,
        &settings, &config, &asset_server, zone, &mut collectibles,
    );
}

// ========== Level 5 (塔防试炼) ==========

fn spawn_level5(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<GameplayConfig>,
    td_config: Res<crate::td::TdWaveConfig>,
    mut td_gold: ResMut<crate::td::TdGold>,
    mut td_state: ResMut<crate::td::TdWaveState>,
    asset_server: Res<AssetServer>,
    level_def: Res<crate::td::level_data::TdLevelDef>,
) {
    td::level::spawn_td_level(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &config,
        &td_config,
        &mut td_gold,
        &mut td_state,
        &level_def,
    );
}

// ========== Level 4 (城市) ==========

fn spawn_level4(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
    settings: Res<ToonSettings>,
    mut collectibles: ResMut<LevelCollectibles>,
    config: Res<GameplayConfig>,
    bank: Res<ZoneBank>,
    asset_server: Res<AssetServer>,
) {
    let Some(zone) = bank.zones.get(GameLevel::Level4.zone_id()) else {
        error!("找不到区域定义: city");
        return;
    };
    spawn_zone(
        &mut commands, &mut *meshes, &mut *std_materials, &mut *materials,
        &settings, &config, &asset_server, zone, &mut collectibles,
    );
}
