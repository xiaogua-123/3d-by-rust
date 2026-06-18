//! 关卡管理 — 区域加载、切换、清理
//!
//! 定义关卡状态机 `GameLevel`、区域配置 `ZoneDef`、关卡资源 `LevelConfig`，
//! 以及区域生成/清理/切换系统。Narrative 区域由 `spawn_zone` 根据 `ZoneDef` 生成。

use bevy::prelude::*;
use bevy::animation::graph::{AnimationGraph, AnimationNodeIndex};
use crate::colliders::{Collider, ColliderShape, CollisionMask, CollisionResponse};
use crate::config::GameplayConfig;
use crate::entity_db::GlbCache;
use crate::game_state::{GamePhase, LevelCollectibles, StartGameEvent, NextLevelEvent};
use crate::proximity_loader::{ProximityModel, ProximityModels};
use crate::world_label::WorldLabel;

/// 标记残响体实体
#[derive(Component)]
pub struct Monster;

/// 标记残响体的动画已完成绑定
#[derive(Component)]
struct MonsterAnimated;

// ═══ 关卡状态 ═══

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameLevel {
    #[default]
    None,
    Demo,
    #[allow(dead_code)]
    Reception,
    #[allow(dead_code)]
    EastWing,
    #[allow(dead_code)]
    Courtyard,
    #[allow(dead_code)]
    WestWing,
    #[allow(dead_code)]
    Underground,
    #[allow(dead_code)]
    WalkTest,
    #[allow(dead_code)]
    MusicTest,
    #[allow(dead_code)]
    ParticleTest,
    #[allow(dead_code)]
    CollisionTest,
}

impl GameLevel {
    pub fn zone_id(self) -> &'static str {
        match self {
            GameLevel::None => "",
            GameLevel::Demo => "demo",
            GameLevel::Reception => "reception",
            GameLevel::EastWing => "east_wing",
            GameLevel::Courtyard => "courtyard",
            GameLevel::WestWing => "west_wing",
            GameLevel::Underground => "underground",
            GameLevel::WalkTest => "walk_test",
            GameLevel::MusicTest => "music_test",
            GameLevel::ParticleTest => "particle_test",
            GameLevel::CollisionTest => "collision_test",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GameLevel::None => "",
            GameLevel::Demo => "Demo 关卡",
            GameLevel::Reception => "接待区",
            GameLevel::EastWing => "东翼",
            GameLevel::Courtyard => "庭院",
            GameLevel::WestWing => "西翼",
            GameLevel::Underground => "地下层",
            GameLevel::WalkTest => "行走测试",
            GameLevel::MusicTest => "音乐测试",
            GameLevel::ParticleTest => "粒子测试",
            GameLevel::CollisionTest => "碰撞测试",
        }
    }

    pub fn next(self) -> Option<GameLevel> {
        match self {
            GameLevel::Demo => Some(GameLevel::Reception),
            GameLevel::Reception => Some(GameLevel::EastWing),
            GameLevel::EastWing => Some(GameLevel::Courtyard),
            GameLevel::Courtyard => Some(GameLevel::WestWing),
            GameLevel::WestWing => Some(GameLevel::Underground),
            _ => None,
        }
    }

    /// 从 zone_id 字符串解析为 GameLevel（用于存档加载）
    pub fn from_zone_id(id: &str) -> Option<GameLevel> {
        match id {
            "demo" => Some(GameLevel::Demo),
            "reception" => Some(GameLevel::Reception),
            "east_wing" => Some(GameLevel::EastWing),
            "courtyard" => Some(GameLevel::Courtyard),
            "west_wing" => Some(GameLevel::WestWing),
            "underground" => Some(GameLevel::Underground),
            "walk_test" => Some(GameLevel::WalkTest),
            "music_test" => Some(GameLevel::MusicTest),
            "particle_test" => Some(GameLevel::ParticleTest),
            "collision_test" => Some(GameLevel::CollisionTest),
            _ => None,
        }
    }
}

// ═══ 资源 ═══

#[derive(Resource, Default)]
pub struct LevelConfig {
    pub current_level: GameLevel,
}

#[derive(Clone)]
pub struct ZoneDef {
    pub id: String,
    pub display_name: String,
    pub glb_scene: Option<String>,
    pub floor_size: f32,
    pub floor_color: (f32, f32, f32),
    pub collectibles: Vec<Vec3>,
}

#[derive(Resource, Default)]
pub struct ZoneBank {
    pub zones: std::collections::HashMap<String, ZoneDef>,
}

#[derive(Resource, Default)]
pub struct ZoneGateMessage(pub Option<String>);

