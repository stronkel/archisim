"""Checks a .glb written by ArchiSim: glTF 2.0 that trimesh and pygltflib
read, with its meshes, materials and textures.

Usage: python3 scripts/validate_glb.py modelo.glb [...]
Requires: pip install pygltflib trimesh
"""
import sys

import pygltflib
import trimesh


def check(path):
    g = pygltflib.GLTF2().load(path)
    assert g.asset.version == "2.0", "no es glTF 2.0"
    assert g.meshes, "no hay mallas"
    blob = g.binary_blob()
    assert blob is not None and len(blob) >= g.buffers[0].byteLength, "falta el binario"
    for v in g.bufferViews:
        assert v.byteOffset + v.byteLength <= len(blob), "una vista se sale del binario"
    for a in g.accessors:
        assert a.bufferView is not None and a.bufferView < len(g.bufferViews)
    for m in g.meshes:
        for p in m.primitives:
            assert p.attributes.POSITION is not None
            assert p.material is not None and p.material < len(g.materials)
            if p.attributes.TEXCOORD_0 is not None:
                mat = g.materials[p.material]
                assert mat.pbrMetallicRoughness.baseColorTexture is not None
    textured = sum(
        1 for m in g.materials if m.pbrMetallicRoughness and m.pbrMetallicRoughness.baseColorTexture
    )
    scene = trimesh.load(path, force="scene")
    faces = sum(len(geo.faces) for geo in scene.geometry.values())
    assert faces > 0, "sin triángulos"
    print(
        f"{path}: {len(g.nodes)} objetos, {len(g.materials)} materiales "
        f"({textured} con textura), {faces} triángulos"
    )


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    for p in sys.argv[1:]:
        check(p)
    print("OK: los glTF son válidos.")


if __name__ == "__main__":
    main()
