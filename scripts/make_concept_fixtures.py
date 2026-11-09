#!/usr/bin/env python3
"""Volume models as SketchUp exports them, made up here (trimesh,
pycollada, manifold3d), to test bringing them in as concept volumes:

* `volumenes.dae` — COLLADA, Z up, in inches as SketchUp writes it: two
  boxes (one exported with both sides of every face, as SketchUp's
  «two-sided faces» option does, the other without its bottom face) and an
  L-shaped house with a gabled roof on each wing, walls and roofs of
  different materials.
* `volumenes.obj` + `volumenes.mtl` — OBJ, Y up, metres: a curved volume
  (a standing capsule) and an open piece (a box without its top and one
  of its sides).

    pip install trimesh pycollada manifold3d numpy
    python3 scripts/make_concept_fixtures.py

No real project goes in here.
"""
import os

import numpy as np
import trimesh
import collada
from collada import source, geometry, material, scene, asset

OUT = os.path.join(os.path.dirname(__file__), "..", "crates", "archisim-mesh", "tests", "fixtures")
os.makedirs(OUT, exist_ok=True)
IN = 1 / 0.0254


def gabled_wing(x0, y0, length, depth, eaves, ridge, along_x):
    """A wing with a gabled roof: a pentagon extruded along its length."""
    if along_x:
        prof = [(0, 0), (depth, 0), (depth, eaves), (depth / 2, ridge), (0, eaves)]
        verts = [(x0 + s, y0 + a, z) for s in (0, length) for (a, z) in prof]
    else:
        prof = [(0, 0), (depth, 0), (depth, eaves), (depth / 2, ridge), (0, eaves)]
        verts = [(x0 + a, y0 + s, z) for s in (0, length) for (a, z) in prof]
    n = len(prof)
    faces = []
    for i in range(n):
        j = (i + 1) % n
        faces += [(i, j, n + j), (i, n + j, n + i)]
    faces += [(0, k + 1, k) for k in range(1, n - 1)]
    faces += [(n, n + k, n + k + 1) for k in range(1, n - 1)]
    m = trimesh.Trimesh(np.array(verts, float), np.array(faces))
    m.fix_normals()
    return m


def box(x0, y0, w, d, h):
    b = trimesh.creation.box((w, d, h))
    b.apply_translation((x0 + w / 2, y0 + d / 2, h / 2))
    return b


# ---- COLLADA ---------------------------------------------------------------
dae = collada.Collada()
dae.assetInfo.unitmeter = 0.0254
dae.assetInfo.unitname = "inch"
dae.assetInfo.upaxis = asset.UP_AXIS.Z_UP


def add_mat(mid, rgb):
    e = material.Effect(mid + "-fx", [], "lambert", diffuse=rgb + (1.0,))
    m = material.Material(mid, mid, e)
    dae.effects.append(e)
    dae.materials.append(m)
    return m


walls = add_mat("Fachada_blanca", (0.92, 0.90, 0.85))
roof = add_mat("Teja_roja", (0.70, 0.30, 0.20))
flat = add_mat("Cubierta_grava", (0.55, 0.55, 0.52))
back = add_mat("BackColor", (0.64, 0.69, 0.75))


def add_geom(gid, parts):
    """`parts`: (vertices, faces, material symbol) in metres; one triangle
    set per material."""
    v = np.vstack([p[0] for p in parts]).astype(np.float64) * IN
    vs = source.FloatSource(gid + "-pos", v.astype(np.float32).flatten(), ("X", "Y", "Z"))
    g = geometry.Geometry(dae, gid + "-geom", gid, [vs])
    il = source.InputList()
    il.addInput(0, "VERTEX", "#" + gid + "-pos")
    off = 0
    for verts, faces, sym in parts:
        g.primitives.append(g.createTriangleSet((np.array(faces) + off).flatten(), il, sym))
        off += len(verts)
    dae.geometries.append(g)
    return g