// ═══ 组件 ═══

#[derive(Component)]
pub struct LevelEntity;

// ═══ 事件 ═══

#[derive(Message)]
pub struct LoadLevelEvent {
    pub level: GameLevel,
}

#[derive(Message)]
pub struct ResetPlayerEvent {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Default for ResetPlayerEvent {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

// ═══ 插件 ═══

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLevel>()
            .init_resource::<LevelConfig>()
            .init_resource::<ZoneBank>()
            .init_resource::<ZoneGateMessage>()
            .add_message::<LoadLevelEvent>()
            .add_message::<ResetPlayerEvent>()
            .add_systems(Startup, load_zones)
            .add_systems(OnEnter(GameLevel::Reception), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::EastWing), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::Courtyard), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::WestWing), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::Underground), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::Demo), (spawn_narrative_zone, spawn_test_monster, spawn_all_preview_models))
            .add_systems(OnEnter(GameLevel::WalkTest), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::MusicTest), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::ParticleTest), spawn_narrative_zone)
            .add_systems(OnEnter(GameLevel::CollisionTest), spawn_narrative_zone)
            .add_systems(OnExit(GameLevel::None), cleanup_level)
            .add_systems(OnExit(GameLevel::Reception), cleanup_level)
            .add_systems(OnExit(GameLevel::EastWing), cleanup_level)
            .add_systems(OnExit(GameLevel::Courtyard), cleanup_level)
            .add_systems(OnExit(GameLevel::WestWing), cleanup_level)
            .add_systems(OnExit(GameLevel::Underground), cleanup_level)
            .add_systems(OnExit(GameLevel::Demo), (cleanup_level, cleanup_proximity_models))
            .add_systems(OnExit(GameLevel::WalkTest), cleanup_level)
            .add_systems(OnExit(GameLevel::MusicTest), cleanup_level)
            .add_systems(OnExit(GameLevel::ParticleTest), cleanup_level)
            .add_systems(OnExit(GameLevel::CollisionTest), cleanup_level)
            .add_systems(
                Update,
                (
                    handle_level_transition,
                    check_zone_transition,
                    zone_gate_message_clear,
                    handle_start_game_level,
                    handle_next_level_transition,
                    check_collectibles_for_level_complete,
                    play_monster_animation.run_if(resource_exists::<MonsterAnimation>),
                    start_monster_animation.run_if(resource_exists::<MonsterAnimation>),
                ),
            )
            .add_systems(OnEnter(GamePhase::GameOver), clear_level_state)
            .add_systems(OnEnter(GamePhase::MainMenu), clear_level_state)
            .add_systems(OnExit(GameLevel::Demo), |mut commands: Commands| {
                commands.remove_resource::<MonsterAnimation>();
            });
    }
}

// ═══ 区域加载 ═══

fn load_zones(mut bank: ResMut<ZoneBank>) {
    // Demo 区域
    bank.zones.insert("demo".into(), ZoneDef {
        id: "demo".into(),
        display_name: "Demo 关卡".into(),
        glb_scene: None,
        floor_size: 40.0,
        floor_color: (0.3, 0.35, 0.4),
        collectibles: vec![],
    });
    info!("区域配置已加载: {} 个区域", bank.zones.len());
}

// ═══ 区域生成系统 ═══

#[allow(clippy::too_many_arguments)]
fn spawn_narrative_zone(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    config: Res<GameplayConfig>,
    asset_server: Res<AssetServer>,
    glb_cache: Res<GlbCache>,
    level: Res<LevelConfig>,
    bank: Res<ZoneBank>,
    mut collectibles: ResMut<LevelCollectibles>,
) {
    let zone_id = level.current_level.zone_id();
    let Some(zone) = bank.zones.get(zone_id) else {
        warn!("未找到区域配置: {}", zone_id);
        return;
    };
    spawn_zone(
        &mut commands, &mut meshes, &mut std_materials,
        &config, &asset_server, &glb_cache,
        zone, &mut collectibles,
    );
}

/// 残响体动画资源
#[derive(Resource)]
pub struct MonsterAnimation {
    pub graph: Handle<AnimationGraph>,
    pub idle: AnimationNodeIndex,
}

