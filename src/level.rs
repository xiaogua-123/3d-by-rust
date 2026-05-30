use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use ron::de::from_reader;
use crate::collision::CollisionShape;
use crate::colliders::{Collider, ColliderShape, CollisionMask, CollisionResponse};
use crate::collectible::Collectible;
use crate::combat::{AttackDamage, MoveSpeed};
use crate::config::GameplayConfig;
use crate::enemy::Enemy;
use crate::game_state::{LevelCollectibles, NextLevelEvent, StartGameEvent, GamePhase};
use crate::npc::{Npc, NpcConfig, NpcPatrol};
use crate::dialogue::DialogueTrigger;
use crate::player::Player;
use crate::solari_demo;
use crate::td;

/// 从模型标识符构造 GLB 场景加载路径
fn npc_model_path(id: &str) -> String {
    format!("models/{id}.glb#Scene0")
}

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
    pub collectibles: Vec<CollectibleDef>,
    #[serde(default)]
    pub enemies: Vec<ZoneEnemyDef>,
    #[serde(default)]
    pub transitions: Vec<ZoneTransitionDef>,
    #[serde(default)]
    pub walls: Vec<ZoneWallDef>,
    #[serde(default)]
    pub platforms: Vec<PlatformDef>,
}

fn default_scale() -> (f32, f32, f32) {
    (1.0, 1.0, 1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneNpcDef {
    pub display_name: String,
    pub conversation_id: String,
    pub start_node: String,
    pub position: (f32, f32, f32),
    pub color: (f32, f32, f32),
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_scale")]
    pub scale: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneEnemyDef {
    pub position: (f32, f32, f32),
    pub patrol: Vec<(f32, f32, f32)>,
    pub speed: f32,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_scale")]
    pub scale: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectibleDef {
    pub position: (f32, f32, f32),
    #[serde(default = "default_scale")]
    pub scale: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDef {
    pub position: (f32, f32, f32),
    #[serde(default = "default_scale")]
    pub scale: (f32, f32, f32),
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
    Solari,
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
            GameLevel::Solari => None,
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
            GameLevel::Solari => "光追演示",
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
            GameLevel::Solari => "solari",
        }
    }

    pub fn from_zone_id(id: &str) -> Option<GameLevel> {
        match id {
            "grassland" => Some(GameLevel::Level1),
            "blue_forest" => Some(GameLevel::Level2),
            "dark_ruins" => Some(GameLevel::Level3),
            "city" => Some(GameLevel::Level4),
            "tower_defense" => Some(GameLevel::Level5),
            "solari" => Some(GameLevel::Solari),
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

/// 进入 Solari 前的关卡，用于退出时恢复
#[derive(Resource, Default)]
struct PreviousLevel(Option<GameLevel>);

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLevel>()
            .init_resource::<LevelConfig>()
            .init_resource::<ZoneBank>()
            .init_resource::<PreviousLevel>()
            .add_message::<LoadLevelEvent>()
            .add_message::<ResetPlayerEvent>()
            .add_systems(Startup, load_zones)
            .add_systems(OnEnter(GameLevel::Level1), spawn_level1)
            .add_systems(OnEnter(GameLevel::Level2), spawn_level2)
            .add_systems(OnEnter(GameLevel::Level3), spawn_level3)
            .add_systems(OnEnter(GameLevel::Level4), spawn_level4)
            .add_systems(OnEnter(GameLevel::Level5), spawn_level5)
            .add_systems(OnEnter(GameLevel::Solari), enter_solari)
            .add_systems(OnExit(GameLevel::Level1), cleanup_level)
            .add_systems(OnExit(GameLevel::Level2), cleanup_level)
            .add_systems(OnExit(GameLevel::Level3), cleanup_level)
            .add_systems(OnExit(GameLevel::Level4), cleanup_level)
            .add_systems(OnExit(GameLevel::Level5), cleanup_level)
            .add_systems(OnExit(GameLevel::None), cleanup_level)
            .add_systems(OnExit(GameLevel::Solari), exit_solari)
            // Solari 动画：展厅物体旋转
            .add_systems(
                Update,
                solari_demo::animate_solari.run_if(in_state(GameLevel::Solari)),
            )
            // Clean up level when entering non-playing states
            .add_systems(OnEnter(GamePhase::GameOver), clear_level_state)
            .add_systems(OnEnter(GamePhase::MainMenu), clear_level_state)
            .add_systems(
                Update,
                (
                    handle_level_transition,
                    debug_level_switch,
                    solari_level_toggle,
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
            Transform::from_xyz(0.0, 0.8, 0.0).with_scale(Vec3::new(54.0, 54.0, 54.0)),
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
        let mat = std_materials.add(StandardMaterial {
            base_color: Color::srgb(zone.floor_color.0, zone.floor_color.1, zone.floor_color.2),
            ..default()
        });
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
    for (i, plat) in zone.platforms.iter().enumerate() {
        let mat = std_materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.4, 0.2),
            ..default()
        });
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/props/platform_{style}.glb#Scene0"))
        // 碰撞体保留 CollisionShape::Box，scale 从模型尺寸推导
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.0, 0.3, 3.0))),
            MeshMaterial3d(mat),
            Transform::from_xyz(plat.position.0, plat.position.1, plat.position.2)
                .with_scale(Vec3::new(plat.scale.0, plat.scale.1, plat.scale.2)),
            CollisionShape::Box { half_extents: Vec3::new(1.5, 0.15, 1.5) },
            LevelEntity,
            Name::new(format!("{}_Platform_{}", zone.id, i)),
        ));
    }

    // 墙壁
    for (i, wall) in zone.walls.iter().enumerate() {
        let mat = std_materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.3, 0.1),
            ..default()
        });
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

    // NPC（指定 model 则使用 GLB 角色模型，否则使用程序化方块）
    for npc_def in &zone.npcs {
        let radius = 2.5;
        let mut entity = commands.spawn((
            Transform::from_xyz(npc_def.position.0, npc_def.position.1, npc_def.position.2)
                .with_scale(Vec3::new(npc_def.scale.0, npc_def.scale.1, npc_def.scale.2)),
            Npc,
            NpcConfig::stationary(&npc_def.display_name, &npc_def.conversation_id, &npc_def.start_node),
            NpcPatrol::default(),
            DialogueTrigger {
                conversation_id: npc_def.conversation_id.clone(),
                start_node: npc_def.start_node.clone(),
                radius,
            },
            // 统一碰撞体系统
            Collider::sphere(0.5, CollisionMask::npc()),
            CollisionResponse::kinematic(),
            LevelEntity,
            Name::new(format!("NPC_{}", npc_def.display_name)),
        ));
        if let Some(model_id) = &npc_def.model {
            entity.insert(SceneRoot(asset_server.load(npc_model_path(model_id))));
        } else {
            entity.insert((
                Mesh3d(meshes.add(Cuboid::new(0.6, 1.6, 0.6))),
                MeshMaterial3d(std_materials.add(StandardMaterial {
                    base_color: Color::srgb(npc_def.color.0, npc_def.color.1, npc_def.color.2),
                    ..default()
                })),
            ));
        }
    }

    // 收集品
    collectibles.total = zone.collectibles.len() as u32;
    for (i, col) in zone.collectibles.iter().enumerate() {
        let mat = std_materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.0),
            emissive: Color::srgb(0.5, 0.4, 0.0).into(),
            ..default()
        });
        // TODO: GLB替换 → SceneRoot(asset_server.load("models/props/collectible_gem.glb#Scene0"))
        // 可选多种模型按 zone 主题区分: coin.glb, crystal.glb, star.glb
        // Collectible 组件保留；动画由 animate_collectibles 系统驱动
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.3).mesh())),
            MeshMaterial3d(mat),
            Transform::from_xyz(col.position.0, col.position.1, col.position.2)
                .with_scale(Vec3::new(col.scale.0, col.scale.1, col.scale.2)),
            Collectible { base_y: col.position.1 },
            // 统一碰撞体系统（触发器）
            Collider::trigger(
                ColliderShape::Sphere { radius: 0.3 },
                CollisionMask::collectible(),
            ),
            LevelEntity,
            Name::new(format!("{}_Collectible_{}", zone.id, i)),
        ));
    }

    // 敌人（指定 model 则使用 GLB 角色模型，否则使用程序化红色方块）
    for (i, enemy_def) in zone.enemies.iter().enumerate() {
        let patrol: Vec<Vec3> = enemy_def.patrol.iter()
            .map(|(x, y, z)| Vec3::new(*x, *y, *z))
            .collect();
        let mut entity = commands.spawn((
            Transform::from_xyz(enemy_def.position.0, enemy_def.position.1, enemy_def.position.2)
                .with_scale(Vec3::new(enemy_def.scale.0, enemy_def.scale.1, enemy_def.scale.2)),
            Enemy {
                patrol_points: patrol,
                current_target: 1,
                attack_cooldown: Timer::from_seconds(1.0, TimerMode::Once),
            },
            MoveSpeed(enemy_def.speed),
            AttackDamage(config.enemy_default_damage as f32),
            // 统一碰撞体系统
            Collider::sphere(0.4, CollisionMask::enemy()),
            CollisionResponse::default(),
            LevelEntity,
            Name::new(format!("{}_Enemy_{}", zone.id, i)),
        ));
        if let Some(model_id) = &enemy_def.model {
            entity.insert(SceneRoot(asset_server.load(npc_model_path(model_id))));
        } else {
            let mat = std_materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.2, 0.2),
                emissive: Color::srgb(0.3, 0.0, 0.0).into(),
                ..default()
            });
            entity.insert((
                Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
                MeshMaterial3d(mat),
            ));
        }
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

