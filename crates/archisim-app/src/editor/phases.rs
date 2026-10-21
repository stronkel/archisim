//! Phases of a refurbishment: what new elements are drawn as, and the
//! phase of selected elements.

use super::{EditResult, Editor};

impl Editor {
    /// The phase of a refurbishment new elements are drawn in.
    pub fn draw_phase(&self) -> archisim_core::Phase {
        self.model().draw_phase
    }

    pub fn set_draw_phase(&mut self, phase: archisim_core::Phase) {
        self.doc.set_draw_phase(phase);
    }

    /// Sets the phase of walls, slabs, doors and windows, columns and
    /// beams; returns how many changed.
    pub fn set_phase(
        &mut self,
        items: &[crate::pick::Element],
        phase: archisim_core::Phase,
    ) -> EditResult<usize> {
        use crate::pick::Element;
        let ids: Vec<u64> = items
            .iter()
            .filter_map(|e| match e {
                Element::Wall(id) => Some(id.0),
                Element::Slab(id) => Some(id.0),
                Element::Opening(id) => Some(id.0),
                Element::Column(id) => Some(id.0),
                Element::Beam(id) => Some(id.0),
                _ => None,
            })
            .collect();
        if ids.is_empty() {
            return Ok(0);
        }
        self.transact("Cambiar fase", |m| {
            ids.iter().try_for_each(|id| m.set_phase(*id, phase))
        })?;
        Ok(ids.len())
    }
}

impl Editor {
    /// Gives elements a discipline by hand (`None`: their own back).
    pub fn set_discipline(
        &mut self,
        ids: &[u64],
        discipline: Option<archisim_core::ElementDiscipline>,
    ) -> EditResult<()> {
        self.transact("Cambiar disciplina", |m| {
            ids.iter()
                .try_for_each(|id| m.set_discipline(*id, discipline))
        })
    }
}
