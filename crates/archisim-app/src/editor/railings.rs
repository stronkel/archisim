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
}

impl Default for RailingSettings {
    fn default() -> Self {
        // 1,10 m: the CTE height where the drop may exceed 6 m.
        Self {
            height: 1.1,
            base_offset: 0.0,
            post_spacing: 1.5,
            glass: false,
        }
    }
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
            m.add_railing(NewRailing {
                level,
                path,
                base_offset: s.base_offset,
                height: s.height,
                post_spacing: s.post_spacing,
                material,
                glass,
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
