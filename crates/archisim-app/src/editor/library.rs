//! The library of types: a folder of family files (`.asfam`), each a recipe
//! with a catalogue of types made from it. A type of the project is saved
//! into its family's file; a type of a file is loaded into the project
//! with its `GlobalId`, so the same door is the same door in every project
//! (the IFC says so too).
//!
//! The folder is the user's (Documentos/ArchiSim/Biblioteca unless they
//! choose another); this module reads and writes it and changes the model
//! through the editor, so every load is one undoable step.

use std::path::{Path, PathBuf};

use archisim_core::{CatalogEntry, FamilyId, FamilyTypeRef, Loaded, Model, OnConflict};
use archisim_family::{Catalog, CatalogMaterial, CatalogType};

use super::{EditResult, Editor};

/// A family file of the library.
#[derive(Debug, Clone)]
pub struct LibraryFile {
    pub path: PathBuf,
    pub family: String,
    pub version: u32,
    /// The whole text (recipe and catalogue).
    pub text: String,
    pub types: Vec<CatalogType>,
    pub materials: Vec<CatalogMaterial>,
}

/// What the library holds: its files, and the ones it could not read
/// (path and why).
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub dir: PathBuf,
    pub files: Vec<LibraryFile>,
    pub errors: Vec<(PathBuf, String)>,
}

/// Documentos/ArchiSim/Biblioteca in the user's home.
pub fn default_library_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let docs = ["Documents", "Documentos"]
        .iter()
        .map(|d| home.join(d))
        .find(|d| d.is_dir())
        .unwrap_or_else(|| home.join("Documents"));
    docs.join("ArchiSim").join("Biblioteca")
}

/// Where the program keeps its settings.
fn settings_file() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(base.join("ArchiSim").join("biblioteca.txt"))
}

/// The library folder the user chose, or the default one.
pub fn library_dir() -> PathBuf {
    settings_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_library_dir)
}

/// Remembers the library folder for the next sessions.
pub fn set_library_dir(dir: &Path) -> std::io::Result<()> {
    let Some(f) = settings_file() else {
        return Ok(());
    };
    if let Some(parent) = f.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(f, dir.to_string_lossy().as_bytes())
}

/// Reads one family file.
pub fn read_file(path: &Path) -> Result<LibraryFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let text = text.replace("\r\n", "\n");
    let expanded = archisim_core::family::expand_shared_pieces(&text).map_err(|e| e.to_string())?;
    let def = archisim_family::parse(&expanded).map_err(|e| e.to_string())?;
    Ok(LibraryFile {
        path: path.to_path_buf(),
        family: def.name.clone(),
        version: def.version,
        text,
        types: def.catalog.types.clone(),
        materials: def.catalog.materials.clone(),
    })
}

/// Reads every `.asfam` of a folder and its subfolders (one level of
/// folders is how people sort them: Puertas, Ventanas, Mobiliario…).
pub fn scan(dir: &Path) -> Library {
    let mut lib = Library {
        dir: dir.to_path_buf(),
        ..Default::default()
    };
    let mut paths = Vec::new();
    let mut pending = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 3 {
                pending.push((p, depth + 1));
            } else if p
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("asfam"))
            {
                paths.push(p);
            }
        }
    }
    paths.sort();
    for p in paths {
        match read_file(&p) {
            Ok(f) => lib.files.push(f),
            Err(e) => lib.errors.push((p, e)),
        }
    }
    lib.files.sort_by_key(|f| f.family.to_lowercase());
    lib
}

/// A file name for a family: its name without the characters file systems
/// do not take.
fn file_name(family: &str) -> String {
    let clean: String = family
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c => c,
        })
        .collect();
    format!("{}.asfam", clean.trim())
}

