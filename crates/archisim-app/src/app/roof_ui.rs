//! The roof tool ("Cubierta") and the properties of a selected roof: its
//! shape (four, two or one slope), pitch, overhang, height and the pitch
//! or gable of each side.

use archisim_core::{RoofId, SlabFunction};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, row, text_row};
use super::*;
use crate::editor::{RoofKind, RoofSettings};
use crate::slab_tool::Outcome as PolyOutcome;

/// Shapes offered, with their Spanish names.
const KINDS: [(RoofKind, &str); 3] = [
    (RoofKind::Hip, "A cuatro aguas"),
    (RoofKind::Gable, "A dos aguas"),
    (RoofKind::Shed, "A un agua"),
];

/// The roof tool's settings for a model: its first pitched roof type (or
/// any roof type, or any slab type), 30 %, 50 cm of eaves.
pub fn default_roof_settings(model: &archisim_core::Model) -> RoofSettings {
    let types: Vec<_> = model.slab_types().collect();
    let roof_type = types
        .iter()
        .find(|t| t.function == SlabFunction::Roof && t.name.to_lowercase().contains("inclinada"))
        .or_else(|| types.iter().find(|t| t.function == SlabFunction::Roof))
        .or(types.first())
        .map_or(archisim_core::SlabTypeId(0), |t| t.id);
    RoofSettings {
        roof_type,
        kind: RoofKind::Hip,
        slope: 0.3,
        overhang: 0.5,
        base_offset: 0.0,
        level_above: true,
        wand: true,
    }
}

/// The types a roof may take: those of roofs (all, if there is none).
fn roof_types(model: &archisim_core::Model) -> Vec<(archisim_core::SlabTypeId, String)> {
    let roofs: Vec<_> = model
        .slab_types()
        .filter(|t| t.function == SlabFunction::Roof)
        .map(|t| (t.id, t.name.clone()))
        .collect();
    if roofs.is_empty() {
        model.slab_types().map(|t| (t.id, t.name.clone())).collect()
    } else {
        roofs
    }
}

/// A pitch typed as a percentage ("30", "27,5 %").
fn parse_percent(text: &str) -> Option<f64> {
    let v = text
        .trim()
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()?;
    v.is_finite().then_some(v / 100.0)
}

