# Phase 1: 模块重组 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构 NI 游戏代码的模块组织方式，从扁平结构改为领域驱动目录结构

**Architecture:** 将 ~45 个扁平模块按领域分组到 `core/`, `game/`, `world/`, `physics/`, `render/`, `ai/`, `audio/`, `ui/`, `network/`, `assets/`, `tools/` 目录下，每个目录有 `mod.rs`。主入口 `main.rs` 精简为只注册顶级域模块 + 向后兼容的 re-export。

**Tech Stack:** Rust + Bevy 0.18, cargo check 验证每步

---

### 文件映射表（所有文件迁移一览）

| 当前路径 | 新路径 |
|----------|--------|
| `src/config.rs` | `src/core/config.rs` |
| `src/game_state.rs` | `src/core/game_state.rs` |
| `src/log.rs` | `src/core/log.rs` |
| `src/player.rs` | `src/game/player.rs` |
| `src/enemy.rs` | `src/game/enemy.rs` |
| `src/npc.rs` | `src/game/npc.rs` |
| `src/dialogue.rs` | `src/game/dialogue.rs` |
| `src/combat.rs` | `src/game/combat.rs` |
| `src/stealth.rs` | `src/game/stealth.rs` |
| `src/collectible.rs` | `src/game/collectible.rs` |
| `src/puzzle.rs` | `src/game/puzzle.rs` |
| `src/inventory.rs` | `src/game/inventory.rs` |
| `src/solari_demo.rs` | `src/game/solari_demo.rs` |
| `src/level.rs` | `src/world/level.rs` |
| `src/level_tool_plugin.rs` | `src/world/level_tool.rs` |
| `src/grid.rs` | `src/world/grid.rs` |
| `src/placement.rs` | `src/world/placement.rs` |
| `src/nav_mesh.rs` | `src/world/nav_mesh.rs` |
| `src/world.rs` | `src/world/terrain.rs` |
| `src/world_label.rs` | `src/world/label.rs` |
| `src/collision.rs` | `src/physics/collision/shape.rs` |
| `src/colliders.rs` | `src/physics/collision/collider.rs` |
| `src/collision_manager.rs` | `src/physics/collision/manager.rs` |
| `src/collision_debug.rs` | `src/physics/collision/debug.rs` |
| `src/ray_cast.rs` | `src/physics/ray_cast.rs` |
| `src/toon/` | `src/render/toon/` (保持子模块结构) |
| `src/camera.rs` | `src/render/camera.rs` |
| `src/camera_motion.rs` | `src/render/camera_motion.rs` |
| `src/particles.rs` | `src/render/particles.rs` |
| `src/animation.rs` | `src/render/animation.rs` |
| `src/scale/` | `src/render/scale/` |
| `src/debug_lighting.rs` | `src/render/debug_lighting.rs` |
| `src/render_utils.rs` | `src/render/render_utils.rs` |
| `src/ai.rs` | `src/ai/ai.rs` |
| `src/pathfinding/` | `src/ai/pathfinding/` |
| `src/audio.rs` | `src/audio/audio.rs` |
| `src/music.rs` | `src/audio/music.rs` |
| `src/ui.rs` | `src/ui/ui.rs` |
| `src/image_gallery.rs` | `src/ui/image_gallery.rs` |
| `src/network.rs` | `src/network/network.rs` |
| `src/loading.rs` | `src/assets/loading.rs` |
| `src/entity_db/` | `src/assets/entity_db/` |
| `src/proximity_loader.rs` | `src/assets/proximity_loader.rs` |
| `src/creative.rs` | `src/tools/creative.rs` |
| `src/stress_test.rs` | `src/tools/stress_test.rs` |
| `src/time_recorder.rs` | `src/tools/time_recorder.rs` |
| `src/td/` | `src/td/` (保持不变) |

---

