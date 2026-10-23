//! Stretching furniture in the 3D view (bloque 5 de la revisión del
//! 22/10/2026): with one object selected, blue handles on its right and
//! left sides, its front and its top change its width, depth and height
//! as the mouse drags them, as the plan's handles do (a type shared with
//! other objects is copied, so only this one changes). While a handle is
//! dragged the object is drawn as it will be (`stretch_preview`): it
//! grows as the mouse moves, not only when it is let go.
//!
//! A ring on the floor around it turns it to any angle: in steps of 15°,
//! or freely with Mayús.

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
    /// The knob of the ring that turns it.
    Turn,
}

/// The steps a turn snaps to (none with Mayús).
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
    /// Mayús held: a turn is not snapped.
    pub free: bool,
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
    /// radius and the angle of its knob (in front of the object).
    pub(super) fn turn_ring(&self, id: ObjectId) -> Option<(glam::DVec3, f64, f64)> {
        let model = self.ed.model();
        let o = model.object(id)?;
        let (lo, hi) = model.object_type_box_3d(o.object_type)?;
        let base = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        let r = [lo[0], hi[0]]
            .iter()
            .flat_map(|x| [lo[1], hi[1]].map(|y| x.hypot(y)))
            .fold(0.0_f64, f64::max)
            + 0.25;
        let angle = o.rotation - std::f64::consts::FRAC_PI_2;
        Some((
            glam::DVec3::new(o.position.x, o.position.y, base + 0.01),
            r,
            angle,
        ))
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
                    free: false,
                })
            });
        }
        let Some(mut g) = self.object_grip else {
            if self.object_move.is_none() {
                self.v3d.set_overlay(None, || None);
            }
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            g.free = ctx.input(|i| i.modifiers.shift);
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(v) = self.grip_value(&g, rect, pos) {
                    g.now = v;
                    self.object_grip = Some(g);
                }
            }
            self.stretch_preview(&g);
            // The picture with the new shape is drawn in the next frame.
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            self.object_grip = None;
            self.v3d.set_overlay(None, || None);
            if (g.now - g.before).abs() > 1e-6 && g.key == SizeKey::Turn {
                let r = self.ed.transact("Girar objeto", |m| turn_object(m, &g));
                if r.is_ok() {
                    self.ed.selection = [Element::Object(g.object)].into_iter().collect();
                }
                self.report(r.map(|_| ()), &format!("Giro {}.", degrees(g.now)));
            } else if (g.now - g.before).abs() > 1e-6 {
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

    /// The object as it will be, drawn in its place while the handle is
    /// dragged: built on a copy of the model with the same change the
    /// release makes, so it grows (cushions, legs, doors…) as it will.
    pub(super) fn stretch_preview(&mut self, g: &ObjectGrip3d) {
        if (g.now - g.before).abs() < 1e-6 {
            self.v3d.set_overlay(None, || None);
            return;
        }
        let key = format!(
            "{}:{}:{:?}:{}",
            self.ed.revision(),
            g.object.0,
            g.key,
            g.now
        );
        let model = self.ed.model();
        let g = *g;
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            if g.key == SizeKey::Turn {
                turn_object(&mut m, &g).ok()?;
            } else {
                crate::drag::set_object_size(&mut m, g.object, g.key.key(), g.now).ok()?;
            }
            let solids = archisim_geom::solid::object_solids(&m, g.object);
            Some(crate::view3d::scene::Scene::of_solids(&m, solids))
        });
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
        if g.key == SizeKey::Turn {
            // The cursor on the floor, about the object's middle.
            if d.z.abs() < 1e-9 {
                return None;
            }
            let t = (g.start.z - o.z) / d.z;
            if t <= 0.0 {
                return None;
            }
            let q = o + d * t;
            let mid = glam::DVec3::new(ob.position.x, ob.position.y, g.start.z);
            let a0 = (g.start.y - mid.y).atan2(g.start.x - mid.x);
            let a = (q.y - mid.y).atan2(q.x - mid.x);
            let mut v = g.before + (a - a0);
            if !g.free {
                v = (v / TURN_STEP).round() * TURN_STEP;
            }
            return Some(v);
        }
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
        // The ring that turns it, on the floor.
        if let Some((id, ..)) = handles.iter().find(|h| h.1 == SizeKey::Turn) {
            if let Some((c, r, _)) = self.turn_ring(*id) {
                let hot = self.object_grip_hover == Some(SizeKey::Turn)
                    || self.object_grip.is_some_and(|g| g.key == SizeKey::Turn);
                let pts: Vec<Pos2> = (0..=64)
                    .filter_map(|k| {
                        let a = k as f64 / 64.0 * std::f64::consts::TAU;
                        self.to_screen_3d(rect, c + glam::DVec3::new(r * a.cos(), r * a.sin(), 0.0))
                    })
                    .collect();
                let colour = if hot {
                    ACCENT
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 170)
                };
                painter.add(egui::Shape::line(
                    pts,
                    Stroke::new(if hot { 2.5 } else { 1.5 }, colour),
                ));
            }
        }
        // The handle taken goes with the cursor, as the object grows.
        let held = self
            .object_grip
            .and_then(|_| painter.ctx().pointer_latest_pos());
        for (_, key, at) in &handles {
            let taken = self.object_grip.is_some_and(|g| g.key == *key);
            let Some(p) = held
                .filter(|_| taken)
                .or_else(|| self.to_screen_3d(rect, *at))
            else {
                continue;
            };
            let hot = self.object_grip_hover == Some(*key)
                || self.object_grip.is_some_and(|g| g.key == *key);
            let r = if hot { 6.5 } else { 5.0 };
            if *key == SizeKey::Turn {
                painter.circle_filled(p, r + 1.0, ACCENT);
                painter.circle_stroke(p, r + 1.0, Stroke::new(1.5, Color32::WHITE));
                continue;
            }
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
            if let Some(p) = held.or_else(|| {
                handles
                    .iter()
                    .find(|h| h.1 == g.key)
                    .and_then(|h| self.to_screen_3d(rect, h.2))
            }) {
                let text = match g.key {
                    SizeKey::Turn => format!(
                        "Giro {}{}",
                        degrees(g.now),
                        if g.free { "" } else { " · Mayús: libre" }
                    ),
                    _ => format!("{} {}", g.key.label(), crate::units::fmt_len(g.now)),
                };
                let font = egui::FontId::proportional(12.0);
                let galley = painter.layout_no_wrap(text, font, Color32::WHITE);
                let r = Rect::from_min_size(p + egui::vec2(12.0, -26.0), galley.size()).expand(4.0);
                painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(20, 24, 32, 220));
                painter.galley(r.min + egui::vec2(4.0, 4.0), galley, Color32::WHITE);
            }
        }
    }
}

/// Turns an object to the grip's angle, about its insertion point.
fn turn_object(
    m: &mut archisim_core::Model,
    g: &ObjectGrip3d,
) -> Result<std::collections::BTreeSet<Element>, archisim_core::ModelError> {
    let at = m
        .object(g.object)
        .ok_or(archisim_core::ModelError::UnknownObject(g.object))?
        .position;
    let t = archisim_draft::Transform::rotate(at, g.now - g.before);
    let items = [Element::Object(g.object)].into_iter().collect();
    crate::xform::transform(m, &items, &t, false)
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
