"""将 FBX 导出为静态 GLB — 保留蒙皮骨骼但不带动画"""
import bpy, os, sys

argv = sys.argv
input_dir = argv[argv.index('--') + 1] if '--' in argv else os.path.dirname(bpy.data.filepath)

base = os.path.join(input_dir, '5.fbx')
standalone = os.path.join(input_dir, 'Standing_Idle_03.fbx')

if os.path.exists(base):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=base)
elif os.path.exists(standalone):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=standalone)

# 清除动画（移除 action 数据，保留骨骼结构）
for action in bpy.data.actions:
    bpy.data.actions.remove(action)

# 删除无材质辅助网格
for obj in list(bpy.data.objects):
    if obj.type == 'MESH' and len(obj.data.materials) == 0:
        bpy.data.objects.remove(obj, do_unlink=True)

output_path = os.path.join(input_dir, 'static_model.glb')
bpy.ops.export_scene.gltf(
    filepath=output_path,
    export_format='GLB',
    export_animations=False,
    export_skins=True,
    export_texcoords=True,
    export_normals=True,
    export_materials='EXPORT',
    export_image_format='JPEG',
)
print(f"导出: {output_path}")
