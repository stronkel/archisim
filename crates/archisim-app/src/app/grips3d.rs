//! Stretching and turning furniture in the 3D view (bloque 5 de la
//! revisión del 22/10/2026 and concepto de interfaz del 24/10/2026): with
//! one object selected, blue arrows on its right and left sides, its front
//! and its top change its width, depth and height as the mouse drags them,
//! as the plan's handles do (a type shared with other objects is copied,
//! so only this one changes). It grows only towards the arrow, the
//! opposite side staying put (with Alt, about its middle). The orange knob
//! on the ring round it turns it: in steps of 15°, or freely with Mayús or
//! with the cursor far from it.
//!
//! While a handle is dragged the object is drawn as it will be
//! (`stretch_preview`): it grows or turns as the mouse moves, not only
//! when it is let go.

use archisim_core::ObjectId;
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect};

use super::*;

/// Which measurement a handle changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKey {
    /// The right side (`true`) or the left.
    Width(bool),
    Depth,
    Height,
    /// The knob of the ring that turns it.
    Turn,
}

/// The steps a turn snaps to (none with Mayús or far from the object).
const TURN_STEP: f64 = std::f64::consts::PI / 12.0;

impl SizeKey {
    fn key(self) -> &'static str {
        match self {
            SizeKey::Width(_) => "width",
            SizeKey::Depth => "depth",
            SizeKey::Height => "height",
            SizeKey::Turn => "rotation",
        }
    }

    /// What stays put when it grows only towards this handle.
    fn keep(self, both: bool) -> crate::drag::Keep {
        use crate::drag::Keep;
        match (self, both) {
            (SizeKey::Width(_), true) => Keep::Middle(0),
            (SizeKey::Depth, true) => Keep::Middle(1),
            (SizeKey::Width(true), false) => Keep::Low(0),
            (SizeKey::Width(false), false) => Keep::High(0),
            (SizeKey::Depth, false) => Keep::Low(1),
            (SizeKey::Height | SizeKey::Turn, _) => Keep::Low(2),
        }
    }

    /// The way it grows, in the object's axes.
    fn dir(self) -> glam::DVec3 {
        match self {
            SizeKey::Width(true) => glam::DVec3::X,
            SizeKey::Width(false) => glam::DVec3::NEG_X,
            SizeKey::Depth => glam::DVec3::Y,
            SizeKey::Height | SizeKey::Turn => glam::DVec3::Z,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SizeKey::Width(_) => "Ancho",
            SizeKey::Depth => "Fondo",
            SizeKey::Height => "Altura",
            SizeKey::Turn => "Giro",
        }
    }
}

/// A handle being dragged: where it was taken and what the measurement
/// was then, and what it is now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectGrip3d {
    pub object: ObjectId,
    pub key: SizeKey,
    pub start: glam::DVec3,
    pub before: f64,
    pub now: f64,
    /// Alt: it grows on both sides, about its middle.
    pub both: bool,
    /// Mayús (or the cursor far away): a turn is not snapped.
    pub free: bool,
    /// Where on the screen it was taken.
    pub press: Pos2,
}

/// How near a handle the cursor takes it, in screen points.
const REACH: f32 = 11.0;

/// How far out of the object an arrow starts, and where it is taken, in
/// screen points.
const ARROW_GAP: f32 = 6.0;
pub(super) const ARROW_GRAB: f32 = ARROW_GAP + 9.0;

impl ArchiSimApp {
    /// The object selected alone, if it is one.
    fn single_object(&self) -> Option<ObjectId> {
        if self.ed.selection.len() != 1 {
            return None;
        }
        match self.ed.selection.iter().next() {
            Some(Element::Object(id)) => Some(*id),
            _ => None,
        }
    }

    /// A measurement of an object's type (its own value or the family's).
    fn object_size(&self, id: ObjectId, key: &str) -> Option<f64> {
        crate::drag::type_number(self.ed.model(), id, key)
    }

