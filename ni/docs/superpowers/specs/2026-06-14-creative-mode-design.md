# 创造模式（Creative Mode）设计文档

## 概述

在 NI 游戏中内置一个类似 Minecraft 创造模式的 3D 关卡可视化编辑器。按 `F6` 切换进入，自由飞行、放置/删除物体，可选择保存到 RON 关卡配置文件。

## 游戏状态

新增 `GamePhase::Creative` 状态，与 `Playing` 互斥切换。

```
F6  → Playing ↔ Creative
ESC → Creative → Playing（不清除放置物）
```

## 快捷键

| 按键 | 功能 |
|------|------|
| F6 | 切换创造模式 |
| WASD | 前后左右飞行 |
| Space | 上升 |
| Shift | 下降 |
| 鼠标 | 视角旋转 |
| 滚轮 | 切换物品 |
| 数字键 1-0 | 选中对应物品槽 |
| 左键 | 放置物体 |
| 右键 | 删除物体 |
| G | 切换网格吸附 |
| H | 切换显示物体名称标签 |
| L | 切换显示原有关卡物体 |
| Ctrl+S | 保存到 RON |

## 文件结构

```
ni/src/
├── creative.rs          # 新建 — 所有创造模式代码
├── main.rs              # 添加 mod creative、注册 CreativePlugin
├── game_state.rs        # 添加 GamePhase::Creative
├── camera.rs            # CameraControllerPlugin 添加 Creative 状态支持
├── ui.rs                # 添加 creative_hud_ui 系统
```

## 系统设计

### 飞行相机

- 复用 `CameraControllerPlugin`，在 `GamePhase::Creative` 下也运行
- `camera_wasd` 系统：Space 改为上升，Shift 改为下降（替换原有的 E/Q）
- 鼠标锁定：进入 Creative 时锁定光标，退出时释放

### 物品 Hotbar

屏幕底部的 egui 面板：

```
┌─────────────────────────────────────────────────────────────┐
│  分类: [道具] [NPC] [敌人] [收集品] [装饰]                   │
│                                                              │
│  ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐      │
│  │  │ │  │ │  │ │  │ │  │ │  │ │  │ │  │ │  │ │  │      │
│  └──┘ └──┘ └──┘ └──┘ └──┘ └──┘ └──┘ └──┘ └──┘ └──┘      │
│    1    2    3    4    5    6    7    8    9    0          │
└─────────────────────────────────────────────────────────────┘
```

- 物品来源：`EntityRegistry` 按分类分组
- 分类标签切换显示对应类别的物品
- 选中物品显示幽灵预览（复用 `PlacementGhost` 机制）
- 屏幕左上角显示网格吸附状态、坐标信息

### 放置系统

- 从摄像机发射射线，计算与 y=0 地面的交点（复用 `placement.rs` 的 `ground_hit`）
- 网格吸附模式下，坐标取整到整数
- 左键 → `commands.spawn(CreativePlacedItem { template_id, saved: false }, SceneRoot, ...)`
- 幽灵预览为半透明（复用 `PlacementGhost` 组件）

### 删除系统

- 右键发射射线，检测最近的 `CreativePlacedItem` 实体
- 命中 → `commands.entity(e).despawn()`
- 如该物体已保存到 RON，标记 dirty 供下次保存时更新

### 层级显示

- `H` 键切换：显示/隐藏 `CreativePlacedItem` 的 3D 名称标签
- `L` 键切换：过滤掉 `LevelEntity` 组件物体（只看创造模式放置的）

### 保存到 RON

- 收集所有 `CreativePlacedItem` 的 `Transform` 和 `template_id`
- 转换为 `ProximityModelDef` 格式（path, position, scale, load_distance, unload_distance）
- 读取当前 `assets/level/level_config.ron`，更新对应关卡的 `proximity_models`
- 写回文件
- 保存后所有物体标记 `saved: true`

### 加载已保存的物体

- 进入 Creative 模式时，读取当前关卡 RON 配置
- `proximity_models` 条目生成为 `CreativePlacedItem { saved: true }`
- 与新建放置的物体统一管理，支持删除和重新保存

## 组件标记

```rust
#[derive(Component)]
pub struct CreativePlacedItem {
    pub template_id: String,
    pub saved: bool,
}
```

## 数据流

```
放置 → commands.spawn(CreativePlacedItem) + SceneRoot + Transform
  ↓
Ctrl+S → 收集所有 CreativePlacedItem → 写入 level_config.ron
  ↓
下次加载 → 读取 RON → 生成 CreativePlacedItem { saved: true }
```

## 依赖关系

- `creative.rs` 依赖：`bevy`, `bevy_egui`, `entity_db::EntityRegistry`, `ui::theme`, `camera::CameraController`, `placement::`(ground_hit 逻辑), `level_tool_plugin`(RON 格式)
- 无循环依赖

## 实现顺序

1. 添加 `GamePhase::Creative` 状态
2. 创建 `creative.rs` 模块骨架（Plugin + 状态切换）
3. 适配 `CameraControllerPlugin` 支持 Creative 状态
4. 实现 Hotbar UI（分类标签 + 物品槽）
5. 实现放置系统（幽灵预览 + 左键放置 + 网格吸附）
6. 实现删除系统（右键射线检测删除）
7. 实现层级控制（H/L 切换）
8. 实现保存/加载（Ctrl+S + RON 读写）
9. 集成到 `main.rs` 和 `ui.rs`
