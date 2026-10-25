//! Arrows and rings on the volumes of Concepto in the 3D view (MQ4, C5):
//! with one volume selected, blue arrows on its four sides push them out
//! or in (the opposite side stays; with Alt, about its middle), the arrow
//! on top raises or lowers it, the square on a top corner scales it whole,
//! and the orange ring turns it about its middle. While dragged it is
//! drawn as it will be.

use archisim_core::{MassId, MassShape, Model, ModelError, Point2};
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect};

use super::*;

/// What a handle of a volume changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassKey {
    /// A side, along the plan's x (0) or y (1), on its high side or low.
    Side(usize, bool),
    Height,
    /// The corner square: the whole volume, about its middle and base.
    Scale,
    Turn,
}

impl MassKey {
    fn label(self) -> &'static str {
        match self {
            MassKey::Side(0, _) => "Ancho",
            MassKey::Side(..) => "Fondo",
            MassKey::Height => "Altura",
            MassKey::Scale => "Escala",
            MassKey::Turn => "Giro",
        }
    }

    /// The way the arrow points.
    fn dir(self) -> glam::DVec3 {
        match self {
            MassKey::Side(0, true) => glam::DVec3::X,
            MassKey::Side(0, false) => glam::DVec3::NEG_X,
            MassKey::Side(_, true) => glam::DVec3::Y,
            MassKey::Side(_, false) => glam::DVec3::NEG_Y,
            _ => glam::DVec3::Z,
        }
    }
}

/// A volume's handle being dragged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassGrip3d {
    pub mass: MassId,
    pub key: MassKey,
    /// Where the handle was.
    pub start: glam::DVec3,
    /// The measurement when it was taken (size, height, scale 1, angle 0)
    /// and now.
    pub before: f64,
    pub now: f64,
    /// Alt: a side moves about the middle.
    pub both: bool,
    /// Where on the screen it was taken.
    pub press: Pos2,
}

/// How near a handle the cursor takes it, in screen points.
const REACH: f32 = 11.0;
const ARROW_GAP: f32 = 6.0;
const ARROW_GRAB: f32 = ARROW_GAP + 9.0;
const TURN_STEP: f64 = std::f64::consts::PI / 12.0;

/// The box of a volume: its plan corners and its base and top heights.
pub fn mass_box(model: &Model, id: MassId) -> Option<([f64; 2], [f64; 2], f64, f64)> {
    let m = model.mass(id)?;
    let body = archisim_geom::forms::mass_body(m);
    let base = model.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    let mut top = 0.0f64;
    for p in &body.positions {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        top = top.max(p[2]);
    }
    (lo[0] <= hi[0]).then_some((lo, hi, base, base + top))
}

