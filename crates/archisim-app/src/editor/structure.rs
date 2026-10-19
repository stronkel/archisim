//! Columns and beams: placing them and changing them, each change one
//! undoable step.

use archisim_core::{
    BeamId, ColumnId, LevelId, MaterialId, MemberTypeId, MemberUsage, Model, NewBeam, NewColumn,
    Point2, Profile, WallTop,
};

use super::{EditResult, Editor, TopChoice};
use crate::pick::Element;

/// Settings of the column tool (what the next column will be).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnSettings {
    pub member_type: MemberTypeId,
    /// Turn of the section, in radians.
    pub rotation: f64,
    pub base_offset: f64,
    /// Height of columns without a top level.
    pub height: f64,
    pub top: TopChoice,
    pub top_offset: f64,
}

impl ColumnSettings {
    pub fn top_on(&self, model: &Model, level: LevelId) -> WallTop {
        let target = match self.top {
            TopChoice::Unconnected => None,
            TopChoice::NextLevel => model.level_above(level),
            TopChoice::Level(l) => Some(l),
        };
        match target {
            Some(level) => WallTop::Level {
                level,
                offset: self.top_offset,
            },
            None => WallTop::Unconnected,
        }
    }
}

/// Settings of the beam tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamSettings {
    pub member_type: MemberTypeId,
    /// The beam belongs to the level above the one being drawn on (it
    /// carries its floor), as beams are drawn on a plan under them; off:
    /// to the active level.
    pub level_above: bool,
    /// Height of the top face above its level (negative: below it).
    pub top_offset: f64,
}

/// The first type of a usage, preferring concrete and the given name.
pub fn default_member_type(model: &Model, usage: MemberUsage) -> MemberTypeId {
    model
        .member_types()
        .find(|t| t.usage == usage)
        .map_or(MemberTypeId(0), |t| t.id)
}

impl Editor {
    /// Places a column on the active level.
    pub fn add_column(&mut self, s: &ColumnSettings, at: Point2) -> EditResult<ColumnId> {
        let level = self.level;
        let top = s.top_on(self.model(), level);
        self.transact("Crear pilar", |m| {
            m.add_column(NewColumn {
                member_type: s.member_type,
                level,
                position: at,
                rotation: s.rotation,
                base_offset: s.base_offset,
                top,
                height: s.height,
            })
        })
    }

    /// The level a new beam goes to from the active one.
    pub fn beam_level(&self, s: &BeamSettings) -> LevelId {
        if s.level_above {
            self.model().level_above(self.level).unwrap_or(self.level)
        } else {
            self.level
        }
    }

    /// Draws a beam from `a` to `b`.
    pub fn add_beam(&mut self, s: &BeamSettings, a: Point2, b: Point2) -> EditResult<BeamId> {
        let level = self.beam_level(s);
        self.transact("Crear viga", |m| {
            m.add_beam(NewBeam {
                member_type: s.member_type,
                level,
                start: a,
                end: b,
                top_offset: s.top_offset,
            })
        })
    }

    /// Changes the type of the selected columns or beams (each keeps to
    /// types of its own usage).
    pub fn set_members_type(&mut self, items: &[Element], t: MemberTypeId) -> EditResult {
        self.transact("Cambiar tipo", |m| {
            for e in items {
                match e {
                    Element::Column(id) => m.edit_column(*id, |c| c.member_type = t)?,
                    Element::Beam(id) => m.edit_beam(*id, |b| b.member_type = t)?,
                    _ => {}
                }
            }
            Ok(())
        })
    }

    pub fn edit_columns(
        &mut self,
        label: &str,
        columns: &[ColumnId],
        edit: impl Fn(&mut archisim_core::Column),
    ) -> EditResult {
        self.transact(label, |m| {
            columns.iter().try_for_each(|c| m.edit_column(*c, &edit))
        })
    }

    pub fn edit_beams(
        &mut self,
        label: &str,
        beams: &[BeamId],
        edit: impl Fn(&mut archisim_core::Beam),
    ) -> EditResult {
        self.transact(label, |m| {
            beams.iter().try_for_each(|b| m.edit_beam(*b, &edit))
        })
    }

