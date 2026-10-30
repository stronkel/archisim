//! Loose railings, each change one undoable step.

use archisim_core::{MaterialId, Model, NewRailing, Point2, Railing, RailingId};

use super::{EditResult, Editor};

/// Settings of the railing tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailingSettings {
    pub height: f64,
    pub base_offset: f64,
    pub post_spacing: f64,
    /// Glass panels instead of bars.
    pub glass: bool,
    /// The type of the railings it draws (E12); none: the project's
    /// first («Barandilla · Contemporáneo» if it has it), unless `plain`.
    pub railing_type: Option<archisim_core::ObjectTypeId>,
    /// Plain posts, rails and bars, with no type.
    pub plain: bool,
}

impl Default for RailingSettings {
    fn default() -> Self {
        // 1,10 m: the CTE height where the drop may exceed 6 m.
        Self {
            height: 1.1,
            base_offset: 0.0,
            post_spacing: 1.5,
            glass: false,
            railing_type: None,
            plain: false,
        }
    }
}

/// The railing type new railings take when none is chosen: the
/// contemporary one of the template, or the project's first.
pub fn default_railing_type(m: &Model) -> Option<archisim_core::ObjectTypeId> {
    let types = m.railing_types();
    types
        .iter()
        .find(|t| t.name == "Barandilla · Contemporáneo")
        .or(types.first())
        .map(|t| t.id)
}

/// The model's steel, or added.
fn steel(model: &mut Model) -> MaterialId {
    let found = model
        .materials()
        .find(|m| m.category == "steel")
        .map(|m| m.id);
    match found {
        Some(m) => m,
        None => model.add_material(
            "Acero S275",
            "steel",
            [120, 126, 134],
            archisim_core::CutPattern::Solid,
        ),
    }
}

/// The model's glass, or added.
pub(crate) fn glass(model: &mut Model) -> MaterialId {
    let found = model
        .materials()
        .find(|m| m.category == "glass")
        .map(|m| m.id);
    match found {
        Some(m) => m,
        None => model.add_material(
            "Vidrio laminado",
            "glass",
            [190, 222, 236],
            archisim_core::CutPattern::Solid,
        ),
    }
}

impl Editor {
    /// Glass panels (true) or bars (false) for a railing.
    pub fn set_railing_glass(&mut self, id: RailingId, on: bool) -> EditResult {
        self.transact("Barandilla de vidrio", |m| {
            let g = on.then(|| glass(m));
            m.edit_railing(id, |r| r.glass = g)
        })
    }

    /// A railing along `path`, on the active level.
    pub fn add_railing(&mut self, s: &RailingSettings, path: Vec<Point2>) -> EditResult<RailingId> {
        let level = self.level;
        self.transact("Crear barandilla", |m| {
            let material = steel(m);
            let glass = s.glass.then(|| glass(m));
            let railing_type = if s.plain {
                None
            } else {
                s.railing_type
                    .filter(|t| m.is_railing_type(*t))
                    .or_else(|| default_railing_type(m))
            };
            m.add_railing(NewRailing {
                level,
                path,
                base_offset: s.base_offset,
                height: s.height,
                post_spacing: s.post_spacing,
                material,
                glass,
                railing_type,
            })
        })
    }

    pub fn edit_railing(
        &mut self,
        label: &str,
        id: RailingId,
        edit: impl FnOnce(&mut Railing),
    ) -> EditResult {
        self.transact(label, |m| m.edit_railing(id, edit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new railing takes the contemporary type unless asked for a plain
    /// one; its type can be changed and its runs are made by the type.
    #[test]
    fn new_railings_take_a_type() {
        let mut ed = Editor::new_project();
        let path = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 2.0),
        ];
        let s = RailingSettings::default();
        let id = ed.add_railing(&s, path.clone()).unwrap();
        let t = ed.model().railing(id).unwrap().railing_type.unwrap();
        assert_eq!(
            ed.model().object_type(t).unwrap().name,
            "Barandilla · Contemporáneo"
        );
        let solids = archisim_geom::solid::loose_railing_solids(ed.model(), id);
        assert!(solids.iter().map(|s| s.triangles.len()).sum::<usize>() > 100);
        // Two runs: the first draws its free end, the second the corner.
        let runs = ed.model().railing(id).unwrap().runs();
        assert_eq!(runs.len(), 2);
        assert_eq!((runs[0].start_end, runs[1].start_end), ((0, 1), (1, 0)));
        let plain = RailingSettings {
            plain: true,
            ..RailingSettings::default()
        };
        let p = ed.add_railing(&plain, path).unwrap();
        assert_eq!(ed.model().railing(p).unwrap().railing_type, None);
        // Another type; the type in use is not purged.
        let other = ed.model().railing_types()[0].id;
        ed.edit_railing("Tipo", p, |r| r.railing_type = Some(other))
            .unwrap();
        assert_eq!(ed.model().railing(p).unwrap().railing_type, Some(other));
        assert!(ed.model().object_type_users(other) >= 1);
    }
}
