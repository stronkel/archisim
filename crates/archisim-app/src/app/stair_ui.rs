//! The stair tool ("Escalera") and the properties of a selected stair:
//! its form (straight, L, U, spiral or any walking line; landings or
//! winders), what carries it, its risers and treads and the checks of the
//! Spanish code (CTE DB-SUA 1).
//!
//! A ready-made stair is drawn with two clicks: its foot (a spiral's
//! centre) and the direction it sets off in; it is as long as the height
//! asks. Space turns it the other way while drawing.

use archisim_core::{
    MaterialId, Stair, StairDesign, StairForm, StairId, StairIssue, StairPreset, StairStructure,
    StairUse,
};
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
        StairIssue::NarrowEdge { going, min } => format!(
            "Peldaño curvo de {} cm en su lado estrecho: mínimo {} cm",
            cm(going),
            cm(min)
        ),
        StairIssue::WideEdge { going, max } => format!(
            "Peldaño curvo de {} cm en su lado ancho: máximo {} cm",
            cm(going),
            cm(max)
        ),
        StairIssue::OpenRisers => {
            "Sin tabica: no vale en zonas de uso público ni para evacuar hacia arriba".to_string()
        }
        StairIssue::SplitLanding => {
            "Mesetas partidas a 45°: solo en escaleras de uso restringido".to_string()
        }
    }
}

/// The ready-made shapes, with their Spanish names.
pub const PRESETS: [(StairPreset, &str); 5] = [
    (StairPreset::Straight, "Recta"),
    (StairPreset::LShape, "En L"),
    (StairPreset::UShape, "En U (ida y vuelta)"),
    (StairPreset::Spiral, "De caracol"),
    (StairPreset::Free, "Libre (línea de huella)"),
];

/// What carries the treads, with their Spanish names.
pub const STRUCTURES: [(StairStructure, &str); 5] = [
    (StairStructure::Slab, "Losa de hormigón"),
    (StairStructure::Stringers, "Dos zancas laterales"),
    (StairStructure::Spine, "Zanca central"),
    (StairStructure::Cantilever, "Peldaños volados (empotrados)"),
    (StairStructure::Newel, "Mástil central"),
];

/// The structures a stair of a form may have.
fn structures_for(form: StairForm) -> Vec<(StairStructure, &'static str)> {
    STRUCTURES
        .iter()
        .copied()
        .filter(|(s, _)| *s != StairStructure::Newel || form == StairForm::Spiral)
        .map(|(s, n)| match (form, s) {
            (StairForm::Spiral, StairStructure::Slab) => (s, "Losa helicoidal"),
            (StairForm::Spiral, StairStructure::Stringers) => (s, "Zancas helicoidales"),
            _ => (s, n),
        })
        .collect()
}

/// The form's name.
fn form_name(d: &StairDesign) -> &'static str {
    match d.form {
        StairForm::Landings if d.split => "Mesetas partidas a 45°",
        StairForm::Landings => "Tramos y mesetas",
        StairForm::Winders => "Peldaños compensados",
        StairForm::Spiral => "De caracol",
    }
}

/// Degrees with a decimal comma, any angle (a spiral turns more than
/// once).
fn degrees_field(ui: &mut Ui, id: Id, value: f64, width: f32) -> Option<f64> {
    let shown = format!("{:.1}", value.to_degrees()).replace('.', ",");
    let mut text = ui
        .ctx()
        .data(|d| d.get_temp::<String>(id.with("buffer")))
        .unwrap_or_else(|| shown.clone());
    let r = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .desired_width(width),
    );
    if r.has_focus() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(id.with("buffer"), text));
        return None;
    }
    if r.lost_focus() {
        ui.ctx().data_mut(|d| d.remove::<String>(id.with("buffer")));
        if text != shown && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            return text
                .trim()
                .trim_end_matches('°')
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(f64::to_radians);
        }
    }
    None
}

