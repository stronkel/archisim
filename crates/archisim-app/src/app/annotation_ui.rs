//! The dimension tool in the window: its settings, the preview of the
//! dimension being drawn and the properties of dimensions and level marks.

use archisim_core::{Dimension, DimensionId, ElevationMarkId, Point2};
use archisim_plan::annotate::{dimension_lines, format_dimension, format_elevation};
use eframe::egui::{self, ComboBox, FontId, Rect, Shape, Stroke, Ui, Vec2};

use super::{fmt_level, ArchiSimApp, ACCENT};
use crate::dim_tool::{DimDirection, DimMode, Outcome};
use crate::units::{fmt_m, fmt_mm};

/// Paper millimetres in metres at 1:50, the scale of the plan view.
const MM: f64 = 0.05;

fn mode_name(m: DimMode) -> &'static str {
    match m {
        DimMode::Linear => "Lineal",
        DimMode::WallOpenings => "Huecos de muro",
        DimMode::LevelMark => "Cota de nivel",
    }
}

fn direction_name(d: DimDirection) -> &'static str {
    match d {
        DimDirection::Automatic => "Automática",
        DimDirection::Horizontal => "Horizontal",
        DimDirection::Vertical => "Vertical",
        DimDirection::Aligned => "Alineada",
    }
}

impl ArchiSimApp {
    pub(super) fn dim_settings_ui(&mut self, ui: &mut Ui) {
        ui.label("Método:");
        let before = self.dim_settings.mode;
        ComboBox::from_id_salt("tool-dim-mode")
            .selected_text(mode_name(self.dim_settings.mode))
            .show_ui(ui, |ui| {
                for (m, hint) in [
                    (
                        DimMode::Linear,
                        "Clic en cada punto, Intro, y clic donde va la línea de cota",
                    ),
                    (
                        DimMode::WallOpenings,
                        "Clic en un muro: su cara con las jambas de puertas y ventanas",
                    ),
                    (
                        DimMode::LevelMark,
                        "Clic: cota de nivel del suelo terminado en ese punto",
                    ),
                ] {
                    ui.selectable_value(&mut self.dim_settings.mode, m, mode_name(m))
                        .on_hover_text(hint);
                }
            });
        if self.dim_settings.mode != before {
            self.dim_tool.finish();
        }
        if self.dim_settings.mode == DimMode::Linear {
            ui.label("Dirección:");
            ComboBox::from_id_salt("tool-dim-direction")
                .selected_text(direction_name(self.dim_settings.direction))
                .show_ui(ui, |ui| {
                    for d in [
                        DimDirection::Automatic,
                        DimDirection::Horizontal,
                        DimDirection::Vertical,
                        DimDirection::Aligned,
                    ] {
                        ui.selectable_value(&mut self.dim_settings.direction, d, direction_name(d));
                    }
                });
        }
        ui.label(egui::RichText::new("en metros, 2 decimales").weak());
    }

