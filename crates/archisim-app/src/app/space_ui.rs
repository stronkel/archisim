//! The room tool ("Local", ArchiCAD's zone, Revit's room) and the
//! properties of a selected room: number, name, use, height and areas.

use archisim_core::SpaceId;
use archisim_geom::Contour;
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, id_row, row, text_row};
use super::*;
use crate::editor::USAGES;
use crate::slab_tool::Outcome as PolyOutcome;

impl ArchiSimApp {
    /// Whether a click with the room tool takes the room around it.
    pub(super) fn zone_wand(&self) -> bool {
        self.space_settings.wand && !self.zone_tool.is_drawing()
    }

    pub(super) fn space_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Local · lo que vas a crear");
        let level = self.ed.level;
        let height = crate::editor::space_height(self.ed.model(), level, &self.space_settings);
        let next = self.ed.model().next_space_number(level);
        group(ui, "tool-space-method", "Método", |ui| {
            ui.add_space(2.0);
            #[derive(Clone, Copy, PartialEq)]
            enum Method {
                Wand,
                Polygon,
                Rectangle,
            }
            let current = match (self.space_settings.wand, self.zone_tool.mode) {
                (true, _) => Method::Wand,
                (false, SlabMode::Rectangle) => Method::Rectangle,
                (false, SlabMode::Polygon) => Method::Polygon,
            };
            let mut method = current;
            if super::widgets::method_picker(
                ui,
                "space-method",
                &mut method,
                &[
                    (
                        Method::Wand,
                        Icon::GeoWand,
                        "Recinto",
                        "Clic dentro de un recinto de muros: el local por las caras interiores",
                    ),
                    (
                        Method::Polygon,
                        Icon::GeoPolygon,
                        "Polígono",
                        "Esquina a esquina",
                    ),
                    (
                        Method::Rectangle,
                        Icon::GeoRect,
                        "Rectángulo",
                        "Dos esquinas opuestas",
                    ),
                ],
            ) {
                self.space_settings.wand = method == Method::Wand;
                match method {
                    Method::Polygon => self.zone_tool.set_mode(SlabMode::Polygon),
                    Method::Rectangle => self.zone_tool.set_mode(SlabMode::Rectangle),
                    Method::Wand => self.zone_tool.finish(),
                }
            }
            if self.space_settings.wand {
                row(
                    ui,
                    "Contorno",
                    "Superficie útil: caras interiores",
                    |ui, w| {
                        self.space_settings.contour = choice(
                            ui,
                            "tool-space-contour",
                            w,
                            self.space_settings.contour,
                            &[
                                (Contour::InteriorFace, "Caras interiores"),
                                (Contour::WallAxis, "Ejes de muros"),
                            ],
                        );
                    },
                );
            }
        });
        group(ui, "tool-space-data", "Datos", |ui| {
            text_row(ui, "Número", "El siguiente libre de esta planta", &next);
            row(ui, "Nombre", "Dormitorio 1, Cocina…", |ui, w| {
                let shown = self.space_settings.name.clone();
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("tool-space-name"), &shown, w - 8.0)
                {
                    self.space_settings.name = v;
                }
            });
            self.usage_row(ui, "tool-space-usage", None);
            row(
                ui,
                "Altura (mm)",
                "Altura libre; vacío: de la planta a la de encima",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-space-height"), height, w - 8.0)
                    {
                        self.space_settings.height = (v > 0.0).then_some(v);
                    }
                },
            );
        });
    }

    /// The use of a room, from a list (the name follows it if it was the
    /// default one).
    fn usage_row(&mut self, ui: &mut Ui, salt: &str, space: Option<SpaceId>) {
        let current = match space {
            Some(id) => self
                .ed
                .model()
                .space(id)
                .map(|s| s.usage.clone())
                .unwrap_or_default(),
            None => self.space_settings.usage.clone(),
        };
        let mut options: Vec<(&str, &str)> = vec![("", "—")];
        options.extend(USAGES.iter().map(|u| (*u, *u)));
        if !current.is_empty() && !USAGES.contains(&current.as_str()) {
            options.push((current.as_str(), current.as_str()));
        }
        let picked = row(
            ui,
            "Uso",
            "El uso del local (IfcSpace.ObjectType)",
            |ui, w| choice(ui, salt, w, current.as_str(), &options).to_string(),
        );
        if picked == current {
            return;
        }
        match space {
            Some(id) => {
                let r = self.ed.edit_space("Uso del local", id, |s| {
                    if s.name == s.usage || s.name == "Local" {
                        s.name = picked.clone();
                    }
                    s.usage = picked.clone();
                });
                self.report(r, "Uso cambiado.");
            }
            None => {
                let s = &mut self.space_settings;
                if s.name == s.usage || s.name == "Local" {
                    s.name = picked.clone();
                }
                s.usage = picked;
            }
        }
    }

    pub(super) fn space_properties(&mut self, ui: &mut Ui, id: SpaceId) {
        let model = self.ed.model();
        let Some(s) = model.space(id).cloned() else {
            return;
        };
        let gross = archisim_geom::room_outline(model, s.level, s.label_point(), Contour::WallAxis)
            .map(|o| archisim_core::signed_area(&o).abs())
            .unwrap_or(s.area());
        props::title(ui, "Local · 1 seleccionado");
        super::properties::view_card_pub(
            ui,
            Icon::Room,
            &format!("{} {}", s.number, s.name),
            "IfcSpace",
        );
        group(ui, "space-data", "Datos de identidad", |ui| {
            row(ui, "Número", "Único en la planta", |ui, w| {
                if let Some(v) =
                    text_edit_committed(ui, Id::unique(("space-num", id)), &s.number, w - 8.0)
                {
                    let r = self.ed.edit_space("Número del local", id, |s| s.number = v);
                    self.report(r, "Número cambiado.");
                }
            });
            row(ui, "Nombre", "", |ui, w| {
                if let Some(v) =
                    text_edit_committed(ui, Id::unique(("space-name", id)), &s.name, w - 8.0)
                {
                    let r = self.ed.edit_space("Nombre del local", id, |s| s.name = v);
                    self.report(r, "Nombre cambiado.");
                }
            });
            self.usage_row(ui, &format!("space-usage-{}", id.0), Some(id));
            id_row(ui, "GlobalId", &s.guid.to_string());
        });
        group(ui, "space-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("space-level", id),
                "Planta",
                Some(s.level),
                &[Element::Space(id)],
            );
            row(ui, "Altura (mm)", "Altura libre del local", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("space-h", id)), s.height, w - 8.0) {
                    let r = self.ed.edit_space("Altura del local", id, |s| s.height = v);
                    self.report(r, "Altura cambiada.");
                }
            });
        });
        group(ui, "space-areas", "Superficies", |ui| {
            text_row(
                ui,
                "Superficie útil",
                "Hasta las caras interiores",
                &fmt_area(s.area()),
            );
            text_row(
                ui,
                "Superficie construida",
                "Hasta los ejes de los muros que lo rodean",
                &fmt_area(gross),
            );
            text_row(ui, "Perímetro (mm)", "", &fmt_mm(s.perimeter()));
            text_row(
                ui,
                "Volumen",
                "",
                &format!(
                    "{} m³",
                    format!("{:.2}", s.area() * s.height).replace('.', ",")
                ),
            );
            if s.label.is_some() && ui.button("Rótulo al centro").clicked() {
                let r = self
                    .ed
                    .edit_space("Mover rótulo de local", id, |s| s.label = None);
                self.report(r, "Rótulo centrado.");
            }
        });
    }

    /// A click of the room tool.
    pub(super) fn space_click(&mut self, p: Point2) {
        if self.zone_wand() {
            match self.ed.add_space_in_room(&self.space_settings, p) {
                Ok(id) => self.space_created(id),
                Err(e) => self.complain(format!("No se puede crear el local: {e}.")),
            }
            return;
        }
        match self.zone_tool.click(p) {
            PolyOutcome::Build(outline) => self.build_space(outline),
            PolyOutcome::Invalid => {
                self.complain("El contorno no puede cruzarse: corrige la última esquina.")
            }
            PolyOutcome::Nothing => {}
        }
    }

    pub(super) fn build_space(&mut self, outline: Vec<Point2>) {
        match self.ed.add_space(&self.space_settings, outline) {
            Ok(id) => {
                self.zone_tool.accept();
                self.space_created(id);
            }
            Err(e) => self.complain(format!("No se puede crear el local: {e}.")),
        }
    }

    fn space_created(&mut self, id: SpaceId) {
        let (num, name, area) = self
            .ed
            .model()
            .space(id)
            .map(|s| (s.number.clone(), s.name.clone(), s.area()))
            .unwrap_or_default();
        self.ed.selection = [Element::Space(id)].into_iter().collect();
        self.say(format!(
            "Local {num} «{name}» de {} útiles. Siguiente: clic en otro recinto.",
            fmt_area(area)
        ));
    }

    /// The outline being drawn with the room tool.
    pub(super) fn draw_space_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Zone, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.zone_tool.corners();
        if corners.is_empty() {
            return;
        }
        let ink = Stroke::new(1.5, ACCENT);
        if self.zone_tool.mode == SlabMode::Rectangle {
            if let Some(outline) = self.zone_tool.rectangle_to(c.point) {
                painter.add(Shape::closed_line(self.polygon(rect, &outline), ink));
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!("S {}", fmt_area(archisim_core::signed_area(&outline).abs())),
                );
            }
            return;
        }
        let mut pts = corners.to_vec();
        pts.push(self.zone_tool.typed_end(c.point).unwrap_or(c.point));
        let screen = self.polygon(rect, &pts);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("S {}", fmt_area(archisim_core::signed_area(&pts).abs())),
            );
        }
    }
}