/// 在 Demo 场景中生成残响体测试模型
fn spawn_test_monster(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // 加载动画片段
    let clip: Handle<AnimationClip> =
        asset_server.load("models/animations/npm_animations/scary_zombie_pack/zombie idle.glb#Animation0");
    let mut graph = AnimationGraph::new();
    let idle = graph.add_clip(clip, 1.0, graph.root);
    let graph_handle = graphs.add(graph);
    commands.insert_resource(MonsterAnimation { graph: graph_handle, idle });

    commands.spawn((
        SceneRoot(asset_server.load("models/animations/npm_animations/scary_zombie_pack/zombie idle.glb#Scene0")),
        Transform::from_xyz(2.0, 0.0, -5.0).with_scale(Vec3::splat(200.0)),
        LevelEntity,
        Monster,
        Name::new("残响体"),
    ));
    info!("残响体已放置到 Demo 场景");
}

/// Demo 场景中注册所有 GLB 预览模型到距离加载系统（靠近时才加载）
fn spawn_all_preview_models(
    mut proximity: ResMut<ProximityModels>,
) {
    // 清空上一轮的注册和追踪，避免重复
    proximity.models.clear();
    proximity.spawned.clear();

    /// 加载距离（米）— 玩家进入此范围内才加载 GLB
    const LOAD_DIST: f32 = 8.0;
    /// 卸载距离（米）— 玩家超出此范围时销毁 GLB 实体
    const UNLOAD_DIST: f32 = 16.0;

    // (id, path, name, x, z, scale)
    let models = [
        ("brain_stem",  "models/entity/BrainStem.glb#Scene0",          "玩家模型(BrainStem)", -7.0,  4.0, 1.0),
        ("phone",       "models/entity/1.glb#Scene0",           "电话机",               -3.5,  4.0, 1.0),
        ("severed_hand","models/entity/2.glb#Scene0",           "断手",                  0.0,  4.0, 1.0),
        ("locker",      "models/entity/3.glb#Scene0",           "储物柜",                3.5,  4.0, 1.0),
        ("gown",        "models/entity/4.glb#Scene0",           "病号服",               -7.0, -1.0, 1.0),
        ("zombie_mass", "models/entity/5.glb#Scene0",           "残响体·量产型",        -3.5, -1.0, 1.0),
        ("boss_resound","models/entity/6.glb#Scene0",           "首领·残响",            0.0, -1.0, 2.5),
        ("boss_black",  "models/entity/7.glb#Scene0",           "首领·黑涡",            3.5, -1.0, 3.0),
        ("surgery",     "models/entity/8.glb#Scene0",           "手术室场景",           -7.0, -6.0, 0.5),
        ("forceps",     "models/entity/9.glb#Scene0",           "手术钳",               -3.5, -6.0, 1.0),
        ("syringe",     "models/entity/10.glb#Scene0",          "注射器",                0.0, -6.0, 1.0),
        ("table",       "models/entity/11.glb#Scene0",          "手术台",                3.5, -6.0, 1.0),
        ("woman",       "models/entity/12.glb#Scene0",          "女性路人",             -7.0,-11.0, 1.0),
        ("scissors",    "models/entity/13.glb#Scene0",          "手术剪",               -3.5,-11.0, 1.0),
    ];
    for (id, path, name, x, z, scale) in &models {
        // 部分模型需要碰撞体（例如储物柜阻挡玩家穿过）
        let collider = match *id {
            "locker" => Some((
                ColliderShape::Box { half_extents: Vec3::new(0.3, 0.9, 0.3) },
                CollisionMask::terrain(),
                false,
                Vec3::ZERO,
            )),
            _ => None,
        };
        proximity.register(ProximityModel {
            id: id.to_string(),
            path: path.to_string(),
            position: Vec3::new(*x, 0.8, *z),
            scale: *scale,
            load_distance: LOAD_DIST,
            unload_distance: UNLOAD_DIST,
            label: Some((name.to_string(), 6.0)),
            collider,
        });
    }

    let anim_models = [
        ("walk",    "models/animations/Walk.glb#Scene0",     "行走动画",    -7.0, 9.0, 1.0),
        ("run",     "models/animations/Running.glb#Scene0",  "跑步动画",    -3.5, 9.0, 1.0),
        ("jump",    "models/animations/Jumping.glb#Scene0",  "跳跃动画",     0.0, 9.0, 1.0),
        ("big_jump","models/animations/Big_Jump.glb#Scene0", "大跳动画",     3.5, 9.0, 1.0),
    ];
    for (id, path, name, x, z, scale) in &anim_models {
        proximity.register(ProximityModel {
            id: id.to_string(),
            path: path.to_string(),
            position: Vec3::new(*x, 0.8, *z),
            scale: *scale,
            load_distance: LOAD_DIST,
            unload_distance: UNLOAD_DIST,
            label: Some((name.to_string(), 6.0)),
            collider: None,
        });
    }

    // ═══ 新下载的小地图测试模型展示区（z=12~15，x=±6） ═══
    let minimap_models = [
        // (id, path, name, x, z, scale)
        ("mini_virtual_city", "models/VirtualCity.glb#Scene0",     "🏙 小地图-虚拟城市",   -7.0, 14.0, 0.5),
        ("mini_floating_01",  "models/Floating_Island_01.glb#Scene0","🏝 浮空岛1",          -2.5, 14.0, 0.8),
        ("mini_floating_02",  "models/Floating_Island_02.glb#Scene0","🏝 浮空岛2",           2.5, 14.0, 0.8),
        ("mini_cesium_man",   "models/CesiumMan.glb#Scene0",       "🧑 角色-玩家标记",      -6.0, 12.0, 1.0),
        ("mini_toy_car",      "models/ToyCar.glb#Scene0",          "🚗 载具-车辆标记",      -2.0, 12.0, 0.8),
        ("mini_lantern",      "models/Lantern.glb#Scene0",         "🏮 POI-灯笼兴趣点",      2.0, 12.0, 0.6),
        ("mini_avocado",      "models/Avocado.glb#Scene0",         "🥑 测试对象-Avocado",    6.0, 12.0, 0.8),
        ("mini_helmet",       "models/DamagedHelmet.glb#Scene0",   "⛑ 测试对象-头盔",       6.0, 14.0, 0.5),
        ("mini_bench",        "models/Bench.glb#Scene0",           "🪑 POI-长椅",           -6.0, 15.5, 0.8),
        ("mini_deer",         "models/Deer.glb#Scene0",            "🦌 野生动物-鹿",        -2.0, 15.5, 0.8),
        ("mini_butterfly",    "models/Butterfly.glb#Scene0",       "🦋 野生动物-蝴蝶",       2.0, 15.5, 0.6),
        ("mini_xyz_tri",      "models/xyz_Triangulon.glb#Scene0",  "👾 敌人标记-三角龙",    -6.0, 17.0, 0.6),
        ("mini_xyz_chick",    "models/xyz_Pentachick.glb#Scene0",  "👾 敌人标记-五角鸡",    -2.0, 17.0, 0.6),
        ("mini_xyz_star",     "models/xyz_Starplant.glb#Scene0",   "👾 敌人标记-星星草",     2.0, 17.0, 0.6),
        ("mini_xyz_bear",     "models/xyz_Hexabear.glb#Scene0",    "👾 敌人标记-六角熊",     6.0, 17.0, 0.6),
        ("mini_xyz_vguy",     "models/xyz_Vguy.glb#Scene0",        "👾 敌人标记-V型小人",    6.0, 15.5, 0.6),
    ];
    for (id, path, name, x, z, scale) in &minimap_models {
        proximity.register(ProximityModel {
            id: id.to_string(),
            path: path.to_string(),
            position: Vec3::new(*x, 0.8, *z),
            scale: *scale,
            load_distance: LOAD_DIST,
            unload_distance: UNLOAD_DIST,
            label: Some((name.to_string(), 4.0)),
            collider: None,
        });
    }

    info!(
        "已注册 {} 个距离加载模型（含 {} 个小地图测试模型）",
        models.len() + anim_models.len() + minimap_models.len(),
        minimap_models.len(),
    );
}

