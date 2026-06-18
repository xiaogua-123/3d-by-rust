//! 塔防波次管理器
//!
//! 状态机驱动波次生命周期：`Idle → Spawning → Active → BetweenWaves`。
//! 按配置生成敌人组合，支持多波次自动推进。

use bevy::prelude::*;
use crate::combat::{AttackDamage, Health, MoveSpeed};
use crate::config::GameplayConfig;
use crate::level::LevelEntity;
use crate::nav_mesh::TdNavMesh;
use crate::pathfinding::NavPath;
use super::data::*;
use super::enemy::TdGridPos;
use super::events::StartNextWaveEvent;
use super::balance::GameDatabase;
use super::spatial::entry::EntityCategory;
use super::spatial::integration::{TdGridObject, TdGridResource};
use crate::world_label::WorldLabel;

#[allow(clippy::too_many_arguments)]
pub fn td_wave_manager(
    time: Res<Time>,
    mut state: ResMut<TdWaveState>,
    config: Res<TdWaveConfig>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spawn_q: Query<(&Transform, &SpawnPoint)>,
    core_q: Query<&Transform, (With<DefenseCore>, Without<SpawnPoint>)>,
    mut start_wave_reader: MessageReader<StartNextWaveEvent>,
    gameplay: Res<GameplayConfig>,
    db: Res<GameDatabase>,
    mut grid: ResMut<TdGridResource>,
    nav_mesh: Res<TdNavMesh>,
) {
    for _ in start_wave_reader.read() {
        if state.phase == WavePhase::Waiting && state.current_wave < config.max_waves {
            start_new_wave(&mut state, &config);
            info!("手动开始第 {} 波!", state.current_wave);
        }
    }

    match state.phase {
        WavePhase::Waiting => {
            state.wave_timer.tick(time.delta());
            if state.wave_timer.is_finished() && state.current_wave < config.max_waves {
                start_new_wave(&mut state, &config);
                info!("自动开始第 {} 波!", state.current_wave);
            }
        }
        WavePhase::Spawning => {
            state.spawn_timer.tick(time.delta());
            if state.spawn_timer.is_finished() {
                spawn_td_enemy(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &spawn_q,
                    &core_q,
                    &mut state,
                    &config,
                    &gameplay,
                    &db,
                    &mut grid,
                    &nav_mesh,
                );
            }
        }
        WavePhase::Active => {
            if state.enemies_to_spawn == 0 && state.enemies_alive == 0 {
                if state.current_wave >= config.max_waves {
                    state.phase = WavePhase::Complete;
                } else {
                    state.phase = WavePhase::Waiting;
                    state.wave_timer =
                        Timer::from_seconds(config.wave_cooldown, TimerMode::Once);
                    info!(
                        "第 {} 波完成! {} 秒后开始下一波",
                        state.current_wave, config.wave_cooldown
                    );
                }
            }
        }
        WavePhase::Complete => {}
    }
}

fn start_new_wave(state: &mut TdWaveState, config: &TdWaveConfig) {
    state.current_wave += 1;
    state.phase = WavePhase::Spawning;
    state.enemies_to_spawn =
        config.enemies_per_wave_base + (state.current_wave - 1) * config.enemies_per_wave_growth;
    state.enemies_alive = 0;
    state.spawn_timer = Timer::from_seconds(config.spawn_interval, TimerMode::Repeating);
}

#[allow(clippy::too_many_arguments)]
fn spawn_td_enemy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spawn_q: &Query<(&Transform, &SpawnPoint)>,
    core_q: &Query<&Transform, (With<DefenseCore>, Without<SpawnPoint>)>,
    state: &mut TdWaveState,
    _config: &TdWaveConfig,
    _gameplay: &GameplayConfig,
    db: &GameDatabase,
    grid: &mut TdGridResource,
    nav_mesh: &TdNavMesh,
) {
    if state.enemies_to_spawn == 0 {
        if state.phase == WavePhase::Spawning {
            state.phase = WavePhase::Active;
        }
        return;
    }

    let spawn_count = spawn_q.iter().count();
    if spawn_count == 0 {
        return;
    }
    let idx = (state.enemies_to_spawn as usize) % spawn_count;
    let Some((spawn_t, _spawn_point)) = spawn_q.iter().nth(idx) else {
        return;
    };

    let enemy_type = pick_enemy_type(state.current_wave);
    let def = db.find_enemy(enemy_type.id());

    let target_pos = core_q
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let pos = spawn_t.translation;
    let waypoints = nav_mesh
        .find_path(pos, target_pos)
        .unwrap_or_else(|| vec![target_pos]);
    let size = def.map(|d| d.size).unwrap_or_else(|| enemy_type.size());
    let def_color = def.map(|d| d.to_color()).unwrap_or_else(|| enemy_type.color());
    let mat = materials.add(StandardMaterial {
        base_color: def_color,
        emissive: def_color.into(),
        ..default()
    });

    let health_val = def.map(|d| d.health).unwrap_or_else(|| enemy_type.health());
    let dmg = def.map(|d| d.damage).unwrap_or_else(|| enemy_type.damage());
    let spd = def.map(|d| d.speed).unwrap_or_else(|| enemy_type.speed());

    let enemy_name = def.map(|d| d.name.clone()).unwrap_or_else(|| format!("{:?}", enemy_type));
    let pos_y = size / 2.0;
    let enemy_entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size, size, size))),
        MeshMaterial3d(mat),
        Transform::from_xyz(pos.x, pos_y, pos.z),
        TdEnemy {
            enemy_type,
            gold_reward: def.map(|d| d.gold).unwrap_or_else(|| enemy_type.gold_reward()),
        },
        Health::new(health_val),
        AttackDamage(dmg),
        MoveSpeed(spd),
        TdGridObject {
            category: EntityCategory::Monster,
            radius: size / 2.0,
        },
        NavPath::new(waypoints),
        TdGridPos(Vec2::new(pos.x, pos.z)),
        LevelEntity,
        Name::new(format!("TdEnemy_{:?}", enemy_type)),
        WorldLabel::new(&enemy_name).with_offset(2.0).with_font_size(12.0),
    ))
    .id();

    let grid_pos = Vec2::new(pos.x, pos.z);
    grid.insert_entity(enemy_entity, EntityCategory::Monster, grid_pos, size / 2.0);

    state.enemies_to_spawn -= 1;
    state.enemies_alive += 1;
}

fn pick_enemy_type(wave: u32) -> TdEnemyType {
    if wave <= 3 {
        return TdEnemyType::Basic;
    }
    if wave <= 6 {
        return if rand::random::<f32>() < 0.3 {
            TdEnemyType::Fast
        } else {
            TdEnemyType::Basic
        };
    }
    let r = rand::random::<f32>();
    if r < 0.3 {
        TdEnemyType::Fast
    } else if r < 0.55 {
        TdEnemyType::Tank
    } else {
        TdEnemyType::Basic
    }
}
