//! The project's presentation styles (see [`PresentationStyle`]): kept
//! by the model so that views and sheets can name them, and saved with
//! the project.

use super::*;
use crate::graphics::PresentationStyle;

impl Model {
    pub fn presentation_styles(&self) -> impl Iterator<Item = &PresentationStyle> {
        self.presentation_styles.values()
    }

    pub fn presentation_style(&self, id: PresentationStyleId) -> Option<&PresentationStyle> {
        self.presentation_styles.get(&id)
    }

    fn check_presentation(
        &self,
        s: &PresentationStyle,
        except: Option<PresentationStyleId>,
    ) -> Result<(), ModelError> {
        if !s.is_valid() {
            return Err(ModelError::NonPositive("presentation style"));
        }
        if self
            .presentation_styles
            .values()
            .any(|o| Some(o.id) != except && o.name == s.name)
        {
            return Err(ModelError::DuplicateName(s.name.clone()));
        }
        Ok(())
    }

    /// Adds a style (its id is given here).
    pub fn add_presentation_style(
        &mut self,
        mut style: PresentationStyle,
    ) -> Result<PresentationStyleId, ModelError> {
        self.check_presentation(&style, None)?;
        style.id = PresentationStyleId(self.take_id());
        let id = style.id;
        self.presentation_styles.insert(id, style);
        Ok(id)
    }

    /// Replaces a style (keeps its id).
    pub fn set_presentation_style(&mut self, style: PresentationStyle) -> Result<(), ModelError> {
        if !self.presentation_styles.contains_key(&style.id) {
            return Err(ModelError::UnknownPresentationStyle(style.id));
        }
        self.check_presentation(&style, Some(style.id))?;
        self.presentation_styles.insert(style.id, style);
        Ok(())
    }

    /// Removes a style; the views that used it go back to the plain look.
    pub fn remove_presentation_style(&mut self, id: PresentationStyleId) -> Result<(), ModelError> {
        self.presentation_styles
            .remove(&id)
            .ok_or(ModelError::UnknownPresentationStyle(id))?;
        for v in self.viewports.values_mut() {
            if v.style.presentation == Some(id) {
                v.style.presentation = None;
            }
        }
        Ok(())
    }

    pub(super) fn restore_presentation_styles(
        &mut self,
        styles: Vec<PresentationStyle>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for s in styles {
            claim(s.id.0, None)
                .and_then(|_| self.check_presentation(&s, None))
                .map_err(|error| RestoreError {
                    element: format!("presentation style {}", s.id.0),
                    error,
                })?;
            self.presentation_styles.insert(s.id, s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_named_once_and_removed_from_views() {
        let mut m = Model::new("T");
        let s = m
            .add_presentation_style(PresentationStyle::new("AD5"))
            .unwrap();
        assert!(m
            .add_presentation_style(PresentationStyle::new("AD5"))
            .is_err());
        let mut bad = PresentationStyle::new("Malo");
        bad.line_scale = -1.0;
        assert!(m.add_presentation_style(bad).is_err());
        let mut edited = m.presentation_style(s).unwrap().clone();
        edited.poche = Some([40, 70, 60]);
        m.set_presentation_style(edited).unwrap();
        assert_eq!(m.presentation_style(s).unwrap().poche, Some([40, 70, 60]));
        m.remove_presentation_style(s).unwrap();
        assert_eq!(m.presentation_styles().count(), 0);
    }
}
