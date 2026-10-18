#!/usr/bin/env python3
"""Generates the IFC files used to test the importer with files written by
another program (IfcOpenShell), not by ArchiSim.

    pip install ifcopenshell
    python3 scripts/make_ifc_fixtures.py

Writes, in crates/archisim-ifc/tests/fixtures/:
  - ifcopenshell_ifc4_m.ifc     IFC4, metres, walls with an axis
  - ifcopenshell_ifc2x3_mm.ifc  IFC2X3, millimetres, walls without an axis
  - ifcopenshell_formas.ifc     IFC2X3, millimetres, written entity by entity
                                as Revit does: an L-shaped wall (polyline
                                axis), a quarter-circle wall (trimmed circle
                                with angles in degrees), a round wall (whole
                                circle), a slab with a hole in its profile
                                and a slab with a shaft (an opening element)

The house is our own invention: 10 × 7 m, a façade of four layers drawn
counter-clockwise (so its exterior is on the right of each wall, the
opposite of ArchiSim), a partition, a door and a window in the façade, a
door in the partition, a ground slab, a column (which ArchiSim does not
import yet) and an empty opening. GlobalIds are fixed so the files do not
change from run to run.
"""

import math
import os
import sys

import numpy as np
import ifcopenshell
import ifcopenshell.api.aggregate
import ifcopenshell.api.context
import ifcopenshell.api.feature
import ifcopenshell.api.geometry
import ifcopenshell.api.material
import ifcopenshell.api.owner
import ifcopenshell.api.owner.settings
import ifcopenshell.api.project
import ifcopenshell.api.pset
import ifcopenshell.api.root
import ifcopenshell.api.spatial
import ifcopenshell.api.style
import ifcopenshell.api.type
import ifcopenshell.api.unit
import ifcopenshell.guid

OUT = os.path.join(os.path.dirname(__file__), "..", "crates", "archisim-ifc", "tests", "fixtures")

counter = 0


def fixed_guids(f):
    """Replaces every GlobalId by a counter, so the output is stable."""
    n = 0
    for e in f:
        if hasattr(e, "GlobalId"):
            n += 1
            e.GlobalId = ifcopenshell.guid.compress("%032x" % (0x5A000000 + n))


def placement(f, element, x, y, z, angle=0.0):
    m = np.eye(4)
    c, s = math.cos(angle), math.sin(angle)
    m[0][0], m[0][1], m[1][0], m[1][1] = c, -s, s, c
    m[0][3], m[1][3], m[2][3] = x, y, z
    ifcopenshell.api.geometry.edit_object_placement(f, product=element, matrix=m, is_si=True)


