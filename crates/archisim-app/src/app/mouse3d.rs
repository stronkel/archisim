//! The mouse in the 3D view (concepto de interfaz del 24/10/2026): the
//! left button picks and moves, the right button held turns the view, the
//! wheel pressed moves it. So turning the view never moves anything by
//! mistake.
//!
//! A left drag that starts on furniture slides it as in Paralives
//! (`app/placing`); one that starts on something else selected moves it
//! over its plane, drawn where it goes; one that starts elsewhere opens a
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
    /// A decal slid over the plane of its face (U8): where it was grabbed
    /// and where that point is now, absolute.
    Decal {
        id: archisim_core::DecalId,
        from: [f64; 3],
        to: [f64; 3],
    },
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
                (_, Some((Element::Decal(id), at)))
                    if self.ed.selection.contains(&Element::Decal(id)) =>
                {
                    let p = [at.x as f64, at.y as f64, at.z as f64];
                    Some(Drag3d::Decal { id, from: p, to: p })
                }
                (_, Some((e, at))) if self.ed.selection.contains(&e) => {
                    if !self.ed.selection.contains(&e) {
                        if !shift {
                            self.ed.selection.clear();
                        }
                        self.ed.selection.insert(e);
                    }
                    let from = Point2::new(at.x as f64, at.y as f64);
                    self.take_for_move(at.z as f64);
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
                            .map_or(to, |p| self.near_walls(from, self.moved_to(from, p))),
                    },
                    Drag3d::Window { start, .. } => Drag3d::Window { start, now: pos },
                    Drag3d::Decal { id, from, to } => Drag3d::Decal {
                        id,
                        from,
                        to: self.on_decal_plane(rect, pos, id, from).unwrap_or(to),
                    },
                });
                if let Some(Drag3d::Move { from, to, .. }) = self.drag3d {
                    self.move_preview_3d(from, to);
                }
                ctx.request_repaint();
            }
        }
        if response.drag_stopped() {
            self.drag3d = None;
            self.move_clear.clear();
            self.v3d.set_overlay(None, || None);
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
                Drag3d::Decal { id, from, to } => {
                    // To the centimetre (along the face: off it stays 0).
                    let mut d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]]
                        .map(|v| (v * 100.0).round() / 100.0);
                    // Kept on the face's plane.
                    if let Some(n) = self.ed.model().decal(id).map(|x| x.normal) {
                        let off = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
                        for k in 0..3 {
                            d[k] -= n[k] * off;
                        }
                    }
                    if d.iter().all(|v| v.abs() < 1e-4) {
                        return;
                    }
                    let r = self.ed.transact("Mover la pegatina", |m| {
                        m.edit_decal(id, |x| {
                            for (c, v) in x.centre.iter_mut().zip(d) {
                                *c += v;
                            }
                        })
                    });
                    self.report(r, "Pegatina movida sobre su cara.");
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

    /// What is moved, drawn where it would go (the shader hides it where
    /// it is): built on a copy of the model with the same move the
    /// release makes.
    fn move_preview_3d(&mut self, from: Point2, to: Point2) {
        if from.distance(to) < 1e-4 {
            self.v3d.set_overlay(None, || None);
            return;
        }
        let key = format!(
            "move3d:{}:{:.3}:{:.3}:{:.3}:{:.3}",
            self.ed.revision(),
            from.x,
            from.y,
            to.x,
            to.y
        );
        let model = self.ed.model();
        let sel = self.ed.selection.clone();
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            let drag = crate::drag::Drag {
                grip: crate::drag::Grip::Body,
                from,
                to,
                variant: false,
            };
            crate::drag::apply(&mut m, &sel, &drag).ok()?;
            let solids: Vec<_> = archisim_geom::solid::model_solids(&m)
                .into_iter()
                .filter(|s| sel.contains(&crate::pick::of_solid(s.owner)))
                .collect();
            Some(crate::view3d::scene::Scene::of_solids(&m, solids))
        });
    }

    /// Before moving the selection in 3D: the faces of the walls of its
    /// level it can be pulled to, and its points; the clear distances are
    /// drawn at height `z`.
    pub(super) fn take_for_move(&mut self, z: f64) {
        let sel = self.ed.selection.clone();
        self.move_points = crate::wall_faces::selection_points(&self.ed, &sel);
        self.move_faces = if self.move_points.is_empty() {
            Vec::new()
        } else {
            crate::wall_faces::faces(&self.ed, self.ed.level, &sel)
        };
        // Measured along the object's own sides when it is one.
        self.move_axis = match sel.iter().next() {
            Some(Element::Object(id)) if sel.len() == 1 => self
                .ed
                .model()
                .object(*id)
                .map_or(Point2::new(1.0, 0.0), |o| {
                    Point2::new(o.rotation.cos(), o.rotation.sin())
                }),
            _ => Point2::new(1.0, 0.0),
        };
        self.move_clear.clear();
        self.move_clear_z = z;
    }

    /// The end of a move from `from` to `to`, pulled to the faces of the
    /// walls near what is moved; keeps its clear distances to draw them.
    fn near_walls(&mut self, from: Point2, to: Point2) -> Point2 {
        if self.move_points.is_empty() {
            return to;
        }
        let d = crate::wall_faces::snap_delta(&self.move_faces, &self.move_points, to - from);
        let moved: Vec<Point2> = self.move_points.iter().map(|p| *p + d).collect();
        self.move_clear = crate::wall_faces::clearances(&self.move_faces, &moved, self.move_axis);
        from + d
    }

    /// The clear distances of what is being moved to the walls around it,
    /// as the plan's temporary dimensions.
    pub(super) fn draw_clearances_3d(&self, painter: &egui::Painter, rect: Rect) {
        if self.move_clear.is_empty() {
            return;
        }
        let size = glam::Vec2::new(rect.width(), rect.height());
        let z = self.move_clear_z as f32;
        let screen = |p: Point2| {
            self.v3d
                .camera
                .project_to_screen(glam::Vec3::new(p.x as f32, p.y as f32, z), size)
                .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
        };
        let blue = super::TRACK_LINE;
        for c in &self.move_clear {
            let (Some(a), Some(b)) = (screen(c.from), screen(c.to)) else {
                continue;
            };
            painter.line_segment([a, b], Stroke::new(1.0, blue));
            let dir = (b - a).normalized();
            let tick = egui::vec2(-dir.y, dir.x) * 4.0;
            for q in [a, b] {
                painter.line_segment([q - tick, q + tick], Stroke::new(1.5, blue));
            }
            let text = crate::units::fmt_len(c.value());
            let galley = painter.layout_no_wrap(text, egui::FontId::monospace(12.0), blue);
            let r = Rect::from_center_size(a + (b - a) * 0.5, galley.size())
                .expand2(egui::vec2(4.0, 2.0));
            painter.rect_filled(r, 3.0, Color32::WHITE);
            painter.rect_stroke(r, 3.0, Stroke::new(1.0, blue), egui::StrokeKind::Inside);
            painter.galley(r.min + egui::vec2(4.0, 2.0), galley, blue);
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

    /// Where the cursor's ray meets the plane of a decal's face (through
    /// `on`, a point of it), absolute.
    fn on_decal_plane(
        &self,
        rect: Rect,
        pos: Pos2,
        id: archisim_core::DecalId,
        on: [f64; 3],
    ) -> Option<[f64; 3]> {
        let n = self.ed.model().decal(id)?.normal;
        let n = glam::DVec3::new(n[0], n[1], n[2]);
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        let den = d.dot(n);
        if den.abs() < 1e-9 {
            return None;
        }
        let t = (glam::DVec3::from_array(on) - o).dot(n) / den;
        (t > 0.0).then(|| (o + d * t).to_array())
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
                let size = glam::Vec2::new(rect.width(), rect.height());
                let screen = |p: Point2| {
                    self.v3d
                        .camera
                        .project_to_screen(glam::Vec3::new(p.x as f32, p.y as f32, z as f32), size)
                        .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                };
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
            Some(Drag3d::Decal { id, from, to }) => {
                // Its outline where it would go.
                let Some(dec) = self.ed.model().decal(id) else {
                    return;
                };
                let base = self
                    .ed
                    .model()
                    .level(dec.level)
                    .map_or(0.0, |l| l.elevation);
                let size = glam::Vec2::new(rect.width(), rect.height());
                let shift = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
                let pts: Vec<Pos2> = dec
                    .corners(base)
                    .iter()
                    .filter_map(|c| {
                        let p = glam::Vec3::new(
                            (c[0] + shift[0]) as f32,
                            (c[1] + shift[1]) as f32,
                            (c[2] + shift[2]) as f32,
                        );
                        self.v3d
                            .camera
                            .project_to_screen(p, size)
                            .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                    })
                    .collect();
                if pts.len() == 4 {
                    painter.add(egui::Shape::closed_line(pts.clone(), Stroke::new(1.5, ACCENT)));
                    let dist = (shift[0].powi(2) + shift[1].powi(2) + shift[2].powi(2)).sqrt();
                    super::handles::tag(
                        painter,
                        pts[2],
                        format!("Mover {}", crate::units::fmt_len(dist)),
                        ACCENT,
                    );
                }
            }
            None => {}
        }
    }
}
