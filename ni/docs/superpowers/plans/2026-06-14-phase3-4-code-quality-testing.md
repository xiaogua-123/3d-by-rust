# Phase 3 & 4: 代码质量与测试 CI 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除所有 clippy 警告，精简 main.rs，规范化错误处理，添加测试覆盖

**Architecture:** 分两阶段推进：(1) 代码质量 — 修复 115 个 clippy 警告、精简 main.rs 到 ≤50 行、消除生产代码 unwrap；(2) 测试覆盖 — 为核心模块添加单元测试，配置 CI。

**Tech Stack:** Rust + Bevy 0.18, clippy, rstest

---

### 前置分析：clippy 告警分类

| 类别 | 数量 | 难度 |
|------|------|------|
| `if can be collapsed` | 24 | 简单 |
| `complex type` | 14 | 中等 |
| `deref` (auto-deref) | 8 | 简单 |
| `too many arguments` | 21 | 中等 |
| `map_or` simplify | 6 | 简单 |
| `dead code` (functions) | ~12 | 需判断 |
| `dead code` (fields) | ~5 | 简单 |
| `impl can be derived` | 3 | 简单 |
| manual range/iter | 5 | 简单 |
| unused imports/vars | 3 | 简单 |
| 其他杂项 | ~14 | 简单-中等 |

**总数: ~115 warnings**

---

### Task 1: 修复简单 clippy 警告（~50 个）

**说明：** 修复最容易的警告：collapsible `if`、auto-deref、`map_or`、未使用导入、可派生 impl、无用转换等。

**Files:** 多处分布

- [ ] **Step 1: 修复 24 个 collapsible `if`**

这类警告形如：
```rust
// 旧
if a { if b { ... } }
// 新
if a && b { ... }
```

运行以下命令自动检测哪些文件：
```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | grep "if statement can be collapsed"
```

逐个文件检查后，对每个 `if` 嵌套做合并。

- [ ] **Step 2: 修复 8 个 auto-deref**

```rust
// 旧: let x = &(*y);
// 新: let x = y;
```

- [ ] **Step 3: 修复 6 个 `map_or` 可简化**

```rust
// 旧: iter.map_or(0, |x| x)
// 新: iter.map_or(0, |x| x)
```
实际上是某些情况可用 `map_or_else` 替换或简化。

- [ ] **Step 4: 修复 3 个未使用导入**

```rust
// src/ai/pathfinding/mod.rs: 删除 unused use bevy::prelude::*
// src/assets/entity_db/mod.rs: 删除 unused use bevy::prelude::*
// src/assets/proximity_loader.rs: 删除 unused CollisionResponse
```

- [ ] **Step 5: 修复未使用变量 `title`**

```rust
// src/assets/loading.rs:403 — 将 `title` 改为 `_title` 或删除
```

- [ ] **Step 6: 修复 3 个可派生 impl**

```rust
// 为可以 #[derive(Default, Clone, Copy)] 的结构体添加派生宏，删除手动 impl
```

- [ ] **Step 7: 修复 5 个手动的 `RangeInclusive::contains` / `Iterator::find` / `is_multiple_of`**

```rust
// 旧: if x >= 0 && x <= 10 { ... }
// 新: if (0..=10).contains(&x) { ... }
```

- [ ] **Step 8: 修复 2 个无用类型转换**

```rust
// 旧: let x: f32 = val as f32;  // val 已经是 f32
// 新: let x: f32 = val;
```

- [ ] **Step 9: 运行 cargo clippy 验证**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | head -5
```

预期: ~50 个警告消失，剩余 ~65 个。

---

### Task 2: 修复复杂类型和函数参数

**说明：** 14 个 `complex type` 警告和 21 个 `too many arguments` 警告。需要通过类型别名和参数对象重构。

- [ ] **Step 1: 列出所有 complex type 位置**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | grep "complex type"
```

- [ ] **Step 2: 为每个 complex type 添加类型别名**

模式：
```rust
// 旧: Query<(&Transform, &Collider, &Player), (Without<Npc>, Without<Enemy>)>
// 新: type PlayerQuery = Query<(&Transform, &Collider, &Player), (Without<Npc>, Without<Enemy>)>;
```

为 `src/collision/manager.rs`、`src/collision/debug.rs`、`src/camera_motion.rs` 等文件中的复杂查询类型添加 type 别名。

- [ ] **Step 3: 列出所有 too many arguments 位置**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | grep "too many arguments"
```

- [ ] **Step 4: 为超参函数创建参数结构体**

模式：
```rust
// 旧: fn spawn_enemy(commands: &mut Commands, x: f32, y: f32, z: f32, hp: u32, speed: f32, ..)
// 新: struct EnemyConfig { x: f32, y: f32, z: f32, hp: u32, speed: f32, ... }
//     fn spawn_enemy(commands: &mut Commands, config: EnemyConfig)
```

- [ ] **Step 5: 运行 cargo clippy 验证**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | head -5
```

预期: ~35 个警告消失，剩余 ~30 个。

---

### Task 3: 处理 dead code 警告

**说明：** ~17 个 dead code 警告。需逐一判断：保留（添加 `#[allow(dead_code)]`）或删除。

