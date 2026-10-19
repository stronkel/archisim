//! The stair tool ("Escalera") and the properties of a selected stair:
//! risers, treads and the checks of the Spanish code (CTE DB-SUA 1).

use archisim_core::{Stair, StairId, StairIssue, StairUse};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, row, text_row};
use super::*;

/// The walking line being drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StairTool {
    pub points: Vec<Point2>,
}

/// Uses offered, with their Spanish names.
const USES: [(StairUse, &str); 2] = [
    (StairUse::General, "Uso general"),
    (StairUse::Restricted, "Uso restringido (vivienda)"),
];

/// A length in centimetres with one decimal and a decimal comma.
fn cm(v: f64) -> String {
    format!("{:.1}", v * 100.0).replace('.', ",")
}

/// What the code asks, in Spanish.
pub fn issue_text(i: &StairIssue) -> String {
    match *i {
        StairIssue::TreadTooShort { going, min } => {
            format!("Huella de {} cm: mínimo {} cm", cm(going), cm(min))
        }
        StairIssue::RiserOutOfRange { riser, min, max } => {
            if min > 0.0 {
                format!(
                    "Tabica de {} cm: entre {} y {} cm",
                    cm(riser),
                    cm(min),
                    cm(max)
                )
            } else {
                format!("Tabica de {} cm: máximo {} cm", cm(riser), cm(max))
            }
        }
        StairIssue::StepRule { value } => {
            format!("2 tabicas + 1 huella = {} cm: entre 54 y 70 cm", cm(value))
        }
        StairIssue::TooNarrow { width, min } => {
            format!("Ancho de {} cm: mínimo {} cm", cm(width), cm(min))
        }
        StairIssue::ShortFlight { risers } => {
            format!("Tramo de {risers} peldaños: mínimo 3")
        }
        StairIssue::NoHandrail => "Sin pasamanos: obligatorio al salvar 55 cm o más".to_string(),
        StairIssue::HandrailBothSides => {
            "Pasamanos en un lado: con 1,20 m de ancho o más, en los dos".to_string()
        }
        StairIssue::FlightTooHigh { height, max } => format!(
            "Tramo que salva {} m: máximo {} m (pon una meseta)",
            fmt_m(height),
            fmt_m(max)
        ),
    }
}

/// A one-line summary: risers × height, treads × going.
fn summary(stair: &Stair) -> String {
    let l = stair.layout();
    let going = l
        .flights
        .iter()
        .map(|f| f.going)
        .fold(f64::INFINITY, f64::min);
    format!(
        "{} tabicas de {} cm · {} huellas de {} cm",
        l.risers(),
        cm(l.riser),
        l.treads(),
        cm(if going.is_finite() { going } else { 0.0 })
    )
}

impl ArchiSimApp {
    pub(super) fn stair_click(&mut self, p: Point2) {
        if self
            .stair_tool
            .points
            .last()
            .is_some_and(|q| q.approx_eq(p))
        {
            return;
        }
        self.stair_tool.points.push(p);
        if self.stair_tool.points.len() == 2 {
            self.say("Clic: otro tramo (en el giro, una meseta) · Intro o doble clic: terminar.");
        }
    }

    /// Builds the stair drawn so far.
    pub(super) fn finish_stair(&mut self) {
        let path = std::mem::take(&mut self.stair_tool.points);
        if path.len() < 2 {
            return;
        }
        match self.ed.add_stair(&self.stair_settings, path) {
            Ok(id) => {
                self.ed.selection = [Element::Stair(id)].into_iter().collect();
                let s = self.ed.model().stair(id).cloned();
                if let Some(s) = s {
                    let issues = s.issues();
                    if issues.is_empty() {
                        self.say(format!("Escalera creada: {}. Cumple el CTE.", summary(&s)));
                    } else {
                        self.complain(format!(
                            "Escalera creada: {}. No cumple el CTE: {}.",
                            summary(&s),
                            issue_text(&issues[0])
                        ));
                    }
                }
            }
            Err(e) => self.complain(format!("No se puede crear la escalera: {e}.")),
        }
    }

