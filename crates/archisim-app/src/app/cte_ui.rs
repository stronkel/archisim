//! The code check window (Código Técnico): the building's use,
//! accessibility and sprinklers, and what of the model meets the code, by
//! section. A clic on a result selects its element in its plan and, for an
//! evacuation route, draws it there.

use super::widgets::SwitchExt as _;
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
    /// The one section shown, picked on its card; `None`, all.
    section: Option<Section>,
    /// «Arreglar lo que no cumple», while it is worked out or shown.
    fix: Option<super::fix_ui::FixState<archisim_cte::fix::Fix>>,
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

/// The element a result is about.
fn subject_element(m: &archisim_core::Model, s: Subject) -> Option<Element> {
    match s {
        Subject::Room(r) => Some(Element::Room(r)),
        Subject::Opening(o) => Some(Element::Opening(o)),
        Subject::Stair(s) => Some(Element::Stair(s)),
        Subject::Slab(s) => Some(Element::Slab(s)),
        Subject::Object(o) => Some(Element::Object(o)),
        Subject::Run(r) => Some(Element::Run(r)),
        Subject::Reinforced(id) => super::fix_ui::structural_element(m, id).0,
        Subject::Wall(w) => Some(Element::Wall(w)),
        Subject::Roof(r) => Some(Element::Roof(r)),
        Subject::Floor(s) => Some(Element::Slab(s)),
        Subject::Device(d) => Some(Element::Device(d)),
        Subject::Building | Subject::Level(_) => None,
    }
}

/// The level of what a result is about.
fn subject_level(m: &archisim_core::Model, s: Subject) -> Option<LevelId> {
    match s {
        Subject::Reinforced(id) => super::fix_ui::structural_element(m, id).1,
        Subject::Opening(o) => m.opening(o).and_then(|o| m.wall(o.wall)).map(|w| w.level),
        Subject::Run(r) => m.runs().find(|x| x.id == r).map(|r| r.level),
        Subject::Object(o) => m.objects().find(|x| x.id == o).map(|o| o.level),
        _ => None,
    }
}

const ROUTE: Color32 = Color32::from_rgb(214, 96, 32);

