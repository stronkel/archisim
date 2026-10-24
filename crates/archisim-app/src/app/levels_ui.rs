//! The levels dialog ("Plantas", ArchiCAD's storey settings): add, rename,
//! move and delete levels. Every change is one undoable step.

use archisim_core::LevelId;
use eframe::egui::{self, Id, Ui};

use super::{fmt_level, length_field, text_edit_committed, ArchiSimApp};

/// Floor-to-floor height used when adding a level above or below.
const DEFAULT_STOREY_HEIGHT: f64 = 3.0;

impl ArchiSimApp {
    pub(super) fn levels_dialog(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.level_to_delete {
            self.confirm_level_delete(ctx, id);
        }
        if !self.show_levels {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Plantas")
            .open(&mut open)
            .resizable(false)
            .default_width(560.0)
            .show(ctx, |ui| self.levels_table(ui));
        self.show_levels = open;
    }

    fn levels_table(&mut self, ui: &mut Ui) {
        // Top to bottom, as in a section.
        let levels: Vec<(LevelId, String, f64)> = self
            .ed
            .model()
            .levels()
            .iter()
            .rev()
            .map(|l| (l.id, l.name.clone(), l.elevation))
            .collect();
        let only_one = levels.len() == 1;
        ui.label(
            egui::RichText::new(
                "Cota sobre el ±0,00 del proyecto. Al cambiar la cota de una planta, todo lo que contiene se mueve con ella.",
            )
            .weak(),
        );
        ui.add_space(4.0);
        egui::Grid::new("levels-grid")
            .num_columns(6)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("");
                ui.strong("Nombre");
                ui.strong("Cota (mm)");
                ui.strong("Altura hasta la de encima");
                ui.strong("Contenido");
                ui.strong("");
                ui.end_row();
                for (i, (id, name, elevation)) in levels.iter().enumerate() {
                    let active = *id == self.ed.level;
                    if ui
                        .radio(active, "")
                        .on_hover_text("Planta activa: la que se ve y donde se dibuja")
                        .clicked()
                    {
                        self.ed.level = *id;
                        self.ed.selection.clear();
                        self.fit_requested = true;
                    }
                    let new = ui
                        .scope(|ui| {
                            ui.set_min_width(170.0);
                            text_edit_committed(
                                ui,
                                Id::unique(("levels-dialog-name", id.0)),
                                name,
                                170.0,
                            )
                        })
                        .inner;
                    if let Some(new) = new {
                        let r = self.ed.rename_level(*id, &new);
                        self.report(r, "Planta renombrada.");
                    }
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("levels-dialog-elev", id.0)),
                        *elevation,
                        70.0,
                    ) {
                        let r = self.ed.set_level_elevation(*id, v);
                        self.report(r, &format!("Planta a cota {}.", fmt_level(v)));
                    }
                    match i.checked_sub(1).map(|j| levels[j].2) {
                        Some(above) => ui.label(crate::units::fmt_len(above - elevation)),
                        None => ui.label("—"),
                    };
                    let (w, o, s, a) = self.ed.model().level_contents(*id);
                    ui.label(format!("{w} muros · {o} huecos · {s} forjados · {a} cotas"));
                    if ui
                        .add_enabled(!only_one, egui::Button::new("Borrar"))
                        .on_disabled_hover_text("El proyecto necesita al menos una planta")
                        .clicked()
                    {
                        if w + o + s + a == 0 {
                            let r = self.ed.remove_level(*id);
                            self.report(r, "Planta borrada.");
                        } else {
                            self.level_to_delete = Some(*id);
                        }
                    }
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let top = levels.first().map_or(0.0, |l| l.2);
            let bottom = levels.last().map_or(0.0, |l| l.2);
            // The storey height of the top floor, if there are two or more.
            let step = match (levels.first(), levels.get(1)) {
                (Some(a), Some(b)) if a.2 - b.2 > 0.5 => a.2 - b.2,
                _ => DEFAULT_STOREY_HEIGHT,
            };
            let r = ui.button("➕ Añadir planta encima");
            super::missions_ui::mark(ui.ctx(), "button:Añadir planta encima", r.rect);
            if r.clicked() {
                let n = levels.len();
                let r = self
                    .ed
                    .add_level(&format!("Planta {n}"), top + step)
                    .map(|_| ());
                self.report(
                    r,
                    &format!("Planta añadida a cota {}.", fmt_level(top + step)),
                );
                self.fit_requested = true;
            }
            if ui.button("➕ Añadir planta debajo").clicked() {
                let e = bottom - DEFAULT_STOREY_HEIGHT;
                let r = self.ed.add_level("Sótano", e).map(|_| ());
                self.report(r, &format!("Planta añadida a cota {}.", fmt_level(e)));
                self.fit_requested = true;
            }
        });
    }

    /// Asks before deleting a level that still holds elements.
    fn confirm_level_delete(&mut self, ctx: &egui::Context, id: LevelId) {
        let model = self.ed.model();
        let Some(level) = model.level(id) else {
            self.level_to_delete = None;
            return;
        };
        let name = level.name.clone();
        let (w, o, s, a) = model.level_contents(id);
        let mut choice = None;
        egui::Modal::new(Id::unique("delete-level")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading(format!("Borrar «{name}»"));
            ui.add_space(6.0);
            ui.label(format!(
                "La planta contiene {w} muros, {o} puertas y ventanas, {s} forjados y {a} cotas. Se borrarán con ella (se puede deshacer con Ctrl+Z)."
            ));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Borrar la planta").clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancelar").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                self.level_to_delete = None;
                let r = self.ed.remove_level(id);
                self.report(r, &format!("Planta «{name}» borrada con su contenido."));
                self.fit_requested = true;
            }
            Some(false) => self.level_to_delete = None,
            None => {}
        }
    }
}
