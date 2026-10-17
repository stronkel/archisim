#!/usr/bin/env python3
"""Validate an IFC file exported by ArchiSim with IfcOpenShell.

Checks:
  1. Schema validation (entity attributes, types, WHERE rules where supported).
  2. Every wall and slab produces 3D geometry, with a volume matching its
     quantities.
  3. Every wall and slab has a type, a material layer set usage, its common
     property set and base quantities, and its layers add up to the type
     thickness.
  4. Every slab's layer set usage runs along Z (AXIS3) and its layers are
     ordered bottom to top, matching the extrusion.

Usage:  python3 scripts/validate_ifc.py out/demo.ifc
Needs:  pip install ifcopenshell
"""
import importlib
import sys

import ifcopenshell
import ifcopenshell.util.element as eu
import ifcopenshell.validate


def main(path: str) -> int:
    f = ifcopenshell.open(path)
    errors = []

    logger = ifcopenshell.validate.json_logger()
    try:
        ifcopenshell.validate.validate(f, logger, express_rules=True)
    except ImportError:  # EXPRESS rules need pytest installed
        print("(reglas EXPRESS omitidas: pip install pytest para activarlas)")
        logger = ifcopenshell.validate.json_logger()
        ifcopenshell.validate.validate(f, logger)
    for s in logger.statements:
        errors.append(f"schema: {s.get('message')}")

    try:
        geom = importlib.import_module("ifcopenshell.geom")
        shape = importlib.import_module("ifcopenshell.util.shape")
        settings = geom.settings()
        geom_ok = True
    except Exception as exc:  # geometry engine missing in some builds
        print(f"(geometry check skipped: {exc})")
        geom_ok = False

    def check_element(el, pset_name, qto_name):
        name = el.Name
        if eu.get_type(el) is None:
            errors.append(f"{name}: no type")
        mat = eu.get_material(el, should_inherit=False)
        if mat is None or not mat.is_a("IfcMaterialLayerSetUsage"):
            errors.append(f"{name}: no material layer set usage")
            mat = None
        psets = eu.get_psets(el)
        for required in (pset_name, qto_name):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        if mat is not None:
            total = sum(l.LayerThickness for l in mat.ForLayerSet.MaterialLayers)
            width = psets.get(qto_name, {}).get("Width")
            if width is not None and abs(total - width) > 1e-6:
                errors.append(f"{name}: layers {total} != width {width}")
        if geom_ok:
            try:
                shp = geom.create_shape(settings, el)
                vol = shape.get_volume(shp.geometry)
                expected = psets[qto_name]["GrossVolume"]
                if abs(vol - expected) > 1e-4:
                    errors.append(f"{name}: volume {vol:.4f} != {expected:.4f}")
            except Exception as exc:
                errors.append(f"{name}: geometry failed: {exc}")
        return mat

    walls = f.by_type("IfcWall")
    if not walls:
        errors.append("no IfcWall found")
    for w in walls:
        check_element(w, "Pset_WallCommon", "Qto_WallBaseQuantities")

    slabs = f.by_type("IfcSlab")
    for s in slabs:
        mat = check_element(s, "Pset_SlabCommon", "Qto_SlabBaseQuantities")
        if mat is None:
            continue
        if mat.LayerSetDirection != "AXIS3":
            errors.append(f"{s.Name}: layer direction {mat.LayerSetDirection}, expected AXIS3")
        # The finish is on top: the last layer (top) must not be the structure
        # unless the type has a single layer.
        layers = mat.ForLayerSet.MaterialLayers
        if len(layers) > 1 and layers[-1].Category == "LoadBearing":
            errors.append(f"{s.Name}: layers seem to be ordered top to bottom")
        type_obj = eu.get_type(s)
        if type_obj is not None and type_obj.PredefinedType != s.PredefinedType:
            errors.append(f"{s.Name}: predefined type differs from its type")

    storeys = f.by_type("IfcBuildingStorey")
    types = len(f.by_type("IfcWallType")) + len(f.by_type("IfcSlabType"))
    print(f"{path}: {len(walls)} muros, {len(slabs)} forjados, {len(storeys)} plantas, "
          f"{types} tipos, {len(f.by_type('IfcMaterial'))} materiales")
    if errors:
        print(f"{len(errors)} error(es):")
        for e in errors:
            print("  -", e)
        return 1
    print("OK: el archivo es válido.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
