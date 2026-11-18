//! «Encajar el entorno»: the panel and the clicks that move the
//! surroundings brought in as one, without the building (`context_fit`).
//! By two points, dragged, turned, raised to a height of the ground, or
//! typed; the buildings of the surroundings that stand on the project are
//! hidden or taken away; the whole surroundings hidden at once.

use archisim_core::Point2;
use eframe::egui::{self, Id, Pos2, Rect, Shape, Stroke, Ui};

use super::props::{self, row};
use super::widgets::SwitchExt;
use super::*;
use crate::context_fit::{self, Fit, Parts};

/// How the next clicks fit the surroundings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// A point of the surroundings, then where it goes.
    Move,
    /// The centre, a direction and the direction it turns to.
    Turn,
    /// A point of the surroundings and where it goes; a second point of
    /// the surroundings and the direction it lies along.
    TwoPoints,
    /// A point of the ground: it takes the height asked for.
    Height,
}

impl FitMode {
    fn steps(self) -> usize {
        match self {
            FitMode::Move => 2,
            FitMode::Turn => 3,
            FitMode::TwoPoints => 4,
            FitMode::Height => 1,
        }
    }
}

/// The panel «Encajar el entorno».
#[derive(Debug, Clone)]
pub struct ContextFitState {
    pub open: bool,
    pub parts: Parts,
    /// The clicks so far, while picking.
    pub mode: Option<FitMode>,
    pub picks: Vec<Point2>,
    /// Typed: shift, turn (about the middle of the building) and rise.
    pub dx: f64,
    pub dy: f64,
    pub angle: f64,
    pub dz: f64,
    /// The height the point of the ground clicked takes («Cota»).
    pub height: f64,
    /// The surroundings hidden from the drawing.
    pub hidden: bool,
}

impl Default for ContextFitState {
    fn default() -> Self {
        ContextFitState {
            open: false,
            parts: Parts::default(),
            mode: None,
            picks: Vec::new(),
            dx: 0.0,
            dy: 0.0,
            angle: 0.0,
            dz: 0.0,
            height: 0.0,
            hidden: false,
        }
    }
}

impl ContextFitState {
    pub fn picking(&self) -> bool {
        self.mode.is_some()
    }

    pub fn stop(&mut self) {
        self.mode = None;
        self.picks.clear();
    }

    /// What the next click is for.
    pub fn hint(&self) -> Option<&'static str> {
        let n = self.picks.len();
        Some(match (self.mode?, n) {
            (FitMode::Move, 0) => "Mover el entorno: clic en un punto del entorno · Esc: dejarlo",
            (FitMode::Move, _) => "Mover el entorno: clic donde va ese punto · Esc: dejarlo",
            (FitMode::Turn, 0) => "Girar el entorno: clic en el centro del giro · Esc: dejarlo",
            (FitMode::Turn, 1) => "Girar el entorno: clic en una dirección del entorno (el borde de una calle) · Esc: dejarlo",
            (FitMode::Turn, _) => "Girar el entorno: clic hacia donde debe mirar esa dirección · Esc: dejarlo",
            (FitMode::TwoPoints, 0) => "Encajar por dos puntos: clic en un punto del entorno (la esquina de un edificio del Catastro, un bordillo…) · Esc: dejarlo",
            (FitMode::TwoPoints, 1) => "Encajar por dos puntos: clic en el punto del proyecto que le corresponde · Esc: dejarlo",
            (FitMode::TwoPoints, 2) => "Encajar por dos puntos: clic en un segundo punto del entorno, lejos del primero · Esc: dejarlo",
            (FitMode::TwoPoints, _) => "Encajar por dos puntos: clic en el punto del proyecto que le corresponde (solo cuenta la dirección) · Esc: dejarlo",
            (FitMode::Height, _) => "Cota del entorno: clic en un punto del terreno · Esc: dejarlo",
        })
    }

    /// The fit the clicks so far and the cursor give, to show it before
    /// the last click.
    fn fit_with(&self, model: &archisim_core::Model, last: Point2) -> Option<Fit> {
        let mut p = self.picks.clone();
        p.push(last);
        Some(match (self.mode?, p.len()) {
            (FitMode::Move, 2) => Fit::shift(p[0], p[1]),
            (FitMode::Turn, 3) => {
                let (u, v) = (p[1] - p[0], p[2] - p[0]);
                Fit::turn(p[0], v.y.atan2(v.x) - u.y.atan2(u.x))
            }
            (FitMode::TwoPoints, 2) => Fit::shift(p[0], p[1]),
            (FitMode::TwoPoints, 4) => Fit::by_two_points(p[0], p[1], p[2], p[3]),
            (FitMode::Height, 1) => {
                let z = context_fit::ground_height(model, p[0])?;
                Fit {
                    dz: self.height - z,
                    ..Fit::default()
                }
            }
            _ => return None,
        })
    }
}