    pub(super) fn dim_hint(&self) -> &'static str {
        match (self.dim_settings.mode, self.dim_tool.is_placing()) {
            (_, true) => "Clic: dónde va la línea de cota · Esc: volver a los puntos",
            (DimMode::Linear, false) if self.dim_tool.is_active() => {
                "Clic: siguiente punto · Intro, doble clic o botón derecho: colocar la línea · Esc: quitar el último punto"
            }
            (DimMode::Linear, false) => "Clic: primer punto a acotar (con forzados) · Esc: flecha",
            (DimMode::WallOpenings, false) => {
                "Clic sobre un muro, del lado de la cara que quieres acotar · Esc: flecha"
            }
            (DimMode::LevelMark, false) => "Clic: cota de nivel del suelo en ese punto · Esc: flecha",
        }
    }

    /// Acts on what a click of the dimension tool asked for.
    pub(super) fn dim_outcome(&mut self, out: Outcome) {
        match out {
            Outcome::Nothing => {}
            Outcome::NoWall => self.complain("Haz clic sobre un muro para acotar sus huecos."),
            Outcome::Mark(p) => match self.ed.add_elevation_mark(p) {
                Ok(_) => {
                    let v = self
                        .ed
                        .model()
                        .floor_elevation_at(self.ed.level, p)
                        .unwrap_or(0.0);
                    self.say(format!("Cota de nivel {} colocada.", format_elevation(v)));
                }
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
            Outcome::Dimension(spec) => {
                let n = spec.points.len();
                match self.ed.add_dimension(spec) {
                    Ok(_) if n > 2 => self.say(format!("Cota encadenada de {} tramos.", n - 1)),
                    Ok(_) => self.say("Cota colocada."),
                    Err(e) => self.complain(format!("No se puede acotar: {e}.")),
                }
            }
        }
    }

    /// The points chosen so far and, when placing, the dimension as it
    /// will be drawn.
    pub(super) fn draw_dim_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != crate::app::Tool::Dimension {
            return;
        }
        let Some(c) = self.cursor else {
            return;
        };
        let pts = self.dim_tool.points();
        for p in pts {
            let s = self.view.to_screen(rect, *p);
            painter.circle_stroke(s, 4.0, Stroke::new(1.5, ACCENT));
        }
        if !self.dim_tool.is_placing() {
            if let Some(last) = pts.last() {
                super::dashed(
                    painter,
                    self.view.to_screen(rect, *last),
                    self.view.to_screen(rect, c.point),
                    ACCENT,
                );
            }
            return;
        }
        let Some(spec) = self
            .dim_tool
            .build(self.ed.level, self.dim_settings, c.point)
        else {
            return;
        };
        let Some(direction) = spec.direction.normalized() else {
            return;
        };
        let d = Dimension {
            id: DimensionId(0),
            level: spec.level,
            points: spec.points,
            direction,
            position: spec.position,
        };
        let g = dimension_lines(&d, MM);
        let ink = Stroke::new(1.2, ACCENT);
        let seg = |a: Point2, b: Point2, s: Stroke| {
            painter.line_segment(
                [self.view.to_screen(rect, a), self.view.to_screen(rect, b)],
                s,
            );
        };
        seg(g.line.0, g.line.1, ink);
        for (a, b) in &g.extensions {
            seg(*a, *b, ink);
        }
        let u = archisim_plan::annotate::reading_direction(direction);
        let diag = (u + u.perp_left()) * (std::f64::consts::FRAC_1_SQRT_2 * 1.25 * MM);
        for t in &g.ticks {
            seg(*t - diag, *t + diag, Stroke::new(2.0, ACCENT));
        }
        for (text, at, dir) in &g.texts {
            let galley = painter.layout_no_wrap(text.clone(), FontId::monospace(12.0), ACCENT);
            let angle = -(dir.y.atan2(dir.x)) as f32;
            let size = galley.size();
            let (sin, cos) = angle.sin_cos();
            let off = Vec2::new(-size.x / 2.0, -size.y);
            let rotated = Vec2::new(off.x * cos - off.y * sin, off.x * sin + off.y * cos);
            let pos = self.view.to_screen(rect, *at) + rotated;
            painter.add(Shape::Text(
                egui::epaint::TextShape::new(pos, galley, ACCENT).with_angle(angle),
            ));
        }
    }

    pub(super) fn dimension_properties(&mut self, ui: &mut Ui, id: DimensionId) {
        let Some(d) = self.ed.model().dimension(id).cloned() else {
            return;
        };
        let values = d.values();
        ui.label(
            egui::RichText::new(if values.len() > 1 {
                "Cota encadenada"
            } else {
                "Cota lineal"
            })
            .strong(),
        );
        ui.label(egui::RichText::new("Anotación de la planta (no se exporta a IFC)").weak());
        ui.add_space(4.0);
        let u = d.direction;
        let dir = if u.y.abs() < 1e-9 {
            "Horizontal".to_string()
        } else if u.x.abs() < 1e-9 {
            "Vertical".to_string()
        } else {
            format!("Alineada {}", crate::units::fmt_angle(u.y.atan2(u.x)))
        };
        egui::Grid::new("dim-props")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Dirección");
                ui.label(dir);
                ui.end_row();
                ui.label("Tramos (m)");
                ui.label(
                    values
                        .iter()
                        .map(|v| format_dimension(*v))
                        .collect::<Vec<_>>()
                        .join(" · "),
                );
                ui.end_row();
                ui.label("Total (m)");
                ui.label(format_dimension(values.iter().sum()));
                ui.end_row();
                ui.label("Puntos");
                ui.label(d.points.len().to_string());
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Arrástrala para mover la línea de cota. No se actualiza al mover los muros (cotas asociativas: fase 3).",
            )
            .weak(),
        );
    }

    pub(super) fn mark_properties(&mut self, ui: &mut Ui, id: ElevationMarkId) {
        let model = self.ed.model();
        let Some(m) = model.elevation_mark(id).cloned() else {
            return;
        };
        let value = model.floor_elevation_at(m.level, m.point).unwrap_or(0.0);
        let level = model
            .level(m.level)
            .map(|l| format!("{} ({})", l.name, fmt_level(l.elevation)))
            .unwrap_or_default();
        ui.label(egui::RichText::new("Cota de nivel").strong());
        ui.label(
            egui::RichText::new(
                "Lee la cara superior del forjado más alto de la planta en ese punto (o la cota de la planta)",
            )
            .weak(),
        );
        ui.add_space(4.0);
        egui::Grid::new("mark-props")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Cota");
                ui.label(format!("{} m", format_elevation(value)));
                ui.end_row();
                ui.label("Planta");
                ui.label(level);
                ui.end_row();
                ui.label("Punto (m)");
                ui.label(format!("{}; {}", fmt_m(m.point.x), fmt_m(m.point.y)));
                ui.end_row();
                ui.label("Sobre la planta (mm)");
                let base = model.level(m.level).map_or(0.0, |l| l.elevation);
                ui.label(fmt_mm(value - base));
                ui.end_row();
            });
    }
}
