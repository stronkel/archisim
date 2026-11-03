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

    /// When the shadows are cast.
    pub fn set_sun(&mut self, sun: archisim_core::Sun) -> EditResult {
        self.transact("Sol de las sombras", |m| m.set_sun(sun))
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

impl Editor {
    /// Saves a point of view of the 3D, named «Cámara N» when unnamed.
    pub fn add_camera(
        &mut self,
        c: archisim_core::SavedCamera,
    ) -> EditResult<archisim_core::CameraId> {
        self.transact("Guardar cámara", |m| m.add_camera(c))
    }

    pub fn rename_camera(&mut self, id: archisim_core::CameraId, name: &str) -> EditResult {
        let name = name.trim().to_string();
        self.transact("Renombrar cámara", |m| {
            m.edit_camera(id, |c| c.name = name)
        })
    }

    /// Moves a saved camera to where the view looks now.
    pub fn update_camera(
        &mut self,
        id: archisim_core::CameraId,
        from: archisim_core::SavedCamera,
    ) -> EditResult {
        self.transact("Actualizar cámara", |m| {
            m.edit_camera(id, |c| {
                c.target = from.target;
                c.pitch = from.pitch;
                c.yaw = from.yaw;
                c.distance = from.distance;
                c.perspective = from.perspective;
            })
        })
    }

    pub fn remove_camera(&mut self, id: archisim_core::CameraId) -> EditResult {
        self.transact("Borrar cámara", |m| m.remove_camera(id).map(|_| ()))
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
            .add_presentation_style(PresentationStyle::new("Blanco y negro"))
            .unwrap();
        assert_eq!(
            ed.model().presentation_style(id).unwrap().name,
            "Blanco y negro 2"
        );
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
