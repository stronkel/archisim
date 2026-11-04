//! The workshop of buildings (taller de edificios, tanda 1): the
//! surroundings' buildings remade with the care of the furniture, in a
//! street of their own, each facing the building of the same kind that
//! ArchiSim makes today (U6) across the street, to be judged before they
//! replace anything.
//!
//! Each is an object of a recipe of `taller/edificio_*.asfam` made by
//! `archisim_props::fine::buildings`.

use archisim_core::{
    BuildingStyle, Model, ModelError, NewEnvBuilding, NewObject, NewRoad, ObjectTypeId, ParamScope,
    Point2, Value,
};

/// The recipes of the buildings.
pub const RECIPES: &[&str] = &[
    include_str!("../taller/edificio_casa_pueblo.asfam"),
    include_str!("../taller/edificio_casa_senorial.asfam"),
    include_str!("../taller/edificio_ensanche.asfam"),
    include_str!("../taller/edificio_bloque_60.asfam"),
    include_str!("../taller/edificio_torre_costa.asfam"),
    include_str!("../taller/edificio_adosados.asfam"),
    include_str!("../taller/edificio_corral.asfam"),
    include_str!("../taller/edificio_cortijo.asfam"),
    include_str!("../taller/edificio_colonizacion.asfam"),
    include_str!("../taller/edificio_chalet80.asfam"),
    include_str!("../taller/edificio_pareadas.asfam"),
    include_str!("../taller/edificio_nave.asfam"),
    include_str!("../taller/edificio_mercado.asfam"),
    include_str!("../taller/edificio_iglesia.asfam"),
    include_str!("../taller/edificio_toros.asfam"),
    include_str!("../taller/edificio_colegio.asfam"),
    include_str!("../taller/edificio_salud.asfam"),
    include_str!("../taller/edificio_gasolinera.asfam"),
    include_str!("../taller/edificio_castilla.asfam"),
    include_str!("../taller/edificio_caserio.asfam"),
    include_str!("../taller/edificio_galeria.asfam"),
    include_str!("../taller/edificio_masia.asfam"),
    include_str!("../taller/edificio_canaria.asfam"),
    include_str!("../taller/edificio_ibicenca.asfam"),
    include_str!("../taller/edificio_oficinas_torre.asfam"),
    include_str!("../taller/edificio_oficinas_70.asfam"),
    include_str!("../taller/edificio_parque_empresarial.asfam"),
    include_str!("../taller/edificio_cafeteria.asfam"),
    include_str!("../taller/edificio_comercio.asfam"),
    include_str!("../taller/edificio_hotel.asfam"),
    include_str!("../taller/edificio_supermercado.asfam"),
    include_str!("../taller/edificio_mundo_entre_medianeras.asfam"),
    include_str!("../taller/edificio_mundo_bloque.asfam"),
    include_str!("../taller/edificio_mundo_aislada.asfam"),
    include_str!("../taller/edificio_mundo_muestra.asfam"),
    include_str!("../taller/edificio_mundo_torre.asfam"),
    include_str!("../taller/edificio_mundo_patio.asfam"),
    include_str!("../taller/edificio_mundo_hilera.asfam"),
];

/// The family of the samples over plans that are not rectangles.
pub const WORLD_SAMPLE: &str = "Muestra de planta del mundo (taller)";

/// The families of the buildings of the world (tanda 6): one typology
/// each, dressed by the skin of a place.
pub const WORLD_TOWNHOUSE: &str = "Casa entre medianeras del mundo (taller)";
pub const WORLD_BLOCK: &str = "Bloque de viviendas del mundo (taller)";
pub const WORLD_HOUSE: &str = "Vivienda aislada del mundo (taller)";
pub const WORLD_TOWER: &str = "Torre del mundo (taller)";
pub const WORLD_COURTYARD: &str = "Casa patio del mundo (taller)";
pub const WORLD_ROW: &str = "Casas en hilera del mundo (taller)";

