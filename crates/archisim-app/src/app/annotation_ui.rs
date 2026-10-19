//! The dimension tool in the window: its settings, the preview of the
//! dimension being drawn and the properties of dimensions and level marks.

use archisim_core::{Dimension, DimensionId, Point2};
use archisim_plan::annotate::{dimension_lines, format_elevation};
use eframe::egui::{self, FontId, Rect, Shape, Stroke, Vec2};

use super::{ArchiSimApp, ACCENT};
use crate::dim_tool::{DimMode, Outcome};

impl ArchiSimApp {
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
        // The points the element under the cursor offers to measure to,
        // so it is clear where the dimension can anchor.
        if !self.dim_tool.is_placing() {
            let reach = self.view.to_metres(super::REACH);
            if let Some(e) = crate::pick::pick(&self.ed, self.ed.level, c.point, reach) {
                if let Some(poly) = crate::pick::outline(&self.ed, e) {
                    for q in poly {
                        let s = self.view.to_screen(rect, q);
                        painter.circle_filled(s, 2.5, ACCENT.gamma_multiply(0.7));
                    }
                }
            }
        }
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
            anchors: Vec::new(),
        };
        let mm = self.plan_scale / 1000.0;
        let g = dimension_lines(&d, mm);
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
        let diag = (u + u.perp_left()) * (std::f64::consts::FRAC_1_SQRT_2 * 1.25 * mm);
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
}