### Task 1: 创建新目录结构和所有 mod.rs 文件

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/game/mod.rs`
- Create: `src/world/mod.rs`
- Create: `src/physics/mod.rs`
- Create: `src/physics/collision/mod.rs`
- Create: `src/render/mod.rs`
- Create: `src/ai/mod.rs`
- Create: `src/audio/mod.rs`
- Create: `src/ui/mod.rs`
- Create: `src/network/mod.rs`
- Create: `src/assets/mod.rs`
- Create: `src/tools/mod.rs`

- [ ] **Step 1: 创建所有目录**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
mkdir -p src/core src/game src/world src/physics/collision
mkdir -p src/render src/ai src/audio src/ui src/network
mkdir -p src/assets src/tools
```

- [ ] **Step 2: 创建 `src/core/mod.rs`**

```rust
//! 核心基础设施 — 配置、状态、日志
pub mod config;
pub mod game_state;
pub mod log;
```

- [ ] **Step 3: 创建 `src/game/mod.rs`**

```rust
//! 游戏玩法 — 玩家、敌人、NPC、战斗、潜行等
pub mod player;
pub mod enemy;
pub mod npc;
pub mod dialogue;
pub mod combat;
pub mod stealth;
pub mod collectible;
pub mod puzzle;
pub mod inventory;
pub mod solari_demo;
```

- [ ] **Step 4: 创建 `src/world/mod.rs`**

```rust
//! 世界/场景 — 关卡、网格、放置、导航网格、地形
pub mod level;
pub mod level_tool;
pub mod grid;
pub mod placement;
pub mod nav_mesh;
pub mod terrain;
pub mod label;

// 向后兼容: 旧 world.rs 中的 WorldPlugin 现在在 terrain 模块中
pub use terrain::WorldPlugin;
```

- [ ] **Step 5: 创建 `src/physics/mod.rs`**

```rust
//! 物理/碰撞系统
pub mod collision;
pub mod ray_cast;
```

- [ ] **Step 6: 创建 `src/physics/collision/mod.rs`**

```rust
//! 碰撞检测系统
pub mod shape;
pub mod collider;
pub mod manager;
pub mod debug;

// 向后兼容 — 将子模块的所有 pub 项提升到 collision 命名空间
// 这样旧的 crate::collision::CollisionShape 等路径仍可解析
pub use shape::*;
pub use collider::*;
pub use manager::*;
pub use debug::*;
```

- [ ] **Step 7: 创建 `src/render/mod.rs`**

```rust
//! 渲染管线 — 卡通着色、相机、粒子、动画、缩放
pub mod toon;
pub mod camera;
pub mod camera_motion;
pub mod particles;
pub mod animation;
pub mod scale;
pub mod debug_lighting;
pub mod render_utils;
```

- [ ] **Step 8: 创建 `src/ai/mod.rs`**

```rust
//! AI 系统 — 行为树、感知、寻路
pub mod ai;
pub mod pathfinding;
```

- [ ] **Step 9: 创建 `src/audio/mod.rs`**

```rust
//! 音频系统 — 音效、音乐
pub mod audio;
pub mod music;
```

- [ ] **Step 10: 创建 `src/ui/mod.rs`**

```rust
//! UI 系统 — HUD、画廊
pub mod ui;
pub mod image_gallery;
```

- [ ] **Step 11: 创建 `src/network/mod.rs`**

```rust
//! 网络模块
pub mod network;
```

- [ ] **Step 12: 创建 `src/assets/mod.rs`**

```rust
//! 资产管理 — 加载、实体数据库、近距离加载
pub mod loading;
pub mod entity_db;
pub mod proximity_loader;
```

- [ ] **Step 13: 创建 `src/tools/mod.rs`**

```rust
//! 开发工具 — 创造模式、压力测试、时间记录
pub mod creative;
pub mod stress_test;
pub mod time_recorder;
```

---

### Task 2: 复制所有扁平文件到新位置

**说明：** 此任务只复制不删除，src/ 下的原文件保持不动，确保编译不中断。

