use super::*;
use crate::{export_ifc, ExportOptions};
use archisim_core::{NewOpening, NewSlab, NewWall};
use archisim_library::add_library;

/// A model with every kind of element and option the exporter writes.
fn model() -> Model {
    let mut m = Model::new("Casa «ida y vuelta»");
    let pb = m.add_level("Planta baja", 0.0);
    let p1 = m.add_level("Planta primera", 3.2);
    let lib = add_library(&mut m).unwrap();
    // Façade clockwise along the exterior faces.
    let c = [
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 8.0),
        Point2::new(10.0, 8.0),
        Point2::new(10.0, 0.0),
    ];
    let mut walls = Vec::new();
    for i in 0..4 {
        walls.push(
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: pb,
                start: c[i],
                end: c[(i + 1) % 4],
                reference: ReferenceLine::ExteriorFace,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap(),
        );
    }
    let partition = m
        .add_wall(NewWall {
            wall_type: lib.partition,
            level: pb,
            start: Point2::new(5.0, 0.265),
            end: Point2::new(5.0, 7.735),
            reference: ReferenceLine::Center,
            height: 2.7,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
    let bearing = m
        .add_wall(NewWall {
            wall_type: lib.bearing_wall,
            level: p1,
            start: Point2::new(1.0, 1.0),
            end: Point2::new(4.0, 1.0),
            reference: ReferenceLine::InteriorFace,
            height: 2.5,
            base_offset: 0.3,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
    let place = |opening_type, wall, along, sill, opens_to, hand, setback| NewOpening {
        opening_type,
        wall,
        along,
        sill,
        opens_to,
        hand,
        setback,
    };
    use WallSide::{Exterior, Interior};
    for o in [
        place(
            lib.entrance_door,
            walls[0],
            2.0,
            0.0,
            Interior,
            Hand::Right,
            Some(0.13),
        ),
        place(
            lib.sliding_door,
            walls[0],
            5.0,
            0.0,
            Exterior,
            Hand::Left,
            None,
        ),
        place(
            lib.window_two_leaves,
            walls[1],
            2.5,
            0.9,
            Interior,
            Hand::Left,
            Some(0.13),
        ),
        place(
            lib.window_one_leaf,
            walls[1],
            7.0,
            1.0,
            Interior,
            Hand::Right,
            None,
        ),
        place(
            lib.fixed_window,
            walls[2],
            4.0,
            1.6,
            Interior,
            Hand::Left,
            Some(0.0),
        ),
        place(
            lib.balcony_door,
            walls[3],
            6.0,
            0.0,
            Exterior,
            Hand::Right,
            Some(0.13),
        ),
        place(
            lib.double_door,
            partition,
            2.0,
            0.0,
            Interior,
            Hand::Left,
            None,
        ),
        place(
            lib.interior_door,
            partition,
            5.0,
            0.0,
            Exterior,
            Hand::Right,
            None,
        ),
        place(
            lib.interior_door,
            bearing,
            1.5,
            0.3,
            Interior,
            Hand::Left,
            Some(0.05),
        ),
    ] {
        m.add_opening(o).unwrap();
    }
    for (slab_type, level, top) in [
        (lib.ground_slab, pb, 0.0),
        (lib.floor_slab, p1, 0.0),
        (lib.flat_roof, p1, 3.0),
    ] {
        m.add_slab(NewSlab {
            slab_type,
            level,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 8.0),
                Point2::new(4.0, 8.0),
                Point2::new(4.0, 5.0),
                Point2::new(0.0, 5.0),
            ],
            top_offset: top,
        })
        .unwrap();
    }
    m
}

fn export(m: &Model) -> String {
    export_ifc(
        m,
        &ExportOptions {
            timestamp: Some("2026-10-17T12:00:00".into()),
            ..Default::default()
        },
    )
}

#[test]
fn round_trip_keeps_the_model() {
    let original = model();
    let ifc = export(&original);
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    let diffs = differences(&original, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    assert_eq!(report.levels, 2);
    assert_eq!(report.walls, 6);
    assert_eq!(report.slabs, 3);
    assert_eq!(report.doors, 5);
    assert_eq!(report.windows, 4);
    assert!(report.skipped.is_empty(), "{report}");
    assert!(report.warnings.is_empty(), "{report}");
    assert!(report.source.starts_with("ArchiSim"));
    // Exporting the imported model and importing it again changes nothing.
    let again = import_ifc(&export(&model)).unwrap();
    assert!(differences(&model, &again.model).is_empty());
    // Materials keep their order, unused ones included.
    let names = |m: &Model| m.materials().map(|x| x.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&original), names(&model));
}

#[test]
fn rejects_what_is_not_ifc() {
    assert!(matches!(import_ifc("hola"), Err(ImportError::Parse(_))));
    let step = "ISO-10303-21;HEADER;FILE_SCHEMA(('AP214'));ENDSEC;DATA;ENDSEC;END-ISO-10303-21;";
    let err = import_ifc(step).unwrap_err();
    assert!(matches!(err, ImportError::Schema(ref s) if s == "AP214"));
    assert!(err.to_string().contains("IFC2X3"));
}

/// Writes STEP data with numbered entities.
struct Data {
    lines: Vec<String>,
}

impl Data {
    fn add(&mut self, s: impl Into<String>) -> String {
        self.lines.push(s.into());
        format!("#{}", self.lines.len())
    }

    fn text(&self, schema: &str) -> String {
        let mut out = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');\n\
             FILE_NAME('prueba.ifc','2026-10-17T12:00:00',(''),(''),'Exportador de prueba','Programa de prueba','');\n\
             FILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n"
        );
        for (i, l) in self.lines.iter().enumerate() {
            out.push_str(&format!("#{}={};\n", i + 1, l));
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

const G: [&str; 12] = [
    "0aaaaaaaaaaaaaaaaaaaa0",
    "0aaaaaaaaaaaaaaaaaaaa1",
    "0aaaaaaaaaaaaaaaaaaaa2",
    "0aaaaaaaaaaaaaaaaaaaa3",
    "0aaaaaaaaaaaaaaaaaaaa4",
    "0aaaaaaaaaaaaaaaaaaaa5",
    "0aaaaaaaaaaaaaaaaaaaa6",
    "0aaaaaaaaaaaaaaaaaaaa7",
    "0aaaaaaaaaaaaaaaaaaaa8",
    "0aaaaaaaaaaaaaaaaaaaa9",
    "0aaaaaaaaaaaaaaaaaaaaA",
    "0aaaaaaaaaaaaaaaaaaaaB",
];

/// A Revit-like IFC2X3 file in millimetres: four walls drawn
/// counter-clockwise (exterior on the right), layers from the exterior on
/// the right side, a door, a slab with its layers from the top, a column.
fn revit_like() -> String {
    let mut d = Data { lines: Vec::new() };
    let o3 = d.add("IFCCARTESIANPOINT((0.,0.,0.))");
    let z = d.add("IFCDIRECTION((0.,0.,1.))");
    let wcs = d.add(format!("IFCAXIS2PLACEMENT3D({o3},$,$)"));
    let ctx = d.add(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,{wcs},$)"
    ));
    let mm = d.add("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
    let rad = d.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    let dim = d.add("IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0)");
    let mwu = d.add(format!(
        "IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.0174532925199433),{rad})"
    ));
    let deg = d.add(format!(
        "IFCCONVERSIONBASEDUNIT({dim},.PLANEANGLEUNIT.,'DEGREE',{mwu})"
    ));
    let units = d.add(format!("IFCUNITASSIGNMENT(({mm},{deg}))"));
    let project = d.add(format!(
        "IFCPROJECT('{}',$,'Proyecto Revit',$,$,$,$,({ctx}),{units})",
        G[0]
    ));
    let site_pl = d.add(format!("IFCLOCALPLACEMENT($,{wcs})"));
    let site = d.add(format!(
        "IFCSITE('{}',$,'Default',$,$,{site_pl},$,$,.ELEMENT.,$,$,$,$,$)",
        G[1]
    ));
    let bldg_pl = d.add(format!("IFCLOCALPLACEMENT({site_pl},{wcs})"));
    let bldg = d.add(format!(
        "IFCBUILDING('{}',$,'Edificio',$,$,{bldg_pl},$,$,.ELEMENT.,$,$,$)",
        G[2]
    ));
    let mut storeys = Vec::new();
    let mut storey_pls = Vec::new();
    for (i, (name, z_mm)) in [("Nivel 1", 0.0), ("Nivel 2", 3000.0)].iter().enumerate() {
        let p = d.add(format!("IFCCARTESIANPOINT((0.,0.,{z_mm:.1}))"));
        let a = d.add(format!("IFCAXIS2PLACEMENT3D({p},$,$)"));
        let pl = d.add(format!("IFCLOCALPLACEMENT({bldg_pl},{a})"));
        storeys.push(d.add(format!(
            "IFCBUILDINGSTOREY('{}',$,'{name}',$,$,{pl},$,$,.ELEMENT.,{z_mm:.1})",
            G[3 + i]
        )));
        storey_pls.push(pl);
    }
    d.add(format!(
        "IFCRELAGGREGATES('1aaaaaaaaaaaaaaaaaaaa0',$,$,$,{project},({site}))"
    ));
    d.add(format!(
        "IFCRELAGGREGATES('1aaaaaaaaaaaaaaaaaaaa1',$,$,$,{site},({bldg}))"
    ));
    d.add(format!(
        "IFCRELAGGREGATES('1aaaaaaaaaaaaaaaaaaaa2',$,$,$,{bldg},({},{}))",
        storeys[0], storeys[1]
    ));

    // Materials with a colour on the render.
    let render = d.add("IFCMATERIAL('Enfoscado de mortero')");
    let colour = d.add("IFCCOLOURRGB($,0.9,0.85,0.8)");
    let shading = d.add(format!(
        "IFCSURFACESTYLERENDERING({colour},0.,$,$,$,$,$,$,.NOTDEFINED.)"
    ));
    let style = d.add(format!("IFCSURFACESTYLE('Enfoscado',.BOTH.,({shading}))"));
    let assign = d.add(format!("IFCPRESENTATIONSTYLEASSIGNMENT(({style}))"));
    let item = d.add(format!("IFCSTYLEDITEM($,({assign}),$)"));
    let srep = d.add(format!(
        "IFCSTYLEDREPRESENTATION({ctx},'Style','Material',({item}))"
    ));
    d.add(format!(
        "IFCMATERIALDEFINITIONREPRESENTATION($,$,({srep}),{render})"
    ));
    let brick = d.add("IFCMATERIAL('Ladrillo perforado')");
    let wool = d.add("IFCMATERIAL('Lana mineral')");
    let plaster = d.add("IFCMATERIAL('Yeso')");
    let tile = d.add("IFCMATERIAL('Baldosa cerámica')");
    let concrete = d.add("IFCMATERIAL('Hormigón armado')");
    let membrane = d.add("IFCMATERIAL('Barrera de vapor')");
    // Exterior → interior, laid towards +Y from the exterior on the right.
    let layers: Vec<String> = [
        (render, 15.0),
        (brick, 115.0),
        (membrane, 0.0),
        (wool, 50.0),
        (plaster, 15.0),
    ]
    .iter()
    .map(|(m, t)| d.add(format!("IFCMATERIALLAYER({m},{t:.1},$)")))
    .collect();
    let wall_set = d.add(format!(
        "IFCMATERIALLAYERSET(({}),'Fachada Revit 195')",
        layers.join(",")
    ));
    let wall_usage = d.add(format!(
        "IFCMATERIALLAYERSETUSAGE({wall_set},.AXIS2.,.POSITIVE.,-97.5)"
    ));
    let wall_type = d.add(format!(
        "IFCWALLTYPE('{}',$,'Muro básico: Fachada 195',$,$,$,$,$,$,.STANDARD.)",
        G[5]
    ));
    let is_ext = d.add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$)");
    let pset = d.add(format!(
        "IFCPROPERTYSET('2aaaaaaaaaaaaaaaaaaaa0',$,'Pset_WallCommon',$,({is_ext}))"
    ));

    // Walls counter-clockwise: the interior is on their left.
    let corners: [(f64, f64); 4] = [(0.0, 0.0), (10000.0, 0.0), (10000.0, 8000.0), (0.0, 8000.0)];
    let mut walls = Vec::new();
    let mut first_wall_pl = String::new();
    for i in 0..4 {
        let (x0, y0) = corners[i];
        let (x1, y1) = corners[(i + 1) % 4];
        let len = (x1 - x0).hypot(y1 - y0);
        let (ux, uy) = ((x1 - x0) / len, (y1 - y0) / len);
        let p = d.add(format!("IFCCARTESIANPOINT(({x0:.1},{y0:.1},0.))"));
        let xd = d.add(format!("IFCDIRECTION(({ux:.1},{uy:.1},0.))"));
        let a = d.add(format!("IFCAXIS2PLACEMENT3D({p},{z},{xd})"));
        let pl = d.add(format!("IFCLOCALPLACEMENT({},{a})", storey_pls[0]));
        if i == 0 {
            first_wall_pl = pl.clone();
        }
        let q0 = d.add("IFCCARTESIANPOINT((0.,0.))");
        let q1 = d.add(format!("IFCCARTESIANPOINT(({len:.1},0.))"));
        let axis = d.add(format!("IFCPOLYLINE(({q0},{q1}))"));
        let axis_rep = d.add(format!(
            "IFCSHAPEREPRESENTATION({ctx},'Axis','Curve2D',({axis}))"
        ));
        let c2 = d.add(format!("IFCCARTESIANPOINT(({:.1},0.))", len / 2.0));
        let a2 = d.add(format!("IFCAXIS2PLACEMENT2D({c2},$)"));
        let prof = d.add(format!(
            "IFCRECTANGLEPROFILEDEF(.AREA.,$,{a2},{len:.1},195.)"
        ));
        let solid = d.add(format!("IFCEXTRUDEDAREASOLID({prof},{wcs},{z},2800.)"));
        let body = d.add(format!(
            "IFCSHAPEREPRESENTATION({ctx},'Body','SweptSolid',({solid}))"
        ));
        let shape = d.add(format!(
            "IFCPRODUCTDEFINITIONSHAPE($,$,({axis_rep},{body}))"
        ));
        walls.push(d.add(format!(
            "IFCWALLSTANDARDCASE('{}{i}',$,'Muro básico:Fachada 195:{i}',$,'Muro básico:Fachada 195',{pl},{shape},'{i}')",
            "3aaaaaaaaaaaaaaaaaaaa"
        )));
    }
    d.add(format!(
        "IFCRELASSOCIATESMATERIAL('2aaaaaaaaaaaaaaaaaaaa1',$,$,$,({}),{wall_usage})",
        walls.join(",")
    ));
    d.add(format!(
        "IFCRELDEFINESBYTYPE('2aaaaaaaaaaaaaaaaaaaa2',$,$,$,({}),{wall_type})",
        walls.join(",")
    ));
    d.add(format!(
        "IFCRELDEFINESBYPROPERTIES('2aaaaaaaaaaaaaaaaaaaa3',$,$,$,({}),{pset})",
        walls.join(",")
    ));

    // A door in the first (south) wall, from x = 2.0 to 2.9 m.
    let p = d.add("IFCCARTESIANPOINT((2000.,-200.,0.))");
    let a = d.add(format!("IFCAXIS2PLACEMENT3D({p},$,$)"));
    let void_pl = d.add(format!("IFCLOCALPLACEMENT({first_wall_pl},{a})"));
    let c2 = d.add("IFCCARTESIANPOINT((450.,200.))");
    let a2 = d.add(format!("IFCAXIS2PLACEMENT2D({c2},$)"));
    let prof = d.add(format!("IFCRECTANGLEPROFILEDEF(.AREA.,$,{a2},900.,400.)"));
    let solid = d.add(format!("IFCEXTRUDEDAREASOLID({prof},{wcs},{z},2100.)"));
    let body = d.add(format!(
        "IFCSHAPEREPRESENTATION({ctx},'Body','SweptSolid',({solid}))"
    ));
    let shape = d.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
    let void = d.add(format!(
        "IFCOPENINGELEMENT('4aaaaaaaaaaaaaaaaaaaa0',$,'Hueco',$,'Opening',{void_pl},{shape},$)"
    ));
    d.add(format!(
        "IFCRELVOIDSELEMENT('4aaaaaaaaaaaaaaaaaaaa1',$,$,$,{},{void})",
        walls[0]
    ));
    // The door: its +Y towards the interior (world +Y), frame centred.
    let p = d.add("IFCCARTESIANPOINT((0.,200.,0.))");
    let a = d.add(format!("IFCAXIS2PLACEMENT3D({p},$,$)"));
    let door_pl = d.add(format!("IFCLOCALPLACEMENT({void_pl},{a})"));
    let c2 = d.add("IFCCARTESIANPOINT((450.,0.))");
    let a2 = d.add(format!("IFCAXIS2PLACEMENT2D({c2},$)"));
    let prof = d.add(format!("IFCRECTANGLEPROFILEDEF(.AREA.,$,{a2},900.,70.)"));
    let solid = d.add(format!("IFCEXTRUDEDAREASOLID({prof},{wcs},{z},2100.)"));
    let body = d.add(format!(
        "IFCSHAPEREPRESENTATION({ctx},'Body','SweptSolid',({solid}))"
    ));
    let shape = d.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
    let door = d.add(format!(
        "IFCDOOR('{}',$,'Puerta sencilla:0915 x 2134mm',$,'0915 x 2134mm',{door_pl},{shape},'100',2100.,900.)",
        G[6]
    ));
    d.add(format!(
        "IFCRELFILLSELEMENT('4aaaaaaaaaaaaaaaaaaaa2',$,$,$,{void},{door})"
    ));
    let lining =
        d.add("IFCDOORLININGPROPERTIES('4aaaaaaaaaaaaaaaaaaaa3',$,$,$,70.,45.,$,$,$,$,$,$,$,$,$)");
    let style = d.add(format!(
        "IFCDOORSTYLE('{}',$,'Puerta sencilla 90',$,$,({lining}),$,$,.SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.)",
        G[7]
    ));
    d.add(format!(
        "IFCRELDEFINESBYTYPE('4aaaaaaaaaaaaaaaaaaaa4',$,$,$,({door}),{style})"
    ));
    let wood = d.add("IFCMATERIAL('Madera de roble')");
    d.add(format!(
        "IFCRELASSOCIATESMATERIAL('4aaaaaaaaaaaaaaaaaaaa5',$,$,$,({door}),{wood})"
    ));

    // A slab under the walls, top at ±0, layers from the top down.
    let pts: Vec<String> = [
        (0.0, 0.0),
        (10000.0, 0.0),
        (10000.0, 8000.0),
        (0.0, 8000.0),
        (0.0, 0.0),
    ]
    .iter()
    .map(|(x, y)| d.add(format!("IFCCARTESIANPOINT(({x:.1},{y:.1}))")))
    .collect();
    let curve = d.add(format!("IFCPOLYLINE(({}))", pts.join(",")));
    let prof = d.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"));
    let p = d.add("IFCCARTESIANPOINT((0.,0.,-250.))");
    let a = d.add(format!("IFCAXIS2PLACEMENT3D({p},$,$)"));
    let solid = d.add(format!("IFCEXTRUDEDAREASOLID({prof},{a},{z},250.)"));
    let body = d.add(format!(
        "IFCSHAPEREPRESENTATION({ctx},'Body','SweptSolid',({solid}))"
    ));
    let shape = d.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
    let slab_pl = d.add(format!("IFCLOCALPLACEMENT({},{wcs})", storey_pls[0]));
    let slab = d.add(format!(
        "IFCSLAB('{}',$,'Suelo:Losa 250',$,'Suelo:Losa 250',{slab_pl},{shape},'200',.FLOOR.)",
        G[8]
    ));
    let l1 = d.add(format!("IFCMATERIALLAYER({tile},20.,$)"));
    let l2 = d.add(format!("IFCMATERIALLAYER({concrete},230.,$)"));
    let slab_set = d.add(format!("IFCMATERIALLAYERSET(({l1},{l2}),'Losa 250')"));
    let slab_usage = d.add(format!(
        "IFCMATERIALLAYERSETUSAGE({slab_set},.AXIS3.,.NEGATIVE.,0.)"
    ));
    d.add(format!(
        "IFCRELASSOCIATESMATERIAL('5aaaaaaaaaaaaaaaaaaaa0',$,$,$,({slab}),{slab_usage})"
    ));

    // A column: not imported yet.
    let column = d.add(format!("IFCCOLUMN('{}',$,'Pilar',$,$,{slab_pl},$,$)", G[9]));

    let mut contained: Vec<String> = walls.clone();
    contained.extend([door, slab, column]);
    d.add(format!(
        "IFCRELCONTAINEDINSPATIALSTRUCTURE('5aaaaaaaaaaaaaaaaaaaa1',$,$,$,({}),{})",
        contained.join(","),
        storeys[0]
    ));
    d.text("IFC2X3")
}

#[test]
fn reads_a_revit_like_file_in_millimetres() {
    let Imported { model, report } = import_ifc(&revit_like()).unwrap();
    assert_eq!(report.schema, "IFC2X3");
    assert_eq!(report.source, "Programa de prueba (Exportador de prueba)");
    assert_eq!(model.project_name, "Proyecto Revit");
    assert_eq!(model.project_guid.as_str(), G[0]);
    assert_eq!(model.building_guid.as_str(), G[2]);

    let levels = model.levels();
    assert_eq!(levels.len(), 2);
    assert_eq!(
        (levels[1].name.as_str(), levels[1].elevation),
        ("Nivel 2", 3.0)
    );
    assert_eq!(levels[0].guid.as_str(), G[3]);

    // Walls turned so their exterior (the right side in the file) is on
    // the left: the outline now runs clockwise. Layers from the exterior,
    // the membrane without thickness left out, the line in the middle.
    assert_eq!(report.walls, 4);
    let walls: Vec<_> = model.walls().collect();
    let south = walls[0];
    assert_eq!(south.guid.as_str(), "3aaaaaaaaaaaaaaaaaaaa0");
    assert_eq!(south.start, Point2::new(10.0, 0.0));
    assert_eq!(south.end, Point2::new(0.0, 0.0));
    assert_eq!(south.reference, ReferenceLine::Center);
    assert_eq!((south.height, south.base_offset), (2.8, 0.0));
    let wt = model.wall_type(south.wall_type).unwrap();
    assert_eq!(wt.guid.as_str(), G[5]);
    assert_eq!(wt.name, "Muro básico: Fachada 195");
    assert_eq!(wt.function, WallFunction::Exterior);
    let names: Vec<_> = wt
        .layers
        .iter()
        .map(|l| model.material(l.material).unwrap().name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Enfoscado de mortero",
            "Ladrillo perforado",
            "Lana mineral",
            "Yeso"
        ]
    );
    assert!((wt.thickness() - 0.195).abs() < 1e-9);
    let render = model.material(wt.layers[0].material).unwrap();
    assert_eq!(render.cut_color, [230, 217, 204]);
    assert_eq!(render.category, "mortar");
    assert_eq!(wt.layers[2].function, LayerFunction::Insulation);
    assert_eq!(model.wall_types().count(), 1);

    // The door: measured from the new start (10, 0), hinges as UNE-EN
    // 12519 sees them, opening inwards, its frame centred.
    assert_eq!(report.doors, 1);
    let door = model.openings().next().unwrap();
    assert_eq!(door.guid.as_str(), G[6]);
    assert_eq!(door.wall, south.id);
    assert!((door.along - 7.55).abs() < 1e-9, "{}", door.along);
    assert_eq!(door.sill, 0.0);
    assert_eq!(door.opens_to, WallSide::Interior);
    assert_eq!(door.hand, Hand::Right);
    assert_eq!(door.setback, None);
    let dt = model.opening_type(door.opening_type).unwrap();
    assert_eq!(dt.guid.as_str(), G[7]);
    assert_eq!(dt.kind, OpeningKind::Door(DoorOperation::SingleSwing));
    assert_eq!((dt.width, dt.height), (0.9, 2.1));
    assert_eq!((dt.frame_width, dt.frame_depth), (0.045, 0.07));
    assert_eq!(
        model.material(dt.frame_material).unwrap().name,
        "Madera de roble"
    );

    // The slab with its layers from the top, top face at ±0.
    assert_eq!(report.slabs, 1);
    let slab = model.slabs().next().unwrap();
    assert_eq!(slab.guid.as_str(), G[8]);
    assert_eq!(slab.top_offset, 0.0);
    assert!((slab.area() - 80.0).abs() < 1e-9);
    let st = model.slab_type(slab.slab_type).unwrap();
    assert_eq!(st.name, "Losa 250");
    assert_eq!(st.function, SlabFunction::Floor);
    assert_eq!(
        model.material(st.layers[0].material).unwrap().name,
        "Baldosa cerámica"
    );

    // The column is reported, and so are the approximations.
    assert_eq!(report.skipped_count(), 1, "{report}");
    let text = report.to_string();
    assert!(text.contains("pilares"), "{text}");
    assert!(text.contains("milímetros"), "{text}");
    assert!(text.contains("espesor cero"), "{text}");

    // And the result exports as a valid model of our own.
    let again = export(&model);
    let back = import_ifc(&again).unwrap();
    assert!(differences(&model, &back.model).is_empty());
}

