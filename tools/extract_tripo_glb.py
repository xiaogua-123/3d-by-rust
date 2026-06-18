#!/usr/bin/env python3
"""从浏览器 Network 面板导出的 URL 列表中提取 Tripo3D GLB 模型下载链接。

用法:
    python extract_tripo_glb.py urls.txt          # 从文件读取
    python extract_tripo_glb.py                    # 使用内置示例
    cat urls.txt | python extract_tripo_glb.py -   # 从 stdin 读取
"""

import re
import sys

# 匹配 Tripo3D CDN 的 .glb 模型文件
TRIPO_GLB_PATTERN = re.compile(
    r'https://studio\.cdn\.tripo3d\.com/tripo-studio/'
    r'\d+/[a-f0-9\-]+/'
    r'tripo_model_[a-f0-9\-]+_meshopt\.glb'
    r'\?auth_key=[^&\s]+'
)


def extract_tripo_glb_urls(url_list):
    """从 URL 列表中筛选 Tripo3D 的 .glb 模型链接。

    Args:
        url_list: 包含所有请求 URL 的列表。

    Returns:
        匹配到的模型 .glb 链接列表。
    """
    return [url for url in url_list if TRIPO_GLB_PATTERN.search(url)]


def read_urls(source):
    """从文件路径或 stdin 读取 URL 列表。"""
    if source == "-":
        return [line.strip() for line in sys.stdin if line.strip()]
    with open(source, "r", encoding="utf-8") as f:
        return [line.strip() for line in f if line.strip()]


def download_glb(url, output_path=None):
    """下载 GLB 文件到本地。

    Args:
        url: Tripo3D GLB 下载链接。
        output_path: 保存路径，默认从 URL 中提取文件名。
    """
    import urllib.request
    import os

    if output_path is None:
        filename = url.split("/")[5].split("?")[0]  # tripo_model_xxx_meshopt.glb
        output_path = filename

    print(f"正在下载: {output_path}")
    urllib.request.urlretrieve(url, output_path)
    size_mb = os.path.getsize(output_path) / (1024 * 1024)
    print(f"下载完成: {output_path} ({size_mb:.1f} MB)")
    return output_path


def main():
    # 解析参数
    args = sys.argv[1:]
    do_download = "--download" in args
    args = [a for a in args if a != "--download"]

    # 读取 URL
    if args:
        urls = read_urls(args[0])
    else:
        # 内置示例 — 替换为你的实际 URL
        urls = [
            "https://studio.cdn.tripo3d.com/tripo-studio/20260529/"
            "765aedc0-06f7-4ed2-8529-24ca4fbdcd67/"
            "tripo_model_765aedc0-06f7-4ed2-8529-24ca4fbdcd67_meshopt.glb"
            "?auth_key=1780099200-NmJLY5bo-0-936621eea26863b2e5cac88cd57177b3",
            "https://studio.tripo3d.com/_nuxt/entry.CWZtl_VP.css",
            "https://studio.tripo3d.com/favicon.ico",
        ]

    glb_links = extract_tripo_glb_urls(urls)

    if not glb_links:
        print("未找到匹配的 Tripo3D GLB 模型链接。")
        print("请检查: URL 列表是否完整 / 模型是否加载过 / CDN 域名是否匹配。")
        sys.exit(1)

    print(f"找到 {len(glb_links)} 个 GLB 模型链接:\n")
    for idx, link in enumerate(glb_links, 1):
        print(f"  [{idx}] {link}")

    print("\n⚠️  auth_key 有时效性，请尽快下载。")

    if do_download:
        print("\n开始下载...\n")
        for link in glb_links:
            try:
                download_glb(link)
            except Exception as e:
                print(f"下载失败: {e}")


if __name__ == "__main__":
    main()