- [ ] **Step 1: 复制 core 文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/config.rs src/core/config.rs
cp src/game_state.rs src/core/game_state.rs
cp src/log.rs src/core/log.rs
```

- [ ] **Step 2: 复制 game 文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/player.rs src/game/player.rs
cp src/enemy.rs src/game/enemy.rs
cp src/npc.rs src/game/npc.rs
cp src/dialogue.rs src/game/dialogue.rs
cp src/combat.rs src/game/combat.rs
cp src/stealth.rs src/game/stealth.rs
cp src/collectible.rs src/game/collectible.rs
cp src/puzzle.rs src/game/puzzle.rs
cp src/inventory.rs src/game/inventory.rs
cp src/solari_demo.rs src/game/solari_demo.rs
```

- [ ] **Step 3: 复制 world 文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/level.rs src/world/level.rs
cp src/level_tool_plugin.rs src/world/level_tool.rs
cp src/grid.rs src/world/grid.rs
cp src/placement.rs src/world/placement.rs
cp src/nav_mesh.rs src/world/nav_mesh.rs
cp src/world.rs src/world/terrain.rs
cp src/world_label.rs src/world/label.rs
```

- [ ] **Step 4: 复制 physics 文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/collision.rs src/physics/collision/shape.rs
cp src/colliders.rs src/physics/collision/collider.rs
cp src/collision_manager.rs src/physics/collision/manager.rs
cp src/collision_debug.rs src/physics/collision/debug.rs
cp src/ray_cast.rs src/physics/ray_cast.rs
```

