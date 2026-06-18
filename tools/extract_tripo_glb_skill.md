# Tripo3D GLB 链接提取器

## 概述

从浏览器 Network 面板导出的 URL 列表中，自动筛选 Tripo3D 的 `.glb` 3D 模型下载链接。

## 使用场景

- Tripo3D 工作区页面返回 400 错误，但模型曾经加载过
- 需要批量提取多个模型的下载链接
- 想自动化下载 Tripo3D 生成的 3D 模型

## 快速使用

### 1. 获取 URL 列表

1. 打开 Chrome DevTools → Network 标签
2. 在 Tripo3D 中加载模型
3. 右键请求列表 → Copy → Copy all URLs
4. 粘贴到 `urls.txt` 文件，每行一个 URL

### 2. 运行脚本

```bash
# 从文件提取
python tools/extract_tripo_glb.py urls.txt

# 提取并自动下载
python tools/extract_tripo_glb.py urls.txt --download

# 从 stdin 读取
cat urls.txt | python tools/extract_tripo_glb.py -
```

### 3. 输出示例

```
找到 1 个 GLB 模型链接:

  [1] https://studio.cdn.tripo3d.com/tripo-studio/20260529/765aedc0.../tripo_model_..._meshopt.glb?auth_key=...

⚠️  auth_key 有时效性，请尽快下载。
```

## 参数说明

| 参数 | 说明 |
|------|------|
| `<file>` | URL 列表文件路径 |
| `-` | 从 stdin 读取 |
| `--download` | 提取后自动下载 GLB 文件 |
| 无参数 | 使用脚本内置示例 |

## 匹配规则

正则模式:
```
https://studio\.cdn\.tripo3d\.com/tripo-studio/\d+/[a-f0-9\-]+/tripo_model_[a-f0-9\-]+_meshopt\.glb\?auth_key=[^&\s]+
```

精确匹配 Tripo3D CDN 的模型文件，不会误匹配 CSS/JS/图片等资源。

## 注意事项

| 问题 | 说明 |
|------|------|
| auth_key 过期 | 链接通常几小时内有效，过期需重新刷新页面 |
| 模型版本 | 下载的是服务器最新保存版本 |
| 无匹配结果 | 检查模型是否加载完成、URL 列表是否完整 |

## 作为 Skill 集成

在其他项目中使用此工具:

1. 复制 `tools/extract_tripo_glb.py` 到目标项目
2. 将本文档内容添加到项目的 Skill 系统中
3. 通过 `python extract_tripo_glb.py <urls.txt>` 调用

## 扩展

脚本支持的扩展点:
- `extract_tripo_glb_urls(url_list)` — 核心筛选函数，可被其他脚本 import
- `download_glb(url, output_path)` — 下载函数，支持自定义保存路径
- `TRIPO_GLB_PATTERN` — 编译后的正则对象，可直接复用