/// Phases in the interface, with their Spanish names.
pub const PHASES: [(archisim_core::Phase, &str); 3] = [
    (archisim_core::Phase::New, "Nuevo"),
    (archisim_core::Phase::Existing, "Existente"),
    (archisim_core::Phase::Demolished, "A derribar"),
];

impl ArchiSimApp {
    /// The phase of the selected elements (reforms): new, existing or to
    /// be demolished.
    pub(super) fn phase_group(&mut self, ui: &mut Ui, salt: &str, items: &[Element]) {
        let model = self.ed.model();
        let phased: Vec<Element> = items
            .iter()
            .copied()
            .filter(|e| {
                matches!(
                    e,
                    Element::Wall(_)
                        | Element::Slab(_)
                        | Element::Opening(_)
                        | Element::Column(_)
                        | Element::Beam(_)
                )
            })
            .collect();
        if phased.is_empty() {
            return;
        }
        let id_of = |e: &Element| match e {
            Element::Wall(i) => i.0,
            Element::Slab(i) => i.0,
            Element::Opening(i) => i.0,
            Element::Column(i) => i.0,
            Element::Beam(i) => i.0,
            _ => 0,
        };
        let first = model.phase(id_of(&phased[0]));
        let current = phased
            .iter()
            .all(|e| model.phase(id_of(e)) == first)
            .then_some(first);
        group(ui, salt, "Fases (reforma)", |ui| {
            let mut opts: Vec<(Option<archisim_core::Phase>, &str)> =
                PHASES.iter().map(|(p, n)| (Some(*p), *n)).collect();
            if current.is_none() {
                opts.insert(0, (None, "Varias"));
            }
            let picked = row(
                ui,
                "Fase",
                "Nuevo: lo que construye el proyecto; existente: lo que hay y se conserva; a derribar: lo que hay y se demuele (sus puertas y ventanas con él)",
                |ui, w| choice(ui, (salt, "phase"), w, current, &opts),
            );
            if let (Some(p), true) = (picked, picked != current) {
                match self.ed.set_phase(&phased, p) {
                    Ok(n) => self.say(format!("{n} elemento(s) en fase «{}».", phase_name(p))),
                    Err(e) => self.complain(format!("No se puede cambiar la fase: {e}.")),
                }
            }
        });
    }

