//! The terrain tool ("Terreno"): survey points clicked one by one with
//! their height; the properties of a terrain (its points, contour lines,
//! depth); and the project's true north.

use archisim_core::{SurveyPoint, TerrainId};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;

/// A height as on a topographic plan: "+1,25".
fn height(z: f64) -> String {
    let s = format!("{:.2}", z.abs()).replace('.', ",");
    if z < -0.0005 {
        format!("-{s}")
    } else {
        format!("+{s}")
    }
}

impl ArchiSimApp {
    pub(super) fn terrain_click(&mut self, p: Point2) {
        if self.terrain_points.iter().any(|q| q.at.approx_eq(p)) {
            return;
        }
        self.terrain_points.push(SurveyPoint {
            at: p,
            z: self.terrain_settings.z,
        });
        let n = self.terrain_points.len();
        self.say(format!(
            "{n} puntos. Cambia la cota en Propiedades antes de cada clic · Intro o doble clic: crear el terreno."
        ));
    }

    pub(super) fn finish_terrain(&mut self) {
        let points = std::mem::take(&mut self.terrain_points);
        if points.len() < 3 {
            self.complain("El terreno necesita al menos tres puntos.");
            self.terrain_points = points;
            return;
        }
        match self.ed.add_terrain(&self.terrain_settings, points) {
            Ok(id) => {
                self.ed.selection = [Element::Terrain(id)].into_iter().collect();
                self.say("Terreno creado: sus curvas de nivel en planta; cambia las cotas en Propiedades.");
            }
            Err(e) => self.complain(format!("No se puede crear el terreno: {e}.")),
        }
    }

    pub(super) fn draw_terrain_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Terrain {
            return;
        }
        let ink = Stroke::new(1.0, ACCENT);
        let pts: Vec<Point2> = self.terrain_points.iter().map(|p| p.at).collect();
        for [a, b, c] in archisim_core::delaunay(&pts) {
            let tri = [pts[a], pts[b], pts[c]];
            painter.add(Shape::closed_line(self.polygon(rect, &tri), ink));
        }
        for p in &self.terrain_points {
            let s = self.view.to_screen(rect, p.at);
            painter.circle_filled(s, 3.0, ACCENT);
            painter.text(
                s + Vec2::new(5.0, -5.0),
                Align2::LEFT_BOTTOM,
                height(p.z),
                FontId::proportional(11.0),
                ACCENT,
            );
        }
        if let Some(c) = self.cursor {
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("Punto a {}", height(self.terrain_settings.z)),
            );
        }
    }

    pub(super) fn terrain_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Terreno · lo que vas a crear");
        group(ui, "tool-terrain", "Terreno", |ui| {
            row(
                ui,
                "Cota del punto (mm)",
                "Del próximo punto, respecto al ±0,00 del proyecto",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-z"),
                        self.terrain_settings.z,
                        w - 8.0,
                    ) {
                        self.terrain_settings.z = v;
                    }
                },
            );
            row(
                ui,
                "Equidistancia (mm)",
                "Entre curvas de nivel",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-contour"),
                        self.terrain_settings.contour,
                        w - 8.0,
                    ) {
                        self.terrain_settings.contour = v.max(0.0);
                    }
                },
            );
            row(
                ui,
                "Profundidad (mm)",
                "Del terreno bajo su punto más bajo, para las secciones",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-depth"),
                        self.terrain_settings.depth,
                        w - 8.0,
                    ) {
                        self.terrain_settings.depth = v;
                    }
                },
            );
            text_row(
                ui,
                "Puntos",
                "Marcados hasta ahora",
                &self.terrain_points.len().to_string(),
            );
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Marca los puntos del levantamiento: escribe la cota y haz clic en su sitio, uno tras otro. Intro o doble clic crea el terreno (tres puntos como mínimo); Esc quita el último.",
        );
    }

    pub(super) fn terrain_properties(&mut self, ui: &mut Ui, id: TerrainId) {
        let Some(t) = self.ed.model().terrain(id).cloned() else {
            return;
        };
        props::title(ui, "Terreno · 1 seleccionado");
        super::properties::view_card_pub(
            ui,
            Icon::Terrain,
            &format!("Terreno de {} puntos", t.points.len()),
            "IfcGeographicElement",
        );
        let (lo, hi) = t.range();
        group(ui, "terrain-data", "Terreno", |ui| {
            row(
                ui,
                "Equidistancia (mm)",
                "Entre curvas de nivel",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("terrain-contour", id.0)),
                        t.contour,
                        w - 8.0,
                    ) {
                        let r = self.ed.edit_terrain("Equidistancia", id, |t| t.contour = v);
                        self.report(r, "Equidistancia cambiada.");
                    }
                },
            );
            row(
                ui,
                "Profundidad (mm)",
                "Bajo su punto más bajo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("terrain-depth", id.0)), t.depth, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_terrain("Profundidad del terreno", id, |t| t.depth = v);
                        self.report(r, "Profundidad cambiada.");
                    }
                },
            );
            text_row(
                ui,
                "Cotas",
                "Más baja y más alta",
                &format!("{} … {}", height(lo), height(hi)),
            );
        });
        group(ui, "terrain-points", "Puntos", |ui| {
            for (i, p) in t.points.iter().enumerate().take(60) {
                let label = format!("{} · {}, {}", i + 1, fmt_m(p.at.x), fmt_m(p.at.y));
                row(ui, &label, "Cota del punto (mm)", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("terrain-z", id.0, i)), p.z, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_terrain("Cota de punto", id, |t| t.points[i].z = v);
                        self.report(r, "Cota cambiada.");
                    }
                });
            }
            if t.points.len() > 60 {
                props::note(ui, "Solo se muestran los 60 primeros puntos.");
            }
        });
        group(ui, "terrain-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &t.guid.to_string());
        });
    }

    /// The true north in the project's properties.
    pub(super) fn north_row(&mut self, ui: &mut Ui) {
        let north = self.ed.model().true_north().to_degrees();
        row(
            ui,
            "Norte (°)",
            "Ángulo del norte geográfico respecto al arriba de la planta, en sentido horario",
            |ui, w| {
                let shown = format!("{:.1}", north).replace('.', ",");
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("project-north"), &shown, w - 8.0)
                {
                    match v
                        .trim()
                        .trim_end_matches('°')
                        .replace(',', ".")
                        .parse::<f64>()
                    {
                        Ok(d) if d.is_finite() => {
                            let r = self.ed.set_true_north(d);
                            self.report(
                                r,
                                "Norte cambiado: se ve en las plantas exportadas y en el IFC.",
                            );
                        }
                        _ => self.complain(format!("«{v}» no es un ángulo: escribe 12 o -7,5.")),
                    }
                }
            },
        );
    }
}
