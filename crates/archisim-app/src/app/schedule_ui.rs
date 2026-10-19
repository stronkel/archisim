//! Schedules in the window: listed in the navigator, shown as a table
//! that can be written as CSV or placed on the sheet shown.

use archisim_core::{LevelId, ScheduleKind};
use eframe::egui::{self, Ui};

use super::icons::Icon;
use super::widgets::list_row;
use super::*;

impl ArchiSimApp {
    /// The schedules in the navigator.
    pub(super) fn nav_table_rows(&mut self, ui: &mut Ui) {
        for kind in ScheduleKind::ALL {
            let t = archisim_plan::schedules::table(self.ed.model(), kind, None);
            let open = self.schedule_open.is_some_and(|o| o.0 == kind);
            let r = list_row(
                ui,
                Some(Icon::Measure),
                None,
                kind.label(),
                &format!("{} filas", t.rows.len()),
                open,
                10.0,
            )
            .on_hover_text("Ver la tabla, exportarla o colocarla en un plano");
            if r.clicked() {
                self.schedule_open = Some((kind, None));
            }
        }
    }

    /// The schedule window.
    pub(super) fn schedule_window(&mut self, ctx: &egui::Context) {
        let Some((mut kind, mut level)) = self.schedule_open else {
            return;
        };
        let mut open = true;
        let mut export = false;
        let mut place = false;
        let on_sheet = match self.mode {
            ViewMode::Sheet(s) => Some(s),
            _ => None,
        };
        let model = self.ed.model();
        let levels: Vec<(Option<LevelId>, String)> =
            std::iter::once((None, "Todo el edificio".to_string()))
                .chain(model.levels().iter().map(|l| (Some(l.id), l.name.clone())))
                .collect();
        let table = archisim_plan::schedules::table(model, kind, level);
        egui::Window::new("Tabla de planificación")
            .open(&mut open)
            .default_size([760.0, 420.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("schedule-kind")
                        .selected_text(kind.label())
                        .show_ui(ui, |ui| {
                            for k in ScheduleKind::ALL {
                                ui.selectable_value(&mut kind, k, k.label());
                            }
                        });
                    let shown = levels
                        .iter()
                        .find(|l| l.0 == level)
                        .map_or("", |l| l.1.as_str());
                    egui::ComboBox::from_id_salt("schedule-level")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (l, name) in &levels {
                                ui.selectable_value(&mut level, *l, name);
                            }
                        });
                    if ui.button("Exportar CSV…").clicked() {
                        export = true;
                    }
                    if ui
                        .add_enabled(on_sheet.is_some(), egui::Button::new("Colocar en el plano"))
                        .on_disabled_hover_text("Abra un plano para colocar la tabla en él")
                        .clicked()
                    {
                        place = true;
                    }
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    egui::Grid::new("schedule-grid")
                        .striped(true)
                        .min_col_width(40.0)
                        .show(ui, |ui| {
                            for c in &table.columns {
                                ui.label(egui::RichText::new(c.name).strong());
                            }
                            ui.end_row();
                            for row in table.rows.iter() {
                                for (i, v) in row.iter().enumerate() {
                                    if table.columns.get(i).is_some_and(|c| c.numeric) {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| ui.label(v),
                                        );
                                    } else {
                                        ui.label(v);
                                    }
                                }
                                ui.end_row();
                            }
                            if let Some(t) = &table.totals {
                                for (i, v) in t.iter().enumerate() {
                                    let text = egui::RichText::new(v).strong();
                                    if table.columns.get(i).is_some_and(|c| c.numeric) {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| ui.label(text),
                                        );
                                    } else {
                                        ui.label(text);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    if table.rows.is_empty() {
                        ui.label("No hay nada que listar todavía.");
                    }
                });
            });
        self.schedule_open = open.then_some((kind, level));
        if export {
            let mut d = rfd::FileDialog::new()
                .set_title("Exportar tabla (CSV)")
                .add_filter("CSV (hoja de cálculo)", &["csv"])
                .set_file_name(format!("{} - {}.csv", self.file_name_stem(), kind.label()));
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            if let Some(path) = d.save_file() {
                match std::fs::write(&path, table.csv()) {
                    Ok(()) => self.say(format!("Tabla exportada en {}.", path.display())),
                    Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
                }
            }
        }
        if let (true, Some(sheet)) = (place, on_sheet) {
            let (w, h) = self
                .ed
                .model()
                .sheet(sheet)
                .map_or((841.0, 594.0), |s| (s.width, s.height));
            let at = Point2::new((w - 160.0) * 0.001, (h - 20.0) * 0.001);
            match self.ed.add_table(sheet, kind, level, at) {
                Ok(t) => {
                    self.sheet_sel = Some(super::sheet_ui::Placed::Table(t));
                    self.say("Tabla colocada: arrástrala a su sitio.");
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            }
        }
    }

    /// Exports the measurements as a BC3 budget.
    pub(super) fn export_bc3(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Exportar mediciones (BC3)")
            .add_filter("BC3 (FIEBDC-3: Presto, Arquímedes…)", &["bc3"])
            .set_file_name(format!("{}.bc3", self.file_name_stem()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match self.ed.export_bc3(&path) {
                Ok(()) => self.say(format!(
                    "Mediciones exportadas en {} (muros, forjados, carpinterías y acabados, sin precios).",
                    path.display()
                )),
                Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
            }
        }
    }
}
