//! The mouse in the 3D view (concepto de interfaz del 24/10/2026): the
//! left button picks and moves, the right button held turns the view, the
//! wheel pressed moves it. So turning the view never moves anything by
//! mistake.
//!
//! A left drag that starts on something selected (or on a piece of
//! furniture) moves it over its floor; one that starts elsewhere opens a
//! selection window: from left to right it takes what is wholly inside,
//! from right to left what it touches (as in AutoCAD).

use archisim_core::Point2;
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect, Stroke};

use super::*;

/// A left drag in the 3D view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drag3d {
    /// Moving the selection over the plane at height `z`.
    Move { from: Point2, to: Point2, z: f64 },
    /// A selection window, between two points of the screen.
    Window { start: Pos2, now: Pos2 },
}

impl ArchiSimApp {
    /// The element of the project under a point of the 3D view (a copy of
    /// a module: the copy), and where it is hit.
    pub(super) fn element_at_3d(&self, rect: Rect, pos: Pos2) -> Option<(Element, glam::Vec3)> {
        let size = rect.size();
        let (at, _, owner) = self.v3d.hit(pos - rect.min, size)?;
        let crate::view3d::scene::Owner::Model(e) = owner else {
            return None;
        };
        let e = crate::pick::solid_of(e)
            .and_then(|o| self.ed.model().module_of_flat_id(o.id()))
            .map_or(e, Element::Module);
        Some((e, at))
    }

