//! Schedules in the window: listed in the navigator, shown as a table
//! that can be written as CSV or placed on the sheet shown.

use archisim_core::{LevelId, ScheduleKind};
use eframe::egui::{self, Ui};

use super::icons::Icon;
use super::tabs::{SchedTab, TabKind};
use super::widgets::list_row;
use super::*;

impl ArchiSimApp {
    /// The schedules in the navigator: each opens in a tab; dragged onto a
    /// sheet, it is placed there.
    pub(super) fn nav_table_rows(&mut self, ui: &mut Ui) {
        let active = self.tabs.get(self.active_tab).map(|t| t.kind);
        let r = list_row(
            ui,
            Some(Icon::Measure),
            None,
            "Mediciones",
            "todo",
            active == Some(TabKind::Schedule(SchedTab::Takeoff)),
            10.0,
        )
        .on_hover_text("Las mediciones de todo el proyecto por tipos, con el BC3");
        if r.clicked() {
            self.open_view(TabKind::Schedule(SchedTab::Takeoff));
        }
        for kind in ScheduleKind::ALL {
            let t = archisim_plan::schedules::table(&self.ed.shown(), kind, None);
            let open = active == Some(TabKind::Schedule(SchedTab::Kind(kind)));
            let r = super::widgets::list_row_draggable(
                ui,
                Some(Icon::Measure),
                None,
                kind.label(),
                &format!("{} filas", t.rows.len()),
                open,
                10.0,
            )
            .on_hover_text("Abrir la tabla en una pestaña; arrástrala a un plano para colocarla");
            if r.clicked() {
                self.open_view(TabKind::Schedule(SchedTab::Kind(kind)));
            }
            super::sheet_drop::drag_source(
                ui,
                &r,
                super::sheet_drop::Drop::Table(kind),
                kind.label(),
            );
        }
    }

    /// A schedule (or all the measurements) in its tab: what it lists,
    /// for which level, and its export.
    pub(super) fn schedule_view(&mut self, ui: &mut Ui, which: SchedTab) {
        let p = super::theme::Palette::current(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        self.canvas = rect;
        ui.painter().rect_filled(rect, 0.0, p.panel);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(egui::vec2(18.0, 12.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut child;
        let shown = self.ed.shown();
        let model: &archisim_core::Model = &shown;
        let levels: Vec<(Option<LevelId>, String)> =
            std::iter::once((None, "Todo el edificio".to_string()))
                .chain(model.levels().iter().map(|l| (Some(l.id), l.name.clone())))
                .collect();
        let mut level = self.schedule_level;
        let mut export_csv = false;
        let mut export_bc3 = false;
        let mut kind_change = None;
        ui.horizontal(|ui| {
            let title = match which {
                SchedTab::Takeoff => "Mediciones".to_string(),
                SchedTab::Kind(k) => k.label().to_string(),
            };
            ui.label(
                egui::RichText::new(title)
                    .font(super::widgets::strong_font(16.0))
                    .color(p.text),
            );
            ui.add_space(12.0);
            if let SchedTab::Kind(k) = which {
                egui::ComboBox::from_id_salt("schedule-kind")
                    .selected_text("Otra tabla")
                    .show_ui(ui, |ui| {
                        for other in ScheduleKind::ALL {
                            if ui.selectable_label(other == k, other.label()).clicked() {
                                kind_change = Some(other);
                            }
                        }
                    });
            }
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
                export_csv = true;
            }
            if ui
                .button("Exportar BC3…")
                .on_hover_text(
                    "Presupuesto FIEBDC-3 (Presto, Arquímedes…) de todo el proyecto, sin precios",
                )
                .clicked()
            {
                export_bc3 = true;
            }
        });
        ui.label(
            egui::RichText::new(
                "Se rehace sola al cambiar el modelo. Para ponerla en un plano, arrástrala desde el navegador hasta el plano.",
            )
            .small()
            .color(p.muted),
        );
        ui.add_space(8.0);
        let kinds: Vec<ScheduleKind> = match which {
            SchedTab::Kind(k) => vec![k],
            SchedTab::Takeoff => ScheduleKind::ALL.to_vec(),
        };
        let tables: Vec<(ScheduleKind, archisim_plan::schedules::Table)> = kinds
            .iter()
            .map(|k| (*k, archisim_plan::schedules::table(model, *k, level)))
            .collect();
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (k, table) in &tables {
                    if which == SchedTab::Takeoff {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(k.label())
                                .font(super::widgets::strong_font(13.0))
                                .color(p.text),
                        );
                        ui.add_space(2.0);
                    }
                    table_grid(ui, *k, table);
                    if table.rows.is_empty() {
                        ui.label(
                            egui::RichText::new("Nada que listar todavía.")
                                .small()
                                .color(p.muted),
                        );
                    }
                    ui.add_space(10.0);
                }
            });
        self.schedule_level = level;
        if let Some(k) = kind_change {
            self.replace_active_tab(TabKind::Schedule(SchedTab::Kind(k)));
        }
        if export_csv {
            let name = match which {
                SchedTab::Takeoff => "Mediciones".to_string(),
                SchedTab::Kind(k) => k.label().to_string(),
            };
            let csv: String = tables
                .iter()
                .map(|(k, t)| {
                    if which == SchedTab::Takeoff {
                        format!("{}\n{}\n", k.label(), t.csv())
                    } else {
                        t.csv()
                    }
                })
                .collect();
            let mut d = rfd::FileDialog::new()
                .set_title("Exportar tabla (CSV)")
                .add_filter("CSV (hoja de cálculo)", &["csv"])
                .set_file_name(format!("{} - {name}.csv", self.file_name_stem()));
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            if let Some(path) = d.save_file() {
                match std::fs::write(&path, csv) {
                    Ok(()) => self.say(format!("Tabla exportada en {}.", path.display())),
                    Err(e) => self.complain(format!("No se pudo exportar: {e}.")),
                }
            }
        }
        if export_bc3 {
            self.export_bc3();
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

/// The rows of a schedule, totals in bold, numbers to the right.
fn table_grid(ui: &mut Ui, kind: ScheduleKind, table: &archisim_plan::schedules::Table) {
    egui::Grid::new(("schedule-grid", kind))
        .striped(true)
        .min_col_width(40.0)
        .spacing([18.0, 4.0])
        .show(ui, |ui| {
            for c in &table.columns {
                ui.label(egui::RichText::new(c.name.as_ref()).strong());
            }
            ui.end_row();
            let cell = |ui: &mut Ui, i: usize, text: egui::RichText| {
                if table.columns.get(i).is_some_and(|c| c.numeric) {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(text)
                    });
                } else {
                    ui.label(text);
                }
            };
            for row in table.rows.iter() {
                for (i, v) in row.iter().enumerate() {
                    cell(ui, i, egui::RichText::new(v));
                }
                ui.end_row();
            }
            if let Some(t) = &table.totals {
                for (i, v) in t.iter().enumerate() {
                    cell(ui, i, egui::RichText::new(v).strong());
                }
                ui.end_row();
            }
        });
}