    /// The stair being drawn, with its flights, landings and steps.
    pub(super) fn draw_stair_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Stair, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        if self.stair_tool.points.is_empty() {
            return;
        }
        let mut path = self.stair_tool.points.clone();
        path.push(c.point);
        let s = &self.stair_settings;
        let model = self.ed.model();
        let height = s.height_on(model, self.ed.level);
        let l = archisim_core::layout(&path, s.width, s.going, height, 0.0);
        let ink = Stroke::new(1.5, ACCENT);
        for f in &l.flights {
            let o = f.outline(s.width);
            painter.add(Shape::closed_line(self.polygon(rect, &o), ink));
            let u = f.direction();
            let n = u.perp_left() * (s.width / 2.0);
            for i in 1..f.treads {
                let p = f.start + u * (i as f64 * f.going);
                painter.line_segment(
                    [
                        self.view.to_screen(rect, p - n),
                        self.view.to_screen(rect, p + n),
                    ],
                    Stroke::new(1.0, ACCENT),
                );
            }
        }
        for landing in &l.landings {
            painter.add(Shape::closed_line(
                self.polygon(rect, &landing.outline),
                ink,
            ));
        }
        let screen: Vec<Pos2> = path.iter().map(|p| self.view.to_screen(rect, *p)).collect();
        painter.add(Shape::line(screen, Stroke::new(1.0, ACCENT)));
        let probe = Stair {
            id: StairId(0),
            guid: Default::default(),
            level: self.ed.level,
            path: path.clone(),
            width: s.width,
            going: s.going,
            base_offset: 0.0,
            top: archisim_core::WallTop::Unconnected,
            height,
            waist: s.waist,
            material: s.material,
            usage: s.usage,
            railings: archisim_core::Railings::Both,
        };
        let check = match probe.issues().first() {
            None if !l.flights.is_empty() => "  ✓ CTE".to_string(),
            None => String::new(),
            Some(i) => format!("  ✗ {}", issue_text(i)),
        };
        self.draw_label(
            painter,
            rect,
            c.point,
            format!("{}{check}", summary(&probe)),
        );
    }

    pub(super) fn stair_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Escalera · lo que vas a crear");
        let model = self.ed.model();
        let height = self.stair_settings.height_on(model, self.ed.level);
        let above: Vec<(archisim_core::LevelId, String)> = {
            let z = model.level(self.ed.level).map_or(0.0, |l| l.elevation);
            model
                .levels()
                .iter()
                .filter(|l| l.elevation > z)
                .map(|l| (l.id, l.name.clone()))
                .collect()
        };
        group(ui, "tool-stair", "Escalera", |ui| {
            row(
                ui,
                "Ancho (mm)",
                "Ancho libre de tramos y mesetas",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-stair-width"),
                        self.stair_settings.width,
                        w - 8.0,
                    ) {
                        self.stair_settings.width = v;
                    }
                },
            );
            row(
                ui,
                "Huella (mm)",
                "La que se busca; cada tramo la ajusta a su longitud",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-stair-going"),
                        self.stair_settings.going,
                        w - 8.0,
                    ) {
                        self.stair_settings.going = v;
                    }
                },
            );
            row(ui, "Losa (mm)", "Espesor bajo los peldaños", |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("tool-stair-waist"),
                    self.stair_settings.waist,
                    w - 8.0,
                ) {
                    self.stair_settings.waist = v;
                }
            });
            row(ui, "Uso", "Para comprobar el CTE DB-SUA", |ui, w| {
                self.stair_settings.usage =
                    choice(ui, "tool-stair-use", w, self.stair_settings.usage, &USES);
            });
            row(ui, "Llega a", "La planta de llegada", |ui, w| {
                let mut opts: Vec<(crate::editor::TopChoice, String)> = vec![
                    (
                        crate::editor::TopChoice::Unconnected,
                        "Altura propia".into(),
                    ),
                    (
                        crate::editor::TopChoice::NextLevel,
                        "La planta de encima".into(),
                    ),
                ];
                opts.extend(
                    above
                        .iter()
                        .map(|(id, n)| (crate::editor::TopChoice::Level(*id), n.clone())),
                );
                let o: Vec<_> = opts.iter().map(|(v, n)| (*v, n.as_str())).collect();
                self.stair_settings.top =
                    choice(ui, "tool-stair-top", w, self.stair_settings.top, &o);
            });
            if self.stair_settings.top == crate::editor::TopChoice::Unconnected {
                row(ui, "Altura (mm)", "Lo que sube", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-stair-height"),
                        self.stair_settings.height,
                        w - 8.0,
                    ) {
                        self.stair_settings.height = v;
                    }
                });
            } else {
                text_row(
                    ui,
                    "Altura",
                    "De esta planta a la de llegada",
                    &format!("{} mm", fmt_mm(height)),
                );
            }
        });
        // How long the flights must be for comfortable steps.
        let risers = (height / 0.175).ceil().max(1.0) as usize;
        let length = (risers - 1) as f64 * self.stair_settings.going;
        ui.add_space(6.0);
        props::note(
            ui,
            &format!(
                "Dibuja la línea de huella del pie a la llegada: cada clic, un tramo; en los giros, meseta. Para {} mm, unas {risers} tabicas de {} cm: unos {} m de tramos.",
                fmt_mm(height),
                cm(height / risers as f64),
                fmt_m(length)
            ),
        );
    }

    pub(super) fn stair_properties(&mut self, ui: &mut Ui, id: StairId) {
        let model = self.ed.model();
        let Some(s) = model.stair(id).cloned() else {
            return;
        };
        let l = s.layout();
        props::title(ui, "Escalera · 1 seleccionada");
        super::properties::view_card(ui, Icon::Stair, &summary(&s), "IfcStair");
        let above: Vec<(archisim_core::LevelId, String)> = {
            let z = model.level(s.level).map_or(0.0, |l| l.elevation);
            model
                .levels()
                .iter()
                .filter(|l| l.elevation > z)
                .map(|l| (l.id, l.name.clone()))
                .collect()
        };
        group(ui, "stair-size", "Dimensiones", |ui| {
            for (label, hint, value, key) in [
                ("Ancho (mm)", "De tramos y mesetas", s.width, 0),
                ("Huella (mm)", "La que se busca en cada tramo", s.going, 1),
                ("Losa (mm)", "Espesor bajo los peldaños", s.waist, 2),
            ] {
                row(ui, label, hint, |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("stair-size", id.0, key)), value, w - 8.0)
                    {
                        let r = self.ed.edit_stair("Cambiar escalera", id, |s| match key {
                            0 => s.width = v,
                            1 => s.going = v,
                            _ => s.waist = v,
                        });
                        self.report(r, "Escalera cambiada.");
                    }
                });
            }
            row(
                ui,
                "Barandillas",
                "Lados con barandilla (subiendo), a 90 cm",
                |ui, w| {
                    use archisim_core::Railings;
                    let r = choice(
                        ui,
                        ("stair-rails", id.0),
                        w,
                        s.railings,
                        &[
                            (Railings::Both, "A los dos lados"),
                            (Railings::Left, "A la izquierda"),
                            (Railings::Right, "A la derecha"),
                            (Railings::None, "Sin barandilla"),
                        ],
                    );
                    if r != s.railings {
                        let res = self.ed.edit_stair("Barandillas", id, |s| s.railings = r);
                        self.report(res, "Barandillas cambiadas.");
                    }
                },
            );
            row(ui, "Uso", "Para comprobar el CTE DB-SUA", |ui, w| {
                let u = choice(ui, ("stair-use", id.0), w, s.usage, &USES);
                if u != s.usage {
                    let r = self
                        .ed
                        .edit_stair("Uso de la escalera", id, |s| s.usage = u);
                    self.report(r, "Uso cambiado.");
                }
            });
        });
        group(ui, "stair-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("stair-level", id),
                "Planta",
                Some(s.level),
                &[Element::Stair(id)],
            );
            row(
                ui,
                "Llega a",
                "La planta de llegada, o altura propia",
                |ui, w| {
                    let current = match s.top {
                        archisim_core::WallTop::Level { level, .. } => Some(level),
                        archisim_core::WallTop::Unconnected => None,
                    };
                    let mut opts: Vec<(Option<archisim_core::LevelId>, String)> =
                        vec![(None, "Altura propia".into())];
                    opts.extend(above.iter().map(|(l, n)| (Some(*l), n.clone())));
                    let o: Vec<_> = opts.iter().map(|(v, n)| (*v, n.as_str())).collect();
                    let picked = choice(ui, ("stair-top", id.0), w, current, &o);
                    if picked != current {
                        let r = self.ed.edit_stair("Llegada de la escalera", id, |s| {
                            s.top = match picked {
                                Some(level) => archisim_core::WallTop::Level { level, offset: 0.0 },
                                None => archisim_core::WallTop::Unconnected,
                            }
                        });
                        self.report(r, "Llegada cambiada.");
                    }
                },
            );
            if matches!(s.top, archisim_core::WallTop::Unconnected) {
                row(ui, "Altura (mm)", "Lo que sube", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("stair-height", id.0)), s.height, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_stair("Altura de la escalera", id, |s| s.height = v);
                        self.report(r, "Altura cambiada.");
                    }
                });
            } else {
                text_row(
                    ui,
                    "Altura",
                    "De su planta a la de llegada",
                    &format!("{} mm", fmt_mm(s.height)),
                );
            }
        });
        group(ui, "stair-steps", "Peldaños", |ui| {
            text_row(
                ui,
                "Tabicas",
                "Todas iguales",
                &format!("{} × {} cm", l.risers(), cm(l.riser)),
            );
            for (i, f) in l.flights.iter().enumerate() {
                text_row(
                    ui,
                    &format!("Tramo {}", i + 1),
                    "Huellas y su fondo",
                    &format!("{} × {} cm", f.treads, cm(f.going)),
                );
            }
            text_row(ui, "Mesetas", "", &l.landings.len().to_string());
        });
        group(ui, "stair-code", "CTE DB-SUA 1", |ui| {
            let issues = s.issues();
            if issues.is_empty() {
                ui.label(
                    egui::RichText::new("✓ Cumple: huella, tabica, 2T+H, ancho y tramos.").small(),
                );
            }
            for i in &issues {
                ui.label(
                    egui::RichText::new(format!("✗ {}", issue_text(i)))
                        .small()
                        .color(egui::Color32::from_rgb(196, 64, 48)),
                );
            }
        });
        self.phase_group(ui, "stair-phase", &[Element::Stair(id)]);
        group(ui, "stair-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &s.guid.to_string());
        });
    }
}