    /// Takes, follows and lets go of a left drag (not on a handle and with
    /// no tool in use).
    pub(super) fn left_drag_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) {
        if response.drag_started_by(PointerButton::Primary) {
            let origin = ctx.input(|i| i.pointer.press_origin());
            let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
            let hit = origin.and_then(|p| self.element_at_3d(rect, p));
            self.drag3d = match (origin, hit) {
                (_, Some((e, at)))
                    if self.ed.selection.contains(&e) || matches!(e, Element::Object(_)) =>
                {
                    if !self.ed.selection.contains(&e) {
                        if !shift {
                            self.ed.selection.clear();
                        }
                        self.ed.selection.insert(e);
                    }
                    let from = Point2::new(at.x as f64, at.y as f64);
                    Some(Drag3d::Move {
                        from,
                        to: from,
                        z: at.z as f64,
                    })
                }
                (Some(start), _) => Some(Drag3d::Window { start, now: start }),
                _ => None,
            };
        }
        let Some(d) = self.drag3d else {
            return;
        };
        if response.dragged_by(PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                self.drag3d = Some(match d {
                    Drag3d::Move { from, z, to } => Drag3d::Move {
                        from,
                        z,
                        to: self
                            .on_plane_3d(rect, pos, z)
                            .map_or(to, |p| self.moved_to(from, p)),
                    },
                    Drag3d::Window { start, .. } => Drag3d::Window { start, now: pos },
                });
            }
        }
        if response.drag_stopped() {
            self.drag3d = None;
            match d {
                Drag3d::Move { from, to, .. } => {
                    if from.distance(to) < 1e-4 {
                        return;
                    }
                    let variant = ctx.input(|i| i.modifiers.command);
                    let drag = crate::drag::Drag {
                        grip: crate::drag::Grip::Body,
                        from,
                        to,
                        variant,
                    };
                    let dist = crate::units::fmt_len(from.distance(to));
                    match self.ed.drag(&drag) {
                        Ok(label) => self.say(format!("{label}: {dist}.")),
                        Err(e) => self.complain(format!("No se puede: {e}.")),
                    }
                }
                Drag3d::Window { start, now } => {
                    let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
                    let found = self.in_window_3d(rect, start, now);
                    if !shift {
                        self.ed.selection.clear();
                    }
                    self.ed.selection.extend(found);
                }
            }
        }
    }

    /// Where the cursor is on the horizontal plane at height `z`.
    fn on_plane_3d(&self, rect: Rect, pos: Pos2, z: f64) -> Option<Point2> {
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        if d.z.abs() < 1e-9 {
            return None;
        }
        let t = (z - o.z) / d.z;
        (t > 0.0).then(|| Point2::new(o.x + d.x * t, o.y + d.y * t))
    }

    /// Where a move from `from` ends with the cursor at `p`: to the
    /// centimetre, or to the ground grid's step when it is on.
    fn moved_to(&self, from: Point2, p: Point2) -> Point2 {
        let delta = Point2::new(p.x - from.x, p.y - from.y);
        let delta = if self.ground_grid {
            super::maquette_ui::snap_to_grid(delta)
        } else {
            crate::snap::round_point(delta)
        };
        Point2::new(from.x + delta.x, from.y + delta.y)
    }

    /// What a selection window takes: from left to right, what is wholly
    /// inside; from right to left, what it touches.
    fn in_window_3d(&self, rect: Rect, a: Pos2, b: Pos2) -> Vec<Element> {
        let window = Rect::from_two_pos(a, b);
        let crossing = b.x < a.x;
        let size = glam::Vec2::new(rect.width(), rect.height());
        let mut out = Vec::new();
        for (e, (lo, hi)) in self.v3d.element_bounds() {
            let mut pts = Vec::new();
            for k in 0..8 {
                let c = glam::Vec3::new(
                    if k & 1 == 0 { lo.x } else { hi.x },
                    if k & 2 == 0 { lo.y } else { hi.y },
                    if k & 4 == 0 { lo.z } else { hi.z },
                );
                if let Some(s) = self.v3d.camera.project_to_screen(c, size) {
                    pts.push(Pos2::new(rect.min.x + s.x, rect.min.y + s.y));
                }
            }
            if pts.len() < 8 {
                continue;
            }
            let r = Rect::from_points(&pts);
            let take = if crossing {
                r.intersects(window)
            } else {
                window.contains_rect(r)
            };
            if take {
                let e = crate::pick::solid_of(e)
                    .and_then(|o| self.ed.model().module_of_flat_id(o.id()))
                    .map_or(e, Element::Module);
                if !out.contains(&e) {
                    out.push(e);
                }
            }
        }
        out
    }

    /// The selection window or the move under way.
    pub(super) fn draw_drag_3d(&self, painter: &egui::Painter, rect: Rect) {
        match self.drag3d {
            Some(Drag3d::Window { start, now }) => {
                let r = Rect::from_two_pos(start, now);
                let crossing = now.x < start.x;
                let fill = if crossing {
                    Color32::from_rgba_unmultiplied(47, 138, 85, 28)
                } else {
                    Color32::from_rgba_unmultiplied(37, 99, 235, 28)
                };
                let line = if crossing {
                    Color32::from_rgb(47, 138, 85)
                } else {
                    ACCENT
                };
                painter.rect_filled(r, 2.0, fill);
                painter.rect_stroke(r, 2.0, Stroke::new(1.2, line), egui::StrokeKind::Inside);
            }
            Some(Drag3d::Move { from, to, z }) => {
                let delta = Point2::new(to.x - from.x, to.y - from.y);
                let size = glam::Vec2::new(rect.width(), rect.height());
                let screen = |p: Point2| {
                    self.v3d
                        .camera
                        .project_to_screen(glam::Vec3::new(p.x as f32, p.y as f32, z as f32), size)
                        .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                };
                // The outlines where they would go, at the height taken.
                for e in &self.ed.selection {
                    let Some(poly) = crate::pick::outline(&self.ed, *e) else {
                        continue;
                    };
                    let pts: Vec<Pos2> = poly
                        .iter()
                        .filter_map(|p| screen(Point2::new(p.x + delta.x, p.y + delta.y)))
                        .collect();
                    if pts.len() == poly.len() && pts.len() > 1 {
                        painter.add(egui::Shape::closed_line(pts, Stroke::new(2.0, ACCENT)));
                    }
                }
                if let (Some(a), Some(b)) = (screen(from), screen(to)) {
                    painter.line_segment([a, b], Stroke::new(1.2, Color32::from_gray(60)));
                    super::handles::tag(
                        painter,
                        b,
                        format!("Mover {}", crate::units::fmt_len(from.distance(to))),
                        ACCENT,
                    );
                }
            }
            None => {}
        }
    }
}