/// A pitch as a percentage with a decimal comma.
fn percent(v: f64) -> String {
    let r = (v * 1000.0).round() / 10.0;
    let t = format!("{r:.1}");
    t.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

impl ArchiSimApp {
    /// Whether a click with the roof tool takes the building around it.
    pub(super) fn roof_wand(&self) -> bool {
        self.roof_settings.wand && !self.roof_tool.is_drawing()
    }

    pub(super) fn roof_click(&mut self, p: Point2) {
        if self.roof_wand() {
            match self.ed.add_roof_over(&self.roof_settings, p) {
                Ok(id) => self.roof_created(id),
                Err(e) => self.complain(format!("No se puede crear la cubierta: {e}.")),
            }
            return;
        }
        match self.roof_tool.click(p) {
            PolyOutcome::Build(outline) => self.build_roof(outline),
            PolyOutcome::Invalid => {
                self.complain("El contorno no puede cruzarse: corrige la última esquina.")
            }
            PolyOutcome::Nothing => {}
        }
    }

    pub(super) fn build_roof(&mut self, outline: Vec<Point2>) {
        match self.ed.add_roof(&self.roof_settings, outline) {
            Ok(id) => {
                self.roof_tool.accept();
                self.roof_created(id);
            }
            Err(e) => self.complain(format!("No se puede crear la cubierta: {e}.")),
        }
    }

    fn roof_created(&mut self, id: RoofId) {
        let (planes, top, level) = self
            .ed
            .model()
            .roof(id)
            .map(|r| {
                (
                    r.planes().len(),
                    r.top_range().1,
                    self.ed
                        .model()
                        .level(r.level)
                        .map(|l| l.name.clone())
                        .unwrap_or_default(),
                )
            })
            .unwrap_or_default();
        self.ed.selection = [Element::Roof(id)].into_iter().collect();
        self.say(format!(
            "Cubierta de {planes} faldones en «{level}», cumbrera a {} mm sobre la planta. Cambia la forma y cada lado en Propiedades.",
            fmt_mm(top)
        ));
    }

    /// The footprint being drawn (the building the wand would take is
    /// drawn by the wand preview).
    pub(super) fn draw_roof_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Roof, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.roof_tool.corners();
        if corners.is_empty() {
            return;
        }
        let ink = Stroke::new(1.5, ACCENT);
        if self.roof_tool.mode == SlabMode::Rectangle {
            if let Some(outline) = self.roof_tool.rectangle_to(c.point) {
                painter.add(Shape::closed_line(self.polygon(rect, &outline), ink));
            }
            return;
        }
        let mut pts = corners.to_vec();
        pts.push(self.roof_tool.typed_end(c.point).unwrap_or(c.point));
        let screen = self.polygon(rect, &pts);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
        }
    }

    pub(super) fn roof_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Cubierta · lo que vas a crear");
        let level = self.ed.roof_level(&self.roof_settings);
        let level_name = self
            .ed
            .model()
            .level(level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        group(ui, "tool-roof-method", "Método", |ui| {
            ui.add_space(2.0);
            #[derive(Clone, Copy, PartialEq)]
            enum Method {
                Wand,
                Polygon,
                Rectangle,
            }
            let current = match (self.roof_settings.wand, self.roof_tool.mode) {
                (true, _) => Method::Wand,
                (false, SlabMode::Rectangle) => Method::Rectangle,
                (false, SlabMode::Polygon) => Method::Polygon,
            };
            let mut method = current;
            if super::widgets::method_picker(
                ui,
                "roof-method",
                &mut method,
                &[
                    (
                        Method::Wand,
                        Icon::GeoWand,
                        "Edificio",
                        "Clic dentro del edificio: la cubierta por las caras exteriores de sus muros",
                    ),
                    (
                        Method::Polygon,
                        Icon::GeoPolygon,
                        "Polígono",
                        "Esquina a esquina (en L, T o U: limahoyas en las esquinas entrantes)",
                    ),
                    (
                        Method::Rectangle,
                        Icon::GeoRect,
                        "Rectángulo",
                        "Dos esquinas opuestas",
                    ),
                ],
            ) {
                self.roof_settings.wand = method == Method::Wand;
                match method {
                    Method::Polygon => self.roof_tool.set_mode(SlabMode::Polygon),
                    Method::Rectangle => self.roof_tool.set_mode(SlabMode::Rectangle),
                    Method::Wand => self.roof_tool.finish(),
                }
            }
        });
        group(ui, "tool-roof-shape", "Forma", |ui| {
            row(ui, "Forma", "Qué lados llevan faldón", |ui, w| {
                self.roof_settings.kind =
                    choice(ui, "tool-roof-kind", w, self.roof_settings.kind, &KINDS);
            });
            row(ui, "Pendiente (%)", "La de todos los faldones", |ui, w| {
                let shown = percent(self.roof_settings.slope);
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("tool-roof-slope"), &shown, w - 8.0)
                {
                    match parse_percent(&v) {
                        Some(s) if s > 0.0 => self.roof_settings.slope = s,
                        _ => {
                            self.complain(format!("«{v}» no es una pendiente: escribe 30 o 27,5."))
                        }
                    }
                }
            });
            row(ui, "Alero (mm)", "Vuelo de aleros y piñones", |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("tool-roof-overhang"),
                    self.roof_settings.overhang,
                    w - 8.0,
                ) {
                    self.roof_settings.overhang = v.max(0.0);
                }
            });
        });
        group(ui, "tool-roof-place", "Restricciones", |ui| {
            row(
                ui,
                "Planta de encima",
                "Dibujada sobre los muros de una planta, la cubierta pertenece a la de encima",
                |ui, _| {
                    ui.checkbox(&mut self.roof_settings.level_above, "");
                },
            );
            text_row(ui, "Planta", "", &level_name);
            row(
                ui,
                "Arranque (mm)",
                "Altura de la cara superior en el borde del contorno, sobre su planta",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-roof-base"),
                        self.roof_settings.base_offset,
                        w - 8.0,
                    ) {
                        self.roof_settings.base_offset = v;
                    }
                },
            );
        });
        let types = roof_types(self.ed.model());
        group(ui, "tool-roof-type", "Tipo", |ui| {
            row(ui, "Composición", "Capas, de arriba abajo", |ui, w| {
                let opts: Vec<_> = types.iter().map(|(id, n)| (*id, n.as_str())).collect();
                self.roof_settings.roof_type =
                    choice(ui, "tool-roof-type", w, self.roof_settings.roof_type, &opts);
            });
        });
    }

    pub(super) fn roof_properties(&mut self, ui: &mut Ui, id: RoofId) {
        let model = self.ed.model();
        let Some(r) = model.roof(id).cloned() else {
            return;
        };
        let type_name = model
            .slab_type(r.roof_type)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let thickness = model.slab_type(r.roof_type).map_or(0.0, |t| t.thickness());
        props::title(ui, "Cubierta · 1 seleccionada");
        super::properties::view_card_pub(ui, Icon::Roof, &type_name, "IfcRoof");
        let types = roof_types(model);
        props::note(
            ui,
            "En la planta, los puntos azules donde se juntan limatesas, limahoyas y cumbrera se arrastran: los faldones que llegan a ellos cambian de pendiente para pasar por el punto nuevo (geometrías especiales).",
        );
        group(ui, "roof-shape", "Forma", |ui| {
            let pitched: Vec<f64> = r.slopes.iter().flatten().copied().collect();
            let slope = pitched.first().copied().unwrap_or(0.3);
            let current = KINDS
                .iter()
                .map(|k| k.0)
                .find(|k| crate::editor::slopes_for(*k, &r.outline, slope) == r.slopes);
            row(ui, "Forma", "Qué lados llevan faldón", |ui, w| {
                let mut opts: Vec<(Option<RoofKind>, &str)> =
                    KINDS.iter().map(|(k, n)| (Some(*k), *n)).collect();
                if current.is_none() {
                    opts.push((None, "A medida"));
                }
                if let Some(k) = choice(ui, ("roof-kind", id.0), w, current, &opts) {
                    if Some(k) != current {
                        let res = self.ed.set_roof_kind(id, k, slope);
                        self.report(res, "Forma cambiada.");
                    }
                }
            });
            let same = pitched.iter().all(|s| (s - slope).abs() < 1e-9);
            row(ui, "Pendiente (%)", "La de todos los faldones", |ui, w| {
                let shown = if same {
                    percent(slope)
                } else {
                    "varias".to_string()
                };
                if let Some(v) =
                    text_edit_committed(ui, Id::unique(("roof-slope", id.0)), &shown, w - 8.0)
                {
                    match parse_percent(&v) {
                        Some(s) => {
                            let res = self.ed.edit_roof("Pendiente de la cubierta", id, |r| {
                                for x in r.slopes.iter_mut().flatten() {
                                    *x = s;
                                }
                            });
                            self.report(res, "Pendiente cambiada.");
                        }
                        None => {
                            self.complain(format!("«{v}» no es una pendiente: escribe 30 o 27,5."))
                        }
                    }
                }
            });
            row(ui, "Alero (mm)", "Vuelo de aleros y piñones", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("roof-overhang", id.0)), r.overhang, w - 8.0)
                {
                    let res = self
                        .ed
                        .edit_roof("Alero de la cubierta", id, |r| r.overhang = v);
                    self.report(res, "Alero cambiado.");
                }
            });
        });
        group(ui, "roof-sides", "Lados", |ui| {
            let n = r.outline.len();
            for i in 0..n {
                let len = r.outline[i].distance(r.outline[(i + 1) % n]);
                let label = format!("Lado {} · {} m", i + 1, fmt_m(len));
                row(
                    ui,
                    &label,
                    "Faldón con su pendiente, o piñón",
                    |ui, w| {
                        let shown = r.slopes[i].map_or("piñón".to_string(), percent);
                        if let Some(v) = text_edit_committed(
                            ui,
                            Id::unique(("roof-side", id.0, i)),
                            &shown,
                            w - 8.0,
                        ) {
                            let t = v.trim().to_lowercase();
                            let new = if t.is_empty() || t.starts_with('p') || t == "-" {
                                Some(None)
                            } else {
                                parse_percent(&t).map(Some)
                            };
                            match new {
                                Some(x) => {
                                    let res = self
                                        .ed
                                        .edit_roof("Lado de la cubierta", id, |r| r.slopes[i] = x);
                                    self.report(res, "Lado cambiado.");
                                }
                                None => self.complain(format!(
                                    "«{v}» no es una pendiente: escribe 30, o «piñón»."
                                )),
                            }
                        }
                    },
                );
            }
        });
        group(ui, "roof-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("roof-level", id),
                "Planta",
                Some(r.level),
                &[Element::Roof(id)],
            );
            row(
                ui,
                "Arranque (mm)",
                "Altura de la cara superior en el borde del contorno, sobre la planta",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("roof-base", id.0)), r.base_offset, w - 8.0)
                    {
                        let res = self
                            .ed
                            .edit_roof("Altura de la cubierta", id, |r| r.base_offset = v);
                        self.report(res, "Altura cambiada.");
                    }
                },
            );
        });
        group(ui, "roof-type", "Tipo", |ui| {
            row(ui, "Composición", "Capas, de arriba abajo", |ui, w| {
                let opts: Vec<_> = types.iter().map(|(t, n)| (*t, n.as_str())).collect();
                let t = choice(ui, ("roof-type", id.0), w, r.roof_type, &opts);
                if t != r.roof_type {
                    let res = self
                        .ed
                        .edit_roof("Tipo de cubierta", id, |r| r.roof_type = t);
                    self.report(res, "Tipo cambiado.");
                }
            });
            text_row(ui, "Canto (mm)", "Del tipo", &fmt_mm(thickness));
        });
        group(ui, "roof-quantities", "Superficies", |ui| {
            let (lo, hi) = r.top_range();
            text_row(
                ui,
                "Superficie inclinada",
                "Sobre la pendiente, aleros incluidos",
                &fmt_area(r.surface_area()),
            );
            text_row(
                ui,
                "En planta",
                "Proyección, aleros incluidos",
                &fmt_area(archisim_core::signed_area(&r.eaves()).abs()),
            );
            text_row(
                ui,
                "Cumbrera (mm)",
                "Punto más alto de la cara superior, sobre la planta",
                &fmt_mm(hi),
            );
            text_row(
                ui,
                "Alero (mm)",
                "Punto más bajo de la cara superior, sobre la planta",
                &fmt_mm(lo),
            );
        });
        group(ui, "roof-walls", "Muros", |ui| {
            let trimmed = self
                .ed
                .model()
                .walls()
                .filter(|w| self.ed.model().wall_roof(w.id) == Some(id))
                .count();
            text_row(
                ui,
                "Recortados debajo",
                "Muros que suben hasta la cara inferior de esta cubierta",
                &trimmed.to_string(),
            );
            if ui
                .button("Recortar los muros de debajo")
                .on_hover_text("Los muros rectos de esta planta y de la de debajo que quedan bajo la cubierta suben hasta ella: los piñones salen solos")
                .clicked()
            {
                match self.ed.trim_walls_under(id) {
                    Ok(0) => self.complain("No hay muros rectos bajo esta cubierta."),
                    Ok(n) => self.say(format!("{n} muros recortados bajo la cubierta.")),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
        });
        self.phase_group(ui, "roof-phase", &[Element::Roof(id)]);
        group(ui, "roof-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &r.guid.to_string());
        });
    }
}
