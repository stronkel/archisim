#!/usr/bin/env python3
"""Small 3D models made with other programs' libraries (trimesh,
pycollada, pygltflib) to test the readers of archisim-mesh: what a
SketchUp or Blender export looks like, without real project files.

    pip install trimesh pycollada pygltflib numpy
    python3 scripts/make_mesh_fixtures.py
"""
import io
import os
import struct
import zlib

import numpy as np
import trimesh
import collada
from collada import source, geometry, material, scene, asset
import pygltflib

OUT = os.path.join(os.path.dirname(__file__), "..", "crates", "archisim-mesh", "tests", "fixtures")
os.makedirs(OUT, exist_ok=True)


def png(w, h, pixel):
    """A PNG without Pillow: pixel(x, y) -> (r, g, b), rows from the top."""
    raw = b"".join(b"\0" + bytes(c for x in range(w) for c in pixel(x, y)) for y in range(h))
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


# 1. A chair as OBJ + MTL with a wood texture (trimesh), y up, metres:
#    seat 0.45 × 0.45 at 0.45, four legs, a back to 0.90.
parts = []
seat = trimesh.creation.box((0.45, 0.04, 0.45)); seat.apply_translation((0, 0.45, 0)); parts.append(seat)
for x in (-0.2, 0.2):
    for z in (-0.2, 0.2):
        leg = trimesh.creation.box((0.04, 0.43, 0.04)); leg.apply_translation((x, 0.215, z)); parts.append(leg)
back = trimesh.creation.box((0.45, 0.45, 0.03)); back.apply_translation((0, 0.675, -0.21)); parts.append(back)
chair = trimesh.util.concatenate(parts)
uv = np.column_stack([(chair.vertices[:, 0] + 0.25) * 2, chair.vertices[:, 1]])
wood = png(16, 16, lambda x, y: (150 + (x % 4) * 8, 100 + (y % 3) * 5, 60))
with open(os.path.join(OUT, "madera.png"), "wb") as f:
    f.write(wood)
from PIL import Image  # noqa: E402  (trimesh needs it for textures)
img = Image.open(io.BytesIO(wood))
chair.visual = trimesh.visual.TextureVisuals(uv=uv, image=img)
obj = trimesh.exchange.obj.export_obj(chair, include_texture=True, return_texture=True, mtl_name="silla.mtl")
text, files = obj
with open(os.path.join(OUT, "silla.obj"), "w") as f:
    f.write(text)
for name, data in files.items():
    path = os.path.join(OUT, name)
    # Its texture under our own name.
    if name.endswith(".png"):
        continue
    with open(path, "wb") as f:
        f.write(data.replace(b"material_0.png", b"madera.png") if isinstance(data, bytes) else data)

# 2. A tree as COLLADA, like SketchUp: inches, z up, a component
#    (library_nodes + instance_node) placed twice in the file's node tree,
#    a brown trunk and a green canopy. 4 m tall: trunk 1.5 m, canopy
#    sphere of 1.25 m radius on top.
mesh = collada.Collada()
mesh.assetInfo.unitmeter = 0.0254
mesh.assetInfo.unitname = "inch"
mesh.assetInfo.upaxis = asset.UP_AXIS.Z_UP
IN = 1 / 0.0254
def add_geom(gid, tm, matsym):
    v = tm.vertices.astype(np.float32).flatten()
    n = tm.vertex_normals.astype(np.float32).flatten()
    vs = source.FloatSource(gid + "-pos", v, ("X", "Y", "Z"))
    ns = source.FloatSource(gid + "-nor", n, ("X", "Y", "Z"))
    g = geometry.Geometry(mesh, gid, gid, [vs, ns])
    il = source.InputList()
    il.addInput(0, "VERTEX", "#" + gid + "-pos")
    il.addInput(1, "NORMAL", "#" + gid + "-nor")
    idx = np.repeat(tm.faces.flatten(), 2)
    g.primitives.append(g.createTriangleSet(idx, il, matsym))
    mesh.geometries.append(g)
    return g
def add_mat(mid, rgb):
    e = material.Effect(mid + "-fx", [], "lambert", diffuse=rgb + (1.0,))
    m = material.Material(mid, mid, e)
    mesh.effects.append(e); mesh.materials.append(m)
    return m