    /// The handles of the object selected, in the model.
    pub(super) fn object_handles_3d(&self) -> Vec<(ObjectId, SizeKey, glam::DVec3)> {
        if self.tool != Tool::Arrow {
            return Vec::new();
        }
        let Some(id) = self.single_object() else {
            return Vec::new();
        };
        let model = self.ed.model();
        let Some(o) = model.object(id) else {
            return Vec::new();
        };
        let Some((lo, hi)) = model.object_type_box_3d(o.object_type) else {
            return Vec::new();
        };
        let mut v = Vec::new();
        if let Some(knob) = self
            .turn_ring(id)
            .map(|(c, r, a)| c + glam::DVec3::new(r * a.cos(), r * a.sin(), 0.0))
        {
            v.push((id, SizeKey::Turn, knob));
        }
        if model.type_grows(o.object_type) {
            return v;
        }
        let base = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let (c, s) = (o.rotation.cos(), o.rotation.sin());
        let world = |x: f64, y: f64, z: f64| {
            glam::DVec3::new(
                o.position.x + x * c - y * s,
                o.position.y + x * s + y * c,
                base + z,
            )
        };
        let mid = [
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ];
        if self.object_size(id, "width").is_some() {
            v.push((id, SizeKey::Width(true), world(hi[0], mid[1], mid[2])));
            v.push((id, SizeKey::Width(false), world(lo[0], mid[1], mid[2])));
        }
        if self.object_size(id, "depth").is_some() {
            v.push((id, SizeKey::Depth, world(mid[0], hi[1], mid[2])));
        }
        if self.object_size(id, "height").is_some() {
            v.push((id, SizeKey::Height, world(mid[0], mid[1], hi[2])));
        }
        v
    }

    /// The ring that turns an object: its middle (on the floor), its
    /// radius and the angle of its knob (in front of the object, as in
    /// the plan).
    pub(super) fn turn_ring(&self, id: ObjectId) -> Option<(glam::DVec3, f64, f64)> {
        let model = self.ed.model();
        let o = model.object(id)?;
        let (lo, hi) = crate::drag::object_box(model, id)?;
        let m = crate::drag::object_middle(model, id)?;
        let base = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let (cx, cy) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        let r = ((hi[0] - cx).hypot(hi[1] - cy)).max(0.2) + 0.25;
        let angle = o.rotation + std::f64::consts::FRAC_PI_2;
        Some((glam::DVec3::new(m.x, m.y, base + 0.01), r, angle))
    }

    fn to_screen_3d(&self, rect: Rect, p: glam::DVec3) -> Option<Pos2> {
        let size = glam::Vec2::new(rect.width(), rect.height());
        self.v3d
            .camera
            .project_to_screen(p.as_vec3(), size)
            .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
    }

    /// Where a size handle's arrow is on the screen: the point on the
    /// object's side it starts from, and the way it points. The arrow is
    /// drawn outside the object, so a small object can still be taken by
    /// its body.
    pub(super) fn handle_on_screen(
        &self,
        rect: Rect,
        key: SizeKey,
        at: glam::DVec3,
    ) -> Option<(Pos2, egui::Vec2)> {
        let rot = self
            .single_object()
            .and_then(|id| self.ed.model().object(id))
            .map_or(0.0, |o| o.rotation);
        let (c, s) = (rot.cos(), rot.sin());
        let d = key.dir();
        let world = glam::DVec3::new(d.x * c - d.y * s, d.x * s + d.y * c, d.z);
        let p = self.to_screen_3d(rect, at)?;
        let dir = self
            .to_screen_3d(rect, at + world * 0.1)
            .map_or(egui::Vec2::X, |q| q - p);
        let dir = if dir.length() > 1e-3 {
            dir.normalized()
        } else {
            egui::Vec2::X
        };
        Some((p, dir))
    }

