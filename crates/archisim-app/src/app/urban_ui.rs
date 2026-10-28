//! Urbanismo: the plot tool (a boundary by its corners, with its setback)
//! and the road tool (an axis by its points, with a cross-section), and
//! the properties of plots and roads.

use archisim_core::{
    MaterialId, Model, NewPlot, NewRoad, PlotId, Point2, RoadBand, RoadId, RoadSection,
};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// Which line of a road the clicks draw (ArchiCAD's reference line of a
/// wall): its axis, or the outer edge of one sidewalk, left or right as
/// seen walking the way it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoadReference {
    LeftEdge,
    #[default]
    Axis,
    RightEdge,
}

impl RoadReference {
    pub const ALL: [RoadReference; 3] = [
        RoadReference::LeftEdge,
        RoadReference::Axis,
        RoadReference::RightEdge,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RoadReference::LeftEdge => "Borde izquierdo",
            RoadReference::Axis => "Eje",
            RoadReference::RightEdge => "Borde derecho",
        }
    }

    /// The road's axis from the line drawn.
    pub fn axis(self, drawn: &[Point2], section: &RoadSection) -> Option<Vec<Point2>> {
        let w = section.width() / 2.0;
        match self {
            RoadReference::Axis => Some(drawn.to_vec()),
            // The road lies to the right of a left edge.
            RoadReference::LeftEdge => archisim_core::offset_path(drawn, -w),
            RoadReference::RightEdge => archisim_core::offset_path(drawn, w),
        }
    }
}

/// What the road tool draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadSettings {
    pub reference: RoadReference,
    pub section: RoadSection,
    pub top_offset: f64,
    pub pavement: MaterialId,
    pub kerb: MaterialId,
    pub sidewalk: MaterialId,
}

impl RoadSettings {
    pub fn new(model: &Model) -> Self {
        let by = |cats: &[&str]| {
            cats.iter()
                .find_map(|c| model.materials().find(|m| m.category == *c))
                .or_else(|| model.materials().next())
                .map_or(MaterialId(0), |m| m.id)
        };
        RoadSettings {
            reference: RoadReference::Axis,
            section: RoadSection::default(),
            top_offset: 0.0,
            pavement: by(&["asphalt", "concrete"]),
            kerb: by(&["stone", "concrete"]),
            sidewalk: by(&["ceramic", "stone", "concrete"]),
        }
    }
}

pub(super) fn material_combo(
    ui: &mut Ui,
    id: Id,
    model: &Model,
    value: MaterialId,
    width: f32,
) -> MaterialId {
    let mut v = value;
    let mut mats: Vec<(MaterialId, String)> =
        model.materials().map(|m| (m.id, m.name.clone())).collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    let shown = model
        .material(value)
        .map_or("—".to_string(), |m| m.name.clone());
    ComboBox::from_id_salt(id)
        .width(width - 8.0)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (mid, name) in &mats {
                ui.selectable_value(&mut v, *mid, name);
            }
        });
    v
}

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// The fields of a road section; returns the section changed, if it was.
fn section_rows(ui: &mut Ui, salt: u64, s: RoadSection) -> Option<RoadSection> {
    let mut out = s;
    let mut changed = false;
    let fields: [(&str, &str, f64); 6] = [
        ("Calzada (mm)", "Ancho total de los carriles", s.carriageway),
        (
            "Aparcamiento (mm)",
            "A cada lado; 0: sin aparcamiento",
            s.parking,
        ),
        ("Bordillo (mm)", "Ancho, a cada lado", s.kerb),
        ("Acera (mm)", "A cada lado", s.sidewalk),
        (
            "Altura del bordillo (mm)",
            "Lo que sube la acera sobre la calzada",
            s.kerb_height,
        ),
        ("Firme (mm)", "Espesor del pavimento de la calzada", s.depth),
    ];
    for (i, (name, hint, value)) in fields.into_iter().enumerate() {
        row(ui, name, hint, |ui, w| {
            if let Some(v) = length_field(ui, Id::unique(("road-sec", salt, i)), value, w - 8.0) {
                let v = v.max(0.0);
                match i {
                    0 => out.carriageway = v,
                    1 => out.parking = v,
                    2 => out.kerb = v,
                    3 => out.sidewalk = v,
                    4 => out.kerb_height = v,
                    _ => out.depth = v,
                }
                changed = true;
            }
        });
    }
    changed.then_some(out)
}

impl ArchiSimApp {
    // ---- Plot tool ----------------------------------------------------------

    pub(super) fn plot_click(&mut self, p: Point2) {
        match self.plot_tool.click(p) {
            SlabOutcome::Build(outline) => self.build_plot(outline),
            SlabOutcome::Invalid => {
                self.complain("La linde no puede cruzarse: corrige la última esquina.")
            }
            SlabOutcome::Nothing => {}
        }
    }