- [ ] **Step 1: 列出所有 dead code 位置**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | grep "never used\|never read\|is never"
```

- [ ] **Step 2: 分类处理**

Solari 演示函数（12 个 `spawn_*` 函数）：添加 `#[allow(dead_code)]`，这些是演示关卡备用函数：
```rust
#[allow(dead_code)]
pub fn spawn_pbr_showcase(...) { ... }
```

从未读取的字段（`stop`, `position`, `height`, `radius`, `max_health`, `direction`, `core`, `waiting`）：添加 `#[allow(dead_code)]`。

未使用的关联函数（`new`, `with_patrol`）：添加 `#[allow(dead_code)]` 或删除。

未使用的函数（`btn_pair`, `spawn_td_level`）：添加 `#[allow(dead_code)]`。

- [ ] **Step 3: 运行 cargo clippy 验证**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | head -5
```

预期: ~17 个警告消失，剩余 ~13 个。

---

### Task 4: 精简 main.rs

**说明：** 从 171 行精简到 ≤50 行。提取插件聚合到 `src/lib.rs`。

**Files:**
- Modify: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: 创建 `src/lib.rs` 聚合插件**

```rust
//! NI 游戏 — 插件聚合与重导出
//!
//! 领域模块化架构：lib.rs 聚合所有领域插件，main.rs 仅启动 App。

use bevy::prelude::*;

use bevy_egui::EguiPlugin;
use render::animation::AnimationPlugin;
use audio::plugin::GameAudioPlugin;
use render::camera::{CameraControllerPlugin, CameraPlugin};
use render::camera_motion::CameraMotionPlugin;
use game::collectible::CollectiblePlugin;
use physics::collision::manager::CollisionManagerPlugin;
use game::combat::CombatPlugin;
use game::dialogue::DialoguePlugin;
use game::enemy::EnemyPlugin;
use game::inventory::InventoryPlugin;
use world::level::LevelPlugin;
use game::npc::NpcPlugin;
use game::player::PlayerPlugin;
use td::TdPlugin;
use ui::plugin::GameUiPlugin;
use world::WorldPlugin;
use render::particles::ParticlePlugin;
use world::label::WorldLabelPlugin;
use world::grid::GameGridPlugin;
use ai::plugin::AiPlugin;
use game::stealth::StealthPlugin;
use assets::loading::LoadingPlugin;

/// 核心游戏插件组 — 在 main.rs 中用一行注册
pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EguiPlugin,
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
            ParticlePlugin,
            TdPlugin,
            AiPlugin,
            GameGridPlugin,
            StealthPlugin,
            LoadingPlugin,
            CameraMotionPlugin,
            WorldLabelPlugin,
            ScalePlugin,
        ));
    }
}

/// 工具/调试插件组 — feature-gated
pub struct ToolPlugins;

impl PluginGroup for ToolPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TimeRecorderPlugin,
            ImageGalleryPlugin,
            EntityDbPlugin,
            StressTestPlugin,
            PathfindingPlugin,
            MusicPlugin,
            PlacementPlugin,
            ProximityLoaderPlugin,
            LevelToolPlugin,
            CreativePlugin,
            CameraControllerPlugin,
            PuzzlePlugin,
            CollisionDebugPlugin,
        ));
    }
}

// 重导出所有 pub use（保持 main.rs 的向后兼容性）
pub use core::*;
pub use game::*;
pub use world::*;
pub use physics::*;
pub use render::*;
pub use ai::*;
pub use audio::*;
pub use ui::*;
pub use network::*;
pub use assets::*;
pub use tools::*;
pub use td::*;
```

等等 — 这样写有问题。`pub use core::*` 这种在 lib.rs 中做 glob 重导出会把所有东西重导出到 crate 根。但我上面用了 `pub use X::*` 这会把所有领域模块的东西全部重导出到 crate 根，可能会冲突。

更好的做法是：lib.rs 只做 PluginGroup 聚合，不重导出。main.rs 保持重导出。

所以实际上应该这样：

```rust
// src/lib.rs — 仅聚合插件
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

use bevy::prelude::*;

// 导入插件...
// ... 所有 use 导入 ...

pub struct GamePlugins;
impl PluginGroup for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EguiPlugin,
            GameStatePlugin, DialoguePlugin, InventoryPlugin,
            NpcPlugin, AnimationPlugin, PlayerPlugin,
            CameraPlugin, WorldPlugin, LevelPlugin,
            GameAudioPlugin, CollectiblePlugin, CombatPlugin,
            EnemyPlugin, GameUiPlugin, CollisionManagerPlugin,
            ParticlePlugin, TdPlugin, AiPlugin, GameGridPlugin,
            StealthPlugin, LoadingPlugin, CameraMotionPlugin,
            WorldLabelPlugin, PuzzlePlugin,
        ));
    }
}

pub struct ToolPlugins;
impl PluginGroup for ToolPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TimeRecorderPlugin, ImageGalleryPlugin,
            EntityDbPlugin, StressTestPlugin, PathfindingPlugin,
            MusicPlugin, PlacementPlugin, ProximityLoaderPlugin,
            LevelToolPlugin, CreativePlugin, CameraControllerPlugin,
            CollisionDebugPlugin, ScalePlugin,
        ));
    }
}
```

main.rs 变为：
```rust
//! NI 3D 潜行恐怖游戏 — 精简入口

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

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

