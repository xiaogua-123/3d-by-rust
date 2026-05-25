# 三渲二（Cel Shading / Toon Shading）渲染实现计划

## 项目现状

- **引擎**: Bevy 0.18 (Rust, WGSL shader)
- **当前渲染**: `StandardMaterial` (PBR) — 所有物体使用标准物理光照
- **光照环境**: 1个 `PointLight` + `GlobalAmbientLight` + 玩家的 `SpotLight`(手电筒)
- **物体材质**: `StandardMaterial` 设置 `base_color`, `emissive`
- **没有自定义 shader**, 没有后处理

---

## 技术方案概述

基于 Bevy 0.18 的 `ExtendedMaterial<StandardMaterial, MyExtension>` 机制，在 PBR 着色器输出阶段注入卡通化处理。描边则通过独立的材质和实体实现。

### 核心技术决策

| 项目 | 方案 |
|------|------|
| 阶梯漫反射 | Half Lambert + Ramp贴图采样 (WGSL `textureSample`) |
| 风格化高光 | Blinn-Phong + `step()`硬边切断 |
| 描边 | 独立 `OutlineMaterial` + 法线外扩顶点着色器 + `CullMode::Front` |
| Ramp贴图 | 程序化生成 3阶色阶 256x1 贴图 |
| 后处理 | 暂不实现（可作为后续优化） |

---

## 文件结构 (新增)

```
ni/
├── assets/
│   └── shaders/
│       ├── toon_shading.wgsl    # 卡通着色扩展shader
│       └── outline.wgsl         # 描边shader
├── src/
│   └── toon/
│       ├── mod.rs               # ToonPlugin + 系统注册
│       ├── material.rs          # ToonExtension + 类型定义
│       ├── outline.rs           # OutlineMaterial + outline组件/系统
│       └── ramp.rs              # Ramp贴图程序化生成
```

---

## Phase 1: 卡通化着色 (toon_shading)

### 1.1 ToonExtension 数据结构

```rust
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct ToonExtension {
    // Ramp贴图 (256x1, 从左到右暗→亮)
    #[texture(100, dimension = "2d")]
    #[sampler(101)]
    ramp_texture: Handle<Image>,

    // 高光阈值 [0.0 ~ 1.0], 低于此值无高光
    #[uniform(102)]
    spec_threshold: f32,
    // 高光平滑度 (0 = 硬边, >0 = 软边)
    #[uniform(102)]
    spec_smoothness: f32,
    // 高光颜色
    #[uniform(102)]
    spec_color: Vec3,

    // 环境光色阶数 (1-5)
    #[uniform(102)]
    shade_steps: u32,

    _padding: u32, // WGSL 16字节对齐
}
```

### 1.2 toon_shading.wgsl 片段着色器逻辑

```wgsl
// 1. 从 StandardMaterial 获取 PbrInput
var pbr_input = pbr_input_from_standard_material(in, is_front);

// 2. 计算 Half Lambert (NdotL ∈ [0, 1])
let NdotL = dot(pbr_input.N, pbr_input.light_dir) * 0.5 + 0.5;

// 3. 采样 Ramp 贴图得到阶梯漫反射
let diffuse = textureSample(ramp_texture, ramp_sampler, vec2(NdotL, 0.5)).rgb;

// 4. 计算 Blinn-Phong 高光 + step 硬边
let H = normalize(pbr_input.V + pbr_input.light_dir);
let spec_intensity = dot(pbr_input.N, H);
let spec = step(spec_threshold, spec_intensity); // 硬边
// 或 smoothstep(spec_threshold - spec_smoothness, spec_threshold, spec_intensity);

// 5. 组合输出
let lit_color = diffuse * pbr_input.material.base_color.rgb
              + spec * spec_color;
out.color = vec4(lit_color, 1.0);
```

### 1.3 Ramp贴图程序化生成 (ramp.rs)

启动时生成一张 `256x1` 的 `Image`：
- 左 1/3: 暗色 (阴影)
- 中 1/3: 中间色 (固有色区域)
- 右 1/3: 亮色 (亮面)
- 硬边：相邻色块之间仅 1-2px 过渡
- 作为资源注入，所有 ToonMaterial 默认使用

### 1.4 替换范围

- `world.rs`: `setup_world` 的 `StandardMaterial` → `ToonMaterial`
- `level.rs`: `spawn_floor`, `spawn_platform`, `spawn_wall` 等 → `ToonMaterial`
- `player.rs`: `SceneRoot("BrainStem.glb")` 的材质 → 通过替换 GLB 加载后的材质（暂保持 StandardMaterial，因为 GLB 材质需额外处理）
- 收集品和敌人: `StandardMaterial` → `ToonMaterial`

---

## Phase 2: 描边 (Outline)

### 2.1 OutlineMaterial

```rust
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct OutlineMaterial {
    #[uniform(0)]
    outline_color: LinearRgba,
    #[uniform(0)]
    outline_width: f32,    // 屏幕空间宽度系数
}
```

### 2.2 outline.wgsl

