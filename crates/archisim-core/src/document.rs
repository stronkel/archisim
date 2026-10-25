//! An open project: the model plus its undo/redo history.
//!
//! Every change goes through [`Document::transact`], which records a named
//! step ("Crear muro", "Mover muro"…). If the edit fails, the model is left
//! exactly as it was. Snapshots are simple and correct for the MVP; they can
//! be replaced by change sets later without touching callers.

use crate::model::{Model, ModelError};

#[derive(Debug, Clone)]
struct Step {
    label: String,
    before: Model,
}

#[derive(Debug, Clone)]
pub struct Document {
    model: Model,
    undo: Vec<Step>,
    redo: Vec<(String, Model)>,
}

impl Document {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The phase new elements are drawn in (see [`Model::draw_phase`]):
    /// a setting of the drawing, not a change to undo.
    pub fn set_draw_phase(&mut self, phase: crate::model::Phase) {
        self.model.draw_phase = phase;
    }

    /// Runs an edit as one undoable step. On error nothing changes.
    pub fn transact<T>(
        &mut self,
        label: impl Into<String>,
        edit: impl FnOnce(&mut Model) -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        let mut working = self.model.clone();
        let result = edit(&mut working)?;
        let before = std::mem::replace(&mut self.model, working);
        self.undo.push(Step {
            label: label.into(),
            before,
        });
        self.redo.clear();
        Ok(result)
    }

    /// Undoes the last step and returns its label.
    pub fn undo(&mut self) -> Option<String> {
        let step = self.undo.pop()?;
        let phase = self.model.draw_phase;
        let after = std::mem::replace(&mut self.model, step.before);
        self.model.draw_phase = phase;
        self.redo.push((step.label.clone(), after));
        Some(step.label)
    }

    /// Redoes the last undone step and returns its label.
    pub fn redo(&mut self) -> Option<String> {
        let (label, after) = self.redo.pop()?;
        let phase = self.model.draw_phase;
        let before = std::mem::replace(&mut self.model, after);
        self.model.draw_phase = phase;
        self.undo.push(Step {
            label: label.clone(),
            before,
        });
        Some(label)
    }

    /// Joins the last `n` steps into one called `label`, so a single Undo
    /// takes them all back (a room: its walls, its floor and its room).
    /// Does nothing if there are fewer than two of them.
    pub fn join_last(&mut self, n: usize, label: impl Into<String>) {
        if n < 2 || self.undo.len() < n {
            return;
        }
        let first = self.undo.len() - n;
        let before = self.undo.drain(first..).next().map(|s| s.before);
        if let Some(before) = before {
            self.undo.push(Step {
                label: label.into(),
                before,
            });
        }
    }

    /// How many steps can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(l, _)| l.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_and_failed_edits() {
        let mut doc = Document::new(Model::new("P"));
        let lvl = doc
            .transact("Crear planta", |m| Ok(m.add_level("PB", 0.0)))
            .unwrap();
        assert!(doc.model().level(lvl).is_some());

        // A failing edit leaves no trace, even if it changed things first.
        let err = doc.transact("Fallo", |m| {
            m.add_level("Temporal", 3.0);
            Err::<(), _>(ModelError::EmptyType)
        });
        assert!(err.is_err());
        assert_eq!(doc.model().levels().len(), 1);
        assert_eq!(doc.undo_label(), Some("Crear planta"));

        assert_eq!(doc.undo().as_deref(), Some("Crear planta"));
        assert!(doc.model().level(lvl).is_none());
        assert_eq!(doc.redo().as_deref(), Some("Crear planta"));
        assert!(doc.model().level(lvl).is_some());
        assert!(doc.redo().is_none());
    }

    #[test]
    fn steps_joined_undo_together() {
        let mut doc = Document::new(Model::new("P"));
        doc.transact("A", |m| Ok(m.add_level("PB", 0.0))).unwrap();
        doc.transact("B", |m| Ok(m.add_level("P1", 3.0))).unwrap();
        doc.transact("C", |m| Ok(m.add_level("P2", 6.0))).unwrap();
        doc.join_last(2, "B y C");
        assert_eq!(doc.undo_len(), 2);
        assert_eq!(doc.undo().as_deref(), Some("B y C"));
        assert_eq!(doc.model().levels().len(), 1);
        assert_eq!(doc.redo().as_deref(), Some("B y C"));
        assert_eq!(doc.model().levels().len(), 3);
    }
}
