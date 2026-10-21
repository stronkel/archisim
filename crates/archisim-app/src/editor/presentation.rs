//! Presentation styles of the project: made, changed, removed and brought
//! from the starter list or from the studio's library, each one undoable
//! step.

use archisim_core::{PresentationStyle, PresentationStyleId};

use super::{EditResult, Editor};

impl Editor {
    /// A new style, named after `name` (with a number if it is taken).
    pub fn add_presentation_style(
        &mut self,
        mut style: PresentationStyle,
    ) -> EditResult<PresentationStyleId> {
        style.name = self.free_style_name(&style.name);
        self.transact("Nuevo estilo de presentación", |m| {
            m.add_presentation_style(style)
        })
    }

    pub fn set_presentation_style(&mut self, style: PresentationStyle) -> EditResult {
        self.transact("Cambiar estilo de presentación", |m| {
            m.set_presentation_style(style)
        })
    }

    pub fn remove_presentation_style(&mut self, id: PresentationStyleId) -> EditResult {
        self.transact("Borrar estilo de presentación", |m| {
            m.remove_presentation_style(id)
        })
    }

    /// Adds the styles whose names the project does not have yet (the
    /// starter ones, or those of a library file). Returns how many.
    pub fn add_missing_styles(&mut self, styles: Vec<PresentationStyle>) -> EditResult<usize> {
        let missing: Vec<PresentationStyle> = styles
            .into_iter()
            .filter(|s| self.model().presentation_styles().all(|o| o.name != s.name))
            .collect();
        if missing.is_empty() {
            return Ok(0);
        }
        let n = missing.len();
        self.transact("Cargar estilos de presentación", |m| {
            for s in missing {
                m.add_presentation_style(s)?;
            }
            Ok(n)
        })
    }

    /// `name`, or `name 2`, `name 3`… if a style has it.
    fn free_style_name(&self, name: &str) -> String {
        let taken = |n: &str| self.model().presentation_styles().any(|s| s.name == n);
        if !taken(name) {
            return name.to_string();
        }
        (2..)
            .map(|i| format!("{name} {i}"))
            .find(|n| !taken(n))
            .expect("some number is free")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_added_named_and_loaded_once() {
        let mut ed = Editor::new_project();
        let starters = ed.model().presentation_styles().count();
        assert!(starters >= 5, "the starter styles");
        let id = ed
            .add_presentation_style(PresentationStyle::new("AD5"))
            .unwrap();
        assert_eq!(ed.model().presentation_style(id).unwrap().name, "AD5 2");
        assert_eq!(
            ed.add_missing_styles(archisim_library::presentation_styles())
                .unwrap(),
            0
        );
        ed.remove_presentation_style(id).unwrap();
        ed.undo();
        assert!(ed.model().presentation_style(id).is_some());
    }
}
