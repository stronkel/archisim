//! The curtain wall tool ("Muro cortina"): two clicks give its line (the
//! outside to the left, as walls); its grid, frames and panels are set in
//! Properties.

use archisim_core::CurtainWallId;
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, group, row, text_row};
use super::*;

impl ArchiSimApp {
    pub(super) fn curtain_click(&mut self, p: Point2) {
        match self.curtain_anchor {
            None => self.curtain_anchor = Some(p),
            Some(a) if a.approx_eq(p) => {}
            Some(a) => {
                self.curtain_anchor = None;
                match self.ed.add_curtain_wall(&self.curtain_settings, a, p) {
                    Ok(id) => {
                        self.ed.selection = [Element::CurtainWall(id)].into_iter().collect();
                        self.say("Muro cortina creado: cambia su modulación en Propiedades.");
                    }
                    Err(e) => self.complain(format!("No se puede crear el muro cortina: {e}.")),
                }
            }
        }
    }

    /// The line being drawn, with its mullions.
    pub(super) fn draw_curtain_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::CurtainWall, Some(c), Some(a)) = (self.tool, self.cursor, self.curtain_anchor)
        else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        let (pa, pb) = (
            self.view.to_screen(rect, a),
            self.view.to_screen(rect, c.point),
        );
        painter.line_segment([pa, pb], ink);
        let len = a.distance(c.point);
        let s = &self.curtain_settings;
        let n = (len / s.bay - 1e-9).ceil().max(1.0) as usize;
        if let Some(u) = (c.point - a).normalized() {
            let nrm = u.perp_left() * (s.frame_depth / 2.0);
            for i in 0..=n {
                let p = a + u * (len * i as f64 / n as f64);
                painter.line_segment(
                    [
                        self.view.to_screen(rect, p - nrm),
                        self.view.to_screen(rect, p + nrm),
                    ],
                    Stroke::new(3.0, ACCENT),
                );
            }
        }
        self.draw_label(
            painter,
            rect,
            c.point,
            format!("Muro cortina   L {} mm   {n} módulos", fmt_mm(len)),
        );
    }

    fn curtain_length_row(ui: &mut Ui, label: &str, hint: &str, id: Id, value: f64) -> Option<f64> {
        let mut out = None;
        row(ui, label, hint, |ui, w| {
            out = length_field(ui, id, value, w - 8.0);
        });
        out
    }

    pub(super) fn curtain_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Muro cortina · lo que vas a dibujar");
        group(ui, "tool-curtain", "Muro cortina", |ui| {
            let s = &mut self.curtain_settings;
            let fields: [(&str, &str, &mut f64); 7] = [
                ("Altura (mm)", "Desde su base", &mut s.height),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    &mut s.base_offset,
                ),
                (
                    "Módulo máx. (mm)",
                    "Distancia mayor entre montantes",
                    &mut s.bay,
                ),
                (
                    "Fila máx. (mm)",
                    "Distancia mayor entre travesaños",
                    &mut s.row,
                ),
                (
                    "Ancho perfil (mm)",
                    "Montantes y travesaños vistos de frente",
                    &mut s.frame_width,
                ),
                (
                    "Fondo perfil (mm)",
                    "Montantes y travesaños de dentro afuera",
                    &mut s.frame_depth,
                ),
                ("Vidrio (mm)", "Espesor del panel", &mut s.panel_thickness),
            ];
            for (i, (label, hint, v)) in fields.into_iter().enumerate() {
                if let Some(x) =
                    Self::curtain_length_row(ui, label, hint, Id::unique(("tool-curtain", i)), *v)
                {
                    *v = x;
                }
            }
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Dos clics: inicio y final. El exterior queda a la izquierda del trazo, como en los muros. Los módulos se reparten iguales, sin pasar del máximo.",
        );
    }

    pub(super) fn curtain_wall_properties(&mut self, ui: &mut Ui, id: CurtainWallId) {
        let Some(c) = self.ed.model().curtain_wall(id).cloned() else {
            return;
        };
        props::title(ui, "Muro cortina · 1 seleccionado");
        super::properties::view_card_pub(
            ui,
            Icon::CurtainWall,
            &format!("Muro cortina de {} m", fmt_m(c.length())),
            "IfcCurtainWall",
        );
        let g = c.grid();
        group(ui, "curtain-data", "Muro cortina", |ui| {
            type Set = fn(&mut archisim_core::CurtainWall, f64);
            let fields: [(&str, &str, f64, Set); 7] = [
                ("Altura (mm)", "Desde su base", c.height, |c, v| {
                    c.height = v;
                    c.top = archisim_core::WallTop::Unconnected;
                }),
                (
                    "Desfase base (mm)",
                    "Respecto a la planta",
                    c.base_offset,
                    |c, v| c.base_offset = v,
                ),
                (
                    "Módulo máx. (mm)",
                    "Distancia mayor entre montantes",
                    c.bay,
                    |c, v| c.bay = v,
                ),
                (
                    "Fila máx. (mm)",
                    "Distancia mayor entre travesaños",
                    c.row,
                    |c, v| c.row = v,
                ),
                (
                    "Ancho perfil (mm)",
                    "Visto de frente",
                    c.frame_width,
                    |c, v| c.frame_width = v,
                ),
                (
                    "Fondo perfil (mm)",
                    "De dentro afuera",
                    c.frame_depth,
                    |c, v| c.frame_depth = v,
                ),
                (
                    "Vidrio (mm)",
                    "Espesor del panel",
                    c.panel_thickness,
                    |c, v| c.panel_thickness = v,
                ),
            ];
            for (i, (label, hint, value, set)) in fields.into_iter().enumerate() {
                if let Some(v) = Self::curtain_length_row(
                    ui,
                    label,
                    hint,
                    Id::unique(("curtain", id.0, i)),
                    value,
                ) {
                    let r = self.ed.edit_curtain_wall(label, id, |c| set(c, v));
                    self.report(r, "Muro cortina cambiado.");
                }
            }
            text_row(
                ui,
                "Retícula",
                "Módulos × filas",
                &format!(
                    "{} × {}  ({} × {} mm)",
                    g.mullions.len() - 1,
                    g.transoms.len() - 1,
                    fmt_mm(g.mullions[1] - g.mullions[0]),
                    fmt_mm(g.transoms[1] - g.transoms[0]),
                ),
            );
        });
        group(ui, "curtain-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &c.guid.to_string());
        });
    }
}
