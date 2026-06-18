//! 时间回溯数据库 — 栈式存储实体位置历史
//!
//! # 数据结构
//! - `PositionHistory`: 每个实体的独立位置栈（ECS 组件）
//! - `TimeDatabase`: 全局数据库（支持 JSON 持久化）
//!
//! # 操作
//! - `R` 键: 回退一步（从栈中弹出记录恢复位置）
//! - `Ctrl+S`: 保存数据库到 `saves/time_database.json`
//! - `Ctrl+L`: 从文件加载数据库

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── 常量 ──

const DB_FILENAME: &str = "time_database.json";

// ── 组件 ──

/// 标记需要被时间回溯追踪的实体
#[derive(Component)]
pub struct RewindTracked;

/// 实体类型分类（用于数据库索引）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackedEntityKind {
    Collectible,
    Player,
    Enemy,
    Npc,
    Other,
}

/// 每个实体的位置历史栈组件
///
/// 栈结构：索引 0 为最旧记录，末尾为最新记录。
/// `pop()` 弹出最近一条记录用于回退。
#[derive(Component)]
pub struct PositionHistory {
    /// 位置记录栈
    pub stack: Vec<TimedPosition>,
    /// 最大栈深度（超出时移除最旧记录）
    pub max_size: usize,
    /// 实体类型
    pub kind: TrackedEntityKind,
}

// ── 数据记录 ──

/// 带时间戳的位置记录
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TimedPosition {
    pub timestamp: f64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// ── 资源 ──

/// 录制控制
#[derive(Resource)]
pub struct TimeRecorder {
    /// 是否正在录制
    pub is_recording: bool,
    /// 是否在回放中
    pub is_rewinding: bool,
    /// 记录间隔（秒）
    pub record_interval: f32,
    /// 累积时间
    pub accumulator: f32,
    /// 游戏时间
    pub game_time: f64,
}

impl Default for TimeRecorder {
    fn default() -> Self {
        Self {
            is_recording: true,
            is_rewinding: false,
            record_interval: 0.5,
            accumulator: 0.0,
            game_time: 0.0,
        }
    }
}

/// 时间回溯数据库 — 持久化用
#[derive(Resource, Default, Serialize, Deserialize)]
pub struct TimeDatabase {
    pub logs: Vec<EntityLog>,
    pub session: SessionMeta,
}

/// 单个实体的完整历史日志
#[derive(Clone, Serialize, Deserialize)]
pub struct EntityLog {
    pub name: String,
    pub kind: String,
    pub records: Vec<TimedPosition>,
}

/// 会话元数据
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SessionMeta {
    pub level: String,
    pub total_entities: usize,
    pub total_records: usize,
}

// ── 事件 ──

/// 回退一步
#[derive(Message)]
pub struct RewindStepEvent;

/// 保存数据库到文件
#[derive(Message)]
pub struct SaveDatabaseEvent;

/// 从文件加载数据库
#[derive(Message)]
pub struct LoadDatabaseEvent;

// ── 插件 ──

pub struct TimeRecorderPlugin;

impl Plugin for TimeRecorderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeRecorder>()
            .init_resource::<TimeDatabase>()
            .add_message::<RewindStepEvent>()
            .add_message::<SaveDatabaseEvent>()
            .add_message::<LoadDatabaseEvent>()
            .add_systems(
                Update,
                (
                    update_game_time,
                    record_positions,
                    handle_rewind,
                    handle_save_database,
                    handle_load_database,
                    keyboard_controls,
                )
                    .chain()
                    .run_if(in_state(crate::game_state::GamePhase::Playing)),
            );
    }
}

// ── 系统 ──

/// 累计游戏时间
fn update_game_time(time: Res<Time>, mut recorder: ResMut<TimeRecorder>) {
    if recorder.is_recording {
        recorder.game_time += time.delta_secs() as f64;
    }
}