- [ ] **Step 5: 复制 render 文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/camera.rs src/render/camera.rs
cp src/camera_motion.rs src/render/camera_motion.rs
cp src/particles.rs src/render/particles.rs
cp src/animation.rs src/render/animation.rs
cp src/debug_lighting.rs src/render/debug_lighting.rs
cp src/render_utils.rs src/render/render_utils.rs
cp -r src/toon/* src/render/toon/
cp -r src/scale/* src/render/scale/
```

- [ ] **Step 6: 复制 AI 和音频文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/ai.rs src/ai/ai.rs
cp -r src/pathfinding/* src/ai/pathfinding/
cp src/audio.rs src/audio/audio.rs
cp src/music.rs src/audio/music.rs
```

- [ ] **Step 7: 复制 UI、网络、资产、工具文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cp src/ui.rs src/ui/ui.rs
cp src/image_gallery.rs src/ui/image_gallery.rs
cp src/network.rs src/network/network.rs
cp src/loading.rs src/assets/loading.rs
cp -r src/entity_db/* src/assets/entity_db/
cp src/proximity_loader.rs src/assets/proximity_loader.rs
cp src/creative.rs src/tools/creative.rs
cp src/stress_test.rs src/tools/stress_test.rs
cp src/time_recorder.rs src/tools/time_recorder.rs
```

**注意：** 文件复制后立即运行 `cargo check` 验证编译是否正常（此时旧模块声明仍有效）。

---

### Task 3: 更新 main.rs — 切换为领域模块 + 向后兼容 re-exports

**Files:**
- Modify: `src/main.rs`

**核心变更：**
1. 删除所有扁平的 `mod X;` 声明（第 29-73 行）
2. 替换为领域模块 `mod core; mod game; ...` 声明（约 14 行）
3. 更新所有 `use X::Y` 导入语句（第 76-116 行）使用新路径
4. 添加 `pub use` re-exports 保持旧路径兼容（供其他模块使用 crate::X 引用）

**完整的新的 main.rs 内容如下（替换整个文件）：**

```rust
//! NI 3D 潜行恐怖游戏 — 入口文件
//!
//! 领域模块化架构：按游戏功能分组到 core/game/world/physics/render/ai/audio/
//! ui/network/assets/tools 等目录下。main.rs 仅做模块注册和 App 启动。

use bevy::prelude::*;

use bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

// ═══ 领域模块声明 ═══
mod core;
mod game;
mod world;
mod physics;
mod render;
mod ai;
mod audio;
mod ui;
mod network;
mod assets;
mod tools;
mod td;             // 塔防（保持现有子模块结构）

// ═══ Use 导入（使用新路径） ═══
use render::animation::AnimationPlugin;
use audio::audio::GameAudioPlugin;
use render::camera::{CameraControllerPlugin, CameraPlugin};
use render::camera_motion::CameraMotionPlugin;
use game::collectible::CollectiblePlugin;
use physics::collision::manager::CollisionManagerPlugin;
use game::combat::CombatPlugin;
use core::config::GameplayConfig;
use game::dialogue::DialoguePlugin;
use game::enemy::EnemyPlugin;
use core::game_state::GameStatePlugin;
use game::inventory::InventoryPlugin;
use world::level::LevelPlugin;
use game::npc::NpcPlugin;
use game::player::PlayerPlugin;
use td::TdPlugin;
use ui::ui::GameUiPlugin;
use world::WorldPlugin;             // re-exported from terrain
use render::particles::ParticlePlugin;
use render::scale::ScalePlugin;
use world::label::WorldLabelPlugin;
use tools::time_recorder::TimeRecorderPlugin;
use ui::image_gallery::ImageGalleryPlugin;
use tools::stress_test::StressTestPlugin;
use assets::entity_db::EntityDbPlugin;
use ai::pathfinding::PathfindingPlugin;
use game::puzzle::PuzzlePlugin;
use world::grid::GameGridPlugin;
use ai::ai::AiPlugin;
use game::stealth::StealthPlugin;
use physics::collision::debug::CollisionDebugPlugin;

use assets::loading::LoadingPlugin;
use audio::music::MusicPlugin;
use world::placement::PlacementPlugin;
use assets::proximity_loader::ProximityLoaderPlugin;
use tools::creative::CreativePlugin;
use world::level_tool::LevelToolPlugin;
// 注意: solari_demo.rs 不导出 Plugin，只导出 spawn_pbr_showcase 函数

use core::log::configured_plugins;
use network::network::NetworkPlugin;

// ═══ 向后兼容 re-exports（保持 crate::X 路径可供其他模块使用） ═══
pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;
pub use physics::collision;                                // crate::collision
pub use physics::collision::collider as colliders;          // crate::colliders
pub use physics::collision::manager as collision_manager;   // crate::collision_manager
pub use physics::collision::debug as collision_debug;       // crate::collision_debug
pub use physics::ray_cast;
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::ai;
pub use ai::pathfinding;
pub use audio::{audio, music};
pub use ui::{ui, image_gallery};
pub use network::network;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((
        GameStatePlugin,
        DialoguePlugin,
        InventoryPlugin,
        NpcPlugin,
        AnimationPlugin,
        PlayerPlugin,
        CameraPlugin,
        WorldPlugin,
        LevelPlugin,
        GameAudioPlugin,
        CollectiblePlugin,
        CombatPlugin,
        EnemyPlugin,
        GameUiPlugin,
        CollisionManagerPlugin,
    ));
    app.add_plugins((ParticlePlugin, TdPlugin, PuzzlePlugin, AiPlugin, GameGridPlugin, StealthPlugin, CollisionDebugPlugin, LoadingPlugin));

    app.add_plugins(CameraMotionPlugin);
    app.add_plugins(WorldLabelPlugin);
    app.add_plugins(TimeRecorderPlugin);
    app.add_plugins(ImageGalleryPlugin);
    app.add_plugins(EntityDbPlugin);
    app.add_plugins(StressTestPlugin);
    app.add_plugins(PathfindingPlugin);
    app.add_plugins(MusicPlugin);
    app.add_plugins(PlacementPlugin);
    app.add_plugins(ProximityLoaderPlugin);
    app.add_plugins(LevelToolPlugin);
    app.add_plugins(CreativePlugin);

    // 渲染分辨率缩放 — 性价比最高的性能优化
    // F11: 开关 | F7: 质量模式 | F8: 信息面板
    app.add_plugins(ScalePlugin);

    // Solari 光追渲染在进入 Solari 关卡时按需激活
    // CameraControllerPlugin 提供通用自由视角（WASD + 鼠标），关卡6使用
    app.add_plugins(CameraControllerPlugin);

    // 全局旋转动画（对任何带有 Rotating 组件的实体生效）
    app.add_systems(Update, render::render_utils::animate_rotation);

    // F5: 快速跳转到 Demo 关卡（展示新模型）
    app.add_systems(Update, debug_switch_to_demo);

    app.init_resource::<GameplayConfig>();
    app.register_type::<core::config::GameplayConfig>();
    app.register_type::<physics::collision::shape::CollisionShape>();
    app.register_type::<physics::collision::collider::Collider>();
    app.register_type::<physics::collision::collider::ColliderShape>();
    app.register_type::<physics::collision::collider::CollisionLayer>();
    app.register_type::<physics::collision::collider::CollisionMask>();
    app.register_type::<physics::collision::collider::SmoothPush>();
    app.init_resource::<render::debug_lighting::LightingDebug>();
    app.add_systems(PostUpdate, render::debug_lighting::sync_lighting_to_world);

    // Always add EguiPlugin since UI depends on it
    app.add_plugins(EguiPlugin::default());

    #[cfg(debug_assertions)]
    app.add_plugins(
        WorldInspectorPlugin::new()
            .run_if(input_toggle_active(true, KeyCode::F3)),
    );

    app.run();
}

/// 调试快捷键：F5 → 跳转到 Demo 关卡
fn debug_switch_to_demo(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<world::level::LoadLevelEvent>,
    mut phase: ResMut<NextState<core::game_state::GamePhase>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        info!("[Debug] F5 → 跳转到 Demo 关卡");
        phase.set(core::game_state::GamePhase::Playing);
        events.write(world::level::LoadLevelEvent { level: world::level::GameLevel::Demo });
    }
}
```

注意：`solari_demo.rs` 中的 `SolariDemoPlugin` 需要确认是否存在。如果不存在，删除对应的 import 和 plugin 注册行。

- [ ] **Step 1: 替换 main.rs 的全部 import + mod 声明**

**在模块声明区域（第 29-73 行），将全部 `mod X;` 替换为领域模块声明：**

详情见下方完整 main.rs 版本。

**将 use 导入（第 76-116 行）全部更新为新路径：**

```rust
// ═══ 旧的 use 导入 — 全部需要更新 ═══
use animation::AnimationPlugin;             → use render::animation::AnimationPlugin;
use audio::GameAudioPlugin;                  → use audio::audio::GameAudioPlugin;
use camera::{CameraControllerPlugin, ...};    → use render::camera::{...};
// ... (完整列表见下方)
```

```rust
// ═══ 领域模块声明 ═══
mod core;
mod game;
mod world;
mod physics;
mod render;
mod ai;
mod audio;
mod ui;
mod network;
mod assets;
mod tools;
mod td;
mod scale;
mod collision_debug;

// ═══ 向后兼容 re-exports（保持 crate::X 路径可用） ═══
pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;    // 别名: crate::world_label → crate::world::label
pub use physics::collision;              // crate::collision → 含 glob re-export
pub use physics::collision::collider as colliders;          // crate::colliders
pub use physics::collision::manager as collision_manager;   // crate::collision_manager
pub use physics::collision::debug as collision_debug;       // crate::collision_debug
pub use physics::ray_cast;
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::ai;
pub use ai::pathfinding;
pub use audio::{audio, music};
pub use ui::{ui, image_gallery};
pub use network::network;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};

// world::terrain 需要特殊处理：旧代码使用 crate::world::WorldPlugin
// 在 world/mod.rs 中添加 pub use terrain::WorldPlugin;
```

注意：
- `collision` 的旧路径是 `crate::collision`，现在通过 `pub use physics::collision;` 保持可用
- `collision_debug` 保持独立模块声明（它在 physics 下是 `physics::collision::debug`）
- `colliders` 的旧路径是 `crate::colliders`，现在通过 collision re-export: `crate::collision::collider`
- `world_label` 现在为 `crate::world::label`，通过 `pub use world::label;` 保持

待修复: 其他文件中引用 `crate::colliders::X` 需要改为 `crate::collision::collider::X`——此修改在后续任务中进行。

- [ ] **Step 2: 运行 cargo check 检查编译错误**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1 | head -80
```

记录所有报错，后面逐步修复。

---

### Task 4: 修复所有模块导入路径

**说明：** 模块移动后，某些内部 `crate::XXX::Symbol` 路径发生了变化。需要逐个修复。

需要修复的路径变更：

| 旧路径 | 新路径 | 需修改的文件数 |
|--------|--------|---------------|
| `crate::colliders::X` | `crate::collision::collider::X` | 多个 |
| `crate::collision::X` | `crate::collision::shape::X` | 多个 |
| `crate::world::WorldPlugin` | 不变（使用 re-export） | 0 |
| `crate::world_label::X` | `crate::world::label::X` | 多个 |
| `crate::world::X` (terrain相关) | `crate::world::terrain::X` | 多个 |

- [ ] **Step 1: 修复 colliders.rs 引用**

在 `main.rs`, `player.rs`, `collision_manager.rs`, `collision_debug.rs` 以及其他引用 `crate::colliders::` 的文件中，将导入改为 `crate::collision::collider::`。

运行 grep 查找所有引用：
```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
grep -rn "crate::colliders" src/ --include="*.rs" | grep -v "src/colliders.rs"
```

逐个文件修改。

- [ ] **Step 2: 修复 collision.rs 引用**

查找并修改：
```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
grep -rn "crate::collision" src/ --include="*.rs" | grep -v "src/collision.rs" | grep -v "src/physics/collision"
```

将 `crate::collision::CollisionShape` 等改为 `crate::collision::shape::CollisionShape`。

- [ ] **Step 3: 修复 world_label 引用**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
grep -rn "crate::world_label" src/ --include="*.rs"
```

改为 `crate::world::label::`。

- [ ] **Step 4: 修复其他导入路径**

运行全面的导入检查：
```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

对每个错误，检查无法解析的路径，根据文件映射表进行修正。

- [ ] **Step 5: 迭代修复直到 cargo check 通过**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

修复 -> 重试，直到通过。

---

### Task 5: 清理旧文件 + 最终验证

- [ ] **Step 1: 删除已迁移的旧源文件**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
# core - 保留原文件? 否，删除
rm src/config.rs src/game_state.rs src/log.rs
# game
rm src/player.rs src/enemy.rs src/npc.rs src/dialogue.rs
rm src/combat.rs src/stealth.rs src/collectible.rs
rm src/puzzle.rs src/inventory.rs src/solari_demo.rs
# world
rm src/level.rs src/level_tool_plugin.rs src/grid.rs
rm src/placement.rs src/nav_mesh.rs src/world.rs src/world_label.rs
# physics
rm src/collision.rs src/colliders.rs src/collision_manager.rs
rm src/collision_debug.rs src/ray_cast.rs
# render (保留 toon/ 和 scale/ 子目录，它们已被复制)
rm src/camera.rs src/camera_motion.rs src/particles.rs
rm src/animation.rs src/debug_lighting.rs src/render_utils.rs
rm -rf src/toon src/scale
# ai
rm src/ai.rs
rm -rf src/pathfinding
# audio
rm src/audio.rs src/music.rs
# ui
rm src/ui.rs src/image_gallery.rs
# network
rm src/network.rs
# assets
rm src/loading.rs src/proximity_loader.rs
rm -rf src/entity_db
rm -rf src/assets  # 删除旧的空 assets 目录（含空的 dialogue/ 和 zones/）
# tools
rm src/creative.rs src/stress_test.rs src/time_recorder.rs
```

- [ ] **Step 2: 最终 cargo check**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo check 2>&1
```

期望结果：编译通过，无错误。

- [ ] **Step 3: cargo clippy 检查**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy -- -D warnings 2>&1
```

如非必要，不修改代码逻辑，仅修复 clippy 指出的路径相关问题。
