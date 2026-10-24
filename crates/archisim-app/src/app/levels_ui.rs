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
        super::widgets::side_sheet(ctx, "Plantas", 560.0)
            .open(&mut open)
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

/// The bubble «Nueva planta» (concepto de interfaz del 24/10/2026): its
/// name, the height of the storey, above or below the level shown, and
/// what will happen to the levels it moves.
#[derive(Debug, Clone, PartialEq)]
pub struct NewLevel {
    /// The button it comes from.
    pub at: egui::Rect,
    pub name: String,
    pub height: f64,
    pub above: bool,
    /// The frame it opened in (its click does not close it).
    pub opened: u64,
}

impl ArchiSimApp {
    pub(super) fn open_new_level(&mut self, at: egui::Rect) {
        let n = self.ed.model().levels().len();
        self.new_level = Some(NewLevel {
            at,
            name: format!("Planta {n}"),
            height: DEFAULT_STOREY_HEIGHT,
            above: true,
            opened: 0,
        });
    }

    /// Where the new level goes and which levels move, by how much: its
    /// elevation, the levels pushed (name, from, to).
    pub(super) fn new_level_plan(&self, d: &NewLevel) -> (f64, Vec<(String, f64, f64)>) {
        let model = self.ed.model();
        let mut levels: Vec<(String, f64)> = model
            .levels()
            .into_iter()
            .map(|l| (l.name.clone(), l.elevation))
            .collect();
        levels.sort_by(|a, b| a.1.total_cmp(&b.1));
        let here = model.level(self.ed.level).map_or(0.0, |l| l.elevation);
        if d.above {
            let next = levels.iter().map(|l| l.1).find(|e| *e > here + 1e-6);
            let at = next.unwrap_or(here + d.height);
            let moved = levels
                .iter()
                .filter(|l| l.1 >= at - 1e-6)
                .map(|l| (l.0.clone(), l.1, l.1 + d.height))
                .collect();
            (at, moved)
        } else {
            let at = here - d.height;
            let moved = levels
                .iter()
                .filter(|l| l.1 < here - 1e-6)
                .map(|l| (l.0.clone(), l.1, l.1 - d.height))
                .collect();
            (at, moved)
        }
    }

    /// The bubble, if open.
    pub(super) fn new_level_bubble(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.new_level.clone() else {
            return;
        };
        let frame = ctx.cumulative_frame_nr();
        if d.opened == 0 {
            d.opened = frame;
        }
        let p = super::theme::Palette::current(ctx);
        let (at, moved) = self.new_level_plan(&d);
        let here = self
            .ed
            .model()
            .level(self.ed.level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let mut add = false;
        let mut cancel = false;
        let area = egui::Area::new(Id::unique("new-level"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(egui::pos2(d.at.left() - 10.0, d.at.top() - 40.0))
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.set_width(300.0);
                        ui.label(
                            egui::RichText::new("Nueva planta")
                                .strong()
                                .size(15.0)
                                .color(p.text),
                        );
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Nombre").small().color(p.muted));
                        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(280.0));
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Altura de la planta")
                                .small()
                                .color(p.muted),
                        );
                        if let Some(v) =
                            super::length_field(ui, Id::unique("new-level-height"), d.height, 120.0)
                        {
                            if v > 0.5 {
                                d.height = v;
                            }
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut d.above, true, format!("Encima de {here}"));
                            ui.selectable_value(&mut d.above, false, "Debajo");
                        });
                        ui.add_space(8.0);
                        // The levels, as a little section, the new one marked.
                        let mut rows: Vec<(String, f64, bool)> = self
                            .ed
                            .model()
                            .levels()
                            .into_iter()
                            .map(|l| {
                                let e = moved
                                    .iter()
                                    .find(|m| m.0 == l.name && (m.1 - l.elevation).abs() < 1e-9)
                                    .map_or(l.elevation, |m| m.2);
                                (l.name.clone(), e, false)
                            })
                            .collect();
                        rows.push((d.name.clone(), at, true));
                        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
                        for (name, e, new) in &rows {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(
                                    egui::vec2(60.0, 14.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().line_segment(
                                    [r.left_center(), r.right_center()],
                                    egui::Stroke::new(
                                        if *new { 2.5 } else { 1.2 },
                                        if *new { p.accent } else { p.muted },
                                    ),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{name}  {}",
                                        super::fmt_level(*e)
                                    ))
                                    .size(12.0)
                                    .color(if *new {
                                        p.accent
                                    } else {
                                        p.text
                                    }),
                                );
                            });
                        }
                        ui.add_space(6.0);
                        let mut note = format!("Se pone a {}.", super::fmt_level(at));
                        match moved.len() {
                            0 => {}
                            1 => note.push_str(&format!(
                                " «{}» {} {} con todo lo que tiene.",
                                moved[0].0,
                                if d.above { "sube" } else { "baja" },
                                crate::units::fmt_len(d.height)
                            )),
                            n => note.push_str(&format!(
                                " Las {n} plantas de {} {} {} con todo lo que tienen.",
                                if d.above { "encima" } else { "debajo" },
                                if d.above { "suben" } else { "bajan" },
                                crate::units::fmt_len(d.height)
                            )),
                        }
                        ui.label(egui::RichText::new(note).size(12.0).color(p.muted));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancelar").clicked() {
                                cancel = true;
                            }
                            let b = egui::Button::new(
                                egui::RichText::new("Añadir planta").color(egui::Color32::WHITE),
                            )
                            .fill(p.accent);
                            if ui.add(b).clicked() {
                                add = true;
                            }
                        });
                    });
            });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|q| !area.response.rect.contains(q))
        });
        if cancel
            || ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || (outside && frame > d.opened)
        {
            self.new_level = None;
            return;
        }
        if add {
            self.new_level = None;
            let r = self.ed.insert_level(&d.name, at, d.height, d.above);
            match r {
                Ok(_) => {
                    self.say(format!(
                        "«{}» añadida a {}.",
                        d.name.trim(),
                        super::fmt_level(at)
                    ));
                    self.fit_requested = true;
                }
                Err(e) => self.complain(format!("No se puede añadir la planta: {e}.")),
            }
            return;
        }
        self.new_level = Some(d);
    }
}