```wgsl
@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // 将法线变换到视图空间
    let normal_vs = (view.inverse_view * vec4(in.normal, 0.0)).xyz;
    // 沿法线方向外扩（在视图空间做，保证屏幕空间宽度一致）
    let offset = normalize(normal_vs) * outline_width * 0.02;
    let world_pos = vec4(in.position, 1.0) + vec4(offset, 0.0) * length(in.position);
    out.position = view.view_proj * world_pos;
    return out;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4(outline_color.rgb, 1.0);
}
```

### 2.3 Outline 生成系统

```rust
#[derive(Component)]
struct ToonOutline; // 标记需要描边的实体

// 系统: 对有 ToonOutline 的实体，生成子实体
// 子实体 = 相同Mesh + OutlineMaterial + Transform::IDENTITY
fn spawn_outline_meshes(...)
```

描边子实体设置：
- `RenderLayers` 与主体区分（可选，用于控制描边在哪些相机可见）
- CullMode: `Front` (只渲染背面 → 外轮廓效果)
- DepthBias 略微偏移防止 z-fighting

### 2.4 描边适用范围

- 玩家模型 (BrainStem.glb) — 加描边
- 敌人 (红色方块) — 加描边
- 平台/墙壁/地板 — 不加描边（避免场景线太杂）
- 收集品 — 可选

---

## Phase 3: 集成与参数调校

### 3.1 新增 `ToonPlugin`

```rust
pub struct ToonPlugin;

impl Plugin for ToonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, ToonExtension>,
        >::default())
        .add_plugins(MaterialPlugin::<OutlineMaterial>::default())
        .init_resource::<ToonSettings>()
        .add_systems(Startup, generate_ramp_texture)
        .add_systems(Update, spawn_outline_meshes);
    }
}
```

### 3.2 `ToonSettings` 全局资源

```rust
#[derive(Resource)]
struct ToonSettings {
    ramp_texture: Handle<Image>,     // 默认Ramp图
    default_spec_threshold: f32,     // 0.8
    default_spec_smoothness: f32,    // 0.01
    default_spec_color: Color,       // 白色
    default_outline_color: Color,    // 黑色
    default_outline_width: f32,      // 0.05
}
```

### 3.3 注册到 main.rs

```rust
mod toon;
use toon::ToonPlugin;

// 在 DefaultPlugins 之后添加
app.add_plugins(ToonPlugin);
```

---

## 实施顺序

| 步骤 | 内容 | 预计改动文件 |
|------|------|-------------|
| **Step 1** | 创建 `src/toon/` 目录结构 + `mod.rs` | 4 新文件 |
| **Step 2** | 实现 `ramp.rs` — 程序化生成Ramp贴图 | ramp.rs |
| **Step 3** | 实现 `material.rs` — `ToonExtension` | material.rs |
| **Step 4** | 编写 `assets/shaders/toon_shading.wgsl` | toon_shading.wgsl |
| **Step 5** | 集成到 `world.rs` + `level.rs` — 替换材质 | world.rs, level.rs |
| **Step 6** | `cargo check` 编译验证 + 运行调参 | — |
| **Step 7** | 实现 `outline.rs` — `OutlineMaterial` + 描边系统 | outline.rs |
| **Step 8** | 编写 `assets/shaders/outline.wgsl` | outline.wgsl |
| **Step 9** | 对玩家/敌人/收集品添加描边组件 | player.rs, level.rs |
| **Step 10** | 最终编译验证 + 视觉效果调参 | — |

---

## 技术和风险说明

### Bevy 0.18 特有约束
- `ExtendedMaterial` 的 uniform 绑定从 slot 100 开始
- 必须同时提供 `fragment_shader()` 和 `deferred_fragment_shader()`
- `OpaqueRendererMethod::Auto` 允许前向/延迟两种路径
- 描边方案用独立实体而非多Pass，因为 Bevy 的 RenderGraph 不直接暴露多Pass API

### GLB 模型注意
- GLB 自带材质定义，在 Bevy 中加载后使用 `SceneRoot` component
- 要让 GLB 也使用 ToonMaterial，需要在场景 spawn 后遍历 `MeshMaterial3d` 组件替换
- **暂定策略**: 第一版 GLB 模型保持原样（BrainStem, CesiumMan），仅替换直接创建的 Mesh

### 光照要求
- 三渲二需要至少一个方向光才能正确显示色阶
- 当前的 `PointLight` 衰减可能产生不符合预期的渐变，建议增加 `DirectionalLight`
- `AmbientLight` 需要降低，否则会冲淡阴影阶梯

### 后处理（留待后续）
- 屏幕空间描边 (Sobel/深度法线) — 更完整的线条覆盖
- LUT 颜色分级 — 胶片质感
- Bloom — 风格化发光

---

## 预期效果

完成后的渲染特征：
1. 地面/平台/墙壁呈现 **2-3阶色块明暗**，边界清晰
2. 光照面有明显 **硬边高光块**
3. 角色/敌人有 **黑色描边轮廓**
4. 通过调整 `ToonSettings` 可切换不同 Ramp 风格（硬边/柔和/冷暖色）
5. 整体画面具有 **二维手绘动画** 的视觉质感