#[test]
fn reads_feet() {
    let text = "ISO-10303-21;HEADER;FILE_SCHEMA(('IFC4'));ENDSEC;DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#1);
#3=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#4=IFCCONVERSIONBASEDUNIT(#3,.LENGTHUNIT.,'FOOT',#2);
#5=IFCUNITASSIGNMENT((#4));
#6=IFCPROJECT('0aaaaaaaaaaaaaaaaaaaa0',$,'Feet',$,$,$,$,$,#5);
#7=IFCBUILDINGSTOREY('0aaaaaaaaaaaaaaaaaaaa1',$,'Level 2',$,$,$,$,$,.ELEMENT.,10.);
ENDSEC;END-ISO-10303-21;";
    let Imported { model, report } = import_ifc(text).unwrap();
    assert!((model.levels()[0].elevation - 3.048).abs() < 1e-9);
    assert!(report.to_string().contains("pies"), "{report}");
}

/// Windows with three and four leaves, each opening its own way, keep
/// every leaf through IFC: partitioning, one panel operation per leaf, and
/// back into the family's leaf settings.
#[test]
fn windows_with_several_leaves_keep_each_leaf() {
    let mut m = model();
    let p1 = m.levels()[1].id;
    let facade = m
        .wall_types()
        .find(|t| t.name.starts_with("Fachada"))
        .unwrap()
        .id;
    let wall = m
        .add_wall(NewWall {
            wall_type: facade,
            level: p1,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(0.0, 8.0),
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
    let family = m.family_by_name("Ventana practicable").unwrap().id;
    let base = m
        .opening_types()
        .find(|t| t.family == family)
        .unwrap()
        .values
        .clone();
    let mut make = |name: &str, width: f64, leaves: &[&str]| {
        let mut values = base.clone();
        values.insert("width".into(), archisim_core::Value::Number(width));
        values.insert(
            "leaves".into(),
            archisim_core::Value::Number(leaves.len() as f64),
        );
        for (i, op) in leaves.iter().enumerate() {
            values.insert(
                format!("leaf_{}", i + 1),
                archisim_core::Value::Text(op.to_string()),
            );
        }
        m.add_family_type(archisim_core::NewFamilyType {
            family,
            name: name.into(),
            mark: None,
            description: String::new(),
            values,
        })
        .unwrap()
    };
    let three = make("Ventana 3 hojas", 2.1, &["fija", "oscilobatiente", "fija"]);
    let four = make(
        "Ventana 4 hojas",
        2.6,
        &["practicable", "fija", "fija", "practicable"],
    );
    let top = make("Ventana proyectante", 0.8, &["proyectante"]);
    for (t, along) in [(three, 1.5), (four, 4.5), (top, 7.0)] {
        m.add_opening(NewOpening {
            opening_type: t,
            wall,
            along,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
    }
    let ifc = export(&m);
    if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("ventanas_hojas.ifc"), &ifc).unwrap();
    }
    assert!(ifc.contains(".TRIPLE_PANEL_VERTICAL."));
    assert!(ifc.contains("'Hoja 2',$,.TILTANDTURNLEFTHAND.,.MIDDLE."));
    assert!(ifc.contains("'Hoja 4',$,.SIDEHUNGRIGHTHAND.,.RIGHT."));
    assert!(ifc.contains(".TOPHUNG.,.MIDDLE."));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(report.warnings.is_empty(), "{report}");
    for (name, leaves) in [
        ("Ventana 3 hojas", vec!["fija", "oscilobatiente", "fija"]),
        (
            "Ventana 4 hojas",
            vec!["practicable", "fija", "fija", "practicable"],
        ),
        ("Ventana proyectante", vec!["proyectante"]),
    ] {
        let t = model.opening_types().find(|t| t.name == name).unwrap();
        assert_eq!(
            t.values["leaves"],
            archisim_core::Value::Number(leaves.len() as f64),
            "{name}"
        );
        for (i, op) in leaves.iter().enumerate() {
            assert_eq!(
                t.values[&format!("leaf_{}", i + 1)],
                archisim_core::Value::Text(op.to_string()),
                "{name} leaf {}",
                i + 1
            );
        }
    }
}

/// Curved walls go out with a circular axis and exact arcs in their body,
/// and come back the same; a partition that stops at a curved face too.
#[test]
fn curved_walls_round_trip() {
    let mut m = Model::new("Curvos");
    let pb = m.add_level("Planta baja", 0.0);
    let lib = add_library(&mut m).unwrap();
    let mut wall = |t, a: (f64, f64), b: (f64, f64), bulge: f64, reference| {
        m.add_wall(NewWall {
            wall_type: t,
            level: pb,
            start: Point2::new(a.0, a.1),
            end: Point2::new(b.0, b.1),
            reference,
            height: 3.0,
            base_offset: 0.0,
            bulge,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap()
    };
    // A round room of 3 m radius, clockwise, and a bay: straight walls with
    // a quarter circle between them.
    let ext = ReferenceLine::ExteriorFace;
    wall(lib.facade, (3.0, 0.0), (-3.0, 0.0), -1.0, ext);
    wall(lib.facade, (-3.0, 0.0), (3.0, 0.0), -1.0, ext);
    wall(
        lib.partition,
        (-2.9, 0.5),
        (2.9, 0.5),
        0.0,
        ReferenceLine::Center,
    );
    let q = -(std::f64::consts::FRAC_PI_8).tan();
    wall(lib.facade, (8.0, 0.0), (8.0, 4.0), 0.0, ext);
    wall(lib.facade, (8.0, 4.0), (11.0, 7.0), q, ext);
    wall(lib.facade, (11.0, 7.0), (14.0, 7.0), 0.0, ext);
    let ifc = export(&m);
    if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("curvos.ifc"), &ifc).unwrap();
    }
    assert_eq!(ifc.matches("IFCTRIMMEDCURVE(").count(), 3);
    assert_eq!(ifc.matches("IFCARCINDEX(").count(), 6);
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(report.skipped.is_empty(), "{report}");
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// A family of the user's own (not a system one), with an instance
/// parameter, a type with a description and a hand-set mark: the IFC
/// carries the recipe, the values and the mark, and they come back the same.
/// The body written to IFC is the family's own pieces.
#[test]
fn any_family_survives_the_round_trip() {
    let mut m = model();
    let source = r#"family "Puerta de prueba"
  category door
  host wall
  version 3
  description "Puerta inventada para la prueba de ida y vuelta"

type_params
  section "Dimensiones"
    width  length = 0.82 range 0.40..3.00 label "Hueco de obra: ancho"
    height length = 2.03 range 1.00..3.00 label "Hueco de obra: alto"
  section "Hoja"
    leaf_depth length = 0.035 range 0.02..0.06 label "Grueso de hoja"
    glazed bool = yes label "Con mirilla"
    finish choice("lacada", "barnizada") = "lacada" label "Acabado"
  section "Materiales"
    frame material = "Madera de pino" label "Cerco"
    leaf material = "Madera de pino" label "Hoja"

instance_params
  kick_plate bool = no label "Zócalo protector"

body
  box (-width / 2, -0.035, 0) to (-width / 2 + 0.06, 0.035, height) material frame
  box (width / 2 - 0.06, -0.035, 0) to (width / 2, 0.035, height) material frame
  box (-width / 2 + 0.06, -0.035, height - 0.06) to (width / 2 - 0.06, 0.035, height) material frame
  box (-width / 2 + 0.06, -leaf_depth / 2, 0) to (width / 2 - 0.06, leaf_depth / 2, height - 0.06) material leaf
  if kick_plate
    box (-width / 2 + 0.06, leaf_depth / 2, 0) to (width / 2 - 0.06, leaf_depth / 2 + 0.002, 0.3) material frame

outputs
  opening_width = width
  opening_height = height
  frame_width = 0.06
  frame_depth = 0.07
  frame_material = frame
  panel_material = leaf
  operation = "single_swing"
"#;
    let family = m.add_family(source).unwrap();
    let mut values = m
        .family_defaults(family, archisim_core::ParamScope::Type)
        .unwrap();
    values.insert("width".into(), archisim_core::Value::Number(0.925));
    values.insert(
        "finish".into(),
        archisim_core::Value::Text("barnizada".into()),
    );
    values.insert("glazed".into(), archisim_core::Value::Bool(false));
    let t = m
        .add_family_type(archisim_core::NewFamilyType {
            family,
            name: "Puerta prueba 92,5".into(),
            mark: Some("PX-7".into()),
            description: "Aseos «planta baja»".into(),
            values,
        })
        .unwrap();
    let wall = m
        .walls()
        .find(|w| w.length() > 7.0 && m.openings_in_wall(w.id).count() == 0)
        .map(|w| w.id);
    let wall = wall.unwrap_or_else(|| {
        let level = m.levels()[1].id;
        let wt = m.wall_types().next().unwrap().id;
        m.add_wall(NewWall {
            wall_type: wt,
            level,
            start: Point2::new(20.0, 0.0),
            end: Point2::new(28.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap()
    });
    let o = m
        .add_opening(NewOpening {
            opening_type: t,
            wall,
            along: 2.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Right,
            setback: None,
        })
        .unwrap();
    let mut inst = m.opening(o).unwrap().values.clone();
    inst.insert("kick_plate".into(), archisim_core::Value::Bool(true));
    m.set_opening_values(o, inst).unwrap();

    let ifc = export(&m);
    assert!(ifc.contains("'ArchiSim_Family'"));
    assert!(
        ifc.contains("'PX-7','Puerta de prueba'"),
        "mark and family in the type"
    );
    assert!(ifc.contains("IFCPROPERTYSINGLEVALUE('kick_plate',$,IFCBOOLEAN(.T.),$)"));
    // The body: five boxes (lining, leaf and kick plate) for that door.
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(report.warnings.is_empty(), "{report}");
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    let back = model.opening_types().find(|x| x.mark == "PX-7").unwrap();
    assert_eq!(
        model.family(back.family).unwrap().name(),
        "Puerta de prueba"
    );
    assert_eq!(back.description, "Aseos «planta baja»");
    let placed = model
        .openings()
        .find(|x| x.opening_type == back.id)
        .unwrap();
    assert_eq!(
        placed.values["kick_plate"],
        archisim_core::Value::Bool(true)
    );
    if let Ok(dir) = std::env::var("ARCHISIM_TEST_OUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("familia_propia.ifc"), &ifc).unwrap();
    }
}

/// A refurbishment: existing and demolished elements keep their phase
/// through IFC (`Status` of the common property sets).
#[test]
fn phases_round_trip() {
    use archisim_core::Phase;
    let mut m = model();
    let walls: Vec<u64> = m.walls().map(|w| w.id.0).collect();
    m.set_phase(walls[0], Phase::Existing).unwrap();
    m.set_phase(walls[1], Phase::Demolished).unwrap();
    let slab = m.slabs().next().unwrap().id.0;
    m.set_phase(slab, Phase::Existing).unwrap();
    // A new door in the existing wall.
    let door = m.openings().find(|o| o.wall.0 == walls[0]).map(|o| o.id.0);
    if let Some(o) = door {
        m.set_phase(o, Phase::New).unwrap();
    }
    let ifc = export(&m);
    assert!(ifc.contains("'Status',$,IFCLABEL('DEMOLISH')"));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    assert!(report
        .warnings
        .keys()
        .any(|w| w.contains("fases de reforma")));
}

/// An inclined slab goes out as a sweep in tilted axes and comes back
/// inclined, with the same top face and layers.
#[test]
fn inclined_slabs_round_trip() {
    let mut m = model();
    let s = m.slabs().next().unwrap().clone();
    let lo = s.outline.iter().fold(f64::MAX, |a, p| a.min(p.x));
    let hi = s.outline.iter().fold(f64::MIN, |a, p| a.max(p.x));
    m.set_slab_slope(
        s.id,
        Some(archisim_core::SlabSlope {
            from: Point2::new(lo, 0.0),
            to: Point2::new(hi, 0.0),
            slope: 0.08,
        }),
    )
    .unwrap();
    let ifc = export(&m);
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let back = model
        .slab(model.slabs().find(|t| t.guid == s.guid).unwrap().id)
        .unwrap();
    let slope = back.slope.expect("inclined");
    assert!((slope.slope - 0.08).abs() < 1e-6, "{slope:?}");
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// A gable roof with the walls under it trimmed: the IFC clips their
/// bodies under its planes (validated in CI on the demo), and ArchiSim
/// reads the trims back.
#[test]
fn trimmed_walls_round_trip() {
    let mut m = model();
    let walls: Vec<archisim_core::WallId> = m.walls().map(|w| w.id).collect();
    let level = m.wall(walls[0]).unwrap().level;
    let st = m
        .slab_types()
        .find(|t| t.function == archisim_core::SlabFunction::Roof)
        .unwrap()
        .id;
    let r = m
        .add_roof(archisim_core::NewRoof {
            roof_type: st,
            level,
            outline: vec![
                Point2::new(-1.0, -1.0),
                Point2::new(12.0, -1.0),
                Point2::new(12.0, 10.0),
                Point2::new(-1.0, 10.0),
            ],
            slopes: vec![Some(0.3), None, Some(0.3), None],
            base_offset: 2.6,
            overhang: 0.3,
        })
        .unwrap();
    for w in &walls[..2] {
        m.set_wall_roof(*w, Some(r)).unwrap();
    }
    let ifc = export(&m);
    assert!(ifc.contains("IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE."));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(
        !report.warnings.keys().any(|w| w.contains("recortados")),
        "{report}"
    );
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// A material's own intersection priority goes to the IFC with its hatch
/// and comes back.
#[test]
fn material_priority_round_trip() {
    let mut m = model();
    let mat = m.materials().next().unwrap().clone();
    m.set_material_priority(mat.id, Some(950)).unwrap();
    let ifc = export(&m);
    assert!(ifc.contains("'Priority',$,IFCINTEGER(950)"));
    let Imported { model, .. } = import_ifc(&ifc).unwrap();
    let back = model.materials().find(|x| x.name == mat.name).unwrap();
    assert_eq!(back.priority, Some(950));
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// A glass railing: its panels go to the IFC as a second face set with
/// the glass's colour, and it comes back with its glass.
#[test]
fn glass_railing_round_trip() {
    let mut m = model();
    let level = m.levels()[0].id;
    let mats: Vec<_> = m.materials().map(|x| x.id).collect();
    m.add_railing(archisim_core::NewRailing {
        level,
        path: vec![Point2::new(0.5, 0.5), Point2::new(3.5, 0.5)],
        base_offset: 0.0,
        height: 1.1,
        post_spacing: 1.5,
        material: mats[0],
        glass: Some(mats[1]),
        railing_type: None,
    })
    .unwrap();
    let ifc = export(&m);
    assert!(ifc.contains("'Glass'"));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert!(model.railings().next().unwrap().glass.is_some());
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// Curtain wall types (R11) go out as `IfcCurtainWallType`s with their
/// grid, frames, panels and extras, used or not, and come back the same,
/// with their GlobalIds; their walls keep their type.
#[test]
fn curtain_wall_types_round_trip() {
    use archisim_core::{
        CurtainCaps, CurtainDivision, CurtainJoint, CurtainRest, NewCurtainWall, WallTop,
    };
    let mut m = Model::new("Muros cortina");
    let pb = m.add_level("PB", 0.0);
    let p1 = m.add_level("P1", 3.2);
    let plain = archisim_library::add_curtain_types(&mut m).unwrap();
    let spandrel = m
        .curtain_types()
        .find(|t| t.name == "Fachada con antepecho opaco")
        .unwrap()
        .id;
    let mut d = m.curtain_type(spandrel).unwrap().design.clone();
    d.columns = CurtainDivision {
        fixed: vec![0.9, 0.6],
        rest: CurtainRest::Count(3),
    };
    d.joint = CurtainJoint::Transoms;
    d.caps = Some(CurtainCaps {
        width: 0.07,
        depth: 0.03,
        material: d.mullion.material,
    });
    let name = m.curtain_type(spandrel).unwrap().name.clone();
    m.edit_curtain_type(spandrel, name, d).unwrap();
    let louvres = m
        .curtain_types()
        .find(|t| t.design.louvres.is_some())
        .unwrap()
        .id;
    for (t, y) in [(plain, 0.0), (spandrel, 5.0), (louvres, 10.0)] {
        m.add_curtain_wall(NewCurtainWall {
            curtain_type: t,
            level: pb,
            start: Point2::new(0.0, y),
            end: Point2::new(6.0, y),
            base_offset: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: 0.0,
            },
            height: 0.0,
        })
        .unwrap();
    }
    let ifc = export(&m);
    assert_eq!(ifc.matches("IFCCURTAINWALLTYPE(").count(), 5);
    assert!(ifc.contains("IFCSHADINGDEVICE("));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    assert_eq!(report.curtain_walls, 3);
    for t in m.curtain_types() {
        assert!(
            model.curtain_types().any(|u| u.guid == t.guid),
            "{}",
            t.name
        );
    }
    assert!(differences(&model, &import_ifc(&export(&model)).unwrap().model).is_empty());
}

#[test]
fn rooms_round_trip_as_spaces() {
    let mut original = model();
    let level = original.levels()[0].id;
    let outline = vec![
        Point2::new(0.5, 0.5),
        Point2::new(4.5, 0.5),
        Point2::new(4.5, 3.5),
        Point2::new(0.5, 3.5),
    ];
    let id = original
        .add_room(archisim_core::NewRoom {
            level,
            number: "0.01".into(),
            name: "Salón".into(),
            outline,
            holes: Vec::new(),
            label: Point2::new(2.0, 2.0),
            bounded: false,
        })
        .unwrap();
    let mut r = original.room(id).unwrap().clone();
    r.height = Some(2.6);
    r.finishes.floor = "Tarima de roble".into();
    r.usage = "Estancia".into();
    original.set_room(r.clone()).unwrap();
    let Imported { model, report } = import_ifc(&export(&original)).unwrap();
    assert_eq!(report.rooms, 1, "{report}");
    let back = model.rooms().next().unwrap();
    assert_eq!(
        (back.number.as_str(), back.name.as_str()),
        ("0.01", "Salón")
    );
    assert_eq!(back.guid, r.guid);
    assert!((back.area() - 12.0).abs() < 1e-6);
    assert_eq!(back.height, Some(2.6));
    assert_eq!(back.finishes.floor, "Tarima de roble");
    assert_eq!(back.usage, "Estancia");
}

/// IFC 4.3: the project placed on the map (`IfcMapConversion` to UTM, the
/// site's latitude and longitude) comes back; IFC4 writes the same map
/// conversion without the 4.3 attributes.
#[test]
fn georeference_round_trip_and_ifc4() {
    let mut m = model();
    m.set_true_north(0.2).unwrap();
    m.set_georeference(Some(archisim_core::Georeference {
        crs: "EPSG:25829".into(),
        eastings: 680_123.25,
        northings: 4_119_456.5,
        height: 3.5,
    }))
    .unwrap();
    let ifc = export(&m);
    assert!(ifc.contains("FILE_SCHEMA(('IFC4X3_ADD2'))"));
    assert!(ifc.contains("IFCPROJECTEDCRS('EPSG:25829',$,'ETRS89',$,'UTM','29N',"));
    assert!(ifc.contains(",680123.25,4119456.5,3.5,"));
    // Latitude 37° N, longitude 6° W.
    assert!(ifc.contains(",.ELEMENT.,(37,"), "{ifc}");
    let Imported { model, .. } = import_ifc(&ifc).unwrap();
    assert_eq!(model.georeference(), m.georeference());
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    // IFC4.
    let ifc4 = export_ifc(
        &m,
        &ExportOptions {
            timestamp: Some("2026-10-17T12:00:00".into()),
            schema: crate::Schema::Ifc4,
            ..Default::default()
        },
    );
    assert!(ifc4.contains("FILE_SCHEMA(('IFC4'))"));
    let conv = ifc4
        .lines()
        .find(|l| l.contains("IFCMAPCONVERSION("))
        .unwrap();
    assert!(conv.ends_with(",1.);"), "{conv}");
    let Imported { model, .. } = import_ifc(&ifc4).unwrap();
    assert_eq!(model.georeference(), m.georeference());
}

/// Parameters of the project go to IFC in their property sets, with their
/// definitions on the project, and come back the same: of the element, of
/// the type, of every kind.
#[test]
fn project_parameters_survive_the_round_trip() {
    use archisim_core::{NewUserParameter, ParamCategory, UserKind, Value};
    let mut m = model();
    let fire = m
        .add_user_parameter(NewUserParameter {
            name: "Resistencia al fuego".into(),
            kind: UserKind::Choice(vec!["—".into(), "EI2 30-C5".into(), "EI2 60-C5".into()]),
            group: "AD5_Datos".into(),
            categories: vec![ParamCategory::Doors],
            by_type: true,
            default: Value::Text("—".into()),
            description: "DB SI".into(),
        })
        .unwrap();
    let finish = m
        .add_user_parameter(NewUserParameter {
            name: "Acabado interior".into(),
            kind: UserKind::Text,
            group: "AD5_Acabados".into(),
            categories: vec![ParamCategory::Walls],
            by_type: false,
            default: Value::Text(String::new()),
            description: String::new(),
        })
        .unwrap();
    let height = m
        .add_user_parameter(NewUserParameter {
            name: "Altura libre".into(),
            kind: UserKind::Length,
            group: "AD5_Datos".into(),
            categories: vec![ParamCategory::Walls, ParamCategory::Doors],
            by_type: false,
            default: Value::Number(0.0),
            description: String::new(),
        })
        .unwrap();
    let door = m
        .openings()
        .find(|o| m.opening_type(o.opening_type).unwrap().is_door())
        .unwrap()
        .clone();
    m.set_parameter_value(door.opening_type.0, fire, Value::Text("EI2 60-C5".into()))
        .unwrap();
    m.set_parameter_value(door.id.0, height, Value::Number(2.05))
        .unwrap();
    let wall = m.walls().next().unwrap().id;
    m.set_parameter_value(
        wall.0,
        finish,
        Value::Text("Pintura plástica \"blanca\"".into()),
    )
    .unwrap();
    let ifc = export(&m);
    assert!(ifc.contains("'AD5_Datos'"));
    assert!(ifc.contains("'ArchiSim_Parameters'"));
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    assert_eq!(report.parameters, 3);
    // The type's value reaches every door of the type.
    let d2 = model.openings().find(|o| o.guid == door.guid).unwrap().id;
    let p = model
        .user_parameters()
        .find(|p| p.name == "Resistencia al fuego")
        .unwrap();
    assert_eq!(
        model.element_parameter(d2.0, p.id),
        Some(Value::Text("EI2 60-C5".into()))
    );
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("parametros.ifc"), &ifc).unwrap();
    }
}

/// Dormers, roof windows, tiles and gutters (docs/MAQUETA.md §5) go out
/// as their own pieces and come back with their roof, not as walls, slabs,
/// windows or pipes of their own.
#[test]
fn dormers_tiles_and_gutters_round_trip() {
    let mut m = Model::new("Buhardillas");
    let level = m.add_level("Planta baja", 0.0);
    let lib = add_library(&mut m).unwrap();
    let r = m
        .add_roof(archisim_core::NewRoof {
            roof_type: lib.pitched_roof,
            level,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 8.0),
                Point2::new(0.0, 8.0),
            ],
            slopes: vec![Some(0.55), None, Some(0.55), None],
            base_offset: 3.0,
            overhang: 0.4,
        })
        .unwrap();
    m.add_dormer(r, archisim_core::DormerKind::Gable, Point2::new(3.0, 0.5))
        .unwrap();
    m.add_dormer(r, archisim_core::DormerKind::Shed, Point2::new(3.5, 7.5))
        .unwrap();
    m.add_dormer(
        r,
        archisim_core::DormerKind::Skylight,
        Point2::new(7.0, 1.5),
    )
    .unwrap();
    let gutter = m.default_gutter();
    m.set_roof_covering(
        r,
        archisim_core::RoofCovering {
            tiles: Some(archisim_core::TileKind::Arabic),
            tile_material: None,
            gutter,
        },
    )
    .unwrap();
    let ifc = export(&m);
    for class in [
        "IFCELEMENTASSEMBLY(",
        ".SKYLIGHT.",
        ".ROOFING.",
        ".GUTTER.",
        ".RIGIDSEGMENT.",
    ] {
        assert!(ifc.contains(class), "{class}");
    }
    let Imported { model, report } = import_ifc(&ifc).unwrap();
    assert_eq!(model.walls().count(), 0, "{report}");
    assert_eq!(model.slabs().count(), 0, "{report}");
    assert_eq!(model.runs().count(), 0, "{report}");
    let diffs = differences(&m, &model);
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
}

/// Revit's «Envolver» of a wall type, which IFC cannot say, goes in
/// ArchiSim's own set on the type and comes back.
#[test]
fn wall_type_wrapping_round_trip() {
    use archisim_core::Wrap;
    let mut m = model();
    let t = m.walls().next().unwrap().wall_type;
    m.set_wall_type_wrap(t, Wrap::Both, Wrap::Interior).unwrap();
    let ifc = export(&m);
    assert!(ifc.contains("'WrapEnds',$,IFCLABEL('both')"));
    let Imported { model, .. } = import_ifc(&ifc).unwrap();
    let name = &m.wall_type(t).unwrap().name;
    let back = model.wall_types().find(|x| &x.name == name).unwrap();
    assert_eq!(
        (back.wrap_ends, back.wrap_inserts),
        (Wrap::Both, Wrap::Interior)
    );
}
