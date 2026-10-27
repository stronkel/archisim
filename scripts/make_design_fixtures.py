#!/usr/bin/env python3
"""Models as a furniture manufacturer hands them out, made with other
programs, to test the FBX, 3DS and STL readers of archisim-mesh and the
reading of a model inside a .zip:

- butaca.fbx (binary FBX 7.4, written by Blender): a lounge chair placed
  away from the origin, turned, with a hierarchy of parts, three
  materials (leather, walnut with its texture inside the file, glass)
  and Blender's own way of writing units and axes (centimetres, y up,
  each part turned −90° and scaled 100).
- butaca-z.fbx (the same, z up as 3ds Max writes them).
- butaca-ascii.fbx (FBX in text, written by Assimp from the binary one).
- butaca.3ds (3D Studio, written by Assimp; 3DS has no units: the same
  chair in millimetres).
- butaca.stl (binary STL, written by Blender, in millimetres, z up).
- butaca.zip: what a download looks like: a folder with the binary FBX,
  its texture as a file next to it, and the 3DS in a folder of its own.
- butaca.json: what Blender says of the chair (its box in metres, z up,
  front towards −y, and its faces per material), which the tests check
  against.

    python3 -m venv /tmp/bl && /tmp/bl/bin/pip install bpy   # Blender as a module
    apt install assimp-utils                                  # the assimp command
    /tmp/bl/bin/python scripts/make_design_fixtures.py
"""
import json
import math
import os
import shutil
import subprocess
import tempfile
import zipfile

import bpy

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "archisim-mesh", "tests", "fixtures")
os.makedirs(OUT, exist_ok=True)
TMP = tempfile.mkdtemp()

bpy.ops.wm.read_factory_settings(use_empty=True)


def material(name, rgb, alpha=1.0, image=None):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
    bsdf.inputs["Alpha"].default_value = alpha
    if image is not None:
        tex = m.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = image
        m.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    return m


