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
  7. Every room (IfcSpace) is aggregated into a storey, has its common
     property set and base quantities, and its 3D volume matches them.
  5. Every opening voids a wall and is filled by exactly one door or window
     (a void, "hueco", marked with ArchiSim_Void, by none);
     every door and window fills an opening, has a type, a material
     constituent set, its common property set and quantities, and 3D
     geometry. Wall volumes are compared with their net volume, after the
     openings are cut.
  9. Every stair (IfcStair) is contained in a storey, made of flights
     with their material and quantities (3D volume included) and landings,
     and its risers add up.
  8. Every pitched roof (IfcRoof) is contained in a storey, made of roof
     slabs (checked as slabs above) and has its common property set and
     base quantities, its area the sum of its planes'.
 10. Every curtain wall (IfcCurtainWall) is contained in a storey, has its
     common property set and quantities, and is made of mullions
     (IfcMember) and panels (IfcPlate) with their material, quantities and
     a 3D volume matching them.
 12. Every object (IfcFurniture, IfcSanitaryTerminal) is contained in a
     storey, has its type and 3D geometry.
 11. Every loose railing (IfcRailing not part of a stair) is contained in
     a storey, has its material, Pset_RailingCommon and 3D geometry.

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

    def in_stair(el):
        return any(rel.RelatingObject.is_a("IfcStair") for rel in el.Decomposes)

    slabs = [s for s in f.by_type("IfcSlab") if not in_stair(s)]
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

    spaces = f.by_type("IfcSpace")
    for sp in spaces:
        name = f"{sp.Name} {sp.LongName}"
        parent = sp.Decomposes[0].RelatingObject if sp.Decomposes else None
        if parent is None or not parent.is_a("IfcBuildingStorey"):
            errors.append(f"{name}: not aggregated into a storey")
        psets = eu.get_psets(sp)
        for required in ("Pset_SpaceCommon", "Qto_SpaceBaseQuantities"):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        q = psets.get("Qto_SpaceBaseQuantities", {})
        if q and q.get("GrossFloorArea", 0) + 1e-9 < q.get("NetFloorArea", 0):
            errors.append(f"{name}: gross area smaller than net area")
        if geom_ok and q:
            try:
                shp = geom.create_shape(settings, sp)
                vol = shape.get_volume(shp.geometry)
                if abs(vol - q["NetVolume"]) > 1e-4:
                    errors.append(f"{name}: volume {vol:.4f} != {q['NetVolume']:.4f}")
            except Exception as exc:
                errors.append(f"{name}: geometry failed: {exc}")

    roofs = f.by_type("IfcRoof")
    for r in roofs:
        name = r.Name
        parts = [p for rel in r.IsDecomposedBy for p in rel.RelatedObjects]
        if not parts or not all(p.is_a("IfcSlab") and p.PredefinedType == "ROOF" for p in parts):
            errors.append(f"{name}: not made of roof slabs")
        if not r.ContainedInStructure:
            errors.append(f"{name}: not contained in a storey")
        psets = eu.get_psets(r)
        for required in ("Pset_RoofCommon", "Qto_RoofBaseQuantities"):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        q = psets.get("Qto_RoofBaseQuantities", {})
        surface = sum(eu.get_psets(p).get("Qto_SlabBaseQuantities", {}).get("NetArea", 0)
                      for p in parts)
        if q and abs(q.get("NetArea", 0) - surface) > 1e-4:
            errors.append(f"{name}: area {q.get('NetArea')} != planes {surface}")

    stairs = f.by_type("IfcStair")
    for st in stairs:
        name = st.Name
        parts = [p for rel in st.IsDecomposedBy for p in rel.RelatedObjects]
        flights = [p for p in parts if p.is_a("IfcStairFlight")]
        if not flights:
            errors.append(f"{name}: no flights")
        if not st.ContainedInStructure:
            errors.append(f"{name}: not contained in a storey")
        psets = eu.get_psets(st)
        if "Pset_StairCommon" not in psets:
            errors.append(f"{name}: missing Pset_StairCommon")
        risers = sum(fl.NumberOfRisers or 0 for fl in flights)
        if psets.get("Pset_StairCommon", {}).get("NumberOfRiser") != risers:
            errors.append(f"{name}: risers {risers} do not add up")
        for fl in flights:
            if eu.get_material(fl) is None:
                errors.append(f"{fl.Name}: no material")
            q = eu.get_psets(fl).get("Qto_StairFlightBaseQuantities", {})
            if geom_ok and q:
                try:
                    shp = geom.create_shape(settings, fl)
                    vol = shape.get_volume(shp.geometry)
                    if abs(vol - q["NetVolume"]) > 1e-4:
                        errors.append(f"{fl.Name}: volume {vol:.4f} != {q['NetVolume']:.4f}")
                except Exception as exc:
                    errors.append(f"{fl.Name}: geometry failed: {exc}")

    curtains = f.by_type("IfcCurtainWall")
    for cw in curtains:
        name = cw.Name
        if not cw.ContainedInStructure:
            errors.append(f"{name}: not contained in a storey")
        psets = eu.get_psets(cw)
        for required in ("Pset_CurtainWallCommon", "Qto_CurtainWallQuantities"):
            if required not in psets:
                errors.append(f"{name}: missing {required}")
        parts = [p for rel in cw.IsDecomposedBy for p in rel.RelatedObjects]
        if not any(p.is_a("IfcMember") for p in parts):
            errors.append(f"{name}: no mullions")
        for p in parts:
            if eu.get_material(p) is None:
                errors.append(f"{p.Name}: no material")
            qto = "Qto_PlateBaseQuantities" if p.is_a("IfcPlate") else "Qto_MemberBaseQuantities"
            q = eu.get_psets(p).get(qto, {})
            if not q:
                errors.append(f"{p.Name}: missing {qto}")
            elif geom_ok:
                try:
                    shp = geom.create_shape(settings, p)
                    vol = shape.get_volume(shp.geometry)
                    if abs(vol - q["NetVolume"]) > 1e-6:
                        errors.append(f"{p.Name}: volume {vol:.6f} != {q['NetVolume']:.6f}")
                except Exception as exc:
                    errors.append(f"{p.Name}: geometry failed: {exc}")

    railings = [r for r in f.by_type("IfcRailing") if not r.Decomposes]
    for r in railings:
        if not r.ContainedInStructure:
            errors.append(f"{r.Name}: not contained in a storey")
        if eu.get_material(r) is None:
            errors.append(f"{r.Name}: no material")
        if "Pset_RailingCommon" not in eu.get_psets(r):
            errors.append(f"{r.Name}: missing Pset_RailingCommon")
        if geom_ok:
            try:
                geom.create_shape(settings, r)
            except Exception as exc:
                errors.append(f"{r.Name}: geometry failed: {exc}")

    objects = f.by_type("IfcFurniture") + f.by_type("IfcSanitaryTerminal")
    for o in objects:
        if not o.ContainedInStructure:
            errors.append(f"{o.Name}: not contained in a storey")
        if eu.get_type(o) is None:
            errors.append(f"{o.Name}: no type")
        if geom_ok:
            try:
                geom.create_shape(settings, o)
            except Exception as exc:
                errors.append(f"{o.Name}: geometry failed: {exc}")

    storeys = f.by_type("IfcBuildingStorey")
    types = sum(len(f.by_type(t)) for t in
                ("IfcWallType", "IfcSlabType", "IfcDoorType", "IfcWindowType",
                 "IfcColumnType", "IfcBeamType"))
    print(f"{path}: {len(walls)} muros, {len(slabs)} forjados, "
          f"{len(f.by_type('IfcDoor'))} puertas, {len(f.by_type('IfcWindow'))} ventanas, "
          f"{len(openings)} huecos, {len(f.by_type('IfcColumn'))} pilares, "
          f"{len(f.by_type('IfcBeam'))} vigas, {len(spaces)} locales, {len(roofs)} cubiertas, {len(stairs)} escaleras, {len(curtains)} muros cortina, {len(railings)} barandillas, {len(objects)} objetos, "
          f"{len(storeys)} plantas, "
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