/// 残响体生成后绑定动画图（每帧重试直到场景加载完成）
fn play_monster_animation(
    monster_q: Query<Entity, (With<Monster>, Without<MonsterAnimated>)>,
    children_q: Query<&Children>,
    anim_player_q: Query<Entity, With<AnimationPlayer>>,
    anim: Res<MonsterAnimation>,
    mut commands: Commands,
) {
    for entity in &monster_q {
        let Ok(children) = children_q.get(entity) else { return };
        let mut stack: Vec<Entity> = children.to_vec();
        let mut i = 0;
        while i < stack.len() {
            let child = stack[i];
            i += 1;
            if anim_player_q.contains(child) {
                commands.entity(child).insert((
                    AnimationGraphHandle(anim.graph.clone()),
                ));
                commands.entity(entity).insert(MonsterAnimated);
                info!("残响体动画图已绑定");
                return;
            }
            if let Ok(grandkids) = children_q.get(child) {
                stack.extend(grandkids.to_vec());
            }
        }
    }
}

/// 等 AnimationGraphHandle 就绪后主动播放空闲动画
fn start_monster_animation(
    monster_q: Query<Entity, With<MonsterAnimated>>,
    children_q: Query<&Children>,
    mut anim_player_q: Query<(&mut AnimationPlayer, Option<&AnimationGraphHandle>)>,
    anim: Res<MonsterAnimation>,
) {
    for entity in &monster_q {
        let Ok(children) = children_q.get(entity) else { return };
        let mut stack: Vec<Entity> = children.to_vec();
        let mut i = 0;
        while i < stack.len() {
            let child = stack[i];
            i += 1;
            if let Ok((mut player, handle)) = anim_player_q.get_mut(child) {
                if handle.is_some() && !player.is_playing_animation(anim.idle) {
                    player.play(anim.idle).repeat();
                    info!("残响体动画已开始播放");
                }
                return;
            }
            if let Ok(grandkids) = children_q.get(child) {
                stack.extend(grandkids.to_vec());
            }
        }
    }
}