// 向后兼容 re-exports
pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;
pub use physics::collision;
pub use physics::collision::collider as colliders;
pub use physics::collision::manager as collision_manager;
pub use physics::collision::debug as collision_debug;
pub use physics::ray_cast;
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::pathfinding;
pub use audio::music;
pub use ui::image_gallery;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};

fn main() {
    let mut app = App::new();
    app.add_plugins(configured_plugins());

    // 核心游戏插件
    app.add_plugins(GamePlugins);
    // 工具/调试插件
    app.add_plugins(ToolPlugins);

    // 全局系统
    app.add_systems(Update, render::render_utils::animate_rotation);
    app.add_systems(Update, debug_switch_to_demo);

    // 资源初始化
    app.init_resource::<GameplayConfig>();
    app.register_type::<GameplayConfig>();
    app.register_type::<Collider>();
    app.register_type::<ColliderShape>();
    app.register_type::<CollisionLayer>();
    app.register_type::<CollisionMask>();
    app.register_type::<SmoothPush>();
    app.init_resource::<LightingDebug>();
    app.add_systems(PostUpdate, sync_lighting_to_world);

    // 调试工具
    #[cfg(debug_assertions)]
    app.add_plugins(
        WorldInspectorPlugin::new()
            .run_if(input_toggle_active(true, KeyCode::F3)),
    );

    app.run();
}
```

嗯，但这样 main.rs 还是在 50 行以上。让我重新考虑。

实际上设计文档说的是 "main.rs 不超过 50 行" 作为成功标准。当前 main.rs 171 行包含：
- 模块声明 (12行)
- use 导入 (50行)
- re-exports (15行)
- fn main (50行)
- fn debug_switch_to_demo (10行)

如果移到 lib.rs 中处理插件聚合，main.rs 仍然需要模块声明和 re-exports。50 行很紧张。

让我想想要怎么做到 ≤50 行：

方案：main.rs 只保留模块声明 + 重导出 + 精简的 main 函数

```rust
// main.rs — 终极精简版
#![allow(unused_imports)]  // re-exports 需要

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod core; mod game; mod world; mod physics;
mod render; mod ai; mod audio; mod ui;
mod network; mod assets; mod tools; mod td;

pub use core::*; pub use game::*; pub use world::*;
pub use physics::{collision, ray_cast};
pub use render::*; pub use ai::*;
pub use audio::*; pub use ui::*;
pub use assets::*; pub use tools::*; pub use td::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(configured_plugins());
    app.add_plugins(GamePlugins);
    app.add_plugins(ToolPlugins);
    app.add_plugins(EguiPlugin::default());
    app.add_systems(Update, (render_utils::animate_rotation, debug_switch_to_demo));
    app.init_resource::<GameplayConfig>()
        .register_type::<GameplayConfig>()
        .register_type::<ColliderShape>()
        .register_type::<CollisionLayer>()
        .register_type::<CollisionMask>()
        .register_type::<SmoothPush>()
        .init_resource::<LightingDebug>()
        .add_systems(PostUpdate, debug_lighting::sync_lighting_to_world);
    #[cfg(debug_assertions)]
    app.add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(true, KeyCode::F3)));
    app.run();
}

fn debug_switch_to_demo(...) { ... }
```

这大约 50 行左右。可以用链式调用压缩。让我算算：模块声明 4 行 + re-exports 4 行 + main 函数 ~30 行 + debug 函数 ~10 行 ≈ 48 行。

好，这个方案可行。

但对于 `pub use core::*; pub use game::*;` 这种 glob 重导出，可能会产生命名冲突。让我检查一下：
- `core` 有 `config`, `game_state`, `log` 模块 — 各自有 pub 项
- `game` 有 `player`, `enemy`, `npc` 等 — `pub use game::*` 会导出 `game::dialogue::DialoguePlugin` 等
- 不同模块间可能会有命名冲突

实际上，更安全的方式是保持当前的精确 re-exports，而不是用 glob。

但精确 re-exports 本身就有很多行。让我试试这种折中：

```rust
// 核心重导出
pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;
pub use physics::{collision, ray_cast};
pub use physics::collision::{collider as colliders, manager as collision_manager, debug as collision_debug};
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::pathfinding;
pub use audio::music;
pub use ui::image_gallery;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};
```

这大约 12 行。加上模块声明 4 行。main 函数 ~30 行。debug 函数 ~10 行。总共 ~56 行 — 仍然略超 50 行限制。

把 debug 函数移到另一个文件可以省 ~10 行。

让我换个思路：把 `register_type` 链也放到 lib.rs 的 PluginGroup 中。

lib.rs 添加：
```rust
impl PluginGroup for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // ... plugins ...
        ))
        .init_resource::<GameplayConfig>()
        .register_type::<GameplayConfig>()
        // ...
    }
}
```

这样 main.rs 的 main 函数可以精简到 ~15 行，总共 ~40 行。完美。

好，让我写这个计划。不过这个计划太长了，我直接在 plan 文件中写下具体方案，然后执行。

实际上，在写 plan 文件之前，让我考虑一下哪些应该放在 plan 里 vs 哪些应该直接交给 subagent。

对于复杂的 main.rs 重构，最好把精确的最终代码写在 plan 里。对于机械的 clippy 修复，只需要描述模式即可。

让我开始写 plan 文件。

关于 lib.rs 和我需要的 use 导入: 设计文档里画了一个精简入口和 PluginGroup，但需要注意 `use` 导入路径。由于 main.rs 中已经通过 `use` 语句导出所有领域模块，如果创建 lib.rs，需要用同样的模块声明。

但这里有个问题：如果我创建 `src/lib.rs` 并重写 `src/main.rs`，那么：
1. `lib.rs` 需要声明所有模块（因为 main.rs 会 `use lib::GamePlugins`）
2. `main.rs` 的模块声明和 lib.rs 的模块声明不能重复
3. 所以 main.rs 需要 `extern crate ni;` 或直接 `use ni::GamePlugins`

在 Rust 中，如果有 lib.rs 和 main.rs，main.rs 自动以 `extern crate` 方式引入 lib crate。所以 main.rs 中不需要重新声明模块，直接 `use crate::GamePlugins` 引用 lib.rs 中的定义。

但问题是，main.rs 中的 re-exports (`pub use game::player`) 需要 lib.rs 中的模块可见。如果模块声明全部移到 lib.rs，main.rs 中就需要用 `use ni::game::player` 来引用。

更简单的方式：不在 main.rs 中声明模块，只在 lib.rs 中声明。main.rs 仅保留 main 函数 + 重导出。

```rust
// lib.rs
mod core;
mod game;
// ... etc

