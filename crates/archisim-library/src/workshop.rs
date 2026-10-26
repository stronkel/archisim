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

/// The workshop's recipes.
pub const RECIPES: &[&str] = &[include_str!("../taller/sofa.asfam")];

/// A piece of the workshop: its type name, its family, its values (sizes
/// as numbers, materials by their name in the cozy palette) and the
/// library's piece it is to replace.
pub struct Piece {
    pub name: &'static str,
    pub family: &'static str,
    pub values: &'static [(&'static str, Val)],
    pub replaces: Option<&'static str>,
}

pub enum Val {
    N(f64),
    M(&'static str),
}

use Val::{M, N};

pub const PIECES: &[Piece] = &[
    Piece {
        name: "Sofá de 3 plazas · lino arena (taller)",
        family: "Sofá (taller)",
        values: &[
            ("width", N(2.2)),
            ("places", N(3.0)),
            ("pillows", N(2.0)),
            ("mat", M("Lino arena")),
            ("cushion", M("Lino arena")),
            ("pillow", M("Terciopelo mostaza")),
            ("frame", M("Roble claro")),
        ],
        replaces: Some("Sofá acogedor de 3 plazas"),
    },
    Piece {
        name: "Sofá de 2 plazas · terciopelo salvia (taller)",
        family: "Sofá (taller)",
        values: &[
            ("width", N(1.7)),
            ("places", N(2.0)),
            ("pillows", N(1.0)),
            ("mat", M("Terciopelo salvia")),
            ("cushion", M("Terciopelo salvia")),
            ("pillow", M("Lino blanco")),
            ("frame", M("Nogal")),
        ],
        replaces: Some("Sofá nórdico con cojines"),
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
    let f = family(m, piece.family)?;
    let mut values = m.family_defaults(f, ParamScope::Type)?;
    for (k, v) in piece.values {
        let v = match v {
            N(n) => Value::Number(*n),
            M(name) => match crate::palette_material(m, name) {
                Some(id) => Value::Material(id.0),
                None => continue,
            },
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
    for piece in PIECES {
        let new = add_piece(&mut m, piece)?;
        let old = match piece.replaces {
            Some(name) => Some(crate::add_object_entry(&mut m, name)?),
            None => None,
        };
        let width = |m: &Model, t: ObjectTypeId| {
            m.object_type_box(t).map(|b| b.1[0] - b.0[0]).unwrap_or(1.0)
        };
        let w = old.map_or(width(&m, new), |o| width(&m, new).max(width(&m, o)));
        for (t, y) in [(Some(new), 0.0), (old, 2.0)] {
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
        end: Point2::new(x - 0.6, 2.3),
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

    /// Every piece of the workshop is made with its own 3D and stays about
    /// as light as the piece it replaces.
    #[test]
    fn the_workshop_pieces_are_made() {
        let m = project().unwrap();
        for piece in PIECES {
            let t = m.object_types().find(|t| t.name == piece.name).unwrap().id;
            assert!(m.type_has_prop(t), "{}", piece.name);
            let (mesh, _) = m.type_prop(t).unwrap();
            let tris = mesh.triangle_count();
            assert!((3_000..25_000).contains(&tris), "{}: {tris}", piece.name);
            if let Some(old) = piece.replaces {
                let o = m.object_types().find(|t| t.name == old).unwrap().id;
                let old_tris = m.type_prop(o).unwrap().0.triangle_count();
                eprintln!("{}: {tris} triángulos (antes {old_tris})", piece.name);
                // No heavier than half again the piece it replaces.
                assert!(
                    tris * 2 < old_tris * 3,
                    "{}: {tris} vs {old_tris}",
                    piece.name
                );
            }
        }
    }
}