/// 生成区域场景（只接受 GLB 模型）
#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
fn spawn_zone(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_materials: &mut Assets<StandardMaterial>,
    config: &GameplayConfig,
    asset_server: &AssetServer,
    glb_cache: &GlbCache,
    zone: &ZoneDef,
    collectibles: &mut LevelCollectibles,
) {
    // 始终生成地面碰撞平面（不依赖 GLB 场景）
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::ground(0.0),
        CollisionResponse::kinematic(),
        LevelEntity,
        Name::new(format!("{}_CollisionPlane", zone.id)),
    ));

    let Some(glb_path) = &zone.glb_scene else {
        // 无 GLB 时生成纹理地面
        info!("区域 {} 无 GLB 场景，生成纹理地面", zone.id);
        let floor_texture: Handle<Image> =
            asset_server.load("textures/rusty_metal_04_4k/textures/rusty_metal_04_diff_4k.jpg");
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(zone.floor_size, zone.floor_size))),
            MeshMaterial3d(std_materials.add(StandardMaterial {
                base_color_texture: Some(floor_texture),
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
            LevelEntity,
            Name::new(format!("{}_Floor", zone.id)),
        ));
        return;
    };

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
        WorldLabel::new(&zone.display_name).with_offset(6.0).with_font_size(16.0),
    ));

    info!("区域 {} 已加载", zone.display_name);
}

// ═══ 关卡切换事件处理 ═══

fn handle_level_transition(
    mut events: MessageReader<LoadLevelEvent>,
    mut level_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
) {
    for ev in events.read() {
        config.current_level = ev.level;
        level_state.set(ev.level);
        info!("切换到关卡: {}", ev.level.display_name());
    }
}

fn handle_start_game_level(
    mut events: MessageReader<StartGameEvent>,
    mut level_state: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
) {
    for _ in events.read() {
        config.current_level = GameLevel::Demo;
        level_state.set(GameLevel::Demo);
    }
}

fn handle_next_level_transition(
    mut events: MessageReader<NextLevelEvent>,
    level_state: Res<State<GameLevel>>,
    mut next_level: ResMut<NextState<GameLevel>>,
    mut config: ResMut<LevelConfig>,
) {
    for _ in events.read() {
        if let Some(next) = level_state.get().next() {
            config.current_level = next;
            next_level.set(next);
            info!("进入下一关: {}", next.display_name());
        }
    }
}

fn check_zone_transition(
    _player: Query<&Transform, With<crate::player::Player>>,
    mut _gate_msg: ResMut<ZoneGateMessage>,
    _keys: Res<ButtonInput<KeyCode>>,
    _level: Res<LevelConfig>,
    _bank: Res<ZoneBank>,
    _commands: Commands,
) {
    // 由用户自行实现区域传送门检测
}

fn zone_gate_message_clear(
    _gate_msg: ResMut<ZoneGateMessage>,
    _time: Res<Time>,
    mut _timer: Local<f32>,
) {
    // 由用户自行实现提示文字自动消失
}

fn check_collectibles_for_level_complete(
    collectibles: Res<LevelCollectibles>,
    mut writer: MessageWriter<crate::game_state::LevelCompleteEvent>,
) {
    if collectibles.total > 0 && collectibles.collected >= collectibles.total {
        writer.write(crate::game_state::LevelCompleteEvent);
    }
}

// ═══ 清理 ═══

fn cleanup_level(
    mut commands: Commands,
    entities: Query<Entity, With<LevelEntity>>,
) {
    for e in &entities {
        commands.entity(e).despawn();
    }
    info!("关卡场景已清理");
}

fn clear_level_state(
    mut config: ResMut<LevelConfig>,
    mut level_state: ResMut<NextState<GameLevel>>,
) {
    config.current_level = GameLevel::None;
    level_state.set(GameLevel::None);
}

fn cleanup_proximity_models(mut proximity: ResMut<ProximityModels>) {
    proximity.spawned.clear();
    info!("ProximityModels 追踪已清理");
}
