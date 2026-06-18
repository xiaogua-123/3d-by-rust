//! 塔防事件定义
//!
//! 定义 `PurchaseTurretEvent`、`StartNextWaveEvent`、`TdVictoryEvent`、
//! `TdDefeatEvent`、`TurretShootEvent`、`EnemyDeathEvent` 等事件类型。

use bevy::prelude::*;
use super::data::TurretType;

#[derive(Message)]
pub struct PurchaseTurretEvent {
    pub turret_type: TurretType,
    pub position: Vec3,
}

#[derive(Message)]
pub struct StartNextWaveEvent;

#[derive(Message)]
pub struct TdVictoryEvent;

#[derive(Message)]
pub struct TdDefeatEvent;

/// 炮塔开火事件（用于播放射击音效）
#[derive(Message)]
pub struct TurretShootEvent;

/// 敌人死亡事件（携带金币奖励值）
#[derive(Message)]
pub struct EnemyDeathEvent {
    #[allow(dead_code)]
    pub gold_reward: f32,
}