pub use core::*;
// ... re-exports ...

pub struct GamePlugins;
impl PluginGroup for GamePlugins { ... }
```

```rust
// main.rs
use ni::{GamePlugins, ToolPlugins, configured_plugins};
// 重导出
pub use ni::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(configured_plugins());
    app.add_plugins(GamePlugins);
    app.add_plugins(ToolPlugins);
    app.add_plugins(EguiPlugin::default());
    #[cfg(debug_assertions)]
    app.add_plugins(WorldInspectorPlugin::new().run_if(...));
    app.run();
}

fn debug_switch_to_demo(...) { ... }
```

这样 main.rs 可以做到 ~40 行。很好。

不过有一个问题：当前的 `main.rs` 中有一些 `app.register_type` 调用。这些需要移到 PluginGroup 的 build 方法中。

另一个问题: `configured_plugins()` 来自 `core::log`。如果 lib.rs 也有模块声明，main.rs 需要 `use ni::core::log::configured_plugins`。或者可以直接把它放在 PluginGroup 中。

还有一个问题: 那些 render_utils 的系统添加。这些可以放在 GamePlugins 或 ToolPlugins 中。

好，这个方案是可行的但比较复杂。让我把完整的计划写出来。

不过，写一个这么复杂的 plan 文档然后读给 subagent 做，可能不如我直接手动执行效率高。因为 Phase 2 也是我写 plan + 执行，但现在用户想继续而且已经有了设计文档，也许我应该简化流程。

实际上，让我直接创建 plan 文件 + 开始执行。这个 plan 的重点是：

1. 修复 clippy — 按模式分组，逐个文件修复
2. 精简 main.rs — 创建 lib.rs 提取 PluginGroup
3. 减少 unwrap
4. 添加测试

Let me write the plan file efficiently since there's a lot of content.<｜end▁of▁thinking｜>

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="Write">
<｜｜DSML｜｜parameter name="content" string="true"># Phase 3 & 4: 代码质量与测试 CI 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 消除所有 clippy 警告，精简 main.rs 到 ≤50 行，减少 unwrap，添加测试覆盖

**Architecture:** 4+2 个 Task：Task 1-3 处理 clippy 警告，Task 4 精简 main.rs + lib.rs，Task 5 减少 unwrap，Task 6 添加测试

**前提条件:** Phase 2 完成，cargo check 0 errors

**当前基线:**
- 115 clippy warnings
- main.rs 171 行
- 11 处 unwrap()（4 处生产代码）
- 8 个 `#[cfg(test)]` 模块，0 个集成测试

---

### Task 1: 修复机械性 clippy 警告 (~50个)

**说明：** 修复最简单、最机械的警告：collapsible `if`、auto-deref、`map_or`、未使用导入、可派生 impl、无用转换等。

**Files:** 多处分布

- [ ] **Step 1: 修复 24 个 collapsible `if`**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo clippy 2>&1 | grep "if statement can be collapsed"
```

手动合并嵌套 `if`：
```rust
// 旧
if a { if b { ... } }
// 新
if a && b { ... }
```

- [ ] **Step 2: 修复 8 个 auto-deref**

去除多余的 `&` 或 `*` 操作符。

- [ ] **Step 3: 修复 6 个 `map_or` 简化**

将 `option.map_or(...)` 简化为 `option.map_or_else(...)` 或直接取值。

- [ ] **Step 4: 修复 3 个未使用导入**

```rust
// 删除:
// src/ai/pathfinding/mod.rs: use bevy::prelude::*;
// src/assets/entity_db/mod.rs: use bevy::prelude::*;
// src/assets/proximity_loader.rs: use CollisionResponse;
```

- [ ] **Step 5: 修复未使用变量 `title`**

`src/assets/loading.rs:403` — `title` → `_title`

- [ ] **Step 6: 修复 3 个可派生 impl**

为结构体添加 `#[derive(Default, Clone, Copy)]`，删除手动 impl。

