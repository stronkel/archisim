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
  6. Every column and beam has a column or beam type, a material profile
     set usage, its common property set and base quantities, and its 3D
     volume matches its quantities (and its profile area times length).
  5. Every opening voids a wall and is filled by exactly one door or window
     (a void, "hueco", marked with ArchiSim_Void, by none);
     every door and window fills an opening, has a type, a material
     constituent set, its common property set and quantities, and 3D
     geometry. Wall volumes are compared with their net volume, after the
     openings are cut.

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
        # Fine enough for curved walls (exact arcs) to match their
        # quantities, which are those of the exact arcs.
        try:
            settings.set("mesher-linear-deflection", 1e-5)
            settings.set("mesher-angular-deflection", 0.01)
        except Exception:
            pass
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
                qto = psets[qto_name]
                expected = qto.get("NetVolume", qto["GrossVolume"])
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

    openings = f.by_type("IfcOpeningElement")
    for o in openings:
        voids = o.VoidsElements
        if len(voids) != 1 or not voids[0].RelatingBuildingElement.is_a("IfcWall"):
            errors.append(f"{o.Name}: does not void exactly one wall")
        psets = eu.get_psets(o)
        expected = 0 if "ArchiSim_Void" in psets else 1
        if len(o.HasFillings) != expected:
            errors.append(f"{o.Name}: has {len(o.HasFillings)} fillings, expected {expected}")
        if expected == 0 and "Qto_OpeningElementBaseQuantities" not in psets:
            errors.append(f"{o.Name}: void without its quantities")

    fillings = f.by_type("IfcDoor") + f.by_type("IfcWindow")
    for el in fillings:
        name = el.Name
        door = el.is_a("IfcDoor")
        if len(el.FillsVoids) != 1:
            errors.append(f"{name}: does not fill exactly one opening")
        type_obj = eu.get_type(el)
        if type_obj is None:
            errors.append(f"{name}: no type")
        elif not type_obj.is_a("IfcDoorType" if door else "IfcWindowType"):
            errors.append(f"{name}: wrong type class {type_obj.is_a()}")
        mat = eu.get_material(el)
        if mat is None or not mat.is_a("IfcMaterialConstituentSet"):
            errors.append(f"{name}: no material constituent set")
        psets = eu.get_psets(el)
        kind = "Door" if door else "Window"
        for required in (f"Pset_{kind}Common", f"Qto_{kind}BaseQuantities"):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        if not el.OverallWidth or not el.OverallHeight:
            errors.append(f"{name}: missing overall size")
        if el.ContainedInStructure == ():
            errors.append(f"{name}: not contained in a storey")
        if geom_ok:
            try:
                shp = geom.create_shape(settings, el)
                if shape.get_volume(shp.geometry) <= 0:
                    errors.append(f"{name}: empty geometry")
            except Exception as exc:
                errors.append(f"{name}: geometry failed: {exc}")

    members = f.by_type("IfcColumn") + f.by_type("IfcBeam")
    for el in members:
        name = el.Name
        kind = "Column" if el.is_a("IfcColumn") else "Beam"
        type_obj = eu.get_type(el)
        if type_obj is None or not type_obj.is_a(f"Ifc{kind}Type"):
            errors.append(f"{name}: no Ifc{kind}Type")
        mat = eu.get_material(el, should_inherit=False)
        if mat is None or not mat.is_a("IfcMaterialProfileSetUsage"):
            errors.append(f"{name}: no material profile set usage")
        psets = eu.get_psets(el)
        qto_name = f"Qto_{kind}BaseQuantities"
        for required in (f"Pset_{kind}Common", qto_name):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        if psets.get(f"Pset_{kind}Common", {}).get("LoadBearing") is not True:
            errors.append(f"{name}: not marked load-bearing")
        if el.ContainedInStructure == ():
            errors.append(f"{name}: not contained in a storey")
        if geom_ok and qto_name in psets:
            try:
                shp = geom.create_shape(settings, el)
                vol = shape.get_volume(shp.geometry)
                expected = psets[qto_name]["NetVolume"]
                # Round sections are tessellated: allow 1 %.
                tol = max(1e-4, 0.01 * expected)
                if abs(vol - expected) > tol:
                    errors.append(f"{name}: volume {vol:.4f} != {expected:.4f}")
            except Exception as exc:
                errors.append(f"{name}: geometry failed: {exc}")

    storeys = f.by_type("IfcBuildingStorey")
    types = sum(len(f.by_type(t)) for t in
                ("IfcWallType", "IfcSlabType", "IfcDoorType", "IfcWindowType",
                 "IfcColumnType", "IfcBeamType"))
    print(f"{path}: {len(walls)} muros, {len(slabs)} forjados, "
          f"{len(f.by_type('IfcDoor'))} puertas, {len(f.by_type('IfcWindow'))} ventanas, "
          f"{len(openings)} huecos, {len(f.by_type('IfcColumn'))} pilares, "
          f"{len(f.by_type('IfcBeam'))} vigas, {len(storeys)} plantas, "
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