def build(schema, millimetres, with_axis, path):
    f = ifcopenshell.api.project.create_file(version=schema)
    # IFC2X3 needs an owner history on every element.
    person = ifcopenshell.api.owner.add_person(f, identification="prueba", family_name="Prueba")
    org = ifcopenshell.api.owner.add_organisation(f, identification="AS", name="ArchiSim pruebas")
    user = ifcopenshell.api.owner.add_person_and_organisation(f, person=person, organisation=org)
    app = ifcopenshell.api.owner.add_application(f)
    ifcopenshell.api.owner.settings.get_user = lambda f: user
    ifcopenshell.api.owner.settings.get_application = lambda f: app
    project = ifcopenshell.api.root.create_entity(f, ifc_class="IfcProject", name="Casa de prueba")
    length = ifcopenshell.api.unit.add_si_unit(f, unit_type="LENGTHUNIT", prefix="MILLI" if millimetres else None)
    area = ifcopenshell.api.unit.add_si_unit(f, unit_type="AREAUNIT")
    volume = ifcopenshell.api.unit.add_si_unit(f, unit_type="VOLUMEUNIT")
    degree = ifcopenshell.api.unit.add_conversion_based_unit(f, name="degree")
    ifcopenshell.api.unit.assign_unit(f, units=[length, area, volume, degree])
    k = 1000.0 if millimetres else 1.0  # project units per metre

    model = ifcopenshell.api.context.add_context(f, context_type="Model")
    body = ifcopenshell.api.context.add_context(
        f, context_type="Model", context_identifier="Body", target_view="MODEL_VIEW", parent=model
    )
    axis_ctx = ifcopenshell.api.context.add_context(
        f, context_type="Model", context_identifier="Axis", target_view="GRAPH_VIEW", parent=model
    )

    site = ifcopenshell.api.root.create_entity(f, ifc_class="IfcSite", name="Parcela")
    building = ifcopenshell.api.root.create_entity(f, ifc_class="IfcBuilding", name="Vivienda")
    ifcopenshell.api.aggregate.assign_object(f, products=[site], relating_object=project)
    ifcopenshell.api.aggregate.assign_object(f, products=[building], relating_object=site)
    storeys = []
    for name, z in [("Planta baja", 0.0), ("Planta primera", 3.0)]:
        s = ifcopenshell.api.root.create_entity(f, ifc_class="IfcBuildingStorey", name=name)
        placement(f, s, 0, 0, z)
        s.Elevation = z * k
        storeys.append(s)
    ifcopenshell.api.aggregate.assign_object(f, products=storeys, relating_object=building)
    ground = storeys[0]
    for e in [site, building]:
        placement(f, e, 0, 0, 0)

    def material(name, category, rgb, transparency=0.0):
        m = ifcopenshell.api.material.add_material(f, name=name, category=category if schema != "IFC2X3" else None)
        style = ifcopenshell.api.style.add_style(f, name=name)
        ifcopenshell.api.style.add_surface_style(
            f,
            style=style,
            ifc_class="IfcSurfaceStyleShading",
            attributes=dict(
                {"SurfaceColour": {"Name": None, "Red": rgb[0], "Green": rgb[1], "Blue": rgb[2]}},
                # IFC2X3 shading has no transparency.
                **({} if schema == "IFC2X3" else {"Transparency": transparency}),
            ),
        )
        ifcopenshell.api.style.assign_material_style(
            f, material=m, style=style, context=body, should_use_presentation_style_assignment=schema == "IFC2X3"
        )
        return m

    render = material("Mortero monocapa", "plaster", (0.92, 0.9, 0.86))
    brick = material("Ladrillo perforado", "brick", (0.9, 0.77, 0.7))
    wool = material("Lana mineral", "insulation", (0.98, 0.95, 0.8))
    lhd = material("Ladrillo hueco doble", "brick", (0.94, 0.84, 0.78))
    plaster = material("Enlucido de yeso", "plaster", (0.96, 0.96, 0.96))
    concrete = material("Hormigón armado", "concrete", (0.8, 0.8, 0.8))
    wood = material("Madera", "wood", (0.77, 0.63, 0.44))
    glass = material("Vidrio", "glass", (0.75, 0.87, 0.93), 0.6)

    def layered_type(ifc_class, name, layers, predefined):
        t = ifcopenshell.api.root.create_entity(f, ifc_class=ifc_class, name=name, predefined_type=predefined)
        s = ifcopenshell.api.material.add_material_set(f, name=name, set_type="IfcMaterialLayerSet")
        for m, thickness in layers:
            layer = ifcopenshell.api.material.add_layer(f, layer_set=s, material=m)
            ifcopenshell.api.material.edit_layer(f, layer=layer, attributes={"LayerThickness": thickness * k})
        ifcopenshell.api.material.assign_material(f, products=[t], type="IfcMaterialLayerSet", material=s)
        return t

    wall_type_class = "IfcWallType"
    facade_t = layered_type(
        wall_type_class,
        "Fachada 4 capas",
        [(render, 0.015), (brick, 0.115), (wool, 0.05), (plaster, 0.015)],
        "STANDARD",
    )
    partition_t = layered_type(
        wall_type_class, "Tabique 10", [(plaster, 0.015), (lhd, 0.07), (plaster, 0.015)], "STANDARD"
    )
    slab_t = layered_type("IfcSlabType", "Solera 20", [(concrete, 0.2)], "BASESLAB")

    wall_class = "IfcWallStandardCase" if schema == "IFC2X3" else "IfcWall"

    def wall(name, wtype, p1, p2, thickness, external):
        w = ifcopenshell.api.root.create_entity(f, ifc_class=wall_class, name=name)
        length_m = math.dist(p1, p2)
        angle = math.atan2(p2[1] - p1[1], p2[0] - p1[0])
        placement(f, w, p1[0], p1[1], 0.0, angle)
        rep = ifcopenshell.api.geometry.add_wall_representation(
            f, context=body, length=length_m, height=2.8, thickness=thickness
        )
        ifcopenshell.api.geometry.assign_representation(f, product=w, representation=rep)
        if with_axis:
            ax = ifcopenshell.api.geometry.add_axis_representation(
                f, context=axis_ctx, axis=((0.0, 0.0), (length_m * k, 0.0))
            )
            ifcopenshell.api.geometry.assign_representation(f, product=w, representation=ax)
        ifcopenshell.api.type.assign_type(f, related_objects=[w], relating_type=wtype)
        pset = ifcopenshell.api.pset.add_pset(f, product=w, name="Pset_WallCommon")
        ifcopenshell.api.pset.edit_pset(f, pset=pset, properties={"IsExternal": external})
        ifcopenshell.api.spatial.assign_container(f, products=[w], relating_structure=ground)
        return w

    # Counter-clockwise: the inside of the house is on the left of each wall.
    corners = [(0.0, 0.0), (10.0, 0.0), (10.0, 7.0), (0.0, 7.0)]
    facade = [
        wall("Fachada %d" % (i + 1), facade_t, corners[i], corners[(i + 1) % 4], 0.195, True) for i in range(4)
    ]
    partition = wall("Tabique", partition_t, (5.0, 0.195), (5.0, 6.805), 0.1, False)

    def opening(host, along, sill, width, height, thickness):
        """An opening box in the host wall's coordinates."""
        o = ifcopenshell.api.root.create_entity(f, ifc_class="IfcOpeningElement")
        rep = ifcopenshell.api.geometry.add_wall_representation(
            f, context=body, length=width, height=height, thickness=thickness + 0.1
        )
        ifcopenshell.api.geometry.assign_representation(f, product=o, representation=rep)
        wm = ifcopenshell.util.placement.get_local_placement(host.ObjectPlacement)
        local = np.eye(4)
        local[0][3], local[1][3], local[2][3] = along - width / 2, -0.05, sill
        world = wm.copy()
        world[:3, 3] /= k
        m = world @ local
        ifcopenshell.api.geometry.edit_object_placement(f, product=o, matrix=m, is_si=True)
        ifcopenshell.api.feature.add_feature(f, feature=o, element=host)
        return o, m

    def filling(ifc_class, ifc_type, o, m, width, height, thickness, rep_kwargs, name):
        d = ifcopenshell.api.root.create_entity(f, ifc_class=ifc_class, name=name)
        d.OverallWidth = width * k
        d.OverallHeight = height * k
        # Inside the opening, centred across the wall: the frame plane.
        local = np.eye(4)
        local[1][3] = 0.05 + (thickness - 0.07) / 2
        ifcopenshell.api.geometry.edit_object_placement(f, product=d, matrix=m @ local, is_si=True)
        if ifc_class == "IfcDoor":
            rep = ifcopenshell.api.geometry.add_door_representation(
                f, context=body, overall_height=height * k, overall_width=width * k, **rep_kwargs
            )
        else:
            rep = ifcopenshell.api.geometry.add_window_representation(
                f, context=body, overall_height=height * k, overall_width=width * k, **rep_kwargs
            )
        ifcopenshell.api.geometry.assign_representation(f, product=d, representation=rep)
        ifcopenshell.api.type.assign_type(f, related_objects=[d], relating_type=ifc_type, should_map_representations=False)
        ifcopenshell.api.feature.add_filling(f, opening=o, element=d)
        ifcopenshell.api.spatial.assign_container(f, products=[d], relating_structure=ground)
        return d

    def opening_type(ifc_class, name, operation, lining, panels=None):
        if schema == "IFC2X3":
            style_class = "IfcDoorStyle" if ifc_class == "IfcDoorType" else "IfcWindowStyle"
            t = ifcopenshell.api.root.create_entity(f, ifc_class=style_class, name=name)
            if style_class == "IfcDoorStyle":
                t.OperationType = operation
                t.ConstructionType = "WOOD"
            else:
                t.OperationType = operation
                t.ConstructionType = "ALUMINIUM"
            t.ParameterTakesPrecedence = False
            t.Sizeable = False
        else:
            t = ifcopenshell.api.root.create_entity(f, ifc_class=ifc_class, name=name)
            if ifc_class == "IfcDoorType":
                t.PredefinedType = "DOOR"
                t.OperationType = operation
            else:
                t.PredefinedType = "WINDOW"
                t.PartitioningType = operation
        props = []
        if lining:
            cls = "IfcDoorLiningProperties" if "Door" in t.is_a() else "IfcWindowLiningProperties"
            props.append(
                f.create_entity(
                    cls,
                    GlobalId=ifcopenshell.guid.new(),
                    LiningDepth=lining[0] * k,
                    LiningThickness=lining[1] * k,
                )
            )
        for op, pos in panels or []:
            props.append(
                f.create_entity(
                    "IfcWindowPanelProperties", GlobalId=ifcopenshell.guid.new(), OperationType=op, PanelPosition=pos
                )
            )
        if props:
            t.HasPropertySets = props
        return t

    entrance_t = opening_type("IfcDoorType", "Puerta exterior 100", "SINGLE_SWING_RIGHT", (0.07, 0.06))
    window_t = opening_type(
        "IfcWindowType",
        "Ventana 2 hojas 150×120",
        "DOUBLE_PANEL_VERTICAL",
        (0.07, 0.06),
        [("SIDEHUNGLEFTHAND", "LEFT"), ("SIDEHUNGRIGHTHAND", "RIGHT")],
    )
    inner_t = opening_type("IfcDoorType", "Puerta de paso 80", "SINGLE_SWING_LEFT", None)
    for t in [entrance_t, window_t, inner_t]:
        is_window = "Window" in t.is_a()
        parts = [("Framing" if is_window else "Lining", wood), ("Glazing" if is_window else "Panel", glass if is_window else wood)]
        if schema == "IFC2X3":
            # IFC2X3 has no constituent sets: a plain list, frame first.
            ml = f.create_entity("IfcMaterialList", Materials=[m for _, m in parts])
            f.create_entity(
                "IfcRelAssociatesMaterial",
                GlobalId=ifcopenshell.guid.new(),
                OwnerHistory=ifcopenshell.api.owner.create_owner_history(f),
                RelatedObjects=[t],
                RelatingMaterial=ml,
            )
        else:
            cs = ifcopenshell.api.material.add_material_set(f, name=t.Name, set_type="IfcMaterialConstituentSet")
            for cname, m in parts:
                c = ifcopenshell.api.material.add_constituent(f, constituent_set=cs, material=m)
                c.Name = cname
            ifcopenshell.api.material.assign_material(f, products=[t], type="IfcMaterialConstituentSet", material=cs)

    door_kw = {"operation_type": "SINGLE_SWING_LEFT"}
    o, m = opening(facade[0], 3.0, 0.0, 1.0, 2.1, 0.195)
    filling("IfcDoor", entrance_t, o, m, 1.0, 2.1, 0.195, door_kw, "Puerta de entrada")
    o, m = opening(facade[1], 3.5, 1.0, 1.5, 1.2, 0.195)
    filling("IfcWindow", window_t, o, m, 1.5, 1.2, 0.195, {"partition_type": "DOUBLE_PANEL_VERTICAL", "panel_properties": [{}, {}]}, "Ventana salón")
    o, m = opening(partition, 2.0, 0.0, 0.8, 2.0, 0.1)
    filling("IfcDoor", inner_t, o, m, 0.8, 2.0, 0.1, door_kw, "Puerta de paso")
    # An empty opening (a passage without a door).
    opening(facade[2], 5.0, 0.0, 0.9, 2.0, 0.195)

    slab = ifcopenshell.api.root.create_entity(f, ifc_class="IfcSlab", name="Solera", predefined_type="BASESLAB")
    placement(f, slab, 0, 0, -0.2)
    rep = ifcopenshell.api.geometry.add_slab_representation(
        f, context=body, depth=0.2, polyline=[(0.0, 0.0), (10.0, 0.0), (10.0, 7.0), (0.0, 7.0)]
    )
    ifcopenshell.api.geometry.assign_representation(f, product=slab, representation=rep)
    ifcopenshell.api.type.assign_type(f, related_objects=[slab], relating_type=slab_t)
    ifcopenshell.api.spatial.assign_container(f, products=[slab], relating_structure=ground)

    column = ifcopenshell.api.root.create_entity(f, ifc_class="IfcColumn", name="Pilar")
    placement(f, column, 5.0, 3.5, 0.0)
    ifcopenshell.api.spatial.assign_container(f, products=[column], relating_structure=ground)

    fixed_guids(f)
    f.header.file_name.name = os.path.basename(path)
    f.header.file_name.time_stamp = "2026-10-17T00:00:00"
    f.write(path)
    print("Escrito:", os.path.relpath(path))


