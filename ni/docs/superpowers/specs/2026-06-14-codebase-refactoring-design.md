# NI 游戏代码重构设计

## 概述

对 NI 3D 潜行恐怖游戏进行全面的代码重构，参考 GitHub 上优秀 Bevy 游戏项目的架构模式和代码组织方式。

## 参考项目

- **BioDynasties3** (ionox0) — 领域模块化架构、事件驱动设计
- **bevy_quickstart** (TheBevyFlock) — 屏幕驱动、入口精简
- **bevy-td-sandbox** (n8behavior) — 塔防子系统组织

## 当前问题

| 问题 | 详情 |
|------|------|
| 文件过大 | creative.rs(953行)、dialogue.rs(855行) 超过800行上限 |
| 模块扁平化 | 45+ 模块在 main.rs 中平铺声明 |
| 入口臃肿 | main.rs 混合插件注册、系统添加和资源初始化 |
| 新旧系统共存 | collision.rs + colliders.rs 两套碰撞系统运行 |
| 调试代码混合 | 调试工具和产品代码没有 feature flag 分离 |
| 系统间耦合 | 系统直接操作其他系统组件，缺乏事件通信模式 |

## 架构设计

### 新目录结构

```
src/
├── main.rs                  # 入口，仅注册顶级插件
├── lib.rs                   # 插件聚合导出
├── core/                    # 核心基础设施
│   ├── mod.rs
│   ├── config.rs            # GameplayConfig
│   ├── game_state.rs        # GamePhase 状态机 + 事件
│   └── log.rs               # 日志/窗口配置
├── game/                    # 游戏玩法
│   ├── mod.rs
│   ├── player/              # 玩家系统
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   ├── input.rs
│   │   └── movement.rs
│   ├── enemy/               # 敌人
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   └── systems.rs
│   ├── npc/                 # NPC + 对话
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   └── dialogue/        # 对话子系统
│   │       ├── mod.rs
│   │       ├── plugin.rs
│   │       ├── bubble.rs
│   │       └── branch.rs
│   ├── combat/              # 战斗
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── stealth/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── collectible/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── puzzle/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   └── inventory/
│       ├── mod.rs
│       └── plugin.rs
├── td/                      # 塔防（保持现有子模块）
│   ├── mod.rs
│   ├── data.rs
│   ├── balance.rs
│   ├── events.rs
│   ├── level.rs / level_data.rs
│   ├── wave.rs
│   ├── turret.rs
│   ├── projectile.rs
│   ├── enemy.rs
│   └── spatial/
├── world/
│   ├── mod.rs
│   ├── level/
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   ├── events.rs
│   │   ├── solari.rs
│   │   └── tool.rs
│   ├── grid/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── placement/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── nav_mesh/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── terrain/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   └── label.rs
├── physics/
│   ├── mod.rs
│   └── collision/
│       ├── mod.rs
│       ├── shape.rs         # 从 collision.rs 迁移
│       ├── collider.rs      # 从 colliders.rs 迁移
│       ├── manager.rs       # 从 collision_manager.rs 迁移
│       └── debug.rs         # 从 collision_debug.rs 迁移
│   └── ray_cast.rs
├── render/
│   ├── mod.rs
│   ├── toon/                # 保持现有
│   │   ├── mod.rs
│   │   ├── material.rs
│   │   ├── outline.rs
│   │   └── ramp.rs
│   ├── camera/
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   └── motion.rs
│   ├── particles/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── animation/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── scale/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── debug_lighting.rs
│   └── utils.rs
├── ai/
│   ├── mod.rs
│   ├── plugin.rs
│   ├── behaviors.rs
│   ├── sensors.rs
│   ├── goals.rs
│   └── pathfinding/         # 从 pathfinding/ 移入
│       ├── mod.rs
│       ├── astar.rs
│       ├── graph.rs
│       ├── components.rs
│       └── systems.rs
├── audio/
│   ├── mod.rs
│   ├── plugin.rs
│   ├── sfx.rs
│   └── music.rs
├── ui/
│   ├── mod.rs
│   ├── plugin.rs
│   ├── hud.rs
│   └── gallery.rs
├── network/
│   ├── mod.rs
│   └── plugin.rs
├── assets/
│   ├── mod.rs
│   ├── loading/
│   │   ├── mod.rs
│   │   └── plugin.rs
│   ├── entity_db/
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   └── plugin.rs
│   └── proximity_loader.rs
└── tools/
    ├── mod.rs
    ├── creative/
    │   ├── mod.rs
    │   ├── plugin.rs
    │   ├── brush.rs
    │   └── palette.rs
    ├── debug/
    │   ├── mod.rs
    │   └── plugin.rs
    ├── stress_test/
    │   ├── mod.rs
    │   └── plugin.rs
    └── time_recorder/
        ├── mod.rs
        └── plugin.rs
```

### 入口设计

```rust
// main.rs — 精简入口
fn main() {
    let mut app = App::new();
    app.add_plugins((configured_plugins(), NetworkPlugin));
    app.add_plugins(GamePlugins);       // lib.rs 中聚合
    app.add_plugins(ToolsPlugins);       // 调试/工具 (feature-gated)
    app.run();
}

// lib.rs — 插件聚合
pub struct GamePlugins;
impl PluginGroup for GamePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CorePlugin,                 // config, game_state, log
            GameplayPlugin,             // player, enemy, npc, combat, stealth, etc.
            WorldPlugin,                // level, grid, placement, nav_mesh, terrain
            PhysicsPlugin,              // collision, ray_cast
            RenderPlugin,               // toon, camera, particles, animation
            AudioPlugin,                // sfx, music
            UiPlugin,                   // hud, gallery
            AssetsPlugin,               // loading, entity_db, proximity_loader
            TdPlugin,                   // tower defense (existing)
            AiPlugin,                   // ai + pathfinding
        ));
    }
}
```

### 事件驱动模式

系统间通信统一使用 Bevy Events（Message），取代直接组件访问：

```rust
// 事件定义集中在 core/game_state.rs
pub struct DamagePlayerEvent(pub u32);
pub struct CollectItemEvent;
pub struct LevelCompleteEvent;
pub struct EnemyDeathEvent { pub position: Vec3, pub reward: u32 };

// 生产者系统 write 事件
fn enemy_attack(mut events: MessageWriter<DamagePlayerEvent>, ...) { ... }

// 消费者系统 read 事件  
fn health_system(mut events: MessageReader<DamagePlayerEvent>, ...) { ... }
```

### Feature Flag 控制

```toml
# Cargo.toml
[features]
default = []
dev-tools = []    # 调试面板、碰撞调试、压力测试
creative = []     # 创造模式
network = []      # 网络功能
```

## 实施阶段

### 阶段一：模块重组（当前）
创建新目录结构，迁移文件，更新 imports，简化入口。

### 阶段二：系统解耦
拆分超限文件，合并碰撞系统，事件化通信。

### 阶段三：代码质量
统一命名规范，错误处理规范化，减少 unwrap。

### 阶段四：测试与CI
按模块添加测试，配置 clippy/audit。

## 成功标准

- [ ] `cargo check` 通过无警告
- [ ] `cargo clippy` 无警告
- [ ] `main.rs` 不超过 50 行
- [ ] 无文件超过 800 行
- [ ] 测试覆盖核心系统
