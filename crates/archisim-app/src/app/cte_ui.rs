//! The code check window (Código Técnico): the building's use,
//! accessibility and sprinklers, and what of the model meets the code, by
//! section. A clic on a result selects its element in its plan and, for an
//! evacuation route, draws it there.

use archisim_core::{Accessibility, BuildingUse, LevelId, Point2};
use archisim_cte::{Report, Section, Status, Subject};
use eframe::egui::{self, Color32, Shape, Stroke, Ui};

use super::tabs::TabKind;
use super::theme::Palette;
use super::*;

/// The window's state.
#[derive(Default)]
pub struct CteWindow {
    pub open: bool,
    /// The report of a revision of the model.
    report: Option<(u64, Report)>,
    /// The route drawn in a plan.
    pub route: Option<(LevelId, Vec<Point2>)>,
    /// Show only what does not meet the code (and warnings).
    only_problems: bool,
}

impl CteWindow {
    /// The report for the model as it is now.
    pub fn current(&mut self, ed: &crate::editor::Editor) -> Report {
        self.report(ed).clone()
    }

    fn report(&mut self, ed: &crate::editor::Editor) -> &Report {
        let rev = ed.revision();
        if self.report.as_ref().is_none_or(|(r, _)| *r != rev) {
            self.report = Some((rev, archisim_cte::check(&ed.shown())));
        }
        &self.report.as_ref().expect("just made").1
    }
}

const ROUTE: Color32 = Color32::from_rgb(214, 96, 32);

impl ArchiSimApp {
    pub(super) fn cte_dialog(&mut self, ctx: &egui::Context) {
        if !self.cte.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Comprobación CTE")
            .open(&mut open)
            .default_width(620.0)
            .default_height(560.0)
            .show(ctx, |ui| self.cte_contents(ui));
        self.cte.open = open;
        if !open {
            self.cte.route = None;
        }
    }

