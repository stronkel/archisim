//! A whole apartment building, modelled as a BIM office would: the block
//! of flats in Florence Street that Antonio modelled in his BIM manager
//! master, rebuilt with every tool of ArchiSim to see how far it goes and
//! what is missing ("archisim edificio").
//!
//! Five storeys over a brick podium on a plot of 38,5 × 12 m between two
//! party walls, the street to the south. Ground floor: two tenancies on the
//! street, the entrance lane with herringbone pavers, toilets, the lift,
//! the fernery under the lightwell with the stair, bicycle parking and
//! storage cages. Levels 1 to 4: six one-bedroom flats round a concrete
//! core (stair, lift and lightwell), with full-height black steel glazing
//! and balconies with planters at both ends, exposed concrete ceilings,
//! timber floors and exposed ducts. The roof: a garden on the street side
//! and a glazed winter deck with the laundry on the lane side.
//!
//! Plan axes: x along the plot from the street (on the plan's left) to the
//! back lane, y across it from the party wall at the plan's bottom; true
//! north is +x, so the street front is the south elevation. In the code,
//! "west", "north"… are the plan's left, top…, as on the drawings.
//! Heights: ground floor 3,40, the others 3,20.

// Floors are numbered as on the drawings (level k of `lv`).
#![allow(clippy::needless_range_loop)]

use std::f64::consts::{FRAC_PI_2, PI};
use std::path::Path;

use archisim_core::{
    CurtainDesign, CurtainProfile, Document, Hand, LayerFunction, LevelId, Model, ModelError,
    NewCurtainWall, NewOpening, NewSlab, NewWall, ObjectTypeId, OpeningTypeId, Point2,
    ReferenceLine, SlabFunction, SlabTypeId, WallFunction, WallId, WallSide, WallTop, WallTypeId,
};

type R<T = ()> = Result<T, ModelError>;

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

/// Storeys: name and elevation of the top of the structural floor.
const LEVELS: [(&str, f64); 7] = [
    ("Planta baja", 0.0),
    ("Planta 1", 3.40),
    ("Planta 2", 6.60),
    ("Planta 3", 9.80),
    ("Planta 4", 13.00),
    ("Cubierta", 16.20),
    ("Pabellón", 19.40),
];

/// The plot: from the street (x = 0) to the back lane, between the party
/// walls (y = 0 and y = 12).
const PLOT: (f64, f64) = (38.5, 12.0);
/// The glazing of the flats, west and east, and the edges of the balconies.
const WEST_GLASS: f64 = 3.32;
const EAST_GLASS: f64 = 35.07;
const WEST_EDGE: f64 = 1.57;
const EAST_EDGE: f64 = 37.06;
/// Inner faces of the party walls (25 cm of concrete).
const SOUTH_IN: f64 = 0.25;
const NORTH_IN: f64 = 11.75;

/// The types of the building, made for it or taken from the library.
struct Types {
    party: WallTypeId,
    core: WallTypeId,
    partition: WallTypeId,
    brick: WallTypeId,
    ground: SlabTypeId,
    floor: SlabTypeId,
    roof: SlabTypeId,
    deck_roof: SlabTypeId,
    light_roof: SlabTypeId,
    glazing: archisim_core::CurtainTypeId,
    flat_door: OpeningTypeId,
    inner_door: OpeningTypeId,
    bath_door: OpeningTypeId,
    void: OpeningTypeId,
    window: OpeningTypeId,
    small_window: OpeningTypeId,
    lift_door: OpeningTypeId,
    shopfront: OpeningTypeId,
    entrance: OpeningTypeId,
    garage: OpeningTypeId,
    balcony_door: OpeningTypeId,
    service_door: OpeningTypeId,
    column: archisim_core::MemberTypeId,
    beam: archisim_core::MemberTypeId,
    railing: Option<ObjectTypeId>,
    mesh: Option<ObjectTypeId>,
    steel: archisim_core::MaterialId,
    concrete: archisim_core::MaterialId,
}