- [ ] **Step 7: 修复 5 个手动范围/迭代器**

替换为 `(0..=10).contains(&x)`、`iter.any()`、`x.is_multiple_of(y)`。

- [ ] **Step 8: 修复 2 个无用类型转换**

```rust
// 旧: x as f32  // x 已是 f32
// 新: x
```

- [ ] **Step 9: cargo clippy 验证**

预期: ~50 个警告消失。

---

### Task 2: 修复复杂类型和函数参数 (~35个)

- [ ] **Step 1: 14 个 complex type — 添加类型别名**

为复杂查询类型添加 `type` 别名。

- [ ] **Step 2: 21 个 too many arguments — 创建参数结构体**

为超参数函数创建 `struct XxxConfig`。

- [ ] **Step 3: cargo clippy 验证**

预期: ~35 个警告消失。

---

### Task 3: 处理 dead code 警告 (~17个)

- [ ] **Step 1: Solari 演示函数** — 添加 `#[allow(dead_code)]`

`spawn_pbr_showcase`, `spawn_reflective_floor`, `spawn_material_grid`, `spawn_center_torus`, `spawn_glass_collection`, `spawn_mirror_collection`, `spawn_emissive_towers`, `spawn_point_lights`, `spawn_directional_light`, `spawn_free_camera`, `spawn_help_text`, `spawn_glowing_cube`

- [ ] **Step 2: 未读字段** — 添加 `#[allow(dead_code)]`

`stop`, `position`, `height`, `radius`, `max_health`, `direction`, `core`, `waiting`

- [ ] **Step 3: 未使用函数/关联函数** — 添加 `#[allow(dead_code)]` 或删除

`btn_pair`, `spawn_td_level`, `NavGraph::new`, `with_patrol`

- [ ] **Step 4: 修复 private type in pub interface**

`render::toon::outline::OutlineEntity` — 改成 `pub use` 或添加 `pub`。

- [ ] **Step 5: cargo clippy 验证**

预期: ~17 个警告消失。

---

### Task 4: 精简 main.rs + 创建 lib.rs

**Files:**
- Create: `src/lib.rs`
- Modify: `src/main.rs`

**当前:** main.rs 171 行，混合模块声明 + use 导入 + re-exports + 插件注册 + 系统添加 + 资源初始化

**目标:** main.rs ≤50 行

- [ ] **Step 1: 创建 `src/lib.rs`**

```rust
//! NI 3D 游戏 — 库入口，聚合所有领域模块和插件

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

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use render::animation::AnimationPlugin;
use audio::plugin::GameAudioPlugin;
use render::camera::{CameraControllerPlugin, CameraPlugin};
use render::camera_motion::CameraMotionPlugin;
use game::collectible::CollectiblePlugin;
use physics::collision::manager::CollisionManagerPlugin;
use game::combat::CombatPlugin;
use core::config::GameplayConfig;
use game::dialogue::DialoguePlugin;
use game::enemy::EnemyPlugin;
use game::inventory::InventoryPlugin;
use world::level::LevelPlugin;
use game::npc::NpcPlugin;
use game::player::PlayerPlugin;
use td::TdPlugin;
use ui::plugin::GameUiPlugin;
use world::WorldPlugin;
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
use ai::plugin::AiPlugin;
use game::stealth::StealthPlugin;
use physics::collision::debug::CollisionDebugPlugin;
use assets::loading::LoadingPlugin;
use audio::music::MusicPlugin;
use world::placement::PlacementPlugin;
use assets::proximity_loader::ProximityLoaderPlugin;
use tools::creative::CreativePlugin;
use world::level_tool::LevelToolPlugin;
use core::log::configured_plugins;
use network::plugin::NetworkPlugin;
use physics::collision::collider::{Collider, ColliderShape, CollisionLayer, CollisionMask, SmoothPush};
use render::debug_lighting::{LightingDebug, sync_lighting_to_world};
use render::render_utils;

pub use core::{config, game_state, log};
pub use game::{player, enemy, npc, dialogue, combat, stealth, collectible, puzzle, inventory, solari_demo};
pub use world::{level, level_tool, grid, placement, nav_mesh, terrain};
pub use world::label as world_label;
pub use physics::collision;
pub use physics::collision::collider as colliders;
pub use physics::collision::manager as collision_manager;
pub use physics::collision::debug as collision_debug;
pub use physics::ray_cast;
pub use render::{toon, camera, camera_motion, particles, animation, scale, debug_lighting, render_utils};
pub use ai::pathfinding;
pub use audio::music;
pub use ui::image_gallery;
pub use assets::{loading, entity_db, proximity_loader};
pub use tools::{creative, stress_test, time_recorder};
pub use td::TdPlugin;

/// 核心游戏插件组
pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameStatePlugin, DialoguePlugin, InventoryPlugin,
            NpcPlugin, AnimationPlugin, PlayerPlugin,
            CameraPlugin, WorldPlugin, LevelPlugin,
            GameAudioPlugin, CollectiblePlugin, CombatPlugin,
            EnemyPlugin, GameUiPlugin, CollisionManagerPlugin,
        ));
        app.add_plugins((
            ParticlePlugin, PuzzlePlugin, AiPlugin, GameGridPlugin,
            StealthPlugin, LoadingPlugin, CameraMotionPlugin,
            WorldLabelPlugin, ScalePlugin,
        ));
        app.add_plugins(CameraControllerPlugin);
        app.init_resource::<GameplayConfig>();
        app.register_type::<GameplayConfig>();
        app.register_type::<Collider>();
        app.register_type::<ColliderShape>();
        app.register_type::<CollisionLayer>();
        app.register_type::<CollisionMask>();
        app.register_type::<SmoothPush>();
        app.init_resource::<LightingDebug>();
        app.add_systems(PostUpdate, sync_lighting_to_world);
        app.add_systems(Update, render_utils::animate_rotation);
    }
}

/// 工具/调试插件组
pub struct ToolPlugins;

impl PluginGroup for ToolPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TimeRecorderPlugin, ImageGalleryPlugin,
            EntityDbPlugin, StressTestPlugin, PathfindingPlugin,
            MusicPlugin, PlacementPlugin, ProximityLoaderPlugin,
            LevelToolPlugin, CreativePlugin, TdPlugin,
            CollisionDebugPlugin,
        ));
    }
}
```

