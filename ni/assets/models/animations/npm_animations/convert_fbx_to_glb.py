"""将目录中的 FBX 文件批量转换为 GLB，保留骨骼和动画。"""

import bpy
import os
import sys

# 获取输入目录（Blender `--` 后的第一个参数）
argv = sys.argv
if '--' in argv:
    args = argv[argv.index('--') + 1:]
    input_dir = args[0] if args else os.path.dirname(bpy.data.filepath)
else:
    input_dir = os.path.dirname(bpy.data.filepath)

print(f"输入目录: {input_dir}")

# 收集所有 FBX（排除基础模型自身，后面特殊处理）
fbx_files = sorted([
    f for f in os.listdir(input_dir)
    if f.lower().endswith('.fbx') and f != '5.fbx'
])
print(f"找到 {len(fbx_files)} 个动画 FBX 文件")

# 检查是否存在基础模型
base_model = os.path.join(input_dir, '5.fbx')
has_base = os.path.exists(base_model)

for fname in fbx_files:
    fbx_path = os.path.join(input_dir, fname)
    glb_name = os.path.splitext(fname)[0] + '.glb'
    glb_path = os.path.join(input_dir, glb_name)

    if os.path.exists(glb_path):
        print(f"  跳过 {glb_name}（已存在）")
        continue

    print(f"  转换 {fname} → {glb_name}")

    bpy.ops.wm.read_factory_settings(use_empty=True)

    try:
        if has_base:
            bpy.ops.import_scene.fbx(filepath=base_model)
            bpy.ops.import_scene.fbx(filepath=fbx_path)
        else:
            bpy.ops.import_scene.fbx(filepath=fbx_path)

        # 调试：打印所有对象
        print(f"    导入后对象列表:")
        for obj in bpy.data.objects:
            mat_count = 0
            face_count = 0
            if obj.type == 'MESH':
                mat_count = len(obj.data.materials)
                face_count = len(getattr(obj.data, 'faces', getattr(obj.data, 'polygons', ())))
            print(f"      {obj.name} type={obj.type} face={face_count} mat={mat_count}")

        # 删除无材质的辅助网格
        to_remove = [obj for obj in bpy.data.objects
                     if obj.type == 'MESH' and len(obj.data.materials) == 0]
        for obj in to_remove:
            mesh_data = obj.data
            print(f"    删除辅助网格: {obj.name}")
            bpy.data.objects.remove(obj, do_unlink=True)
            if mesh_data.users == 0:
                bpy.data.meshes.remove(mesh_data)

        # 导出 GLB
        bpy.ops.export_scene.gltf(
            filepath=glb_path,
            export_format='GLB',
            export_animations=True,
        )
        print(f"    完成: {glb_name}")

    except Exception as e:
        print(f"    失败: {e}")

print("全部转换完成！")
