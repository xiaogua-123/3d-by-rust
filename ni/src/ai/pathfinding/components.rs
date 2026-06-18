//! 寻路组件 — 挂载到需要寻路的实体上
//!
//! 定义 `Navigator`（寻路代理）、`NavTarget`（目标位置）、
//! `NavPath`（路径缓存）组件和 `RequestPathEvent` 事件。

use bevy::prelude::*;
use bevy::time::Timer;

/// 寻路代理 — 挂载到任何需要自动寻路的实体上
///
/// 系统会自动检测此组件并驱动寻路，无需手动管理路径。
/// 只需确保实体同时有 `NavTarget` 和 `Transform`。
#[derive(Component)]
pub struct Navigator {
    /// 移动速度（单位/秒）
    pub speed: f32,
    /// 到达 waypoint 的距离阈值
    pub threshold: f32,
    /// 重新寻路间隔（秒），动态目标时有用
    pub recalc_interval: f32,
    /// 是否需要水平旋转面向移动方向
    pub rotate_to_face: bool,
    /// 路径随机扰动强度（0.0=无扰动，0.5=中等，1.0=大幅偏移）
    ///
    /// 影响两个层面：
    /// - 路径层：A* 算出的路径点会随机横向偏移，路线自然变化
    /// - 行走层：跟随路径时有微小方向抖动，模拟不完美步态
    pub perturbation: f32,
    /// 内部计时器
    pub(crate) timer: Timer,
    /// 是否等待路径计算中
    #[allow(dead_code)]
    pub(crate) waiting: bool,
}

impl Default for Navigator {
    fn default() -> Self {
        Self {
            speed: 3.0,
            threshold: 0.5,
            recalc_interval: 2.0,
            rotate_to_face: true,
            perturbation: 0.0,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            waiting: false,
        }
    }
}

impl Navigator {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            ..Default::default()
        }
    }

    /// 带重算间隔的构造
    pub fn with_recalc(speed: f32, recalc_interval: f32) -> Self {
        Self {
            speed,
            recalc_interval,
            timer: Timer::from_seconds(recalc_interval, TimerMode::Repeating),
            ..Default::default()
        }
    }

    /// 带扰动参数的构造
    pub fn with_perturbation(speed: f32, perturbation: f32) -> Self {
        Self {
            speed,
            perturbation,
            ..Default::default()
        }
    }
}

/// 寻路目标 — 设置此组件即可驱动 NPC 向目标移动
///
/// 每帧检测位置变化，自动触发重新寻路。
/// 如果目标实体移动频繁，设置合理的 `recalc_interval` 避免每帧重算。
#[derive(Component, Clone)]
pub struct NavTarget {
    pub position: Vec3,
}

impl NavTarget {
    pub fn new(pos: Vec3) -> Self {
        Self { position: pos }
    }
}

impl From<Vec3> for NavTarget {
    fn from(position: Vec3) -> Self {
        Self { position }
    }
}

/// 当前路径 — 由寻路系统自动写入，外部只读
///
/// - `waypoints`: 世界坐标路径点列表
/// - `index`: 当前正在前往的路径点下标
/// - `completed`: 是否已到达终点
#[derive(Component)]
pub struct NavPath {
    pub waypoints: Vec<Vec3>,
    pub index: usize,
    pub completed: bool,
}

impl NavPath {
    pub fn new(waypoints: Vec<Vec3>) -> Self {
        let completed = waypoints.is_empty();
        Self {
            waypoints,
            index: 0,
            completed,
        }
    }

    /// 当前目标点
    pub fn current(&self) -> Option<Vec3> {
        if self.completed || self.index >= self.waypoints.len() {
            None
        } else {
            Some(self.waypoints[self.index])
        }
    }

    /// 移动到下一个路径点
    pub fn advance(&mut self) {
        if self.index + 1 < self.waypoints.len() {
            self.index += 1;
        } else {
            self.completed = true;
        }
    }
}

/// 寻路请求消息 — 触发一次寻路计算
#[derive(Message)]
pub struct RequestPathEvent {
    pub entity: Entity,
    pub to: Vec3,
}