// ========== Solari 实时光追关卡 ==========

/// 6 键切换进出 Solari 关卡
fn solari_level_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<LevelConfig>,
    mut level_writer: MessageWriter<LoadLevelEvent>,
    mut previous: ResMut<PreviousLevel>,
    phase: Res<State<GamePhase>>,
) {
    if !keys.just_pressed(KeyCode::Digit6) {
        return;
    }
    // 只在游戏中有效
    if phase.get() != &GamePhase::Playing {
        return;
    }

    match config.current_level {
        GameLevel::Solari => {
            // 退出 Solari，回到之前的关卡
            let target = previous.0.unwrap_or(GameLevel::Level1);
            info!("退出光追演示，返回 {:?}", target);
            level_writer.write(LoadLevelEvent {
                level: target,
                spawn_point: None,
            });
        }
        _ => {
            // 进入 Solari — 先保存当前关卡，再发送事件
            previous.0 = Some(config.current_level);
            info!("进入光追演示关卡 (来自 {:?})", previous.0);
            level_writer.write(LoadLevelEvent {
                level: GameLevel::Solari,
                spawn_point: None,
            });
        }
    }
}

fn enter_solari(
    mut commands: Commands,
    player_q: Query<Entity, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bank: Res<ZoneBank>,
) {
    // 移除玩家实体（含子实体：相机、手电筒、模型）
    if let Ok(player_entity) = player_q.single() {
        commands.entity(player_entity).despawn();
    }

    // 从 zone 数据生成碰撞平面和过渡触发器
    if let Some(zone) = bank.zones.get("solari") {
        // 碰撞平面
        commands.spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            CollisionShape::Plane { y: 0.0 },
            LevelEntity,
            Name::new("solari_Collision"),
        ));

        // 过渡触发器
        for trans in &zone.transitions {
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
                    Name::new(format!("solari_to_{}", trans.target_zone)),
                ));
            }
        }
    }

    // 生成 PBR 材质展厅
    solari_demo::spawn_pbr_showcase(&mut commands, &mut *meshes, &mut *materials);
}

fn exit_solari(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<crate::player::PlayerSettings>,
    solari_objects: Query<Entity, With<LevelEntity>>,
) {
    // 清理 Solari 场景物体
    for entity in solari_objects.iter() {
        commands.entity(entity).despawn();
    }
    // 重新生成玩家
    crate::player::spawn_player(commands, assets, settings);
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
        &mut commands, &mut *meshes, &mut *std_materials,
        &config, &asset_server, zone, &mut collectibles,
    );
}

// ========== Level 2 ==========

fn spawn_level2(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
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
        &mut commands, &mut *meshes, &mut *std_materials,
        &config, &asset_server, zone, &mut collectibles,
    );
}

// ========== Level 3 ==========

fn spawn_level3(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
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
        &mut commands, &mut *meshes, &mut *std_materials,
        &config, &asset_server, zone, &mut collectibles,
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
        &mut commands, &mut *meshes, &mut *std_materials,
        &config, &asset_server, zone, &mut collectibles,
    );
}
