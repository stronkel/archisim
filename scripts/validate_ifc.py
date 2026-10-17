#!/usr/bin/env python3
"""Validate an IFC file exported by ArchiSim with IfcOpenShell.

Checks:
  1. Schema validation (entity attributes, types, WHERE rules where supported).
  2. Every wall produces 3D geometry, with a volume matching its quantities.
  3. Every wall has a type, a material layer set usage, Pset_WallCommon and
     Qto_WallBaseQuantities, and its layers add up to the type thickness.

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

    walls = f.by_type("IfcWall")
    if not walls:
        errors.append("no IfcWall found")
    for w in walls:
        name = w.Name
        wtype = eu.get_type(w)
        if wtype is None:
            errors.append(f"{name}: no type")
        mat = eu.get_material(w, should_inherit=False)
        if mat is None or not mat.is_a("IfcMaterialLayerSetUsage"):
            errors.append(f"{name}: no material layer set usage")
        psets = eu.get_psets(w)
        for required in ("Pset_WallCommon", "Qto_WallBaseQuantities"):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        if mat is not None and mat.is_a("IfcMaterialLayerSetUsage"):
            total = sum(l.LayerThickness for l in mat.ForLayerSet.MaterialLayers)
            width = psets.get("Qto_WallBaseQuantities", {}).get("Width")
            if width is not None and abs(total - width) > 1e-6:
                errors.append(f"{name}: layers {total} != width {width}")
        if geom_ok:
            try:
                shp = geom.create_shape(settings, w)
                vol = shape.get_volume(shp.geometry)
                expected = psets["Qto_WallBaseQuantities"]["GrossVolume"]
                if abs(vol - expected) > 1e-4:
                    errors.append(f"{name}: volume {vol:.4f} != {expected:.4f}")
            except Exception as exc:
                errors.append(f"{name}: geometry failed: {exc}")

    storeys = f.by_type("IfcBuildingStorey")
    print(f"{path}: {len(walls)} muros, {len(storeys)} plantas, "
          f"{len(f.by_type('IfcWallType'))} tipos, {len(f.by_type('IfcMaterial'))} materiales")
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