impl ArchiSimApp {
    pub(super) fn context_fit_dialog(&mut self, ctx: &egui::Context) {
        if !self.context_fit.open {
            return;
        }
        let mut open = true;
        super::widgets::side_sheet(ctx, "Encajar el entorno", 400.0)
            .open(&mut open)
            .show(ctx, |ui| self.context_fit_contents(ui));
        self.context_fit.open = open;
        if !open {
            self.context_fit.stop();
        }
    }

    fn context_fit_contents(&mut self, ui: &mut Ui) {
        let counts = context_fit::counts(self.ed.model());
        props::note(
            ui,
            "Mueve, gira y sube todo lo traído alrededor del edificio, sin tocar el edificio. Si no tienes las coordenadas ni el ángulo del proyecto, trae el entorno y encájalo aquí.",
        );
        if !counts.any() {
            props::note(
                ui,
                "Este proyecto no tiene entorno: tráelo con Archivo → Traer el entorno.",
            );
            return;
        }
        let mut parts = self.context_fit.parts;
        ui.label(egui::RichText::new("Qué se mueve").strong());
        let part = |ui: &mut Ui, on: &mut bool, name: &str, n: usize| {
            ui.add_enabled_ui(n > 0, |ui| {
                ui.switch(on, format!("{name} ({n})"));
            });
        };
        part(
            ui,
            &mut parts.terrain,
            "Terreno y ortofoto",
            counts.terrains,
        );
        part(
            ui,
            &mut parts.buildings,
            "Edificios del entorno",
            counts.buildings,
        );
        part(
            ui,
            &mut parts.streets,
            "Calles, glorietas, marcas y plazas",
            counts.streets,
        );
        part(ui, &mut parts.plots, "Parcelas", counts.plots);
        part(
            ui,
            &mut parts.trees,
            "Árboles, mobiliario, coches y personas",
            counts.trees,
        );
        ui.switch(&mut parts.map, "La situación en el mapa sigue al entorno")
            .on_hover_text(
                "La georreferencia y el norte se corrigen para que el entorno siga en su sitio real y el edificio pase a estar donde lo has encajado: el IFC sale bien situado",
            );
        self.context_fit.parts = parts;
        ui.separator();
        ui.label(egui::RichText::new("Con el ratón").strong());
        let mut start = None;
        ui.horizontal_wrapped(|ui| {
            let st = &self.context_fit;
            for (mode, name, hint) in [
                (
                    FitMode::TwoPoints,
                    "Dos puntos",
                    "Un punto del entorno sobre el del proyecto, y una segunda pareja para el giro",
                ),
                (FitMode::Move, "Mover", "De un punto del entorno a donde va"),
                (
                    FitMode::Turn,
                    "Girar",
                    "Centro, una dirección del entorno y hacia dónde mira",
                ),
                (
                    FitMode::Height,
                    "Cota",
                    "Un punto del terreno toma la cota de abajo",
                ),
            ] {
                if ui
                    .selectable_label(st.mode == Some(mode), name)
                    .on_hover_text(hint)
                    .clicked()
                {
                    start = Some(mode);
                }
            }
        });
        if let Some(mode) = start {
            if self.context_fit.mode == Some(mode) {
                self.context_fit.stop();
            } else {
                self.set_tool(Tool::Arrow);
                self.context_fit.mode = Some(mode);
                self.context_fit.picks.clear();
            }
        }
        row(
            ui,
            "Cota del punto (mm)",
            "La cota que toma el punto del terreno en el que se hace clic con «Cota» (0: el ±0,00 del proyecto)",
            |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("fit-height"),
                    self.context_fit.height,
                    w - 8.0,
                ) {
                    self.context_fit.height = v;
                }
            },
        );
        ui.separator();
        ui.label(egui::RichText::new("Tecleado").strong());
        let st = &mut self.context_fit;
        for (name, hint, value, id) in [
            (
                "Mover en X (mm)",
                "Hacia el este del plano",
                &mut st.dx,
                "fit-dx",
            ),
            (
                "Mover en Y (mm)",
                "Hacia el norte del plano",
                &mut st.dy,
                "fit-dy",
            ),
            ("Subir (mm)", "Negativo: bajar", &mut st.dz, "fit-dz"),
        ] {
            row(ui, name, hint, |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(id), *value, w - 8.0) {
                    *value = v;
                }
            });
        }
        row(
            ui,
            "Girar (°)",
            "En sentido contrario a las agujas del reloj, alrededor del centro del edificio",
            |ui, w| {
                if let Some(v) =
                    super::structure_ui::angle_field(ui, Id::unique("fit-angle"), st.angle, w - 8.0)
                {
                    st.angle = v;
                }
            },
        );
        let typed = Fit {
            about: context_fit::project_middle(self.ed.model()),
            angle: st.angle,
            shift: archisim_core::Vec2::new(st.dx, st.dy),
            dz: st.dz,
        };
        let mut apply = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!typed.is_identity(), egui::Button::new("Aplicar"))
                .clicked()
            {
                apply = Some(typed);
            }
            if ui.small_button("Poner a cero").clicked() {
                *st = ContextFitState {
                    open: st.open,
                    parts: st.parts,
                    height: st.height,
                    hidden: st.hidden,
                    ..ContextFitState::default()
                };
            }
        });
        if let Some(fit) = apply {
            self.apply_context_fit(&fit);
        }
        ui.separator();
        ui.label(egui::RichText::new("Lo que pisa el proyecto").strong());
        let on = context_fit::buildings_on_the_project(self.ed.model());
        props::note(
            ui,
            &if on.is_empty() {
                "Ningún edificio del entorno pisa el edificio.".to_string()
            } else {
                format!(
                    "{} edificios del entorno caen sobre el edificio (suelen ser el mismo edificio traído del Catastro, o el solar antes de la obra).",
                    on.len()
                )
            },
        );
        ui.add_enabled_ui(!on.is_empty(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Ocultarlos").clicked() {
                    self.ed.hide(on.iter().map(|m| Element::Mass(*m)));
                    self.say(format!("{} edificios del entorno ocultos.", on.len()));
                }
                if ui.button("Quitarlos").clicked() {
                    let n = on.len();
                    let r = self.ed.transact(
                        "Quitar los edificios del entorno que pisan el proyecto",
                        |m| {
                            for id in &on {
                                m.remove_mass(*id)?;
                            }
                            Ok::<(), archisim_core::ModelError>(())
                        },
                    );
                    self.report(
                        r.map_err(|e| e.to_string()),
                        &format!("{n} edificios del entorno quitados."),
                    );
                }
            });
        });
        ui.separator();
        let mut hide = self.context_fit.hidden;
        if ui
            .switch(&mut hide, "Ocultar el entorno")
            .on_hover_text("Lo que se mueve con los interruptores de arriba deja de verse en todas las vistas, para trabajar en el edificio")
            .changed()
        {
            let items = context_fit::context_elements(self.ed.model(), self.context_fit.parts);
            if hide {
                self.ed.hide(items);
            } else {
                self.ed.unhide(items);
            }
            self.context_fit.hidden = hide;
        }
    }

    /// Moves the surroundings in one step of Undo.
    pub(super) fn apply_context_fit(&mut self, fit: &Fit) {
        let parts = self.context_fit.parts;
        let mut n = 0;
        let r = self.ed.transact("Encajar el entorno", |m| {
            n = context_fit::fit_context(m, parts, fit)?;
            Ok::<(), archisim_core::ModelError>(())
        });
        let mut what = Vec::new();
        if fit.shift.length() > 1e-9 {
            what.push(format!(
                "movido {}",
                crate::units::fmt_len(fit.shift.length())
            ));
        }
        if fit.angle.abs() > 1e-12 {
            what.push(format!("girado {:.2}°", fit.angle.to_degrees()).replace('.', ","));
        }
        if fit.dz.abs() > 1e-9 {
            what.push(format!(
                "{} {}",
                if fit.dz > 0.0 { "subido" } else { "bajado" },
                crate::units::fmt_len(fit.dz.abs())
            ));
        }
        self.report(
            r.map_err(|e| e.to_string()),
            &format!("Entorno {} ({n} elementos).", what.join(", ")),
        );
    }

    /// A click on the plan while fitting the surroundings.
    pub(super) fn context_fit_click(&mut self, p: Point2) {
        let Some(mode) = self.context_fit.mode else {
            return;
        };
        if mode == FitMode::Height && context_fit::ground_height(self.ed.model(), p).is_none() {
            self.complain("Ahí no hay terreno: haz clic sobre el terreno del entorno.");
            return;
        }
        if self.context_fit.picks.len() + 1 < mode.steps() {
            self.context_fit.picks.push(p);
            return;
        }
        let fit = self.context_fit.fit_with(self.ed.model(), p);
        self.context_fit.stop();
        if let Some(fit) = fit {
            self.apply_context_fit(&fit);
        }
    }

    /// Where the surroundings would go, over the plan, while picking.
    pub(super) fn draw_context_fit_preview(&self, painter: &egui::Painter, rect: Rect) {
        let st = &self.context_fit;
        if !st.picking() {
            return;
        }
        let accent = egui::Color32::from_rgb(0xd9, 0x6a, 0x1c);
        let dot = |at: Point2| {
            let q = self.polygon(rect, &[at]);
            if let Some(q) = q.first() {
                painter.circle_stroke(*q, 5.0, Stroke::new(2.0, accent));
            }
        };
        for p in &st.picks {
            dot(*p);
        }
        let Some(c) = self.cursor.map(|c| c.point) else {
            return;
        };
        if let Some(last) = st.picks.last() {
            let line: Vec<Pos2> = self.polygon(rect, &[*last, c]);
            painter.add(Shape::dashed_line(
                &line,
                Stroke::new(1.5, accent),
                6.0,
                4.0,
            ));
        }
        let Some(fit) = st.fit_with(self.ed.model(), c) else {
            return;
        };
        if !fit.moves() {
            return;
        }
        let model = self.ed.model();
        let lines = context_fit::preview_lines(model, self.ed.level, st.parts);
        // Not too many lines: the first ones are enough to see where it goes.
        for l in lines.iter().take(4000) {
            let moved: Vec<Point2> = l.iter().map(|p| fit.apply(*p)).collect();
            let pts = self.polygon(rect, &moved);
            painter.add(Shape::line(
                pts,
                Stroke::new(1.2, accent.gamma_multiply(0.85)),
            ));
        }
    }
}