    /// The handle under a point of the screen (the knob, or an arrow).
    fn object_handle_at(&self, rect: Rect, pos: Pos2) -> Option<(ObjectId, SizeKey, glam::DVec3)> {
        self.object_handles_3d()
            .into_iter()
            .filter_map(|h| {
                let at = if h.1 == SizeKey::Turn {
                    self.to_screen_3d(rect, h.2)?
                } else {
                    let (p, d) = self.handle_on_screen(rect, h.1, h.2)?;
                    p + d * ARROW_GRAB
                };
                Some((h, at.distance(pos)))
            })
            .filter(|(_, d)| *d <= REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    /// Hover, grab, drag and let go of the handles. Returns whether one is
    /// taken (the camera then stays still).
    pub(super) fn object_grips_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) -> bool {
        if self.object_grip.is_none() {
            self.object_grip_hover = response
                .hover_pos()
                .and_then(|p| self.object_handle_at(rect, p))
                .map(|h| h.1);
        }
        if response.drag_started_by(PointerButton::Primary) {
            let press = ctx.input(|i| i.pointer.press_origin());
            let taken = press.and_then(|p| self.object_handle_at(rect, p));
            self.object_grip = taken.and_then(|(id, key, at)| {
                let before = match key {
                    SizeKey::Turn => self.ed.model().object(id)?.rotation,
                    _ => self.object_size(id, key.key())?,
                };
                Some(ObjectGrip3d {
                    object: id,
                    key,
                    start: at,
                    before,
                    now: before,
                    both: false,
                    free: false,
                    press: press.unwrap_or(Pos2::ZERO),
                })
            });
        }
        let Some(mut g) = self.object_grip else {
            if self.object_move.is_none() && self.drag3d.is_none() {
                self.v3d.set_overlay(None, || None);
            }
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            let (alt, shift) = ctx.input(|i| (i.modifiers.alt, i.modifiers.shift));
            g.both = alt;
            g.free = shift;
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(v) = self.grip_value(&mut g, rect, pos) {
                    g.now = v;
                }
            }
            self.object_grip = Some(g);
            self.stretch_preview(&g);
            // The picture with the new shape is drawn in the next frame.
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            self.object_grip = None;
            self.v3d.set_overlay(None, || None);
            if (g.now - g.before).abs() > 1e-6 && g.key == SizeKey::Turn {
                let r = self.ed.transact("Girar objeto", |m| {
                    crate::drag::turn_object(m, g.object, g.now)
                });
                if r.is_ok() {
                    self.ed.selection = [Element::Object(g.object)].into_iter().collect();
                }
                self.report(r, &format!("Giro {}.", degrees(g.now)));
            } else if (g.now - g.before).abs() > 1e-6 {
                let keep = g.key.keep(g.both);
                let r = self.ed.transact("Estirar objeto", |m| {
                    crate::drag::resize_object(m, g.object, g.key.key(), g.now, keep)
                });
                self.report(
                    r,
                    &format!("{} {}.", g.key.label(), crate::units::fmt_len(g.now)),
                );
            }
        }
        true
    }

