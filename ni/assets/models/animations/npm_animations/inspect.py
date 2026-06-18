"""检查 GLB 和 FBX 的内容结构"""
import bpy
import sys
import os

argv = sys.argv
if '--' in argv:
    args = argv[argv.index('--') + 1:]
    target_path = args[0]
else:
    target_path = os.path.dirname(bpy.data.filepath)

name = os.path.splitext(os.path.basename(target_path))[0]
ext = os.path.splitext(target_path)[1].lower()

print(f"\n=== 检查: {target_path} ===")

if ext == '.glb':
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=target_path)
elif ext == '.fbx':
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=target_path)

print(f"  对象数量: {len(bpy.data.objects)}")
print(f"  网格数量: {len(bpy.data.meshes)}")
print(f"  材质数量: {len(bpy.data.materials)}")
print(f"  骨骼数量: {len(bpy.data.armatures)}")
print(f"  动作数量: {len(bpy.data.actions)}")

for obj in bpy.data.objects:
    print(f"  对象: {obj.name} 类型={obj.type} 隐藏={obj.hide_get()}")

for mesh in bpy.data.meshes:
    print(f"  网格: {mesh.name} 顶点={len(mesh.vertices)} 面={len(mesh.polygons)} 材质={len(mesh.materials)}")

for armature in bpy.data.armatures:
    print(f"  骨骼: {armature.name} 骨头={len(armature.bones)}")

for action in bpy.data.actions:
    print(f"  动作: {action.name} 帧={int(action.frame_range[1] - action.frame_range[0])}")
