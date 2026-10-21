//! The insertion methods every drawing tool shares in the contextual bar
//! (`docs/INTERFAZ.md` §2.6): **Spline** (a free curve through clicked
//! points), **Elegir línea** (an existing line of the 2D drawing or of an
//! imported DXF) and the **length and angle** fields, filled as the mouse
//! moves and typed to place the next point exactly.

use archisim_core::{FootingKind, Point2};
use eframe::egui::{self, Id, Ui};

use super::*;
use crate::spline::{catmull_rom, picked_line, spline_arcs, PickedLine};
use crate::wall_tool::Piece;

/// How the tool in use takes its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawMethod {
    /// The tool's own (straight, chain, rectangle…).
    #[default]
    Normal,
    /// Clicks are the control points of a free curve.
    Spline,
    /// A click on a line of the 2D drawing.
    PickLine,
}

/// Control points per span when a curve becomes points.
const PER_SPAN: usize = 8;

impl ArchiSimApp {
    /// Whether the tool in use draws a free curve, and whether the curve
    /// closes (an outline) or not (a path).
    pub(super) fn spline_closes(&self) -> Option<bool> {
        match self.tool {
            Tool::Wall | Tool::Railing | Tool::Road | Tool::Pipe | Tool::Duct => Some(false),
            // Spline makes an open line (a path, or loose edges); it closes
            // on its first point.
            Tool::Mass => Some(false),
            Tool::Slab if !self.slab_settings.wand => Some(true),
            Tool::Zone | Tool::Plot => Some(true),
            _ => None,
        }
    }

    /// Whether the tool in use can follow a line of the drawing.
    pub(super) fn picks_lines(&self) -> bool {
        self.spline_closes().is_some()
            || matches!(
                self.tool,
                Tool::Beam | Tool::CurtainWall | Tool::Footing | Tool::Grid | Tool::Roof
            )
    }

    pub(super) fn set_draw_method(&mut self, m: DrawMethod) {
        self.draw_method = if self.draw_method == m {
            DrawMethod::Normal
        } else {
            m
        };
        self.spline_points.clear();
        match self.draw_method {
            DrawMethod::Spline => self.say(
                "Spline: clic en cada punto de la curva · Doble clic, Intro o clic derecho: terminar.",
            ),
            DrawMethod::PickLine => self.say(
                "Elegir línea: clic en una línea, polilínea, arco o círculo del dibujo 2D (o de un DXF).",
            ),
            DrawMethod::Normal => {}
        }
    }

    /// A click while drawing a spline or picking a line. Returns whether it
    /// was used.
    pub(super) fn method_click(&mut self, p: Point2) -> bool {
        match self.draw_method {
            DrawMethod::Normal => false,
            DrawMethod::Spline if self.spline_closes().is_some() => {
                // Open by default; a click on its first point closes it.
                if self.spline_points.len() >= 3
                    && self.spline_points[0].distance(p) < self.view.to_metres(REACH)
                {
                    self.finish_spline_as(true);
                } else if !self.spline_points.last().is_some_and(|q| q.approx_eq(p)) {
                    self.spline_points.push(p);
                }
                true
            }
            DrawMethod::PickLine if self.picks_lines() => {
                let reach = self.view.to_metres(REACH);
                let model = self.ed.model();
                let line = model
                    .drafts_in(archisim_core::ViewRef::Level(self.ed.level))
                    .filter(|d| {
                        archisim_draft::distance(&d.shape, p, crate::pick::paper_k()) <= reach
                    })
                    .min_by(|a, b| {
                        let k = crate::pick::paper_k();
                        archisim_draft::distance(&a.shape, p, k)
                            .total_cmp(&archisim_draft::distance(&b.shape, p, k))
                    })
                    .and_then(|d| picked_line(&d.shape));
                match line {
                    Some(l) => self.follow_line(l),
                    None => self.complain(
                        "Ahí no hay una línea del dibujo 2D: clic sobre una línea, polilínea, arco o círculo.",
                    ),
                }
                true
            }
            _ => false,
        }
    }

    /// Ends the spline being drawn (Enter, double click) and makes the
    /// element along it: open, unless the tool only makes outlines.
    pub(super) fn finish_spline(&mut self) {
        let closes = self.spline_closes() == Some(true);
        self.finish_spline_as(closes);
    }

