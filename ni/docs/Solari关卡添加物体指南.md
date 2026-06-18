# Solari 光追关卡 — 添加物体指南

> 适用范围：`ni/src/solari_demo.rs` | 关卡 ID: `solari` | 切换键: `6`

---

## 一、核心要求

### 1. 材质：必须使用 `StandardMaterial`

Solari 实时光追渲染器**只支持** `StandardMaterial`。游戏中的 `ToonMaterial`、`ColorMaterial` 或其他自定义着色器材质在 Solari 下不会正确渲染。

```rust
// ✅ 正确
MeshMaterial3d(StandardMaterial {
    base_color: Color::srgb(1.0, 0.5, 0.0),
    metallic: 0.5,
    perceptual_roughness: 0.3,
    ..default()
})

// ❌ 错误 — Solari 下不可见
MeshMaterial3d(ToonMaterial { .. })
```

### 2. 组件：必须添加 `LevelEntity`

所有 Solari 物体必须带 `LevelEntity` 标记。退出关卡时 `exit_solari()` 会清理所有带此标记的实体，**不加会导致残留或内存泄漏**。

```rust
commands.spawn((
    Mesh3d(mesh),
    MeshMaterial3d(material),
    LevelEntity,              // ← 必须
    Name::new("Solari_XXX"),  // ← 建议带 Solari_ 前缀，方便调试
));
```

### 3. 注册到正确入口

所有物体在 `spawn_pbr_showcase()` 中统一生成，新物体也在该函数中调用：

```rust
pub(crate) fn spawn_pbr_showcase(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // 已有
    spawn_reflective_floor(commands, meshes, materials);
    // ...
    spawn_your_new_object(commands, meshes, materials);  // ← 加在这里
}
```

---

## 二、材质辅助函数

项目中已封装好两个快速创建材质的辅助函数：

### `pbr_mat` — 通用 PBR 材质

```rust
fn pbr_mat(
    materials: &mut Assets<StandardMaterial>,
    base_color: Color,
    metallic: f32,
    roughness: f32,
    emissive: LinearRgba,
) -> Handle<StandardMaterial>
```

| 参数 | 说明 | 典型值 |
|------|------|--------|
| `base_color` | 基底颜色 | `Color::srgb(r, g, b)` |
| `metallic` | 金属度 | `0.0`(非金属) / `0.5`(半金属) / `1.0`(镜面) |
| `roughness` | 粗糙度 | `0.05`(光滑) ~ `0.9`(粗糙) |
| `emissive` | 自发光 | `LinearRgba::BLACK`(无) / `LinearRgba::from(color) * 3.0`(发光) |

使用示例：

```rust
// 金属球（镜面反射）
pbr_mat(materials, Color::srgb(0.85, 0.85, 0.88), 1.0, 0.05, LinearRgba::BLACK)

// 发光柱
pbr_mat(materials, Color::srgb(1.0, 0.2, 0.1), 0.0, 0.4, LinearRgba::from(color) * 3.0)
```

### `glass_mat` — 透明玻璃材质

```rust
fn glass_mat(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    roughness: f32,
) -> Handle<StandardMaterial>
```

内部自动设置 `metallic: 0.0` 和 `alpha_mode: AlphaMode::Blend`。颜色使用 `Color::srgba()` 控制透明度：

```rust
// 透明玻璃球
glass_mat(materials, Color::srgba(0.70, 0.85, 1.0, 0.35), 0.05)
```

---

## 三、动画

需要旋转的物体添加 `Rotating` 组件（已在 `animate_solari()` 系统中自动处理）：

```rust
#[derive(Component)]
pub(super) struct Rotating(pub f32);  // 角速度（弧度/秒）

// 使用：0.8 弧度/秒绕 Y 轴旋转
Rotating(0.8),
```

---

## 四、碰撞与过渡

Solari 的碰撞平面（阻挡玩家行走）和过渡触发器（传送到其他关卡）**不由物体组件控制**，而是通过 `assets/zones/solari.ron` 配置：