bark = add_mat("Corteza", (0.45, 0.30, 0.18))
leaf = add_mat("Hojas", (0.25, 0.55, 0.20))
trunk = trimesh.creation.cylinder(radius=0.15 * IN, height=1.5 * IN, sections=12)
trunk.apply_translation((0, 0, 0.75 * IN))
crown = trimesh.creation.icosphere(subdivisions=2, radius=1.25 * IN)
crown.apply_translation((0, 0, 2.75 * IN))
gt = add_geom("tronco", trunk, "m_tronco")
gc = add_geom("copa", crown, "m_copa")
comp = scene.Node("arbol", [
    scene.GeometryNode(gt, [scene.MaterialNode("m_tronco", bark, inputs=[])]),
    scene.GeometryNode(gc, [scene.MaterialNode("m_copa", leaf, inputs=[])]),
])
mesh.nodes.append(comp)
inst = scene.Node("ejemplar", [scene.NodeNode(comp)])
root = scene.Node("SketchUp", [inst])
vs_ = scene.Scene("escena", [root])
mesh.scenes.append(vs_)
mesh.scene = vs_
mesh.write(os.path.join(OUT, "arbol.dae"))

# 3. A car as GLB (pygltflib), y up, with two materials (body red, glass
#    transparent), the body moved by a node translation: 4.2 × 1.8 × 1.5 m.
body = trimesh.creation.box((4.2, 1.0, 1.8)); body.apply_translation((0, 0.5, 0))
cabin = trimesh.creation.box((2.2, 0.5, 1.6)); cabin.apply_translation((-0.3, 1.25, 0))
def prim_arrays(tm):
    return tm.vertices.astype(np.float32), tm.faces.astype(np.uint32).flatten()
blob = b""
accessors, views, meshes, nodes = [], [], [], []
for k, (tm, mat) in enumerate([(body, 0), (cabin, 1)]):
    v, f = prim_arrays(tm)
    vb, fb = v.tobytes(), f.tobytes()
    for data, target in ((vb, pygltflib.ARRAY_BUFFER), (fb, pygltflib.ELEMENT_ARRAY_BUFFER)):
        views.append(pygltflib.BufferView(buffer=0, byteOffset=len(blob), byteLength=len(data), target=target))
        blob += data
        while len(blob) % 4:
            blob += b"\0"
    accessors.append(pygltflib.Accessor(bufferView=len(views) - 2, componentType=pygltflib.FLOAT, count=len(v),
                                        type=pygltflib.VEC3, min=v.min(axis=0).tolist(), max=v.max(axis=0).tolist()))
    accessors.append(pygltflib.Accessor(bufferView=len(views) - 1, componentType=pygltflib.UNSIGNED_INT,
                                        count=len(f), type=pygltflib.SCALAR))
    meshes.append(pygltflib.Mesh(primitives=[pygltflib.Primitive(attributes=pygltflib.Attributes(POSITION=len(accessors) - 2),
                                                                 indices=len(accessors) - 1, material=mat)]))
    nodes.append(pygltflib.Node(mesh=k, translation=[0.0, 0.0, 0.0] if k == 0 else [0.0, 0.0, 0.0]))
g = pygltflib.GLTF2(
    scene=0,
    scenes=[pygltflib.Scene(nodes=[2])],
    nodes=nodes + [pygltflib.Node(children=[0, 1], translation=[10.0, 0.0, 0.0])],
    meshes=meshes,
    materials=[
        pygltflib.Material(name="Carrocería", pbrMetallicRoughness=pygltflib.PbrMetallicRoughness(baseColorFactor=[0.6, 0.02, 0.02, 1.0])),
        pygltflib.Material(name="Vidrio", alphaMode="BLEND", pbrMetallicRoughness=pygltflib.PbrMetallicRoughness(baseColorFactor=[0.6, 0.8, 0.9, 0.3])),
    ],
    accessors=accessors,
    bufferViews=views,
    buffers=[pygltflib.Buffer(byteLength=len(blob))],
)
g.set_binary_blob(blob)
g.save_binary(os.path.join(OUT, "coche.glb"))

# 4. The same car as .gltf with its buffer in a .bin beside it.
g2 = pygltflib.GLTF2().from_json(g.to_json())
g2.buffers = [pygltflib.Buffer(byteLength=len(blob), uri="coche.bin")]
with open(os.path.join(OUT, "coche.bin"), "wb") as f:
    f.write(blob)
g2.save_json(os.path.join(OUT, "coche.gltf"))
print("fixtures in", os.path.normpath(OUT))
