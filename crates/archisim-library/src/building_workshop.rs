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
];

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
            position: Point2::new(x + w / 2.0, -half - 0.3 - d),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_street_is_made() {
        let m = project().unwrap();
        assert_eq!(m.objects().count(), BUILDINGS.len());
        for b in BUILDINGS {
            let t = m.object_types().find(|t| t.name == b.name).unwrap().id;
            let (mesh, _) = m.type_prop(t).expect("a model");
            assert!(mesh.positions.len() > 1000, "{}", b.name);
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