/// A one-line summary: risers × height, treads × going.
fn summary(stair: &Stair) -> String {
    let l = stair.layout();
    let going = l.going();
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
        if self.stair_settings.preset != StairPreset::Free {
            let Some(foot) = self.stair_tool.points.first().copied() else {
                self.stair_tool.points.push(p);
                self.say(if self.stair_settings.preset == StairPreset::Spiral {
                    "Clic hacia donde arranca el primer peldaño · Espacio: girar al otro lado."
                } else {
                    "Clic hacia donde sube · Espacio: girar al otro lado."
                });
                return;
            };
            if foot.distance(p) < 0.05 {
                return;
            }
            self.stair_tool.points.clear();
            let r = self
                .ed
                .add_preset_stair(&self.stair_settings, foot, p - foot);
            self.stair_made(r);
            return;
        }
        self.stair_tool.points.push(p);
        if self.stair_tool.points.len() == 2 {
            self.say("Clic: otro tramo (en el giro, una meseta) · Intro o doble clic: terminar.");
        }
    }

    /// Builds the stair drawn so far.
    pub(super) fn finish_stair(&mut self) {
        if self.stair_settings.preset != StairPreset::Free {
            return;
        }
        let path = std::mem::take(&mut self.stair_tool.points);
        if path.len() < 2 {
            return;
        }
        let r = self.ed.add_stair(&self.stair_settings, path);
        self.stair_made(r);
    }

    fn stair_made(&mut self, r: crate::editor::EditResult<StairId>) {
        match r {
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

    /// Turns the ready-made stair being drawn the other way.
    pub(super) fn flip_stair_turn(&mut self) -> bool {
        if self.tool != Tool::Stair
            || self.stair_settings.preset == StairPreset::Free
            || self.stair_settings.preset == StairPreset::Straight
            || self.stair_tool.points.is_empty()
        {
            return false;
        }
        self.stair_settings.left = !self.stair_settings.left;
        true
    }

    /// The stair the tool would make with the cursor where it is.
    fn stair_being_drawn(&self, c: Point2) -> Option<Stair> {
        let s = &self.stair_settings;
        let model = self.ed.model();
        let (path, design) = if s.preset == StairPreset::Free {
            let mut path = self.stair_tool.points.clone();
            path.push(c);
            (path, s.free_design())
        } else {
            let foot = *self.stair_tool.points.first()?;
            s.preset_on(model, self.ed.level, foot, c - foot)?
        };
        Some(Stair {
            id: StairId(0),
            guid: Default::default(),
            level: self.ed.level,
            path,
            width: s.width,
            going: s.going,
            base_offset: 0.0,
            top: archisim_core::WallTop::Unconnected,
            height: s.height_on(model, self.ed.level),
            waist: s.waist,
            material: s.material,
            usage: s.usage,
            railings: archisim_core::Railings::Both,
            railing_type: None,
            design,
        })
    }

    /// The stair being drawn, with its treads and landings.
    pub(super) fn draw_stair_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Stair, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        if self.stair_tool.points.is_empty() {
            return;
        }
        let probe = self.stair_being_drawn(c.point);
        let l = probe.as_ref().map(Stair::layout);
        let ink = Stroke::new(1.5, ACCENT);
        if let Some(l) = &l {
            for st in &l.steps {
                painter.add(Shape::closed_line(
                    self.polygon(rect, &st.outline()),
                    Stroke::new(1.0, ACCENT),
                ));
            }
            for landing in &l.landings {
                painter.add(Shape::closed_line(
                    self.polygon(rect, &landing.outline),
                    ink,
                ));
            }
            let screen: Vec<Pos2> = l
                .walk
                .iter()
                .map(|p| self.view.to_screen(rect, *p))
                .collect();
            painter.add(Shape::line(screen, Stroke::new(1.0, ACCENT)));
        }
        let text = match &probe {
            Some(s) if l.as_ref().is_some_and(|l| !l.steps.is_empty()) => {
                let check = match s.issues().first() {
                    None => "  ✓ CTE".to_string(),
                    Some(i) => format!("  ✗ {}", issue_text(i)),
                };
                format!("{}{check}", summary(s))
            }
            _ => "No cabe: más alto o menos ancho".to_string(),
        };
        self.draw_label(painter, rect, c.point, text);
    }

    /// The materials of the model, and "the stair's" first.
    fn tread_materials(&self) -> Vec<(Option<MaterialId>, String)> {
        let mut v: Vec<(Option<MaterialId>, String)> = vec![(None, "El de la escalera".into())];
        v.extend(
            self.ed
                .model()
                .materials()
                .map(|m| (Some(m.id), m.name.clone())),
        );
        v
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
        let materials = self.tread_materials();
        let preset = self.stair_settings.preset;
        group(ui, "tool-stair-form", "Forma", |ui| {
            row(ui, "Forma", "Hecha (dos clics) o libre", |ui, w| {
                let p = choice(ui, "tool-stair-preset", w, preset, &PRESETS);
                if p != preset {
                    self.stair_settings.preset = p;
                    self.stair_tool.points.clear();
                    // A spiral round its newel; the others on a slab, unless
                    // asked otherwise.
                    match (p, self.stair_settings.structure) {
                        (StairPreset::Spiral, StairStructure::Slab) => {
                            self.stair_settings.structure = StairStructure::Newel;
                            self.stair_settings.closed_risers = false;
                            // 1,84 m across: treads of 5,5 cm by the newel
                            // and 42 cm outside, 24 cm at the walking line.
                            self.stair_settings.width = 0.8;
                            self.stair_settings.going = 0.24;
                            self.stair_settings.core = 0.12;
                            self.stair_settings.usage = StairUse::Restricted;
                        }
                        (q, StairStructure::Newel) if q != StairPreset::Spiral => {
                            self.stair_settings.structure = StairStructure::Slab;
                            self.stair_settings.closed_risers = true;
                        }
                        _ => {}
                    }
                }
            });
            if matches!(
                preset,
                StairPreset::LShape | StairPreset::UShape | StairPreset::Free
            ) {
                row(ui, "Giros", "Meseta, partida o compensados", |ui, w| {
                    let now = (
                        self.stair_settings.winders,
                        self.stair_settings.split && !self.stair_settings.winders,
                    );
                    let (winders, split) = choice(
                        ui,
                        "tool-stair-winders",
                        w,
                        now,
                        &[
                            ((false, false), "Con meseta"),
                            ((false, true), "Meseta partida a 45°"),
                            ((true, false), "Peldaños compensados"),
                        ],
                    );
                    self.stair_settings.winders = winders;
                    self.stair_settings.split = split;
                });
            }
            if matches!(
                preset,
                StairPreset::LShape | StairPreset::UShape | StairPreset::Spiral
            ) {
                row(
                    ui,
                    "Gira hacia",
                    "Subiendo · Espacio al dibujar",
                    |ui, w| {
                        self.stair_settings.left = choice(
                            ui,
                            "tool-stair-left",
                            w,
                            self.stair_settings.left,
                            &[(true, "La izquierda"), (false, "La derecha")],
                        );
                    },
                );
            }
            if preset == StairPreset::UShape {
                row(ui, "Ojo (mm)", "Hueco entre los dos tramos", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-stair-gap"),
                        self.stair_settings.gap,
                        w - 8.0,
                    ) {
                        self.stair_settings.gap = v.max(0.0);
                    }
                });
            }
            if preset == StairPreset::Spiral {
                row(
                    ui,
                    "Radio del mástil (mm)",
                    "Donde empiezan los peldaños",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique("tool-stair-core"),
                            self.stair_settings.core,
                            w - 8.0,
                        ) {
                            self.stair_settings.core = v.max(0.0);
                        }
                    },
                );
                row(
                    ui,
                    "Meseta de llegada (°)",
                    "En la planta de arriba; 0, sin meseta",
                    |ui, w| {
                        if let Some(v) = degrees_field(
                            ui,
                            Id::unique("tool-stair-arrival"),
                            self.stair_settings.arrival,
                            w - 8.0,
                        ) {
                            self.stair_settings.arrival = v.abs().min(std::f64::consts::PI);
                        }
                    },
                );
            }
        });
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
            if preset != StairPreset::Free {
                row(
                    ui,
                    "Tabica máx. (mm)",
                    "Las justas para no pasarla",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique("tool-stair-riser"),
                            self.stair_settings.riser,
                            w - 8.0,
                        ) {
                            self.stair_settings.riser = v.max(0.05);
                        }
                    },
                );
            }
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
                    &crate::units::fmt_len(height),
                );
            }
        });
        let form = match preset {
            StairPreset::Spiral => StairForm::Spiral,
            _ => StairForm::Landings,
        };
        group(ui, "tool-stair-build", "Estructura y peldaños", |ui| {
            row(ui, "Estructura", "Lo que lleva los peldaños", |ui, w| {
                self.stair_settings.structure = choice(
                    ui,
                    "tool-stair-structure",
                    w,
                    self.stair_settings.structure,
                    &structures_for(form),
                );
            });
            let st = self.stair_settings.structure;
            if st == StairStructure::Slab {
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
            } else {
                row(ui, "Tabicas", "Peldaño cerrado o abierto", |ui, w| {
                    self.stair_settings.closed_risers = choice(
                        ui,
                        "tool-stair-risers",
                        w,
                        self.stair_settings.closed_risers,
                        &[(true, "Con tabica"), (false, "Sin tabica (abierta)")],
                    );
                });
            }
            if matches!(st, StairStructure::Stringers | StairStructure::Spine) {
                row(ui, "Zanca (mm)", "Ancho × canto", |ui, w| {
                    let [a, b] = self.stair_settings.stringer;
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let half = (w - 12.0) / 2.0;
                    if let Some(v) = length_field(ui, Id::unique("tool-stair-sw"), a, half) {
                        self.stair_settings.stringer[0] = v;
                    }
                    if let Some(v) = length_field(ui, Id::unique("tool-stair-sd"), b, half) {
                        self.stair_settings.stringer[1] = v;
                    }
                });
            }
            row(ui, "Peldañeado", "Material de las huellas", |ui, w| {
                let o: Vec<_> = materials.iter().map(|(v, n)| (*v, n.as_str())).collect();
                self.stair_settings.tread_material = choice(
                    ui,
                    "tool-stair-tread-mat",
                    w,
                    self.stair_settings.tread_material,
                    &o,
                );
            });
            if st != StairStructure::Slab || self.stair_settings.tread_material.is_some() {
                row(
                    ui,
                    "Grosor de huella (mm)",
                    "Del peldaño o de su acabado",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique("tool-stair-tread"),
                            self.stair_settings.tread,
                            w - 8.0,
                        ) {
                            self.stair_settings.tread = v.clamp(0.005, 0.3);
                        }
                    },
                );
            }
        });
        // How long the flights must be for comfortable steps.
        let risers = (height / self.stair_settings.riser.max(0.05) - 1e-9)
            .ceil()
            .max(1.0) as usize;
        let length = (risers - 1) as f64 * self.stair_settings.going;
        ui.add_space(6.0);
        let how = match preset {
            StairPreset::Free => "Dibuja la línea de huella del pie a la llegada: cada clic, un tramo; en los giros, meseta o peldaños compensados.".to_string(),
            StairPreset::Spiral => "Clic en el centro y clic hacia donde arranca: da las vueltas que pida la altura.".to_string(),
            _ => "Clic en el arranque y clic hacia donde sube: mide lo que pida la altura. Espacio gira al otro lado.".to_string(),
        };
        props::note(
            ui,
            &format!(
                "{how} Para {}, {risers} tabicas de {} cm: unos {} m de línea de huella.",
                crate::units::fmt_len(height),
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
        super::properties::view_card(
            ui,
            Icon::Stair,
            &format!("{} · {}", form_name(&s.design), summary(&s)),
            "IfcStair",
        );
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
            let mut sizes = vec![
                ("Ancho (mm)", "De tramos y mesetas", s.width, 0),
                ("Huella (mm)", "La que se busca en cada tramo", s.going, 1),
            ];
            if s.design.structure == StairStructure::Slab {
                sizes.push(("Losa (mm)", "Espesor bajo los peldaños", s.waist, 2));
            }
            for (label, hint, value, key) in sizes {
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
        // What its railings are made of (E12).
        if s.railings != archisim_core::Railings::None {
            if let Some(pick) = super::railing_ui::railing_type_choice(
                ui,
                self.ed.model(),
                &format!("stair-railing-type-{}", id.0),
                s.railing_type,
            ) {
                let res = self
                    .ed
                    .edit_stair("Tipo de barandilla", id, |s| s.railing_type = pick);
                self.report(res, "Tipo de barandilla cambiado.");
            }
        }
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
                    &crate::units::fmt_len(s.height),
                );
            }
        });
        let materials = self.tread_materials();
        group(ui, "stair-build", "Forma y estructura", |ui| {
            let d = s.design;
            if d.form == StairForm::Spiral {
                text_row(ui, "Forma", "", form_name(&d));
                row(
                    ui,
                    "Giro total (°)",
                    "Del primer peldaño al último",
                    |ui, w| {
                        if let Some(v) = degrees_field(
                            ui,
                            Id::unique(("stair-sweep", id.0)),
                            d.sweep.abs(),
                            w - 8.0,
                        ) {
                            let sign = if d.sweep < 0.0 { -1.0 } else { 1.0 };
                            let r = self.ed.edit_stair("Giro de la escalera", id, |s| {
                                s.design.sweep = sign * v.abs()
                            });
                            self.report(r, "Giro cambiado.");
                        }
                    },
                );
                row(ui, "Sentido", "Subiendo", |ui, w| {
                    let left = d.sweep >= 0.0;
                    let pick = choice(
                        ui,
                        ("stair-sense", id.0),
                        w,
                        left,
                        &[(true, "Gira a la izquierda"), (false, "Gira a la derecha")],
                    );
                    if pick != left {
                        let r = self.ed.edit_stair("Sentido de la escalera", id, |s| {
                            s.design.sweep = -s.design.sweep
                        });
                        self.report(r, "Sentido cambiado.");
                    }
                });
                row(
                    ui,
                    "Radio del mástil (mm)",
                    "Donde empiezan los peldaños",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("stair-core", id.0)), d.core, w - 8.0)
                        {
                            let r = self
                                .ed
                                .edit_stair("Mástil de la escalera", id, |s| s.design.core = v);
                            self.report(r, "Mástil cambiado.");
                        }
                    },
                );
                row(
                    ui,
                    "Meseta de llegada (°)",
                    "En la planta de arriba; 0, sin meseta",
                    |ui, w| {
                        if let Some(v) = degrees_field(
                            ui,
                            Id::unique(("stair-arrival", id.0)),
                            d.arrival,
                            w - 8.0,
                        ) {
                            let v = v.abs().min(std::f64::consts::PI);
                            let r = self
                                .ed
                                .edit_stair("Meseta de llegada", id, |s| s.design.arrival = v);
                            self.report(r, "Meseta de llegada cambiada.");
                        }
                    },
                );
            } else {
                row(ui, "Forma", "En los giros", |ui, w| {
                    let now = (d.form, d.split && d.form == StairForm::Landings);
                    let f = choice(
                        ui,
                        ("stair-form", id.0),
                        w,
                        now,
                        &[
                            ((StairForm::Landings, false), "Tramos y mesetas"),
                            ((StairForm::Landings, true), "Mesetas partidas a 45°"),
                            ((StairForm::Winders, false), "Peldaños compensados"),
                        ],
                    );
                    if f != now {
                        let r = self.ed.edit_stair("Forma de la escalera", id, |s| {
                            s.design.form = f.0;
                            s.design.split = f.1;
                        });
                        self.report(r, "Forma cambiada.");
                    }
                });
            }
            row(ui, "Estructura", "Lo que lleva los peldaños", |ui, w| {
                let st = choice(
                    ui,
                    ("stair-structure", id.0),
                    w,
                    d.structure,
                    &structures_for(d.form),
                );
                if st != d.structure {
                    let r = self
                        .ed
                        .edit_stair("Estructura de la escalera", id, |s| s.design.structure = st);
                    self.report(r, "Estructura cambiada.");
                }
            });
            if d.structure != StairStructure::Slab {
                row(ui, "Tabicas", "Peldaño cerrado o abierto", |ui, w| {
                    let c = choice(
                        ui,
                        ("stair-risers", id.0),
                        w,
                        d.closed_risers,
                        &[(true, "Con tabica"), (false, "Sin tabica (abierta)")],
                    );
                    if c != d.closed_risers {
                        let r = self
                            .ed
                            .edit_stair("Tabicas", id, |s| s.design.closed_risers = c);
                        self.report(r, "Tabicas cambiadas.");
                    }
                });
            }
            if matches!(
                d.structure,
                StairStructure::Stringers | StairStructure::Spine
            ) {
                row(ui, "Zanca (mm)", "Ancho × canto", |ui, w| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let half = (w - 12.0) / 2.0;
                    for (i, v) in d.stringer.iter().enumerate() {
                        if let Some(v) =
                            length_field(ui, Id::unique(("stair-stringer", id.0, i)), *v, half)
                        {
                            let r = self
                                .ed
                                .edit_stair("Zancas", id, |s| s.design.stringer[i] = v);
                            self.report(r, "Zancas cambiadas.");
                        }
                    }
                });
            }
            row(ui, "Peldañeado", "Material de las huellas", |ui, w| {
                let o: Vec<_> = materials.iter().map(|(v, n)| (*v, n.as_str())).collect();
                let m = choice(ui, ("stair-tread-mat", id.0), w, d.tread_material, &o);
                if m != d.tread_material {
                    let r = self
                        .ed
                        .edit_stair("Peldañeado", id, |s| s.design.tread_material = m);
                    self.report(r, "Peldañeado cambiado.");
                }
            });
            if d.structure != StairStructure::Slab || d.tread_material.is_some() {
                row(
                    ui,
                    "Grosor de huella (mm)",
                    "Del peldaño o de su acabado",
                    |ui, w| {
                        if let Some(v) =
                            length_field(ui, Id::unique(("stair-tread", id.0)), d.tread, w - 8.0)
                        {
                            let r = self
                                .ed
                                .edit_stair("Grosor de huella", id, |s| s.design.tread = v);
                            self.report(r, "Huella cambiada.");
                        }
                    },
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
            for (i, run) in l.runs.iter().enumerate() {
                let going = l.steps[run.steps.clone()]
                    .iter()
                    .map(|st| st.going)
                    .fold(f64::INFINITY, f64::min);
                text_row(
                    ui,
                    &format!("Tramo {}", i + 1),
                    "Huellas y la menor, en la línea de huella",
                    &format!(
                        "{} × {} cm",
                        run.steps.len(),
                        cm(if going.is_finite() { going } else { 0.0 })
                    ),
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