    /// Ends the spline, closed (back to its first point) or open.
    fn finish_spline_as(&mut self, closes: bool) {
        let pts = std::mem::take(&mut self.spline_points);
        let Some(outline_only) = self.spline_closes() else {
            return;
        };
        let closes = closes || outline_only;
        if pts.len() < 2 || (closes && pts.len() < 3) {
            return;
        }
        let pieces: Vec<(Point2, Point2, f64)> = spline_arcs(&pts, closes);
        let points = {
            let mut c = catmull_rom(&pts, closes, PER_SPAN);
            if closes {
                // An outline does not repeat its first point.
                c.dedup_by(|a, b| a.approx_eq(*b));
                if c.len() > 1 && c[0].approx_eq(c[c.len() - 1]) {
                    c.pop();
                }
                // A path (a railing, a road, a band) closes on itself.
                if !self.closed_line_is_outline() {
                    c.push(c[0]);
                }
            }
            c
        };
        self.follow_line(PickedLine {
            pieces,
            closed: closes,
            points,
        });
    }

    /// Whether a closed line makes an outline with the tool in use (a
    /// slab, room, plot, roof, face or volume), not a path that goes round.
    fn closed_line_is_outline(&self) -> bool {
        match self.tool {
            Tool::Slab | Tool::Zone | Tool::Plot | Tool::Roof => true,
            Tool::Mass => self.mass_settings.mode != crate::editor::MassMode::Path,
            _ => false,
        }
    }

    /// Makes the element of the tool in use along a line.
    fn follow_line(&mut self, l: PickedLine) {
        let tool = self.tool;
        match tool {
            Tool::Wall => {
                let pieces: Vec<Piece> = l
                    .pieces
                    .iter()
                    .map(|(a, b, bulge)| Piece {
                        start: *a,
                        end: *b,
                        bulge: *bulge,
                    })
                    .collect();
                self.build_walls(pieces);
            }
            Tool::Slab | Tool::Zone | Tool::Plot | Tool::Roof | Tool::Mass if l.closed => {
                let outline = l.points;
                match tool {
                    Tool::Slab => self.build_slab(outline),
                    Tool::Zone => self.build_space(outline),
                    Tool::Plot => self.build_plot(outline),
                    Tool::Roof => self.build_roof(outline),
                    _ if self.mass_settings.mode == crate::editor::MassMode::Path => {
                        self.concept.path = outline;
                        self.finish_path();
                    }
                    _ => self.build_mass(outline),
                }
            }
            Tool::Mass => {
                // Open: a band along it («Seguir») or loose edges (Arista).
                self.concept.path = l.points;
                self.finish_path();
            }
            Tool::Slab | Tool::Zone | Tool::Plot | Tool::Roof => self.complain(
                "Para un contorno hace falta una línea cerrada (polilínea cerrada o círculo).",
            ),
            Tool::Railing => {
                self.railing_points = l.points;
                self.finish_railing();
            }
            Tool::Road => {
                self.road_points = l.points;
                self.finish_road();
            }
            Tool::Pipe | Tool::Duct => {
                self.run_points = l.points;
                self.finish_run();
            }
            // One element along each straight piece.
            Tool::Beam | Tool::CurtainWall | Tool::Footing | Tool::Grid => {
                let mut made = 0;
                for (a, b, bulge) in &l.pieces {
                    if bulge.abs() > 1e-9 || a.approx_eq(*b) {
                        continue;
                    }
                    let ok = match tool {
                        Tool::Beam => self.ed.add_beam(&self.beam_settings, *a, *b).is_ok(),
                        Tool::CurtainWall => self
                            .ed
                            .add_curtain_wall(&self.curtain_settings, *a, *b)
                            .is_ok(),
                        Tool::Grid => self.ed.add_grid(*a, *b).is_ok(),
                        _ => {
                            let s = self.footing_settings;
                            let kind = match s.mode {
                                crate::editor::FootingMode::Pad => {
                                    FootingKind::TieBeam { start: *a, end: *b }
                                }
                                _ => s.between(*a, *b, None),
                            };
                            self.ed.add_footing(&s, kind).is_ok()
                        }
                    };
                    made += usize::from(ok);
                }
                if made == 0 {
                    self.complain("Esa línea no tiene tramos rectos para este elemento.");
                } else {
                    self.say(format!("{made} elementos a lo largo de la línea."));
                }
            }
            _ => {}
        }
    }

