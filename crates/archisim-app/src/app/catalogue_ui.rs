//! The system's catalogue (M8): the box types of the library folder «Cajas
//! tipo» in the Caja tool (placed as modules, against the boxes beside
//! them), a box saved as a box type, the box type's own page when the
//! project is one (code, revision, approve, new revision and what a house
//! may choose), and the choices of each copy.

use std::path::{Path, PathBuf};

use archisim_core::{
    BoxId, CatalogueEntry, CatalogueOption, CatalogueStatus, ModuleId, ModuleSourceId,
};
use eframe::egui::{self, ComboBox, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// The library folder of the box types.
pub const BOX_TYPES_FOLDER: &str = "Cajas tipo";

/// A box type found in the library folder.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogueRow {
    pub path: PathBuf,
    pub code: String,
    pub label: String,
    pub name: String,
    pub approved: bool,
}

/// The box types of a folder, by code.
pub fn read_catalogue(dir: &Path) -> Vec<CatalogueRow> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some(archisim_file::EXTENSION) {
            continue;
        }
        let Ok(model) = archisim_file::open(&path) else {
            continue;
        };
        let Some(c) = model.catalogue() else {
            continue;
        };
        out.push(CatalogueRow {
            path,
            code: c.code.clone(),
            label: c.label(),
            name: c.name.clone(),
            approved: c.approved(),
        });
    }
    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
}

/// Today as «09/11/2026».
pub(super) fn today() -> String {
    let iso = archisim_ifc::utc_now();
    let (y, rest) = iso.split_at(4.min(iso.len()));
    let m = rest.get(1..3).unwrap_or("");
    let d = rest.get(4..6).unwrap_or("");
    format!("{d}/{m}/{y}")
}

impl ArchiSimApp {
    fn catalogue_dir(&self) -> PathBuf {
        crate::editor::library::library_dir().join(BOX_TYPES_FOLDER)
    }

    /// The box types of the library (read once, and again on asking).
    fn catalogue_rows(&mut self) -> Vec<CatalogueRow> {
        if self.box_catalogue.is_none() {
            let dir = self.catalogue_dir();
            self.box_catalogue = Some(read_catalogue(&dir));
        }
        self.box_catalogue.clone().unwrap_or_default()
    }

    /// The module of the project that is this box type, if it is loaded.
    fn loaded_box_type(&self, code: &str) -> Option<ModuleSourceId> {
        self.ed
            .model()
            .module_sources()
            .find(|s| s.model.catalogue().is_some_and(|c| c.code == code))
            .map(|s| s.id)
    }

    /// Places copies of a box type: loads it if it is not in the project.
    pub(super) fn place_box_type(&mut self, row: &CatalogueRow) {
        match self.loaded_box_type(&row.code) {
            Some(id) => {
                self.module_tool.source = Some(id);
                self.set_tool(Tool::Module);
                self.say(format!(
                    "{}: clic para colocar una copia (junto a otra caja, se pega a ella) · Espacio: girar.",
                    row.label
                ));
            }
            None => self.load_module_from(&row.path.clone()),
        }
    }

    /// The box types of the library, in the Caja tool.
    pub(super) fn catalogue_group(&mut self, ui: &mut Ui) {
        let rows = self.catalogue_rows();
        let mut place = None;
        let mut refresh = false;
        group(ui, "tool-box-catalogue", "Cajas tipo del catálogo", |ui| {
            if rows.is_empty() {
                props::note(
                    ui,
                    &format!(
                        "No hay cajas tipo en la carpeta «{BOX_TYPES_FOLDER}» de la biblioteca. Selecciona una caja con lo que lleva y usa «Guardar como caja tipo…»."
                    ),
                );
            }
            for r in &rows {
                ui.horizontal(|ui| {
                    let mark = if r.approved { "✓" } else { "·" };
                    ui.label(format!("{mark} {}", r.label))
                        .on_hover_text(&r.name);
                    if ui
                        .small_button("Colocar")
                        .on_hover_text("Colocar copias de esta caja tipo en la planta")
                        .clicked()
                    {
                        place = Some(r.clone());
                    }
                });
            }
            if ui.small_button("Volver a leer la carpeta").clicked() {
                refresh = true;
            }
        });
        if refresh {
            self.box_catalogue = None;
        }
        if let Some(r) = place {
            self.place_box_type(&r);
        }
    }

