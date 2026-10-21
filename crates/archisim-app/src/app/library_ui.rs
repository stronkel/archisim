//! The library window: the folder of family files, the types in them to
//! load into the project, the families of the project with a newer recipe,
//! and purging the types no element uses.

use archisim_core::{FamilyTypeRef, LayeredKind, LayeredRef, Loaded, LoadedLayered, OnConflict};
use eframe::egui::{self, Ui};

use super::*;
use crate::editor::library::{self, FamilyUpdate, Library};

/// The state of the library window.
#[derive(Default)]
pub struct LibraryWindow {
    pub open: bool,
    /// The folder as last read (read again with «Volver a leer»).
    lib: Option<Library>,
    filter: String,
    /// A type whose GlobalId the project has with other values: file and
    /// type index, waiting for the user to choose.
    conflict: Option<(usize, usize)>,
    /// The same for a wall or slab type: its file and index.
    layered_conflict: Option<(usize, usize)>,
    /// The purge is waiting for confirmation: how many types would go.
    purge: Option<usize>,
}

impl LibraryWindow {
    fn library(&mut self) -> &Library {
        self.lib
            .get_or_insert_with(|| library::scan(&library::library_dir()))
    }

    /// Shows this library instead of the folder's (tests).
    #[cfg(test)]
    pub fn set(&mut self, lib: Library) {
        self.lib = Some(lib);
    }

    #[cfg(test)]
    pub fn conflict(&self) -> Option<(usize, usize)> {
        self.conflict
    }

    /// Reads the folder again (after saving into it, or changing it).
    pub fn refresh(&mut self) {
        self.lib = None;
    }
}

impl ArchiSimApp {
    pub(super) fn library_dialog(&mut self, ctx: &egui::Context) {
        if !self.library.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Biblioteca")
            .open(&mut open)
            .default_width(560.0)
            .default_height(520.0)
            .show(ctx, |ui| self.library_contents(ui));
        self.library.open = open;
        self.library_conflict(ctx);
        self.purge_confirmation(ctx);
    }

