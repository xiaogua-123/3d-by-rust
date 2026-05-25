// ═══════════════════════════════════════════
// 塔防事件定义
// ═══════════════════════════════════════════

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