def build_shapes(path):
    """Walls and slabs of other shapes, written entity by entity."""
    f = ifcopenshell.file(schema="IFC2X3")
    g = iter(range(1, 10_000))

    def guid():
        return ifcopenshell.guid.compress("%032x" % (0x5B000000 + next(g)))

    person = f.createIfcPerson(None, "Prueba", None)
    org = f.createIfcOrganization(None, "ArchiSim pruebas")
    pao = f.createIfcPersonAndOrganization(person, org)
    app = f.createIfcApplication(org, "1.0", "Formas de prueba", "FP")
    oh = f.createIfcOwnerHistory(pao, app, None, "ADDED", None, None, None, 0)
    mm = f.createIfcSIUnit(None, "LENGTHUNIT", "MILLI", "METRE")
    rad = f.createIfcSIUnit(None, "PLANEANGLEUNIT", None, "RADIAN")
    deg = f.createIfcConversionBasedUnit(
        f.createIfcDimensionalExponents(0, 0, 0, 0, 0, 0, 0),
        "PLANEANGLEUNIT",
        "DEGREE",
        f.createIfcMeasureWithUnit(f.createIfcPlaneAngleMeasure(math.pi / 180), rad),
    )
    units = f.createIfcUnitAssignment([mm, deg])
    pt = lambda *c: f.createIfcCartesianPoint([float(x) for x in c])
    d3 = lambda *c: f.createIfcDirection([float(x) for x in c])
    origin = f.createIfcAxis2Placement3D(pt(0, 0, 0), None, None)
    ctx = f.createIfcGeometricRepresentationContext(None, "Model", 3, 1e-5, origin, None)
    project = f.createIfcProject(guid(), oh, "Formas", None, None, None, None, [ctx], units)
    site_pl = f.createIfcLocalPlacement(None, origin)
    site = f.createIfcSite(guid(), oh, "Parcela", None, None, site_pl, None, None, "ELEMENT", None, None, None, None, None)
    bld_pl = f.createIfcLocalPlacement(site_pl, origin)
    building = f.createIfcBuilding(guid(), oh, "Edificio", None, None, bld_pl, None, None, "ELEMENT", None, None, None)
    st_pl = f.createIfcLocalPlacement(bld_pl, origin)
    storey = f.createIfcBuildingStorey(guid(), oh, "Planta baja", None, None, st_pl, None, None, "ELEMENT", 0.0)
    f.createIfcRelAggregates(guid(), oh, None, None, project, [site])
    f.createIfcRelAggregates(guid(), oh, None, None, site, [building])
    f.createIfcRelAggregates(guid(), oh, None, None, building, [storey])
    brick = f.createIfcMaterial("Ladrillo")
    concrete = f.createIfcMaterial("Hormigón")
    contained = []

    def poly(points):
        pts = [pt(*p) for p in points]
        return f.createIfcPolyline(pts + [pts[0]])

    def body(profile, depth):
        solid = f.createIfcExtrudedAreaSolid(profile, origin, d3(0, 0, 1), depth)
        return f.createIfcShapeRepresentation(ctx, "Body", "SweptSolid", [solid])

    def layered(element, material, thickness, name):
        layer = f.createIfcMaterialLayer(material, thickness, None)
        lset = f.createIfcMaterialLayerSet([layer], name)
        usage = f.createIfcMaterialLayerSetUsage(lset, "AXIS2", "POSITIVE", -thickness / 2)
        f.createIfcRelAssociatesMaterial(guid(), oh, None, None, [element], usage)

    def wall(name, axis_curve, profile, thickness):
        pl = f.createIfcLocalPlacement(st_pl, origin)
        axis = f.createIfcShapeRepresentation(ctx, "Axis", "Curve2D", [axis_curve])
        shape = f.createIfcProductDefinitionShape(None, None, [axis, body(profile, 2700.0)])
        w = f.createIfcWall(guid(), oh, name, None, None, pl, shape, None)
        layered(w, brick, thickness, "Muro de ladrillo %d" % int(thickness))
        contained.append(w)
        return w

    def band(path_pts, half):
        """The outline of a wall of half-thickness `half` along a polyline."""
        left, right = [], []
        n = len(path_pts)
        for i, p in enumerate(path_pts):
            dirs = []
            if i > 0:
                dirs.append(np.subtract(p, path_pts[i - 1]))
            if i < n - 1:
                dirs.append(np.subtract(path_pts[i + 1], p))
            normals = [np.array([-d[1], d[0]]) / np.hypot(*d) for d in dirs]
            nrm = sum(normals)
            nrm = nrm / np.dot(nrm, normals[0])
            left.append(tuple(np.add(p, nrm * half)))
            right.append(tuple(np.subtract(p, nrm * half)))
        return left + right[::-1]

    # An L of two pieces in one wall: (0,0) → (6000,0) → (6000,4000).
    path_l = [(0.0, 0.0), (6000.0, 0.0), (6000.0, 4000.0)]
    l_axis = f.createIfcPolyline([pt(*p) for p in path_l])
    wall("Muro en L", l_axis, f.createIfcArbitraryClosedProfileDef("AREA", None, poly(band(path_l, 120.0))), 240.0)

    # A quarter circle of radius 4000 round (10000, 0), from 0° to 90°.
    c = (10000.0, 0.0)
    circle = f.createIfcCircle(f.createIfcAxis2Placement2D(pt(*c), None), 4000.0)
    arc_axis = f.createIfcTrimmedCurve(
        circle, [f.createIfcParameterValue(0.0)], [f.createIfcParameterValue(90.0)], True, "PARAMETER"
    )
    arc_pts = [
        (c[0] + r * math.cos(a), c[1] + r * math.sin(a))
        for r, angles in [(4120.0, range(0, 91, 3)), (3880.0, range(90, -1, -3))]
        for a in [math.radians(x) for x in angles]
    ]
    wall("Muro curvo", arc_axis, f.createIfcArbitraryClosedProfileDef("AREA", None, poly(arc_pts)), 240.0)

    # A round wall of radius 1500 round (3000, 8000): a whole circle.
    rc = (3000.0, 8000.0)
    round_axis = f.createIfcCircle(f.createIfcAxis2Placement2D(pt(*rc), None), 1500.0)
    ring = lambda r: [(rc[0] + r * math.cos(math.radians(a)), rc[1] + r * math.sin(math.radians(a))) for a in range(0, 360, 5)]
    wall(
        "Muro redondo",
        round_axis,
        f.createIfcArbitraryProfileDefWithVoids("AREA", None, poly(ring(1600.0)), [poly(ring(1400.0)[::-1])]),
        200.0,
    )

    def slab(name, profile, z):
        pl = f.createIfcLocalPlacement(st_pl, f.createIfcAxis2Placement3D(pt(0, 0, z), None, None))
        shape = f.createIfcProductDefinitionShape(None, None, [body(profile, 300.0)])
        s = f.createIfcSlab(guid(), oh, name, None, None, pl, shape, None, "FLOOR")
        layer = f.createIfcMaterialLayer(concrete, 300.0, None)
        usage = f.createIfcMaterialLayerSetUsage(f.createIfcMaterialLayerSet([layer], "Losa 30"), "AXIS3", "POSITIVE", 0.0)
        f.createIfcRelAssociatesMaterial(guid(), oh, None, None, [s], usage)
        contained.append(s)
        return s, pl

    # A slab of 8 × 6 m with a stair hole of 1 × 3 m in its profile.
    outer = [(20000.0, 0.0), (28000.0, 0.0), (28000.0, 6000.0), (20000.0, 6000.0)]
    stair = [(21000.0, 1000.0), (21000.0, 4000.0), (22000.0, 4000.0), (22000.0, 1000.0)]
    slab("Losa con escalera", f.createIfcArbitraryProfileDefWithVoids("AREA", None, poly(outer), [poly(stair)]), -300.0)

    # A slab of 6 × 5 m with a shaft of 0.6 × 0.8 m cut by an opening element.
    plain = [(30000.0, 0.0), (36000.0, 0.0), (36000.0, 5000.0), (30000.0, 5000.0)]
    shafted, spl = slab("Losa con patinillo", f.createIfcArbitraryClosedProfileDef("AREA", None, poly(plain)), -300.0)
    opl = f.createIfcLocalPlacement(spl, f.createIfcAxis2Placement3D(pt(33000.0, 2000.0, -50.0), None, None))
    hole = f.createIfcRectangleProfileDef("AREA", None, f.createIfcAxis2Placement2D(pt(300.0, 400.0), None), 600.0, 800.0)
    oshape = f.createIfcProductDefinitionShape(None, None, [body(hole, 400.0)])
    shaft = f.createIfcOpeningElement(guid(), oh, "Patinillo", None, None, opl, oshape, None)
    f.createIfcRelVoidsElement(guid(), oh, None, None, shafted, shaft)

    f.createIfcRelContainedInSpatialStructure(guid(), oh, None, None, contained, storey)
    f.header.file_name.name = os.path.basename(path)
    f.header.file_name.time_stamp = "2026-10-18T00:00:00"
    f.write(path)
    print("Escrito:", os.path.relpath(path))


def main():
    os.makedirs(OUT, exist_ok=True)
    build_shapes(os.path.join(OUT, "ifcopenshell_formas.ifc"))
    build("IFC4", False, True, os.path.join(OUT, "ifcopenshell_ifc4_m.ifc"))
    build("IFC2X3", True, False, os.path.join(OUT, "ifcopenshell_ifc2x3_mm.ifc"))


if __name__ == "__main__":
    import ifcopenshell.util.placement  # noqa: F401  (used by opening())

    sys.exit(main())