注意：`TdPlugin` 同时出现在 `GamePlugins` 和 `ToolPlugins` 会报错！需要决定放在哪边。由于塔防是游戏的一部分，应放在 `GamePlugins`。

- [ ] **Step 2: 重写 `src/main.rs`**（目标 ≤50 行）

```rust
//! NI 3D 潜行恐怖游戏 — 精简入口

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use core::log::configured_plugins;

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use ni::{GamePlugins, ToolPlugins};

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((GamePlugins, ToolPlugins));
    app.add_plugins(EguiPlugin::default());

    #[cfg(debug_assertions)]
    app.add_plugins(
        WorldInspectorPlugin::new()
            .run_if(input_toggle_active(true, KeyCode::F3)),
    );

    app.run();
}
```

等等 — 如果 `lib.rs` 声明了 `mod core;` 等模块，在 main.rs 中 `use ni::{GamePlugins, ToolPlugins}` 引用了 lib crate。那么 main.rs 本身就不能再声明 `mod core;` 了。

但是当前的 `main.rs` 中的重导出 (`pub use core::*`) 需要模块声明。如果模块声明移到了 lib.rs，重导出也需要在 lib.rs 中。

而 `NetworkPlugin` 需要在 main.rs 中额外添加（因为它是条件编译的）。

让我重写：

**lib.rs** — 包含模块声明、use 导入、re-exports、PluginGroups

**main.rs** — 仅 main 函数 + debug 函数，不需要 `mod` 声明、不需要 re-exports

```rust
// main.rs
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ni::core::log::configured_plugins;
use ni::network::plugin::NetworkPlugin;

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use ni::{GamePlugins, ToolPlugins};

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((GamePlugins, ToolPlugins));
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
    mut events: MessageWriter<ni::world::level::LoadLevelEvent>,
    mut phase: ResMut<NextState<ni::core::game_state::GamePhase>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        info!("[Debug] F5 → 跳转到 Demo 关卡");
        phase.set(ni::core::game_state::GamePhase::Playing);
        events.write(ni::world::level::LoadLevelEvent { level: ni::world::level::GameLevel::Demo });
    }
}
```

但这样 main.rs 中的 `debug_switch_to_demo` 如何作为 system 运行？需要 `use ni::*` 并把它添加到 Update。但如果 `lib.rs` 中已经有了 `animate_rotation` 的 system 添加，这个 system 还需要在 main.rs 中添加。

实际上 `debug_switch_to_demo` 是调试用的，可以放在 lib.rs 的 ToolPlugins 中。或者留在 main.rs 中。

让我简化：main.rs 只保留 main 函数和 debug 函数。

```rust
// main.rs (目标 ~30行)
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ni::core::log::configured_plugins;
use ni::network::plugin::NetworkPlugin;
use ni::{GamePlugins, ToolPlugins};

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((GamePlugins, ToolPlugins));
    app.add_plugins(EguiPlugin::default());
    app.add_systems(Update, debug_switch_to_demo);
    #[cfg(debug_assertions)]
    app.add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(true, KeyCode::F3)));
    app.run();
}

fn debug_switch_to_demo(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<ni::world::level::LoadLevelEvent>,
    mut phase: ResMut<NextState<ni::core::game_state::GamePhase>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        info!("[Debug] F5 → 跳转到 Demo 关卡");
        phase.set(ni::core::game_state::GamePhase::Playing);
        events.write(ni::world::level::LoadLevelEvent { level: ni::world::level::GameLevel::Demo });
    }
}
```

