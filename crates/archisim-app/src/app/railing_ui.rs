//! The railing tool ("Barandilla"): its line clicked point by point, for
//! terraces, flat roofs and voids; height, foot and posts in Properties.

use archisim_core::RailingId;
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;

impl ArchiSimApp {
    pub(super) fn railing_click(&mut self, p: Point2) {
        if self.railing_points.last().is_some_and(|q| q.approx_eq(p)) {
            return;
        }
        self.railing_points.push(p);
    }

    pub(super) fn finish_railing(&mut self) {
        let path = std::mem::take(&mut self.railing_points);
        if path.len() < 2 {
            self.railing_points = path;
            return;
        }
        match self.ed.add_railing(&self.railing_settings, path) {
            Ok(id) => {
                self.ed.selection = [Element::Railing(id)].into_iter().collect();
                self.say(
                    "Barandilla creada: montantes, pasamanos y barrotes a menos de 10 cm (CTE).",
                );
            }
            Err(e) => self.complain(format!("No se puede crear la barandilla: {e}.")),
        }
    }

    pub(super) fn draw_railing_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Railing || self.railing_points.is_empty() {
            return;
        }
        let mut pts = self.railing_points.clone();
        if let Some(c) = self.cursor {
            pts.push(c.point);
        }
        let screen: Vec<Pos2> = pts.iter().map(|p| self.view.to_screen(rect, *p)).collect();
        painter.add(Shape::line(screen, Stroke::new(2.0, ACCENT)));
        if let (Some(c), Some(a)) = (self.cursor, self.railing_points.last()) {
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("Barandilla   L {} mm", fmt_mm(a.distance(c.point))),
            );
        }
    }

    pub(super) fn railing_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Barandilla · lo que vas a dibujar");
        group(ui, "tool-railing", "Barandilla", |ui| {
            let s = &mut self.railing_settings;
            let fields: [(&str, &str, &mut f64); 3] = [
                (
                    "Altura (mm)",
                    "Del pie al pasamanos (CTE: 900 o 1100)",
                    &mut s.height,
                ),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    &mut s.base_offset,
                ),
                (
                    "Montantes cada (mm)",
                    "Distancia mayor entre montantes",
                    &mut s.post_spacing,
                ),
            ];
            for (i, (label, hint, v)) in fields.into_iter().enumerate() {
                row(ui, label, hint, |ui, w| {
                    if let Some(x) = length_field(ui, Id::unique(("tool-railing", i)), *v, w - 8.0)
                    {
                        *v = x;
                    }
                });
            }
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Clic en cada punto de la barandilla; Intro, doble clic o clic derecho la termina; Esc quita el último punto. Los barrotes se reparten sin dejar más de 10 cm entre ellos.",
        );
    }

    pub(super) fn railing_properties(&mut self, ui: &mut Ui, id: RailingId) {
        let Some(r) = self.ed.model().railing(id).cloned() else {
            return;
        };
        props::title(ui, "Barandilla · 1 seleccionada");
        super::properties::view_card_pub(
            ui,
            Icon::Railing,
            &format!("Barandilla de {} m", fmt_m(r.length())),
            "IfcRailing",
        );
        group(ui, "railing-data", "Barandilla", |ui| {
            type Set = fn(&mut archisim_core::Railing, f64);
            let fields: [(&str, &str, f64, Set); 3] = [
                ("Altura (mm)", "Del pie al pasamanos", r.height, |r, v| {
                    r.height = v
                }),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    r.base_offset,
                    |r, v| r.base_offset = v,
                ),
                (
                    "Montantes cada (mm)",
                    "Distancia mayor",
                    r.post_spacing,
                    |r, v| r.post_spacing = v,
                ),
            ];
            for (i, (label, hint, value, set)) in fields.into_iter().enumerate() {
                row(ui, label, hint, |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("railing", id.0, i)), value, w - 8.0)
                    {
                        let r = self.ed.edit_railing(label, id, |r| set(r, v));
                        self.report(r, "Barandilla cambiada.");
                    }
                });
            }
            let posts = r
                .pieces()
                .iter()
                .filter(|p| p.kind == archisim_core::RailingPieceKind::Post)
                .count();
            text_row(ui, "Montantes", "Contando las esquinas", &posts.to_string());
        });
        group(ui, "railing-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &r.guid.to_string());
        });
    }
}
