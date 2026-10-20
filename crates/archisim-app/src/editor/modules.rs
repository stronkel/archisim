//! Modules: loading another project as a module, placing its copies,
//! telling when its file has changed and updating it; and the model the
//! views draw, with the copies' elements in it.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use archisim_core::{Model, ModuleId, ModuleSourceId, NewModulePlacement, Point2};

use super::{explain, explain_file, EditResult, Editor};

/// The model the views draw: the project itself or, when it places
/// modules, the project with every copy's elements.
pub enum Shown<'a> {
    Plain(&'a Model),
    Flat(Rc<Model>),
}

impl std::ops::Deref for Shown<'_> {
    type Target = Model;
    fn deref(&self) -> &Model {
        match self {
            Shown::Plain(m) => m,
            Shown::Flat(m) => m,
        }
    }
}

/// The flattened model of a revision.
#[derive(Default)]
pub struct FlatCache(RefCell<Option<Flat>>);

/// A revision, its flattened model and why it could not be made.
type Flat = (u64, Rc<Model>, Option<String>);

/// How a module file on disk compares with the copy the project keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    UpToDate,
    /// The file has changed since it was loaded: it can be updated.
    Changed,
    /// The file is not where it was (the project keeps its copy).
    Missing,
}

impl Editor {
    /// What the views draw (see [`Shown`]).
    pub fn shown(&self) -> Shown<'_> {
        let model = self.model();
        if !model.has_modules() {
            return Shown::Plain(model);
        }
        let mut cache = self.flat.0.borrow_mut();
        if let Some((rev, m, _)) = cache.as_ref() {
            if *rev == self.revision() {
                return Shown::Flat(m.clone());
            }
        }
        let (flat, problem) = match model.flattened() {
            Ok(m) => (m, None),
            Err(e) => (
                model.clone(),
                Some(format!("{}: {}", e.element, explain(&e.error))),
            ),
        };
        let flat = Rc::new(flat);
        *cache = Some((self.revision(), flat.clone(), problem));
        Shown::Flat(flat)
    }

    /// Where a module's file is: its path, from the project's folder if it
    /// is relative.
    pub fn module_path(&self, id: ModuleSourceId) -> Option<PathBuf> {
        let s = self.model().module_source(id)?;
        let p = PathBuf::from(&s.path);
        if p.is_absolute() {
            return Some(p);
        }
        let dir = self
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_default();
        Some(dir.join(p))
    }

    /// How each module's file compares with the project's copy.
    pub fn module_status(&self) -> Vec<(ModuleSourceId, ModuleStatus)> {
        self.model()
            .module_sources()
            .map(|s| {
                let status = match self
                    .module_path(s.id)
                    .and_then(|p| std::fs::read_to_string(p).ok())
                {
                    None => ModuleStatus::Missing,
                    Some(text) if archisim_core::text_stamp(&text) == s.stamp => {
                        ModuleStatus::UpToDate
                    }
                    Some(_) => ModuleStatus::Changed,
                };
                (s.id, status)
            })
            .collect()
    }

    /// Loads a project as a module (its types and materials come along).
    pub fn load_module(&mut self, path: &Path) -> EditResult<ModuleSourceId> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("no se puede leer el módulo: {e}"))?;
        let (text, stamp, module) =
            archisim_file::read_module(&text).map_err(|e| explain_file(&e))?;
        if let (Some(a), Ok(b)) = (self.path.as_deref(), path.canonicalize()) {
            if a.canonicalize().is_ok_and(|a| a == b) {
                return Err("un proyecto no puede ser módulo de sí mismo".into());
            }
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Módulo".into());
        let rel = self.relative(path);
        self.transact("Cargar módulo", |m| {
            m.add_module_source(name, rel, text.into(), stamp, module)
        })
    }

    /// `path` from the project's folder, if it is inside it.
    fn relative(&self, path: &Path) -> String {
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let dir = self
            .path
            .as_deref()
            .and_then(Path::parent)
            .and_then(|d| d.canonicalize().ok());
        match dir.and_then(|d| abs.strip_prefix(&d).ok().map(Path::to_path_buf)) {
            Some(r) => r.to_string_lossy().replace('\\', "/"),
            None => abs.to_string_lossy().into_owned(),
        }
    }

    /// Takes the module's file as it is now: every copy changes.
    pub fn update_module(&mut self, id: ModuleSourceId) -> EditResult {
        let path = self
            .module_path(id)
            .ok_or("el módulo ya no está en el proyecto")?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("no se puede leer {}: {e}", path.display()))?;
        let (text, stamp, module) =
            archisim_file::read_module(&text).map_err(|e| explain_file(&e))?;
        self.transact("Actualizar módulo", |m| {
            m.update_module_source(id, text.into(), stamp, module)
        })
    }

    /// Places a copy of a module on the active level.
    pub fn place_module(
        &mut self,
        source: ModuleSourceId,
        at: Point2,
        rotation: f64,
        mirror: bool,
    ) -> EditResult<ModuleId> {
        let level = self.level;
        self.transact("Colocar módulo", |m| {
            m.add_levels_for_module(source, level)?;
            m.add_module(NewModulePlacement {
                source,
                level,
                position: at,
                rotation,
                mirror,
            })
        })
    }

    pub fn edit_module(
        &mut self,
        label: &str,
        id: ModuleId,
        edit: impl FnOnce(&mut archisim_core::ModulePlacement),
    ) -> EditResult {
        self.transact(label, |m| m.edit_module(id, edit))
    }

    /// Removes a module and its copies.
    pub fn remove_module_source(&mut self, id: ModuleSourceId) -> EditResult {
        self.transact("Quitar módulo", |m| m.remove_module_source(id).map(|_| ()))
    }

    /// Why the copies of the modules could not be shown, if they could
    /// not (the views then show the project without them).
    pub fn modules_problem(&self) -> Option<String> {
        if !self.model().has_modules() {
            return None;
        }
        drop(self.shown());
        self.flat
            .0
            .borrow()
            .as_ref()
            .filter(|(rev, _, _)| *rev == self.revision())
            .and_then(|(_, _, p)| p.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_are_loaded_placed_and_updated() {
        let dir = std::env::temp_dir().join(format!("archisim-modulos-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A house type of the starter library with one wall.
        let mut house = Editor::new_project();
        let ws = crate::editor::WallSettings {
            wall_type: house.model().wall_types().next().unwrap().id,
            reference: archisim_core::ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            top: crate::editor::TopChoice::Unconnected,
            top_offset: 0.0,
        };
        house
            .add_wall(&ws, Point2::new(0.0, 0.0), Point2::new(8.0, 0.0))
            .unwrap();
        let house_path = dir.join("tipo-b.archisim");
        archisim_file::save(house.model(), &house_path).unwrap();

        let mut plan = Editor::new_project();
        plan.path = Some(dir.join("masterplan.archisim"));
        let src = plan.load_module(&house_path).unwrap();
        assert_eq!(
            plan.model().module_source(src).unwrap().path,
            "tipo-b.archisim"
        );
        assert_eq!(plan.module_status(), [(src, ModuleStatus::UpToDate)]);
        for x in [0.0, 12.0, 24.0] {
            plan.place_module(src, Point2::new(x, 0.0), 0.0, x > 0.0)
                .unwrap();
        }
        assert_eq!(plan.shown().walls().count(), 3);
        assert_eq!(plan.model().walls().count(), 0);

        // The house type changes: the masterplan says so and updates.
        house
            .add_wall(&ws, Point2::new(8.0, 0.0), Point2::new(8.0, 6.0))
            .unwrap();
        archisim_file::save(house.model(), &house_path).unwrap();
        assert_eq!(plan.module_status(), [(src, ModuleStatus::Changed)]);
        plan.update_module(src).unwrap();
        assert_eq!(plan.module_status(), [(src, ModuleStatus::UpToDate)]);
        assert_eq!(plan.shown().walls().count(), 6);
        plan.undo();
        assert_eq!(plan.shown().walls().count(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }
}
