//! 塔防关卡生成与胜负判定
//!
//! 生成 TD 竞技场（地板、网格、核心、出生点、障碍物、灯光），
//! 构建导航网格，检测胜利/失败条件并触发对应事件。

use bevy::prelude::*;
use crate::config::GameplayConfig;
use crate::game_state::{GamePhase, Score};
use crate::level::LevelEntity;
use crate::nav_mesh::TdNavMesh;
use super::data::*;
use super::events::{TdDefeatEvent, TdVictoryEvent};
use super::level_data::TdLevelDef;
use crate::world_label::WorldLabel;

/// 检测防御核心生命值，判断是否失败
/// 核心血量 ≤ 0 时发送失败事件
pub fn td_check_game_end(
    core_q: Query<&DefenseCore>,                    // 查询防御核心
    mut defeat_writer: MessageWriter<TdDefeatEvent>,// 失败事件发送器
) {
    if let Ok(core) = core_q.single() {
        // 核心被摧毁，触发失败
        if core.current_health <= 0.0 {
            defeat_writer.write(TdDefeatEvent);
        }
    }
}

/// 检测胜利条件
/// 所有波次完成 + 场上无存活敌人 → 胜利
pub fn td_check_victory(
    state: Res<TdWaveState>,                       // 塔防波次状态
    mut victory_writer: MessageWriter<TdVictoryEvent>,// 胜利事件发送器
    mut victory_sent: Local<bool>,                 // 本地标记：防止重复发送胜利
) {
    if state.phase == WavePhase::Complete && state.enemies_alive == 0 && !*victory_sent {
        victory_writer.write(TdVictoryEvent);
        *victory_sent = true;
    }
}

/// 处理胜利事件
/// 加分、打印日志、切换到关卡完成界面
pub fn td_handle_victory(
    mut events: MessageReader<TdVictoryEvent>,
    mut phase: ResMut<NextState<GamePhase>>,       // 游戏状态切换
    mut score: ResMut<Score>,                      // 总分
    gold: Res<TdGold>,                             // 当前金币（用于计算奖励）
) {
    for _ in events.read() {
        score.0 += gold.0 * 10;                     // 金币 * 10 作为奖励分数
        info!("塔防胜利! 最终分数: {}", score.0);
        phase.set(GamePhase::LevelComplete);        // 切换到胜利界面
    }
}

/// 处理失败事件
/// 打印日志、切换到游戏结束界面
pub fn td_check_defeat(
    mut events: MessageReader<TdDefeatEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        info!("塔防失败! 防御核心被摧毁!");
        phase.set(GamePhase::GameOver);            // 切换到失败界面
    }
}

