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
        let after = std::mem::replace(&mut self.model, step.before);
        self.redo.push((step.label.clone(), after));
        Some(step.label)
    }

    /// Redoes the last undone step and returns its label.
    pub fn redo(&mut self) -> Option<String> {
        let (label, after) = self.redo.pop()?;
        let before = std::mem::replace(&mut self.model, after);
        self.undo.push(Step {
            label: label.clone(),
            before,
        });
        Some(label)
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
            Err::<(), _>(ModelError::EmptyWallType)
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
}