    fn library_contents(&mut self, ui: &mut Ui) {
        let lib = self.library.library().clone();
        ui.horizontal(|ui| {
            ui.label("Carpeta:");
            ui.label(egui::RichText::new(lib.dir.to_string_lossy()).monospace())
                .on_hover_text("Los ficheros .asfam (familias), .astipos (muros y forjados) y los modelos 3D (.obj, .gltf, .glb, .dae, que salen en la galería de objetos) de esta carpeta y de sus subcarpetas");
        });
        ui.horizontal(|ui| {
            if ui.button("Cambiar carpeta…").clicked() {
                if let Some(dir) = rfd::FileDialog::new().set_directory(&lib.dir).pick_folder() {
                    match library::set_library_dir(&dir) {
                        Ok(()) => self.say(format!("Biblioteca en {}.", dir.display())),
                        Err(e) => self.complain(format!("No se puede recordar la carpeta: {e}.")),
                    }
                    self.library.lib = Some(library::scan(&dir));
                }
            }
            if ui.button("Volver a leer").clicked() {
                self.library.refresh();
                self.folder_objects = None;
            }
            if ui
                .button("Instalar la biblioteca ampliada")
                .on_hover_text("Más de ochenta árboles, arbustos, mobiliario urbano, personas y vehículos, en la subcarpeta «Ampliada»; salen en las galerías de objetos")
                .clicked()
            {
                match crate::editor::library::install_extended(&lib.dir) {
                    Ok(n) => {
                        self.say(format!(
                            "Biblioteca ampliada instalada: {n} tipos en {}.",
                            lib.dir.join("Ampliada").display()
                        ));
                        self.library.refresh();
                        self.folder_objects = None;
                    }
                    Err(e) => self.complain(format!("No se puede instalar: {e}.")),
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Buscar");
            ui.text_edit_singleline(&mut self.library.filter);
        });
        ui.separator();
        let filter = self.library.filter.trim().to_lowercase();
        let mut load: Option<(usize, usize)> = None;
        let mut load_layered: Option<(usize, usize)> = None;
        let mut update: Option<FamilyUpdate> = None;
        let mut purge = false;
        let updates = library::family_updates(self.ed.model(), &lib);
        let in_project: std::collections::BTreeMap<String, String> = self
            .ed
            .model()
            .opening_types()
            .map(|t| (t.guid.to_string(), t.name.clone()))
            .chain(
                self.ed
                    .model()
                    .object_types()
                    .map(|t| (t.guid.to_string(), t.name.clone())),
            )
            .collect();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 40.0)
            .show(ui, |ui| {
                ui.strong("Tipos de la biblioteca");
                if lib.files.is_empty() && lib.layered.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "La carpeta no tiene familias todavía. Para llevar un tipo del proyecto a la biblioteca: Navegador → clic derecho sobre un tipo de muro, forjado, puerta, ventana u objeto → «Guardar en la biblioteca».",
                        )
                        .weak(),
                    );
                }
                for (fi, f) in lib.files.iter().enumerate() {
                    let matches = |s: &str| filter.is_empty() || s.to_lowercase().contains(&filter);
                    let shown: Vec<usize> = (0..f.types.len())
                        .filter(|&i| matches(&f.family) || matches(&f.types[i].name))
                        .collect();
                    if shown.is_empty() && !(f.types.is_empty() && matches(&f.family)) {
                        continue;
                    }
                    let title = format!("{} · v{} ({} tipos)", f.family, f.version, f.types.len());
                    egui::CollapsingHeader::new(title)
                        .id_salt(("lib-file", &f.path))
                        .default_open(!filter.is_empty())
                        .show(ui, |ui| {
                            if f.types.is_empty() {
                                ui.label(egui::RichText::new("Solo la familia, sin tipos").weak());
                            }
                            for &ti in &shown {
                                let t = &f.types[ti];
                                ui.horizontal(|ui| {
                                    if !t.mark.is_empty() {
                                        ui.label(egui::RichText::new(&t.mark).strong());
                                    }
                                    let r = ui.label(&t.name);
                                    if !t.description.is_empty() {
                                        r.on_hover_text(&t.description);
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button("Cargar")
                                                .on_hover_text("Añadir el tipo al proyecto, con su familia y sus materiales")
                                                .clicked()
                                            {
                                                load = Some((fi, ti));
                                            }
                                            if in_project.contains_key(&t.guid) {
                                                ui.label(egui::RichText::new("en el proyecto").weak());
                                            }
                                        },
                                    );
                                });
                            }
                        });
                }
                let in_project_layered: std::collections::BTreeSet<String> = self
                    .ed
                    .model()
                    .wall_types()
                    .map(|t| t.guid.to_string())
                    .chain(self.ed.model().slab_types().map(|t| t.guid.to_string()))
                    .collect();
                for (fi, f) in lib.layered.iter().enumerate() {
                    let matches = |s: &str| filter.is_empty() || s.to_lowercase().contains(&filter);
                    let shown: Vec<usize> = (0..f.catalog.types.len())
                        .filter(|&i| matches(&f.catalog.types[i].name))
                        .collect();
                    if shown.is_empty() {
                        continue;
                    }
                    let file = f
                        .path
                        .file_stem()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let title = format!("{file} · tipos multicapa ({})", f.catalog.types.len());
                    egui::CollapsingHeader::new(title)
                        .id_salt(("lib-layered", &f.path))
                        .default_open(!filter.is_empty())
                        .show(ui, |ui| {
                            for &ti in &shown {
                                let t = &f.catalog.types[ti];
                                let total: f64 = t.layers.iter().map(|l| l.1).sum();
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(match t.kind {
                                        LayeredKind::Wall(_) => "Muro",
                                        LayeredKind::Slab(_) => "Forjado",
                                    }).weak());
                                    ui.label(&t.name).on_hover_text(
                                        t.layers
                                            .iter()
                                            .map(|(m, th, _)| format!("{} mm · {m}", crate::units::fmt_mm(*th)))
                                            .collect::<Vec<_>>()
                                            .join("\n"),
                                    );
                                    ui.label(egui::RichText::new(format!("{} mm", crate::units::fmt_mm(total))).weak());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button("Cargar")
                                                .on_hover_text("Añadir el tipo al proyecto, con sus capas y sus materiales")
                                                .clicked()
                                            {
                                                load_layered = Some((fi, ti));
                                            }
                                            if in_project_layered.contains(&t.guid) {
                                                ui.label(egui::RichText::new("en el proyecto").weak());
                                            }
                                        },
                                    );
                                });
                            }
                        });
                }
                for (p, e) in &lib.errors {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "{}: {e}",
                            p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
                        ),
                    );
                }
                ui.add_space(8.0);
                ui.strong("Familias del proyecto");
                for f in self.ed.model().families() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} · v{}", f.name(), f.version()));
                        if let Some(u) = updates.iter().find(|u| u.family == f.id) {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(format!("Actualizar a v{}", u.to))
                                    .on_hover_text(format!(
                                        "De la versión {} a la {}, de {}. Los tipos conservan sus valores y toman los de los parámetros nuevos por defecto.",
                                        u.from, u.to, u.source
                                    ))
                                    .clicked()
                                {
                                    update = Some(u.clone());
                                }
                            });
                        }
                    });
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button("Purgar tipos sin usar…")
                .on_hover_text("Quitar del proyecto los tipos que ningún elemento usa")
                .clicked()
            {
                purge = true;
            }
        });
        if let Some((fi, ti)) = load {
            self.load_from_library(fi, ti, OnConflict::Ask);
        }
        if let Some((fi, ti)) = load_layered {
            self.load_layered_from_library(fi, ti, OnConflict::Ask);
        }
        if let Some(u) = update {
            match self.ed.update_family(&u) {
                Ok(n) => self.say(format!(
                    "Familia «{}» actualizada a la versión {} ({n} tipos).",
                    u.name, u.to
                )),
                Err(e) => self.complain(format!("No se puede actualizar «{}»: {e}.", u.name)),
            }
        }
        if purge {
            let mut probe = self.ed.model().clone();
            self.library.purge = Some(probe.purge_unused_types());
        }
    }

    pub(super) fn load_from_library(&mut self, fi: usize, ti: usize, on_conflict: OnConflict) {
        let lib = self.library.library().clone();
        let Some(file) = lib.files.get(fi) else {
            return;
        };
        let name = file.types[ti].name.clone();
        let guid = file.types[ti].guid.clone();
        let had = self
            .ed
            .model()
            .family_types()
            .into_iter()
            .any(|(t, _, _)| match t {
                FamilyTypeRef::Opening(id) => self
                    .ed
                    .model()
                    .opening_type(id)
                    .is_some_and(|x| x.guid.to_string() == guid),
                FamilyTypeRef::Object(id) => self
                    .ed
                    .model()
                    .object_type(id)
                    .is_some_and(|x| x.guid.to_string() == guid),
            });
        match self.ed.load_library_type(file, ti, on_conflict) {
            Ok(Loaded::New(_)) => self.say(format!("Tipo «{name}» cargado en el proyecto.")),
            Ok(Loaded::AlreadyThere(_)) => {
                self.say(format!("El proyecto ya tiene el tipo «{name}», igual."))
            }
            Ok(Loaded::Replaced(_)) => self.say(format!(
                "El tipo «{name}» del proyecto ha tomado los valores de la biblioteca."
            )),
            Err(_) if had && on_conflict == OnConflict::Ask => {
                self.library.conflict = Some((fi, ti));
            }
            Err(e) => self.complain(format!("No se puede cargar «{name}»: {e}.")),
        }
    }

    /// Loads a wall or slab type of the library into the project.
    pub(super) fn load_layered_from_library(
        &mut self,
        fi: usize,
        ti: usize,
        on_conflict: OnConflict,
    ) {
        let lib = self.library.library().clone();
        let Some(file) = lib.layered.get(fi) else {
            return;
        };
        let Some(t) = file.catalog.types.get(ti) else {
            return;
        };
        let name = t.name.clone();
        let had = self
            .ed
            .model()
            .wall_types()
            .map(|w| w.guid.to_string())
            .chain(self.ed.model().slab_types().map(|s| s.guid.to_string()))
            .any(|g| g == t.guid);
        match self.ed.load_layered_type(file, ti, on_conflict) {
            Ok(LoadedLayered::New(_)) => self.say(format!("Tipo «{name}» cargado en el proyecto.")),
            Ok(LoadedLayered::AlreadyThere(_)) => {
                self.say(format!("El proyecto ya tiene el tipo «{name}», igual."))
            }
            Ok(LoadedLayered::Replaced(_)) => self.say(format!(
                "El tipo «{name}» del proyecto ha tomado las capas de la biblioteca."
            )),
            Err(_) if had && on_conflict == OnConflict::Ask => {
                self.library.layered_conflict = Some((fi, ti));
            }
            Err(e) => self.complain(format!("No se puede cargar «{name}»: {e}.")),
        }
    }

    fn library_conflict(&mut self, ctx: &egui::Context) {
        let layered = self.library.layered_conflict.is_some();
        let Some((fi, ti)) = self.library.conflict.or(self.library.layered_conflict) else {
            return;
        };
        let name = if layered {
            self.library
                .library()
                .layered
                .get(fi)
                .and_then(|f| f.catalog.types.get(ti))
                .map(|t| t.name.clone())
        } else {
            self.library
                .library()
                .files
                .get(fi)
                .and_then(|f| f.types.get(ti))
                .map(|t| t.name.clone())
        }
        .unwrap_or_default();
        let mut choice = None;
        egui::Window::new("El tipo ya está en el proyecto")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "El proyecto tiene el tipo «{name}» (el mismo, con el mismo GlobalId) pero con otros valores."
                ));
                ui.horizontal(|ui| {
                    if ui
                        .button("Sustituir el del proyecto")
                        .on_hover_text("Sus elementos cambian con él")
                        .clicked()
                    {
                        choice = Some(Some(OnConflict::Replace));
                    }
                    if ui
                        .button("Cargarlo como otro tipo")
                        .on_hover_text("Con otro nombre y otro GlobalId")
                        .clicked()
                    {
                        choice = Some(Some(OnConflict::Rename));
                    }
                    if ui.button("Cancelar").clicked() {
                        choice = Some(None);
                    }
                });
            });
        if let Some(c) = choice {
            self.library.conflict = None;
            self.library.layered_conflict = None;
            if let Some(c) = c {
                if layered {
                    self.load_layered_from_library(fi, ti, c);
                } else {
                    self.load_from_library(fi, ti, c);
                }
            }
        }
    }

    fn purge_confirmation(&mut self, ctx: &egui::Context) {
        let Some(n) = self.library.purge else {
            return;
        };
        let mut done = None;
        egui::Window::new("Purgar tipos sin usar")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if n == 0 {
                    ui.label("Todos los tipos del proyecto se usan.");
                    if ui.button("Cerrar").clicked() {
                        done = Some(false);
                    }
                } else {
                    ui.label(format!(
                        "Se quitarán {n} tipos que ningún elemento usa (puertas, ventanas, objetos, muros y forjados; siempre queda uno de muro y uno de forjado). Se puede deshacer."
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Purgar").clicked() {
                            done = Some(true);
                        }
                        if ui.button("Cancelar").clicked() {
                            done = Some(false);
                        }
                    });
                }
            });
        if let Some(go) = done {
            self.library.purge = None;
            if go {
                match self.ed.purge_unused_types() {
                    Ok(n) => self.say(format!("{n} tipos sin usar quitados del proyecto.")),
                    Err(e) => self.complain(format!("No se puede purgar: {e}.")),
                }
            }
        }
    }

    /// Saves a wall or slab type of the project into the library folder.
    pub(super) fn save_layered_to_library(&mut self, t: LayeredRef) {
        let lib = library::scan(&library::library_dir());
        match library::save_layered_type(&lib, self.ed.model(), t) {
            Ok(path) => self.say(format!(
                "Tipo guardado en la biblioteca ({}).",
                path.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default()
            )),
            Err(e) => self.complain(format!("No se puede guardar en la biblioteca: {e}.")),
        }
        self.library.refresh();
    }

    /// Saves a type of the project into the library folder.
    pub(super) fn save_type_to_library(&mut self, t: FamilyTypeRef) {
        // A type with a 3D model: the model as a .glb, in a folder of its
        // kind (the gallery shows it from there).
        if let FamilyTypeRef::Object(id) = t {
            let model = self.ed.model();
            if let Some((glb, kind)) = archisim_library::imported::export_glb(model, id) {
                let name = model
                    .object_type(id)
                    .map(|t| t.name.replace(['/', '\\', ':'], "-"))
                    .unwrap_or_default();
                let dir = library::library_dir().join("Modelos 3D").join(kind.label());
                let path = dir.join(format!("{name}.glb"));
                let r = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, glb));
                match r {
                    Ok(()) => self.say(format!(
                        "Modelo 3D «{name}» guardado en la biblioteca (Modelos 3D/{}/{name}.glb).",
                        kind.label()
                    )),
                    Err(e) => self.complain(format!("No se puede guardar en la biblioteca: {e}.")),
                }
                self.folder_objects = None;
                self.library.refresh();
                return;
            }
        }
        let lib = library::scan(&library::library_dir());
        let result = self.ed.catalog_entry(t).and_then(|(entry, version)| {
            library::save_type(&lib, &entry, version).map(|p| (entry, p))
        });
        match result {
            Ok((entry, path)) => self.say(format!(
                "Tipo «{}» guardado en la biblioteca ({}).",
                entry.entry.name,
                path.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default()
            )),
            Err(e) => self.complain(format!("No se puede guardar en la biblioteca: {e}.")),
        }
        self.library.refresh();
    }
}
