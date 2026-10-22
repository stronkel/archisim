//! Setting-out grids and footings: placing them and changing them, each
//! change one undoable step.

use archisim_core::{
    ColumnId, FootingId, FootingKind, GridId, MaterialId, Model, NewFooting, NewGrid, Point2,
    WallId,
};

use super::{EditResult, Editor};

/// What the footing tool places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootingMode {
    /// Under a column (clicked), or free (a click at its centre).
    Pad,
    /// Under a wall (clicked), or free (two clicks).
    Strip,
    /// Between two points.
    TieBeam,
}

impl FootingMode {
    pub const ALL: [FootingMode; 3] = [FootingMode::Pad, FootingMode::Strip, FootingMode::TieBeam];

    pub fn name(self) -> &'static str {
        match self {
            FootingMode::Pad => "Zapata aislada",
            FootingMode::Strip => "Zapata corrida",
            FootingMode::TieBeam => "Viga de atado",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The size of one kind of footing (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootingSize {
    pub width: f64,
    /// Along the second side (pads only).
    pub length: f64,
    pub depth: f64,
}

/// Settings of the footing tool (what the next footing will be).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootingSettings {
    pub mode: FootingMode,
    pub material: MaterialId,
    /// Size of each kind, by `FootingMode`.
    pub sizes: [FootingSize; 3],
    /// Height of the top face over the level (negative: under the floor).
    pub top_offset: f64,
    /// Turn of a free pad, radians.
    pub rotation: f64,
}

impl FootingSettings {
    pub fn new(model: &Model) -> Self {
        FootingSettings {
            mode: FootingMode::Pad,
            material: concrete(model),
            sizes: [
                FootingSize {
                    width: 1.2,
                    length: 1.2,
                    depth: 0.6,
                },
                FootingSize {
                    width: 0.6,
                    length: 0.6,
                    depth: 0.5,
                },
                FootingSize {
                    width: 0.4,
                    length: 0.4,
                    depth: 0.4,
                },
            ],
            top_offset: -0.35,
            rotation: 0.0,
        }
    }

    pub fn size(&self) -> FootingSize {
        self.sizes[self.mode.index()]
    }

    pub fn size_mut(&mut self) -> &mut FootingSize {
        &mut self.sizes[self.mode.index()]
    }

    /// A pad: under `column`, or free at `at`.
    pub fn pad(&self, at: Point2, column: Option<ColumnId>) -> FootingKind {
        FootingKind::Pad {
            position: at,
            length: self.sizes[0].length,
            rotation: self.rotation,
            column,
        }
    }

    /// The footing of the current mode from `a` to `b` (strip or tie
    /// beam), or at `a` (pad).
    pub fn between(&self, a: Point2, b: Point2, wall: Option<WallId>) -> FootingKind {
        match self.mode {
            FootingMode::Pad => self.pad(a, None),
            FootingMode::Strip => FootingKind::Strip {
                start: a,
                end: b,
                wall,
            },
            FootingMode::TieBeam => FootingKind::TieBeam { start: a, end: b },
        }
    }
}

/// The concrete of the project (the first material in that category).
pub fn concrete(model: &Model) -> MaterialId {
    model
        .materials()
        .find(|m| m.category == "concrete")
        .or_else(|| model.materials().next())
        .map_or(MaterialId(0), |m| m.id)
}

impl Editor {
    /// Draws a grid line from `a` to `b`, named after the last one.
    pub fn add_grid(&mut self, a: Point2, b: Point2) -> EditResult<GridId> {
        self.transact("Crear eje", |m| {
            let name = m.next_grid_name(a, b);
            m.add_grid(NewGrid {
                name,
                start: a,
                end: b,
            })
        })
    }

    /// A reference plane from `a` to `b`, named «Referencia N».
    pub fn add_ref_plane(&mut self, a: Point2, b: Point2) -> EditResult<archisim_core::RefPlaneId> {
        self.transact("Crear plano de referencia", |m| m.add_ref_plane("", a, b))
    }

    pub fn edit_grid(
        &mut self,
        label: &str,
        id: GridId,
        edit: impl FnOnce(&mut archisim_core::GridLine),
    ) -> EditResult {
        self.transact(label, |m| m.edit_grid(id, edit))
    }

    /// Places a footing of the tool's size on the active level.
    pub fn add_footing(&mut self, s: &FootingSettings, kind: FootingKind) -> EditResult<FootingId> {
        let level = self.level;
        let size = s.size();
        let width = match kind {
            FootingKind::Pad { .. } => s.sizes[0].width,
            _ => size.width,
        };
        let depth = match kind {
            FootingKind::Pad { .. } => s.sizes[0].depth,
            _ => size.depth,
        };
        self.transact("Crear cimentación", |m| {
            m.add_footing(NewFooting {
                level,
                kind,
                material: s.material,
                width,
                depth,
                top_offset: s.top_offset,
            })
        })
    }

    pub fn edit_footings(
        &mut self,
        label: &str,
        ids: &[FootingId],
        edit: impl Fn(&mut archisim_core::Footing),
    ) -> EditResult {
        self.transact(label, |m| {
            for id in ids {
                m.edit_footing(*id, &edit)?;
            }
            Ok(())
        })
    }

    /// Footings under every column (pads) and wall (strips) of the active
    /// level that has none yet. Returns how many were made.
    pub fn found_level(&mut self, s: &FootingSettings) -> EditResult<usize> {
        let level = self.level;
        let s = *s;
        self.transact("Cimentar la planta", |m| {
            let pads: Vec<ColumnId> = m
                .columns_on_level(level)
                .map(|c| c.id)
                .filter(|c| {
                    !m.footings().any(
                        |f| matches!(f.kind, FootingKind::Pad { column: Some(x), .. } if x == *c),
                    )
                })
                .collect();
            let strips: Vec<WallId> = m
                .walls_on_level(level)
                .filter(|w| w.bulge.abs() < 1e-9)
                .map(|w| w.id)
                .filter(|w| {
                    !m.footings().any(
                        |f| matches!(f.kind, FootingKind::Strip { wall: Some(x), .. } if x == *w),
                    )
                })
                .collect();
            let mut n = 0;
            for c in pads {
                m.add_footing(NewFooting {
                    level,
                    kind: s.pad(Point2::new(0.0, 0.0), Some(c)),
                    material: s.material,
                    width: s.sizes[0].width,
                    depth: s.sizes[0].depth,
                    top_offset: s.top_offset,
                })?;
                n += 1;
            }
            for w in strips {
                m.add_footing(NewFooting {
                    level,
                    kind: FootingKind::Strip {
                        start: Point2::new(0.0, 0.0),
                        end: Point2::new(1.0, 0.0),
                        wall: Some(w),
                    },
                    material: s.material,
                    width: s.sizes[1].width,
                    depth: s.sizes[1].depth,
                    top_offset: s.top_offset,
                })?;
                n += 1;
            }
            Ok(n)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;

    #[test]
    fn a_level_is_founded_once() {
        let mut ed = Editor::new_project();
        let s = FootingSettings::new(ed.model());
        let ws = crate::editor::WallSettings {
            wall_type: ed.model().wall_types().next().unwrap().id,
            reference: archisim_core::ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            top: crate::editor::TopChoice::Unconnected,
            top_offset: 0.0,
        };
        ed.add_wall(&ws, Point2::new(0.0, 0.0), Point2::new(6.0, 0.0))
            .unwrap();
        assert_eq!(ed.found_level(&s).unwrap(), 1);
        assert_eq!(ed.found_level(&s).unwrap(), 0);
        let f = ed.model().footings().next().unwrap();
        assert_eq!(f.outline().len(), 4);
        let g = ed
            .add_grid(Point2::new(0.0, -1.0), Point2::new(0.0, 5.0))
            .unwrap();
        assert_eq!(ed.model().grid(g).unwrap().name, "1");
    }
}
