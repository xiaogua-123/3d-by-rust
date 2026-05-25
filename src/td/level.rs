// ═══════════════════════════════════════════
// TD 关卡生成 + 游戏结束检测
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::config::GameplayConfig;
use crate::game_state::{GamePhase, Score};
use crate::level::LevelEntity;
use super::data::*;
use super::events::{TdDefeatEvent, TdVictoryEvent};
use super::level_data::TdLevelDef;

pub fn td_check_game_end(
    core_q: Query<&DefenseCore>,
    mut defeat_writer: MessageWriter<TdDefeatEvent>,
) {
    if let Ok(core) = core_q.single() {
        if core.current_health <= 0.0 {
            defeat_writer.write(TdDefeatEvent);
        }
    }
}

pub fn td_check_victory(
    state: Res<TdWaveState>,
    mut victory_writer: MessageWriter<TdVictoryEvent>,
    mut victory_sent: Local<bool>,
) {
    if state.phase == WavePhase::Complete && state.enemies_alive == 0 && !*victory_sent {
        victory_writer.write(TdVictoryEvent);
        *victory_sent = true;
    }
}

pub fn td_handle_victory(
    mut events: MessageReader<TdVictoryEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
    mut score: ResMut<Score>,
    gold: Res<TdGold>,
) {
    for _ in events.read() {
        score.0 += gold.0 * 10;
        info!("塔防胜利! 最终分数: {}", score.0);
        phase.set(GamePhase::LevelComplete);
    }
}

pub fn td_check_defeat(
    mut events: MessageReader<TdDefeatEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    for _ in events.read() {
        info!("塔防失败! 防御核心被摧毁!");
        phase.set(GamePhase::GameOver);
    }
}

pub fn spawn_td_level(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    _asset_server: &AssetServer,
    _config: &GameplayConfig,
    td_config: &TdWaveConfig,
    gold: &mut TdGold,
    wave_state: &mut TdWaveState,
    level_def: &TdLevelDef,
) {
    gold.0 = td_config.starting_gold;
    *wave_state = TdWaveState::default();
    wave_state.wave_timer = Timer::from_seconds(td_config.wave_cooldown, TimerMode::Once);

    let arena = level_def.arena_size;

    // 地面
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.3, 0.35),
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(arena, arena))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        crate::collision::CollisionShape::Plane { y: 0.0 },
        LevelEntity,
        Name::new("TD_Floor"),
    ));

    // 网格线
    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.35, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for i in -4..=4 {
        let x = i as f32 * (arena / 10.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 0.01, arena))),
            MeshMaterial3d(grid_mat.clone()),
            Transform::from_xyz(x, 0.005, 0.0),
            LevelEntity,
            Name::new(format!("GridLine_X_{}", i)),
        ));
        let z = i as f32 * (arena / 10.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(arena, 0.01, 0.05))),
            MeshMaterial3d(grid_mat.clone()),
            Transform::from_xyz(0.0, 0.005, z),
            LevelEntity,
            Name::new(format!("GridLine_Z_{}", i)),
        ));
    }

    // 防御核心
    let core_color = Color::srgb(0.2, 0.6, 1.0);
    let core_mat = materials.add(StandardMaterial {
        base_color: core_color,
        emissive: core_color.into(),
        ..default()
    });
    let core_h = level_def.core.height;
    let core_r = level_def.core.radius;
    let core_pos = level_def.core.position;

    commands
        .spawn((
            Transform::from_xyz(core_pos.0, core_pos.1, core_pos.2),
            DefenseCore {
                max_health: level_def.core.max_health,
                current_health: level_def.core.max_health,
            },
            LevelEntity,
            Name::new("DefenseCore"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cone::new(core_r, core_h))),
                MeshMaterial3d(core_mat.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Name::new("CoreCrystal"),
            ));
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

    // 敌人生成点
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
            Name::new(format!("SpawnPoint_{}", i)),
        ));
    }

    // 障碍物
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
            crate::collision::CollisionShape::Box {
                half_extents: Vec3::new(0.5 * sx, 0.5 * sy, 0.5 * sz),
            },
            LevelEntity,
            Name::new(format!("TD_Obstacle_{}", i)),
        ));
    }

    // 光源
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

    info!(
        "塔防试炼已加载! 初始金币: {}, 最大波次: {}",
        td_config.starting_gold, td_config.max_waves
    );
}
