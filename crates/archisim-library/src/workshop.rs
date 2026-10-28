//! The workshop (M1): the models being remade with more care, in a
//! project of their own, each one next to the library's piece it is to
//! replace, to be looked at and judged before anything enters the library.
//!
//! Their families (`taller/*.asfam`) are not system families: a project
//! that uses one keeps its copy, as with any family.

use archisim_core::{
    Model, ModelError, NewObject, NewWall, ObjectTypeId, ParamScope, Point2, ReferenceLine, Value,
    WallTop,
};

/// The workshop's recipes (a piece whose family is `flora:<species>` is
/// a plant of the library grown with the workshop's values instead).
pub const RECIPES: &[&str] = &[
    include_str!("../taller/planta.asfam"),
    include_str!("../taller/farola.asfam"),
    include_str!("../taller/cocina_bajo.asfam"),
    include_str!("../taller/cocina_columna.asfam"),
    include_str!("../taller/cocina_alto.asfam"),
    include_str!("../taller/cocina_campana.asfam"),
    include_str!("../taller/bano_lavabo.asfam"),
    include_str!("../taller/bano_inodoro.asfam"),
    include_str!("../taller/bano_bide.asfam"),
    include_str!("../taller/bano_ducha.asfam"),
    include_str!("../taller/bano_banera.asfam"),
    include_str!("../taller/bano_espejo.asfam"),
    include_str!("../taller/bano_toallero.asfam"),
    include_str!("../taller/dormitorio_cama.asfam"),
    include_str!("../taller/dormitorio_mesita.asfam"),
    include_str!("../taller/dormitorio_armario.asfam"),
    include_str!("../taller/dormitorio_comoda.asfam"),
    include_str!("../taller/dormitorio_banco.asfam"),
    include_str!("../taller/dormitorio_tocador.asfam"),
    include_str!("../taller/dormitorio_cama_infantil.asfam"),
    include_str!("../taller/dormitorio_cuna.asfam"),
    include_str!("../taller/dormitorio_litera.asfam"),
    include_str!("../taller/dormitorio_galan.asfam"),
    include_str!("../taller/dormitorio_espejo_pie.asfam"),
    include_str!("../taller/luz_colgante.asfam"),
    include_str!("../taller/luz_plafon.asfam"),
    include_str!("../taller/luz_aplique.asfam"),
    include_str!("../taller/luz_pie.asfam"),
    include_str!("../taller/luz_mesa.asfam"),
    include_str!("../taller/exterior_mesa.asfam"),
    include_str!("../taller/exterior_silla.asfam"),
    include_str!("../taller/exterior_sofa.asfam"),
    include_str!("../taller/exterior_tumbona.asfam"),
    include_str!("../taller/exterior_sombrilla.asfam"),
    include_str!("../taller/exterior_maceton.asfam"),
    include_str!("../taller/exterior_pergola.asfam"),
    include_str!("../taller/urbano_banco.asfam"),
    include_str!("../taller/urbano_farola.asfam"),
    include_str!("../taller/urbano_papelera.asfam"),
    include_str!("../taller/urbano_bolardo.asfam"),
    include_str!("../taller/urbano_jardinera.asfam"),
    include_str!("../taller/urbano_aparcabicis.asfam"),
    include_str!("../taller/urbano_fuente.asfam"),
    include_str!("../taller/interior_planta.asfam"),
    include_str!("../taller/interior_decoracion.asfam"),
    include_str!("../taller/interior_cuadro.asfam"),
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
        "Armario (taller)",
        "Armario",
        0.0,
        &[("width", N(1.5)), ("doors", N(3.0)), ("drawers", N(1.0))],
    ),
    (
        "Cómoda (taller)",
        "Cómoda",
        0.35,
        &[("columns", N(2.0)), ("width", N(1.1))],
    ),
    ("Mesita de noche (taller)", "Mesita", 0.45, &[]),
    ("Cama (taller)", "Cama", 0.08, &[("width", N(1.6))]),
    ("Mesita de noche (taller)", "Mesita", 0.08, &[]),
    ("Tocador (taller)", "Tocador", 0.6, &[]),
    ("Galán de noche (taller)", "Galán", 0.4, &[]),
    ("Espejo de pie (taller)", "Espejo de pie", 0.3, &[]),
    // The children's room, further along the same wall.
    ("Cama infantil (taller)", "Cama infantil", 1.0, &[]),
    ("Cuna (taller)", "Cuna", 0.5, &[]),
    ("Litera (taller)", "Litera", 0.7, &[]),
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
        let t = kitchen_type(
            m,
            k,
            "Banco de pie de cama (taller)",
            "Banco",
            &[("width", N(1.3))],
        )?;
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
        "Cómoda (taller)",
        "Cómoda",
        &[("columns", N(2.0)), ("width", N(1.1))],
    )?;
    put(m, chest, x + 0.55, wall, 0.0)?;
    let lamp = kitchen_type(m, k, "Lámpara de mesa (taller)", "Lámpara de mesa", &[])?;
    put(m, lamp, x + 0.8, wall, 0.85)?;
    let sconce = kitchen_type(
        m,
        k,
        "Aplique (taller)",
        "Aplique",
        &[("elevation", N(1.8))],
    )?;
    put(m, sconce, x + 1.6, wall, 0.0)?;
    let floor = kitchen_type(m, k, "Lámpara de pie (taller)", "Lámpara de pie", &[])?;
    put(m, floor, x + 2.3, wall, 0.0)?;
    let pendant = kitchen_type(
        m,
        k,
        "Lámpara colgante (taller)",
        "Lámpara colgante",
        &[("diameter", N(0.5))],
    )?;
    put(m, pendant, x + 1.2, wall - 1.4, 0.0)?;
    let ceiling = kitchen_type(m, k, "Plafón (taller)", "Plafón", &[])?;
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
        "Pérgola (taller)",
        "Pérgola",
        &[("width", N(3.2)), ("depth", N(3.0))],
    )?;
    put(m, pergola, x + 1.6, front)?;
    let table = kitchen_type(
        m,
        k,
        "Mesa de exterior (taller)",
        "Mesa de exterior",
        &[("width", N(1.6)), ("depth", N(0.9)), ("chairs", N(4.0))],
    )?;
    put(m, table, x + 1.6, front - 1.05)?;
    let sofa = kitchen_type(m, k, "Sofá de exterior (taller)", "Sofá de exterior", &[])?;
    put(m, sofa, x + 4.3, front)?;
    let lounger = kitchen_type(m, k, "Tumbona (taller)", "Tumbona", &[])?;
    put(m, lounger, x + 6.1, front)?;
    let parasol = kitchen_type(
        m,
        k,
        "Sombrilla (taller)",
        "Sombrilla",
        &[("width", N(2.2))],
    )?;
    // Beside the lounger's head, not through it: a parasol on its pole
    // stands at its side and shades the head; a cantilever one hangs
    // its canopy over the head from its arm beside it.
    if matches!(k.collection, "Contemporáneo" | "Industrial") {
        put(m, parasol, x + 6.1, front + 0.6)?;
    } else {
        put(m, parasol, x + 6.85, front + 0.75)?;
    }
    let pot = kitchen_type(m, k, "Macetón (taller)", "Macetón", &[])?;
    put(m, pot, x + 8.6, front)?;
    let chair = kitchen_type(m, k, "Silla de exterior (taller)", "Silla de exterior", &[])?;
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
    let decor = kitchen_type(m, k, "Decoración (taller)", "Decoración", &values(&[]))?;
    put(m, decor, x + 0.55, 0.75)?;
    let picture = kitchen_type(
        m,
        k,
        "Cuadro o espejo (taller)",
        "Cuadro o espejo",
        &values(&[
            ("width", N(0.7)),
            ("height", N(0.85)),
            ("elevation", N(1.2)),
        ]),
    )?;
    put(m, picture, x + 1.1, 0.0)?;
    let plant = kitchen_type(
        m,
        k,
        "Planta en maceta (taller)",
        "Planta en maceta",
        &values(&[]),
    )?;
    put(m, plant, x + 2.25, 0.0)?;
    Ok(x + 2.8)
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
    ("Banco (taller)", "Banco de calle"),
    ("Farola (taller)", "Farola"),
    ("Papelera (taller)", "Papelera"),
    ("Bolardo (taller)", "Bolardo"),
    ("Jardinera (taller)", "Jardinera"),
    ("Aparcabicis (taller)", "Aparcabicis"),
    ("Fuente de beber (taller)", "Fuente de beber"),
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
        "Bañera (taller)",
        "Bañera",
        0.0,
        &[("width", N(1.7)), ("depth", N(0.75)), ("height", N(0.55))],
    ),
    ("Inodoro (taller)", "Inodoro", 0.3, &[]),
    ("Bidé (taller)", "Bidé", 0.3, &[]),
    (
        "Lavabo (taller)",
        "Lavabo",
        0.35,
        &[("width", N(0.8)), ("depth", N(0.48))],
    ),
    (
        "Toallero (taller)",
        "Toallero",
        0.25,
        &[
            ("width", N(0.5)),
            ("elevation", N(1.15)),
            ("height", N(0.8)),
        ],
    ),
    (
        "Ducha (taller)",
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
            "Espejo (taller)",
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
        "Columna de cocina (taller)",
        0.6,
        &[("tall", T("Frigorífico integrado"))],
    ),
    (
        "Módulo bajo de cocina (taller)",
        0.6,
        &[("unit", T("Cajonera")), ("drawers", N(3.0))],
    ),
    (
        "Módulo bajo de cocina (taller)",
        0.8,
        &[("unit", T("Fregadero")), ("doors", N(2.0))],
    ),
    (
        "Módulo bajo de cocina (taller)",
        0.6,
        &[("unit", T("Lavavajillas"))],
    ),
    (
        "Módulo bajo de cocina (taller)",
        0.8,
        &[("unit", T("Placa")), ("drawers", N(2.0))],
    ),
    (
        "Módulo bajo de cocina (taller)",
        0.6,
        &[("unit", T("Horno"))],
    ),
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
            M(n) => match crate::palette_material(m, n) {
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
            "Mueble alto de cocina (taller)",
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
        "Campana de cocina (taller)",
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
    let source = RECIPES
        .iter()
        .find(|r| {
            r.lines()
                .next()
                .is_some_and(|l| l.trim() == format!("family \"{name}\""))
        })
        .ok_or_else(|| ModelError::NotAnObject(name.to_string()))?;
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
            M(name) => match crate::palette_material(m, name) {
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
    // The street furniture, further east.
    for (k, extras) in TERRACES.iter().zip(STREET_EXTRAS) {
        x = place_street(&mut m, k, extras, x, wall)? + 1.8;
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
    Ok(m)
}

#[cfg(test)]
mod tests {
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
                    .any(|n| t.name.starts_with(n))
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