/// Saves a type into the library: into its family's file if there is one
/// (replacing the entry of the same `GlobalId`, or adding it; the recipe
/// becomes the project's if it is newer), or into a new file. Returns the
/// file.
pub fn save_type(lib: &Library, entry: &CatalogEntry, version: u32) -> Result<PathBuf, String> {
    let existing = lib.files.iter().find(|f| f.family == entry.family_name);
    let (path, recipe, mut catalog) = match existing {
        Some(f) => {
            let recipe = if version > f.version {
                entry.recipe.clone()
            } else {
                archisim_family::without_catalog(&f.text)
            };
            (
                f.path.clone(),
                recipe,
                Catalog {
                    materials: f.materials.clone(),
                    types: f.types.clone(),
                },
            )
        }
        None => (
            lib.dir.join(file_name(&entry.family_name)),
            entry.recipe.clone(),
            Catalog::default(),
        ),
    };
    match catalog
        .types
        .iter_mut()
        .find(|t| t.guid == entry.entry.guid)
    {
        Some(t) => *t = entry.entry.clone(),
        None => {
            // Another type of the same name: this one is a different one.
            if catalog.types.iter().any(|t| t.name == entry.entry.name) {
                return Err(format!(
                    "la biblioteca ya tiene otro tipo «{}» de la familia «{}»: cámbiale el nombre antes de guardarlo",
                    entry.entry.name, entry.family_name
                ));
            }
            catalog.types.push(entry.entry.clone());
        }
    }
    for m in &entry.materials {
        match catalog.materials.iter_mut().find(|x| x.name == m.name) {
            Some(x) => *x = m.clone(),
            None => catalog.materials.push(m.clone()),
        }
    }
    let text = format!(
        "{}\n{}",
        recipe.trim_end(),
        archisim_family::catalog_text(&catalog)
    );
    // What is written must read back.
    let expanded = archisim_core::family::expand_shared_pieces(&text).map_err(|e| e.to_string())?;
    archisim_family::parse(&expanded).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&lib.dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// A newer recipe of a family of the project: from the program (its
/// system families) or from the library.
#[derive(Debug, Clone)]
pub struct FamilyUpdate {
    pub family: FamilyId,
    pub name: String,
    pub from: u32,
    pub to: u32,
    pub recipe: String,
    /// Where it comes from: "ArchiSim" or the library file.
    pub source: String,
}

/// The families of the project that have a newer recipe.
pub fn family_updates(model: &Model, lib: &Library) -> Vec<FamilyUpdate> {
    let mut out = Vec::new();
    for f in model.families() {
        let mut best: Option<FamilyUpdate> = None;
        if let Some(src) = archisim_core::family::system_family(f.name()) {
            if let Ok(def) = archisim_family::parse(&src) {
                if def.version > f.version() {
                    best = Some(FamilyUpdate {
                        family: f.id,
                        name: f.name().to_string(),
                        from: f.version(),
                        to: def.version,
                        recipe: src,
                        source: "ArchiSim".into(),
                    });
                }
            }
        }
        for file in lib.files.iter().filter(|x| x.family == f.name()) {
            let better = best
                .as_ref()
                .map_or(file.version > f.version(), |b| file.version > b.to);
            if better {
                best = Some(FamilyUpdate {
                    family: f.id,
                    name: f.name().to_string(),
                    from: f.version(),
                    to: file.version,
                    recipe: file.text.clone(),
                    source: file
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                });
            }
        }
        out.extend(best);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

impl Editor {
    /// Loads a type of a library file into the project.
    pub fn load_library_type(
        &mut self,
        file: &LibraryFile,
        index: usize,
        on_conflict: OnConflict,
    ) -> EditResult<Loaded> {
        let entry = file
            .types
            .get(index)
            .ok_or_else(|| "ese tipo ya no está en el fichero".to_string())?
            .clone();
        let text = file.text.clone();
        let materials = file.materials.clone();
        let label = format!("Cargar «{}» de la biblioteca", entry.name);
        self.transact(&label, |m| {
            m.load_catalog_type(&text, &entry, &materials, on_conflict)
        })
    }

    /// Replaces a family of the project with a newer recipe.
    pub fn update_family(&mut self, update: &FamilyUpdate) -> EditResult<usize> {
        let label = format!("Actualizar la familia «{}»", update.name);
        let (id, recipe) = (update.family, update.recipe.clone());
        self.transact(&label, |m| m.update_family(id, &recipe))
    }

    /// Removes the types no element uses.
    pub fn purge_unused_types(&mut self) -> EditResult<usize> {
        self.transact("Purgar tipos sin usar", |m| Ok(m.purge_unused_types()))
    }

    /// A type as the library saves it.
    pub fn catalog_entry(&self, t: FamilyTypeRef) -> EditResult<(CatalogEntry, u32)> {
        let m = self.model();
        let e = m.catalog_entry(t).map_err(|e| e.to_string())?;
        let version = m
            .family_by_name(&e.family_name)
            .map(|f| f.version())
            .unwrap_or(1);
        Ok((e, version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{family, NewFamilyType, ParamScope, Value};

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("archisim-lib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// A door type saved from one project into the library folder loads in
    /// another the same, with its GlobalId; saving it again replaces its
    /// entry instead of adding another.
    #[test]
    fn library_folder_round_trip() {
        let dir = temp_dir("rt");
        let mut a = Editor::new_project();
        let t = a
            .transact("t", |m| {
                let door = m.system_family(family::SWING_DOOR)?;
                let mut values = m.family_defaults(door, ParamScope::Type)?;
                values.insert("width".into(), Value::Number(0.725));
                m.add_family_type(NewFamilyType {
                    family: door,
                    name: "Baño 72,5".into(),
                    mark: Some("P-09".into()),
                    description: String::new(),
                    values,
                })
            })
            .unwrap();
        let (entry, version) = a.catalog_entry(FamilyTypeRef::Opening(t)).unwrap();
        let path = save_type(&scan(&dir), &entry, version).unwrap();
        assert!(path.ends_with("Puerta abatible.asfam"));
        // Twice: one entry.
        save_type(&scan(&dir), &entry, version).unwrap();
        let lib = scan(&dir);
        assert!(lib.errors.is_empty(), "{:?}", lib.errors);
        assert_eq!(lib.files.len(), 1);
        assert_eq!(lib.files[0].types.len(), 1);
        let mut b = Editor::new_project();
        let loaded = b
            .load_library_type(&lib.files[0], 0, OnConflict::Ask)
            .unwrap();
        let FamilyTypeRef::Opening(bt) = loaded.type_ref() else {
            panic!()
        };
        let (ta, tb) = (
            a.model().opening_type(t).unwrap(),
            b.model().opening_type(bt).unwrap(),
        );
        assert_eq!(ta.guid, tb.guid);
        assert_eq!(tb.mark, "P-09");
        assert!((tb.width - 0.725).abs() < 1e-12);
        // One undoable step.
        assert!(b.undo().is_some());
        assert!(b.model().opening_type(bt).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An old copy of a system family is offered the program's newer one.
    #[test]
    fn older_families_are_offered_an_update() {
        let mut ed = Editor::new_project();
        let door = ed
            .transact("f", |m| m.system_family(family::SWING_DOOR))
            .unwrap();
        let src = ed.model().family(door).unwrap().source.clone();
        let v = ed.model().family(door).unwrap().version();
        let older = src.replacen(&format!("version {v}"), "version 1", 1);
        ed.transact("old", |m| m.update_family(door, &older))
            .unwrap();
        assert_eq!(ed.model().family(door).unwrap().version(), 1);
        let ups = family_updates(ed.model(), &Library::default());
        let up = ups.iter().find(|u| u.family == door).expect("an update");
        assert_eq!((up.from, up.to), (1, v));
        ed.update_family(up).unwrap();
        assert_eq!(ed.model().family(door).unwrap().version(), v);
        assert!(family_updates(ed.model(), &Library::default())
            .iter()
            .all(|u| u.family != door));
    }
}