# A walnut texture: dark brown with lighter stripes, 32 × 32.
img = bpy.data.images.new("nogal", 32, 32)
px = []
for y in range(32):
    for x in range(32):
        light = 1.0 if (x // 4) % 2 == 0 else 0.0
        px += [0.30 + 0.12 * light, 0.17 + 0.07 * light, 0.08, 1.0]
img.pixels = px
img.filepath_raw = os.path.join(TMP, "nogal.png")
img.file_format = "PNG"
img.save()
img.pack()

leather = material("Cuero coñac", (0.45, 0.20, 0.08))
walnut = material("Nogal", (0.8, 0.8, 0.8), image=img)
glass = material("Vidrio", (0.85, 0.92, 0.95), alpha=0.35)

root = bpy.data.objects.new("Butaca", None)
bpy.context.collection.objects.link(root)
root.location = (1.0, 2.0, 0.0)
root.rotation_euler = (0.0, 0.0, math.radians(30))


def box(name, size, at, mat, parent, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=1)
    o = bpy.context.active_object
    o.name = name
    o.scale = size
    o.location = at
    o.rotation_euler = rot
    o.data.materials.append(mat)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    o.parent = parent
    return o


seat = box("Asiento", (0.80, 0.80, 0.16), (0, 0, 0.40), leather, root)
box("Respaldo", (0.80, 0.12, 0.60), (0, 0.36, 0.38), leather, seat, rot=(math.radians(-15), 0, 0))
for i, (x, y) in enumerate([(-0.35, -0.35), (0.35, -0.35), (-0.35, 0.35), (0.35, 0.35)]):
    bpy.ops.mesh.primitive_cylinder_add(vertices=12, radius=0.025, depth=0.32, location=(x, y, 0.16))
    leg = bpy.context.active_object
    leg.name = f"Pata {i + 1}"
    leg.data.materials.append(walnut)
    leg.parent = root
    # Already in the root's frame: keep where it is in the world.
    leg.matrix_parent_inverse = root.matrix_world.inverted()
bpy.ops.mesh.primitive_plane_add(size=0.3, location=(0.55, -0.1, 0.5))
tray = bpy.context.active_object
tray.name = "Bandeja"
tray.data.materials.append(glass)
tray.parent = root
tray.matrix_parent_inverse = root.matrix_world.inverted()

bpy.context.view_layer.update()

# What Blender says: the box of every mesh in the world, and the faces of
# each material (triangles).
lo = [math.inf] * 3
hi = [-math.inf] * 3
faces = {}
for o in bpy.data.objects:
    if o.type != "MESH":
        continue
    for v in o.data.vertices:
        w = o.matrix_world @ v.co
        for k in range(3):
            lo[k] = min(lo[k], w[k])
            hi[k] = max(hi[k], w[k])
    for p in o.data.polygons:
        name = o.data.materials[p.material_index].name
        faces[name] = faces.get(name, 0) + len(p.vertices) - 2

fbx = os.path.join(OUT, "butaca.fbx")
bpy.ops.export_scene.fbx(filepath=fbx, path_mode="COPY", embed_textures=True)
# The same as 3ds Max writes them: z up, front −y, millimetres.
bpy.ops.export_scene.fbx(filepath=os.path.join(OUT, "butaca-z.fbx"), path_mode="COPY", embed_textures=True,
                         axis_up="Z", axis_forward="-Y", global_scale=1.0, apply_unit_scale=True,
                         bake_space_transform=False)
# The same, its texture as a file next to it (for the zip).
loose = os.path.join(TMP, "Butaca Ejemplo", "FBX", "butaca.fbx")
os.makedirs(os.path.dirname(loose))
bpy.ops.export_scene.fbx(filepath=loose, path_mode="COPY", embed_textures=False)

subprocess.run(["assimp", "export", fbx, os.path.join(OUT, "butaca-ascii.fbx"), "-ffbxa"], check=True, capture_output=True)
# 3DS and STL hand their parts out already in place (no hierarchy), in
# millimetres, z up: the chair flattened and scaled by 1000, written by
# Blender as OBJ and STL, and the OBJ by Assimp as 3DS.
for o in list(bpy.data.objects):
    if o.type == "MESH":
        mw = o.matrix_world.copy()
        o.parent = None
        o.matrix_world = mw
bpy.data.objects.remove(root)
for o in bpy.data.objects:
    o.select_set(True)
    bpy.context.view_layer.objects.active = o
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
bpy.ops.transform.resize(value=(1000, 1000, 1000), center_override=(0, 0, 0))
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
obj = os.path.join(TMP, "mm", "butaca.obj")
os.makedirs(os.path.dirname(obj))
bpy.ops.wm.obj_export(filepath=obj, up_axis="Z", forward_axis="Y", path_mode="COPY", export_selected_objects=False)
bpy.ops.wm.stl_export(filepath=os.path.join(OUT, "butaca.stl"), up_axis="Z", forward_axis="Y", ascii_format=False)
subprocess.run(["assimp", "export", obj, os.path.join(OUT, "butaca.3ds"), "-f3ds"], check=True, capture_output=True)

os.makedirs(os.path.join(TMP, "Butaca Ejemplo", "3DS"))
shutil.copy(os.path.join(OUT, "butaca.3ds"), os.path.join(TMP, "Butaca Ejemplo", "3DS", "butaca.3ds"))
with zipfile.ZipFile(os.path.join(OUT, "butaca.zip"), "w", zipfile.ZIP_DEFLATED) as z:
    for folder, _, files in os.walk(os.path.join(TMP, "Butaca Ejemplo")):
        for f in sorted(files):
            p = os.path.join(folder, f)
            z.write(p, os.path.relpath(p, TMP))

with open(os.path.join(OUT, "butaca.json"), "w") as f:
    json.dump({"lo": lo, "hi": hi, "faces": faces}, f, indent=1, ensure_ascii=False, sort_keys=True)
    f.write("\n")
print("box", lo, hi, "faces", faces)