def split_top(m, top_sym, side_sym, sloped_is_roof=False):
    """The faces of a mesh by material: up (or sloped) faces one, the rest
    the other."""
    nz = m.face_normals[:, 2]
    is_top = nz > 0.2 if sloped_is_roof else nz > 0.9
    return [
        (m.vertices, m.faces[is_top], top_sym),
        (m.vertices, m.faces[~is_top], side_sym),
    ]


nodes = []
# A: a 12 × 10 × 9 m block with both sides of each face exported.
a = box(0, 0, 12, 10, 9)
parts = split_top(a, "m_grava", "m_fachada")
parts.append((a.vertices, a.faces[:, ::-1], "m_dorso"))
ga = add_geom("bloque_a", parts)
nodes.append(scene.Node("bloque_a", [scene.GeometryNode(ga, [
    scene.MaterialNode("m_grava", flat, inputs=[]),
    scene.MaterialNode("m_fachada", walls, inputs=[]),
    scene.MaterialNode("m_dorso", back, inputs=[]),
])]))
# B: an 8 × 8 × 15 m tower without its bottom face, moved by its node.
b = box(0, 0, 8, 8, 15)
keep = b.face_normals[:, 2] > -0.9
bm = trimesh.Trimesh(b.vertices, b.faces[keep], process=False)
gb = add_geom("torre_b", [(bm.vertices, bm.faces, "m_fachada")])
nodes.append(scene.Node("torre_b", [scene.GeometryNode(gb, [
    scene.MaterialNode("m_fachada", walls, inputs=[]),
])], transforms=[scene.TranslateTransform(20 * IN, 0, 0)]))
# C: an L-shaped house, two gabled wings (eaves 6 m, ridge 9 m).
w1 = gabled_wing(0, 20, 16, 8, 6, 9, True)
w2 = gabled_wing(8, 20, 14, 8, 6, 9, False)
house = trimesh.boolean.union([w1, w2])
assert house.is_watertight
gc = add_geom("casa_l", split_top(house, "m_teja", "m_fachada", sloped_is_roof=True))
nodes.append(scene.Node("casa_l", [scene.GeometryNode(gc, [
    scene.MaterialNode("m_teja", roof, inputs=[]),
    scene.MaterialNode("m_fachada", walls, inputs=[]),
])]))
root = scene.Node("SketchUp", nodes)
vs_ = scene.Scene("escena", [root])
dae.scenes.append(vs_)
dae.scene = vs_
dae.write(os.path.join(OUT, "volumenes.dae"))

# ---- OBJ ---------------------------------------------------------------------
capsule = trimesh.creation.capsule(height=8.0, radius=3.0, count=[16, 16])
capsule.apply_translation((0, 0, 7.0))  # standing on z = 0, 14 m high
openbox = box(15, -2, 6, 4, 3)
nz, ny = openbox.face_normals[:, 2], openbox.face_normals[:, 1]
keep = ~((nz > 0.9) | (ny > 0.9))  # no top, no side looking to +y
lines = ["# Volúmenes inventados para las pruebas de ArchiSim", "mtllib volumenes.mtl"]
count = 0


def put(name, mat, verts, faces):
    global count
    lines.append(f"o {name}")
    for x, y, z in verts:
        # Ours z up → the file's y up.
        lines.append(f"v {x:.6f} {z:.6f} {-y:.6f}")
    lines.append(f"usemtl {mat}")
    for f in faces:
        lines.append("f " + " ".join(str(count + 1 + int(i)) for i in f))
    count += len(verts)


put("cupula", "Hormigon", capsule.vertices, capsule.faces)
put("abierta", "Vidrio_verde", openbox.vertices, openbox.faces[keep])
with open(os.path.join(OUT, "volumenes.obj"), "w") as f:
    f.write("\n".join(lines) + "\n")
with open(os.path.join(OUT, "volumenes.mtl"), "w") as f:
    f.write("newmtl Hormigon\nKd 0.75 0.74 0.70\n\nnewmtl Vidrio_verde\nKd 0.40 0.62 0.50\n")
print("volumenes.dae, volumenes.obj, volumenes.mtl")