/// 生成整个塔防关卡
/// 包括：地面、网格、防御核心、敌人生成点、障碍物、灯光
#[allow(dead_code, clippy::too_many_arguments)]
pub fn spawn_td_level(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,                     // 网格资源
    materials: &mut Assets<StandardMaterial>,      // 材质资源
    _asset_server: &AssetServer,
    _config: &GameplayConfig,
    td_config: &TdWaveConfig,                      // 塔防配置（金币、波次冷却等）
    gold: &mut TdGold,                             // 金币资源
    wave_state: &mut TdWaveState,                  // 波次状态
    level_def: &TdLevelDef,                        // 关卡数据定义
) {
    // 初始化金币与波次状态
    gold.0 = td_config.starting_gold;
    *wave_state = TdWaveState::default();
    wave_state.wave_timer = Timer::from_seconds(td_config.wave_cooldown, TimerMode::Once);

    let arena = level_def.arena_size;              // 竞技场大小

    // TODO(GLBi): 所有程序化几何体 → GLB 模型
    //   参考: docs/zrx____________________游戏文档.md §6、§9、§10
    //   资产: 地下层·残响核心(E区)主题 — 残响体、手术台、铁架病床
    //   通用原则: 模型不存在时保持正方体占位

    // ====================== 地面 ======================
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.3, 0.35),
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(arena, arena))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        crate::collision::Collider::ground(0.0),
        LevelEntity,
        Name::new("Arena"),
    ));

    // TODO(GLBi): 地面 Plane → GLB 竞技场场景
    //   参考: 游戏文档 §10 — 档案室塔防(地下层·残响核心)
    //   程序化地面 + 网格线 → models/td/arena_{theme}.glb#Scene0

    // ====================== 网格辅助线 ======================
    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.35, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for i in -4..=4 {
        // X 方向网格线
        let x = i as f32 * (arena / 10.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 0.01, arena))),
            MeshMaterial3d(grid_mat.clone()),
            Transform::from_xyz(x, 0.005, 0.0),
            LevelEntity,
            Name::new(format!("GridLine_X_{}", i)),
        ));
        // Z 方向网格线
        let z = i as f32 * (arena / 10.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(arena, 0.01, 0.05))),
            MeshMaterial3d(grid_mat.clone()),
            Transform::from_xyz(0.0, 0.005, z),
            LevelEntity,
            Name::new(format!("GridLine_Z_{}", i)),
        ));
    }

    // ====================== 防御核心 ======================
    // TODO(GLBi): 核心 Cone+Torus → GLB 3D 模型
    //   参考: 游戏文档 §6.5 — 回音石(600-800面, 掌心大小, 内部彩色光晕)
    //   路径: models/td/defense_core.glb#Scene0
    //   子实体: 核心水晶 + 光环圆环 + 血条 → 合并为完整模型
    let core_color = Color::srgb(0.2, 0.6, 1.0);
    let core_mat = materials.add(StandardMaterial {
        base_color: core_color,
        emissive: core_color.into(),
        ..default()
    });
    let core_h = level_def.core.height;
    let core_r = level_def.core.radius;
    let core_pos = level_def.core.position;

    // 生成核心实体（带血条组件+名字标签）
    commands
        .spawn((
            Transform::from_xyz(core_pos.0, core_pos.1, core_pos.2),
            DefenseCore {
                max_health: level_def.core.max_health,
                current_health: level_def.core.max_health,
            },
            LevelEntity,
            Name::new("DefenseCore"),
            WorldLabel::new("防御核心").with_offset(3.5).with_font_size(15.0),
        ))
        .with_children(|parent| {
            // 核心锥形水晶
            parent.spawn((
                Mesh3d(meshes.add(Cone::new(core_r, core_h))),
                MeshMaterial3d(core_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Name::new("CoreCrystal"),
            ));
            // 核心光环圆环
            parent.spawn((
                Mesh3d(meshes.add(Torus::new(0.8, 0.05))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.7, 1.0),
                    emissive: Color::srgb(0.3, 0.7, 1.0).into(),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("CoreRing"),
            ));
            // 血条背景
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.0, 0.15, 0.15))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.2, 0.2),
                    ..default()
                })),
                Transform::from_xyz(0.0, 2.0, 0.0),
                Name::new("CoreHealthBg"),
            ));
        });

    // ====================== 敌人生成点标记 ======================
    // TODO(GLBi): 生成点 Cuboid → GLB 传送门/裂痕模型
    //   参考: 游戏文档 §6.1 — 残响体穿越裂隙出现
    //   路径: models/td/spawn_rift.glb#Scene0
    let spawn_marker_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.2),
        emissive: Color::srgb(0.3, 0.0, 0.0).into(),
        ..default()
    });

    for (i, sp) in level_def.spawn_points.iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.3, 0.3))),
            MeshMaterial3d(spawn_marker_mat.clone()),
            Transform::from_xyz(sp.position.0, sp.position.1, sp.position.2),
            SpawnPoint {
                direction: Vec3::new(sp.direction.0, sp.direction.1, sp.direction.2),
            },
            LevelEntity,
            Name::new(format!("SpawnRift_{}", i)),
        ));
    }

    // ====================== 障碍物/墙体 ======================
    // TODO(GLBi): 障碍物 Cuboid → GLB 环境道具
    //   参考: 游戏文档 §6.6 — 手术台(8000-12000面)、§6.7 — 铁架病床(4000-6000面)
    //   §6.2 — 储物柜(8000-12000面)
    //   路径: models/props/{locker|bed|operating_table}.glb#Scene0
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.35, 0.3),
        ..default()
    });

    for (i, obs) in level_def.obstacles.iter().enumerate() {
        let (sx, sy, sz) = obs.scale;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(obs.position.0, obs.position.1, obs.position.2)
                .with_scale(Vec3::new(sx, sy, sz)),
            crate::collision::Collider::new(
                crate::collision::ColliderShape::Box {
                    half_extents: Vec3::new(0.5 * sx, 0.5 * sy, 0.5 * sz),
                },
                crate::collision::CollisionMask::terrain(),
            ),
            LevelEntity,
            Name::new(format!("SceneProp_{}", i)),
        ));
    }

    // ====================== 核心点光源 ======================
    commands.spawn((
        PointLight {
            color: Color::srgb(0.3, 0.6, 1.0),
            intensity: 50000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
        LevelEntity,
        Name::new("CoreLight"),
    ));

    // 加载完成日志
    info!(
        "塔防试炼已加载! 初始金币: {}, 最大波次: {}",
        td_config.starting_gold, td_config.max_waves
    );
}

/// 根据竞技场尺寸和障碍物构建导航网格
pub fn td_build_navmesh(
    level_def: Res<TdLevelDef>,
    mut commands: Commands,
) {
    let nav_mesh = TdNavMesh::build(level_def.arena_size, &level_def.obstacles);
    commands.insert_resource(nav_mesh);
}