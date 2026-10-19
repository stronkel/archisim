//! Pitched roofs: placing them (over the building by clicking inside it,
//! or by their footprint) and changing them, each change one undoable step.

use archisim_core::{LevelId, NewRoof, Point2, Roof, RoofId, SlabTypeId};
use archisim_geom::{room_outline, Contour};

use super::{EditResult, Editor};

/// The usual shapes of a roof, to set the pitched sides at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoofKind {
    /// Every side pitched ("a cuatro aguas" on a rectangle).
    Hip,
    /// The sides along the longest one pitched, the others gables ("a
    /// dos aguas").
    Gable,
    /// One side pitched, rising from it ("a un agua").
    Shed,
}

/// Settings of the roof tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoofSettings {
    pub roof_type: SlabTypeId,
    pub kind: RoofKind,
    /// Rise per run (0.3 = 30 %).
    pub slope: f64,
    pub overhang: f64,
    /// Height of the top face along the footprint, above the roof's level.
    pub base_offset: f64,
    /// The roof belongs to the level above the one it is drawn on (it
    /// covers its walls), as beams; off: to the active level.
    pub level_above: bool,
    /// A click inside the building takes the outer faces of its walls.
    pub wand: bool,
}

/// The pitch of each side of a footprint for a kind of roof.
pub fn slopes_for(kind: RoofKind, outline: &[Point2], slope: f64) -> Vec<Option<f64>> {
    let n = outline.len();
    let side = |i: usize| outline[(i + 1) % n] - outline[i];
    let longest = (0..n)
        .max_by(|a, b| {
            // The first of the longest.
            side(*a)
                .length()
                .total_cmp(&side(*b).length())
                .then(b.cmp(a))
        })
        .unwrap_or(0);
    let along = side(longest).normalized().unwrap_or(Point2::new(1.0, 0.0));
    (0..n)
        .map(|i| match kind {
            RoofKind::Hip => Some(slope),
            RoofKind::Gable => {
                let u = side(i).normalized().unwrap_or(along);
                (u.dot(along).abs() > 0.999).then_some(slope)
            }
            RoofKind::Shed => (i == longest).then_some(slope),
        })
        .collect()
}

impl Editor {
    /// The level a new roof goes to from the active one.
    pub fn roof_level(&self, s: &RoofSettings) -> LevelId {
        if s.level_above {
            self.model().level_above(self.level).unwrap_or(self.level)
        } else {
            self.level
        }
    }

    /// A roof over the building around `p`: the outer faces of the walls
    /// of the active level.
    pub fn add_roof_over(&mut self, s: &RoofSettings, p: Point2) -> EditResult<RoofId> {
        let outline = room_outline(self.model(), self.level, p, Contour::ExteriorFace)
            .map_err(|e| crate::units::explain_room(&e))?;
        self.add_roof(s, outline)
    }

    /// A roof on a footprint.
    pub fn add_roof(&mut self, s: &RoofSettings, outline: Vec<Point2>) -> EditResult<RoofId> {
        let level = self.roof_level(s);
        let mut outline = outline;
        if archisim_core::signed_area(&outline) < 0.0 {
            outline.reverse();
        }
        let slopes = slopes_for(s.kind, &outline, s.slope);
        self.transact("Crear cubierta", |m| {
            m.add_roof(NewRoof {
                roof_type: s.roof_type,
                level,
                outline,
                slopes,
                base_offset: s.base_offset,
                overhang: s.overhang,
            })
        })
    }

    pub fn edit_roof(
        &mut self,
        label: &str,
        id: RoofId,
        edit: impl FnOnce(&mut Roof),
    ) -> EditResult {
        self.transact(label, |m| m.edit_roof(id, edit))
    }

    /// Gives a roof the pitched sides of a kind, at `slope`.
    pub fn set_roof_kind(&mut self, id: RoofId, kind: RoofKind, slope: f64) -> EditResult {
        let Some(r) = self.model().roof(id) else {
            return Err("la cubierta ya no existe".into());
        };
        let slopes = slopes_for(kind, &r.outline, slope);
        self.edit_roof("Forma de la cubierta", id, |r| r.slopes = slopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 6.0),
            Point2::new(0.0, 6.0),
        ]
    }

    #[test]
    fn kinds_of_roof() {
        let r = rect();
        assert_eq!(slopes_for(RoofKind::Hip, &r, 0.3), vec![Some(0.3); 4]);
        assert_eq!(
            slopes_for(RoofKind::Gable, &r, 0.3),
            vec![Some(0.3), None, Some(0.3), None]
        );
        assert_eq!(
            slopes_for(RoofKind::Shed, &r, 0.3),
            vec![Some(0.3), None, None, None]
        );
    }

    #[test]
    fn roofs_with_the_editor() {
        let mut ed = Editor::new_project();
        let pb = ed.level;
        let p1 = ed.add_level("Cubierta", 3.0).unwrap();
        ed.level = pb;
        let t = ed.model().slab_types().next().unwrap().id;
        let s = RoofSettings {
            roof_type: t,
            kind: RoofKind::Gable,
            slope: 0.3,
            overhang: 0.5,
            base_offset: 0.0,
            level_above: true,
            wand: false,
        };
        let id = ed.add_roof(&s, rect()).unwrap();
        assert_eq!(ed.model().roof(id).unwrap().level, p1);
        assert_eq!(ed.model().roof(id).unwrap().planes().len(), 2);
        ed.set_roof_kind(id, RoofKind::Hip, 0.4).unwrap();
        assert_eq!(ed.model().roof(id).unwrap().planes().len(), 4);
        assert!(ed.edit_roof("Alero", id, |r| r.overhang = -1.0).is_err());
        ed.undo();
        assert_eq!(ed.model().roof(id).unwrap().planes().len(), 2);
    }
}