    fn cte_contents(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let s = self.ed.model().code_settings();
        let mut new = s;
        egui::Grid::new("cte-settings")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Uso del edificio");
                egui::ComboBox::from_id_salt("cte-use")
                    .selected_text(archisim_cte::use_name(s.building_use))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for u in BuildingUse::ALL {
                            ui.selectable_value(
                                &mut new.building_use,
                                u,
                                archisim_cte::use_name(u),
                            );
                        }
                    });
                ui.end_row();
                ui.label("Accesibilidad");
                egui::ComboBox::from_id_salt("cte-access")
                    .selected_text(archisim_cte::accessibility_name(s.accessibility))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for a in Accessibility::ALL {
                            ui.selectable_value(
                                &mut new.accessibility,
                                a,
                                archisim_cte::accessibility_name(a),
                            );
                        }
                    });
                ui.end_row();
                ui.label("Extinción automática");
                ui.checkbox(
                    &mut new.sprinklers,
                    "rociadores en todo el edificio (recorridos +25 %)",
                );
                ui.end_row();
            });
        if new != s {
            let r = self.ed.set_code_settings(new);
            self.report(r, "Uso del edificio cambiado.");
        }
        ui.label(
            egui::RichText::new(
                "Cada local es lo que dice su uso (Despacho, Aula, Pasillo, Aseo accesible…): de él salen su ocupación y lo que se le pide. La ocupación de un local se puede fijar en sus propiedades.",
            )
            .weak()
            .size(11.0),
        );
        ui.separator();
        let report = self.cte.current(&self.ed);
        let model = self.ed.model();
        ui.horizontal(|ui| {
            ui.colored_label(p.bad, format!("No cumple: {}", report.count(Status::Fails)));
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("Avisos: {}", report.count(Status::Warning)),
            );
            ui.colored_label(p.good, format!("Cumple: {}", report.count(Status::Passes)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.cte.only_problems, "Solo lo que falla");
            });
        });
        let mut pick: Option<archisim_cte::Item> = None;
        let mut export = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 36.0)
            .show(ui, |ui| {
                for section in Section::ALL {
                    let items: Vec<&archisim_cte::Item> = report
                        .items_of(section)
                        .filter(|i| {
                            !self.cte.only_problems
                                || matches!(i.status, Status::Fails | Status::Warning)
                        })
                        .collect();
                    if items.is_empty() {
                        continue;
                    }
                    let fails = items.iter().filter(|i| i.status == Status::Fails).count();
                    egui::CollapsingHeader::new(if fails > 0 {
                        format!("{} · {fails} no cumple", section.title())
                    } else {
                        section.title().to_string()
                    })
                    .id_salt(("cte-section", section))
                    .default_open(true)
                    .show(ui, |ui| {
                        for i in items {
                            let (mark, color) = match i.status {
                                Status::Fails => ("✖", p.bad),
                                Status::Warning => ("!", ui.visuals().warn_fg_color),
                                Status::Passes => ("✔", p.good),
                                Status::Info => ("·", p.muted),
                            };
                            let who = archisim_cte::subject_name(model, i.subject);
                            let mut job = egui::text::LayoutJob::default();
                            let f = egui::TextFormat {
                                color: p.text,
                                ..Default::default()
                            };
                            job.append(
                                &format!("{mark} "),
                                0.0,
                                egui::TextFormat { color, ..f.clone() },
                            );
                            if !matches!(i.subject, Subject::Building | Subject::Level(_)) {
                                job.append(
                                    &format!("{who} · "),
                                    0.0,
                                    egui::TextFormat {
                                        color: p.muted,
                                        ..f.clone()
                                    },
                                );
                            }
                            job.append(&i.text, 0.0, f.clone());
                            if !i.value.is_empty() && i.status != Status::Info {
                                let fig = if i.required.is_empty() {
                                    format!("  {}", i.value)
                                } else {
                                    format!("  {} (exigido {})", i.value, i.required)
                                };
                                job.append(&fig, 0.0, egui::TextFormat { color, ..f.clone() });
                            }
                            let r = ui
                                .add(egui::Label::new(job).wrap().sense(egui::Sense::click()))
                                .on_hover_text(format!("{} · clic: verlo en la planta", i.rule));
                            if r.clicked() {
                                pick = Some(i.clone());
                            }
                        }
                    });
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Exportar informe (CSV)…").clicked() {
                export = true;
            }
            if self.cte.route.is_some() && ui.button("Quitar el recorrido de la planta").clicked() {
                self.cte.route = None;
            }
        });
        if let Some(i) = pick {
            self.cte_pick(&i);
        }
        if export {
            self.cte_export(&report);
        }
    }

    pub(super) fn cte_pick(&mut self, i: &archisim_cte::Item) {
        let element = match i.subject {
            Subject::Room(r) => Some(Element::Room(r)),
            Subject::Opening(o) => Some(Element::Opening(o)),
            Subject::Stair(s) => Some(Element::Stair(s)),
            Subject::Slab(s) => Some(Element::Slab(s)),
            Subject::Object(o) => Some(Element::Object(o)),
            Subject::Run(r) => Some(Element::Run(r)),
            Subject::Building | Subject::Level(_) => None,
        };
        if let Some(level) = i.level {
            self.open_view(TabKind::Plan(level));
        }
        self.set_tool(Tool::Arrow);
        self.ed.selection = element.into_iter().collect();
        self.cte.route = match (i.level, i.route.is_empty()) {
            (Some(l), false) => Some((l, i.route.clone())),
            _ => None,
        };
    }

    fn cte_export(&mut self, report: &Report) {
        let mut d = rfd::FileDialog::new()
            .set_title("Exportar la comprobación CTE")
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("{}-cte.csv", self.ed.display_name()));
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.save_file() {
            match std::fs::write(&path, report.to_csv(self.ed.model())) {
                Ok(()) => self.say(format!("Informe guardado en {}.", path.display())),
                Err(e) => self.complain(format!("No se puede guardar el informe: {e}.")),
            }
        }
    }

    /// The evacuation route picked in the window, on its plan.
    pub(super) fn draw_cte_route(&self, painter: &egui::Painter, rect: egui::Rect) {
        let Some((level, route)) = &self.cte.route else {
            return;
        };
        if *level != self.ed.level || route.len() < 2 {
            return;
        }
        let pts = self.polygon(rect, route);
        painter.add(Shape::line(pts.clone(), Stroke::new(3.0, ROUTE)));
        painter.circle_filled(pts[0], 5.0, ROUTE);
        // An arrow at the exit.
        let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
        let d = (b - a).normalized();
        if d.length() > 0.0 {
            let n = egui::vec2(-d.y, d.x);
            painter.add(Shape::convex_polygon(
                vec![b, b - d * 12.0 + n * 6.0, b - d * 12.0 - n * 6.0],
                ROUTE,
                Stroke::NONE,
            ));
        }
    }
}