这大约 30 行。完美。

但问题是：`ni::world::level::LoadLevelEvent` 这些长路径看起来很冗长。以前在 main.rs 中直接 `use world::level::*` 就可以。现在需要通过 `use ni::` 引用。

我们可以添加一些 `use ni::*` 来简化。但因为 `lib.rs` 中有 `pub use world::*`，所以 `use ni::*` 应该能导入 `LoadLevelEvent` 等。

好，让我继续写计划。

主要的挑战是：需要确保 lib.rs 的模块路径与所有现有的 `crate::` 引用兼容。Phase 1 中我们已经把 `crate::collision` 等路径通过 main.rs 的 `pub use` 暴露。现在模块声明移到 lib.rs 后，这些路径仍然有效因为 lib.rs 也会 `pub use`。

但等等 — 如果 main.rs 声明了 `mod core;` 但现在移到 lib.rs，之前 `crate::core::*` 路径会变成 `ni::core::*`。不过 `crate` 在 lib 语境下就是 `ni`，在 main 语境下就是 binary。如果一个模块用了 `crate::player::Player`，它解析到的是 main.rs 中的 `pub use game::player`。如果 main.rs 不再有模块声明，这些路径需要 lib.rs 中的 `pub use`。

实际上，自引用 (`use crate::...`) 在 Rust 中是从 crate 根解析的。如果模块在 lib.rs 中声明，lib.rs 内的 `crate::` 引用 lib crate；main.rs 内的 `crate::` 引用 binary crate。所以如果 main.rs 不声明模块、只 `use ni::*`，那么 main.rs 中的 `crate::` 是无效的。

但所有其他模块（在 lib.rs 声明的）中的 `crate::` 引用是 lib crate，这就没问题。

重要的是：之前 main.rs 的 re-exports (`pub use game::player as player`) 让所有模块可以通过 `crate::player::Player` 访问。如果 main.rs 不再做 re-export，那么就需要 lib.rs 来做。

这就是为什么 lib.rs 需要包含所有这些 re-exports。

现在，值得担心的一个问题是：当前所有模块都是用 `crate::` 来引用其他模块。在 Phase 1 中，所有模块都通过这些路径工作。如果我把模块声明从 main.rs 移到 lib.rs，`crate::` 路径在 lib crate 中仍然解析正确。但这次 action 不会改动其他模块中的 crate:: 路径，它们仍然会解析到 `ni::`，也就是 lib crate。

这应该没问题，因为 lib.rs 做了和 main.rs 相同的 re-exports。

好，让我确认这个方案可行，然后把计划写完。

还有一个重要问题：当前的 `Cargo.toml` 可能没有配置 `[[bin]]` 和 `[lib]`。如果既有 `src/main.rs` 又有 `src/lib.rs`，Cargo 自动识别为 binary + library。不需要额外配置。但需要确保 lib crate 的名字不冲突。默认情况下，binary 名字是 `ni`，library 名字也是 `ni`。这可能会冲突。

实际上，对于 Bevy 项目，`src/main.rs` 是默认 binary。添加 `src/lib.rs` 后，Cargo 自动创建一个同名的 lib crate。两者可以共存（`cargo run` 运行 binary，`cargo test` 测试 lib）。

但要注意：如果 main.rs 和 lib.rs 都声明 `mod core;`，会产生重复。所以模块声明只能在一个地方。

方案：模块声明只在 lib.rs 中。main.rs 不声明任何模块，通过 `use ni::*` 引用 lib crate。

好，这个方案明确且可行。

ok let me continue writing the plan file.

Actually wait, the plan is getting extremely long. Let me just save a concise but complete plan and then start executing it.<｜end▁of▁thinking｜>

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="Write">
<｜｜DSML｜｜parameter name="content" string="true"># Phase 3 & 4: 代码质量与测试 CI 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 消除所有 clippy 警告，精简 main.rs 到 ≤50 行，减少 unwrap，添加测试覆盖

**基线:**
- 115 clippy warnings  |  main.rs 171 行  |  11 处 unwrap  |  8 个 #[cfg(test)]  |  0 集成测试

---

### Task 1: 修复机械性 clippy 警告（collapsible if、deref、map_or、未使用导入等 ~50个）

机械修复以下几类，按文件逐个修改：
- 24 collapsible `if`: 合并嵌套 `if a { if b { } }` → `if a && b { }`
- 8 auto-deref: 删除多余的 `&`/`*`
- 6 `map_or` 简化
- 3 未使用导入: `ai/pathfinding/mod.rs`, `assets/entity_db/mod.rs`, `assets/proximity_loader.rs`
- 1 未使用变量: `loading.rs:403` `title` → `_title`
- 3 可派生 impl: 加 `#[derive(Default, Clone)]` 等
- 5 手动范围/迭代器: 用 `contains`、`any`、`is_multiple_of`
- 2 无用转换: 删除 `as f32`

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
# 自动修复 collapsible if 等:
cargo clippy --fix --allow-dirty 2>&1 | tail -5
cargo clippy 2>&1 | head -5
```

---

### Task 2: 修复复杂类型和函数参数（~35个）

- 14 complex type: `type FooQuery = Query<(...)>;`
- 21 too many arguments: 为超参函数创建 `XxxConfig` 参数结构体

---

### Task 3: 处理 dead code（~17个）

- 12 Solari 演示函数: 加 `#[allow(dead_code)]`
- 未读字段: 加 `#[allow(dead_code)]`
- private type in pub interface: `OutlineEntity` 修复可见性