    /// The object as it will be, drawn in its place while the handle is
    /// dragged: built on a copy of the model with the same change the
    /// release makes, so it grows (cushions, legs, doors…) as it will.
    pub(super) fn stretch_preview(&mut self, g: &ObjectGrip3d) {
        if (g.now - g.before).abs() < 1e-6 {
            self.v3d.set_overlay(None, || None);
            return;
        }
        let key = format!(
            "{}:{}:{:?}:{}:{}",
            self.ed.revision(),
            g.object.0,
            g.key,
            g.now,
            g.both
        );
        let model = self.ed.model();
        let g = *g;
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            if g.key == SizeKey::Turn {
                crate::drag::turn_object(&mut m, g.object, g.now).ok()?;
            } else {
                crate::drag::resize_object(
                    &mut m,
                    g.object,
                    g.key.key(),
                    g.now,
                    g.key.keep(g.both),
                )
                .ok()?;
            }
            let solids = archisim_geom::solid::object_solids(&m, g.object);
            Some(crate::view3d::scene::Scene::of_solids(&m, solids))
        });
    }

    /// The measurement a handle gives with the cursor at `pos`, measured
    /// from where it was taken (the arrows are a little outside the
    /// object). A turn far from the object is not snapped.
    fn grip_value(&self, g: &mut ObjectGrip3d, rect: Rect, pos: Pos2) -> Option<f64> {
        if g.key == SizeKey::Turn {
            let (c, r, _) = self.turn_ring(g.object)?;
            let q = self.on_floor(rect, pos, c.z)?;
            let rel = q - c;
            if rel.truncate().length() > 3.0 * r {
                g.free = true;
            }
            // The knob is in front: +90° from the object's own angle.
            let a = rel.y.atan2(rel.x) - std::f64::consts::FRAC_PI_2;
            let step = if g.free { 1f64.to_radians() } else { TURN_STEP };
            return Some(((a / step).round() * step).rem_euclid(std::f64::consts::TAU));
        }
        let now = self.grip_delta(g, rect, pos)?;
        let then = self.grip_delta(g, rect, g.press).unwrap_or(0.0);
        let v = ((g.before + now - then) * 100.0).round() / 100.0;
        Some(v.max(0.05))
    }

    /// The cursor on the horizontal plane at height `z`.
    fn on_floor(&self, rect: Rect, pos: Pos2, z: f64) -> Option<glam::DVec3> {
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
        (t > 0.0).then(|| o + d * t)
    }

    /// How far the cursor at `pos` is from a size handle, along its way.
    fn grip_delta(&self, g: &ObjectGrip3d, rect: Rect, pos: Pos2) -> Option<f64> {
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        let ob = self.ed.model().object(g.object)?;
        let (c, s) = (ob.rotation.cos(), ob.rotation.sin());
        Some(match g.key {
            SizeKey::Height => {
                // On an upright plane through the handle, facing the camera.
                let mut n = self.v3d.camera.forward().as_dvec3();
                n.z = 0.0;
                let n = n.normalize_or(glam::DVec3::X);
                let den = d.dot(n);
                if den.abs() < 1e-9 {
                    return None;
                }
                let t = (g.start - o).dot(n) / den;
                (o + d * t).z - g.start.z
            }
            _ => {
                // On the level plane through the handle.
                let q = self.on_floor(rect, pos, g.start.z)? - g.start;
                let (x, y) = (q.x * c + q.y * s, -q.x * s + q.y * c);
                let k = if g.both { 2.0 } else { 1.0 };
                match g.key {
                    SizeKey::Width(true) => k * x,
                    SizeKey::Width(false) => -k * x,
                    _ => k * y,
                }
            }
        })
    }

    /// The handles, and the new size or angle while one is dragged.
    pub(super) fn draw_object_grips_3d(&self, painter: &egui::Painter, rect: Rect) {
        let handles = self.object_handles_3d();
        let hover = painter.ctx().pointer_hover_pos();
        for (id, key, at) in &handles {
            let hot = self.object_grip_hover == Some(*key)
                || self.object_grip.is_some_and(|g| g.key == *key);
            if *key == SizeKey::Turn {
                // The knob, with an arc of its ring (the whole ring while
                // it is taken).
                let Some((c, r, _)) = self.turn_ring(*id) else {
                    continue;
                };
                let Some(kp) = self.to_screen_3d(rect, *at) else {
                    continue;
                };
                // On the screen the ring is an ellipse: drawn by points.
                let base = (at.y - c.y).atan2(at.x - c.x);
                let span = if hot { std::f64::consts::TAU } else { 0.9 };
                let pts: Vec<Pos2> = (0..=48)
                    .filter_map(|k| {
                        let a = base - span / 2.0 + span * k as f64 / 48.0;
                        self.to_screen_3d(rect, c + glam::DVec3::new(r * a.cos(), r * a.sin(), 0.0))
                    })
                    .collect();
                let ring = Color32::from_rgba_unmultiplied(
                    super::handles::TURN.r(),
                    super::handles::TURN.g(),
                    super::handles::TURN.b(),
                    if hot { 210 } else { 150 },
                );
                for w in pts.chunks(2) {
                    if let [a, b] = w {
                        painter.line_segment([*a, *b], egui::Stroke::new(1.8, ring));
                    }
                }
                let k = if hot { 7.0 } else { 5.5 };
                painter.circle(
                    kp,
                    k,
                    super::handles::TURN,
                    egui::Stroke::new(2.0, Color32::WHITE),
                );
                continue;
            }
            let Some((p, dir)) = self.handle_on_screen(rect, *key, *at) else {
                continue;
            };
            let near = hover.is_some_and(|h| h.distance(p + dir * ARROW_GRAB) <= REACH);
            super::handles::arrow(
                painter,
                p + dir * (ARROW_GAP + 3.0),
                dir,
                ACCENT,
                hot || near,
            );
        }
        // The side that stays put while one is dragged.
        if let Some(g) = self.object_grip.filter(|g| !g.both) {
            let opposite = match g.key {
                SizeKey::Width(r) => handles.iter().find(|h| h.1 == SizeKey::Width(!r)),
                _ => None,
            };
            if let Some(p) = opposite.and_then(|h| self.to_screen_3d(rect, h.2)) {
                super::handles::pin(painter, p);
            }
        }
        if let Some(g) = self.object_grip {
            let at = painter.ctx().pointer_latest_pos().or_else(|| {
                handles
                    .iter()
                    .find(|h| h.1 == g.key)
                    .and_then(|h| self.to_screen_3d(rect, h.2))
            });
            if let Some(p) = at {
                let (text, fill) = match g.key {
                    SizeKey::Turn => (
                        format!(
                            "Giro {}{}",
                            degrees(g.now),
                            if g.free { "" } else { " · Mayús: libre" }
                        ),
                        super::handles::TURN,
                    ),
                    _ => (
                        format!(
                            "{} {} → {}{}",
                            g.key.label(),
                            crate::units::fmt_len(g.before),
                            crate::units::fmt_len(g.now),
                            if g.both { " · por los dos lados" } else { "" }
                        ),
                        ACCENT,
                    ),
                };
                super::handles::tag(painter, p, text, fill);
            }
        }
    }
}

/// An angle in whole degrees, between 0 and 360, as the card writes it.
fn degrees(a: f64) -> String {
    let d = a.to_degrees().rem_euclid(360.0);
    let d = if (d - d.round()).abs() < 0.05 {
        format!("{:.0}", d.round() % 360.0)
    } else {
        format!("{d:.1}").replace('.', ",")
    };
    format!("{d}°")
}
