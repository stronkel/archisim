//! The workshop (M1): the models being remade with more care, in a
//! project of their own, each one next to the library's piece it is to
//! replace, to be looked at and judged before anything enters the library.
//!
//! The packages Antonio approved (the kitchen, the bathroom, the
//! bedroom, the lamps, the terrace, the street, the study, bars, offices,
//! shops, sport and pools) are system families now; the workshop places
//! them all the same. What is still being judged (`taller/*.asfam`) is
//! not a system family: a project that uses one keeps its copy.

use archisim_core::{
    Model, ModelError, NewObject, NewWall, ObjectTypeId, ParamScope, Point2, ReferenceLine, Value,
    WallTop,
};

/// The workshop's recipes (a piece whose family is `flora:<species>` is
/// a plant of the library grown with the workshop's values instead).
pub const RECIPES: &[&str] = &[
    include_str!("../taller/planta.asfam"),
    include_str!("../taller/farola.asfam"),
];

/// A piece of the workshop: its type name, its family, its values (sizes
/// as numbers, materials by their name in the cozy palette, choices by
/// their name), the library's piece it is to replace and the most
/// triangles it may have.
pub struct Piece {
    pub name: &'static str,
    pub family: &'static str,
    pub values: &'static [(&'static str, Val)],
    pub replaces: Option<&'static str>,
    pub most: usize,
}