/// The volume changed as a handle at `value` says (`before` the value
/// when it was taken).
pub fn apply_mass_grip(
    model: &mut Model,
    id: MassId,
    key: MassKey,
    before: f64,
    value: f64,
    both: bool,
) -> Result<(), ModelError> {
    let (lo, hi, _, _) = mass_box(model, id).ok_or(ModelError::UnknownElement(id.0))?;
    let c = Point2::new((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
    match key {
        MassKey::Side(axis, high) => {
            let size = hi[axis] - lo[axis];
            if size < 1e-6 || value < 0.05 {
                return Ok(());
            }
            let k = value / size;
            // What stays: the opposite side, or the middle.
            let fixed = if both {
                (lo[axis] + hi[axis]) / 2.0
            } else if high {
                lo[axis]
            } else {
                hi[axis]
            };
            model.edit_mass(id, |m| {
                m.map_points(|p| {
                    let mut q = p;
                    if axis == 0 {
                        q.x = fixed + (p.x - fixed) * k;
                    } else {
                        q.y = fixed + (p.y - fixed) * k;
                    }
                    q
                });
            })
        }
        MassKey::Height => {
            let v = value.max(0.05);
            model.edit_mass(id, |m| set_height(m, v))
        }
        MassKey::Scale => {
            let k = (value / before.max(1e-9)).max(0.05);
            model.edit_mass(id, |m| {
                m.map_points(|p| c + (p - c) * k);
                if let MassShape::Sweep { width, .. } = &mut m.shape {
                    *width *= k;
                }
                let h = m.height * k;
                set_height(m, h);
            })
        }
        MassKey::Turn => {
            let a = value - before;
            let (s, co) = a.sin_cos();
            model.edit_mass(id, |m| {
                m.map_points(|p| {
                    let d = p - c;
                    c + Point2::new(d.x * co - d.y * s, d.x * s + d.y * co)
                })
            })
        }
    }
}

/// A volume's height, its triangles' too when it is made of them; by
/// floors, a whole number of them.
fn set_height(m: &mut archisim_core::Mass, h: f64) {
    let h = match m.floor_height {
        Some(f) if f > 0.0 => ((h / f).round().max(1.0)) * f,
        _ => h,
    };
    if let MassShape::Mesh(mesh) = &mut m.shape {
        let top = mesh.top();
        if top > 1e-9 {
            let k = h / top;
            for p in &mut mesh.positions {
                p[2] *= k;
            }
        }
    }
    m.height = h;
}

impl ArchiSimApp {
    /// The volume selected alone, if it is one.
    fn single_mass(&self) -> Option<MassId> {
        if self.ed.selection.len() != 1 || self.tool != Tool::Arrow {
            return None;
        }
        match self.ed.selection.iter().next() {
            Some(Element::Mass(id)) => Some(*id),
            _ => None,
        }
    }

    /// The ring that turns a volume: its middle (at its base), radius
    /// and the angle of its knob.
    fn mass_ring(&self, id: MassId) -> Option<(glam::DVec3, f64, f64)> {
        let (lo, hi, base, _) = mass_box(self.ed.model(), id)?;
        let (cx, cy) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        let r = (hi[0] - cx).hypot(hi[1] - cy).max(0.3) + 0.4;
        Some((
            glam::DVec3::new(cx, cy, base + 0.02),
            r,
            -std::f64::consts::FRAC_PI_2,
        ))
    }

    /// The handles of the volume selected, in the model.
    pub(super) fn mass_handles_3d(&self) -> Vec<(MassId, MassKey, glam::DVec3)> {
        let Some(id) = self.single_mass() else {
            return Vec::new();
        };
        let Some((lo, hi, base, top)) = mass_box(self.ed.model(), id) else {
            return Vec::new();
        };
        let mid = [
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (base + top) / 2.0,
        ];
        let mut v = vec![
            (
                id,
                MassKey::Side(0, true),
                glam::DVec3::new(hi[0], mid[1], mid[2]),
            ),
            (
                id,
                MassKey::Side(0, false),
                glam::DVec3::new(lo[0], mid[1], mid[2]),
            ),
            (
                id,
                MassKey::Side(1, true),
                glam::DVec3::new(mid[0], hi[1], mid[2]),
            ),
            (
                id,
                MassKey::Side(1, false),
                glam::DVec3::new(mid[0], lo[1], mid[2]),
            ),
            (id, MassKey::Height, glam::DVec3::new(mid[0], mid[1], top)),
            (id, MassKey::Scale, glam::DVec3::new(hi[0], hi[1], top)),
        ];
        if let Some((c, r, a)) = self.mass_ring(id) {
            v.push((
                id,
                MassKey::Turn,
                c + glam::DVec3::new(r * a.cos(), r * a.sin(), 0.0),
            ));
        }
        v
    }

    fn screen_3d(&self, rect: Rect, p: glam::DVec3) -> Option<Pos2> {
        let size = glam::Vec2::new(rect.width(), rect.height());
        self.v3d
            .camera
            .project_to_screen(p.as_vec3(), size)
            .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
    }

    /// Where an arrow is on the screen and the way it points.
    fn mass_arrow(&self, rect: Rect, key: MassKey, at: glam::DVec3) -> Option<(Pos2, egui::Vec2)> {
        let p = self.screen_3d(rect, at)?;
        let dir = self
            .screen_3d(rect, at + key.dir() * 0.1)
            .map_or(egui::Vec2::X, |q| q - p);
        let dir = if dir.length() > 1e-3 {
            dir.normalized()
        } else {
            egui::Vec2::X
        };
        Some((p, dir))
    }

    /// Where a handle is taken on the screen.
    pub(super) fn mass_grab_point(
        &self,
        rect: Rect,
        key: MassKey,
        at: glam::DVec3,
    ) -> Option<Pos2> {
        match key {
            MassKey::Turn | MassKey::Scale => self.screen_3d(rect, at),
            _ => {
                let (p, d) = self.mass_arrow(rect, key, at)?;
                Some(p + d * ARROW_GRAB)
            }
        }
    }

    fn mass_handle_at(&self, rect: Rect, pos: Pos2) -> Option<(MassId, MassKey, glam::DVec3)> {
        self.mass_handles_3d()
            .into_iter()
            .filter_map(|h| Some((h, self.mass_grab_point(rect, h.1, h.2)?.distance(pos))))
            .filter(|(_, d)| *d <= REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    /// The measurement a handle is about.
    fn mass_measure(&self, id: MassId, key: MassKey) -> Option<f64> {
        let (lo, hi, base, top) = mass_box(self.ed.model(), id)?;
        Some(match key {
            MassKey::Side(axis, _) => hi[axis] - lo[axis],
            MassKey::Height => top - base,
            MassKey::Scale => 1.0,
            MassKey::Turn => 0.0,
        })
    }

    /// Hover, grab, drag and let go of a volume's handles. Returns whether
    /// one is taken (the camera then stays still).
    pub(super) fn mass_grips_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) -> bool {
        if self.mass_grip.is_none() {
            self.mass_grip_hover = response
                .hover_pos()
                .and_then(|p| self.mass_handle_at(rect, p))
                .map(|h| h.1);
        }
        if response.drag_started_by(PointerButton::Primary) {
            let press = ctx.input(|i| i.pointer.press_origin());
            let taken = press.and_then(|p| self.mass_handle_at(rect, p));
            self.mass_grip = taken.and_then(|(id, key, at)| {
                let before = self.mass_measure(id, key)?;
                Some(MassGrip3d {
                    mass: id,
                    key,
                    start: at,
                    before,
                    now: before,
                    both: false,
                    press: press.unwrap_or(Pos2::ZERO),
                })
            });
        }
        let Some(mut g) = self.mass_grip else {
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            g.both = ctx.input(|i| i.modifiers.alt);
            let free = ctx.input(|i| i.modifiers.shift);
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(v) = self.mass_grip_value(&g, rect, pos, free) {
                    g.now = v;
                }
            }
            self.mass_grip = Some(g);
            self.mass_preview(&g);
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            self.mass_grip = None;
            self.v3d.set_overlay(None, || None);
            if (g.now - g.before).abs() > 1e-6 {
                let what = match g.key {
                    MassKey::Turn => "Girar volumen",
                    MassKey::Scale => "Escalar volumen",
                    MassKey::Height => "Altura del volumen",
                    MassKey::Side(..) => "Empujar lado del volumen",
                };
                let r = self.ed.transact(what, |m| {
                    apply_mass_grip(m, g.mass, g.key, g.before, g.now, g.both)
                });
                if r.is_ok() {
                    self.ed.selection = [Element::Mass(g.mass)].into_iter().collect();
                }
                self.report(r, &mass_grip_text(&g));
            }
        }
        true
    }

    /// The volume as it will be, drawn in its place while dragged.
    fn mass_preview(&mut self, g: &MassGrip3d) {
        if (g.now - g.before).abs() < 1e-6 {
            self.v3d.set_overlay(None, || None);
            return;
        }
        let key = format!(
            "mass:{}:{}:{:?}:{}:{}",
            self.ed.revision(),
            g.mass.0,
            g.key,
            g.now,
            g.both
        );
        let model = self.ed.model();
        let g = *g;
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            apply_mass_grip(&mut m, g.mass, g.key, g.before, g.now, g.both).ok()?;
            let solids = archisim_geom::solid::mass_solids(&m, g.mass);
            Some(crate::view3d::scene::Scene::of_solids(&m, solids))
        });
    }

    /// The cursor on the horizontal plane at height `z`.
    fn mass_on_plane(&self, rect: Rect, pos: Pos2, z: f64) -> Option<glam::DVec3> {
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

    /// The cursor on an upright plane through `at` facing the camera.
    fn mass_on_upright(&self, rect: Rect, pos: Pos2, at: glam::DVec3) -> Option<glam::DVec3> {
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        let mut n = self.v3d.camera.forward().as_dvec3();
        n.z = 0.0;
        let n = n.normalize_or(glam::DVec3::X);
        let den = d.dot(n);
        if den.abs() < 1e-9 {
            return None;
        }
        let t = (at - o).dot(n) / den;
        Some(o + d * t)
    }

    /// The value of a handle with the cursor at `pos`.
    fn mass_grip_value(&self, g: &MassGrip3d, rect: Rect, pos: Pos2, free: bool) -> Option<f64> {
        let round = |v: f64| (v * 100.0).round() / 100.0;
        match g.key {
            MassKey::Turn => {
                let (c, r, a0) = self.mass_ring(g.mass)?;
                let q = self.mass_on_plane(rect, pos, c.z)? - c;
                let free = free || q.truncate().length() > 3.0 * r;
                let a = q.y.atan2(q.x) - a0;
                let step = if free { 1f64.to_radians() } else { TURN_STEP };
                let a = (a / step).round() * step;
                // Between −180° and 180°.
                let tau = std::f64::consts::TAU;
                Some((a + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI)
            }
            MassKey::Height => {
                let now = self.mass_on_upright(rect, pos, g.start)?.z;
                let then = self.mass_on_upright(rect, g.press, g.start)?.z;
                Some(round(g.before + now - then).max(0.05))
            }
            MassKey::Side(axis, high) => {
                let now = self.mass_on_plane(rect, pos, g.start.z)?;
                let then = self.mass_on_plane(rect, g.press, g.start.z)?;
                let d = if axis == 0 {
                    now.x - then.x
                } else {
                    now.y - then.y
                };
                let d = if high { d } else { -d };
                let k = if g.both { 2.0 } else { 1.0 };
                Some(round(g.before + k * d).max(0.05))
            }
            MassKey::Scale => {
                let (lo, hi, _, _) = mass_box(self.ed.model(), g.mass)?;
                let c = glam::DVec2::new((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
                let corner = g.start.truncate() - c;
                let r0 = corner.length().max(1e-6);
                let now = self.mass_on_plane(rect, pos, g.start.z)?.truncate() - c;
                let then = self.mass_on_plane(rect, g.press, g.start.z)?.truncate() - c;
                let k = (r0 + now.dot(corner) / r0 - then.dot(corner) / r0) / r0;
                Some(((k * 100.0).round() / 100.0).max(0.05))
            }
        }
    }

    /// The handles of the volume selected, and what changes while one is
    /// dragged.
    pub(super) fn draw_mass_grips_3d(&self, painter: &egui::Painter, rect: Rect) {
        let handles = self.mass_handles_3d();
        for (id, key, at) in &handles {
            let hot =
                self.mass_grip_hover == Some(*key) || self.mass_grip.is_some_and(|g| g.key == *key);
            match key {
                MassKey::Turn => {
                    let Some((c, r, a0)) = self.mass_ring(*id) else {
                        continue;
                    };
                    let span = if hot { std::f64::consts::TAU } else { 1.0 };
                    let pts: Vec<Pos2> = (0..=48)
                        .filter_map(|k| {
                            let a = a0 - span / 2.0 + span * k as f64 / 48.0;
                            self.screen_3d(
                                rect,
                                c + glam::DVec3::new(r * a.cos(), r * a.sin(), 0.0),
                            )
                        })
                        .collect();
                    let ring = super::handles::TURN.gamma_multiply(if hot { 0.85 } else { 0.6 });
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.8, ring)));
                    if let Some(kp) = self.screen_3d(rect, *at) {
                        painter.circle(
                            kp,
                            if hot { 7.0 } else { 5.5 },
                            super::handles::TURN,
                            egui::Stroke::new(2.0, Color32::WHITE),
                        );
                    }
                }
                MassKey::Scale => {
                    if let Some(p) = self.screen_3d(rect, *at) {
                        let s = if hot { 6.5 } else { 5.0 };
                        let r = Rect::from_center_size(p, egui::vec2(2.0 * s, 2.0 * s));
                        painter.rect(
                            r,
                            1.5,
                            ACCENT,
                            egui::Stroke::new(2.0, Color32::WHITE),
                            egui::StrokeKind::Middle,
                        );
                    }
                }
                _ => {
                    let Some((p, dir)) = self.mass_arrow(rect, *key, *at) else {
                        continue;
                    };
                    super::handles::arrow(painter, p + dir * (ARROW_GAP + 3.0), dir, ACCENT, hot);
                }
            }
        }
        if let Some(g) = self.mass_grip {
            if let Some(p) = painter.ctx().pointer_latest_pos() {
                let fill = if g.key == MassKey::Turn {
                    super::handles::TURN
                } else {
                    ACCENT
                };
                super::handles::tag(painter, p, mass_grip_text(&g), fill);
            }
        }
    }
}

/// What a handle is doing, in words.
fn mass_grip_text(g: &MassGrip3d) -> String {
    match g.key {
        MassKey::Turn => format!("Giro {:.0}°", g.now.to_degrees()),
        MassKey::Scale => format!("Escala {:.0} %", g.now * 100.0),
        _ => format!(
            "{} {} → {}{}",
            g.key.label(),
            crate::units::fmt_len(g.before),
            crate::units::fmt_len(g.now),
            if g.both { " · por los dos lados" } else { "" }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewMass};

    /// A 4 × 3 m box, 3 m high, from the origin.
    fn block() -> (Model, MassId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Masa", "concrete", [200, 200, 200], CutPattern::Solid);
        let id = m
            .add_mass(NewMass {
                name: "Volumen 1".into(),
                level: l,
                shape: MassShape::Prism(vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                    Point2::new(0.0, 3.0),
                ]),
                base_offset: 0.0,
                height: 3.0,
                floor_height: None,
                material: mat,
            })
            .unwrap();
        (m, id)
    }

    fn size(m: &Model, id: MassId) -> ([f64; 2], [f64; 2], f64) {
        let (lo, hi, base, top) = mass_box(m, id).unwrap();
        (lo, hi, top - base)
    }

    #[test]
    fn sides_height_scale_and_turn() {
        let (m0, id) = block();
        // The east side out to 6 m: the west one stays.
        let mut m = m0.clone();
        apply_mass_grip(&mut m, id, MassKey::Side(0, true), 4.0, 6.0, false).unwrap();
        assert_eq!(size(&m, id), ([0.0, 0.0], [6.0, 3.0], 3.0));
        // The south side, about the middle (Alt): 5 m deep, centred.
        let mut m = m0.clone();
        apply_mass_grip(&mut m, id, MassKey::Side(1, false), 3.0, 5.0, true).unwrap();
        assert_eq!(size(&m, id), ([0.0, -1.0], [4.0, 4.0], 3.0));
        // Up to 7,5 m.
        let mut m = m0.clone();
        apply_mass_grip(&mut m, id, MassKey::Height, 3.0, 7.5, false).unwrap();
        assert!((size(&m, id).2 - 7.5).abs() < 1e-9);
        // Twice its size, about its middle, height too.
        let mut m = m0.clone();
        apply_mass_grip(&mut m, id, MassKey::Scale, 1.0, 2.0, false).unwrap();
        assert_eq!(size(&m, id), ([-2.0, -1.5], [6.0, 4.5], 6.0));
        // A quarter turn about its middle: 3 × 4.
        let mut m = m0;
        apply_mass_grip(
            &mut m,
            id,
            MassKey::Turn,
            0.0,
            std::f64::consts::FRAC_PI_2,
            false,
        )
        .unwrap();
        let (lo, hi, _) = size(&m, id);
        assert!((hi[0] - lo[0] - 3.0).abs() < 1e-9 && (hi[1] - lo[1] - 4.0).abs() < 1e-9);
        assert!(((lo[0] + hi[0]) / 2.0 - 2.0).abs() < 1e-9);
    }
}