```ron
(
    id: "solari",
    floor_size: 20.0,
    spawn_point: (0.0, 0.5, 6.0),
    transitions: [(target_zone: "blue_forest", ...)],
)
```

如果需要新物体有碰撞，需在此 `.ron` 文件中添加对应的碰撞平面或过渡触发区域。

---

## 五、光照

Solari 关卡使用三种光源，新物体受这些光源影响：

| 光源类型 | 数量 | 说明 |
|----------|------|------|
| `PointLight` | 3 个 | 暖橙/冷蓝/紫，强度 2500-3000，范围 12 |
| `DirectionalLight` | 1 个 | 微弱环境光，`OVERCAST_DAY` 亮度 |
| 自发光物体 | 3 根塔 | 红/绿/蓝，emissive ×3，提供间接光照 |

不需要额外添加光源来照亮新物体，除非需要特定的彩色照明效果。

---

## 六、完整示例

添加一个自发光浮动立方体的完整步骤：

```rust
// 1. 在 solari_demo.rs 中写生成函数
fn spawn_glowing_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let color = Color::srgb(1.0, 0.8, 0.2);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.5)).mesh().build())),
        MeshMaterial3d(pbr_mat(
            materials,
            color,
            0.0,
            0.3,
            LinearRgba::from(color) * 2.0,  // 自发光 ×2
        )),
        Transform::from_xyz(0.0, 2.5, 0.0),
        Rotating(0.5),
        LevelEntity,
        Name::new("Solari_GlowingCube"),
    ));
}

// 2. 在 spawn_pbr_showcase() 末尾调用
spawn_glowing_cube(commands, meshes, materials);
```

---

## 七、添加 GLB 模型

Solari 支持 GLB 模型。Bevy 加载 GLB 时将材质自动转为 `StandardMaterial`，和 Solari 兼容。

### 方式一：SceneRoot 加载（推荐）

```rust
fn spawn_glb_scene(commands: &mut Commands, assets: &AssetServer) {
    commands.spawn((
        SceneRoot(assets.load("models/your_model.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
        LevelEntity,
        Name::new("Solari_GLBModel"),
    ));
}
```

**⚠️ 注意：** GLB 场景树中的子实体默认没有 `LevelEntity`。已在 `level.rs` 中将 `exit_solari()` 的 `despawn()` 改为 `despawn_recursive()`，确保递归销毁整个场景树。

### 方式二：手动拆解网格（精确控制）

```rust
fn spawn_glb_parts(
    commands: &mut Commands,
    gltfs: &Assets<Gltf>,
    meshes: &mut Assets<Mesh>,
    assets: &AssetServer,
) {
    let Some(gltf) = gltfs.get(assets.load("models/your_model.glb")) else { return };
    for (i, mesh) in gltf.meshes.iter().enumerate() {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(gltf.materials[i].clone()),
            Transform::IDENTITY,
            LevelEntity,
            Name::new(format!("Solari_GLBPart_{}", i)),
        ));
    }
}
```

### GLB 材质兼容性

| GLB 材质类型 | Solari 兼容？ | 说明 |
|-------------|-------------|------|
| 标准 PBR 材质 | ✅ | 绝大多数 GLB 模型 |
| 透明材质 | ✅ | 自动映射到 `AlphaMode::Blend` |
| 自发光材质 | ✅ | 光追下产生真实间接光照 |
| 自定义着色器扩展 | ❌ | 可能渲染失败 |

---

## 八、检查清单

添加物体后确认：

- [ ] 材质使用了 `StandardMaterial`
- [ ] 组件中添加了 `LevelEntity`
- [ ] 组件中添加了 `Name`（带 `Solari_` 前缀）
- [ ] 生成函数在 `spawn_pbr_showcase()` 中注册
- [ ] 需要碰撞时在 `solari.ron` 中配置
- [ ] 需要旋转动画时添加 `Rotating` 组件
- [ ] GLB 模型需确认材质是否兼容 Solari