impl ArchiSimApp {
    pub(super) fn cte_dialog(&mut self, ctx: &egui::Context) {
        if !self.cte.open {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Comprobación CTE")
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
                ui.switch(
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
        let mut start_fix = false;
        super::report_view::summary(
            ui,
            super::report_view::Tally {
                pass: report.count(Status::Passes),
                warn: report.count(Status::Warning),
                fail: report.count(Status::Fails),
            },
            "el proyecto",
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.switch(&mut self.cte.only_problems, "Solo lo que falla");
                if ui
                    .add_enabled(
                        report.count(Status::Fails) > 0,
                        egui::Button::new("Arreglar lo que no cumple…"),
                    )
                    .on_hover_text("Arregla solo lo que tiene una respuesta clara —la estructura, el diámetro y la pendiente de las tuberías, el ancho y el sentido de las puertas— y te enseña lo que cambia antes de hacerlo; lo demás te lo deja dicho")
                    .clicked()
                {
                    start_fix = true;
                }
            });
        });
        if start_fix {
            self.cte.fix = Some(super::fix_ui::FixState::start(
                self.ed.revision(),
                self.ed.model().clone(),
                ui.ctx().clone(),
                |m| {
                    let f = archisim_cte::fix::fix(m);
                    let rows = f
                        .lines
                        .iter()
                        .map(|l| super::fix_ui::Row {
                            element: subject_element(m, l.subject),
                            level: subject_level(m, l.subject),
                            name: l.name.clone(),
                            what: l.what,
                            before: l.before.clone(),
                            after: l.after.clone(),
                        })
                        .collect();
                    super::fix_ui::Shown {
                        rows,
                        fails_before: f.fails_before,
                        fails_after: f.fails_after,
                        left: f.left.clone(),
                        fix: f,
                    }
                },
            ));
        }
        let rev = self.ed.revision();
        if self.cte.fix.as_ref().is_some_and(|f| f.revision() != rev) {
            self.cte.fix = None;
        }
        if let Some(f) = self.cte.fix.as_mut() {
            f.poll();
            match super::fix_ui::panel(
                ui,
                f,
                "Solo lo que tiene una respuesta clara: la estructura, el diámetro y la pendiente de las tuberías, el ancho y el sentido de las puertas.",
            ) {
                super::fix_ui::Answer::Nothing => {}
                super::fix_ui::Answer::Back => self.cte.fix = None,
                super::fix_ui::Answer::Pick(e, l) => self.fix_pick(e, l),
                super::fix_ui::Answer::Apply => {
                    if let Some(super::fix_ui::FixState::Ready(_, s)) = self.cte.fix.take() {
                        let n = s.rows.len();
                        let r = self.ed.transact("Arreglar lo que no cumple el CTE", |m| {
                            archisim_cte::fix::apply_all(m, &s.fix)
                        });
                        self.report(
                            r,
                            &format!(
                                "{n} cambio{} para cumplir el CTE; quedan {} sin cumplir. Deshacer los quita.",
                                if n == 1 { "" } else { "s" },
                                s.fails_after
                            ),
                        );
                    }
                }
            }
            return;
        }
        let model = self.ed.model();
        let mut pick: Option<archisim_cte::Item> = None;
        let mut export = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 36.0)
            .show(ui, |ui| {
                // A card per section: its counts and bar; a clic shows it
                // alone (another clic, all of them).
                let shown: Vec<Section> = Section::ALL
                    .into_iter()
                    .filter(|s| report.items_of(*s).next().is_some())
                    .collect();
                // As many cards a row as fit.
                let per_row = ((ui.available_width() + 6.0) / 226.0).floor().max(1.0) as usize;
                for row in shown.chunks(per_row) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for &section in row {
                            let items: Vec<&archisim_cte::Item> =
                                report.items_of(section).collect();
                            let count =
                                |st: Status| items.iter().filter(|i| i.status == st).count();
                            let t = super::report_view::Tally {
                                pass: count(Status::Passes),
                                warn: count(Status::Warning),
                                fail: count(Status::Fails),
                            };
                            let on = self.cte.section == Some(section);
                            let edge = if t.fail > 0 {
                                p.bad
                            } else if t.warn > 0 {
                                ui.visuals().warn_fg_color
                            } else {
                                p.good
                            };
                            let r = egui::Frame::new()
                                .fill(if on {
                                    edge.gamma_multiply(0.12)
                                } else {
                                    p.panel
                                })
                                .stroke(Stroke::new(
                                    if on { 2.0 } else { 1.0 },
                                    edge.gamma_multiply(0.7),
                                ))
                                .corner_radius(egui::CornerRadius::same(10))
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.set_width(196.0);
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(section.title())
                                                    .strong()
                                                    .size(12.5)
                                                    .color(p.text),
                                            )
                                            .truncate(),
                                        );
                                        ui.horizontal(|ui| {
                                            if t.fail > 0 {
                                                super::report_view::pill(
                                                    ui,
                                                    &format!("✖ {}", t.fail),
                                                    p.bad,
                                                );
                                            }
                                            if t.warn > 0 {
                                                super::report_view::pill(
                                                    ui,
                                                    &format!("! {}", t.warn),
                                                    ui.visuals().warn_fg_color,
                                                );
                                            }
                                            super::report_view::pill(
                                                ui,
                                                &format!("✔ {}", t.pass),
                                                p.good,
                                            );
                                        });
                                        super::report_view::bar(ui, t);
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                                .on_hover_text(if on {
                                    "Clic: ver todas"
                                } else {
                                    "Clic: ver solo esta"
                                });
                            if r.clicked() {
                                self.cte.section = if on { None } else { Some(section) };
                            }
                        }
                    });
                    ui.add_space(4.0);
                }
                ui.add_space(6.0);
                for section in Section::ALL {
                    if self.cte.section.is_some_and(|s| s != section) {
                        continue;
                    }
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
                    egui::CollapsingHeader::new(
                        egui::RichText::new(if fails > 0 {
                            format!(
                                "{} · {fails} no cumple{}",
                                section.title(),
                                if fails == 1 { "" } else { "n" }
                            )
                        } else {
                            section.title().to_string()
                        })
                        .strong()
                        .size(14.0),
                    )
                    .id_salt(("cte-section", section))
                    .default_open(fails > 0 || self.cte.section.is_some())
                    // The section picked on its card, open.
                    .open((self.cte.section == Some(section)).then_some(true))
                    .show(ui, |ui| {
                        // The text takes what the pill, the element and the
                        // figures leave.
                        let text_w =
                            (ui.available_width() - 80.0 - 150.0 - 260.0 - 40.0).max(260.0);
                        egui::Grid::new(("cte-items", section))
                            .striped(true)
                            .num_columns(4)
                            .spacing(egui::vec2(10.0, 4.0))
                            .show(ui, |ui| {
                                for i in items {
                                    let (label, colour) = match i.status {
                                        Status::Fails => ("No cumple", p.bad),
                                        Status::Warning => ("Revisar", ui.visuals().warn_fg_color),
                                        Status::Passes => ("Cumple", p.good),
                                        Status::Info => ("Dato", p.muted),
                                    };
                                    ui.add_sized(egui::vec2(80.0, 18.0), |ui: &mut Ui| {
                                        super::report_view::pill(ui, label, colour)
                                    });
                                    let who = if matches!(
                                        i.subject,
                                        Subject::Building | Subject::Level(_)
                                    ) {
                                        String::new()
                                    } else {
                                        archisim_cte::subject_name(model, i.subject)
                                    };
                                    ui.add_sized(
                                        egui::vec2(150.0, 18.0),
                                        egui::Label::new(
                                            egui::RichText::new(&who).color(p.muted).size(12.0),
                                        )
                                        .truncate(),
                                    );
                                    let r =
                                        ui.vertical(|ui| {
                                            ui.set_width(text_w);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&i.text).size(12.5),
                                                )
                                                .wrap()
                                                .sense(egui::Sense::click()),
                                            )
                                        })
                                        .inner
                                        .on_hover_text(
                                            format!("{} · clic: verlo en la planta", i.rule),
                                        );
                                    if r.clicked() {
                                        pick = Some(i.clone());
                                    }
                                    let fig = match (
                                        i.value.is_empty() || i.status == Status::Info,
                                        i.required.is_empty(),
                                    ) {
                                        (true, _) => String::new(),
                                        (false, true) => i.value.clone(),
                                        (false, false) => {
                                            format!("{}  ·  exigido {}", i.value, i.required)
                                        }
                                    };
                                    ui.vertical(|ui| {
                                        ui.set_width(260.0);
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(fig)
                                                    .color(colour)
                                                    .size(12.0)
                                                    .strong(),
                                            )
                                            .wrap(),
                                        );
                                    });
                                    ui.end_row();
                                }
                            });
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
        let element = subject_element(self.ed.model(), i.subject);
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