    /// «Guardar como caja tipo…» of a selected box: the box and what it
    /// holds, as a box type in design, in the library folder.
    pub(super) fn save_box_type(&mut self, id: BoxId) {
        let Some(b) = self.ed.model().modular_box(id).cloned() else {
            return;
        };
        let dir = self.catalogue_dir();
        let _ = std::fs::create_dir_all(&dir);
        let suggested = format!("CB-{}.{}", b.mark, archisim_file::EXTENSION);
        let mut d = rfd::FileDialog::new()
            .set_title("Guardar la caja como caja tipo")
            .add_filter("ArchiSim", &[archisim_file::EXTENSION])
            .set_file_name(suggested);
        if dir.is_dir() {
            d = d.set_directory(&dir);
        }
        let Some(path) = d.save_file() else {
            return;
        };
        self.write_box_type(id, &path);
    }

    /// Writes the box type of box `id` to `path`; its code is the file's
    /// name.
    pub(super) fn write_box_type(&mut self, id: BoxId, path: &Path) {
        let Some(b) = self.ed.model().modular_box(id).cloned() else {
            return;
        };
        let code = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("CB-{}", b.mark));
        let name = if b.kind.is_empty() {
            "Caja".to_string()
        } else {
            b.kind.clone()
        };
        let entry = CatalogueEntry::new(code.clone(), name.clone());
        let made = self.ed.model().box_type_model(id, entry);
        match made {
            Ok(mut t) => {
                t.project_name = name;
                match archisim_file::save(&t, path) {
                    Ok(()) => {
                        self.box_catalogue = None;
                        self.say(format!(
                            "Caja tipo {code} guardada (en diseño): ábrela para revisarla y aprobarla."
                        ));
                    }
                    Err(e) => self.complain(format!("No se puede guardar la caja tipo: {e}.")),
                }
            }
            Err(e) => self.complain(format!("No se puede hacer la caja tipo: {e}.")),
        }
    }

    /// The page of a project that is a box type (or can become one).
    pub(super) fn box_type_group(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let entry = model.catalogue().cloned();
        let one_box = model.boxes().count() == 1;
        let Some(c) = entry else {
            if one_box {
                let mut make = false;
                group(ui, "box-type", "Caja tipo del catálogo", |ui| {
                    props::note(
                        ui,
                        "Este proyecto tiene una sola caja: puede ser una caja tipo del catálogo.",
                    );
                    make = ui.button("Hacer de este proyecto una caja tipo").clicked();
                });
                if make {
                    let name = self
                        .ed
                        .model()
                        .boxes()
                        .next()
                        .map(|b| b.kind.clone())
                        .unwrap_or_default();
                    let code = self.ed.model().project_name.clone();
                    let r = self.ed.transact("Caja tipo", |m| {
                        m.set_catalogue(Some(CatalogueEntry::new(code, name)))
                    });
                    self.report(r, "Ahora es una caja tipo en diseño.");
                }
            }
            return;
        };
        // Doors are swapped for doors, windows for windows, a wall's
        // finish for another wall type, and so on (M15).
        let slots = model.slot_types();
        let opening_names: Vec<String> = slots.iter().map(|(_, n)| n.clone()).collect();
        let kind_of = |name: &str| slots.iter().find(|(_, n)| n == name).map(|(k, _)| *k);
        let mut n = c.clone();
        let mut approve = false;
        let mut reopen = false;
        let draft = c.status == CatalogueStatus::Draft;
        group(ui, "box-type", "Caja tipo del catálogo", |ui| {
            text_row(ui, "Estado", "", c.status.label());
            text_row(
                ui,
                "Revisión",
                "La última aprobada",
                &if c.revision == 0 {
                    "ninguna aún".to_string()
                } else if c.approved_on.is_empty() {
                    format!("{}", c.revision)
                } else {
                    format!("{} · {}", c.revision, c.approved_on)
                },
            );
            ui.add_enabled_ui(draft, |ui| {
                row(ui, "Código", "En el catálogo y en las copias", |ui, w| {
                    ui.add(egui::TextEdit::singleline(&mut n.code).desired_width(w - 8.0));
                });
                row(ui, "Nombre", "", |ui, w| {
                    ui.add(egui::TextEdit::singleline(&mut n.name).desired_width(w - 8.0));
                });
                row(ui, "Notas", "Qué cambia en esta revisión", |ui, w| {
                    ui.add(egui::TextEdit::multiline(&mut n.notes).desired_width(w - 8.0));
                });
            });
            ui.horizontal(|ui| {
                if draft {
                    approve = ui
                        .button(format!("Aprobar la revisión {}", c.revision + 1))
                        .on_hover_text("La caja tipo queda cerrada: las viviendas la usan tal cual")
                        .clicked();
                } else {
                    reopen = ui
                        .button(format!("Abrir la revisión {}", c.revision + 1))
                        .on_hover_text("Para cambiarla; las viviendas siguen con la aprobada hasta que se actualicen")
                        .clicked();
                }
            });
            if !draft {
                props::note(
                    ui,
                    "Aprobada: no se puede cambiar. Para mejorarla, abre una revisión nueva, cámbiala y apruébala.",
                );
            }
        });
        group(ui, "box-type-options", "Lo que elige cada vivienda", |ui| {
            props::note(
                ui,
                "Un tipo de la caja tipo que cada vivienda puede cambiar por otros de su clase: puertas y ventanas, acabados de muros y suelos, techos y equipamiento.",
            );
            let mut remove = None;
            for (i, o) in n.options.iter_mut().enumerate() {
                ui.add_enabled_ui(draft, |ui| {
                    row(ui, "Opción", "", |ui, w| {
                        ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(w - 8.0));
                    });
                    row(ui, "Tipo", "El de la caja tipo", |ui, w| {
                        ComboBox::from_id_salt(("box-type-slot", i))
                            .width(w - 8.0)
                            .selected_text(o.slot.clone())
                            .show_ui(ui, |ui| {
                                for (k, t) in &slots {
                                    ui.selectable_value(
                                        &mut o.slot,
                                        t.clone(),
                                        format!("{} · {t}", k.label()),
                                    );
                                }
                            });
                    });
                    let slot_kind = kind_of(&o.slot);
                    for t in opening_names
                        .iter()
                        .filter(|t| **t != o.slot && kind_of(t) == slot_kind)
                    {
                        let mut on = o.choices.contains(t);
                        if ui.checkbox(&mut on, t).changed() {
                            if on {
                                o.choices.push(t.clone());
                            } else {
                                o.choices.retain(|x| x != t);
                            }
                        }
                    }
                    if ui.small_button("Quitar la opción").clicked() {
                        remove = Some(i);
                    }
                });
                ui.separator();
            }
            if let Some(i) = remove {
                n.options.remove(i);
            }
            if draft && !opening_names.is_empty() && ui.button("Añadir una opción").clicked() {
                n.options.push(CatalogueOption {
                    name: format!("Opción {}", n.options.len() + 1),
                    slot: opening_names[0].clone(),
                    choices: Vec::new(),
                });
            }
        });
        if approve {
            let on = today();
            let r = self.ed.transact("Aprobar la caja tipo", |m| {
                m.approve_catalogue(on).map(drop)
            });
            self.report(r, "Caja tipo aprobada: queda cerrada en esta revisión.");
        } else if reopen {
            let r = self.ed.transact("Revisión nueva de la caja tipo", |m| {
                m.new_catalogue_revision()
            });
            self.report(r, "Revisión nueva abierta: ya se puede cambiar.");
        } else if n != c && draft {
            let r = self.ed.transact("Caja tipo", |m| m.set_catalogue(Some(n)));
            if let Err(e) = r {
                self.complain(format!("No se puede cambiar la caja tipo: {e}."));
            }
        }
    }

    /// What a copy of a box type is and what it chose.
    pub(super) fn module_box_type_group(&mut self, ui: &mut Ui, id: ModuleId) {
        let model = self.ed.model();
        let Some(p) = model.module(id).cloned() else {
            return;
        };
        let Some(c) = model
            .module_source(p.source)
            .and_then(|s| s.model.catalogue().cloned())
        else {
            return;
        };
        let mut choices = p.choices.clone();
        group(ui, "module-box-type", "Caja tipo", |ui| {
            text_row(ui, "Caja tipo", "", &c.label());
            if !c.approved() {
                props::note(ui, "Aviso: esta caja tipo no está aprobada.");
            }
            for o in &c.options {
                let current = choices
                    .iter()
                    .find(|(n, _)| *n == o.name)
                    .map(|(_, v)| v.clone());
                let base = format!("{} (la de la caja)", o.slot);
                row(ui, &o.name, "", |ui, w| {
                    let mut v = current.clone();
                    ComboBox::from_id_salt(("module-choice", id, &o.name))
                        .width(w - 8.0)
                        .selected_text(v.clone().unwrap_or_else(|| base.clone()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut v, None, &base);
                            for ch in &o.choices {
                                ui.selectable_value(&mut v, Some(ch.clone()), ch);
                            }
                        });
                    if v != current {
                        choices.retain(|(n, _)| *n != o.name);
                        if let Some(v) = v {
                            choices.push((o.name.clone(), v));
                        }
                    }
                });
            }
        });
        if choices != p.choices {
            let r = self
                .ed
                .edit_module("Elección de la caja tipo", id, |m| m.choices = choices);
            self.report(r, "Elección cambiada.");
        }
    }
}