    /// Esc while drawing a spline: the last point goes; with none, back to
    /// the tool's own way. Returns whether it was used.
    pub(super) fn method_escape(&mut self) -> bool {
        match self.draw_method {
            DrawMethod::Normal => false,
            DrawMethod::Spline if self.spline_points.pop().is_some() => true,
            _ => {
                self.draw_method = DrawMethod::Normal;
                true
            }
        }
    }

    pub(super) fn draw_method_preview(&self, painter: &egui::Painter, rect: Rect) {
        if self.draw_method != DrawMethod::Spline || self.spline_points.is_empty() {
            return;
        }
        let Some(closes) = self.spline_closes() else {
            return;
        };
        let mut pts = self.spline_points.clone();
        if let Some(c) = self.cursor {
            pts.push(c.point);
        }
        let curve = catmull_rom(&pts, closes && pts.len() >= 3, PER_SPAN);
        let screen = self.polygon(rect, &curve);
        painter.add(Shape::line(screen, Stroke::new(1.5, ACCENT)));
        for p in &self.spline_points {
            let s = self.view.to_screen(rect, *p);
            painter.rect_stroke(
                Rect::from_center_size(s, egui::vec2(6.0, 6.0)),
                0.0,
                Stroke::new(1.0, ACCENT),
                egui::StrokeKind::Middle,
            );
        }
    }

    // ---- Length and angle -------------------------------------------------

    /// The fields of the contextual bar: the length and angle from the
    /// last point to the cursor, typed to place the next point there.
    pub(super) fn length_angle_fields(&mut self, ui: &mut Ui) {
        let Some(a) = self.track_anchor() else {
            return;
        };
        let to = self.cursor.map_or(a, |c| c.point);
        let d = to - a;
        let (len, ang) = (d.length(), d.y.atan2(d.x));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("L").small());
        let typed_len = text_edit_committed(ui, Id::unique("bar-length"), &fmt_mm(len), 64.0)
            .and_then(|t| crate::units::parse_length(&t));
        ui.label(egui::RichText::new("mm  ∠").small());
        let typed_ang = text_edit_committed(
            ui,
            Id::unique("bar-angle"),
            &super::structure_ui::fmt_deg(ang),
            48.0,
        )
        .and_then(|t| {
            t.trim()
                .trim_end_matches('°')
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .map(f64::to_radians)
        });
        ui.label(egui::RichText::new("°").small());
        if typed_len.is_none() && typed_ang.is_none() {
            return;
        }
        // The Enter that closed the field is not the plan's (it would end
        // what is being drawn).
        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let l = typed_len.unwrap_or(len);
        let t = typed_ang.unwrap_or(ang);
        if l <= 1e-6 {
            self.complain("La longitud tiene que ser mayor que cero.");
            return;
        }
        self.point_at(a + Point2::new(t.cos(), t.sin()) * l);
    }

    /// A point placed as a click of the tool in use there.
    pub(super) fn point_at(&mut self, p: Point2) {
        if self.method_click(p) {
            return;
        }
        match self.tool {
            Tool::Wall => {
                if let Outcome::Build(pieces) = self.wall_tool.click(p) {
                    self.build_walls(pieces);
                }
            }
            Tool::Slab => match self.slab_tool.click(p) {
                SlabOutcome::Build(outline) => self.build_slab(outline),
                SlabOutcome::Invalid => {
                    self.complain("El contorno no puede cruzarse: corrige la última esquina.")
                }
                SlabOutcome::Nothing => {}
            },
            Tool::Zone => self.space_click(p),
            Tool::Roof => self.roof_click(p),
            Tool::Stair => self.stair_click(p),
            Tool::Railing => self.railing_click(p),
            Tool::CurtainWall => self.curtain_click(p),
            Tool::Section => self.section_click(p),
            Tool::Column | Tool::Beam => self.member_click(p),
            Tool::Grid | Tool::Footing => self.foundation_click(p),
            Tool::Mass | Tool::Tape => self.concept_click(p),
            Tool::Plot => self.plot_click(p),
            Tool::Road => self.road_click(p),
            Tool::Pipe | Tool::Duct => self.run_click(p),
            Tool::Device => self.device_click(p),
            Tool::Terrain => self.terrain_click(p),
            _ => {}
        }
        // The cursor where the point went, for what follows.
        if let Some(c) = &mut self.cursor {
            c.point = p;
        }
    }
}
