//! Stretching furniture in the 3D view (bloque 5 de la revisión del
//! 22/10/2026): with one object selected, blue handles on its right and
//! left sides, its front and its top change its width, depth and height
//! as the mouse drags them, as the plan's handles do (a type shared with
//! other objects is copied, so only this one changes).

use archisim_core::{ObjectId, Value};
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect, Stroke};

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
}

/// How near a handle the cursor takes it, in screen points.
const REACH: f32 = 11.0;

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
        let model = self.ed.model();
        let t = model.object_type(model.object(id)?.object_type)?;
        if let Some(Value::Number(v)) = t.values.get(key) {
            return Some(*v);
        }
        let d = model
            .family_defaults(t.family, archisim_family::ParamScope::Type)
            .ok()?;
        match d.get(key) {
            Some(Value::Number(v)) => Some(*v),
            _ => None,
        }
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

    /// The handle under a point of the screen.
    fn object_handle_at(&self, rect: Rect, pos: Pos2) -> Option<(ObjectId, SizeKey, glam::DVec3)> {
        self.object_handles_3d()
            .into_iter()
            .filter_map(|h| self.to_screen_3d(rect, h.2).map(|s| (h, s.distance(pos))))
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
            let taken = ctx
                .input(|i| i.pointer.press_origin())
                .and_then(|p| self.object_handle_at(rect, p));
            self.object_grip = taken.and_then(|(id, key, at)| {
                let before = self.object_size(id, key.key())?;
                Some(ObjectGrip3d {
                    object: id,
                    key,
                    start: at,
                    before,
                    now: before,
                })
            });
        }
        let Some(mut g) = self.object_grip else {
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
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
                let r = self.ed.transact("Estirar objeto", |m| {
                    crate::drag::set_object_size(m, g.object, g.key.key(), g.now)
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
    fn grip_value(&self, g: &ObjectGrip3d, rect: Rect, pos: Pos2) -> Option<f64> {
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
                match g.key {
                    SizeKey::Width(true) => 2.0 * x,
                    SizeKey::Width(false) => -2.0 * x,
                    _ => y,
                }
            }
        };
        let v = ((g.before + delta) * 100.0).round() / 100.0;
        Some(v.max(0.05))
    }

    /// The handles, and the new size while one is dragged.
    pub(super) fn draw_object_grips_3d(&self, painter: &egui::Painter, rect: Rect) {
        let handles = self.object_handles_3d();
        for (_, key, at) in &handles {
            let Some(p) = self.to_screen_3d(rect, *at) else {
                continue;
            };
            let hot = self.object_grip_hover == Some(*key)
                || self.object_grip.is_some_and(|g| g.key == *key);
            let r = if hot { 6.5 } else { 5.0 };
            painter.rect_filled(
                Rect::from_center_size(p, egui::vec2(2.0 * r, 2.0 * r)),
                1.5,
                ACCENT,
            );
            painter.rect_stroke(
                Rect::from_center_size(p, egui::vec2(2.0 * r, 2.0 * r)),
                1.5,
                Stroke::new(1.5, Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }
        if let Some(g) = self.object_grip {
            if let Some(p) = handles
                .iter()
                .find(|h| h.1 == g.key)
                .and_then(|h| self.to_screen_3d(rect, h.2))
            {
                let text = format!("{} {}", g.key.label(), crate::units::fmt_len(g.now));
                let font = egui::FontId::proportional(12.0);
                let galley = painter.layout_no_wrap(text, font, Color32::WHITE);
                let r = Rect::from_min_size(p + egui::vec2(12.0, -26.0), galley.size()).expand(4.0);
                painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(20, 24, 32, 220));
                painter.galley(r.min + egui::vec2(4.0, 4.0), galley, Color32::WHITE);
            }
        }
    }
}
