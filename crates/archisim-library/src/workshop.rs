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

/// A kitchen of the workshop (paquete 2a): one collection's materials
/// for every module of its run.
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
        if let Some(id) = crate::palette_material(m, mat) {
            v.insert((*key).into(), Value::Material(id.0));
        }
    }
    for (key, val) in values {
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
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_of_a_piece_is_in_the_palette() {
        for k in KITCHENS {
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
                    m.object_type(o.object_type)
                        .is_some_and(|t| t.name.contains(k.collection))
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