/// 定期记录被追踪实体的位置
fn record_positions(
    time: Res<Time>,
    mut recorder: ResMut<TimeRecorder>,
    mut db: ResMut<TimeDatabase>,
    mut tracked_q: Query<(&Name, &Transform, &mut PositionHistory), With<RewindTracked>>,
) {
    if !recorder.is_recording || recorder.is_rewinding {
        return;
    }

    recorder.accumulator += time.delta_secs();
    if recorder.accumulator < recorder.record_interval {
        return;
    }
    recorder.accumulator = 0.0;

    let ts = recorder.game_time;
    let mut total_entities = 0usize;
    let mut total_records = 0usize;

    for (name, transform, mut history) in tracked_q.iter_mut() {
        total_entities += 1;
        let record = TimedPosition {
            timestamp: ts,
            x: transform.translation.x,
            y: transform.translation.y,
            z: transform.translation.z,
        };

        // 推入实体栈
        history.stack.push(record);
        if history.stack.len() > history.max_size {
            history.stack.remove(0);
        }
        total_records += 1;

        // 同步到全局数据库
        sync_to_database(&mut db, name, &history.kind, record);
    }

    db.session.total_records = db.session.total_records.saturating_add(total_records);
    db.session.total_entities = total_entities;
}

/// 同步一条记录到全局数据库
fn sync_to_database(
    db: &mut TimeDatabase,
    name: &Name,
    kind: &TrackedEntityKind,
    record: TimedPosition,
) {
    let name_str = name.as_str();
    let kind_str = match kind {
        TrackedEntityKind::Collectible => "collectible",
        TrackedEntityKind::Player => "player",
        TrackedEntityKind::Enemy => "enemy",
        TrackedEntityKind::Npc => "npc",
        TrackedEntityKind::Other => "other",
    };

    if let Some(log) = db.logs.iter_mut().find(|l: &&mut EntityLog| l.name == name_str) {
        log.records.push(record);
    } else {
        db.logs.push(EntityLog {
            name: name_str.to_string(),
            kind: kind_str.to_string(),
            records: vec![record],
        });
    }
}

/// 处理回退事件：从栈中弹出最新记录并恢复位置
fn handle_rewind(
    mut events: MessageReader<RewindStepEvent>,
    mut recorder: ResMut<TimeRecorder>,
    mut tracked_q: Query<(&mut Transform, &mut PositionHistory), With<RewindTracked>>,
) {
    for _ in events.read() {
        recorder.is_rewinding = true;

        let mut restored = 0u32;
        for (mut transform, mut history) in tracked_q.iter_mut() {
            if let Some(record) = history.stack.pop() {
                transform.translation.x = record.x;
                transform.translation.y = record.y;
                transform.translation.z = record.z;
                restored += 1;
            }
        }

        info!("时间回溯: 恢复了 {} 个实体的位置", restored);
        recorder.is_rewinding = false;
    }
}

/// 保存数据库到 JSON 文件
fn handle_save_database(
    mut events: MessageReader<SaveDatabaseEvent>,
    db: Res<TimeDatabase>,
) {
    for _ in events.read() {
        let path = db_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&*db) {
            Ok(json) => match fs::write(&path, &json) {
                Ok(_) => info!(
                    "时间数据库已保存: {} ({} 个实体, {} 条记录)",
                    path.display(),
                    db.logs.len(),
                    db.session.total_records
                ),
                Err(e) => error!("保存时间数据库失败: {}", e),
            },
            Err(e) => error!("序列化时间数据库失败: {}", e),
        }
    }
}

/// 从 JSON 文件加载数据库
fn handle_load_database(
    mut events: MessageReader<LoadDatabaseEvent>,
    mut db: ResMut<TimeDatabase>,
) {
    for _ in events.read() {
        let path = db_file_path();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<TimeDatabase>(&json) {
                Ok(loaded) => {
                    *db = loaded;
                    info!(
                        "时间数据库已加载: {} ({} 个实体)",
                        path.display(),
                        db.logs.len()
                    );
                }
                Err(e) => error!("解析时间数据库失败: {}", e),
            },
            Err(e) => error!("读取时间数据库失败: {}", e),
        }
    }
}

/// 键盘快捷键
fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut rewind_writer: MessageWriter<RewindStepEvent>,
    mut save_writer: MessageWriter<SaveDatabaseEvent>,
    mut load_writer: MessageWriter<LoadDatabaseEvent>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        rewind_writer.write(RewindStepEvent);
    }
    if keys.pressed(KeyCode::ControlLeft) && keys.just_pressed(KeyCode::KeyS) {
        save_writer.write(SaveDatabaseEvent);
    }
    if keys.pressed(KeyCode::ControlLeft) && keys.just_pressed(KeyCode::KeyL) {
        load_writer.write(LoadDatabaseEvent);
    }
}

/// 获取数据库文件路径（可执行文件所在目录/saves/time_database.json）
fn db_file_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    path.pop();
    path.push("saves");
    path.push(DB_FILENAME);
    path
}
