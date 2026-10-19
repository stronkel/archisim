//! Sections and elevations, each change one undoable step.

use archisim_core::{NewSection, Point2, SectionId, SectionKind, SectionLine};

use super::{EditResult, Editor};

impl Editor {
    /// Draws a section from `a` to `b`, looking to its left, with the next
    /// free letter.
    pub fn add_section(&mut self, a: Point2, b: Point2, depth: f64) -> EditResult<SectionId> {
        let name = self.model().next_section_name();
        self.transact("Crear sección", |m| {
            m.add_section(NewSection {
                name,
                kind: SectionKind::Section,
                start: a,
                end: b,
                depth,
            })
        })
    }

    /// The four elevations around the building (those missing). Returns
    /// how many were made.
    pub fn add_elevations(&mut self) -> EditResult<Vec<SectionId>> {
        let specs = archisim_plan::section::elevations_around(self.model());
        if specs.is_empty() {
            return Ok(Vec::new());
        }
        self.transact("Crear alzados", |m| {
            specs.into_iter().map(|s| m.add_section(s)).collect()
        })
    }

    pub fn edit_section(
        &mut self,
        label: &str,
        id: SectionId,
        edit: impl FnOnce(&mut SectionLine),
    ) -> EditResult {
        self.transact(label, |m| m.edit_section(id, edit))
    }

    /// Renames a section; names are unique among sections and among
    /// elevations.
    pub fn rename_section(&mut self, id: SectionId, name: &str) -> EditResult {
        let name = name.trim();
        if name.is_empty() {
            return Err("la sección necesita un nombre".into());
        }
        let model = self.model();
        let Some(kind) = model.section(id).map(|s| s.kind) else {
            return Err("la sección ya no existe".into());
        };
        if model
            .sections()
            .any(|s| s.id != id && s.kind == kind && s.name == name)
        {
            return Err(format!("ya hay una con el nombre «{name}»"));
        }
        let name = name.to_string();
        self.transact("Renombrar sección", |m| {
            m.edit_section(id, |s| s.name = name)
        })
    }

    /// Turns a section round: it looks the other way.
    pub fn flip_section(&mut self, id: SectionId) -> EditResult {
        self.transact("Invertir sección", |m| {
            m.edit_section(id, |s| std::mem::swap(&mut s.start, &mut s.end))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_with_the_editor() {
        let mut ed = Editor::new_project();
        let a = ed
            .add_section(Point2::new(0.0, 1.0), Point2::new(5.0, 1.0), 6.0)
            .unwrap();
        let b = ed
            .add_section(Point2::new(1.0, 0.0), Point2::new(1.0, 5.0), 6.0)
            .unwrap();
        assert_eq!(ed.model().section(a).unwrap().name, "A");
        assert_eq!(ed.model().section(b).unwrap().name, "B");
        assert!(ed.rename_section(b, "A").is_err());
        ed.rename_section(b, "C").unwrap();
        ed.flip_section(a).unwrap();
        assert_eq!(ed.model().section(a).unwrap().start, Point2::new(5.0, 1.0));
        ed.undo();
        assert_eq!(ed.model().section(a).unwrap().start, Point2::new(0.0, 1.0));
        // An empty project has nothing to look at.
        assert!(ed.add_elevations().unwrap().is_empty());
    }
}