    pub(super) fn build_plot(&mut self, outline: Vec<Point2>) {
        let level = self.ed.level;
        let setback = self.plot_setback;
        let r = self.ed.transact("Crear parcela", |m| {
            m.add_plot(NewPlot {
                name: m.next_plot_name(),
                level,
                outline,
                setback,
                reference: String::new(),
            })
        });
        match r {
            Ok(id) => {
                self.plot_tool.accept();
                self.ed.selection = [Element::Plot(id)].into_iter().collect();
                let (name, area) = self
                    .ed
                    .model()
                    .plot(id)
                    .map(|p| (p.name.clone(), p.area()))
                    .unwrap_or_default();
                self.say(format!(
                    "{name}: {} m². La referencia catastral y el retranqueo, en Propiedades.",
                    comma(area, 2)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear la parcela: {e}.")),
        }
    }

    // ---- Road tool ----------------------------------------------------------

    pub(super) fn road_click(&mut self, p: Point2) {
        if self.road_points.last().is_some_and(|l| l.approx_eq(p)) {
            self.finish_road();
        } else {
            self.road_points.push(p);
        }
    }

    pub(super) fn finish_road(&mut self) {
        let drawn = std::mem::take(&mut self.road_points);
        if drawn.len() < 2 {
            return;
        }
        let level = self.ed.level;
        let s = self.road_settings;
        let Some(axis) = s.reference.axis(&drawn, &s.section) else {
            self.complain("No se puede crear el vial: el eje dibujado no tiene longitud.");
            return;
        };
        let r = self.ed.transact("Crear vial", |m| {
            m.add_road(NewRoad {
                name: m.next_road_name(),
                level,
                axis,
                section: s.section,
                top_offset: s.top_offset,
                pavement: s.pavement,
                kerb_material: s.kerb,
                sidewalk_material: s.sidewalk,
            })
        });
        match r {
            Ok(id) => {
                self.ed.selection = [Element::Road(id)].into_iter().collect();
                let (name, len) = self
                    .ed
                    .model()
                    .road(id)
                    .map(|r| (r.name.clone(), r.length()))
                    .unwrap_or_default();
                self.say(format!("{name}: {} m de eje.", comma(len, 2)));
            }
            Err(e) => self.complain(format!("No se puede crear el vial: {e}.")),
        }
    }

    // ---- Preview ------------------------------------------------------------

    pub(super) fn draw_urban_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        match self.tool {
            Tool::Plot => {
                let corners = self.plot_tool.corners();
                if corners.is_empty() {
                    return;
                }
                if self.plot_tool.mode == SlabMode::Rectangle {
                    if let Some(o) = self.plot_tool.rectangle_to(c.point) {
                        painter.add(Shape::closed_line(self.polygon(rect, &o), ink));
                        self.draw_label(
                            painter,
                            rect,
                            c.point,
                            format!("{} m²", comma(archisim_core::signed_area(&o).abs(), 2)),
                        );
                    }
                    return;
                }
                let mut pts = corners.to_vec();
                pts.push(self.plot_tool.typed_end(c.point).unwrap_or(c.point));
                let screen = self.polygon(rect, &pts);
                painter.add(Shape::line(screen.clone(), ink));
                if pts.len() >= 3 {
                    dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
                    self.draw_label(
                        painter,
                        rect,
                        c.point,
                        format!("{} m²", comma(archisim_core::signed_area(&pts).abs(), 2)),
                    );
                }
            }
            Tool::Road => {
                if self.road_points.is_empty() {
                    return;
                }
                let mut drawn = self.road_points.clone();
                drawn.push(c.point);
                let s = self.road_settings;
                // The whole road as it will be: each band filled with its
                // colour, the kerbs drawn, and the line clicked.
                if let Some(axis) = s.reference.axis(&drawn, &s.section) {
                    self.paint_road_bands(painter, rect, &axis, &s.section);
                }
                painter.add(Shape::line(self.polygon(rect, &drawn), ink));
                let len: f64 = drawn.windows(2).map(|x| x[0].distance(x[1])).sum();
                self.draw_label(painter, rect, c.point, format!("L {} m", comma(len, 2)));
            }
            _ => {}
        }
    }

    /// The bands of a road along `axis`, filled segment by segment (each
    /// piece is convex, the whole strip may not be).
    pub(super) fn paint_road_bands(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        axis: &[Point2],
        section: &RoadSection,
    ) {
        for (band, a, b) in section.band_offsets() {
            let (Some(near), Some(far)) = (
                archisim_core::offset_path(axis, a),
                archisim_core::offset_path(axis, b),
            ) else {
                continue;
            };
            let fill = match band {
                RoadBand::Carriageway => Color32::from_rgba_unmultiplied(90, 96, 104, 90),
                RoadBand::Parking => Color32::from_rgba_unmultiplied(140, 146, 154, 80),
                RoadBand::Kerb => Color32::from_rgba_unmultiplied(235, 235, 230, 200),
                RoadBand::Sidewalk => Color32::from_rgba_unmultiplied(214, 196, 160, 110),
            };
            for i in 0..near.len().saturating_sub(1) {
                let quad = [near[i], near[i + 1], far[i + 1], far[i]];
                painter.add(Shape::convex_polygon(
                    self.polygon(rect, &quad),
                    fill,
                    Stroke::new(0.8, ACCENT.gamma_multiply(0.5)),
                ));
            }
        }
    }

    pub(super) fn urban_hint(&self) -> &'static str {
        match self.tool {
            Tool::Plot if self.plot_tool.is_drawing() && self.plot_tool.mode == SlabMode::Rectangle => {
                "Clic: esquina opuesta · Esc: anular"
            }
            Tool::Plot if self.plot_tool.is_drawing() => {
                "Clic: siguiente esquina de la linde · Clic en la primera o Intro: cerrar · Esc: quitar la última"
            }
            Tool::Plot => "Clic: primera esquina de la linde · Esc: flecha",
            Tool::Road if !self.road_points.is_empty() => match self.road_settings.reference {
                RoadReference::Axis => "Clic: siguiente punto del eje · Doble clic o Intro: terminar · Esc: quitar el último",
                _ => "Clic: siguiente punto del borde · Doble clic o Intro: terminar · Esc: quitar el último",
            },
            Tool::Road => match self.road_settings.reference {
                RoadReference::Axis => "Clic: comienzo del eje del vial (eje o borde, en la barra de arriba; la sección, en Propiedades) · Esc: flecha",
                _ => "Clic: comienzo del borde de la acera (eje o borde, en la barra de arriba; la sección, en Propiedades) · Esc: flecha",
            },
            _ => "",
        }
    }

    // ---- Properties ---------------------------------------------------------

    pub(super) fn plot_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Parcela · lo que vas a dibujar");
        group(ui, "tool-plot", "Parcela", |ui| {
            row(
                ui,
                "Retranqueo (mm)",
                "Lo que el edificio se separa de la linde",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-plot-setback"),
                        self.plot_setback,
                        w - 8.0,
                    ) {
                        self.plot_setback = v.max(0.0);
                    }
                },
            );
            props::note(
                ui,
                "La superficie de las parcelas es la que usan las masas de Concepto para la edificabilidad y la ocupación.",
            );
        });
    }

    pub(super) fn road_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Vial · lo que vas a dibujar");
        let model = self.ed.model();
        let mut s = self.road_settings;
        group(ui, "tool-road-ref", "Dibujo", |ui| {
            text_row(ui, "Línea dibujada", "", s.reference.name());
            props::note(
                ui,
                "Eje o bordes de las aceras: se elige en «Referencia», en la barra bajo las pestañas.",
            );
        });
        group(ui, "tool-road", "Sección tipo", |ui| {
            if let Some(sec) = section_rows(ui, 0, s.section) {
                s.section = sec;
            }
            text_row(ui, "Ancho total (mm)", "", &fmt_mm(s.section.width()));
            row(ui, "Rasante (mm)", "La calzada sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique("tool-road-top"), s.top_offset, w - 8.0)
                {
                    s.top_offset = v;
                }
            });
        });
        group(ui, "tool-road-mat", "Materiales", |ui| {
            row(ui, "Calzada", "", |ui, w| {
                s.pavement = material_combo(ui, Id::unique("tool-road-pav"), model, s.pavement, w);
            });
            row(ui, "Bordillo", "", |ui, w| {
                s.kerb = material_combo(ui, Id::unique("tool-road-kerb"), model, s.kerb, w);
            });
            row(ui, "Acera", "", |ui, w| {
                s.sidewalk = material_combo(ui, Id::unique("tool-road-side"), model, s.sidewalk, w);
            });
        });
        self.road_settings = s;
    }

    pub(super) fn plot_properties(&mut self, ui: &mut Ui, id: PlotId) {
        let Some(p) = self.ed.model().plot(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionada", p.name));
        group(ui, "plot-data", "Parcela", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("plot-name", id)), &p.name, w - 8.0)
                {
                    let r = self.ed.transact("Nombre de la parcela", |m| {
                        m.edit_plot(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(r, "Nombre cambiado.");
                }
            });
            row(ui, "Referencia catastral", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("plot-ref", id)), &p.reference, w - 8.0)
                {
                    let r = self.ed.transact("Referencia catastral", |m| {
                        m.edit_plot(id, |x| x.reference = t.trim().to_string())
                    });
                    self.report(r, "Referencia cambiada.");
                }
            });
            row(
                ui,
                "Retranqueo (mm)",
                "Lo que el edificio se separa de la linde",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("plot-setback", id)), p.setback, w - 8.0)
                    {
                        let r = self.ed.transact("Retranqueo", |m| {
                            m.edit_plot(id, |x| x.setback = v.max(0.0))
                        });
                        self.report(r, "Retranqueo cambiado.");
                    }
                },
            );
        });
        group(ui, "plot-figures", "Superficies", |ui| {
            text_row(ui, "Superficie (m²)", "", &comma(p.area(), 2));
            text_row(ui, "Perímetro (m)", "", &comma(p.perimeter(), 2));
            text_row(
                ui,
                "Área edificable (m²)",
                "Dentro del retranqueo",
                &p.buildable()
                    .map_or("— (el retranqueo no cabe)".into(), |b| {
                        comma(archisim_core::signed_area(&b).abs(), 2)
                    }),
            );
            let built: f64 = self
                .ed
                .model()
                .masses()
                .filter(|m| {
                    self.ed
                        .model()
                        .plot_under(&m.outline())
                        .is_some_and(|x| x.id == id)
                })
                .map(|m| m.figures(None).built)
                .sum();
            if built > 0.0 {
                text_row(
                    ui,
                    "Edificabilidad de sus masas",
                    "Lo construido en sus masas de Concepto entre su superficie",
                    &format!("{} m²/m²", comma(built / p.area(), 2)),
                );
            }
        });
        group(ui, "plot-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcSite (dentro del emplazamiento)");
        });
    }

    pub(super) fn road_properties(&mut self, ui: &mut Ui, id: RoadId) {
        let model = self.ed.model();
        let Some(r) = model.road(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionado", r.name));
        group(ui, "road-data", "Vial", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("road-name", id)), &r.name, w - 8.0)
                {
                    let res = self.ed.transact("Nombre del vial", |m| {
                        m.edit_road(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(res, "Nombre cambiado.");
                }
            });
            self.level_row(
                ui,
                ("road-level", id),
                "Planta",
                Some(r.level),
                &[Element::Road(id)],
            );
            row(ui, "Rasante (mm)", "La calzada sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("road-top", id)), r.top_offset, w - 8.0)
                {
                    let res = self.ed.transact("Rasante del vial", |m| {
                        m.edit_road(id, |x| x.top_offset = v)
                    });
                    self.report(res, "Rasante cambiada.");
                }
            });
        });
        group(ui, "road-section", "Sección tipo", |ui| {
            if let Some(sec) = section_rows(ui, id.0, r.section) {
                let res = self.ed.transact("Sección del vial", |m| {
                    m.edit_road(id, |x| x.section = sec)
                });
                self.report(res, "Sección cambiada.");
            }
        });
        group(ui, "road-figures", "Mediciones", |ui| {
            text_row(ui, "Longitud del eje (m)", "", &comma(r.length(), 2));
            for band in [RoadBand::Carriageway, RoadBand::Parking, RoadBand::Sidewalk] {
                let a = r.area_of(band);
                if a > 0.0 {
                    text_row(ui, &format!("{} (m²)", band.name()), "", &comma(a, 2));
                }
            }
        });
        let (p, k, s) = (r.pavement, r.kerb_material, r.sidewalk_material);
        group(ui, "road-mat", "Materiales", |ui| {
            for (i, (name, v)) in [("Calzada", p), ("Bordillo", k), ("Acera", s)]
                .into_iter()
                .enumerate()
            {
                row(ui, name, "", |ui, w| {
                    let n =
                        material_combo(ui, Id::unique(("road-mat", id, i)), self.ed.model(), v, w);
                    if n != v {
                        let res = self.ed.transact("Material del vial", |m| {
                            m.edit_road(id, |x| match i {
                                0 => x.pavement = n,
                                1 => x.kerb_material = n,
                                _ => x.sidewalk_material = n,
                            })
                        });
                        self.report(res, "Material cambiado.");
                    }
                });
            }
        });
        group(ui, "road-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                "IfcRoad con IfcPavement e IfcKerb (IFC 4.3)",
            );
        });
    }

    /// Whether a mass keeps the setback of the plot it is on: «Dentro»,
    /// «Se sale del retranqueo», or nothing off any plot.
    pub(super) fn mass_setback(&self, outline: &[Point2]) -> Option<&'static str> {
        let model = self.ed.model();
        let plot = model.plot_under(outline)?;
        let b = plot.buildable()?;
        Some(
            if outline.iter().all(|p| {
                crate::pick::contains(&b, *p) || crate::pick::distance_to_outline(&b, *p) < 1e-6
            }) {
                "Dentro del retranqueo"
            } else {
                "Se sale del retranqueo"
            },
        )
    }
}
