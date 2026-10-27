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
    include_str!("../taller/fregadero.asfam"),
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
        name: "Fregadero de acero 80 (taller)",
        family: "Módulo fregadero (taller)",
        values: &[
            ("width", N(0.8)),
            ("doors", N(2.0)),
            ("mat", M("Roble claro")),
            ("sink", M("Acero inoxidable")),
        ],
        replaces: Some("Bajo fregadero 80"),
        most: 14_000,
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
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_of_a_piece_is_in_the_palette() {
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