    /// Turns the selected columns a quarter (Space), or flips the selected
    /// beams end for end.
    pub fn turn_members(&mut self, items: &[Element]) -> EditResult<usize> {
        let n = items
            .iter()
            .filter(|e| matches!(e, Element::Column(_) | Element::Beam(_)))
            .count();
        if n == 0 {
            return Ok(0);
        }
        self.transact("Girar", |m| {
            for e in items {
                match e {
                    Element::Column(id) => m.edit_column(*id, |c| {
                        c.rotation = normal_angle(c.rotation + std::f64::consts::FRAC_PI_2)
                    })?,
                    Element::Beam(id) => {
                        m.edit_beam(*id, |b| std::mem::swap(&mut b.start, &mut b.end))?
                    }
                    _ => {}
                }
            }
            Ok(())
        })?;
        Ok(n)
    }

    // ---- Types ---------------------------------------------------------

    /// Copies a column or beam type under a new name.
    pub fn duplicate_member_type(&mut self, id: MemberTypeId) -> EditResult<MemberTypeId> {
        let model = self.model();
        let Some(t) = model.member_type(id) else {
            return Err("el tipo no existe".into());
        };
        let taken: Vec<String> = model.member_types().map(|t| t.name.clone()).collect();
        let name = super::types::copy_name(&t.name, &taken);
        self.transact("Duplicar tipo", |m| m.duplicate_member_type(id, name))
    }

    /// Changes a column or beam type; all its elements follow.
    pub fn edit_member_type(
        &mut self,
        id: MemberTypeId,
        name: &str,
        material: MaterialId,
        profile: Profile,
    ) -> EditResult {
        let name = name.trim();
        if name.is_empty() {
            return Err("el tipo necesita un nombre".into());
        }
        if self
            .model()
            .member_types()
            .any(|t| t.id != id && t.name == name)
        {
            return Err(format!("ya hay un tipo llamado «{name}»"));
        }
        self.transact("Modificar tipo", |m| {
            m.edit_member_type(id, name, material, profile)
        })
    }

    pub fn remove_member_type(&mut self, id: MemberTypeId) -> EditResult {
        let users = self.model().member_type_users(id);
        if users > 0 {
            return Err(format!("lo usan {users} elemento(s)"));
        }
        self.transact("Borrar tipo", |m| m.remove_member_type(id).map(|_| ()))
    }
}

/// An angle in (-π, π].
pub fn normal_angle(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    let mut r = a.rem_euclid(t);
    if r > std::f64::consts::PI + 1e-12 {
        r -= t;
    }
    if r.abs() < 1e-12 {
        0.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;

    #[test]
    fn columns_and_beams_with_the_editor() {
        let mut ed = Editor::new_project();
        let pb = ed.level;
        let p1 = ed.add_level("Planta 1", 3.0).unwrap();
        ed.level = pb;
        let model = ed.model();
        let col = ColumnSettings {
            member_type: default_member_type(model, MemberUsage::Column),
            rotation: 0.0,
            base_offset: 0.0,
            height: 2.5,
            top: TopChoice::NextLevel,
            top_offset: -0.3,
        };
        let c = ed.add_column(&col, Point2::new(1.0, 1.0)).unwrap();
        assert!((ed.model().column(c).unwrap().height - 2.7).abs() < 1e-9);
        let beam = BeamSettings {
            member_type: default_member_type(ed.model(), MemberUsage::Beam),
            level_above: true,
            top_offset: 0.0,
        };
        let b = ed
            .add_beam(&beam, Point2::new(1.0, 1.0), Point2::new(6.0, 1.0))
            .unwrap();
        // Drawn on the ground floor, it carries the floor above.
        assert_eq!(ed.model().beam(b).unwrap().level, p1);
        assert_eq!(
            ed.turn_members(&[Element::Column(c), Element::Beam(b)]),
            Ok(2)
        );
        assert!(
            (ed.model().column(c).unwrap().rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12
        );
        assert_eq!(ed.model().beam(b).unwrap().start, Point2::new(6.0, 1.0));
        // A beam type is no column type.
        assert!(ed
            .set_members_type(&[Element::Column(c)], beam.member_type)
            .is_err());
        let copy = ed.duplicate_member_type(col.member_type).unwrap();
        let t = ed.model().member_type(copy).unwrap().clone();
        ed.edit_member_type(
            copy,
            "Pilar 35×35",
            t.material,
            Profile::Rectangle {
                width: 0.35,
                depth: 0.35,
            },
        )
        .unwrap();
        ed.set_members_type(&[Element::Column(c)], copy).unwrap();
        assert!(ed.remove_member_type(copy).is_err());
        ed.undo();
        ed.remove_member_type(copy).unwrap();
    }
}