pub enum Val {
    N(f64),
    M(&'static str),
    T(&'static str),
    B(bool),
}

use Val::{B, M, N, T};

impl Val {
    fn copy(&self) -> Val {
        match self {
            N(n) => N(*n),
            M(n) => M(n),
            T(t) => T(t),
            B(b) => B(*b),
        }
    }
}

pub const PIECES: &[Piece] = &[
    Piece {
        name: "Espatifilo en maceta esmaltada (taller)",
        family: "Planta de interior (taller)",
        values: &[
            ("height", N(0.85)),
            ("pot", N(0.3)),
            ("model3d", T("Espatifilo")),
            ("pot_mat", M("Cerámica esmaltada")),
            ("leaves", M("Hoja verde")),
            ("leaves2", M("Hoja verde tierna")),
            ("stem", M("Tallo verde")),
            ("flowers", M("Pétalo blanco")),
            ("spadix", M("Espádice")),
            ("soil", M("Tierra")),
        ],
        replaces: Some("Costilla de Adán"),
        most: 17_000,
    },
    Piece {
        name: "Pilea en maceta de barro (taller)",
        family: "Planta de interior (taller)",
        values: &[
            ("height", N(0.55)),
            ("pot", N(0.2)),
            ("model3d", T("Pilea")),
            ("pot_mat", M("Terracota")),
            ("leaves", M("Hoja verde")),
            ("leaves2", M("Hoja verde tierna")),
            ("stem", M("Tallo verde")),
            ("soil", M("Tierra")),
        ],
        replaces: Some("Costilla de Adán"),
        most: 17_000,
    },
    Piece {
        name: "Farola de fundición 3,5 m (taller)",
        family: "Farola de fundición (taller)",
        values: &[
            ("height", N(3.5)),
            ("mat", M("Fundición")),
            ("light", M("Cristal esmerilado")),
        ],
        replaces: Some("Farola fernandina 3,5 m"),
        most: 12_000,
    },
];

/// A kitchen or a bathroom of the workshop (paquete 2): one collection's
/// materials for every piece of it.
pub struct Kitchen {
    pub collection: &'static str,
    /// Materials by parameter, names of the cozy palette.
    pub materials: &'static [(&'static str, &'static str)],
}

/// The eight kitchens, one per collection.
pub const KITCHENS: &[Kitchen] = &[
    Kitchen {
        collection: "Nórdico cálido",
        materials: &[
            ("mat", "Lacado blanco"),
            ("plinth", "Roble claro"),
            ("top", "Roble claro"),
            ("metal", "Acero inoxidable"),
            ("sink", "Acero inoxidable"),
            ("dark", "Cuero coñac"),
            ("black", "Vitrocerámica"),
            ("ring", "Cal blanca"),
            ("wood", "Roble claro"),
            ("ceramic", "Cerámica blanca"),
            ("hood", "Lacado blanco"),
        ],
    },
    Kitchen {
        collection: "Mediterráneo",
        materials: &[
            ("mat", "Lacado azul añil"),
            ("plinth", "Lacado azul añil"),
            ("top", "Microcemento"),
            ("metal", "Latón"),
            ("sink", "Cerámica blanca"),
            ("dark", "Fresno oscuro"),
            ("black", "Metal negro"),
            ("ring", "Latón"),
            ("wood", "Pino viejo"),
            ("ceramic", "Cerámica turquesa"),
            ("hood", "Cal blanca"),
        ],
    },
    Kitchen {
        collection: "Años 50",
        materials: &[
            ("mat", "Lacado azul celeste"),
            ("plinth", "Nogal"),
            ("top", "Terrazo"),
            ("metal", "Acero inoxidable"),
            ("sink", "Acero inoxidable"),
            ("dark", "Metal negro"),
            ("black", "Chapa crema"),
            ("ring", "Acero inoxidable"),
            ("wood", "Nogal"),
            ("ceramic", "Cerámica esmaltada"),
            ("hood", "Chapa crema"),
        ],
    },
    Kitchen {
        collection: "Japandi",
        materials: &[
            ("mat", "Roble claro"),
            ("plinth", "Fresno oscuro"),
            ("top", "Piedra gris"),
            ("metal", "Metal negro"),
            ("sink", "Granito negro"),
            ("dark", "Fresno oscuro"),
            ("black", "Vitrocerámica"),
            ("ring", "Piedra gris"),
            ("wood", "Roble claro"),
            ("ceramic", "Gres"),
            ("hood", "Roble claro"),
        ],
    },
    Kitchen {
        collection: "Contemporáneo",
        materials: &[
            ("mat", "Lacado arena"),
            ("plinth", "Metal negro"),
            ("top", "Piedra caliza"),
            ("metal", "Acero inoxidable"),
            ("sink", "Acero inoxidable"),
            ("dark", "Metal negro"),
            ("black", "Vitrocerámica"),
            ("ring", "Piedra gris"),
            ("wood", "Roble claro"),
            ("ceramic", "Cerámica blanca"),
            ("hood", "Acero inoxidable"),
        ],
    },
    Kitchen {
        collection: "Rústica",
        materials: &[
            ("mat", "Lacado salvia"),
            ("plinth", "Lacado salvia"),
            ("top", "Roble claro"),
            ("metal", "Hierro forjado"),
            ("sink", "Cerámica blanca"),
            ("dark", "Fresno oscuro"),
            ("black", "Metal negro"),
            ("ring", "Hierro forjado"),
            ("wood", "Pino viejo"),
            ("ceramic", "Terracota"),
            ("hood", "Cal blanca"),
        ],
    },
    Kitchen {
        collection: "Industrial",
        materials: &[
            ("mat", "Roble ahumado"),
            ("plinth", "Metal negro"),
            ("top", "Hormigón pulido"),
            ("metal", "Metal negro"),
            ("sink", "Acero inoxidable"),
            ("dark", "Metal negro"),
            ("black", "Fundición"),
            ("ring", "Metal negro"),
            ("wood", "Fresno oscuro"),
            ("ceramic", "Gres"),
            ("hood", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Clásica",
        materials: &[
            ("mat", "Lacado marfil"),
            ("plinth", "Lacado marfil"),
            ("top", "Mármol"),
            ("metal", "Latón"),
            ("sink", "Cerámica blanca"),
            ("dark", "Fresno oscuro"),
            ("black", "Metal negro"),
            ("ring", "Latón"),
            ("wood", "Nogal"),
            ("ceramic", "Cerámica esmaltada"),
            ("hood", "Cal blanca"),
        ],
    },
];

/// The eight bathrooms, one per collection (paquete 2b).
pub const BATHROOMS: &[Kitchen] = &[
    Kitchen {
        collection: "Nórdico cálido",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Roble claro"),
            ("top", "Piedra caliza"),
            ("metal", "Acero inoxidable"),
            ("seat", "Lacado blanco"),
            ("frame", "Roble claro"),
            ("wood", "Roble claro"),
            ("towel", "Toalla blanca"),
            ("dark", "Cuero coñac"),
        ],
    },
    Kitchen {
        collection: "Mediterráneo",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Microcemento"),
            ("top", "Microcemento"),
            ("metal", "Latón"),
            ("seat", "Madera encalada"),
            ("frame", "Madera encalada"),
            ("wood", "Enea"),
            ("towel", "Lino blanco"),
            ("dark", "Fresno oscuro"),
        ],
    },
    Kitchen {
        collection: "Años 50",
        materials: &[
            ("mat", "Cerámica esmaltada"),
            ("unit", "Lacado azul celeste"),
            ("top", "Terrazo"),
            ("metal", "Acero inoxidable"),
            ("seat", "Lacado azul celeste"),
            ("frame", "Nogal"),
            ("wood", "Nogal"),
            ("towel", "Toalla blanca"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Japandi",
        materials: &[
            ("mat", "Gres"),
            ("unit", "Roble claro"),
            ("top", "Piedra gris"),
            ("metal", "Metal negro"),
            ("seat", "Roble claro"),
            ("frame", "Roble claro"),
            ("wood", "Roble claro"),
            ("towel", "Lino piedra"),
            ("dark", "Fresno oscuro"),
        ],
    },
    Kitchen {
        collection: "Contemporáneo",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Lacado arena"),
            ("top", "Piedra caliza"),
            ("metal", "Acero inoxidable"),
            ("seat", "Lacado blanco"),
            ("frame", "Roble claro"),
            ("wood", "Roble claro"),
            ("towel", "Lino gris"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Rústica",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Pino viejo"),
            ("top", "Piedra caliza"),
            ("metal", "Hierro forjado"),
            ("seat", "Pino viejo"),
            ("frame", "Pino viejo"),
            ("wood", "Enea"),
            ("towel", "Lino crudo"),
            ("dark", "Fresno oscuro"),
        ],
    },
    Kitchen {
        collection: "Industrial",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Roble ahumado"),
            ("top", "Hormigón pulido"),
            ("metal", "Metal negro"),
            ("seat", "Roble ahumado"),
            ("frame", "Roble ahumado"),
            ("wood", "Roble ahumado"),
            ("towel", "Lino gris"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Clásica",
        materials: &[
            ("mat", "Cerámica blanca"),
            ("unit", "Lacado marfil"),
            ("top", "Mármol"),
            ("metal", "Latón"),
            ("seat", "Nogal"),
            ("frame", "Latón"),
            ("wood", "Nogal"),
            ("towel", "Toalla blanca"),
            ("dark", "Fresno oscuro"),
        ],
    },
];

/// The eight bedrooms, one per collection (paquete 3).
pub const BEDROOMS: &[Kitchen] = &[
    Kitchen {
        collection: "Nórdico cálido",
        materials: &[
            ("frame", "Roble claro"),
            ("front", "Lacado blanco"),
            ("top", "Roble claro"),
            ("plinth", "Roble claro"),
            ("metal", "Metal negro"),
            ("dark", "Cuero coñac"),
            ("cushion", "Bouclé crudo"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino arena"),
            ("spread_mat", "Terciopelo salvia"),
            ("pillow", "Terciopelo salvia"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Cerámica blanca"),
            ("toy_mat", "Lana cruda"),
            ("clothes_mat", "Vaquero"),
        ],
    },
    Kitchen {
        collection: "Mediterráneo",
        materials: &[
            ("frame", "Madera encalada"),
            ("front", "Madera encalada"),
            ("top", "Madera encalada"),
            ("weave", "Ratán"),
            ("metal", "Hierro forjado"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Lino blanco"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino blanco"),
            ("spread_mat", "Lino añil"),
            ("pillow", "Lana teja"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Terracota"),
            ("toy_mat", "Lino arena"),
            ("clothes_mat", "Lino añil"),
        ],
    },
    Kitchen {
        collection: "Años 50",
        materials: &[
            ("frame", "Teca"),
            ("front", "Teca"),
            ("top", "Teca"),
            ("metal", "Latón"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Lana oliva"),
            ("linen", "Lino blanco"),
            ("duvet", "Lana cruda"),
            ("spread_mat", "Terciopelo mostaza"),
            ("pillow", "Pana azul petróleo"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Cerámica turquesa"),
            ("toy_mat", "Terciopelo mostaza"),
            ("clothes_mat", "Lana oliva"),
        ],
    },
    Kitchen {
        collection: "Japandi",
        materials: &[
            ("frame", "Roble ahumado"),
            ("front", "Roble ahumado"),
            ("top", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Lino piedra"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino piedra"),
            ("spread_mat", "Lana carbón"),
            ("pillow", "Lino gris"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Gres"),
            ("books", "Lomo crema"),
            ("books2", "Lomo verde"),
            ("toy_mat", "Lino piedra"),
            ("clothes_mat", "Lana carbón"),
        ],
    },
    Kitchen {
        collection: "Contemporáneo",
        materials: &[
            ("frame", "Lacado arena"),
            ("front", "Lacado arena"),
            ("top", "Lacado arena"),
            ("metal", "Metal negro"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Tela gris"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino gris"),
            ("spread_mat", "Lana cruda"),
            ("pillow", "Terciopelo rosa"),
            ("shade", "Cristal esmerilado"),
            ("ceramic", "Cerámica blanca"),
            ("toy_mat", "Terciopelo rosa"),
            ("clothes_mat", "Tela gris"),
        ],
    },
    Kitchen {
        collection: "Rústica",
        materials: &[
            ("frame", "Pino viejo"),
            ("front", "Pino viejo"),
            ("top", "Pino viejo"),
            ("metal", "Hierro forjado"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Lino crudo"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino crudo"),
            ("spread_mat", "Lana teja"),
            ("pillow", "Lino arena"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Terracota"),
            ("toy_mat", "Lana teja"),
            ("clothes_mat", "Vaquero"),
        ],
    },
    Kitchen {
        collection: "Industrial",
        materials: &[
            ("frame", "Roble ahumado"),
            ("front", "Roble ahumado"),
            ("top", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("dark", "Metal negro"),
            ("cushion", "Cuero coñac"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino gris"),
            ("spread_mat", "Lana carbón"),
            ("pillow", "Pana azul petróleo"),
            ("bulb", "Cristal esmerilado"),
            ("ceramic", "Gres"),
            ("toy_mat", "Lana cruda"),
            ("clothes_mat", "Vaquero"),
        ],
    },
    Kitchen {
        collection: "Clásica",
        materials: &[
            ("frame", "Lacado marfil"),
            ("front", "Lacado marfil"),
            ("stone", "Mármol"),
            ("metal", "Latón"),
            ("dark", "Fresno oscuro"),
            ("cushion", "Terciopelo verde botella"),
            ("linen", "Lino blanco"),
            ("duvet", "Lino blanco"),
            ("spread_mat", "Terciopelo verde botella"),
            ("pillow", "Terciopelo mostaza"),
            ("shade", "Pantalla de lino"),
            ("ceramic", "Cerámica blanca"),
            ("toy_mat", "Terciopelo rosa"),
            ("clothes_mat", "Lana carbón"),
        ],
    },
];

/// The pieces of a workshop bedroom along its wall, left to right:
/// family, label, how far it stands from the one before (m) and its
/// values. The bench goes at the foot of the bed.
const ROOM: &[(&str, &str, f64, Values)] = &[
    (
        "Armario",
        "Armario",
        0.0,
        &[("width", N(1.5)), ("doors", N(3.0)), ("drawers", N(1.0))],
    ),
    (
        "Cómoda",
        "Cómoda",
        0.35,
        &[("columns", N(2.0)), ("width", N(1.1))],
    ),
    ("Mesita de noche", "Mesita", 0.45, &[]),
    ("Cama", "Cama", 0.08, &[("width", N(1.6))]),
    ("Mesita de noche", "Mesita", 0.08, &[]),
    ("Tocador", "Tocador", 0.6, &[]),
    ("Galán de noche", "Galán", 0.4, &[]),
    ("Espejo de pie", "Espejo de pie", 0.3, &[]),
    // The children's room, further along the same wall.
    ("Cama infantil", "Cama infantil", 1.0, &[]),
    ("Cuna", "Cuna", 0.5, &[]),
    ("Litera", "Litera", 0.7, &[]),
];

/// Places a bedroom with its back on a wall along `y = wall` from `x`
/// (looking south); returns where it ends.
fn place_bedroom(m: &mut Model, k: &Kitchen, x: f64, wall: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let mut at = x;
    let mut bed = None;
    for (family_name, label, gap, values) in ROOM {
        let vals: Vec<(&str, Val)> = values.iter().map(|(key, v)| (*key, v.copy())).collect();
        let t = kitchen_type(m, k, family_name, label, &vals)?;
        let (w, depth) = m
            .object_type_box(t)
            .map_or((0.6, 0.6), |(lo, hi)| (hi[0] - lo[0], hi[1] - lo[1]));
        at += gap;
        let cx = at + w / 2.0;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(cx, wall),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
        if *label == "Cama" {
            bed = Some((cx, depth));
        }
        at += w;
    }
    // The bench at the foot of the bed.
    if let Some((cx, depth)) = bed {
        let t = kitchen_type(m, k, "Banco de pie de cama", "Banco", &[("width", N(1.3))])?;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(cx, wall - depth - 0.12),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
    }
    Ok(at)
}

/// The lamps of the eight collections (paquete 4).
pub const LIGHTS: &[Kitchen] = &[
    Kitchen {
        collection: "Nórdico cálido",
        materials: &[
            ("shade", "Pantalla de lino"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Lacado blanco"),
            ("frame", "Roble claro"),
            ("metal", "Metal negro"),
            ("ceramic", "Cerámica blanca"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Mediterráneo",
        materials: &[
            ("shade", "Ratán"),
            ("weave", "Ratán"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Cal blanca"),
            ("frame", "Madera encalada"),
            ("metal", "Hierro forjado"),
            ("ceramic", "Terracota"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Años 50",
        materials: &[
            ("shade", "Cristal esmerilado"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Latón"),
            ("frame", "Teca"),
            ("metal", "Latón"),
            ("ceramic", "Cerámica turquesa"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Japandi",
        materials: &[
            ("shade", "Papel de arroz"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Metal negro"),
            ("frame", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("ceramic", "Gres"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Contemporáneo",
        materials: &[
            ("shade", "Cristal esmerilado"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Metal negro"),
            ("frame", "Roble claro"),
            ("metal", "Metal negro"),
            ("ceramic", "Cerámica blanca"),
            ("stone", "Mármol"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Rústica",
        materials: &[
            ("shade", "Pantalla de lino"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Hierro forjado"),
            ("frame", "Pino viejo"),
            ("metal", "Hierro forjado"),
            ("ceramic", "Cera"),
            ("glass", "Vidrio claro"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Industrial",
        materials: &[
            ("shade", "Cristal esmerilado"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Metal negro"),
            ("frame", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("ceramic", "Gres"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Clásica",
        materials: &[
            ("shade", "Pantalla de lino"),
            ("bulb", "Cristal esmerilado"),
            ("body_mat", "Latón"),
            ("frame", "Lacado marfil"),
            ("metal", "Latón"),
            ("ceramic", "Cera"),
            ("stone", "Mármol"),
            ("dark", "Metal negro"),
        ],
    },
];

/// Places a collection's lamps along a wall at `y = wall` from `x`
/// (looking south): a chest of drawers of its bedroom with the table lamp
/// on it, the wall light, the floor lamp, and the pendant and the ceiling
/// light hung from a ceiling at 2.6 m before them. Returns where it ends.
fn place_lights(
    m: &mut Model,
    k: &Kitchen,
    room: &Kitchen,
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, base: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: std::f64::consts::PI,
            base_offset: base,
        })
        .map(|_| ())
    };
    let chest = kitchen_type(
        m,
        room,
        "Cómoda",
        "Cómoda",
        &[("columns", N(2.0)), ("width", N(1.1))],
    )?;
    put(m, chest, x + 0.55, wall, 0.0)?;
    let lamp = kitchen_type(m, k, "Lámpara de mesa", "Lámpara de mesa", &[])?;
    put(m, lamp, x + 0.8, wall, 0.85)?;
    let sconce = kitchen_type(m, k, "Aplique", "Aplique", &[("elevation", N(1.8))])?;
    put(m, sconce, x + 1.6, wall, 0.0)?;
    let floor = kitchen_type(m, k, "Lámpara de pie", "Lámpara de pie", &[])?;
    put(m, floor, x + 2.3, wall, 0.0)?;
    let pendant = kitchen_type(
        m,
        k,
        "Lámpara colgante",
        "Lámpara colgante",
        &[("diameter", N(0.5))],
    )?;
    put(m, pendant, x + 1.2, wall - 1.4, 0.0)?;
    let ceiling = kitchen_type(m, k, "Plafón", "Plafón", &[])?;
    put(m, ceiling, x + 2.6, wall - 1.4, 0.0)?;
    Ok(x + 3.2)
}

/// The terraces of the eight collections (paquete 5a).
pub const TERRACES: &[Kitchen] = &[
    Kitchen {
        collection: "Nórdico cálido",
        materials: &[
            ("frame", "Roble claro"),
            ("metal", "Metal negro"),
            ("cushion", "Lino arena"),
            ("pillow", "Terciopelo salvia"),
            ("mat", "Lino crudo"),
            ("accent", "Lino gris"),
            ("pot_mat", "Microcemento"),
            ("stem", "Espiga"),
            ("leaves", "Gramínea"),
            ("leaves2", "Hoja verde tierna"),
            ("flowers", "Espiga"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Mediterráneo",
        materials: &[
            ("frame", "Madera encalada"),
            ("metal", "Hierro forjado"),
            ("weave", "Enea"),
            ("cushion", "Lino blanco"),
            ("pillow", "Lino añil"),
            ("mat", "Paja"),
            ("accent", "Lino añil"),
            ("top", "Cerámica esmaltada"),
            ("masonry", "Cal blanca"),
            ("pot_mat", "Terracota"),
            ("stem", "Corteza de olivo"),
            ("leaves", "Hoja de olivo"),
            ("leaves2", "Envés de olivo"),
            ("dark", "Fresno oscuro"),
        ],
    },
    Kitchen {
        collection: "Años 50",
        materials: &[
            ("frame", "Teca"),
            ("metal", "Acero inoxidable"),
            ("weave", "Terciopelo mostaza"),
            ("cushion", "Lana oliva"),
            ("pillow", "Terciopelo mostaza"),
            ("mat", "Lino blanco"),
            ("accent", "Terciopelo mostaza"),
            ("pot_mat", "Cerámica turquesa"),
            ("stem", "Hoja de lavanda"),
            ("leaves", "Hoja de lavanda"),
            ("flowers", "Lavanda"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Japandi",
        materials: &[
            ("frame", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("weave", "Lino piedra"),
            ("cushion", "Lino piedra"),
            ("pillow", "Lana carbón"),
            ("mat", "Lino piedra"),
            ("accent", "Lino gris"),
            ("pot_mat", "Gres"),
            ("stem", "Espiga"),
            ("leaves", "Gramínea"),
            ("leaves2", "Hoja verde tierna"),
            ("flowers", "Espiga"),
            ("dark", "Fresno oscuro"),
        ],
    },
    Kitchen {
        collection: "Contemporáneo",
        materials: &[
            ("frame", "Roble claro"),
            ("metal", "Metal negro"),
            ("cushion", "Tela gris"),
            ("pillow", "Terciopelo rosa"),
            ("mat", "Tela gris"),
            ("accent", "Tela gris"),
            ("top", "Hormigón pulido"),
            ("pot_mat", "Hormigón pulido"),
            ("stem", "Corteza de olivo"),
            ("leaves", "Hoja de olivo"),
            ("leaves2", "Envés de olivo"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Rústica",
        materials: &[
            ("frame", "Pino viejo"),
            ("metal", "Hierro forjado"),
            ("cushion", "Lino crudo"),
            ("pillow", "Lana teja"),
            ("mat", "Lino crudo"),
            ("accent", "Lana teja"),
            ("pot_mat", "Roble ahumado"),
            ("stem", "Hoja de lavanda"),
            ("leaves", "Hoja de lavanda"),
            ("flowers", "Lavanda"),
            ("dark", "Piedra gris"),
        ],
    },
    Kitchen {
        collection: "Industrial",
        materials: &[
            ("frame", "Roble ahumado"),
            ("metal", "Metal negro"),
            ("cushion", "Tela gris"),
            ("pillow", "Pana azul petróleo"),
            ("mat", "Tela gris"),
            ("accent", "Tela gris"),
            ("pot_mat", "Chapa teja"),
            ("stem", "Espiga"),
            ("leaves", "Gramínea"),
            ("leaves2", "Hoja verde tierna"),
            ("flowers", "Espiga"),
            ("dark", "Metal negro"),
        ],
    },
    Kitchen {
        collection: "Clásica",
        materials: &[
            ("frame", "Lacado marfil"),
            ("metal", "Fundición"),
            ("weave", "Ratán"),
            ("cushion", "Lino blanco"),
            ("pillow", "Terciopelo verde botella"),
            ("mat", "Lino blanco"),
            ("accent", "Terciopelo verde botella"),
            ("top", "Mármol"),
            ("masonry", "Piedra caliza"),
            ("pot_mat", "Piedra caliza"),
            ("stem", "Corteza de olivo"),
            ("leaves", "Hoja de olivo"),
            ("leaves2", "Envés de olivo"),
            ("dark", "Fundición"),
        ],
    },
];

/// Places a collection's terrace from `x` along `y = front` (looking
/// south): the table with its chairs under the pergola, the sofa, the
/// sun lounger under its parasol and a big pot. Returns where it ends.
fn place_terrace(m: &mut Model, k: &Kitchen, x: f64, front: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pergola = kitchen_type(
        m,
        k,
        "Pérgola",
        "Pérgola",
        &[("width", N(3.2)), ("depth", N(3.0))],
    )?;
    put(m, pergola, x + 1.6, front)?;
    let table = kitchen_type(
        m,
        k,
        "Mesa de terraza",
        "Mesa de exterior",
        &[("width", N(1.6)), ("depth", N(0.9)), ("chairs", N(4.0))],
    )?;
    put(m, table, x + 1.6, front - 1.05)?;
    let sofa = kitchen_type(m, k, "Sofá de exterior", "Sofá de exterior", &[])?;
    put(m, sofa, x + 4.3, front)?;
    let lounger = kitchen_type(m, k, "Tumbona", "Tumbona", &[])?;
    put(m, lounger, x + 6.1, front)?;
    let parasol = kitchen_type(m, k, "Sombrilla", "Sombrilla", &[("width", N(2.2))])?;
    // Beside the lounger's head, not through it: a parasol on its pole
    // stands at its side and shades the head; a cantilever one hangs
    // its canopy over the head from its arm beside it.
    if matches!(k.collection, "Contemporáneo" | "Industrial") {
        put(m, parasol, x + 6.1, front + 0.6)?;
    } else {
        put(m, parasol, x + 6.85, front + 0.75)?;
    }
    let pot = kitchen_type(m, k, "Macetón", "Macetón", &[])?;
    put(m, pot, x + 8.6, front)?;
    let chair = kitchen_type(m, k, "Silla de exterior", "Silla de exterior", &[])?;
    put(m, chair, x + 8.6, front - 1.2)?;
    Ok(x + 9.2)
}

/// What the house plants, the decoration and the pictures of each
/// collection (paquete 6, tanda 6a) have besides its terrace's
/// materials, in the order of [`TERRACES`].
pub const INDOOR_EXTRAS: [&[(&str, &str)]; 8] = [
    &[
        ("pot_mat", "Cerámica blanca"),
        ("stem", "Corteza de pino"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("mat", "Cerámica blanca"),
        ("books", "Libro crema"),
        ("books2", "Libro azul"),
        ("books3", "Libro teja"),
        ("art", "Lino arena"),
        ("art2", "Lana teja"),
        ("art3", "Terciopelo salvia"),
        ("mount", "Papel"),
    ],
    &[
        ("pot_mat", "Terracota"),
        ("stem", "Corteza de pino"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("fruit", "Limón"),
        ("mat", "Terracota"),
        ("glaze2", "Cerámica esmaltada"),
        ("mount", "Cerámica blanca"),
        ("art2", "Lino añil"),
        ("art3", "Cerámica esmaltada"),
    ],
    &[
        ("pot_mat", "Cerámica turquesa"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("fruit", "Naranja"),
        ("fruit2", "Manzana verde"),
        ("mat", "Cerámica turquesa"),
        ("glaze2", "Terciopelo mostaza"),
        ("glass", "Espejo"),
        ("metal", "Latón"),
    ],
    &[
        ("pot_mat", "Gres"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("mat", "Fundición"),
        ("glaze2", "Cerámica blanca"),
        ("art", "Papel de arroz"),
        ("mount", "Lino piedra"),
        ("art3", "Metal negro"),
        ("dark", "Metal negro"),
    ],
    &[
        ("pot_mat", "Hormigón pulido"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("mat", "Cerámica blanca"),
        ("glaze2", "Latón"),
        ("books", "Libro crema"),
        ("books2", "Lana carbón"),
        ("books3", "Libro teja"),
        ("art", "Lino blanco"),
        ("art2", "Terciopelo rosa"),
        ("art3", "Pana azul petróleo"),
        ("dark", "Metal negro"),
    ],
    &[
        ("weave", "Ratán"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde tierna"),
        ("leaves2", "Hoja verde"),
        ("mat", "Terracota"),
        ("glass", "Vidrio claro"),
        ("dark", "Hierro forjado"),
    ],
    &[
        ("pot_mat", "Acero inoxidable"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("mat", "Cerámica blanca"),
        ("glass", "Vidrio claro"),
        ("books", "Libro crema"),
        ("books2", "Libro teja"),
        ("dark", "Metal negro"),
    ],
    &[
        ("pot_mat", "Cerámica esmaltada"),
        ("stem", "Tallo verde"),
        ("leaves", "Hoja verde"),
        ("leaves2", "Hoja verde tierna"),
        ("mat", "Cerámica blanca"),
        ("metal", "Oro viejo"),
        ("flowers", "Pétalo blanco"),
        ("art", "Lino añil"),
        ("art2", "Lana oliva"),
        ("art3", "Corteza de pino"),
    ],
];

/// Places a collection's corner of the living room from `x` along the
/// wall at `y = wall` (looking south): a sideboard with its decoration on
/// top and a picture over it, and a house plant beside. Returns where it
/// ends.
fn place_living(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, base: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, wall),
            rotation: std::f64::consts::PI,
            base_offset: base,
        })
        .map(|_| ())
    };
    let values = |more: &[(&'static str, Val)]| -> Vec<(&'static str, Val)> {
        let mut v: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
        v.extend(more.iter().map(|(k, v)| (*k, v.copy())));
        v
    };
    let board = kitchen_type(m, k, "Aparador", "Aparador", &values(&[("width", N(1.6))]))?;
    put(m, board, x + 0.9, 0.0)?;
    let decor = kitchen_type(m, k, "Decoración", "Decoración", &values(&[]))?;
    put(m, decor, x + 0.55, 0.75)?;
    let picture = kitchen_type(
        m,
        k,
        "Cuadro o espejo",
        "Cuadro o espejo",
        &values(&[
            ("width", N(0.7)),
            ("height", N(0.85)),
            ("elevation", N(1.2)),
        ]),
    )?;
    put(m, picture, x + 1.1, 0.0)?;
    let plant = kitchen_type(m, k, "Planta de interior", "Planta en maceta", &values(&[]))?;
    put(m, plant, x + 2.25, 0.0)?;
    Ok(x + 2.8)
}

/// What the study of each collection (paquete 6, tanda 6b) has besides
/// its terrace's materials, in the order of [`TERRACES`].
pub const STUDY_EXTRAS: [&[(&str, &str)]; 8] = [
    &[
        ("top", "Roble claro"),
        ("front", "Roble claro"),
        ("cushion", "Lino arena"),
    ],
    &[
        ("top", "Madera encalada"),
        ("front", "Madera encalada"),
        ("frame", "Madera encalada"),
        ("weave", "Enea"),
    ],
    &[
        ("top", "Teca"),
        ("front", "Teca"),
        ("accent", "Terciopelo mostaza"),
        ("cushion", "Lana oliva"),
    ],
    &[
        ("top", "Roble ahumado"),
        ("frame", "Roble ahumado"),
        ("weave", "Lino piedra"),
    ],
    &[
        ("top", "Cerámica blanca"),
        ("front", "Cerámica blanca"),
        ("dark", "Metal negro"),
        ("mesh", "Lana carbón"),
    ],
    &[
        ("top", "Pino viejo"),
        ("frame", "Pino viejo"),
        ("weave", "Enea"),
    ],
    &[
        ("top", "Roble ahumado"),
        ("metal", "Acero inoxidable"),
        ("dark", "Metal negro"),
    ],
    &[
        ("top", "Teca"),
        ("frame", "Teca"),
        ("front", "Teca"),
        ("metal", "Oro viejo"),
        ("accent", "Terciopelo verde botella"),
        ("cushion", "Terciopelo verde botella"),
    ],
];

/// Places a collection's study from `x` along the wall at `y = wall`
/// (looking south): the desk against the wall with its lamp on it, the
/// chair before it, and a bookcase beside. Returns where it ends.
fn place_study(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64, base: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: base,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let values = |more: &[(&'static str, Val)]| -> Vec<(&'static str, Val)> {
        let mut v: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
        v.extend(more.iter().map(|(k, v)| (*k, v.copy())));
        v
    };
    let desk = kitchen_type(m, k, "Escritorio", "Escritorio", &values(&[]))?;
    put(m, desk, x + 0.8, wall, pi, 0.0)?;
    let lamp = kitchen_type(m, k, "Lámpara de mesa", "Lámpara de mesa", &[])?;
    put(m, lamp, x + 1.25, wall - 0.15, pi, 0.75)?;
    // The chair before the desk, facing it.
    let chair = kitchen_type(m, k, "Silla de oficina", "Silla de trabajo", &values(&[]))?;
    put(m, chair, x + 0.7, wall - 0.68, pi, 0.0)?;
    let shelves = kitchen_type(
        m,
        k,
        "Estantería",
        "Estantería",
        &values(&[("width", N(0.9))]),
    )?;
    put(m, shelves, x + 2.2, wall, pi, 0.0)?;
    Ok(x + 2.8)
}

/// What the bar of each collection (paquete 7, tanda 7a) has besides its
/// terrace's materials, in the order of [`TERRACES`].
pub const BAR_EXTRAS: [&[(&str, &str)]; 8] = [
    &[
        ("frame", "Roble claro"),
        ("top", "Roble claro"),
        ("front", "Roble claro"),
        ("carcass", "Lacado blanco"),
        ("metal", "Acero inoxidable"),
        ("dark", "Metal negro"),
        ("cushion", "Lino arena"),
        ("pillow", "Terciopelo salvia"),
    ],
    &[
        ("frame", "Madera encalada"),
        ("front", "Madera encalada"),
        ("top", "Pino viejo"),
        ("masonry", "Cal blanca"),
        ("accent", "Cerámica esmaltada"),
        ("carcass", "Lacado blanco"),
        ("metal", "Hierro forjado"),
        ("dark", "Hierro forjado"),
        ("weave", "Enea"),
        ("cushion", "Lino blanco"),
        ("pillow", "Lino añil"),
    ],
    &[
        ("frame", "Teca"),
        ("front", "Teca"),
        ("top", "Teca"),
        ("accent", "Lacado azul celeste"),
        ("carcass", "Lacado marfil"),
        ("metal", "Acero inoxidable"),
        ("dark", "Metal negro"),
        ("cushion", "Terciopelo mostaza"),
    ],
    &[
        ("frame", "Roble ahumado"),
        ("front", "Roble ahumado"),
        ("top", "Piedra caliza"),
        ("dark", "Fresno oscuro"),
        ("carcass", "Lacado arena"),
        ("metal", "Acero inoxidable"),
        ("weave", "Lino piedra"),
        ("cushion", "Lino piedra"),
    ],
    &[
        ("mat", "Microcemento"),
        ("top", "Granito negro"),
        ("front", "Lacado blanco"),
        ("carcass", "Lacado blanco"),
        ("dark", "Metal negro"),
        ("metal", "Acero inoxidable"),
        ("accent", "Lacado salvia"),
        ("cushion", "Lana carbón"),
        ("light", "Luz cálida"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("front", "Pino viejo"),
        ("top", "Pino viejo"),
        ("masonry", "Piedra caliza"),
        ("carcass", "Pino viejo"),
        ("metal", "Hierro forjado"),
        ("dark", "Hierro forjado"),
        ("cushion", "Lino crudo"),
        ("pillow", "Lana teja"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("front", "Metal negro"),
        ("top", "Roble ahumado"),
        ("masonry", "Ladrillo visto"),
        ("carcass", "Lacado arena"),
        ("dark", "Metal negro"),
        ("metal", "Acero inoxidable"),
        ("cushion", "Cuero coñac"),
    ],
    &[
        ("frame", "Nogal"),
        ("front", "Nogal"),
        ("top", "Mármol"),
        ("carcass", "Lacado marfil"),
        ("metal", "Latón"),
        ("dark", "Fundición"),
        ("weave", "Ratán"),
        ("cushion", "Terciopelo verde botella"),
    ],
];

/// The bar pieces of the workshop: family and the start of their types'
/// names.
pub const BAR_PIECES: [(&str, &str); 5] = [
    ("Barra de bar", "Barra de bar"),
    ("Contrabarra", "Contrabarra"),
    ("Mesa de bar", "Mesa de bar"),
    ("Silla de bar", "Silla de bar"),
    ("Banco corrido", "Banco corrido"),
];

/// Places a collection's bar from `x` along the wall at `y = wall`
/// (looking south): the back bar on the wall, the bar counter before it
/// with three of the collection's stools, and a banquette with two café
/// tables and their chairs. Returns where it ends.
fn place_bar(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let values: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
    let mut types = Vec::new();
    for (family, label) in BAR_PIECES {
        types.push(kitchen_type(m, k, family, label, &values)?);
    }
    let [counter, back, table, chair, bench] = [types[0], types[1], types[2], types[3], types[4]];
    put(m, back, x + 1.2, wall, pi)?;
    put(m, counter, x + 1.2, wall - 1.4, pi)?;
    let stool = kitchen_type(m, k, "Taburete", "Taburete", &[("height", N(0.75))])?;
    for sx in [x + 0.42, x + 1.2, x + 1.98] {
        put(m, stool, sx, wall - 2.25 - 0.19, 0.0)?;
    }
    put(m, bench, x + 4.0, wall, pi)?;
    for tx in [x + 3.55, x + 4.45] {
        put(m, table, tx, wall - 0.95 - 0.3, 0.0)?;
        put(m, chair, tx, wall - 1.75, 0.0)?;
    }
    Ok(x + 4.9)
}

/// What the office of each collection (paquete 7, tanda 7b) has besides
/// its terrace's materials, in the order of [`TERRACES`].
pub const OFFICE_EXTRAS: [&[(&str, &str)]; 8] = [
    &[
        ("frame", "Roble claro"),
        ("top", "Roble claro"),
        ("front", "Roble claro"),
        ("carcass", "Lacado blanco"),
        ("screen", "Fieltro salvia"),
        ("cushion", "Lino arena"),
    ],
    &[
        ("frame", "Madera encalada"),
        ("front", "Madera encalada"),
        ("top", "Madera encalada"),
        ("masonry", "Cal blanca"),
        ("accent", "Cerámica esmaltada"),
        ("carcass", "Lacado blanco"),
        ("weave", "Enea"),
        ("screen", "Lino blanco"),
        ("metal", "Hierro forjado"),
    ],
    &[
        ("frame", "Teca"),
        ("front", "Teca"),
        ("top", "Teca"),
        ("accent", "Terciopelo mostaza"),
        ("carcass", "Lacado marfil"),
        ("metal", "Latón"),
        ("screen", "Fieltro teja"),
    ],
    &[
        ("frame", "Roble ahumado"),
        ("front", "Roble ahumado"),
        ("top", "Roble ahumado"),
        ("carcass", "Lacado arena"),
        ("dark", "Fresno oscuro"),
        ("screen", "Papel de arroz"),
        ("weave", "Lino piedra"),
    ],
    &[
        ("top", "Lacado blanco"),
        ("front", "Lacado blanco"),
        ("frame", "Lacado blanco"),
        ("mat", "Microcemento"),
        ("carcass", "Lacado blanco"),
        ("dark", "Metal negro"),
        ("screen", "Fieltro gris"),
        ("accent", "Lacado salvia"),
        ("light", "Luz cálida"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("front", "Pino viejo"),
        ("top", "Pino viejo"),
        ("masonry", "Piedra caliza"),
        ("carcass", "Pino viejo"),
        ("weave", "Lino crudo"),
        ("metal", "Hierro forjado"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("front", "Metal negro"),
        ("top", "Roble ahumado"),
        ("carcass", "Lacado arena"),
        ("dark", "Metal negro"),
        ("metal", "Acero inoxidable"),
        ("screen", "Fieltro teja"),
    ],
    &[
        ("frame", "Nogal"),
        ("front", "Nogal"),
        ("top", "Nogal"),
        ("carcass", "Lacado marfil"),
        ("metal", "Latón"),
        ("accent", "Cuero coñac"),
        ("screen", "Terciopelo verde botella"),
    ],
];

/// The office pieces of the workshop: family and the start of their
/// types' names.
pub const OFFICE_PIECES: [(&str, &str); 5] = [
    ("Puestos de trabajo", "Puestos de trabajo"),
    ("Mesa de reuniones", "Mesa de reuniones"),
    ("Mostrador de recepción", "Mostrador de recepción"),
    ("Armario de oficina", "Armario de oficina"),
    ("Mampara acústica", "Mampara acústica"),
];

/// Places a collection's office from `x` along the wall at `y = wall`
/// (looking south): the cabinet on the wall, the reception before it with
/// a screen, the bench of four places with the collection's work chairs,
/// and the meeting table with six of its chairs. Returns where it ends.
fn place_office(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let values: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
    let mut types = Vec::new();
    for (family, label) in OFFICE_PIECES {
        types.push(kitchen_type(m, k, family, label, &values)?);
    }
    let [bench, meeting, desk, cabinet, screen] =
        [types[0], types[1], types[2], types[3], types[4]];
    put(m, cabinet, x + 0.9, wall, pi)?;
    put(m, desk, x + 3.0, wall - 1.0, pi)?;
    put(m, screen, x + 4.6, wall - 1.0, pi / 2.0)?;
    let task = kitchen_type(m, k, "Silla de oficina", "Silla de puesto", &values)?;
    let y0 = wall - 4.6;
    put(m, bench, x + 1.6, y0, 0.0)?;
    for cx in [x + 0.9, x + 2.3] {
        // The work chair faces `-y` as made: turned, it faces the desk.
        put(m, task, cx, y0 - 0.09, pi)?;
        put(m, task, cx, y0 + 1.69, 0.0)?;
    }
    let chair = kitchen_type(m, k, "Silla", "Silla de reuniones", &values)?;
    let t0 = wall - 4.35;
    put(m, meeting, x + 5.8, t0, 0.0)?;
    for cx in [x + 5.05, x + 5.8, x + 6.55] {
        put(m, chair, cx, t0 - 0.47, 0.0)?;
        put(m, chair, cx, t0 + 1.1 + 0.47, pi)?;
    }
    Ok(x + 7.2)
}

/// What the shop of each collection (paquete 7, tanda 7c) has besides
/// its terrace's materials, in the order of [`TERRACES`].
pub const SHOP_EXTRAS: [&[(&str, &str)]; 8] = [
    &[
        ("frame", "Roble claro"),
        ("top", "Roble claro"),
        ("front", "Lacado blanco"),
        ("curtain", "Lino arena"),
    ],
    &[
        ("frame", "Madera encalada"),
        ("top", "Madera encalada"),
        ("masonry", "Cal blanca"),
        ("accent", "Cerámica esmaltada"),
        ("metal", "Hierro forjado"),
        ("curtain", "Lino añil"),
    ],
    &[
        ("frame", "Teca"),
        ("top", "Teca"),
        ("accent", "Lacado azul celeste"),
        ("metal", "Latón"),
        ("curtain", "Terciopelo mostaza"),
    ],
    &[
        ("frame", "Roble ahumado"),
        ("top", "Roble ahumado"),
        ("dark", "Fresno oscuro"),
        ("curtain", "Lino piedra"),
    ],
    &[
        ("front", "Lacado blanco"),
        ("top", "Microcemento"),
        ("dark", "Metal negro"),
        ("light", "Luz cálida"),
        ("curtain", "Tela gris"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("top", "Pino viejo"),
        ("masonry", "Piedra caliza"),
        ("metal", "Hierro forjado"),
        ("curtain", "Lino crudo"),
    ],
    &[
        ("frame", "Pino viejo"),
        ("top", "Roble ahumado"),
        ("dark", "Metal negro"),
        ("front", "Fieltro teja"),
        ("metal", "Acero inoxidable"),
        ("curtain", "Lana carbón"),
    ],
    &[
        ("frame", "Nogal"),
        ("top", "Nogal"),
        ("accent", "Cuero coñac"),
        ("metal", "Latón"),
        ("curtain", "Terciopelo verde botella"),
    ],
];

/// The shop pieces of the workshop: family and the start of their types'
/// names.
pub const SHOP_PIECES: [(&str, &str); 5] = [
    ("Mostrador de tienda", "Mostrador de tienda"),
    ("Estantería de tienda", "Estantería de tienda"),
    ("Mesa expositora", "Mesa expositora"),
    ("Burro de ropa", "Burro de ropa"),
    ("Probador", "Probador"),
];

/// Places a collection's shop from `x` along the wall at `y = wall`
/// (looking south): two shelvings and the fitting room on the wall, the
/// counter, the display table and the clothes rail before them. Returns
/// where it ends.
fn place_shop(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    wall: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let values: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
    let mut types = Vec::new();
    for (family, label) in SHOP_PIECES {
        types.push(kitchen_type(m, k, family, label, &values)?);
    }
    let [counter, shelving, table, rail, fitting] =
        [types[0], types[1], types[2], types[3], types[4]];
    put(m, shelving, x + 0.6, wall, pi)?;
    put(m, shelving, x + 1.8, wall, pi)?;
    put(m, fitting, x + 3.0, wall, pi)?;
    put(m, counter, x + 1.2, wall - 2.2, pi)?;
    put(m, table, x + 3.0, wall - 2.9, 0.0)?;
    put(m, rail, x + 4.7, wall - 2.9, 0.0)?;
    Ok(x + 5.4)
}

/// What the pool of each collection (paquete 8, tanda 8a) is made of and
/// its form (1 rectangular, 2 with Roman steps, 3 infinity edge), in the
/// order of [`TERRACES`].
pub const POOLS: [(&[(&str, &str)], &str); 8] = [
    (
        &[
            ("lining", "Gresite blanco"),
            ("coping", "Hormigón pulido"),
            ("deck", "Roble claro"),
            ("frame", "Roble claro"),
            ("metal", "Acero inoxidable"),
        ],
        "Rectangular",
    ),
    (
        &[
            ("lining", "Gresite azul"),
            ("coping", "Terracota"),
            ("deck", "Terracota"),
            ("accent", "Cerámica esmaltada"),
            ("masonry", "Cal blanca"),
            ("metal", "Acero inoxidable"),
        ],
        "Con escalera romana",
    ),
    (
        &[
            ("lining", "Gresite azul"),
            ("coping", "Terrazo"),
            ("deck", "Terrazo"),
            ("accent", "Gresite blanco"),
            ("metal", "Acero inoxidable"),
            ("deck", "Terrazo"),
        ],
        "Rectangular",
    ),
    (
        &[
            ("lining", "Pizarra"),
            ("coping", "Piedra gris"),
            ("deck", "Roble ahumado"),
            ("frame", "Roble ahumado"),
            ("accent", "Bambú"),
            ("metal", "Metal negro"),
        ],
        "Rectangular",
    ),
    (
        &[
            ("lining", "Gresite blanco"),
            ("coping", "Microcemento"),
            ("deck", "Microcemento"),
            ("metal", "Acero inoxidable"),
        ],
        "Desbordante",
    ),
    (
        &[
            ("lining", "Gresite verde"),
            ("coping", "Piedra caliza"),
            ("deck", "Piedra caliza"),
            ("frame", "Pino viejo"),
            ("masonry", "Piedra gris"),
            ("dark", "Hierro forjado"),
            ("metal", "Hierro forjado"),
        ],
        "Rectangular",
    ),
    (
        &[
            ("lining", "Hormigón pulido"),
            ("coping", "Acero corten"),
            ("deck", "Pino viejo"),
            ("dark", "Metal negro"),
            ("accent", "Latón"),
            ("metal", "Metal negro"),
        ],
        "Rectangular",
    ),
    (
        &[
            ("lining", "Gresite azul"),
            ("coping", "Mármol"),
            ("deck", "Piedra caliza"),
            ("accent", "Cerámica esmaltada"),
            ("metal", "Latón"),
        ],
        "Con escalera romana",
    ),
];

/// Places a collection's pool from `x`, south of the walls (`y` from
/// `front`): the pool with its deck, the shower at its east end and two
/// of the collection's loungers beyond its north edge. Returns where it
/// ends.
fn place_pool(
    m: &mut Model,
    k: &Kitchen,
    (extras, form): (&[(&'static str, &'static str)], &'static str),
    x: f64,
    front: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let (l, w, deck) = (7.0, 3.5, 1.0);
    let mut values: Vec<(&'static str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
    values.extend([
        ("length", N(l)),
        ("width", N(w)),
        ("deck_width", N(deck)),
        ("form", T(form)),
    ]);
    let pool = kitchen_type(m, k, "Piscina", "Piscina", &values)?;
    let shower = kitchen_type(m, k, "Ducha de piscina", "Ducha de piscina", &values)?;
    let lounger = kitchen_type(m, k, "Tumbona", "Tumbona de piscina", &[])?;
    let half = l / 2.0 + 0.32 + deck;
    let cx = x + half;
    let depth = w + 2.0 * (0.32 + deck);
    put(m, pool, cx, front - depth, 0.0)?;
    put(m, shower, cx + half + 0.8, front - depth / 2.0, pi / 2.0)?;
    for dx in [-1.0, 1.0] {
        put(m, lounger, cx + dx, front + 0.3, 0.0)?;
    }
    Ok(cx + half + 1.6)
}

/// A sport piece: its family, its label and its materials besides the
/// recipe's.
pub type SportPiece = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

/// The sport pieces (paquete 8, tanda 8b): one model each, no
/// collections; by family, label and their values besides the recipe's.
pub const SPORT_PIECES: [SportPiece; 11] = [
    ("Canasta de baloncesto", "Canasta de baloncesto", &[]),
    ("Portería", "Portería", &[]),
    ("Mesa de ping-pong", "Mesa de ping-pong", &[]),
    (
        "Mesa de ping-pong",
        "Mesa de ping-pong verde",
        &[("board", "Tablero verde")],
    ),
    ("Cama elástica", "Cama elástica", &[]),
    ("Espaldera", "Espaldera", &[]),
    ("Mancuernero", "Mancuernero", &[]),
    ("Banco de pesas", "Banco de pesas", &[]),
    ("Cinta de correr", "Cinta de correr", &[]),
    ("Bicicleta estática", "Bicicleta estática", &[]),
    ("Esterilla de yoga", "Esterilla de yoga", &[]),
];

/// A type of a piece without collection: «<label> (taller)».
fn sport_type(
    m: &mut Model,
    family_name: &str,
    label: &str,
    values: &[(&str, Val)],
) -> Result<ObjectTypeId, ModelError> {
    let name = format!("{label} (taller)");
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let f = family(m, family_name)?;
    let mut v = m.family_defaults(f, ParamScope::Type)?;
    for (key, val) in values {
        let val = match val {
            N(n) => Value::Number(*n),
            M(n) => match palette_or_project(m, n) {
                Some(id) => Value::Material(id.0),
                None => continue,
            },
            T(t) => Value::Text((*t).into()),
            B(b) => Value::Bool(*b),
        };
        v.insert((*key).into(), val);
    }
    m.add_object_type(&name, f, v)
}

/// The sport at home and the gym (paquete 8, tanda 8b), east of `x`
/// (looking south): in the garden, the hoop and the goal on the line of
/// the walls, two table tennis tables and the trampoline; then the gym
/// on its wall: wall bars, dumbbells, the bench under its rack, the
/// treadmill and the bike facing the wall, and the yoga mat. Returns
/// where it ends (the gym's wall is put by the caller).
fn place_sport(m: &mut Model, x: f64, wall: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64, turn: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: turn,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let pi = std::f64::consts::PI;
    let mut t = Vec::new();
    for (family, label, extras) in SPORT_PIECES {
        let values: Vec<(&str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
        t.push(sport_type(m, family, label, &values)?);
    }
    // The garden.
    put(m, t[0], x + 0.6, wall, pi)?;
    put(m, t[1], x + 3.2, wall, pi)?;
    put(m, t[2], x + 3.2, wall - 4.0, 0.0)?;
    put(m, t[3], x + 3.2, wall - 6.2, 0.0)?;
    put(m, t[4], x + 7.0, wall - 3.6, 0.0)?;
    // The gym.
    let g = x + 10.0;
    put(m, t[5], g + 0.9, wall, pi)?;
    put(m, t[6], g + 2.6, wall, pi)?;
    put(m, t[7], g + 4.4, wall, pi)?;
    put(m, t[8], g + 6.2, wall - 1.9, 0.0)?;
    put(m, t[9], g + 7.3, wall - 1.07, 0.0)?;
    put(m, t[10], g + 2.6, wall - 2.8, 0.0)?;
    Ok(g + 7.8)
}

/// The courts, the grandstand, the playground and calisthenics (paquete 8,
/// tanda 8c): one model each, no collections.
pub const PARK_PIECES: [SportPiece; 13] = [
    ("Pista de pádel", "Pista de pádel", &[]),
    ("Pista de tenis", "Pista de tenis", &[]),
    ("Pista de baloncesto", "Pista de baloncesto", &[]),
    ("Grada", "Grada", &[]),
    ("Columpio", "Columpio", &[]),
    ("Tobogán", "Tobogán", &[]),
    ("Balancín", "Balancín", &[]),
    ("Balancín de muelle", "Balancín de muelle", &[]),
    ("Estación de calistenia", "Estación de calistenia", &[]),
    ("Elíptica de parque", "Elíptica de parque", &[]),
    ("Pista de fútbol sala", "Pista de fútbol sala", &[]),
    ("Campo de fútbol", "Campo de fútbol 11", &[]),
    ("Campo de fútbol", "Campo de fútbol 7", &[]),
];

/// The courts in a row south of the line of the walls (`wall`), east of
/// `x`, the grandstand before the tennis court; then the playground and
/// the calisthenics station. Returns where it ends.
fn place_park(m: &mut Model, x: f64, wall: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let mut t = Vec::new();
    for (family, label, extras) in PARK_PIECES {
        let values: Vec<(&str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
        let mut values = values;
        if label.ends_with(" 7") {
            values.push(("game", T("Fútbol 7")));
        }
        t.push(sport_type(m, family, label, &values)?);
    }
    // Each court's far side 2 m short of the line of the walls.
    let edge = wall - 2.0;
    let padel = x + 10.5;
    put(m, t[0], padel, edge - 10.5)?;
    let tennis = padel + 10.5 + 2.0 + 18.29;
    put(m, t[1], tennis, edge - 18.29)?;
    put(m, t[3], tennis, edge - 18.29 - 1.0 - 3.2)?;
    let basket = tennis + 18.29 + 2.0 + 16.0;
    put(m, t[2], basket, edge - 19.0)?;
    let futsal = basket + 16.0 + 2.0 + 22.0;
    put(m, t[10], futsal, edge - 24.0)?;
    let eleven = futsal + 22.0 + 2.0 + 56.5;
    put(m, t[11], eleven, edge - 76.0)?;
    let seven = eleven + 56.5 + 2.0 + 35.5;
    put(m, t[12], seven, edge - 51.0)?;
    // The playground, its far sides 1 m short of the line.
    let p = seven + 35.5 + 2.0;
    let far = wall - 1.0;
    for (i, (dx, depth)) in [
        (1.9, 1.6),
        (5.0, 5.4),
        (7.6, 0.6),
        (9.6, 0.8),
        (15.2, 2.4),
        (11.0, 1.0),
    ]
    .into_iter()
    .enumerate()
    {
        let k = [4, 5, 6, 7, 8, 9][i];
        put(m, t[k], p + dx, far - depth)?;
    }
    Ok(p + 19.0)
}

/// What the street furniture of each collection (paquete 5, tanda 5b)
/// has besides its terrace's materials, in the order of [`TERRACES`].
pub const STREET_EXTRAS: [&[(&str, &str)]; 8] = [
    &[("mat", "Microcemento"), ("top", "Piedra gris")],
    &[
        ("mat", "Piedra caliza"),
        ("accent", "Cerámica esmaltada"),
        ("top", "Piedra caliza"),
    ],
    &[("mat", "Terrazo"), ("accent", "Cerámica turquesa")],
    &[("mat", "Piedra gris"), ("top", "Piedra gris")],
    &[("mat", "Hormigón pulido"), ("top", "Piedra gris")],
    &[("mat", "Piedra gris"), ("top", "Piedra gris")],
    &[("mat", "Acero corten"), ("dark", "Metal negro")],
    &[("mat", "Piedra caliza"), ("top", "Piedra caliza")],
];

/// The street furniture's types of a collection, by the label of each
/// (paquete 5, tanda 5b).
pub const STREET_PIECES: [(&str, &str); 7] = [
    ("Banco", "Banco de calle"),
    ("Farola", "Farola"),
    ("Papelera", "Papelera"),
    ("Bolardo", "Bolardo"),
    ("Jardinera", "Jardinera"),
    ("Aparcabicis", "Aparcabicis"),
    ("Fuente de beber", "Fuente de beber"),
];

/// Places a collection's street furniture from `x` along `y = front`
/// (looking south, the street before it): a bench, a streetlight, a
/// litter bin, two bollards, a planter, a bike rack and a drinking
/// fountain. Returns where it ends.
fn place_street(
    m: &mut Model,
    k: &Kitchen,
    extras: &[(&'static str, &'static str)],
    x: f64,
    front: f64,
) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let put = |m: &mut Model, t: ObjectTypeId, px: f64, py: f64| {
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(px, py),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })
        .map(|_| ())
    };
    let mut values: Vec<(&str, Val)> = extras.iter().map(|(k, v)| (*k, M(v))).collect();
    values.push(("light", M("Luz de farola")));
    values.push(("water", M("Agua de fuente")));
    // Turned to face the street, a bike rack runs west from its first
    // stand: that is at its east end.
    let at = [1.0, 2.6, 3.4, 4.0, 5.6, 8.6, 9.6];
    for (i, (family_name, label)) in STREET_PIECES.iter().enumerate() {
        let mut v: Vec<(&str, Val)> = values.iter().map(|(k, v)| (*k, v.copy())).collect();
        if *label == "Aparcabicis" {
            v.push(("count", N(3.0)));
        }
        let t = kitchen_type(m, k, family_name, label, &v)?;
        put(m, t, x + at[i], front)?;
        if *label == "Bolardo" {
            put(m, t, x + at[i] + 0.5, front)?;
        }
    }
    Ok(x + 10.2)
}

/// The pieces of a workshop bathroom, left to right along its wall:
/// family, label, how far it stands from the one before (m) and its
/// values. Mirror and towel rail hang over the washbasin's place.
const SUITE: &[(&str, &str, f64, Values)] = &[
    (
        "Bañera",
        "Bañera",
        0.0,
        &[("width", N(1.7)), ("depth", N(0.75)), ("height", N(0.55))],
    ),
    ("Inodoro", "Inodoro", 0.3, &[]),
    ("Bidé", "Bidé", 0.3, &[]),
    (
        "Lavabo",
        "Lavabo",
        0.35,
        &[("width", N(0.8)), ("depth", N(0.48))],
    ),
    (
        "Toallero",
        "Toallero",
        0.25,
        &[
            ("width", N(0.5)),
            ("elevation", N(1.15)),
            ("height", N(0.8)),
        ],
    ),
    (
        "Plato de ducha",
        "Ducha",
        0.3,
        &[("width", N(0.9)), ("depth", N(0.9))],
    ),
];

/// Places a bathroom with its back on a wall along `y = wall` from `x`
/// (looking south); returns where it ends.
fn place_bathroom(m: &mut Model, k: &Kitchen, x: f64, wall: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let mut at = x;
    let mut basin = None;
    for (family_name, label, gap, values) in SUITE {
        let mut vals: Vec<(&str, Val)> = Vec::new();
        for (key, v) in values.iter() {
            vals.push((
                key,
                match v {
                    N(n) => N(*n),
                    M(n) => M(n),
                    T(t) => T(t),
                    B(b) => B(*b),
                },
            ));
        }
        let t = kitchen_type(m, k, family_name, label, &vals)?;
        let w = m.object_type_box(t).map_or(0.6, |(lo, hi)| hi[0] - lo[0]);
        at += gap;
        let cx = at + w / 2.0;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(cx, wall),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
        if *label == "Lavabo" {
            basin = Some(cx);
        }
        at += w;
    }
    // The mirror over the washbasin.
    if let Some(cx) = basin {
        let t = kitchen_type(
            m,
            k,
            "Espejo",
            "Espejo",
            &[
                ("width", N(0.6)),
                ("height", N(0.85)),
                ("elevation", N(1.15)),
            ],
        )?;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(cx, wall),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
    }
    Ok(at)
}

/// The modules of a workshop kitchen, left to right: family, width and
/// what it holds (values by name).
/// A value of a type by its parameter's name.
type Values = &'static [(&'static str, Val)];

const RUN: &[(&str, f64, Values)] = &[
    (
        "Columna de cocina",
        0.6,
        &[("tall", T("Frigorífico integrado"))],
    ),
    (
        "Módulo bajo de cocina",
        0.6,
        &[("unit", T("Cajonera")), ("drawers", N(3.0))],
    ),
    (
        "Módulo bajo de cocina",
        0.8,
        &[("unit", T("Fregadero")), ("doors", N(2.0))],
    ),
    ("Módulo bajo de cocina", 0.6, &[("unit", T("Lavavajillas"))]),
    (
        "Módulo bajo de cocina",
        0.8,
        &[("unit", T("Placa")), ("drawers", N(2.0))],
    ),
    ("Módulo bajo de cocina", 0.6, &[("unit", T("Horno"))]),
];

/// A type of a workshop kitchen's family in its collection, with
/// `values` over it (found if already made).
fn kitchen_type(
    m: &mut Model,
    k: &Kitchen,
    family_name: &str,
    label: &str,
    values: &[(&str, Val)],
) -> Result<ObjectTypeId, ModelError> {
    let name = format!("{label} · {} (taller)", k.collection);
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let f = family(m, family_name)?;
    let mut v = m.family_defaults(f, ParamScope::Type)?;
    v.insert("model3d".into(), Value::Text(k.collection.into()));
    for (key, mat) in k.materials {
        // Only what this family has (a bed has no worktop).
        if !v.contains_key(*key) {
            continue;
        }
        if let Some(id) = crate::palette_material(m, mat) {
            v.insert((*key).into(), Value::Material(id.0));
        }
    }
    for (key, val) in values {
        // Shared lists of values: only what this family has.
        if !v.contains_key(*key) {
            continue;
        }
        let val = match val {
            N(n) => Value::Number(*n),
            M(n) => match palette_or_project(m, n) {
                Some(id) => Value::Material(id.0),
                None => continue,
            },
            T(t) => Value::Text((*t).into()),
            B(b) => Value::Bool(*b),
        };
        v.insert((*key).into(), val);
    }
    m.add_object_type(&name, f, v)
}

/// Places a kitchen with its back on a wall along `y = wall` from `x`
/// (looking south): its run of modules, its wall units and its hood.
/// Returns where it ends.
fn place_kitchen(m: &mut Model, k: &Kitchen, x: f64, wall: f64) -> Result<f64, ModelError> {
    let level = m.levels()[0].id;
    let mut at = x;
    let mut hob_at = None;
    for (family_name, w, values) in RUN {
        let label = values
            .iter()
            .find_map(|(key, v)| match v {
                T(t) if *key == "unit" || *key == "tall" => Some(*t),
                _ => None,
            })
            .unwrap_or("Módulo");
        let mut vals: Vec<(&str, Val)> = vec![("width", N(*w))];
        for (key, v) in values.iter() {
            vals.push((
                key,
                match v {
                    N(n) => N(*n),
                    M(n) => M(n),
                    T(t) => T(t),
                    B(b) => B(*b),
                },
            ));
        }
        let t = kitchen_type(
            m,
            k,
            family_name,
            &format!("{label} {:.0}", w * 100.0),
            &vals,
        )?;
        let cx = at + w / 2.0;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(cx, wall),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
        if label == "Placa" {
            hob_at = Some((cx, *w));
        }
        at += w;
    }
    // Wall units over the modules from the column to the hob, and over
    // the oven; the hood over the hob.
    let (hx, hw) = hob_at.unwrap_or((at - 0.5, 0.8));
    let left = (x + 0.6, hx - hw / 2.0);
    let right = (hx + hw / 2.0, at);
    for (a, e) in [left, right] {
        let w = e - a;
        if w < 0.3 {
            continue;
        }
        let doors = (w / 0.5).round().max(1.0);
        let t = kitchen_type(
            m,
            k,
            "Mueble alto de cocina",
            &format!("Altos {:.0}", w * 100.0),
            &[
                ("width", N(w)),
                ("doors", N(doors)),
                ("elevation", N(1.45)),
                ("height", N(0.7)),
            ],
        )?;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new((a + e) / 2.0, wall),
            rotation: std::f64::consts::PI,
            base_offset: 0.0,
        })?;
    }
    let built = k.collection == "Japandi";
    let t = kitchen_type(
        m,
        k,
        "Campana extractora",
        "Campana",
        &[
            ("width", N(if built { hw } else { 0.9 })),
            ("depth", N(if built { 0.35 } else { 0.5 })),
            ("elevation", N(if built { 1.45 } else { 1.55 })),
            ("height", N(if built { 0.7 } else { 1.05 })),
        ],
    )?;
    m.add_object(NewObject {
        object_type: t,
        level,
        position: Point2::new(hx, wall),
        rotation: std::f64::consts::PI,
        base_offset: 0.0,
    })?;
    Ok(at)
}

/// The project's copy of a workshop family, loading it if needed.
pub fn family(m: &mut Model, name: &str) -> Result<archisim_core::FamilyId, ModelError> {
    if let Some(f) = m.family_by_name(name) {
        return Ok(f.id);
    }
    // The packages Antonio approved are system families now.
    let Some(source) = RECIPES.iter().find(|r| {
        r.lines()
            .next()
            .is_some_and(|l| l.trim() == format!("family \"{name}\""))
    }) else {
        return m.system_family(name);
    };
    m.add_family(source)
}

/// Adds a workshop piece's type to the project (or finds it).
pub fn add_piece(m: &mut Model, piece: &Piece) -> Result<ObjectTypeId, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == piece.name) {
        return Ok(t.id);
    }
    // A plant grown by ArchiSim: the library's species, with the
    // workshop's values over it.
    let (f, mut values) = match piece.family.strip_prefix("flora:") {
        Some(species) => {
            let t = crate::add_object_entry(m, species)?;
            let ty = m
                .object_type(t)
                .ok_or_else(|| ModelError::NotAnObject(species.to_string()))?;
            (ty.family, ty.values.clone())
        }
        None => {
            let f = family(m, piece.family)?;
            (f, m.family_defaults(f, ParamScope::Type)?)
        }
    };
    for (k, v) in piece.values {
        let v = match v {
            N(n) => Value::Number(*n),
            M(name) => match palette_or_project(m, name) {
                Some(id) => Value::Material(id.0),
                None => continue,
            },
            T(name) => Value::Text((*name).into()),
            B(b) => Value::Bool(*b),
        };
        values.insert((*k).into(), v);
    }
    m.add_object_type(piece.name, f, values)
}

/// The workshop project: in the front row the new pieces, behind each one
/// the piece of the library it is to replace, all looking south, and a
/// partition wall behind them all, as the wall of a living room.
pub fn project() -> Result<Model, ModelError> {
    let mut m = crate::new_project("Taller de modelos");
    let level = m.levels()[0].id;
    let mut x = 0.0;
    // The wall runs behind the furniture, not through the trees.
    let mut wall_end = 0.0;
    for piece in PIECES {
        let tree = piece.family.starts_with("flora:");
        let new = add_piece(&mut m, piece)?;
        let old = match piece.replaces {
            Some(name) => Some(crate::add_object_entry(&mut m, name)?),
            None => None,
        };
        let width = |m: &Model, t: ObjectTypeId| {
            m.object_type_box(t).map(|b| b.1[0] - b.0[0]).unwrap_or(1.0)
        };
        let w = old.map_or(width(&m, new), |o| width(&m, new).max(width(&m, o)));
        // A tree's crown is wide: the old one well behind.
        let behind = if tree { w + 1.0 } else { 2.0 };
        for (t, y) in [(Some(new), 0.0), (old, behind)] {
            if let Some(t) = t {
                m.add_object(NewObject {
                    object_type: t,
                    level,
                    position: Point2::new(x + w / 2.0, y),
                    rotation: std::f64::consts::PI,
                    base_offset: 0.0,
                })?;
            }
        }
        x += w + 1.2;
        if !tree {
            wall_end = x;
        }
    }
    let partition = m
        .wall_types()
        .find(|t| t.name.contains("Tabique"))
        .or_else(|| m.wall_types().next())
        .map(|t| t.id)
        .ok_or_else(|| ModelError::NotAnObject("Tabique".into()))?;
    m.add_wall(NewWall {
        wall_type: partition,
        level,
        start: Point2::new(-0.6, 2.3),
        end: Point2::new(wall_end - 0.6, 2.3),
        reference: ReferenceLine::InteriorFace,
        height: 2.6,
        base_offset: 0.0,
        bulge: 0.0,
        top: WallTop::Unconnected,
    })?;
    // The kitchens, east of the pieces, each on its own wall.
    let mut x = wall_end + 2.0;
    let wall = 1.2;
    for k in KITCHENS {
        let end = place_kitchen(&mut m, k, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The bedrooms, further east.
    for k in BEDROOMS {
        let end = place_bedroom(&mut m, k, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The lamps, further east, each collection's on its wall.
    for (k, room) in LIGHTS.iter().zip(BEDROOMS) {
        let end = place_lights(&mut m, k, room, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The terraces, further east, without walls.
    for k in TERRACES {
        x = place_terrace(&mut m, k, x, wall)? + 1.8;
    }
    // The pools, further east, without walls (south of the line of the
    // walls).
    for (k, pool) in TERRACES.iter().zip(POOLS) {
        x = place_pool(&mut m, k, pool, x, wall - 1.0)? + 1.8;
    }
    // The sport at home and the gym, further east, the gym on its wall.
    {
        let end = place_sport(&mut m, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x + 9.7, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The courts, the playground and calisthenics, further east.
    x = place_park(&mut m, x, wall)? + 1.8;
    // The street furniture, further east.
    for (k, extras) in TERRACES.iter().zip(STREET_EXTRAS) {
        x = place_street(&mut m, k, extras, x, wall)? + 1.8;
    }
    // The studies, further east, each on its wall.
    for (k, extras) in TERRACES.iter().zip(STUDY_EXTRAS) {
        let end = place_study(&mut m, k, extras, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The bars, further east, each on its wall.
    for (k, extras) in TERRACES.iter().zip(BAR_EXTRAS) {
        let end = place_bar(&mut m, k, extras, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The offices, further east, each on its wall.
    for (k, extras) in TERRACES.iter().zip(OFFICE_EXTRAS) {
        let end = place_office(&mut m, k, extras, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The shops, further east, each on its wall.
    for (k, extras) in TERRACES.iter().zip(SHOP_EXTRAS) {
        let end = place_shop(&mut m, k, extras, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The living room corners, further east, each on its wall.
    for (k, extras) in TERRACES.iter().zip(INDOOR_EXTRAS) {
        let end = place_living(&mut m, k, extras, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The bathrooms, further east.
    for k in BATHROOMS {
        let end = place_bathroom(&mut m, k, x, wall)?;
        m.add_wall(NewWall {
            wall_type: partition,
            level,
            start: Point2::new(x - 0.3, wall),
            end: Point2::new(end + 0.3, wall),
            reference: ReferenceLine::InteriorFace,
            height: 2.6,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })?;
        x = end + 1.8;
    }
    // The doors (paquete 9), south of everything.
    place_doors(&mut m, 0.0, -9.0)?;
    place_windows(&mut m, 0.0, -26.0)?;
    place_railings(&mut m, 0.0, -56.0)?;
    Ok(m)
}

/// The railings (E12) from `x` along `y`: one of each type of the
/// template, each an L (3 m and a 1,2 m return) so that its free ends,
/// its corner and its bays show; 1,10 m high, posts every 1,5 m at most.
pub fn place_railings(
    m: &mut Model,
    x: f64,
    y: f64,
) -> Result<Vec<archisim_core::RailingId>, ModelError> {
    let types = crate::railings::add_railing_types(m)?;
    let level = m.levels()[0].id;
    let material = m
        .materials()
        .find(|x| x.category == "steel" || x.category == "metal")
        .or_else(|| m.materials().next())
        .map(|x| x.id)
        .ok_or_else(|| ModelError::NotAnObject("material".into()))?;
    let mut out = Vec::new();
    for (i, t) in types.into_iter().enumerate() {
        let x0 = x + i as f64 * 4.5;
        out.push(m.add_railing(archisim_core::NewRailing {
            level,
            path: vec![
                Point2::new(x0, y + 1.2),
                Point2::new(x0, y),
                Point2::new(x0 + 3.0, y),
            ],
            base_offset: 0.0,
            height: 1.1,
            post_spacing: 1.5,
            material,
            glass: None,
            railing_type: Some(t),
        })?);
    }
    Ok(out)
}

/// How near two pieces must be to count as touching: closer than this a
/// gap does not show.
pub const TOUCH: f64 = 0.004;

/// The pieces in the air of every model of the workshop: for each type,
/// its loose pieces (role and box). See [`archisim_core::props::check`].
pub fn loose_report(m: &Model) -> Vec<(String, Vec<archisim_core::props::check::Loose>)> {
    let mut out = Vec::new();
    for t in m.object_types() {
        if !m.objects().any(|o| o.object_type == t.id) {
            continue;
        }
        let (Some((kind, _)), Some((mesh, _))) = (m.prop_params(t.id), m.type_prop(t.id)) else {
            continue;
        };
        let loose = archisim_core::props::check::loose_pieces(
            &mesh,
            archisim_core::props::check::anchors_for(&kind),
            TOUCH,
        );
        if !loose.is_empty() {
            out.push((format!("{} [{kind}]", t.name), loose));
        }
    }
    out
}

/// A material of the palette by name or, if the palette has none of that
/// name, the project's (the template's «Metal lacado»).
fn palette_or_project(m: &mut Model, name: &str) -> Option<archisim_core::MaterialId> {
    crate::palette_material(m, name)
        .or_else(|| m.materials().find(|x| x.name == name).map(|x| x.id))
}

/// The doors of the workshop (paquete 9): one of each collection in a
/// partition, the hands alternating, and the entrance doors in a façade.
pub const DOOR_COLLECTIONS: [&str; 8] = archisim_core::props::fine::door::COLLECTIONS;

/// Places the workshop's doors from `x` along a partition at `y`, three
/// entrance doors in a façade 4 m south, and doors for hospitals, industry
/// and security (fire doors, vision panels, a security door) in another
/// partition 4 m north; the end of the partition.
pub fn place_doors(m: &mut Model, x: f64, y: f64) -> Result<f64, ModelError> {
    use archisim_core::{Hand, NewFamilyType, NewOpening, WallSide};
    let level = m.levels()[0].id;
    let family = m.system_family("Puerta abatible")?;
    let wall_type = |m: &Model, name: &str| {
        m.wall_types()
            .find(|t| t.name.contains(name))
            .or_else(|| m.wall_types().next())
            .map(|t| t.id)
            .ok_or_else(|| ModelError::NotAnObject(name.into()))
    };
    let partition = wall_type(m, "Tabique")?;
    let facade = wall_type(m, "Fachada")?;
    let step = 1.6;
    let end = x + step * DOOR_COLLECTIONS.len() as f64 + 0.4;
    let wall = |m: &mut Model, t, y: f64, end: f64| {
        m.add_wall(NewWall {
            wall_type: t,
            level,
            start: Point2::new(x, y),
            end: Point2::new(end, y),
            reference: ReferenceLine::Center,
            height: 2.7,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
    };
    let inner = wall(m, partition, y, end)?;
    let door_type = |m: &mut Model, name: String, set: &[(&str, Value)]| {
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        for (k, v) in set {
            values.insert((*k).to_string(), v.clone());
        }
        m.add_family_type(NewFamilyType {
            family,
            name,
            mark: None,
            description: String::new(),
            values,
        })
    };
    // Each collection's woods and metal: leaf, frame and hardware.
    let looks: [(&str, &str, &str); 8] = [
        ("Lacado blanco", "Lacado blanco", "Metal negro"),
        ("Madera encalada", "Madera encalada", "Hierro forjado"),
        ("Teca", "Teca", "Latón"),
        ("Roble claro", "Roble claro", "Metal negro"),
        ("Lacado blanco", "Lacado blanco", "Acero inoxidable"),
        ("Pino viejo", "Pino viejo", "Hierro forjado"),
        ("Roble ahumado", "Metal negro", "Metal negro"),
        ("Lacado marfil", "Lacado marfil", "Latón"),
    ];
    let mats = |m: &mut Model, i: usize| -> Vec<(&'static str, Value)> {
        let (leaf, frame, metal) = looks[i % 8];
        [
            ("panel_material", leaf),
            ("frame_material", frame),
            ("hardware_material", metal),
        ]
        .into_iter()
        .filter_map(|(k, n)| palette_or_project(m, n).map(|id| (k, Value::Material(id.0))))
        .collect()
    };
    for (i, c) in DOOR_COLLECTIONS.iter().enumerate() {
        let mut set = vec![
            ("model3d", Value::Text((*c).to_string())),
            ("width", Value::Number(0.825)),
        ];
        set.extend(mats(m, i));
        let t = door_type(m, format!("Puerta {c} (taller)"), &set)?;
        m.add_opening(NewOpening {
            opening_type: t,
            wall: inner,
            along: 1.0 + step * i as f64,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: if i % 2 == 0 { Hand::Left } else { Hand::Right },
            setback: None,
        })?;
    }
    // The entrance doors, in a façade.
    let outer = wall(m, facade, y - 4.0, x + 6.0)?;
    let entrances: [(&str, &str, f64, f64, bool); 3] = [
        ("Clásica", "Seis cuarterones", 1.45, 2.0, true),
        ("Mediterráneo", "Media vidriera", 1.0, 1.0, true),
        ("Rústica", "Tablas", 1.0, 1.0, false),
    ];
    for (i, (c, design, width, leaves, peephole)) in entrances.iter().enumerate() {
        let mut set = vec![
            ("model3d", Value::Text((*c).to_string())),
            ("leaf_design", Value::Text((*design).to_string())),
            ("width", Value::Number(*width)),
            ("height", Value::Number(2.2)),
            ("leaves", Value::Number(*leaves)),
            ("peephole", Value::Bool(*peephole)),
        ];
        let k = DOOR_COLLECTIONS.iter().position(|x| x == c).unwrap_or(0);
        set.extend(mats(m, k));
        let t = door_type(m, format!("Puerta de entrada {c} (taller)"), &set)?;
        m.add_opening(NewOpening {
            opening_type: t,
            wall: outer,
            along: 1.2 + 2.0 * i as f64,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Right,
            setback: None,
        })?;
    }
    // Doors for hospitals, industry and security, in a partition 4 m
    // north: what each is, its leaf, handle and extras.
    let special = wall(m, partition, y + 4.0, end)?;
    let t = |s: &str| Value::Text(s.to_string());
    let b = Value::Bool;
    type Special<'a> = (&'a str, f64, f64, Vec<(&'a str, Value)>, [&'a str; 3]);
    let specials: [Special; 6] = [
        (
            "cortafuegos EI2 60 con ojo de buey",
            0.9,
            1.0,
            vec![
                ("leaf_design", t("Chapa con ojo de buey")),
                ("handle_style", t("Barra antipánico")),
                ("closer", b(true)),
                ("fire_rating", t("EI2 60-C5")),
            ],
            ["Metal lacado", "Metal lacado", "Acero inoxidable"],
        ),
        (
            "de hospital con ventanilla",
            1.05,
            1.0,
            vec![
                ("leaf_design", t("Lisa con ventanilla cuadrada")),
                ("handle_style", t("Placa de empuje y tirador")),
                ("kick_plate", b(true)),
                ("closer", b(true)),
            ],
            ["Lacado blanco", "Lacado blanco", "Acero inoxidable"],
        ),
        (
            "de quirófano de dos hojas",
            1.6,
            2.0,
            vec![
                ("leaf_design", t("Chapa con ojo de buey")),
                ("handle_style", t("Placa de empuje y tirador")),
                ("kick_plate", b(true)),
            ],
            ["Lacado blanco", "Acero inoxidable", "Acero inoxidable"],
        ),
        (
            "metálica embutida",
            0.9,
            1.0,
            vec![
                ("leaf_design", t("Chapa embutida")),
                ("handle_style", t("Palanca con placa")),
            ],
            ["Metal lacado", "Metal lacado", "Metal negro"],
        ),
        (
            "cortafuegos de dos hojas",
            1.6,
            2.0,
            vec![
                ("leaf_design", t("Chapa con mirilla alargada")),
                ("handle_style", t("Barra antipánico")),
                ("closer", b(true)),
                ("fire_rating", t("EI2 90-C5")),
            ],
            ["Chapa teja", "Metal lacado", "Metal negro"],
        ),
        (
            "acorazada",
            0.9,
            1.0,
            vec![
                ("leaf_design", t("Acorazada")),
                ("handle_style", t("Escudo de seguridad")),
                ("peephole", b(true)),
            ],
            ["Roble ahumado", "Roble ahumado", "Latón"],
        ),
    ];
    let mut along = 1.0;
    for (name, width, leaves, extra, [leaf, frame, metal]) in specials {
        let mut set = vec![
            ("model3d", t("Contemporáneo")),
            ("width", Value::Number(width)),
            ("height", Value::Number(2.1)),
            ("leaves", Value::Number(leaves)),
        ];
        set.extend(
            [
                ("panel_material", leaf),
                ("frame_material", frame),
                ("hardware_material", metal),
            ]
            .into_iter()
            .filter_map(|(k, n)| palette_or_project(m, n).map(|id| (k, Value::Material(id.0)))),
        );
        set.extend(extra);
        let ty = door_type(m, format!("Puerta {name} (taller)"), &set)?;
        along += width / 2.0;
        m.add_opening(NewOpening {
            opening_type: ty,
            wall: special,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
        along += width / 2.0 + 0.6;
    }
    // Sliding doors, 4 m further north: one over the wall of each
    // collection, and two into it.
    let sliding = m.system_family("Puerta corredera")?;
    let pocket = m.system_family("Puerta corredera empotrada")?;
    let row = wall(m, partition, y + 8.0, end + 8.0)?;
    let mut along = 1.2;
    let kinds: Vec<(archisim_core::FamilyId, String, usize, f64)> = DOOR_COLLECTIONS
        .iter()
        .enumerate()
        .map(|(i, c)| (sliding, format!("Puerta corredera {c} (taller)"), i, 0.0))
        .chain([
            (
                pocket,
                "Puerta empotrada Japandi (taller)".to_string(),
                3,
                0.0,
            ),
            (
                pocket,
                "Puerta empotrada Clásica a medio abrir (taller)".to_string(),
                7,
                50.0,
            ),
        ])
        .collect();
    for (family, name, k, open) in kinds {
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        values.insert("model3d".into(), t(DOOR_COLLECTIONS[k]));
        values.insert("width".into(), Value::Number(0.825));
        let (leaf, frame, metal) = looks[k];
        for (key, n) in [
            ("panel_material", leaf),
            ("frame_material", frame),
            ("hardware_material", metal),
        ] {
            if let Some(id) = palette_or_project(m, n) {
                values.insert(key.into(), Value::Material(id.0));
            }
        }
        let ty = m.add_family_type(NewFamilyType {
            family,
            name,
            mark: None,
            description: String::new(),
            values,
        })?;
        let o = m.add_opening(NewOpening {
            opening_type: ty,
            wall: row,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Right,
            setback: None,
        })?;
        if open > 0.0 {
            let mut values = m.opening(o).map(|x| x.values.clone()).unwrap_or_default();
            values.insert("slide_open".into(), Value::Number(open));
            m.set_opening_values(o, values)?;
        }
        // Room for the leaf to slide beside the opening.
        along += 2.1;
    }
    // Pivot, folding and garage doors, 4 m further north.
    let row = wall(m, facade, y + 12.0, x + 26.0)?;
    let pivot = m.system_family("Puerta pivotante")?;
    let folding = m.system_family("Puerta plegable")?;
    let garage = m.system_family("Puerta de garaje")?;
    type Other<'a> = (
        archisim_core::FamilyId,
        &'a str,
        f64,
        Vec<(&'a str, Value)>,
        &'a str,
        f64,
    );
    let others: Vec<Other> = vec![
        (
            pivot,
            "pivotante Contemporáneo",
            1.2,
            vec![("model3d", t("Contemporáneo"))],
            "",
            0.0,
        ),
        (
            pivot,
            "pivotante Nórdico de tablas",
            1.4,
            vec![
                ("model3d", t("Nórdico cálido")),
                ("leaf_design", t("Tablas")),
            ],
            "",
            0.0,
        ),
        (
            folding,
            "plegable Mediterráneo a medio abrir",
            1.6,
            vec![
                ("model3d", t("Mediterráneo")),
                ("leaves", Value::Number(4.0)),
            ],
            "fold_open",
            45.0,
        ),
        (
            folding,
            "plegable Japandi de seis hojas",
            2.4,
            vec![("model3d", t("Japandi")), ("leaves", Value::Number(6.0))],
            "",
            0.0,
        ),
        (garage, "de garaje seccional", 2.5, vec![], "", 0.0),
        (
            garage,
            "de garaje seccional con ventanas y puerta peatonal",
            3.0,
            vec![
                ("garage_look", t("Con ventanas")),
                ("wicket", Value::Bool(true)),
                ("panels", Value::Number(5.0)),
            ],
            "",
            0.0,
        ),
        (
            garage,
            "de garaje basculante a medio abrir",
            2.5,
            vec![("kind", t("basculante"))],
            "garage_open",
            40.0,
        ),
        (
            garage,
            "de garaje enrollable",
            2.5,
            vec![("kind", t("enrollable"))],
            "garage_open",
            30.0,
        ),
    ];
    let mut along = 1.0;
    for (family, name, width, extra, open_key, open) in others {
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        values.insert("width".into(), Value::Number(width));
        for (k, v) in extra {
            values.insert(k.into(), v);
        }
        let ty = m.add_family_type(NewFamilyType {
            family,
            name: format!("Puerta {name} (taller)"),
            mark: None,
            description: String::new(),
            values,
        })?;
        along += width / 2.0;
        let o = m.add_opening(NewOpening {
            opening_type: ty,
            wall: row,
            along,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
        if open > 0.0 {
            let mut values = m.opening(o).map(|x| x.values.clone()).unwrap_or_default();
            values.insert(open_key.into(), Value::Number(open));
            m.set_opening_values(o, values)?;
        }
        along += width / 2.0 + 0.8;
    }
    // Arches and shaped openings, 4 m further north, in a thick façade.
    place_arches(m, x, y + 16.0, facade)?;
    Ok(end)
}

/// Arched doors and windows, an oculus and shaped voids along a façade
/// at `y` (the workshop's arches row).
fn place_arches(
    m: &mut Model,
    x: f64,
    y: f64,
    facade: archisim_core::WallTypeId,
) -> Result<(), ModelError> {
    use archisim_core::{Hand, NewFamilyType, NewOpening, WallSide};
    let level = m.levels()[0].id;
    let row = m.add_wall(NewWall {
        wall_type: facade,
        level,
        start: Point2::new(x, y),
        end: Point2::new(x + 30.0, y),
        reference: ReferenceLine::Center,
        height: 4.0,
        base_offset: 0.0,
        bulge: 0.0,
        top: WallTop::Unconnected,
    })?;
    let t = |s: &str| Value::Text(s.into());
    let num = Value::Number;
    let door = m.system_family("Puerta en arco")?;
    let window = m.system_family("Ventana en arco")?;
    let void = m.system_family("Hueco")?;
    let white = palette_or_project(m, "Lacado blanco");
    let stone = palette_or_project(m, "Piedra caliza");
    type Arch<'a> = (archisim_core::FamilyId, &'a str, f64, Vec<(&'a str, Value)>);
    let mut items: Vec<Arch> = vec![
        (
            door,
            "Puerta en arco con fijos, como la de la foto",
            2.4,
            vec![
                ("width", num(2.4)),
                ("height", num(3.0)),
                ("layout", t("Puerta con fijos en arco")),
                ("leaves", num(1.0)),
                ("leaf_style", t("Vidriera")),
                ("bars", t("Cuarterones")),
                ("door_width", num(0.9)),
                ("door_height", num(2.2)),
            ],
        ),
        (
            door,
            "Puerta en arco de dos hojas con recercado y clave",
            1.4,
            vec![
                ("surround", t("Con clave e impostas")),
                ("leaf_style", t("Cuarterones")),
            ],
        ),
        (
            door,
            "Puerta apuntada de una hoja",
            1.0,
            vec![
                ("width", num(1.0)),
                ("height", num(2.7)),
                ("arch", t("Apuntado")),
                ("rise", num(0.87)),
                ("leaves", num(1.0)),
                ("leaf_style", t("Lisa")),
                ("surround", t("Moldurado")),
            ],
        ),
        (
            window,
            "Ventana de medio punto con fijo en arco",
            1.0,
            vec![
                ("bars", t("Cuarterones")),
                ("surround", t("Plano")),
                ("sill", Value::Bool(true)),
            ],
        ),
        (
            window,
            "Ventana rebajada de hojas en arco",
            1.2,
            vec![
                ("width", num(1.2)),
                ("height", num(1.5)),
                ("arch", t("Rebajado")),
                ("rise", num(0.25)),
                ("layout", t("Hojas en arco")),
                ("surround", t("Moldurado")),
            ],
        ),
        (
            window,
            "Óculo con recercado y clave",
            0.8,
            vec![
                ("width", num(0.8)),
                ("height", num(0.8)),
                ("arch", t("Óculo")),
                ("bars", t("Cuarterones")),
                ("surround", t("Con clave e impostas")),
            ],
        ),
        (
            void,
            "Hueco de medio punto",
            1.2,
            vec![
                ("width", num(1.2)),
                ("height", num(2.6)),
                ("shape", t("Medio punto")),
            ],
        ),
        (
            void,
            "Hueco carpanel",
            2.0,
            vec![
                ("width", num(2.0)),
                ("height", num(2.5)),
                ("shape", t("Carpanel")),
                ("rise", num(0.45)),
            ],
        ),
    ];
    let mut along = 1.2;
    for (family, name, width, extra) in items.drain(..) {
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        for (k, v) in extra {
            values.insert(k.into(), v);
        }
        let photo = name.contains("foto");
        for (key, id) in [
            ("frame_material", if photo { white } else { None }),
            ("panel_material", if photo { white } else { None }),
            ("surround_material", stone),
        ] {
            if let (Some(id), true) = (id, values.contains_key(key)) {
                values.insert(key.into(), Value::Material(id.0));
            }
        }
        let ty = m.add_family_type(NewFamilyType {
            family,
            name: format!("{name} (taller)"),
            mark: None,
            description: String::new(),
            values,
        })?;
        along += width / 2.0 + 0.3;
        let sill = if family == window {
            if name.contains("Óculo") {
                1.6
            } else {
                0.9
            }
        } else {
            0.0
        };
        m.add_opening(NewOpening {
            opening_type: ty,
            wall: row,
            along,
            sill,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
        along += width / 2.0 + 0.9;
    }
    Ok(())
}

/// Places the workshop's windows (paquete 9, segunda parte) from `x`
/// along a façade at `y`: one of each collection, two sashes, at a sill
/// of 0,90; and 8 m north another façade with what goes outside — the
/// roller blind half down, louvred shutters open, boarded ones closed,
/// a flat grille of bars, a boxed wrought grille with scrolls, a boxed
/// grid; and the sliding windows (correderas) 6 m south, with balcony
/// doors and their extras 6 m further. The end of the façades.
pub fn place_windows(m: &mut Model, x: f64, y: f64) -> Result<f64, ModelError> {
    use archisim_core::{Hand, NewFamilyType, NewOpening, WallSide};
    let level = m.levels()[0].id;
    let family = m.system_family("Ventana practicable")?;
    let facade = m
        .wall_types()
        .find(|t| t.name.contains("Fachada"))
        .or_else(|| m.wall_types().next())
        .map(|t| t.id)
        .ok_or_else(|| ModelError::NotAnObject("Fachada".into()))?;
    let step = 1.9;
    let end = x + step * DOOR_COLLECTIONS.len() as f64 + 0.4;
    let wall = |m: &mut Model, y: f64| {
        m.add_wall(NewWall {
            wall_type: facade,
            level,
            start: Point2::new(x, y),
            end: Point2::new(end, y),
            reference: ReferenceLine::Center,
            height: 2.9,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
    };
    let window_type = |m: &mut Model, name: String, set: &[(&str, Value)]| {
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        for (k, v) in set {
            values.insert((*k).to_string(), v.clone());
        }
        m.add_family_type(NewFamilyType {
            family,
            name,
            mark: None,
            description: String::new(),
            values,
        })
    };
    // Each collection's frame and hardware.
    let looks: [(&str, &str); 8] = [
        ("Roble claro", "Metal negro"),
        ("Lacado blanco", "Hierro forjado"),
        ("Metal negro", "Latón"),
        ("Roble claro", "Metal negro"),
        ("Metal lacado", "Acero inoxidable"),
        ("Pino viejo", "Hierro forjado"),
        ("Metal negro", "Metal negro"),
        ("Lacado blanco", "Latón"),
    ];
    let palette = |m: &mut Model, key: &'static str, name: &str| {
        palette_or_project(m, name).map(|id| (key, Value::Material(id.0)))
    };
    let front = wall(m, y)?;
    let place = |m: &mut Model, t, wall, i: usize, sill: f64| {
        m.add_opening(NewOpening {
            opening_type: t,
            wall,
            along: 1.1 + step * i as f64,
            sill,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
    };
    for (i, c) in DOOR_COLLECTIONS.iter().enumerate() {
        let mut set = vec![
            ("model3d", Value::Text((*c).to_string())),
            ("width", Value::Number(1.2)),
            ("height", Value::Number(1.3)),
        ];
        let (frame, metal) = looks[i];
        set.extend(palette(m, "frame_material", frame));
        set.extend(palette(m, "hardware_material", metal));
        let t = window_type(m, format!("Ventana {c} (taller)"), &set)?;
        place(m, t, front, i, 0.9)?;
    }
    // What goes outside, in another façade.
    let back = wall(m, y + 8.0)?;
    let t = |s: &str| Value::Text(s.to_string());
    type Outside<'a> = (&'a str, &'a str, Vec<(&'a str, Value)>);
    let outside: [Outside; 6] = [
        (
            "persiana",
            "Contemporáneo",
            vec![
                ("shutter", Value::Bool(true)),
                ("shutter_down", Value::Number(55.0)),
                ("height", Value::Number(1.55)),
            ],
        ),
        (
            "mallorquinas",
            "Mediterráneo",
            vec![
                ("outside_shutters", t("Mallorquinas")),
                ("shutters_open", Value::Bool(true)),
            ],
        ),
        (
            "contraventanas cerradas",
            "Rústica",
            vec![
                ("outside_shutters", t("De tablas")),
                ("shutters_open", Value::Bool(false)),
            ],
        ),
        (
            "reja de barrotes",
            "Clásica",
            vec![("grille", t("Barrotes y pletinas"))],
        ),
        (
            "reja de cajón con volutas",
            "Mediterráneo",
            vec![
                ("grille", t("Forja con volutas")),
                ("grille_box", Value::Bool(true)),
                ("grille_depth", Value::Number(0.2)),
            ],
        ),
        (
            "reja de cajón en cuadrícula",
            "Rústica",
            vec![
                ("grille", t("Cuadrícula")),
                ("grille_box", Value::Bool(true)),
            ],
        ),
    ];
    for (i, (name, c, extra)) in outside.into_iter().enumerate() {
        let k = DOOR_COLLECTIONS.iter().position(|x| *x == c).unwrap_or(0);
        let mut set = vec![
            ("model3d", t(c)),
            ("width", Value::Number(1.2)),
            ("height", Value::Number(1.3)),
        ];
        let (frame, metal) = looks[k];
        set.extend(palette(m, "frame_material", frame));
        set.extend(palette(m, "hardware_material", metal));
        let shutters = if name == "mallorquinas" {
            "Lacado salvia"
        } else {
            "Pino viejo"
        };
        set.extend(palette(m, "shutters_material", shutters));
        set.extend(palette(m, "grille_material", "Hierro forjado"));
        set.extend(extra);
        let ty = window_type(m, format!("Ventana con {name} (taller)"), &set)?;
        place(m, ty, back, i, 0.9)?;
    }
    // The sliding windows (correderas), 6 m south: one of each
    // collection, and balcony doors and their extras 6 m further.
    let sliders = m.system_family("Ventana corredera")?;
    let slider_type = |m: &mut Model, name: String, set: &[(&str, Value)]| {
        let mut values = m.family_defaults(sliders, ParamScope::Type)?;
        for (k, v) in set {
            values.insert((*k).to_string(), v.clone());
        }
        m.add_family_type(NewFamilyType {
            family: sliders,
            name,
            mark: None,
            description: String::new(),
            values,
        })
    };
    let row = wall(m, y - 6.0)?;
    for (i, c) in DOOR_COLLECTIONS.iter().enumerate() {
        let mut set = vec![
            ("model3d", t(c)),
            ("width", Value::Number(1.6)),
            ("height", Value::Number(1.3)),
        ];
        let (frame, metal) = looks[i];
        set.extend(palette(m, "frame_material", frame));
        set.extend(palette(m, "hardware_material", metal));
        let ty = slider_type(m, format!("Corredera {c} (taller)"), &set)?;
        place(m, ty, row, i, 0.9)?;
    }
    let balconies = wall(m, y - 12.0)?;
    let doors: [Outside; 5] = [
        (
            "balconera de cuatro hojas",
            "Clásica",
            vec![
                ("leaves", Value::Number(4.0)),
                ("width", Value::Number(1.85)),
                ("height", Value::Number(2.2)),
            ],
        ),
        (
            "balconera minimalista",
            "Contemporáneo",
            vec![
                ("width", Value::Number(1.85)),
                ("height", Value::Number(2.3)),
                ("slider_handle", t("Tirador de barra")),
            ],
        ),
        (
            "mosquitera",
            "Mediterráneo",
            vec![("width", Value::Number(1.6)), ("screen", Value::Bool(true))],
        ),
        (
            "persiana",
            "Contemporáneo",
            vec![
                ("width", Value::Number(1.6)),
                ("height", Value::Number(1.55)),
                ("shutter", Value::Bool(true)),
                ("shutter_down", Value::Number(35.0)),
                ("slider_profile", t("Aluminio con rotura de puente térmico")),
            ],
        ),
        (
            "reja",
            "Rústica",
            vec![
                ("width", Value::Number(1.6)),
                ("grille", t("Barrotes y pletinas")),
            ],
        ),
    ];
    for (i, (name, c, extra)) in doors.into_iter().enumerate() {
        let k = DOOR_COLLECTIONS.iter().position(|x| *x == c).unwrap_or(0);
        let mut set = vec![("model3d", t(c)), ("height", Value::Number(1.3))];
        let (frame, metal) = looks[k];
        set.extend(palette(m, "frame_material", frame));
        set.extend(palette(m, "hardware_material", metal));
        set.extend(palette(m, "grille_material", "Hierro forjado"));
        set.extend(extra);
        let tall = set
            .iter()
            .any(|(k, v)| *k == "height" && matches!(v, Value::Number(h) if *h > 2.0));
        let ty = slider_type(m, format!("Corredera {name} (taller)"), &set)?;
        // Wider than the others: further apart.
        m.add_opening(NewOpening {
            opening_type: ty,
            wall: balconies,
            along: 1.4 + 2.6 * i as f64,
            sill: if tall { 0.0 } else { 0.9 },
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
    }
    // Sash windows (guillotina), 6 m further.
    let sash = m.system_family("Ventana de guillotina")?;
    let sashes = wall(m, y - 18.0)?;
    type Sash<'a> = (&'a str, &'a str, f64, Vec<(&'a str, Value)>);
    let kinds: [Sash; 3] = [
        (
            "clásica subida",
            "Clásica",
            40.0,
            vec![("glazing_bars", t("Cuadrícula"))],
        ),
        ("industrial", "Industrial", 0.0, vec![]),
        (
            "mediterránea con mallorquinas",
            "Mediterráneo",
            0.0,
            vec![
                ("outside_shutters", t("Mallorquinas")),
                ("shutters_open", Value::Bool(true)),
            ],
        ),
    ];
    for (i, (name, c, open, extra)) in kinds.into_iter().enumerate() {
        let k = DOOR_COLLECTIONS.iter().position(|x| *x == c).unwrap_or(0);
        let mut values = m.family_defaults(sash, ParamScope::Type)?;
        values.insert("model3d".into(), t(c));
        let (frame, metal) = looks[k];
        for (key, v) in palette(m, "frame_material", frame)
            .into_iter()
            .chain(palette(m, "hardware_material", metal))
            .chain(palette(m, "shutters_material", "Lacado salvia"))
            .chain(extra)
        {
            values.insert(key.into(), v);
        }
        let ty = m.add_family_type(NewFamilyType {
            family: sash,
            name: format!("Guillotina {name} (taller)"),
            mark: None,
            description: String::new(),
            values,
        })?;
        let o = m.add_opening(NewOpening {
            opening_type: ty,
            wall: sashes,
            along: 1.5 + 2.6 * i as f64,
            sill: 0.8,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })?;
        if open > 0.0 {
            let mut values = m.opening(o).map(|x| x.values.clone()).unwrap_or_default();
            values.insert("sash_open".into(), Value::Number(open));
            m.set_opening_values(o, values)?;
        }
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    /// Nothing of the workshop in the air:
    /// `cargo test --release -p archisim-library -- nothing_in_the_air --nocapture`
    /// lists what is.
    #[test]
    fn nothing_in_the_air() {
        let m = project().unwrap();
        let report = loose_report(&m);
        for (name, loose) in &report {
            println!("{name}");
            // Each group by its piece nearest to what holds.
            let groups = loose.iter().map(|l| l.group).max().unwrap_or(0) + 1;
            for g in 0..groups {
                let mine: Vec<_> = loose.iter().filter(|l| l.group == g).collect();
                let Some(l) = mine.iter().min_by(|a, b| a.gap.total_cmp(&b.gap)) else {
                    continue;
                };
                println!(
                    "    {} pieces, {} ({} tri, {:.3} m off): x {:.3}..{:.3}  y {:.3}..{:.3}  z {:.3}..{:.3}",
                    mine.len(),
                    l.role,
                    l.triangles,
                    l.gap,
                    l.lo[0],
                    l.hi[0],
                    l.lo[1],
                    l.hi[1],
                    l.lo[2],
                    l.hi[2]
                );
                let (role, lo, hi) = l.near;
                println!(
                    "        near {role}: x {:.3}..{:.3}  y {:.3}..{:.3}  z {:.3}..{:.3}",
                    lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
                );
            }
        }
        println!("{} types with pieces in the air", report.len());
        assert!(report.is_empty(), "pieces in the air (see above)");
    }

    use super::*;

    #[test]
    fn every_material_of_a_piece_is_in_the_palette() {
        for k in KITCHENS
            .iter()
            .chain(BATHROOMS)
            .chain(BEDROOMS)
            .chain(LIGHTS)
            .chain(TERRACES)
        {
            for (key, name) in k.materials {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{}: {key} = {name}",
                    k.collection
                );
            }
        }
        for piece in PIECES {
            for (k, v) in piece.values {
                if let M(name) = v {
                    assert!(
                        crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                        "{}: {k} = {name}",
                        piece.name
                    );
                }
            }
        }
    }

    /// The triangles of a type's model: made by ArchiSim, or grown (as
    /// near the camera as it gets).
    fn triangles(m: &Model, t: ObjectTypeId) -> usize {
        if m.type_grows(t) {
            let o = m.objects().find(|o| o.object_type == t).unwrap().id;
            let (mesh, _) = m.plant_mesh(o, archisim_core::flora::Lod::Full).unwrap();
            return mesh.triangle_count();
        }
        assert!(m.type_has_prop(t));
        m.type_prop(t).unwrap().0.triangle_count()
    }

    /// Every kitchen of the workshop is made in its collection: each
    /// module with its own model, its worktop one run.
    #[test]
    fn the_workshop_kitchens_are_made() {
        let m = project().unwrap();
        for k in KITCHENS {
            let mine: Vec<_> = m
                .objects()
                .filter(|o| {
                    m.object_type(o.object_type).is_some_and(|t| {
                        t.name.contains(k.collection)
                            && m.family(t.family)
                                .is_some_and(|f| f.def().category == "kitchen")
                    })
                })
                .collect();
            assert_eq!(mine.len(), RUN.len() + 3, "{}", k.collection);
            for o in &mine {
                let t = o.object_type;
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (100..=30_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
            let first = mine.iter().find(|o| m.worktop_run_of(o.id).is_some());
            let run = m.worktop_run_of(first.expect("a worktop run").id).unwrap();
            assert_eq!(run.members.len(), RUN.len() - 1, "{}", k.collection);
        }
    }

    /// Every bathroom of the workshop is made in its collection.
    #[test]
    fn the_workshop_bathrooms_are_made() {
        let m = project().unwrap();
        for k in BATHROOMS {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| {
                    t.name.contains(k.collection)
                        && !t.name.contains(" 60")
                        && !t.name.contains(" 80")
                        && !t.name.starts_with("Espejo de pie")
                        && !t.name.starts_with("Ducha de piscina")
                })
                .filter(|t| {
                    [
                        "Bañera", "Inodoro", "Bidé", "Lavabo", "Toallero", "Ducha", "Espejo",
                    ]
                    .iter()
                    .any(|n| t.name.starts_with(n))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), SUITE.len() + 1, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=30_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every bedroom of the workshop is made in its collection.
    #[test]
    fn the_workshop_bedrooms_are_made() {
        let m = project().unwrap();
        for k in BEDROOMS {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    [
                        "Armario",
                        "Cómoda",
                        "Mesita",
                        "Cama",
                        "Cama infantil",
                        "Tocador",
                        "Banco",
                        "Galán",
                        "Espejo de pie",
                        "Cuna",
                        "Litera",
                    ]
                    .iter()
                    .any(|n| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 11, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every lamp of the workshop is made in its collection.
    #[test]
    fn the_workshop_lamps_are_made() {
        let m = project().unwrap();
        for k in LIGHTS {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    ["Lámpara", "Aplique", "Plafón"]
                        .iter()
                        .any(|n| t.name.starts_with(n))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 5, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=30_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every terrace of the workshop is made in its collection.
    #[test]
    fn the_workshop_terraces_are_made() {
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    [
                        "Pérgola",
                        "Mesa de exterior",
                        "Sofá de exterior",
                        "Tumbona",
                        "Sombrilla",
                        "Macetón",
                        "Silla de exterior",
                    ]
                    .iter()
                    .any(|n| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 7, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's pool and shower are made, and their materials
    /// are in the palette.
    #[test]
    fn the_workshop_sport_is_made() {
        for (_, _, extras) in SPORT_PIECES {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for (_, label, _) in SPORT_PIECES.iter().chain(&PARK_PIECES) {
            let name = format!("{label} (taller)");
            let t = m
                .object_types()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("{name}"));
            assert!(m.type_has_prop(t.id), "{name}");
            let tris = m.type_prop(t.id).unwrap().0.triangle_count();
            assert!((50..=40_000).contains(&tris), "{name}: {tris}");
            assert!(m.objects().any(|o| o.object_type == t.id), "{name}");
        }
    }

    #[test]
    fn the_workshop_pools_are_made() {
        for (extras, _) in POOLS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    ["Piscina ·", "Ducha de piscina ·"]
                        .iter()
                        .any(|n| t.name.starts_with(n))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 2, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
        // Each pool object takes its room out of the ground.
        let dug = m.excavations();
        assert_eq!(dug.len(), 8);
        for e in &dug {
            assert!(e.bottom < -1.4, "{}", e.bottom);
            assert!(archisim_core::signed_area(&e.outline) > 20.0);
        }
    }

    /// Every collection's shop is made, and its materials are in the
    /// palette.
    #[test]
    fn the_workshop_shops_are_made() {
        for extras in SHOP_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    SHOP_PIECES
                        .iter()
                        .any(|(_, n)| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 5, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (100..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's office is made, and its materials are in the
    /// palette.
    #[test]
    fn the_workshop_offices_are_made() {
        for extras in OFFICE_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    OFFICE_PIECES
                        .iter()
                        .any(|(_, n)| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 5, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (100..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's bar is made, and its materials are in the
    /// palette.
    #[test]
    fn the_workshop_bars_are_made() {
        for extras in BAR_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    BAR_PIECES
                        .iter()
                        .any(|(_, n)| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 5, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (100..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's desk and chair are made, and their materials
    /// are in the palette.
    #[test]
    fn the_workshop_studies_are_made() {
        for extras in STUDY_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    ["Escritorio ·", "Silla de trabajo ·", "Estantería ·"]
                        .iter()
                        .any(|n| t.name.starts_with(n))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 3, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (50..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's house plant, decoration and picture is made, and
    /// its materials are in the palette.
    #[test]
    fn the_workshop_living_rooms_are_made() {
        for extras in INDOOR_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    [
                        "Planta en maceta ·",
                        "Decoración ·",
                        "Cuadro o espejo ·",
                        "Aparador ·",
                    ]
                    .iter()
                    .any(|n| t.name.starts_with(n))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 4, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (20..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every collection's street furniture is made, and its materials are
    /// in the palette.
    #[test]
    fn the_workshop_street_is_made() {
        for extras in STREET_EXTRAS {
            for (key, name) in extras {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == *name),
                    "{key} = {name}"
                );
            }
        }
        let m = project().unwrap();
        for k in TERRACES {
            let mine: Vec<_> = m
                .object_types()
                .filter(|t| t.name.contains(k.collection))
                .filter(|t| {
                    STREET_PIECES
                        .iter()
                        .any(|(_, n)| t.name.starts_with(&format!("{n} ·")))
                })
                .map(|t| t.id)
                .collect();
            assert_eq!(mine.len(), 7, "{}", k.collection);
            for t in mine {
                assert!(m.type_has_prop(t), "{}", k.collection);
                let tris = m.type_prop(t).unwrap().0.triangle_count();
                assert!(
                    (20..=40_000).contains(&tris),
                    "{}: {tris}",
                    m.object_type(t).unwrap().name
                );
            }
        }
    }

    /// Every piece of the workshop is made with its own 3D and stays within
    /// its weight.
    #[test]
    fn the_workshop_pieces_are_made() {
        let m = project().unwrap();
        for piece in PIECES {
            let t = m.object_types().find(|t| t.name == piece.name).unwrap().id;
            let tris = triangles(&m, t);
            // Light enough to fill a house with them.
            assert!((100..=piece.most).contains(&tris), "{}: {tris}", piece.name);
            if let Some(old) = piece.replaces {
                let o = m.object_types().find(|t| t.name == old).unwrap().id;
                let old_tris = triangles(&m, o);
                eprintln!("{}: {tris} triángulos (antes {old_tris})", piece.name);
            }
        }
    }
}
