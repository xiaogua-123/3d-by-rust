# 从0开始-3d

基于 **Rust + Bevy 0.18** 的 3D 冒险游戏项目。

## 项目结构

```
.
├── ni/              # 核心游戏项目（Rust + Bevy）
│   ├── src/         # Rust 游戏源码
│   ├── assets/      # 游戏资源（模型、纹理、音效、字体）
│   └── docs/        # 游戏设计文档
├── 3d/              # 3D 工具/资源
├── tools/           # 工具脚本
├── package.json     # JS 依赖（three.js、gltf-transform）
└── .github/         # CI/CD 配置（GitHub Actions）
```

## 快速开始

```bash
# 运行游戏
cd ni
cargo run              # debug 模式
cargo run --release    # release 模式
```

详细文档请参阅 [ni/Readme.md](ni/Readme.md)。

## 技术栈

- **引擎**: Bevy 0.18 (ECS)
- **语言**: Rust (edition 2024)
- **UI**: egui
- **渲染**: 卡通渲染 (toon shader) + Solari 光照系统
- **粒子**: bevy_hanabi (GPU 粒子)
- **3D 模型**: GLB 格式
- **音频**: WAV 格式

## 许可证

MIT