pub fn build() -> Result<Document, ModelError> {
    let mut doc = Document::new(Model::new("Viviendas en Florence Street"));
    // `ARCHISIM_TIEMPOS=1` says how long each step takes.
    let loud = std::env::var_os("ARCHISIM_TIEMPOS").is_some();
    let step = |doc: &mut Document, name: &str, f: &mut dyn FnMut(&mut Model) -> R| -> R {
        let t = std::time::Instant::now();
        doc.transact(name, |m| f(m))?;
        if loud {
            eprintln!(
                "  {name:<40} {:>8.1} ms",
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        Ok(())
    };
    let mut lv = Vec::new();
    let mut ty = None;
    step(&mut doc, "Plantas y tipos", &mut |m| {
        lv = LEVELS.iter().map(|(n, z)| m.add_level(*n, *z)).collect();
        ty = Some(types(m)?);
        Ok(())
    })?;
    let t = ty.ok_or(ModelError::EmptyType)?;
    let lv = lv;
    step(&mut doc, "Ejes", &mut |m| grids(m, lv[0]))?;
    step(&mut doc, "Planta baja", &mut |m| ground_floor(m, &t, &lv))?;
    for k in 1..=4 {
        let name = format!("Planta {k}: viviendas");
        step(&mut doc, &name, &mut |m| typical_floor(m, &t, &lv, k))?;
    }
    step(&mut doc, "Forjados", &mut |m| slabs(m, &t, &lv))?;
    step(&mut doc, "Escalera y núcleo", &mut |m| core(m, &t, &lv))?;
    step(&mut doc, "Cubierta", &mut |m| roof(m, &t, &lv))?;
    step(&mut doc, "Estructura y cimentación", &mut |m| {
        structure(m, &t, &lv)
    })?;
    step(&mut doc, "Barandillas", &mut |m| railings(m, &t, &lv))?;
    step(&mut doc, "Mobiliario de las viviendas", &mut |m| {
        for k in 1..=4 {
            flats_furniture(m, lv[k], k)?;
        }
        Ok(())
    })?;
    step(&mut doc, "Planta baja: equipamiento", &mut |m| {
        ground_furniture(m, lv[0])
    })?;
    step(&mut doc, "Cubierta: jardín y pabellón", &mut |m| {
        roof_furniture(m, &lv)
    })?;
    step(&mut doc, "Instalaciones", &mut |m| services(m, &lv))?;
    step(&mut doc, "Locales", &mut |m| rooms(m, &lv))?;
    step(&mut doc, "Entorno", &mut |m| site(m, lv[0]))?;
    step(&mut doc, "Documentación", &mut |m| documentation(m, &lv))?;
    Ok(doc)
}

// ---------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------

fn material(m: &mut Model, name: &str, category: &str, rgb: [u8; 3]) -> archisim_core::MaterialId {
    if let Some(x) = m.materials().find(|x| x.name == name) {
        return x.id;
    }
    if let Some(id) = archisim_library::palette_material(m, name) {
        return id;
    }
    let pattern = match category {
        "concrete" => archisim_core::CutPattern::Cross,
        "brick" => archisim_core::CutPattern::Diagonal,
        "insulation" => archisim_core::CutPattern::Insulation,
        "wood" => archisim_core::CutPattern::Solid,
        _ => archisim_core::CutPattern::Dots,
    };
    m.add_material(name, category, rgb, pattern)
}

fn opening_type(m: &Model, name: &str) -> R<OpeningTypeId> {
    m.opening_types()
        .find(|t| t.name == name)
        .map(|t| t.id)
        .ok_or_else(|| ModelError::NotAnObject(name.to_string()))
}

/// A type of the library with other sizes, under its own name.
fn sized(
    m: &mut Model,
    base: &str,
    name: &str,
    width: f64,
    height: f64,
    about: &str,
) -> R<OpeningTypeId> {
    use archisim_core::{NewFamilyType, Value};
    let b = opening_type(m, base)?;
    let t = m.opening_type(b).cloned().ok_or(ModelError::EmptyType)?;
    let mut values = t.values.clone();
    values.insert("width".into(), Value::Number(width));
    values.insert("height".into(), Value::Number(height));
    m.add_family_type(NewFamilyType {
        family: t.family,
        name: name.into(),
        mark: None,
        description: about.into(),
        values,
    })
}

fn types(m: &mut Model) -> R<Types> {
    use archisim_core::Layer;
    let lib = archisim_library::add_library(m)?;
    let concrete = material(m, "Hormigón visto", "concrete", [182, 180, 174]);
    let screed = material(m, "Mortero de cemento", "mortar", [200, 196, 188]);
    let timber = material(m, "Tarima de madera recuperada", "wood", [150, 112, 76]);
    let xps = material(
        m,
        "Poliestireno extruido (XPS)",
        "insulation",
        [200, 220, 240],
    );
    let membrane = material(m, "Lámina impermeabilizante", "membrane", [40, 40, 44]);
    let steel = material(m, "Metal negro", "metal", [34, 34, 36]);
    let decking = material(m, "Tarima de exterior", "wood", [134, 104, 74]);
    let sheet = material(m, "Chapa grecada", "metal", [150, 152, 156]);
    let glass = material(m, "Vidrio doble", "glass", [190, 214, 222]);
    let l = |material, thickness, function| Layer {
        material,
        thickness,
        function,
    };
    let party = m.add_wall_type(
        "Muro medianero de hormigón visto 25",
        WallFunction::Exterior,
        vec![l(concrete, 0.25, LayerFunction::Structure)],
    )?;
    let core = m.add_wall_type(
        "Muro de hormigón visto 20",
        WallFunction::Interior,
        vec![l(concrete, 0.20, LayerFunction::Structure)],
    )?;
    let brick = m
        .wall_types()
        .find(|w| w.name == "Fachada ladrillo visto a soga 30")
        .map(|w| w.id)
        .unwrap_or(lib.facade);
    let floor = m.add_slab_type(
        "Losa HA 25 vista con tarima",
        SlabFunction::Floor,
        vec![
            l(timber, 0.02, LayerFunction::Finish),
            l(screed, 0.05, LayerFunction::Substrate),
            l(concrete, 0.25, LayerFunction::Structure),
        ],
    )?;
    let deck_roof = m.add_slab_type(
        "Cubierta transitable con tarima sobre losa HA 25",
        SlabFunction::Roof,
        vec![
            l(decking, 0.03, LayerFunction::Finish),
            l(xps, 0.08, LayerFunction::Insulation),
            l(membrane, 0.005, LayerFunction::Membrane),
            l(screed, 0.065, LayerFunction::Substrate),
            l(concrete, 0.25, LayerFunction::Structure),
        ],
    )?;
    let light_roof = m.add_slab_type(
        "Cubierta ligera de chapa sobre perfiles",
        SlabFunction::Roof,
        vec![
            l(sheet, 0.04, LayerFunction::Finish),
            l(xps, 0.10, LayerFunction::Insulation),
            l(steel, 0.02, LayerFunction::Structure),
        ],
    )?;
    // Black steel and glass: narrow frames, bays of a metre, a transom at
    // door height (the photos of the building).
    let frame = CurtainProfile {
        width: 0.05,
        depth: 0.10,
        material: steel,
    };
    let mut design = CurtainDesign::simple(1.0, 2.3, frame, 0.024, glass);
    design.rows = archisim_core::CurtainDivision {
        fixed: vec![2.3],
        rest: archisim_core::CurtainRest::MaxSpacing(1.2),
    };
    let glazing = m.add_curtain_type("Carpintería de acero negro y vidrio", design)?;
    let flat_door = sized(
        m,
        "Puerta de entrada 1 hoja 100×215",
        "Puerta de vivienda EI2 30 90×215",
        0.9,
        2.15,
        "Puerta de entrada a vivienda, resistente al fuego EI2 30-C5",
    )?;
    let lift_door = sized(
        m,
        "Puerta corredera 1 hoja 90×205",
        "Puerta de ascensor 90×210",
        0.9,
        2.1,
        "Puerta de piso del ascensor, telescópica",
    )?;
    let shopfront = sized(
        m,
        "Ventana fija 60×60",
        "Escaparate de acero 240×260",
        2.4,
        2.6,
        "Escaparate fijo de acero negro en el zócalo de ladrillo",
    )?;
    // The doors of the black steel glazing: glazed leaves.
    let balcony_door = sized(
        m,
        "Puerta de entrada 1 hoja 100×215",
        "Puerta balconera de acero y vidrio",
        0.9,
        2.2,
        "Hoja acristalada en el muro cortina de acero negro (toma el ancho y alto de su celda)",
    )?;
    if let Some(bt) = m.opening_type(balcony_door).cloned() {
        let mut values = bt.values.clone();
        values.insert(
            "leaf_design".into(),
            archisim_core::Value::Text("Vidriera".into()),
        );
        let _ = m.edit_family_type(balcony_door, bt.family, values);
    }
    let garage = sized(
        m,
        "Puerta de garaje seccional 250×220",
        "Puerta seccional del aparcamiento de bicicletas 760×240",
        7.6,
        2.4,
        "Acceso de bicicletas por el callejón trasero",
    )?;
    let mesh = m
        .object_types()
        .find(|t| t.name == "Barandilla de malla")
        .map(|t| t.id);
    let railing = m
        .object_types()
        .find(|t| t.name == "Barandilla · Industrial")
        .map(|t| t.id);
    Ok(Types {
        party,
        core,
        partition: lib.partition,
        brick,
        ground: lib.ground_slab,
        floor,
        roof: deck_roof,
        deck_roof,
        light_roof,
        glazing,
        flat_door,
        inner_door: lib.interior_door,
        bath_door: opening_type(m, "Puerta de paso 72,5 (baño)")?,
        void: lib.void,
        window: lib.window_one_leaf,
        small_window: lib.fixed_window,
        lift_door,
        shopfront,
        entrance: opening_type(m, "Puerta pivotante de entrada 120×240")?,
        garage,
        balcony_door,
        service_door: opening_type(m, "Puerta de paso 82,5")?,
        column: lib.column,
        beam: lib.beam,
        railing,
        mesh,
        steel,
        concrete,
    })
}

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

/// The top of a wall: the next level, under its slab.
fn up_to(level: LevelId) -> WallTop {
    WallTop::Level { level, offset: 0.0 }
}

fn wall(
    m: &mut Model,
    wall_type: WallTypeId,
    level: LevelId,
    top: WallTop,
    a: Point2,
    b: Point2,
    reference: ReferenceLine,
) -> R<WallId> {
    m.add_wall(NewWall {
        wall_type,
        level,
        start: a,
        end: b,
        reference,
        height: 3.0,
        base_offset: 0.0,
        bulge: 0.0,
        top,
    })
}

/// A wall on its axis.
fn axis(
    m: &mut Model,
    t: WallTypeId,
    level: LevelId,
    top: LevelId,
    a: Point2,
    b: Point2,
) -> R<WallId> {
    wall(m, t, level, up_to(top), a, b, ReferenceLine::Center)
}

/// Where a point falls along a wall, from its start.
fn along(m: &Model, w: WallId, at: Point2) -> R<f64> {
    let w = m.wall(w).ok_or(ModelError::InvalidOutline)?;
    let d = w.end - w.start;
    let len = d.length();
    let v = at - w.start;
    Ok((v.x * d.x + v.y * d.y) / len)
}

/// An opening centred on a point of a wall.
fn hole(
    m: &mut Model,
    opening_type: OpeningTypeId,
    w: WallId,
    at: Point2,
    sill: f64,
    hand: Hand,
) -> R {
    let along = along(m, w, at)?;
    m.add_opening(NewOpening {
        opening_type,
        wall: w,
        along,
        sill,
        opens_to: WallSide::Interior,
        hand,
        setback: None,
    })
    .inspect_err(|e| eprintln!("hueco en {at:?}: {e:?}"))?;
    Ok(())
}

fn object_type(m: &mut Model, name: &str) -> R<ObjectTypeId> {
    archisim_library::add_object_entry(m, name)
}

fn obj(m: &mut Model, level: LevelId, name: &str, x: f64, y: f64, rotation: f64) -> R {
    obj_at(m, level, name, x, y, rotation, 0.0)
}

fn obj_at(
    m: &mut Model,
    level: LevelId,
    name: &str,
    x: f64,
    y: f64,
    rotation: f64,
    base_offset: f64,
) -> R {
    let object_type = object_type(m, name)?;
    m.add_object(archisim_core::NewObject {
        level,
        object_type,
        position: p(x, y),
        rotation,
        base_offset,
    })?;
    Ok(())
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
    vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]
}

// ---------------------------------------------------------------------
// Grids
// ---------------------------------------------------------------------

fn grids(m: &mut Model, pb: LevelId) -> R {
    use archisim_core::NewGrid;
    for (i, x) in [0.0, WEST_GLASS, 15.7, 17.0, 21.7, 25.9, EAST_GLASS, PLOT.0]
        .into_iter()
        .enumerate()
    {
        m.add_grid(NewGrid {
            name: format!("{}", i + 1),
            start: p(x, -2.5),
            end: p(x, PLOT.1 + 2.5),
        })?;
    }
    for (name, y) in [("A", 0.125), ("B", 3.2), ("C", 7.6), ("D", 11.875)] {
        m.add_grid(NewGrid {
            name: name.into(),
            start: p(-2.5, y),
            end: p(PLOT.0 + 2.5, y),
        })?;
    }
    let _ = pb;
    Ok(())
}

// ---------------------------------------------------------------------
// Ground floor
// ---------------------------------------------------------------------

fn ground_floor(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    let (pb, p1) = (lv[0], lv[1]);
    // Party walls, outer faces on the boundaries.
    wall(
        m,
        t.party,
        pb,
        up_to(p1),
        p(PLOT.0, 0.0),
        p(0.0, 0.0),
        ReferenceLine::ExteriorFace,
    )?;
    wall(
        m,
        t.party,
        pb,
        up_to(p1),
        p(0.0, PLOT.1),
        p(PLOT.0, PLOT.1),
        ReferenceLine::ExteriorFace,
    )?;
    // The brick podium: the street front and the back lane.
    let west = wall(
        m,
        t.brick,
        pb,
        up_to(p1),
        p(0.0, SOUTH_IN),
        p(0.0, NORTH_IN),
        ReferenceLine::ExteriorFace,
    )?;
    let east = wall(
        m,
        t.brick,
        pb,
        up_to(p1),
        p(PLOT.0, NORTH_IN),
        p(PLOT.0, SOUTH_IN),
        ReferenceLine::ExteriorFace,
    )?;
    hole(m, t.shopfront, west, p(0.0, 2.2), 0.3, Hand::Left)?;
    hole(m, t.entrance, west, p(0.0, 5.8), 0.0, Hand::Right)?;
    hole(m, t.shopfront, west, p(0.0, 9.0), 0.3, Hand::Left)?;
    hole(m, t.garage, east, p(PLOT.0, 7.6), 0.0, Hand::Left)?;
    hole(m, t.service_door, east, p(PLOT.0, 2.0), 0.0, Hand::Right)?;

    let c = |m: &mut Model, a: Point2, b: Point2| axis(m, t.core, pb, p1, a, b);
    let w = |m: &mut Model, a: Point2, b: Point2| axis(m, t.partition, pb, p1, a, b);
    // The entrance lane between the tenancies, to the fernery and the
    // bicycles.
    let lane_s = c(m, p(0.25, 4.5), p(17.4, 4.5))?;
    let lane_n = c(m, p(0.25, 7.1), p(17.1, 7.1))?;
    hole(m, t.inner_door, lane_s, p(10.5, 4.5), 0.0, Hand::Right)?;
    hole(m, t.bath_door, lane_s, p(12.4, 4.5), 0.0, Hand::Left)?;
    hole(m, t.bath_door, lane_s, p(14.5, 4.5), 0.0, Hand::Left)?;
    hole(m, t.service_door, lane_s, p(16.6, 4.5), 0.0, Hand::Right)?;
    hole(m, t.inner_door, lane_n, p(10.6, 7.1), 0.0, Hand::Left)?;
    // Tenancy 1 (café, to the north) and its back room by the fernery.
    let back = w(m, p(12.9, 7.2), p(12.9, NORTH_IN))?;
    hole(m, t.inner_door, back, p(12.9, 10.9), 0.0, Hand::Left)?;
    // Glass to the fernery.
    m.add_curtain_wall(NewCurtainWall {
        curtain_type: t.glazing,
        level: pb,
        start: p(17.15, 7.15),
        end: p(17.15, NORTH_IN),
        base_offset: 0.0,
        top: WallTop::Level {
            level: p1,
            offset: -0.32,
        },
        height: 0.0,
    })?;
    // Tenancy 2 (to the south) and the toilets.
    c(m, p(11.7, SOUTH_IN), p(11.7, 4.4))?;
    w(m, p(13.1, SOUTH_IN), p(13.1, 4.45))?;
    w(m, p(13.15, 2.2), p(15.9, 2.2))?;
    w(m, p(15.9, 2.25), p(15.9, 4.45))?;
    // The lift shaft (the same up to the roof) and the stores.
    let lift_w = c(m, p(17.4, SOUTH_IN), p(17.4, 4.4))?;
    let _ = lift_w;
    c(m, p(19.6, SOUTH_IN), p(19.6, 3.2))?;
    let lift_n = c(m, p(17.5, 3.2), p(25.5, 3.2))?;
    hole(m, t.lift_door, lift_n, p(18.5, 3.2), 0.0, Hand::Left)?;
    hole(m, t.service_door, lift_n, p(22.5, 3.2), 0.0, Hand::Left)?;
    c(m, p(25.5, SOUTH_IN), p(25.5, 6.6))?;
    let store = c(m, p(25.6, 6.6), p(29.0, 6.6))?;
    hole(m, t.service_door, store, p(27.3, 6.6), 0.0, Hand::Right)?;
    // The fernery: open to the lightwell, the stair rising in it.
    c(m, p(22.2, 7.6), p(22.2, NORTH_IN))?;
    Ok(())
}

// ---------------------------------------------------------------------
// The flats
// ---------------------------------------------------------------------

fn typical_floor(m: &mut Model, t: &Types, lv: &[LevelId], k: usize) -> R {
    let (l, top) = (lv[k], lv[k + 1]);
    let (x0, x1) = if k == 1 {
        (0.0, PLOT.0)
    } else {
        (WEST_EDGE, EAST_EDGE)
    };
    wall(
        m,
        t.party,
        l,
        up_to(top),
        p(x1, 0.0),
        p(x0, 0.0),
        ReferenceLine::ExteriorFace,
    )?;
    wall(
        m,
        t.party,
        l,
        up_to(top),
        p(x0, PLOT.1),
        p(x1, PLOT.1),
        ReferenceLine::ExteriorFace,
    )?;
    let c = |m: &mut Model, a: Point2, b: Point2| axis(m, t.core, l, top, a, b);
    let w = |m: &mut Model, a: Point2, b: Point2| axis(m, t.partition, l, top, a, b);

    // Concrete blades between the flats, out to the edge of the balconies.
    let ab = c(m, p(WEST_EDGE, 7.9), p(17.0, 7.9))?;
    c(m, p(WEST_EDGE, 4.06), p(15.7, 4.06))?;
    let x157 = c(m, p(15.7, SOUTH_IN), p(15.7, 7.8))?;
    let x170 = c(m, p(17.0, 4.6), p(17.0, NORTH_IN))?;
    let x217 = c(m, p(21.7, 4.6), p(21.7, NORTH_IN))?;
    let y32 = c(m, p(15.8, 3.2), p(EAST_EDGE, 3.2))?;
    c(m, p(17.4, SOUTH_IN), p(17.4, 3.1))?;
    c(m, p(19.6, SOUTH_IN), p(19.6, 3.1))?;
    let y46 = c(m, p(21.8, 4.6), p(25.9, 4.6))?;
    let x259 = c(m, p(25.9, 3.3), p(25.9, 9.0))?;
    c(m, p(26.0, 7.6), p(EAST_EDGE, 7.6))?;

    // Entrances from the common corridor.
    hole(m, t.flat_door, ab, p(16.35, 7.9), 0.0, Hand::Right)?; // A
    hole(m, t.flat_door, x157, p(15.7, 4.75), 0.0, Hand::Left)?; // B
    hole(m, t.flat_door, x157, p(15.7, 3.45), 0.0, Hand::Right)?; // C
    hole(m, t.flat_door, y46, p(25.05, 4.6), 0.0, Hand::Left)?; // D
    hole(m, t.flat_door, x259, p(25.9, 3.9), 0.0, Hand::Right)?; // E
    hole(m, t.flat_door, y32, p(24.9, 3.2), 0.0, Hand::Left)?; // F
    hole(m, t.lift_door, y32, p(18.5, 3.2), 0.0, Hand::Left)?;
    hole(m, t.service_door, y32, p(16.55, 3.2), 0.0, Hand::Right)?;
    // Bedrooms looking onto the lightwell.
    hole(m, t.window, x170, p(17.0, 10.4), 0.9, Hand::Left)?;
    hole(m, t.window, x217, p(21.7, 10.4), 0.9, Hand::Right)?;
    // D's living room off its hall.
    hole(m, t.void, x259, p(25.9, 8.3), 0.0, Hand::Left)?;

    // A (north-west): living to the street, bath, bedroom by the lightwell.
    let a = w(m, p(11.5, 9.0), p(16.9, 9.0))?;
    w(m, p(11.5, 9.05), p(11.5, NORTH_IN))?;
    w(m, p(13.1, 9.05), p(13.1, NORTH_IN))?;
    hole(m, t.bath_door, a, p(12.3, 9.0), 0.0, Hand::Left)?;
    hole(m, t.inner_door, a, p(14.4, 9.0), 0.0, Hand::Right)?;
    // B (west, middle).
    let b = w(m, p(11.0, 5.4), p(15.6, 5.4))?;
    w(m, p(11.0, 5.45), p(11.0, 7.8))?;
    w(m, p(12.8, 5.45), p(12.8, 7.8))?;
    hole(m, t.bath_door, b, p(11.9, 5.4), 0.0, Hand::Left)?;
    hole(m, t.inner_door, b, p(14.2, 5.4), 0.0, Hand::Right)?;
    // C (south-west), its bath and bedroom on a light court.
    let cc = w(m, p(10.9, 2.9), p(15.6, 2.9))?;
    w(m, p(10.9, SOUTH_IN), p(10.9, 2.85))?;
    let court_c = w(m, p(13.9, SOUTH_IN), p(13.9, 2.85))?;
    let court_cb = w(m, p(13.95, 1.2), p(15.6, 1.2))?;
    hole(m, t.inner_door, cc, p(12.4, 2.9), 0.0, Hand::Left)?;
    hole(m, t.bath_door, cc, p(14.75, 2.9), 0.0, Hand::Right)?;
    if k > 1 {
        let narrow = opening_type(m, "Ventana oscilobatiente 1 hoja 80×120 con premarco")?;
        hole(m, narrow, court_c, p(13.9, 0.75), 0.9, Hand::Left)?;
        hole(m, t.small_window, court_cb, p(14.75, 1.2), 1.4, Hand::Left)?;
    }
    // D (north-east): hall, two bedrooms and bath, living to the back.
    let d = w(m, p(24.2, 4.7), p(24.2, 8.95))?;
    w(m, p(21.8, 7.6), p(24.15, 7.6))?;
    let d9 = w(m, p(21.8, 9.0), p(25.8, 9.0))?;
    hole(m, t.inner_door, d, p(24.2, 6.1), 0.0, Hand::Left)?;
    hole(m, t.bath_door, d, p(24.2, 8.3), 0.0, Hand::Right)?;
    hole(m, t.inner_door, d9, p(25.0, 9.0), 0.0, Hand::Left)?;
    // E (east, middle).
    let e = w(m, p(26.0, 5.0), p(30.6, 5.0))?;
    w(m, p(27.7, 5.05), p(27.7, 7.5))?;
    w(m, p(30.6, 5.05), p(30.6, 7.5))?;
    hole(m, t.bath_door, e, p(26.85, 5.0), 0.0, Hand::Left)?;
    hole(m, t.inner_door, e, p(29.1, 5.0), 0.0, Hand::Right)?;
    // F (south-east): passage, bedroom, light court, bath, store, living.
    let f_bed = w(m, p(23.6, SOUTH_IN), p(23.6, 3.1))?;
    let f = w(m, p(23.65, 2.0), p(29.45, 2.0))?;
    let f_court = w(m, p(25.4, SOUTH_IN), p(25.4, 1.95))?;
    w(m, p(27.4, SOUTH_IN), p(27.4, 1.95))?;
    let f_live = w(m, p(29.5, SOUTH_IN), p(29.5, 3.1))?;
    hole(m, t.inner_door, f_bed, p(23.6, 2.55), 0.0, Hand::Left)?;
    hole(m, t.bath_door, f, p(26.4, 2.0), 0.0, Hand::Right)?;
    hole(m, t.service_door, f, p(28.45, 2.0), 0.0, Hand::Left)?;
    hole(m, t.void, f_live, p(29.5, 2.55), 0.0, Hand::Left)?;
    if k > 1 {
        hole(m, t.window, f_bed, p(23.6, 1.1), 0.9, Hand::Right)?;
        hole(m, t.small_window, f_court, p(25.4, 1.1), 1.4, Hand::Left)?;
    }

    // Full-height glazing at both ends, between the blades.
    let soffit = m.slab_type(t.floor).map_or(0.32, |s| s.thickness());
    for (x, y0, y1) in [
        (WEST_GLASS, SOUTH_IN, 3.96),
        (WEST_GLASS, 4.16, 7.8),
        (WEST_GLASS, 8.0, NORTH_IN),
        (EAST_GLASS, SOUTH_IN, 3.1),
        (EAST_GLASS, 3.3, 7.5),
        (EAST_GLASS, 7.7, NORTH_IN),
    ] {
        // Outside on the left: the street's end runs northwards, the
        // lane's southwards.
        let (start, end) = if x < 10.0 {
            (p(x, y0), p(x, y1))
        } else {
            (p(x, y1), p(x, y0))
        };
        let cw = m.add_curtain_wall(NewCurtainWall {
            curtain_type: t.glazing,
            level: l,
            start,
            end,
            base_offset: 0.0,
            top: WallTop::Level {
                level: top,
                offset: -soffit,
            },
            height: 0.0,
        })?;
        // The door to the balcony (or to the podium's terrace), opening
        // in, in the second bay.
        m.add_curtain_door(cw, 1, t.balcony_door, Hand::Left, WallSide::Interior)?;
    }
    // On level 1 the podium's roof is a terrace: a brick parapet at the
    // street and at the back lane.
    if k == 1 {
        for (a, b) in [
            (p(0.0, SOUTH_IN), p(0.0, NORTH_IN)),
            (p(PLOT.0, NORTH_IN), p(PLOT.0, SOUTH_IN)),
        ] {
            m.add_wall(NewWall {
                wall_type: t.brick,
                level: l,
                start: a,
                end: b,
                reference: ReferenceLine::ExteriorFace,
                height: 1.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Slabs
// ---------------------------------------------------------------------

/// The stair and the lightwell, open through every floor.
fn core_hole() -> Vec<Point2> {
    vec![
        p(17.1, 4.7),
        p(20.4, 4.7),
        p(20.4, 7.2),
        p(21.6, 7.2),
        p(21.6, NORTH_IN),
        p(17.1, NORTH_IN),
    ]
}

fn slabs(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    m.add_slab(NewSlab {
        slab_type: t.ground,
        level: lv[0],
        outline: rect(0.0, 0.0, PLOT.0, PLOT.1),
        top_offset: 0.0,
    })?;
    for k in 1..=5 {
        let (x0, x1) = if k == 1 {
            (0.0, PLOT.0)
        } else {
            (WEST_EDGE, EAST_EDGE)
        };
        let slab_type = if k == 5 { t.roof } else { t.floor };
        let s = m.add_slab(NewSlab {
            slab_type,
            level: lv[k],
            outline: rect(x0, 0.0, x1, PLOT.1),
            top_offset: 0.0,
        })?;
        if k == 5 {
            // On the roof only the lightwell stays open; the stair has its
            // own head.
            m.add_slab_hole(s, rect(17.1, 7.2, 21.6, NORTH_IN))?;
            m.add_slab_hole(s, rect(17.1, 4.7, 20.4, 7.1))?;
        } else {
            m.add_slab_hole(s, core_hole())?;
        }
        if k > 1 {
            // The light courts of C and F, open to the sky.
            m.add_slab_hole(s, rect(13.95, SOUTH_IN, 15.6, 1.15))?;
            m.add_slab_hole(s, rect(23.65, SOUTH_IN, 25.35, 1.95))?;
        }
    }
    // The winter deck's roof and the head of the stair and the lift.
    m.add_slab(NewSlab {
        slab_type: t.light_roof,
        level: lv[6],
        outline: rect(17.0, 0.0, 35.6, PLOT.1),
        top_offset: 0.0,
    })?;
    let _ = t.deck_roof;
    Ok(())
}

// ---------------------------------------------------------------------
// Core: the stair, the lift's walls on the roof
// ---------------------------------------------------------------------

fn core(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    use archisim_core::{
        preset_stair, PresetSpec, StairDesign, StairPreset, StairStructure, StairUse, Vec2,
    };
    // A U stair in the open core, from the corridor round to it: up
    // westwards on the south side, back eastwards on the north side.
    for k in 0..5 {
        let height = LEVELS[k + 1].1 - LEVELS[k].1;
        let spec = PresetSpec {
            preset: StairPreset::UShape,
            winders: false,
            split: false,
            left: false,
            width: 1.15,
            going: 0.28,
            riser: 0.18,
            height,
            gap: 0.2,
            core: 0.12,
            edge: archisim_core::walking_edge(StairUse::General, 1.15),
        };
        let (path, form, sweep) = preset_stair(&spec, p(20.4, 5.3), Vec2::new(-1.0, 0.0))
            .ok_or(ModelError::InvalidShape("edificio: stair"))?;
        m.add_stair(archisim_core::NewStair {
            level: lv[k],
            path,
            width: 1.15,
            going: 0.28,
            base_offset: 0.0,
            top: up_to(lv[k + 1]),
            height: 0.0,
            waist: 0.15,
            material: t.concrete,
            usage: StairUse::General,
            design: StairDesign {
                form,
                sweep,
                structure: StairStructure::Slab,
                closed_risers: true,
                tread: 0.03,
                ..Default::default()
            },
        })?;
    }
    // The head of the stair and the lift's overrun on the roof.
    let (cu, pav) = (lv[5], lv[6]);
    let c = |m: &mut Model, a: Point2, b: Point2| axis(m, t.core, cu, pav, a, b);
    let west = c(m, p(17.0, 3.2), p(17.0, 7.2))?;
    c(m, p(17.1, 7.2), p(21.6, 7.2))?;
    let east = c(m, p(21.7, 0.0), p(21.7, 7.2))?;
    c(m, p(17.0, 0.0), p(17.0, 3.1))?;
    c(m, p(17.1, 3.2), p(19.6, 3.2))?;
    c(m, p(19.6, SOUTH_IN), p(19.6, 3.1))?;
    hole(m, t.flat_door, west, p(17.0, 4.2), 0.0, Hand::Left)?;
    hole(m, t.flat_door, east, p(21.7, 6.0), 0.0, Hand::Right)?;
    Ok(())
}

// ---------------------------------------------------------------------
// Roof: the winter deck
// ---------------------------------------------------------------------

fn roof(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    let (cu, pav) = (lv[5], lv[6]);
    // Party walls up as parapets along the garden and up to the deck's
    // roof along the pavilion.
    for (x0, x1, high) in [
        (WEST_EDGE, 17.0, false),
        (21.7, EAST_GLASS, true),
        (EAST_GLASS, EAST_EDGE, false),
    ] {
        for south in [true, false] {
            let (a, b) = if south {
                (p(x1, 0.0), p(x0, 0.0))
            } else {
                (p(x0, PLOT.1), p(x1, PLOT.1))
            };
            m.add_wall(NewWall {
                wall_type: t.party,
                level: cu,
                start: a,
                end: b,
                reference: ReferenceLine::ExteriorFace,
                height: 1.1,
                base_offset: 0.0,
                bulge: 0.0,
                top: if high {
                    up_to(pav)
                } else {
                    WallTop::Unconnected
                },
            })?;
        }
    }
    // The winter deck: glass to the east and to the lightwell.
    let soffit = m.slab_type(t.light_roof).map_or(0.16, |s| s.thickness());
    for (a, b) in [
        (p(EAST_GLASS, NORTH_IN), p(EAST_GLASS, SOUTH_IN)),
        (p(21.8, NORTH_IN), p(21.8, 7.3)),
    ] {
        let cw = m.add_curtain_wall(NewCurtainWall {
            curtain_type: t.glazing,
            level: cu,
            start: a,
            end: b,
            base_offset: 0.0,
            top: WallTop::Level {
                level: pav,
                offset: -soffit,
            },
            height: 0.0,
        })?;
        // Out to the deck by the lane, in the middle of the east glass.
        if a.x > 30.0 {
            m.add_curtain_door(cw, 5, t.balcony_door, Hand::Right, WallSide::Interior)?;
            m.add_curtain_door(cw, 6, t.balcony_door, Hand::Left, WallSide::Interior)?;
        }
    }
    // The laundry in its north-east corner.
    let w1 = axis(m, t.partition, cu, pav, p(31.6, NORTH_IN), p(31.6, 9.5))?;
    axis(m, t.partition, cu, pav, p(31.65, 9.5), p(34.95, 9.5))?;
    hole(m, t.inner_door, w1, p(31.6, 10.0), 0.0, Hand::Left)?;
    // The light courts end in planted skylights: a low upstand round them.
    for r in [
        rect(13.95, SOUTH_IN, 15.6, 1.15),
        rect(23.65, SOUTH_IN, 25.35, 1.95),
    ] {
        let _ = r;
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Structure: the transfer beams over the podium and the footings
// ---------------------------------------------------------------------

fn structure(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    use archisim_core::{FootingKind, NewBeam, NewColumn, NewFooting};
    let (pb, p1) = (lv[0], lv[1]);
    let depth = m.member_type(t.beam).map_or(0.5, |x| x.profile.size().1);
    let slab = m.slab_type(t.floor).map_or(0.32, |s| s.thickness());
    // Columns under the blades of the flats where the podium is open.
    let columns = [
        (WEST_GLASS, 4.06),
        (8.0, 4.06),
        (WEST_GLASS, 7.9),
        (8.0, 7.9),
        (29.5, 3.2),
        (33.0, 3.2),
        (EAST_GLASS, 3.2),
        (29.5, 7.6),
        (33.0, 7.6),
        (EAST_GLASS, 7.6),
    ];
    for (x, y) in columns {
        m.add_column(NewColumn {
            member_type: t.column,
            level: pb,
            position: p(x, y),
            rotation: 0.0,
            base_offset: 0.0,
            top: WallTop::Level {
                level: p1,
                offset: -slab - depth,
            },
            height: 0.0,
        })?;
    }
    for (a, b) in [
        (p(0.25, 4.06), p(11.6, 4.06)),
        (p(0.25, 7.9), p(12.8, 7.9)),
        (p(25.9, 3.2), p(PLOT.0 - 0.25, 3.2)),
        (p(26.0, 7.6), p(PLOT.0 - 0.25, 7.6)),
    ] {
        m.add_beam(NewBeam {
            member_type: t.beam,
            level: p1,
            start: a,
            end: b,
            top_offset: -slab,
        })?;
    }
    // Strip footings under the concrete walls of the ground floor, pads
    // under the columns.
    let walls: Vec<_> = m
        .walls_on_level(pb)
        .filter(|w| w.wall_type == t.party || w.wall_type == t.core)
        .map(|w| w.id)
        .collect();
    let top = -0.35;
    for w in walls {
        m.add_footing(NewFooting {
            level: pb,
            kind: FootingKind::Strip {
                start: p(0.0, 0.0),
                end: p(1.0, 0.0),
                wall: Some(w),
            },
            material: t.concrete,
            width: 0.8,
            depth: 0.6,
            top_offset: top,
        })?;
    }
    let cols: Vec<_> = m.columns_on_level(pb).map(|c| c.id).collect();
    for col in cols {
        m.add_footing(NewFooting {
            level: pb,
            kind: FootingKind::Pad {
                position: p(0.0, 0.0),
                length: 1.6,
                rotation: 0.0,
                column: Some(col),
            },
            material: t.concrete,
            width: 1.6,
            depth: 0.7,
            top_offset: top,
        })?;
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Railings and cages
// ---------------------------------------------------------------------

fn railing(
    m: &mut Model,
    t: &Types,
    level: LevelId,
    path: Vec<Point2>,
    height: f64,
    kind: Option<ObjectTypeId>,
) -> R {
    m.add_railing(archisim_core::NewRailing {
        level,
        path,
        base_offset: 0.0,
        height,
        post_spacing: 1.2,
        material: t.steel,
        glass: None,
        railing_type: kind,
    })?;
    Ok(())
}

fn railings(m: &mut Model, t: &Types, lv: &[LevelId]) -> R {
    // The balconies, between the blades: vertical black bars.
    for k in 2..=4 {
        for (x, y0, y1) in [
            (WEST_EDGE + 0.06, 0.3, 3.96),
            (WEST_EDGE + 0.06, 4.16, 7.8),
            (WEST_EDGE + 0.06, 8.0, 11.7),
            (EAST_EDGE - 0.06, 0.3, 3.1),
            (EAST_EDGE - 0.06, 3.3, 7.5),
            (EAST_EDGE - 0.06, 7.7, 11.7),
        ] {
            railing(m, t, lv[k], vec![p(x, y0), p(x, y1)], 1.1, t.railing)?;
        }
    }
    // Round the open core on every floor: at the corridor's edge and on
    // the roof round the lightwell.
    for k in 1..=4 {
        railing(
            m,
            t,
            lv[k],
            vec![p(17.15, 4.75), p(20.35, 4.75)],
            1.1,
            t.railing,
        )?;
    }
    railing(
        m,
        t,
        lv[5],
        vec![p(17.15, NORTH_IN), p(17.15, 7.3), p(21.55, 7.3)],
        1.1,
        t.railing,
    )?;
    // The roof's edges.
    railing(
        m,
        t,
        lv[5],
        vec![p(WEST_EDGE + 0.06, 0.3), p(WEST_EDGE + 0.06, 11.7)],
        1.1,
        t.railing,
    )?;
    railing(
        m,
        t,
        lv[5],
        vec![p(EAST_EDGE - 0.06, 0.3), p(EAST_EDGE - 0.06, 11.7)],
        1.1,
        t.railing,
    )?;
    // Round the light courts on the roof.
    for r in [rect(13.9, 0.3, 15.65, 1.2), rect(23.6, 0.3, 25.4, 2.0)] {
        railing(m, t, lv[5], vec![r[0], r[3], r[2], r[1]], 1.1, t.railing)?;
    }
    // The storage cages of the ground floor: chain-link mesh, 2,2 m high.
    let pb = lv[0];
    for i in 0..4 {
        let y = 0.3 + i as f64 * 1.55;
        railing(m, t, pb, vec![p(29.6, y), p(36.0, y)], 2.2, t.mesh)?;
    }
    for x in [29.6, 31.2, 32.8, 34.4, 36.0] {
        railing(m, t, pb, vec![p(x, 0.3), p(x, 4.95)], 2.2, t.mesh)?;
    }
    // The cages' fronts, open to the aisle.
    Ok(())
}

// ---------------------------------------------------------------------
// Furniture of the flats
// ---------------------------------------------------------------------

/// A kitchen of modules along a wall from `x0`: the fridge column, a
/// drawer unit, the sink, the dishwasher, the hob and a cupboard, with
/// wall units and the hood over the hob. `face` is the wall's face and
/// `north` says the wall is to the north (the kitchen facing south).
fn kitchen(m: &mut Model, level: LevelId, x0: f64, face: f64, north: bool) -> R {
    let rot = if north { PI } else { 0.0 };
    // Along the wall: in plan, a kitchen facing south runs westwards from
    // its first module seen from the room; here always eastwards.
    let mut x = x0;
    for (name, w) in [
        ("Columna frigorífico 60", 0.6),
        ("Bajo cajonera 60", 0.6),
        ("Bajo fregadero 80", 0.8),
        ("Bajo lavavajillas 60", 0.6),
        ("Bajo placa 90", 0.9),
        ("Bajo con dos puertas 80", 0.8),
    ] {
        obj(m, level, name, x + w / 2.0, face, rot)?;
        x += w;
    }
    // Wall units over the sink and the cupboard, the hood over the hob.
    obj(m, level, "Alto con puerta 40", x0 + 1.4, face, rot)?;
    obj(m, level, "Alto con puerta 40", x0 + 1.8, face, rot)?;
    obj(m, level, "Campana integrada 60", x0 + 3.65, face, rot)?;
    obj(m, level, "Alto con puerta 40", x0 + 4.3, face, rot)?;
    Ok(())
}

/// A living room: sofa against a wall, rug, armchair, coffee table, the
/// dining table with four chairs, a floor lamp and a plant.
#[allow(clippy::too_many_arguments)]
fn living(
    m: &mut Model,
    level: LevelId,
    sofa_x: f64,
    sofa_face: f64,
    sofa_north: bool,
    table: Point2,
    plant: Point2,
) -> R {
    let (rot, s) = if sofa_north { (PI, -1.0) } else { (0.0, 1.0) };
    obj(
        m,
        level,
        "Sofá acogedor de 3 plazas",
        sofa_x,
        sofa_face,
        rot,
    )?;
    obj(
        m,
        level,
        "Alfombra 200×140",
        sofa_x,
        sofa_face + s * 0.8,
        rot,
    )?;
    obj(
        m,
        level,
        "Mesa de centro",
        sofa_x,
        sofa_face + s * 1.25,
        rot,
    )?;
    obj(
        m,
        level,
        "Butaca de orejas",
        sofa_x + 1.6,
        sofa_face + s * 1.6,
        rot + PI * 0.75,
    )?;
    obj(
        m,
        level,
        "Lámpara de pie",
        sofa_x - 1.25,
        sofa_face + s * 0.1,
        rot,
    )?;
    obj(
        m,
        level,
        "Mesa de comedor 160×90",
        table.x,
        table.y - 0.45,
        0.0,
    )?;
    for (dx, dy, r) in [
        (-0.4, -0.55, 0.0),
        (0.4, -0.55, 0.0),
        (-0.4, 0.55, PI),
        (0.4, 0.55, PI),
    ] {
        obj(
            m,
            level,
            "Silla",
            table.x + dx,
            table.y + dy - 0.2 * (r == 0.0) as i32 as f64,
            r,
        )?;
    }
    obj(m, level, "Kentia", plant.x, plant.y, 0.0)?;
    Ok(())
}

/// A bedroom: double bed against `head` (a wall to the north, south, east
/// or west of the bed), two bedside tables and a wardrobe.
fn bedroom(
    m: &mut Model,
    level: LevelId,
    bed: Point2,
    rot: f64,
    wardrobe: Point2,
    wardrobe_rot: f64,
) -> R {
    obj(m, level, "Cama doble 150×200", bed.x, bed.y, rot)?;
    let side = Point2::new(rot.cos(), rot.sin());
    for s in [-1.0, 1.0] {
        let at = bed + (side * (s * 1.05));
        obj(m, level, "Mesita de noche", at.x, at.y, rot)?;
    }
    obj(
        m,
        level,
        "Armario empotrado 180",
        wardrobe.x,
        wardrobe.y,
        wardrobe_rot,
    )?;
    Ok(())
}

/// A bathroom against a wall at `y` (facing `north`): WC, basin and the
/// shower at one end.
fn bathroom(m: &mut Model, level: LevelId, x0: f64, x1: f64, face: f64, north: bool) -> R {
    let rot = if north { PI } else { 0.0 };
    obj(m, level, "Inodoro suspendido", x0 + 0.35, face, rot)?;
    obj(m, level, "Lavabo suspendido 60", x0 + 0.95, face, rot)?;
    obj(m, level, "Espejo redondo 60", x0 + 0.95, face, rot)?;
    let shower_x = (x1 - 0.45).max(x0 + 1.7);
    let _ = shower_x;
    Ok(())
}

fn flats_furniture(m: &mut Model, l: LevelId, k: usize) -> R {
    // A (north-west).
    kitchen(m, l, 6.5, NORTH_IN, true)?;
    living(m, l, 5.0, 8.0, false, p(7.6, 10.0), p(3.7, 8.4))?;
    bathroom(m, l, 11.6, 13.0, NORTH_IN, true)?;
    obj(m, l, "Ducha con mampara 120×80", 12.3, 9.1, 0.0)?;
    bedroom(m, l, p(15.0, NORTH_IN), PI, p(16.9, 10.4), FRAC_PI_2)?;
    // B (west, middle).
    kitchen(m, l, 6.0, 7.8, true)?;
    living(m, l, 4.9, 4.16, false, p(7.2, 6.0), p(3.7, 4.5))?;
    bathroom(m, l, 11.1, 12.7, 7.8, true)?;
    obj(m, l, "Ducha con mampara 120×80", 11.9, 5.5, 0.0)?;
    bedroom(m, l, p(14.2, 7.8), PI, p(15.6, 6.3), FRAC_PI_2)?;
    // C (south-west).
    kitchen(m, l, 6.0, SOUTH_IN, false)?;
    living(m, l, 4.9, 3.96, true, p(7.4, 2.4), p(3.7, 3.6))?;
    obj(m, l, "Inodoro suspendido", 15.2, 1.25, 0.0)?;
    obj(m, l, "Lavabo suspendido 60", 14.4, 1.25, 0.0)?;
    bedroom(m, l, p(12.4, SOUTH_IN), 0.0, p(11.0, 1.6), -FRAC_PI_2)?;
    // D (north-east).
    kitchen(m, l, 26.3, NORTH_IN, true)?;
    living(m, l, 33.0, 7.7, false, p(29.0, 9.9), p(34.7, 8.0))?;
    obj(m, l, "Inodoro suspendido", 22.2, 7.65, 0.0)?;
    obj(m, l, "Lavabo suspendido 60", 22.9, 7.65, 0.0)?;
    obj(m, l, "Ducha con mampara 120×80", 23.4, 8.95, PI)?;
    bedroom(m, l, p(23.0, NORTH_IN), PI, p(21.8, 10.2), -FRAC_PI_2)?;
    obj(m, l, "Cama individual 90×190", 22.4, 4.7, 0.0)?;
    obj(m, l, "Escritorio 140", 23.3, 7.55, PI)?;
    // E (east, middle).
    kitchen(m, l, 30.75, 7.5, true)?;
    living(m, l, 33.0, 3.3, false, p(32.6, 6.2), p(34.7, 3.6))?;
    bathroom(m, l, 26.1, 27.6, 7.5, true)?;
    obj(m, l, "Ducha con mampara 120×80", 26.85, 5.1, 0.0)?;
    bedroom(m, l, p(29.15, 7.5), PI, p(30.5, 6.2), FRAC_PI_2)?;
    // F (south-east).
    kitchen(m, l, 29.65, SOUTH_IN, false)?;
    obj(m, l, "Sofá nórdico de 2 plazas", 33.6, 3.1, PI)?;
    obj(m, l, "Mesa de comedor 160×90", 32.2, 1.6, FRAC_PI_2)?;
    obj(m, l, "Inodoro suspendido", 25.85, SOUTH_IN, 0.0)?;
    obj(m, l, "Lavabo suspendido 60", 26.5, SOUTH_IN, 0.0)?;
    obj(m, l, "Ducha con mampara 120×80", 26.75, 1.45, PI)?;
    bedroom(m, l, p(21.6, SOUTH_IN), 0.0, p(19.7, 1.6), -FRAC_PI_2)?;
    obj(m, l, "Lavadora 60", 28.0, SOUTH_IN, 0.0)?;
    // The balconies: planters along the rail and a table for two.
    if k >= 2 {
        for (x, rot) in [
            (WEST_EDGE + 0.15, -FRAC_PI_2),
            (EAST_EDGE - 0.15, FRAC_PI_2),
        ] {
            for (y0, y1) in [(0.25, 3.96), (4.16, 7.8), (8.0, 11.75)] {
                let mid = (y0 + y1) / 2.0;
                obj(m, l, "Jardinera de acero corten", x, mid - 0.9, rot)?;
                obj(
                    m,
                    l,
                    "Macetón con gramíneas",
                    x + (rot.signum() * -0.25),
                    mid + 1.1,
                    rot,
                )?;
                let tx = if x < 10.0 { 2.55 } else { 36.0 };
                obj(m, l, "Mesa de bistró con dos sillas", tx, mid + 0.2, rot)?;
            }
        }
    } else {
        // On level 1, the podium's roof: long planters at the parapets
        // and a table on each terrace.
        for (x, rot) in [(0.15, -FRAC_PI_2), (PLOT.0 - 0.15, FRAC_PI_2)] {
            for y in [1.1, 2.6, 5.1, 6.6, 9.1, 10.6] {
                obj(m, l, "Jardinera 120", x, y, rot)?;
            }
            let tx = if x < 10.0 { 1.9 } else { 36.6 };
            for y in [2.1, 6.0, 9.8] {
                obj(m, l, "Mesa de bistró con dos sillas", tx, y, rot)?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Ground floor: tenancies, bicycles, cages, the fernery
// ---------------------------------------------------------------------

fn ground_furniture(m: &mut Model, pb: LevelId) -> R {
    // The café: kitchen along the street front's north end, the bar and
    // tables.
    let mut x = 0.4;
    for (name, w) in [
        ("Columna frigorífico 60", 0.6),
        ("Bajo fregadero 80", 0.8),
        ("Bajo lavavajillas 60", 0.6),
        ("Bajo placa 90", 0.9),
        ("Bajo cajonera 60", 0.6),
    ] {
        obj(m, pb, name, x + w / 2.0, NORTH_IN, PI)?;
        x += w;
    }
    obj(m, pb, "Contrabarra con botellas", 4.6, NORTH_IN, PI)?;
    obj(m, pb, "Barra de bar", 2.6, 9.4, 0.0)?;
    for (x, yy) in [
        (6.5, 8.6),
        (8.5, 8.6),
        (6.5, 10.6),
        (8.5, 10.6),
        (10.6, 9.6),
    ] {
        obj(m, pb, "Mesa de bar", x, yy, 0.0)?;
        obj(m, pb, "Silla de bar", x - 0.5, yy, FRAC_PI_2)?;
        obj(m, pb, "Silla de bar", x + 0.5, yy, -FRAC_PI_2)?;
    }
    obj(m, pb, "Banco corrido", 14.9, NORTH_IN, PI)?;
    obj(m, pb, "Mesa de caballetes 200×100", 14.9, 10.4, 0.0)?;
    // The other tenancy: an office of the housing company.
    obj(m, pb, "Puestos de trabajo", 4.5, 1.6, 0.0)?;
    obj(m, pb, "Mesa de reuniones", 8.8, 2.3, 0.0)?;
    obj(m, pb, "Mostrador de recepción", 2.5, 4.4, PI)?;
    // Toilets: a wheelchair one and a plain one.
    obj(m, pb, "Inodoro suspendido", 15.4, 4.4, PI)?;
    obj(m, pb, "Lavabo suspendido 60", 14.2, 4.4, PI)?;
    obj(m, pb, "Barra de apoyo abatible 75", 15.4, 4.4, PI)?;
    obj(m, pb, "Inodoro suspendido", 12.4, SOUTH_IN, 0.0)?;
    obj(m, pb, "Lavabo suspendido 60", 12.4, 1.6, -FRAC_PI_2)?;
    // Bicycles: racks in rows with their bikes, cargo bikes by the door.
    for row in 0..3 {
        let yy = 8.2 + row as f64 * 1.6;
        for i in 0..7 {
            let x = 23.0 + i as f64 * 0.75;
            obj(m, pb, "Aparcabicis de rueda", x, yy, FRAC_PI_2)?;
            if (i + row) % 3 != 2 {
                obj(m, pb, "Bicicleta", x, yy + 0.7, FRAC_PI_2)?;
            }
        }
    }
    for i in 0..6 {
        obj(
            m,
            pb,
            "Aparcabicis de onda",
            31.0 + i as f64 * 1.0,
            10.9,
            0.0,
        )?;
    }
    obj(m, pb, "Bicicleta de carga", 33.0, 8.8, 0.0)?;
    obj(m, pb, "Bicicleta de carga", 33.0, 9.6, 0.0)?;
    // Bins in their store.
    for x in [20.4, 21.4, 22.4, 23.4] {
        obj(m, pb, "Contenedor de residuos", x, SOUTH_IN, 0.0)?;
    }
    // The fernery under the lightwell: ferns, palms and a tree fern in
    // the light that falls from the roof.
    for (name, x, yy) in [
        ("Helecho", 17.6, 8.0),
        ("Helecho", 18.4, 10.9),
        ("Helecho", 20.9, 11.2),
        ("Helecho", 21.6, 8.3),
        ("Kentia", 19.0, 9.0),
        ("Kentia", 21.2, 10.0),
        ("Ficus en maceta", 18.0, 11.2),
        ("Costilla de Adán", 20.2, 8.0),
    ] {
        obj(m, pb, name, x, yy, 0.0)?;
    }
    let palm = object_type(m, "Palmito")?;
    let fern = object_type(m, "Mirto")?;
    m.add_planting(archisim_core::NewPlanting {
        name: "Helechera".into(),
        level: pb,
        path: rect(17.4, 7.4, 21.9, NORTH_IN - 0.2),
        closed: true,
        smooth: false,
        arrangement: archisim_core::Arrangement::Area,
        count: 6,
        species: vec![palm, fern],
        size_variation: 0.3,
        shape_variation: 0.6,
        spread: 0.4,
        seed: 11,
        base_offset: 0.0,
    })?;
    Ok(())
}

// ---------------------------------------------------------------------
// Roof
// ---------------------------------------------------------------------

fn roof_furniture(m: &mut Model, lv: &[LevelId]) -> R {
    let cu = lv[5];
    // The garden to the west: raised beds of grasses and lavender round
    // a lawn, pergolas over the tables, the barbecue's bench.
    let grass = object_type(m, "Pennisetum")?;
    let lavender = object_type(m, "Lavanda")?;
    let rosemary = object_type(m, "Romero")?;
    for (i, (cx, cy)) in [(5.2, 9.3), (6.2, 2.6), (12.3, 6.0)]
        .into_iter()
        .enumerate()
    {
        // Hexagonal beds.
        let hex: Vec<Point2> = (0..6)
            .map(|j| {
                let a = j as f64 * PI / 3.0;
                p(cx + 1.4 * a.cos(), cy + 1.4 * a.sin())
            })
            .collect();
        m.add_planting(archisim_core::NewPlanting {
            name: format!("Arriate {}", i + 1),
            level: cu,
            path: hex,
            closed: true,
            smooth: false,
            arrangement: archisim_core::Arrangement::Area,
            count: 9,
            species: vec![grass, lavender, rosemary],
            size_variation: 0.3,
            shape_variation: 0.6,
            spread: 0.5,
            seed: 20 + i as u64,
            base_offset: 0.45,
        })?;
    }
    for (x, y, r) in [(9.0, 9.6, 0.0), (9.0, 2.4, 0.0)] {
        obj(m, cu, "Pérgola de lamas 4×3", x, y, r)?;
        obj(m, cu, "Mesa de teca para seis", x, y - 1.4, 0.0)?;
    }
    for y in [1.0, 5.0, 8.5] {
        obj(
            m,
            cu,
            "Jardinera de madera con flores",
            WEST_EDGE + 0.4,
            y,
            -FRAC_PI_2,
        )?;
    }
    obj(m, cu, "Banco de hormigón y madera", 14.9, 5.9, -FRAC_PI_2)?;
    obj(m, cu, "Sofá de exterior con mesa", 4.0, 5.9, -FRAC_PI_2)?;
    obj(m, cu, "Macetón con olivo", 15.8, 9.0, 0.0)?;
    obj(m, cu, "Macetón con olivo", 15.8, 2.5, 0.0)?;
    // The winter deck: long tables, the laundry's machines.
    for (x, y) in [(25.0, 2.2), (25.0, 9.8), (29.0, 2.2), (29.0, 9.8)] {
        obj(m, cu, "Mesa de caballetes 200×100", x, y, 0.0)?;
        obj(m, cu, "Banco corrido", x, y - 0.95, 0.0)?;
    }
    obj(m, cu, "Sofá rinconera 300×200", 33.6, 1.4, 0.0)?;
    obj(m, cu, "Kentia", 34.6, 8.8, 0.0)?;
    obj(m, cu, "Kentia", 22.4, 5.0, 0.0)?;
    for x in [32.3, 33.0, 33.7, 34.4] {
        obj(m, cu, "Lavadora 60", x, NORTH_IN, PI)?;
    }
    obj(m, cu, "Persona", 26.0, 6.0, 0.4)?;
    obj(m, cu, "Mujer", 26.6, 6.2, 3.0)?;
    obj(m, cu, "Persona", 8.0, 6.4, 1.0)?;
    Ok(())
}

// ---------------------------------------------------------------------
// Services
// ---------------------------------------------------------------------

fn services(m: &mut Model, lv: &[LevelId]) -> R {
    use archisim_core::{DeviceKind, NewDevice, NewRun, ServiceSystem};
    let device = |m: &mut Model,
                  level,
                  kind: DeviceKind,
                  name: &str,
                  at: Point2,
                  rot: f64|
     -> R<archisim_core::DeviceId> {
        let (width, depth, height, elevation) = kind.defaults();
        m.add_device(NewDevice {
            level,
            kind,
            name: name.into(),
            position: at,
            rotation: rot,
            elevation,
            width,
            depth,
            height,
        })
    };
    let pendant = object_type(m, "Lámpara colgante")?;
    let ceiling = object_type(m, "Plafón de techo")?;
    // The riser in its cupboard and the lift's switchboard: from the
    // ground floor to the roof.
    let riser = device(
        m,
        lv[0],
        DeviceKind::Riser,
        "Montante de instalaciones",
        p(16.5, 1.5),
        0.0,
    )?;
    let _ = riser;
    for k in 1..=4 {
        let l = lv[k];
        let soffit = 3.2 - 0.32;
        // Each flat: a pendant over the table, ceiling lights in the
        // other rooms, a board at the door, sockets and switches.
        for (x, y) in [
            (7.6, 10.0),
            (7.2, 6.0),
            (7.4, 2.4),
            (29.0, 9.9),
            (32.6, 6.2),
            (32.2, 1.6),
        ] {
            let d = device(m, l, DeviceKind::Light, "Luminaria colgante", p(x, y), 0.0)?;
            m.set_device_fixture(d, Some(pendant))?;
        }
        for (x, y) in [
            (14.9, 10.4),
            (12.3, 10.4),
            (14.2, 6.6),
            (11.9, 6.6),
            (12.4, 1.5),
            (14.75, 2.0),
            (23.0, 10.4),
            (23.0, 6.1),
            (23.0, 8.3),
            (29.15, 6.3),
            (26.8, 6.3),
            (21.6, 1.6),
            (26.4, 1.0),
            (19.2, 3.9),
            (24.0, 3.9),
        ] {
            let d = device(m, l, DeviceKind::Light, "Luminaria de techo", p(x, y), 0.0)?;
            m.set_device_fixture(d, Some(ceiling))?;
        }
        for (x, y, r) in [
            (15.95, 8.3, -FRAC_PI_2),
            (15.95, 4.4, -FRAC_PI_2),
            (15.95, 3.0, -FRAC_PI_2),
            (25.4, 5.0, PI),
            (26.35, 4.4, FRAC_PI_2),
            (24.4, 2.95, PI),
        ] {
            device(m, l, DeviceKind::Panel, "Cuadro de vivienda", p(x, y), r)?;
        }
        for (x, y, r) in [
            (3.6, 9.0, -FRAC_PI_2),
            (3.6, 5.0, -FRAC_PI_2),
            (3.6, 1.0, -FRAC_PI_2),
            (34.8, 9.0, FRAC_PI_2),
            (34.8, 5.0, FRAC_PI_2),
            (34.8, 1.6, FRAC_PI_2),
        ] {
            device(m, l, DeviceKind::Socket, "Enchufe", p(x, y), r)?;
        }
        // Exposed spiral ducts: from each kitchen's hood along the ceiling
        // to the façade, as the photographs show them.
        for (a, b) in [
            (p(10.15, 11.4), p(3.6, 11.4)),
            (p(9.65, 7.45), p(3.6, 7.45)),
            (p(9.65, 0.6), p(3.6, 0.6)),
            (p(29.95, 11.4), p(34.8, 11.4)),
            (p(34.4, 7.15), p(34.8, 7.15)),
            (p(33.3, 0.6), p(34.8, 0.6)),
        ] {
            m.add_run(NewRun {
                level: l,
                system: ServiceSystem::ExhaustAir,
                path: vec![a, b],
                elevation: soffit - 0.2,
                width: 0.15,
                height: 0.15,
                slope: 0.0,
            })?;
        }
        // Water and drainage from the riser along the corridor.
        for (system, dy, width, elev) in [
            (ServiceSystem::ColdWater, 0.0, 0.032, 2.6),
            (ServiceSystem::HotWater, 0.08, 0.025, 2.6),
        ] {
            m.add_run(NewRun {
                level: l,
                system,
                path: vec![p(16.5, 3.4 + dy), p(25.6, 3.4 + dy)],
                elevation: elev,
                width,
                height: width,
                slope: 0.0,
            })?;
        }
        // Each flat's branch: from its door along the ceiling to the
        // bathroom and on to the kitchen's sink (and F's washing machine).
        let branches: [&[(f64, f64)]; 6] = [
            &[
                (16.35, 8.4),
                (12.3, 8.4),
                (12.3, 10.6),
                (8.1, 10.6),
                (8.1, 11.3),
            ],
            &[
                (15.2, 4.75),
                (11.9, 4.75),
                (11.9, 6.8),
                (7.6, 6.8),
                (7.6, 7.4),
            ],
            &[
                (15.2, 3.45),
                (14.75, 3.45),
                (14.75, 2.0),
                (12.0, 2.0),
                (12.0, 0.6),
                (7.6, 0.6),
            ],
            &[
                (25.05, 5.2),
                (24.6, 5.2),
                (24.6, 8.3),
                (22.6, 8.3),
                (22.6, 8.1),
                (25.4, 8.1),
                (25.4, 11.0),
                (28.1, 11.0),
            ],
            &[
                (26.4, 3.9),
                (26.4, 6.3),
                (27.0, 6.3),
                (27.0, 4.2),
                (32.4, 4.2),
                (32.4, 7.0),
            ],
            &[
                (24.9, 2.6),
                (26.4, 2.6),
                (26.4, 1.0),
                (28.0, 1.0),
                (28.0, 2.6),
                (31.3, 2.6),
                (31.3, 0.7),
            ],
        ];
        for path in branches {
            for (system, d, width) in [
                (ServiceSystem::ColdWater, 0.0, 0.02),
                (ServiceSystem::HotWater, 0.06, 0.016),
            ] {
                m.add_run(NewRun {
                    level: l,
                    system,
                    path: path.iter().map(|(x, y)| p(*x + d, *y + d)).collect(),
                    elevation: 2.55,
                    width,
                    height: width,
                    slope: 0.0,
                })?;
            }
        }
        // Downpipes from the balconies.
        for (x, y) in [(WEST_EDGE + 0.1, 4.25), (EAST_EDGE - 0.1, 7.8)] {
            let d = device(
                m,
                l,
                DeviceKind::Downpipe,
                "Bajante de pluviales",
                p(x, y),
                0.0,
            )?;
            let _ = d;
        }
    }
    // Fire protection of the common corridor of each floor (DB-SI 4, H3):
    // an extinguisher and an emergency light on the wall of the core,
    // facing the corridor, and the exit sign over the stair.
    let face = 3.2 + 0.125;
    for &l in &lv[1..=4] {
        let (w, d, h, z) = DeviceKind::Extinguisher.defaults();
        m.add_device(NewDevice {
            level: l,
            kind: DeviceKind::Extinguisher,
            name: "Extintor 21A-113B".into(),
            position: p(21.2, face + d / 2.0),
            rotation: 0.0,
            elevation: z,
            width: w,
            depth: d,
            height: h,
        })?;
        for (kind, x, name) in [
            (DeviceKind::EmergencyLight, 23.4, "Alumbrado de emergencia"),
            (DeviceKind::ExitSign, 20.0, "Señal de salida"),
        ] {
            let (w, d, h, z) = kind.defaults();
            m.add_device(NewDevice {
                level: l,
                kind,
                name: name.into(),
                position: p(x, face + d / 2.0),
                rotation: 0.0,
                elevation: z,
                width: w,
                depth: d,
                height: h,
            })?;
        }
    }
    // On the deck's roof, three rows of solar panels looking south.
    for y in [3.0, 6.0, 9.0] {
        let (_, d, h, _) = DeviceKind::SolarPanel.defaults();
        let w = 4.0 * archisim_core::SOLAR_MODULE;
        m.add_device(NewDevice {
            level: lv[6],
            kind: DeviceKind::SolarPanel,
            name: "Placas fotovoltaicas".into(),
            position: p(28.5, y),
            rotation: PI,
            elevation: 0.0,
            width: w,
            depth: d,
            height: h,
        })?;
    }
    // Common areas: lights in the corridor of the ground floor and in the
    // fernery, the general board.
    let pb = lv[0];
    for x in [2.0, 6.0, 10.0, 14.0, 20.0, 24.0] {
        let d = device(
            m,
            pb,
            DeviceKind::Light,
            "Luminaria de techo",
            p(x, 5.8),
            0.0,
        )?;
        m.set_device_fixture(d, Some(ceiling))?;
    }
    device(
        m,
        pb,
        DeviceKind::Panel,
        "Cuadro general",
        p(16.6, 2.0),
        0.0,
    )?;
    // Water to the toilets and to the café's kitchen; on the roof, to the
    // laundry.
    for (system, d, width) in [
        (ServiceSystem::ColdWater, 0.0, 0.025),
        (ServiceSystem::HotWater, 0.06, 0.02),
    ] {
        for (level, path) in [
            (
                pb,
                vec![
                    p(16.5, 2.4),
                    p(16.5, 3.4),
                    p(13.6, 3.4),
                    p(13.6, 1.0),
                    p(12.4, 1.0),
                ],
            ),
            (
                pb,
                vec![p(16.5, 3.4), p(16.5, 5.8), p(2.0, 5.8), p(2.0, 11.2)],
            ),
            (
                lv[5],
                vec![
                    p(16.5, 1.5),
                    p(16.5, 2.6),
                    p(31.0, 2.6),
                    p(31.0, 11.0),
                    p(34.6, 11.0),
                ],
            ),
        ] {
            m.add_run(NewRun {
                level,
                system,
                path: path.into_iter().map(|q| p(q.x + d, q.y + d)).collect(),
                elevation: 2.6,
                width,
                height: width,
                slope: 0.0,
            })?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Rooms
// ---------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn room(
    m: &mut Model,
    level: LevelId,
    number: &str,
    name: &str,
    usage: &str,
    area: [f64; 4],
    floor: &str,
    ceiling: &str,
) -> R {
    use archisim_geom::{room_outline, signed_area, Contour};
    let [x0, y0, x1, y1] = area;
    let label = p((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let given = rect(x0, y0, x1, y1);
    let box_area = (x1 - x0) * (y1 - y0);
    // Bounded by its walls where they close it; else, as drawn (rooms
    // that end at glass or at an open edge).
    let found = room_outline(m, level, label, Contour::InteriorFace)
        .ok()
        .filter(|o| signed_area(o).abs() < box_area * 1.25);
    let bounded = found.is_some();
    let outline = found.unwrap_or(given);
    room_with(
        m, level, number, name, usage, outline, label, bounded, floor, ceiling,
    )
}

/// A room with its outline given.
#[allow(clippy::too_many_arguments)]
fn room_with(
    m: &mut Model,
    level: LevelId,
    number: &str,
    name: &str,
    usage: &str,
    outline: Vec<Point2>,
    label: Point2,
    bounded: bool,
    floor: &str,
    ceiling: &str,
) -> R {
    use archisim_core::{FloorFill, HatchPattern, NewRoom};
    let id = m.add_room(NewRoom {
        level,
        number: number.into(),
        name: name.into(),
        outline,
        holes: Vec::new(),
        label,
        bounded,
    })?;
    if let Some(mut r) = m.room(id).cloned() {
        r.usage = usage.into();
        r.finishes = archisim_core::Finishes {
            floor: floor.into(),
            walls: "Hormigón visto y pintura plástica".into(),
            ceiling: ceiling.into(),
        };
        r.floor_fill = Some(if floor.contains("Tarima") {
            FloorFill {
                pattern: HatchPattern::Oak,
                color: [150, 112, 76],
                background: Some([226, 206, 178]),
                scale: 1.0,
                angle: 0.0,
            }
        } else if floor.contains("adoquín") {
            FloorFill {
                pattern: HatchPattern::ConcretePavers,
                color: [128, 126, 120],
                background: Some([214, 210, 202]),
                scale: 1.0,
                angle: 0.0,
            }
        } else if floor.contains("Gres") {
            FloorFill {
                pattern: HatchPattern::Tiles,
                color: [160, 160, 156],
                background: Some([238, 238, 234]),
                scale: 1.0,
                angle: 0.0,
            }
        } else {
            FloorFill {
                pattern: HatchPattern::ConcreteFinished,
                color: [150, 148, 142],
                background: Some([222, 220, 214]),
                scale: 1.0,
                angle: 0.0,
            }
        });
        m.set_room(r)?;
    }
    Ok(())
}

fn rooms(m: &mut Model, lv: &[LevelId]) -> R {
    let wood = "Tarima de madera recuperada";
    let tiles = "Gres porcelánico";
    let concrete = "Hormigón pulido";
    let exposed = "Losa de hormigón visto";
    let pb = lv[0];
    for (n, name, usage, a, floor) in [
        (
            "0.01",
            "Café",
            "Comercial",
            [0.25, 7.2, 12.85, NORTH_IN],
            concrete,
        ),
        (
            "0.02",
            "Café: trastienda",
            "Almacén",
            [12.95, 7.2, 17.1, NORTH_IN],
            concrete,
        ),
        (
            "0.03",
            "Oficina de la cooperativa",
            "Administrativo",
            [0.25, SOUTH_IN, 11.6, 4.4],
            concrete,
        ),
        ("0.04", "Aseo", "Aseo", [11.8, SOUTH_IN, 13.05, 4.4], tiles),
        (
            "0.05",
            "Aseo accesible",
            "Aseo",
            [13.15, 2.25, 15.85, 4.4],
            tiles,
        ),
        (
            "0.06",
            "Cuarto de limpieza",
            "Limpieza",
            [13.15, SOUTH_IN, 17.3, 2.15],
            concrete,
        ),
        (
            "0.07",
            "Paso de entrada",
            "Portal",
            [0.0, 4.6, 17.1, 7.0],
            "Pavimento de adoquín en espiga",
        ),
        (
            "0.08",
            "Helechera",
            "Zona común",
            [17.2, 7.2, 22.1, NORTH_IN],
            concrete,
        ),
        (
            "0.09",
            "Cuarto de basuras",
            "Instalaciones",
            [19.7, SOUTH_IN, 25.4, 3.1],
            concrete,
        ),
        (
            "0.10",
            "Trasteros",
            "Trastero",
            [25.6, SOUTH_IN, PLOT.0 - 0.25, 6.5],
            concrete,
        ),
        (
            "0.11",
            "Aparcamiento de bicicletas",
            "Aparcamiento",
            [22.3, 6.7, PLOT.0 - 0.25, NORTH_IN],
            concrete,
        ),
    ] {
        room(m, pb, n, name, usage, a, floor, exposed)?;
    }
    for k in 1..=4 {
        let l = lv[k];
        let r = |n: &str| format!("{k}.{n}");
        let flats: [(&str, &str, &str, [f64; 4], &str); 24] = [
            (
                "A1",
                "Estar-cocina A",
                "Estar",
                [WEST_GLASS + 0.05, 8.0, 11.45, NORTH_IN],
                wood,
            ),
            (
                "A2",
                "Baño A",
                "Baño",
                [11.55, 9.05, 13.05, NORTH_IN],
                tiles,
            ),
            (
                "A3",
                "Dormitorio A",
                "Dormitorio",
                [13.15, 9.05, 16.9, NORTH_IN],
                wood,
            ),
            ("A4", "Paso A", "Pasillo", [11.5, 8.0, 16.9, 8.95], wood),
            (
                "B1",
                "Estar-cocina B",
                "Estar",
                [WEST_GLASS + 0.05, 4.16, 10.95, 7.8],
                wood,
            ),
            ("B2", "Baño B", "Baño", [11.05, 5.45, 12.75, 7.8], tiles),
            (
                "B3",
                "Dormitorio B",
                "Dormitorio",
                [12.85, 5.45, 15.6, 7.8],
                wood,
            ),
            ("B4", "Paso B", "Pasillo", [11.0, 4.16, 15.6, 5.35], wood),
            (
                "C1",
                "Estar-cocina C",
                "Estar",
                [WEST_GLASS + 0.05, SOUTH_IN, 10.85, 3.96],
                wood,
            ),
            ("C2", "Baño C", "Baño", [13.95, 1.25, 15.6, 2.85], tiles),
            (
                "C3",
                "Dormitorio C",
                "Dormitorio",
                [10.95, SOUTH_IN, 13.85, 2.85],
                wood,
            ),
            ("C4", "Paso C", "Pasillo", [10.9, 2.95, 15.6, 3.96], wood),
            (
                "D1",
                "Estar-cocina D",
                "Estar",
                [26.0, 7.7, EAST_GLASS - 0.05, NORTH_IN],
                wood,
            ),
            ("D2", "Baño D", "Baño", [21.8, 7.65, 24.15, 8.95], tiles),
            (
                "D3",
                "Dormitorio D",
                "Dormitorio",
                [21.8, 9.05, 25.8, NORTH_IN],
                wood,
            ),
            (
                "D4",
                "Dormitorio 2 D",
                "Dormitorio",
                [21.8, 4.7, 24.15, 7.55],
                wood,
            ),
            (
                "E1",
                "Estar-cocina E",
                "Estar",
                [27.75, 3.3, EAST_GLASS - 0.05, 7.5],
                wood,
            ),
            ("E2", "Baño E", "Baño", [26.0, 5.05, 27.65, 7.5], tiles),
            (
                "E3",
                "Dormitorio E",
                "Dormitorio",
                [27.75, 5.05, 30.55, 7.5],
                wood,
            ),
            (
                "F1",
                "Estar-cocina F",
                "Estar",
                [29.55, SOUTH_IN, EAST_GLASS - 0.05, 3.1],
                wood,
            ),
            (
                "F2",
                "Baño F",
                "Baño",
                [25.45, SOUTH_IN, 27.35, 1.95],
                tiles,
            ),
            (
                "F3",
                "Dormitorio F",
                "Dormitorio",
                [19.7, SOUTH_IN, 23.55, 3.1],
                wood,
            ),
            ("F4", "Paso F", "Pasillo", [23.65, 2.05, 29.45, 3.1], wood),
            (
                "F5",
                "Lavadero F",
                "Lavadero",
                [27.45, SOUTH_IN, 29.45, 1.95],
                tiles,
            ),
        ];
        for (n, name, usage, a, floor) in flats {
            room(m, l, &r(n), name, usage, a, floor, exposed)?;
        }
        // The common corridor with the stair and the landing to A: open
        // to each other, one room (the stair takes you down from it).
        room_with(
            m,
            l,
            &r("Z1"),
            "Pasillo común y escalera",
            "Pasillo común",
            vec![
                p(15.8, 3.3),
                p(25.8, 3.3),
                p(25.8, 4.5),
                p(21.6, 4.5),
                p(21.6, 7.1),
                p(16.9, 7.1),
                p(16.9, 7.8),
                p(15.8, 7.8),
            ],
            p(23.6, 3.9),
            false,
            concrete,
            exposed,
        )?;
        room(
            m,
            l,
            &r("Z2"),
            "Cuarto de instalaciones",
            "Instalaciones",
            [15.8, SOUTH_IN, 17.3, 3.1],
            concrete,
            exposed,
        )?;
    }
    let cu = lv[5];
    room(
        m,
        cu,
        "5.01",
        "Pabellón de invierno",
        "Zona común",
        [21.85, SOUTH_IN, EAST_GLASS - 0.05, NORTH_IN],
        "Tarima de exterior",
        "Chapa vista",
    )?;
    room(
        m,
        cu,
        "5.02",
        "Lavandería",
        "Lavadero",
        [31.65, 9.55, 34.95, NORTH_IN],
        tiles,
        "Chapa vista",
    )?;
    Ok(())
}

// ---------------------------------------------------------------------
// Site
// ---------------------------------------------------------------------

fn site(m: &mut Model, pb: LevelId) -> R {
    use archisim_core::{NewMass, NewPlot, NewRoad, RoadSection};
    m.add_plot(NewPlot {
        name: "Parcela".into(),
        level: pb,
        outline: rect(0.0, 0.0, PLOT.0, PLOT.1),
        setback: 0.0,
        reference: "Florence Street 6".into(),
    })?;
    let by = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
    let paving = by(m, "ceramic")
        .or_else(|| by(m, "concrete"))
        .ok_or(ModelError::EmptyType)?;
    let kerb = by(m, "concrete").ok_or(ModelError::EmptyType)?;
    let asphalt = m.add_material(
        "Aglomerado asfáltico",
        "asphalt",
        [96, 96, 100],
        archisim_core::CutPattern::Solid,
    );
    m.add_road(NewRoad {
        name: "Florence Street".into(),
        level: pb,
        axis: vec![p(-6.5, -30.0), p(-6.5, 42.0)],
        section: RoadSection {
            carriageway: 7.0,
            parking: 2.2,
            kerb: 0.15,
            sidewalk: 2.8,
            kerb_height: 0.15,
            depth: 0.4,
        },
        top_offset: -0.17,
        pavement: asphalt,
        kerb_material: kerb,
        sidewalk_material: paving,
    })?;
    // The neighbours, as masses: a two-storey house to the north, a
    // three-storey block to the south.
    let concept = by(m, "concept").ok_or(ModelError::EmptyType)?;
    for (name, r, h, floors) in [
        (
            "Colindante oeste",
            rect(0.0, PLOT.1, 30.0, 24.0),
            7.0,
            Some(3.5),
        ),
        (
            "Colindante este",
            rect(0.0, -12.0, 38.5, 0.0),
            10.0,
            Some(3.3),
        ),
    ] {
        m.add_mass(NewMass {
            name: name.into(),
            level: pb,
            shape: archisim_core::MassShape::Prism(r),
            base_offset: 0.0,
            height: h,
            floor_height: floors,
            material: concept,
        })?;
    }
    // Street trees along the far pavement and someone passing by.
    let plane = object_type(m, "Plátano de sombra")?;
    m.add_planting(archisim_core::NewPlanting {
        name: "Arbolado de Florence Street".into(),
        level: pb,
        path: vec![p(-11.0, -26.0), p(-11.0, 38.0)],
        closed: false,
        smooth: false,
        arrangement: archisim_core::Arrangement::Line,
        count: 7,
        species: vec![plane],
        size_variation: 0.2,
        shape_variation: 0.5,
        spread: 0.0,
        seed: 3,
        base_offset: -0.17,
    })?;
    obj_at(m, pb, "Persona", -1.6, 3.0, FRAC_PI_2, -0.02)?;
    obj_at(m, pb, "Bicicleta", -1.8, 8.0, FRAC_PI_2, -0.02)?;
    obj_at(m, pb, "Aparcabicis de onda", -1.2, 10.0, 0.0, -0.02)?;
    m.set_true_north(FRAC_PI_2)?;
    Ok(())
}

// ---------------------------------------------------------------------
// Documentation: sections, dimensions, tags, sheets, data
// ---------------------------------------------------------------------

fn documentation(m: &mut Model, lv: &[LevelId]) -> R {
    use archisim_core::{NewSection, NewSheet, NewViewport, ProjectInfo, SectionKind, TagTarget};
    for (name, start, end, depth) in [
        ("A", p(-3.0, 9.6), p(PLOT.0 + 3.0, 9.6), 6.0),
        ("B", p(19.0, -3.0), p(19.0, PLOT.1 + 3.0), 4.0),
    ] {
        m.add_section(NewSection {
            name: name.into(),
            kind: SectionKind::Section,
            start,
            end,
            depth,
        })?;
    }
    for e in archisim_plan::section::elevations_around(m) {
        m.add_section(e)?;
    }
    // Tags on the doors and windows of the typical floor, inside.
    let ops: Vec<_> = m
        .openings()
        .filter(|o| m.wall(o.wall).is_some_and(|w| w.level == lv[2]))
        .filter(|o| {
            m.opening_type(o.opening_type)
                .is_some_and(|t| t.kind != archisim_core::OpeningKind::Void)
        })
        .map(|o| o.id)
        .collect();
    for o in ops {
        if let Some(g) = archisim_geom::opening_geometry(m, o) {
            let at = g.point((g.t0 + g.t1) / 2.0, g.exterior_offset - 0.45);
            m.add_tag(TagTarget::Opening(o), at, false)?;
        }
    }
    // Overall dimensions along the party walls of each floor, and level
    // marks in the corridor.
    for (i, l) in lv.iter().enumerate().take(5) {
        let (x0, x1) = if i <= 1 {
            (0.0, PLOT.0)
        } else {
            (WEST_EDGE, EAST_EDGE)
        };
        m.add_dimension(archisim_core::NewDimension {
            level: *l,
            points: vec![
                p(x0, 0.0),
                p(WEST_GLASS, 0.0),
                p(15.7, 0.0),
                p(21.7, 0.0),
                p(EAST_GLASS, 0.0),
                p(x1, 0.0),
            ],
            direction: archisim_core::Vec2::new(1.0, 0.0),
            position: p(x0, -1.2),
        })?;
        m.add_dimension(archisim_core::NewDimension {
            level: *l,
            points: vec![p(x0, 0.0), p(x0, PLOT.1)],
            direction: archisim_core::Vec2::new(0.0, 1.0),
            position: p(x0 - 1.2, 0.0),
        })?;
        m.add_elevation_mark(*l, p(20.8, 3.9))?;
    }
    archisim_geom::anchors::follow(m, &Default::default());

    m.set_info(ProjectInfo {
        client: "Cooperativa de vivienda (ejercicio del máster BIM)".into(),
        site: "Florence Street, Brunswick (Melbourne)".into(),
        author: "Antonio Fernández".into(),
        studio: "AD5".into(),
        number: "MBIM-01".into(),
        phase: "Modelo de prueba de ArchiSim".into(),
        date: "octubre 2026".into(),
    });
    m.set_georeference(Some(archisim_core::Georeference {
        crs: "EPSG:32755".into(),
        eastings: 320_363.0,
        northings: 5_818_056.0,
        height: 45.0,
    }))?;
    m.set_code_settings(archisim_core::CodeSettings {
        building_use: archisim_core::BuildingUse::Dwelling,
        accessibility: Default::default(),
        sprinklers: true,
    });

    // Sheets: each floor at 1:100 on A2, sections and elevations, a
    // presentation board.
    let plans: [(&str, &str, usize); 4] = [
        ("A-01", "Planta baja", 0),
        ("A-02", "Planta 1", 1),
        ("A-03", "Planta tipo (2 a 4)", 2),
        ("A-04", "Cubierta", 5),
    ];
    for (number, name, k) in plans {
        let sheet = m.add_sheet(NewSheet {
            number: number.into(),
            name: name.into(),
            width: 594.0,
            height: 420.0,
        })?;
        m.add_viewport(NewViewport {
            sheet,
            level: lv[k],
            section: None,
            axo: None,
            scale: 100.0,
            cut_plane: 1.2,
            crop: None,
            centre: p(0.29, 0.25),
        })?;
    }
    let find = |m: &Model, t: &str| m.sections().find(|s| s.title() == t).map(|s| s.id);
    for (number, name, titles) in [
        ("A-05", "Secciones", ["Sección A", "Sección B"]),
        ("A-06", "Alzados", ["Alzado Sur", "Alzado Norte"]),
    ] {
        let sheet = m.add_sheet(NewSheet {
            number: number.into(),
            name: name.into(),
            width: 594.0,
            height: 420.0,
        })?;
        let mut left = 0.03;
        for title in titles {
            let Some(id) = find(m, title) else { continue };
            let Some(crop) = archisim_plan::sheet::section_extent(m, id, 100.0) else {
                continue;
            };
            let half = (crop.1 - crop.0) * (0.5 / 100.0);
            m.add_viewport(NewViewport {
                sheet,
                level: lv[0],
                section: Some(id),
                axo: None,
                scale: 100.0,
                cut_plane: 1.0,
                crop: Some(crop),
                centre: p(left + half.x, 0.22),
            })?;
            left += 2.0 * half.x + 0.03;
        }
    }
    let style = m.presentation_styles().next().map(|s| s.id);
    archisim_plan::layout::add_presentation_sheet(
        m,
        &archisim_plan::layout::PresentationSheet {
            number: "A-07".into(),
            name: "Presentación".into(),
            width: 594.0,
            height: 420.0,
            level: lv[2],
            style,
        },
    )?;
    Ok(())
}

// ---------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------

pub fn write(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let t = std::time::Instant::now();
    let doc = build()?;
    let m = doc.model();
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("edificio.{}", archisim_file::EXTENSION));
    archisim_file::save(m, &project)?;
    crate::summary(m);
    println!(
        "  Proyecto: {} (montado en {:.1} s)",
        project.display(),
        t.elapsed().as_secs_f64()
    );
    crate::export_all(m, dir, "edificio")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn building_builds() {
        let doc = super::build().unwrap();
        let m = doc.model();
        assert_eq!(m.levels().len(), 7);
        assert_eq!(m.stairs().count(), 5);
        for s in m.stairs() {
            assert!(s.issues().is_empty(), "{:?}", s.issues());
        }
        // Six flats a floor, four floors.
        let beds = m.rooms().filter(|r| r.usage == "Dormitorio").count();
        assert_eq!(beds, 4 * 7);
        assert!(m.walls().count() > 150, "{}", m.walls().count());
        assert!(m.objects().count() > 500, "{}", m.objects().count());
    }
}