/// A building of the workshop: its family, its type's name, its values
/// (numbers), and the style of today's building it stands opposite, with
/// that one's floors.
pub struct Building {
    pub family: &'static str,
    pub name: &'static str,
    pub values: &'static [(&'static str, f64)],
    pub today: BuildingStyle,
    pub floors: usize,
    /// What it takes along the street and its depth.
    pub size: (f64, f64),
    /// The gap to the next one.
    pub gap: f64,
    /// Where its origin is from the start of what it takes along the
    /// street (half of it, for most).
    pub left: f64,
    /// Today's buildings it is made of (a row of terraced houses, a pair)
    /// and how much of its depth is garden in front, not building.
    pub parts: usize,
    pub garden: f64,
    /// Which tanda it belongs to.
    pub tanda: usize,
}

/// The street of the workshop, west to east: a row of the old centre
/// (two village houses, the town house and the «ensanche» between party
/// walls), the 1960s block, the coastal tower and the terraced houses.
pub const BUILDINGS: &[Building] = &[
    Building {
        family: "Casa de pueblo andaluza (taller)",
        name: "Casa de pueblo · 2 plantas (taller)",
        values: &[
            ("width", 7.0),
            ("depth", 10.0),
            ("floors", 2.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Andalusian,
        floors: 2,
        size: (7.0, 10.0),
        left: 7.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 0.0,
    },
    Building {
        family: "Casa de pueblo andaluza (taller)",
        name: "Casa de pueblo · 3 plantas sin cierro (taller)",
        values: &[
            ("width", 6.0),
            ("depth", 10.0),
            ("floors", 3.0),
            ("variant", 2.0),
            ("cierro", 0.0),
        ],
        today: BuildingStyle::Andalusian,
        floors: 3,
        size: (6.0, 10.0),
        left: 6.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 0.0,
    },
    Building {
        family: "Casa de pueblo andaluza (taller)",
        name: "Casa de pueblo · 1 planta (taller)",
        values: &[
            ("width", 5.5),
            ("depth", 10.0),
            ("floors", 1.0),
            ("variant", 4.0),
        ],
        today: BuildingStyle::Andalusian,
        floors: 1,
        size: (5.5, 10.0),
        left: 5.5 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 0.0,
    },
    Building {
        family: "Casa señorial del casco (taller)",
        name: "Casa señorial (taller)",
        values: &[
            ("width", 12.0),
            ("depth", 14.0),
            ("floors", 3.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::HistoricCentre,
        floors: 3,
        size: (12.0, 14.0),
        left: 12.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 0.0,
    },
    Building {
        family: "Edificio de ensanche (taller)",
        name: "Edificio de ensanche (taller)",
        values: &[
            ("width", 14.0),
            ("depth", 16.0),
            ("floors", 5.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Ensanche,
        floors: 5,
        size: (14.0, 16.0),
        left: 14.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 6.0,
    },
    Building {
        family: "Bloque de los años 60 y 70 (taller)",
        name: "Bloque de los 60 (taller)",
        values: &[
            ("width", 18.0),
            ("depth", 12.0),
            ("floors", 6.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Block1960s,
        floors: 6,
        size: (18.0, 12.0),
        left: 18.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 8.0,
    },
    Building {
        family: "Torre de apartamentos de costa (taller)",
        name: "Torre de costa (taller)",
        values: &[
            ("width", 20.0),
            ("depth", 13.0),
            ("floors", 10.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::CoastalHotel,
        floors: 10,
        size: (20.0, 13.0),
        left: 20.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 4.0,
    },
    Building {
        family: "Torre de apartamentos de costa (taller)",
        name: "Torre de costa · barandillas de vidrio (taller)",
        values: &[
            ("width", 16.0),
            ("depth", 12.0),
            ("floors", 7.0),
            ("variant", 3.0),
            ("glass_rail", 1.0),
        ],
        today: BuildingStyle::CoastalHotel,
        floors: 7,
        size: (16.0, 12.0),
        left: 16.0 / 2.0,
        parts: 1,
        garden: 0.0,
        tanda: 1,
        gap: 8.0,
    },
    Building {
        family: "Adosados de promotora (taller)",
        name: "Adosados · 4 viviendas (taller)",
        values: &[
            ("houses", 4.0),
            ("house_width", 6.0),
            ("depth", 10.0),
            ("garden", 3.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Terraced,
        floors: 2,
        size: (24.0, 13.0),
        left: 24.0 / 2.0,
        parts: 4,
        garden: 3.0,
        tanda: 1,
        gap: 0.0,
    },
    Building {
        family: "Corral de vecinos (taller)",
        name: "Corral de vecinos (taller)",
        values: &[("width", 16.0), ("depth", 24.0), ("variant", 1.0)],
        today: BuildingStyle::Courtyard,
        floors: 2,
        size: (16.0, 24.0),
        left: 8.0,
        parts: 1,
        garden: 0.0,
        tanda: 2,
        gap: 6.0,
    },
    Building {
        family: "Cortijo andaluz (taller)",
        name: "Cortijo (taller)",
        values: &[("width", 32.0), ("depth", 28.0), ("variant", 1.0)],
        today: BuildingStyle::Courtyard,
        floors: 2,
        size: (32.0, 28.0),
        left: 16.0,
        parts: 1,
        garden: 0.0,
        tanda: 2,
        gap: 6.0,
    },
    Building {
        family: "Viviendas de colonización (taller)",
        name: "Viviendas de colonización · 3 casas (taller)",
        values: &[
            ("houses", 3.0),
            ("house_width", 7.0),
            ("yard_width", 4.0),
            ("depth", 10.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Andalusian,
        floors: 2,
        size: (33.0, 10.0),
        left: 16.5,
        parts: 3,
        garden: 0.0,
        tanda: 2,
        gap: 6.0,
    },
    Building {
        family: "Chalet de los 80 (taller)",
        name: "Chalet de los 80 (taller)",
        values: &[("width", 12.0), ("depth", 10.0), ("variant", 1.0)],
        today: BuildingStyle::Detached,
        floors: 2,
        size: (21.6, 16.0),
        left: 9.0,
        parts: 1,
        garden: 6.0,
        tanda: 2,
        gap: 4.0,
    },
    Building {
        family: "Viviendas pareadas (taller)",
        name: "Viviendas pareadas (taller)",
        values: &[("house_width", 8.0), ("depth", 10.0), ("variant", 1.0)],
        today: BuildingStyle::SemiDetached,
        floors: 2,
        size: (21.0, 14.5),
        left: 10.5,
        parts: 2,
        garden: 4.5,
        tanda: 2,
        gap: 6.0,
    },
    Building {
        family: "Nave de polígono (taller)",
        name: "Nave de polígono (taller)",
        values: &[
            ("width", 24.0),
            ("depth", 40.0),
            ("height", 8.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Warehouse,
        floors: 1,
        size: (28.0, 50.0),
        left: 14.0,
        parts: 1,
        garden: 10.0,
        tanda: 2,
        gap: 6.0,
    },
    Building {
        family: "Mercado de abastos (taller)",
        name: "Mercado de abastos (taller)",
        values: &[("width", 24.0), ("depth", 36.0), ("variant", 1.0)],
        today: BuildingStyle::Facility,
        floors: 1,
        size: (26.0, 41.0),
        left: 13.0,
        parts: 1,
        garden: 5.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Iglesia parroquial (taller)",
        name: "Iglesia parroquial (taller)",
        values: &[("width", 11.0), ("depth", 28.0), ("variant", 1.0)],
        today: BuildingStyle::Facility,
        floors: 2,
        size: (20.0, 37.0),
        left: 11.0,
        parts: 1,
        garden: 9.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Plaza de toros (taller)",
        name: "Plaza de toros (taller)",
        values: &[("radius", 28.0), ("variant", 1.0)],
        today: BuildingStyle::Facility,
        floors: 2,
        size: (64.0, 60.0),
        left: 32.0,
        parts: 1,
        garden: 4.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Colegio público (taller)",
        name: "Colegio público (taller)",
        values: &[("width", 40.0), ("variant", 1.0)],
        today: BuildingStyle::Facility,
        floors: 2,
        size: (60.0, 34.0),
        left: 30.0,
        parts: 1,
        garden: 24.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Centro de salud (taller)",
        name: "Centro de salud (taller)",
        values: &[("width", 32.0), ("depth", 14.0), ("variant", 1.0)],
        today: BuildingStyle::Offices,
        floors: 2,
        size: (32.0, 28.0),
        left: 16.0,
        parts: 1,
        garden: 14.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Gasolinera (taller)",
        name: "Gasolinera (taller)",
        values: &[("variant", 1.0)],
        today: BuildingStyle::Facility,
        floors: 1,
        size: (32.0, 24.0),
        left: 16.0,
        parts: 1,
        garden: 16.0,
        tanda: 3,
        gap: 6.0,
    },
    Building {
        family: "Casa castellana con soportales (taller)",
        name: "Casa castellana con soportales (taller)",
        values: &[("width", 12.0), ("depth", 12.0), ("variant", 1.0)],
        today: BuildingStyle::HistoricCentre,
        floors: 3,
        size: (12.0, 12.0),
        left: 6.0,
        parts: 1,
        garden: 0.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Caserío vasco (taller)",
        name: "Caserío vasco (taller)",
        values: &[("width", 15.0), ("depth", 14.0), ("variant", 1.0)],
        today: BuildingStyle::Detached,
        floors: 2,
        size: (28.0, 20.0),
        left: 14.0,
        parts: 1,
        garden: 6.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Casa con galería gallega (taller)",
        name: "Casa con galería gallega (taller)",
        values: &[("width", 7.0), ("depth", 12.0), ("variant", 1.0)],
        today: BuildingStyle::HistoricCentre,
        floors: 5,
        size: (7.0, 12.0),
        left: 3.5,
        parts: 1,
        garden: 0.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Masía catalana (taller)",
        name: "Masía catalana (taller)",
        values: &[("width", 9.0), ("variant", 1.0)],
        today: BuildingStyle::Detached,
        floors: 3,
        size: (27.0, 26.0),
        left: 13.5,
        parts: 1,
        garden: 12.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Casa canaria (taller)",
        name: "Casa canaria (taller)",
        values: &[("width", 11.0), ("depth", 10.0), ("variant", 1.0)],
        today: BuildingStyle::Detached,
        floors: 2,
        size: (17.0, 15.0),
        left: 8.5,
        parts: 1,
        garden: 5.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Casa payesa ibicenca (taller)",
        name: "Casa payesa ibicenca (taller)",
        values: &[("variant", 1.0)],
        today: BuildingStyle::Courtyard,
        floors: 2,
        size: (22.0, 14.0),
        left: 10.0,
        parts: 1,
        garden: 5.0,
        tanda: 4,
        gap: 6.0,
    },
    Building {
        family: "Torre de oficinas (taller)",
        name: "Torre de oficinas (taller)",
        values: &[
            ("width", 24.0),
            ("depth", 20.0),
            ("floors", 12.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::Offices,
        floors: 12,
        size: (32.0, 30.0),
        left: 16.0,
        parts: 1,
        garden: 10.0,
        tanda: 5,
        gap: 6.0,
    },
    Building {
        family: "Oficinas de los 70 (taller)",
        name: "Oficinas de los 70 (taller)",
        values: &[("width", 30.0), ("depth", 14.0), ("variant", 1.0)],
        today: BuildingStyle::Offices,
        floors: 6,
        size: (38.0, 20.0),
        left: 17.0,
        parts: 1,
        garden: 6.0,
        tanda: 5,
        gap: 6.0,
    },
    Building {
        family: "Edificio de parque empresarial (taller)",
        name: "Edificio de parque empresarial (taller)",
        values: &[("width", 36.0), ("depth", 16.0), ("variant", 1.0)],
        today: BuildingStyle::Offices,
        floors: 3,
        size: (40.0, 34.0),
        left: 20.0,
        parts: 1,
        garden: 18.0,
        tanda: 5,
        gap: 6.0,
    },
    Building {
        family: "Edificio con cafetería en esquina (taller)",
        name: "Edificio con cafetería en esquina (taller)",
        values: &[("width", 14.0), ("depth", 12.0), ("variant", 1.0)],
        today: BuildingStyle::ShopAndFlats,
        floors: 4,
        size: (19.0, 17.0),
        left: 7.0,
        parts: 1,
        garden: 5.0,
        tanda: 5,
        gap: 2.0,
    },
    Building {
        family: "Comercio con viviendas (taller)",
        name: "Comercio con viviendas · farmacia (taller)",
        values: &[
            ("width", 10.0),
            ("depth", 12.0),
            ("shop", 0.0),
            ("variant", 1.0),
        ],
        today: BuildingStyle::ShopAndFlats,
        floors: 4,
        size: (10.0, 15.0),
        left: 5.0,
        parts: 1,
        garden: 3.0,
        tanda: 5,
        gap: 0.0,
    },
    Building {
        family: "Comercio con viviendas (taller)",
        name: "Comercio con viviendas · banco (taller)",
        values: &[
            ("width", 10.0),
            ("depth", 12.0),
            ("shop", 1.0),
            ("variant", 2.0),
        ],
        today: BuildingStyle::ShopAndFlats,
        floors: 4,
        size: (10.0, 15.0),
        left: 5.0,
        parts: 1,
        garden: 3.0,
        tanda: 5,
        gap: 0.0,
    },
    Building {
        family: "Comercio con viviendas (taller)",
        name: "Comercio con viviendas · panadería (taller)",
        values: &[
            ("width", 10.0),
            ("depth", 12.0),
            ("shop", 2.0),
            ("variant", 3.0),
        ],
        today: BuildingStyle::ShopAndFlats,
        floors: 4,
        size: (10.0, 15.0),
        left: 5.0,
        parts: 1,
        garden: 3.0,
        tanda: 5,
        gap: 0.0,
    },
    Building {
        family: "Comercio con viviendas (taller)",
        name: "Comercio con viviendas · moda (taller)",
        values: &[
            ("width", 10.0),
            ("depth", 12.0),
            ("shop", 3.0),
            ("variant", 4.0),
        ],
        today: BuildingStyle::ShopAndFlats,
        floors: 4,
        size: (10.0, 15.0),
        left: 5.0,
        parts: 1,
        garden: 3.0,
        tanda: 5,
        gap: 6.0,
    },
    Building {
        family: "Hotel urbano (taller)",
        name: "Hotel urbano (taller)",
        values: &[("width", 22.0), ("depth", 16.0), ("variant", 1.0)],
        today: BuildingStyle::ShopAndFlats,
        floors: 8,
        size: (22.0, 21.0),
        left: 11.0,
        parts: 1,
        garden: 5.0,
        tanda: 5,
        gap: 6.0,
    },
    Building {
        family: "Supermercado (taller)",
        name: "Supermercado (taller)",
        values: &[("width", 40.0), ("depth", 30.0), ("variant", 1.0)],
        today: BuildingStyle::Warehouse,
        floors: 1,
        size: (44.0, 52.0),
        left: 22.0,
        parts: 1,
        garden: 22.0,
        tanda: 5,
        gap: 0.0,
    },
];

/// The family of a building, from its recipe, with the materials it
/// names added to the project from the palette.
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
        .ok_or_else(|| ModelError::NotAnObject(name.into()))?;
    for line in source.lines() {
        if let Some(at) = line.find(" material = \"") {
            let rest = &line[at + " material = \"".len()..];
            if let Some(end) = rest.find('"') {
                crate::palette_material(m, &rest[..end]);
            }
        }
    }
    m.add_family(source)
}

/// The type of a building (or the one already there).
pub fn building_type(m: &mut Model, b: &Building) -> Result<ObjectTypeId, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == b.name) {
        return Ok(t.id);
    }
    let f = family(m, b.family)?;
    let mut v = m.family_defaults(f, ParamScope::Type)?;
    for (key, n) in b.values {
        let val = match v.get(*key) {
            Some(Value::Bool(_)) => Value::Bool(*n > 0.5),
            _ => Value::Number(*n),
        };
        v.insert((*key).into(), val);
    }
    m.add_object_type(b.name, f, v)
}

/// The street of the workshop: the new buildings on the south side, their
/// fronts to the north, and today's buildings of the same kinds opposite.
pub fn project() -> Result<Model, ModelError> {
    let mut m = crate::new_project("Taller de edificios");
    let level = m.levels()[0].id;
    let find = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
    let asphalt = find(&m, "asphalt")
        .or_else(|| find(&m, "concrete"))
        .or_else(|| m.materials().next().map(|x| x.id))
        .ok_or_else(|| ModelError::NotAnObject("asfalto".into()))?;
    let stone = find(&m, "stone").unwrap_or(asphalt);
    let paving = find(&m, "ceramic").unwrap_or(stone);
    let length: f64 = BUILDINGS.iter().map(|b| b.size.0 + b.gap).sum();
    let section = archisim_core::street_types()
        .into_iter()
        .find(|t| t.name.starts_with("Calle de barrio"))
        .or_else(|| archisim_core::street_types().into_iter().next())
        .ok_or_else(|| ModelError::NotAnObject("tipos de calle".into()))?
        .section;
    let half = section.width() / 2.0;
    m.add_road(NewRoad {
        name: "Calle del taller".into(),
        level,
        axis: vec![Point2::new(-8.0, 0.0), Point2::new(length + 8.0, 0.0)],
        section,
        top_offset: 0.0,
        profile: Vec::new(),
        changes: Vec::new(),
        pavement: asphalt,
        kerb_material: stone,
        sidewalk_material: paving,
    })?;
    let mut x = 0.0;
    for b in BUILDINGS {
        let t = building_type(&mut m, b)?;
        let (w, d) = b.size;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(x + b.left, -half - 0.3 - d),
            rotation: 0.0,
            base_offset: 0.0,
        })?;
        // Today's, opposite: the same plan without the garden.
        let depth = if b.today == BuildingStyle::Terraced {
            d - 3.0
        } else {
            d
        };
        let y0 = half + 0.3;
        let parts = if b.today == BuildingStyle::Terraced {
            4
        } else {
            1
        };
        let pw = w / parts as f64;
        for k in 0..parts {
            let x0 = x + pw * k as f64;
            let name = format!("Hoy: {} {}", b.today.name(), m.env_buildings().count() + 1);
            m.add_env_building(NewEnvBuilding {
                name,
                level,
                outline: vec![
                    Point2::new(x0, y0),
                    Point2::new(x0 + pw, y0),
                    Point2::new(x0 + pw, y0 + depth),
                    Point2::new(x0, y0 + depth),
                ],
                style: b.today,
                floors: Some(b.floors as u32),
                floor_height: None,
                base_offset: 0.0,
            })?;
        }
        x += w + b.gap;
    }
    Ok(m)
}

/// A type of a family of the workshop with some values (or the one of
/// that name already there).
fn type_with(
    m: &mut Model,
    family_name: &str,
    name: &str,
    values: &[(&str, f64)],
) -> Result<ObjectTypeId, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let f = family(m, family_name)?;
    let mut v = m.family_defaults(f, ParamScope::Type)?;
    for (key, n) in values {
        v.insert((*key).into(), Value::Number(*n));
    }
    m.add_object_type(name, f, v)
}

/// One row of the world's workshop: what it is and where it goes.
pub struct WorldPiece {
    pub skin: usize,
    pub family: &'static str,
    pub name: String,
    pub values: Vec<(&'static str, f64)>,
    /// Its origin (the middle of its back) and its plan.
    pub at: (f64, f64),
    pub size: (f64, f64),
}

/// The buildings of the world's workshop: a row for each place, west to
/// east three houses of its street side by side (three variants), its
/// block of flats and its detached house in its plot; the rows from north
/// to south. `only` keeps the places named by their key.
pub fn world_pieces(only: &[&str]) -> Vec<WorldPiece> {
    use archisim_core::props::fine::buildings::world::{types::usual_width, SKINS};
    let mut out = Vec::new();
    let mut y = 0.0;
    for (i, sk) in SKINS.iter().enumerate() {
        if !only.is_empty() && !only.contains(&sk.key) {
            continue;
        }
        let w = usual_width(sk);
        let d = 12.0;
        let mut x = 0.0;
        for v in 1..=3 {
            out.push(WorldPiece {
                skin: i,
                family: WORLD_TOWNHOUSE,
                name: format!("Entre medianeras · {} · {v} (taller)", sk.name),
                values: vec![
                    ("skin", i as f64),
                    ("variant", v as f64),
                    ("width", w),
                    ("depth", d),
                    ("floors", sk.floors as f64),
                ],
                at: (x + w / 2.0, y),
                size: (w, d),
            });
            x += w;
        }
        x += 6.0;
        let (bw, bd) = (22.0, 14.0);
        out.push(WorldPiece {
            skin: i,
            family: WORLD_BLOCK,
            name: format!("Bloque · {} (taller)", sk.name),
            values: vec![
                ("skin", i as f64),
                ("variant", 1.0),
                ("width", bw),
                ("depth", bd),
                ("floors", sk.block.floors as f64),
            ],
            at: (x + bw / 2.0, y),
            size: (bw, bd),
        });
        x += bw + 9.0;
        let (hw, hd) = (10.0, 8.0);
        out.push(WorldPiece {
            skin: i,
            family: WORLD_HOUSE,
            name: format!("Vivienda aislada · {} (taller)", sk.name),
            values: vec![
                ("skin", i as f64),
                ("variant", 1.0),
                ("width", hw),
                ("depth", hd),
                ("floors", sk.house.floors as f64),
            ],
            at: (x + hw / 2.0, y + 3.0),
            size: (hw, hd),
        });
        x += hw + 12.0;
        // Tanda 8: the tower, the courtyard house and the row.
        let (tw, td) = (24.0, 20.0);
        out.push(WorldPiece {
            skin: i,
            family: WORLD_TOWER,
            name: format!("Torre · {} (taller)", sk.name),
            values: vec![
                ("skin", i as f64),
                ("variant", 1.0),
                ("width", tw),
                ("depth", td),
                ("floors", sk.tower.floors as f64),
            ],
            at: (x + tw / 2.0, y - 6.0),
            size: (tw, td),
        });
        x += tw + 8.0;
        let (cw, cd) = (16.0, 18.0);
        out.push(WorldPiece {
            skin: i,
            family: WORLD_COURTYARD,
            name: format!("Casa patio · {} (taller)", sk.name),
            values: vec![
                ("skin", i as f64),
                ("variant", 1.0),
                ("width", cw),
                ("depth", cd),
                ("floors", sk.house.floors.clamp(1, 2) as f64),
            ],
            at: (x + cw / 2.0, y - 4.0),
            size: (cw, cd),
        });
        x += cw + 8.0;
        let unit = (usual_width(sk)).min(9.0);
        let (rw, rd) = (unit * 4.0, 10.0);
        out.push(WorldPiece {
            skin: i,
            family: WORLD_ROW,
            name: format!("Casas en hilera · {} (taller)", sk.name),
            values: vec![
                ("skin", i as f64),
                ("variant", 1.0),
                ("width", rw),
                ("depth", rd),
                ("floors", sk.floors.clamp(2, 3) as f64),
            ],
            at: (x + rw / 2.0, y),
            size: (rw, rd),
        });
        // Tanda 9: the place as built today, abandoned and being built.
        let yy = y - 40.0;
        let mut x = 0.0;
        let era: [(&str, &str, f64, f64, f64, f64, usize); 6] = [
            (
                "Entre medianeras de hoy",
                WORLD_TOWNHOUSE,
                1.0,
                0.0,
                w,
                d,
                sk.floors,
            ),
            (
                "Bloque de hoy",
                WORLD_BLOCK,
                1.0,
                0.0,
                bw,
                bd,
                sk.block.floors,
            ),
            (
                "Vivienda aislada de hoy",
                WORLD_HOUSE,
                1.0,
                0.0,
                hw,
                hd,
                sk.house.floors,
            ),
            (
                "Entre medianeras abandonada",
                WORLD_TOWNHOUSE,
                0.0,
                1.0,
                w,
                d,
                sk.floors,
            ),
            (
                "Bloque abandonado",
                WORLD_BLOCK,
                0.0,
                1.0,
                bw,
                bd,
                sk.block.floors,
            ),
            (
                "Bloque en obra",
                WORLD_BLOCK,
                0.0,
                2.0,
                bw,
                bd,
                sk.block.floors,
            ),
        ];
        for (what, family, e, st, pw, pd, floors) in era {
            out.push(WorldPiece {
                skin: i,
                family,
                name: format!("{what} · {} (taller)", sk.name),
                values: vec![
                    ("skin", i as f64),
                    ("variant", 2.0),
                    ("era", e),
                    ("state", st),
                    ("width", pw),
                    ("depth", pd),
                    ("floors", floors as f64),
                ],
                at: (x + pw / 2.0, yy),
                size: (pw, pd),
            });
            x += pw + 10.0;
        }
        y -= 90.0;
    }
    out
}

/// The samples over plans that are not rectangles, for a place: the three
/// typologies over the three plans, full (north) and light (south).
pub fn sample_pieces(skin: usize) -> Vec<WorldPiece> {
    let mut out = Vec::new();
    for lod in 0..2 {
        for t in 0..3 {
            for plan in 0..3 {
                out.push(WorldPiece {
                    skin,
                    family: WORLD_SAMPLE,
                    name: format!("Muestra · lugar {skin} · tipología {t} · planta {plan} · versión {lod} (taller)"),
                    values: vec![
                        ("skin", skin as f64),
                        ("variant", 1.0),
                        ("typology", t as f64),
                        ("plan", plan as f64),
                        ("lod", lod as f64),
                    ],
                    at: (30.0 * plan as f64, -30.0 * t as f64 - 110.0 * lod as f64),
                    size: (20.0, 18.0),
                });
            }
        }
    }
    out
}

/// The samples of a place as a project.
pub fn sample_project(skin: usize) -> Result<Model, ModelError> {
    let mut m = crate::new_project("Muestras de plantas del mundo");
    let level = m.levels()[0].id;
    for p in sample_pieces(skin) {
        let t = type_with(&mut m, p.family, &p.name, &p.values)?;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(p.at.0, p.at.1),
            rotation: 0.0,
            base_offset: 0.0,
        })?;
    }
    Ok(m)
}

/// The world's workshop as a project (see [`world_pieces`]).
pub fn world_project(only: &[&str]) -> Result<Model, ModelError> {
    let mut m = crate::new_project("Taller de edificios del mundo");
    let level = m.levels()[0].id;
    for p in world_pieces(only) {
        let t = type_with(&mut m, p.family, &p.name, &p.values)?;
        m.add_object(NewObject {
            object_type: t,
            level,
            position: Point2::new(p.at.0, p.at.1),
            rotation: 0.0,
            base_offset: 0.0,
        })?;
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The world's workshop is made, and every role of its buildings has
    /// its material in the recipe.
    #[test]
    fn the_world_is_made() {
        let m = world_project(&["andalucia", "paris", "japon"]).unwrap();
        assert_eq!(m.objects().count(), 42);
        for t in m.object_types().filter(|t| t.name.ends_with("(taller)")) {
            let (mesh, _) = m.type_prop(t.id).expect("a model");
            let family = m.family(t.family).unwrap().name().to_string();
            let recipe = RECIPES
                .iter()
                .find(|r| r.lines().next().is_some_and(|l| l.contains(&family)))
                .unwrap();
            for (role, _) in &mesh.parts {
                assert!(
                    recipe.contains(&format!("    {role} material")),
                    "{}: the role «{role}» has no material",
                    t.name
                );
            }
        }
    }

    #[test]
    fn the_street_is_made() {
        let m = project().unwrap();
        assert_eq!(m.objects().count(), BUILDINGS.len());
        for b in BUILDINGS {
            let t = m.object_types().find(|t| t.name == b.name).unwrap().id;
            let (mesh, _) = m.type_prop(t).expect("a model");
            assert!(mesh.positions.len() > 1000, "{}", b.name);
            // Every piece of the model has its material in the recipe.
            let recipe = RECIPES
                .iter()
                .find(|r| r.lines().next().is_some_and(|l| l.contains(b.family)))
                .unwrap();
            for (role, _) in &mesh.parts {
                assert!(
                    recipe.contains(&format!("    {role} material")),
                    "{}: the role «{role}» has no material",
                    b.family
                );
            }
        }
        // Every material a recipe names is in the palette.
        for r in RECIPES {
            for line in r.lines() {
                if let Some(at) = line.find(" material = \"") {
                    let rest = &line[at + " material = \"".len()..];
                    let name = &rest[..rest.find('"').unwrap()];
                    assert!(
                        crate::COZY_PALETTE.iter().any(|e| e.0 == name)
                            || crate::stylized::stylized(name).is_some(),
                        "{name}"
                    );
                }
            }
        }
    }
}
