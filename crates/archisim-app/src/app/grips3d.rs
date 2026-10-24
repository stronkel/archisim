//! Stretching furniture in the 3D view (bloque 5 de la revisión del
//! 22/10/2026): with one object selected, blue arrows on its right and
//! left sides, its front and its top change its width, depth and height
//! as the mouse drags them, as the plan's handles do (a type shared with
//! other objects is copied, so only this one changes). It grows only
//! towards the arrow, the opposite side staying put (with Alt, about its
//! middle); the orange dot on the ring in front of it turns it.

use archisim_core::ObjectId;
use eframe::egui::{self, PointerButton, Pos2, Rect};

use super::*;

/// Which measurement a handle changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKey {
    /// The right side (`true`) or the left.
    Width(bool),
    Depth,
    Height,
}

impl SizeKey {
    fn key(self) -> &'static str {
        match self {
            SizeKey::Width(_) => "width",
            SizeKey::Depth => "depth",
            SizeKey::Height => "height",
        }
    }

    /// What stays put when it grows only towards this handle.
    fn keep(self) -> crate::drag::Keep {
        use crate::drag::Keep;
        match self {
            SizeKey::Width(true) => Keep::Low(0),
            SizeKey::Width(false) => Keep::High(0),
            SizeKey::Depth => Keep::Low(1),
            SizeKey::Height => Keep::Low(2),
        }
    }

    /// The way it grows, in the object's axes.
    fn dir(self) -> glam::DVec3 {
        match self {
            SizeKey::Width(true) => glam::DVec3::X,
            SizeKey::Width(false) => glam::DVec3::NEG_X,
            SizeKey::Depth => glam::DVec3::Y,
            SizeKey::Height => glam::DVec3::Z,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SizeKey::Width(_) => "Ancho",
            SizeKey::Depth => "Fondo",
            SizeKey::Height => "Altura",
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
    /// Alt: on both sides, about its middle.
    pub both: bool,
    /// Where on the screen it was taken.
    pub press: Pos2,
}

/// The turning handle being dragged: the angle the object would take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectTurn3d {
    pub object: ObjectId,
    pub angle: f64,
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
        if model.type_grows(o.object_type) {
            return Vec::new();
        }
        let Some((lo, hi)) = model.object_type_box_3d(o.object_type) else {
            return Vec::new();
        };
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
        let mut v = Vec::new();
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

    fn to_screen_3d(&self, rect: Rect, p: glam::DVec3) -> Option<Pos2> {
        let size = glam::Vec2::new(rect.width(), rect.height());
        self.v3d
            .camera
            .project_to_screen(p.as_vec3(), size)
            .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
    }

    /// Where a handle's arrow is on the screen: the point on the object's
    /// side it starts from, and the way it points. The arrow is drawn
    /// outside the object, so a small object can still be taken by its
    /// body.
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

    /// The handle under a point of the screen (on its arrow).
    fn object_handle_at(&self, rect: Rect, pos: Pos2) -> Option<(ObjectId, SizeKey, glam::DVec3)> {
        self.object_handles_3d()
            .into_iter()
            .filter_map(|h| {
                self.handle_on_screen(rect, h.1, h.2)
                    .map(|(p, d)| (h, (p + d * ARROW_GRAB).distance(pos)))
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
                let before = self.object_size(id, key.key())?;
                Some(ObjectGrip3d {
                    object: id,
                    key,
                    start: at,
                    before,
                    now: before,
                    both: false,
                    press: press.unwrap_or(Pos2::ZERO),
                })
            });
            if self.object_grip.is_none() {
                self.object_turn = ctx
                    .input(|i| i.pointer.press_origin())
                    .filter(|p| self.on_turn_handle(rect, *p))
                    .and_then(|_| self.single_object())
                    .and_then(|id| {
                        let angle = self.ed.model().object(id)?.rotation;
                        Some(ObjectTurn3d { object: id, angle })
                    });
            }
        }
        if let Some(mut t) = self.object_turn {
            if response.dragged_by(PointerButton::Primary) {
                if let Some(at) = response
                    .interact_pointer_pos()
                    .and_then(|p| self.ground_at(rect, p, t.object))
                {
                    if let Some(a) = crate::drag::turn_angle(self.ed.model(), t.object, at) {
                        t.angle = a;
                        self.object_turn = Some(t);
                    }
                }
            }
            if response.drag_stopped() {
                self.object_turn = None;
                let r = self.ed.transact("Girar objeto", |m| {
                    crate::drag::turn_object(m, t.object, t.angle)
                });
                self.report(r, &format!("Girado a {:.0}°.", t.angle.to_degrees()));
            }
            return true;
        }
        let Some(mut g) = self.object_grip else {
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            g.both = ctx.input(|i| i.modifiers.alt);
            self.object_grip = Some(g);
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(v) = self.grip_value(&g, rect, pos) {
                    g.now = v;
                    self.object_grip = Some(g);
                }
            }
        }
        if response.drag_stopped() {
            self.object_grip = None;
            if (g.now - g.before).abs() > 1e-6 {
                let keep = if g.both {
                    match g.key {
                        SizeKey::Width(_) => crate::drag::Keep::Middle(0),
                        SizeKey::Depth => crate::drag::Keep::Middle(1),
                        SizeKey::Height => crate::drag::Keep::Low(2),
                    }
                } else {
                    g.key.keep()
                };
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

    /// The measurement a handle gives with the cursor at `pos`.
    /// (Measured from where the arrow was taken, not from the object's
    /// side: the arrow is a little outside it.)
    fn grip_value(&self, g: &ObjectGrip3d, rect: Rect, pos: Pos2) -> Option<f64> {
        let now = self.grip_delta(g, rect, pos)?;
        let then = self.grip_delta(g, rect, g.press).unwrap_or(0.0);
        let v = ((g.before + now - then) * 100.0).round() / 100.0;
        Some(v.max(0.05))
    }

    /// How far the cursor at `pos` is from the handle, along its way.
    fn grip_delta(&self, g: &ObjectGrip3d, rect: Rect, pos: Pos2) -> Option<f64> {
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        let ob = self.ed.model().object(g.object)?;
        let (c, s) = (ob.rotation.cos(), ob.rotation.sin());
        let delta = match g.key {
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
                if d.z.abs() < 1e-9 {
                    return None;
                }
                let t = (g.start.z - o.z) / d.z;
                if t <= 0.0 {
                    return None;
                }
                let q = o + d * t - g.start;
                let (x, y) = (q.x * c + q.y * s, -q.x * s + q.y * c);
                let k = if g.both { 2.0 } else { 1.0 };
                match g.key {
                    SizeKey::Width(true) => k * x,
                    SizeKey::Width(false) => -k * x,
                    _ => k * y,
                }
            }
        };
        Some(delta)
    }

    /// Where the cursor is on the floor the object stands on.
    fn ground_at(&self, rect: Rect, pos: Pos2, id: ObjectId) -> Option<archisim_core::Point2> {
        let model = self.ed.model();
        let o = model.object(id)?;
        let z = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (or, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        if d.z.abs() < 1e-9 {
            return None;
        }
        let t = (z - or.z) / d.z;
        (t > 0.0).then(|| {
            let q = or + d * t;
            archisim_core::Point2::new(q.x, q.y)
        })
    }

    /// The turning handle of the object selected and its middle, on the
    /// floor, in the model.
    fn turn_handle_3d(&self) -> Option<(glam::DVec3, glam::DVec3)> {
        if self.tool != Tool::Arrow {
            return None;
        }
        let id = self.single_object()?;
        let model = self.ed.model();
        let o = model.object(id)?;
        let z = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let h = crate::drag::turn_handle(model, id)?;
        let m = crate::drag::object_middle(model, id)?;
        Some((glam::DVec3::new(h.x, h.y, z), glam::DVec3::new(m.x, m.y, z)))
    }

    fn on_turn_handle(&self, rect: Rect, pos: Pos2) -> bool {
        self.turn_handle_3d()
            .and_then(|(h, _)| self.to_screen_3d(rect, h))
            .is_some_and(|s| s.distance(pos) <= REACH)
    }

    /// The handles, and the new size while one is dragged.
    pub(super) fn draw_object_grips_3d(&self, painter: &egui::Painter, rect: Rect) {
        let handles = self.object_handles_3d();
        for (_, key, at) in &handles {
            let Some((p, dir)) = self.handle_on_screen(rect, *key, *at) else {
                continue;
            };
            let hot = self.object_grip_hover == Some(*key)
                || self.object_grip.is_some_and(|g| g.key == *key);
            super::handles::arrow(painter, p + dir * (ARROW_GAP + 3.0), dir, ACCENT, hot);
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
        if let Some((h, m)) = self.turn_handle_3d() {
            if let (Some(hp), Some(mp)) = (self.to_screen_3d(rect, h), self.to_screen_3d(rect, m)) {
                let hot = self.object_turn.is_some()
                    || painter
                        .ctx()
                        .pointer_hover_pos()
                        .is_some_and(|p| p.distance(hp) <= REACH);
                super::handles::turn(painter, mp, hp, hot);
            }
        }
        if let Some(t) = self.object_turn {
            if let Some(hp) = self
                .turn_handle_3d()
                .and_then(|(h, _)| self.to_screen_3d(rect, h))
            {
                super::handles::tag(
                    painter,
                    hp,
                    format!("{:.0}°", t.angle.to_degrees()),
                    super::handles::TURN,
                );
            }
        }
        if let Some(g) = self.object_grip {
            if let Some(p) = handles
                .iter()
                .find(|h| h.1 == g.key)
                .and_then(|h| self.to_screen_3d(rect, h.2))
            {
                let text = format!(
                    "{} {} → {}",
                    g.key.label(),
                    crate::units::fmt_len(g.before),
                    crate::units::fmt_len(g.now)
                );
                super::handles::tag(painter, p, text, ACCENT);
            }
        }
    }
}