---

### Task 4: 精简 main.rs + 创建 lib.rs

**核心思路:** 模块声明全部移到 `src/lib.rs`，main.rs 仅保留 main 函数 + debug 函数。

- [ ] **Step 1: 读取当前 main.rs 完整内容**
- [ ] **Step 2: 创建 `src/lib.rs`** 包含:
  - `mod` 声明所有 12 个领域模块
  - `use` 导入所有插件
  - re-exports（保持 `crate::` 路径可用）
  - `GamePlugins` 和 `ToolPlugins` 两个 PluginGroup
  - `register_type`、`init_resource`、`add_systems` 等移到 GamePlugins
- [ ] **Step 3: 重写 `src/main.rs`** ≤50 行:

```rust
//! NI 3D 潜行恐怖游戏 — 精简入口
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ni::core::log::configured_plugins;
use ni::network::plugin::NetworkPlugin;
use ni::{GamePlugins, ToolPlugins};

#[cfg(debug_assertions)]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins((GamePlugins, ToolPlugins));
    app.add_plugins(EguiPlugin::default());
    app.add_systems(Update, debug_switch_to_demo);
    #[cfg(debug_assertions)]
    app.add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(true, KeyCode::F3)));
    app.run();
}

fn debug_switch_to_demo(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<ni::world::level::LoadLevelEvent>,
    mut phase: ResMut<NextState<ni::core::game_state::GamePhase>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        info!("[Debug] F5 → 跳转到 Demo 关卡");
        phase.set(ni::core::game_state::GamePhase::Playing);
        events.write(ni::world::level::LoadLevelEvent { level: ni::world::level::GameLevel::Demo });
    }
}
```

- [ ] **Step 4: cargo check 验证**

---

### Task 5: 减少生产代码 unwrap（4处）

```rust
// src/network/plugin.rs:45
// 旧: let json = serde_json::to_string(self).unwrap();
// 新: let json = serde_json::to_string(self).unwrap_or_default();
//    或: .unwrap_or_else(|e| { error!("序列化失败: {}", e); String::new() })

// src/network/plugin.rs:449
// 旧: let s = stream.as_mut().unwrap();
// 新: let Some(s) = stream.as_mut() else { return; };

// src/physics/collision/manager.rs:497
// 旧: closest_hit.as_ref().unwrap().distance
// 新: 用 if let Some 或 match
```

---

### Task 6: 添加测试

- [ ] **在 `src/physics/collision/collider.rs` 添加测试:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    #[test]
    fn plane_ground_height() {
        let collider = Collider::new(ColliderShape::Plane { normal: Vec3::Y, distance: 0.0 });
        let transform = Transform::IDENTITY;
        assert_eq!(collider.ground_height_at(&transform, Vec2::ZERO), Some(0.0));
        assert_eq!(collider.ground_height_at(&transform, Vec2::new(100.0, 100.0)), Some(0.0));
    }

    #[test]
    fn box_ground_height_inside() {
        let collider = Collider::new(ColliderShape::Box { half_extents: Vec3::splat(1.0) });
        let transform = Transform::from_xyz(0.0, 5.0, 0.0);
        assert_eq!(collider.ground_height_at(&transform, Vec2::ZERO), Some(6.0));
    }

    #[test]
    fn box_ground_height_outside() {
        let collider = Collider::new(ColliderShape::Box { half_extents: Vec3::splat(1.0) });
        let transform = Transform::from_xyz(0.0, 5.0, 0.0);
        assert_eq!(collider.ground_height_at(&transform, Vec2::new(10.0, 0.0)), None);
    }
}
```

- [ ] **在 `src/game/dialogue/branch.rs` 添加测试:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dialogue::types::*;

    #[test]
    fn apply_start_quest_effect() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::StartQuest("test_quest".into())];
        let pending = apply_effects(&effects, &mut tracker);
        assert!(tracker.active_quests.contains(&"test_quest".into()));
        assert!(pending.is_empty());
    }

    #[test]
    fn apply_give_item_effect() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::GiveItem("sword".into(), 1)];
        let pending = apply_effects(&effects, &mut tracker);
        assert_eq!(pending.len(), 1);
        match &pending[0] {
            PendingEffect::GiveItem(id, amount) => {
                assert_eq!(id, "sword");
                assert_eq!(*amount, 1);
            }
            _ => panic!("expected GiveItem"),
        }
    }
}
```

- [ ] **在 `src/td/balance.rs` 添加测试**（为现有测试补充覆盖率）

- [ ] **创建 `tests/` 目录和集成测试**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
mkdir -p tests
```

```rust
// tests/collision_test.rs
// 集成测试：碰撞系统
```

- [ ] **运行所有测试验证**

```bash
cd "E:/游戏制作项目/从0开始-3d/ni"
cargo test 2>&1
```