    /// Phases of the active plan: what it shows, and what new elements are
    /// drawn as.
    pub(super) fn phase_view_group(&mut self, ui: &mut Ui) {
        group(ui, "view-phases", "Fases (reforma)", |ui| {
            let opts: Vec<(archisim_plan::phases::PhaseView, &str)> =
                archisim_plan::phases::PhaseView::ALL
                    .iter()
                    .map(|v| (*v, v.name()))
                    .collect();
            row(
                ui,
                "Mostrar",
                "Todo; estado actual (lo existente y lo que se derriba); demoliciones (en amarillo); estado reformado (lo nuevo en rojo)",
                |ui, w| {
                    self.phase_view = choice(ui, "view-phase", w, self.phase_view, &opts);
                },
            );
            let draw = self.ed.draw_phase();
            let opts: Vec<(archisim_core::Phase, &str)> = PHASES.to_vec();
            let picked = row(
                ui,
                "Dibujar como",
                "Fase de lo que dibujes a partir de ahora (el levantamiento del estado actual, como existente)",
                |ui, w| choice(ui, "draw-phase", w, draw, &opts),
            );
            if picked != draw {
                self.ed.set_draw_phase(picked);
                self.say(format!(
                    "Lo que dibujes ahora será «{}».",
                    phase_name(picked)
                ));
            }
        });
    }
}

pub fn phase_name(p: archisim_core::Phase) -> &'static str {
    PHASES.iter().find(|(q, _)| *q == p).map_or("", |(_, n)| *n)
}
