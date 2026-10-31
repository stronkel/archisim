//! Placing as in Paralives (revisión de Antonio del 23/10/2026), the
//! interface (the rules are in `crate::placing`): small
//! things (decoration, table lamps, plants) rest by themselves on the
//! table, shelf, worktop or sill under the cursor; furniture is dragged
//! along the floor in the 3D view and its back sticks to the wall it comes
//! near; pieces that are made to be joined (worktops, wall units,
//! bookcases, sideboards, sofas) become one when one is placed against
//! another.

use archisim_core::{ObjectTypeId, Point2};
use eframe::egui::{self, Pos2, Rect};

use super::*;
use crate::view3d::scene::Owner;

/// The largest side in plan of what rests on surfaces by itself.
const SMALL: f64 = 0.7;

impl ArchiSimApp {
    /// Whether objects of a type rest on what is under them (decoration,
    /// table lamps, plants and anything small), not on the floor.
    pub(super) fn rests_on_surfaces(&self, t: ObjectTypeId) -> bool {
        let m = self.ed.model();
        let Some(ty) = m.object_type(t) else {
            return false;
        };
        let folder = m.family(ty.family).map(|f| f.def().folder.as_str());
        if matches!(folder, Some("Decoración" | "Plantas de interior")) {
            return true;
        }
        let Some((lo, hi)) = m.object_type_box_3d(t) else {
            return false;
        };
        let side = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let tall = hi[2] - lo[2];
        side <= SMALL
            && tall <= 1.0
            && !matches!(
                folder,
                Some("Sofás y sillones" | "Sillas" | "Taburetes" | "Jardín y terraza")
            )
    }

    /// Height over the level an object placed at `p` stands on: the top of
    /// the surface under the cursor for small things (in 3D, what the
    /// cursor touches; in plan, the highest furniture under the point),
    /// and the floor for the rest.
    pub(super) fn surface_under(&self, p: Point2) -> f64 {
        let Some(t) = self.object_settings.object_type else {
            return 0.0;
        };
        if !self.rests_on_surfaces(t) {
            return 0.0;
        }
        if self.mode == ViewMode::ThreeD {
            if let Some(c) = &self.cursor3d {
                if let (Some(Owner::Model(Element::Object(_))), Some(n)) = (c.owner, c.normal) {
                    if n.z > 0.7 {
                        return (c.world.z - self.level_z_3d()).max(0.0) as f64;
                    }
                }
            }
            return 0.0;
        }
        crate::placing::plan_surface_at(self.ed.model(), self.ed.level, p, None).unwrap_or(0.0)
    }

    /// Where an object placed with the tool at `p` goes: small things on
    /// the surface under them; in the Maqueta, furniture near a wall with
    /// its back on it, facing the room. Position, rotation and height over
    /// the level.
    pub(super) fn placement_spot(&self, p: Point2) -> (Point2, f64, f64) {
        let s = &self.object_settings;
        let surface = self.surface_under(p);
        if surface > 0.0 {
            return (p, s.rotation, s.base_offset + surface);
        }
        let small = s.object_type.is_some_and(|t| self.rests_on_surfaces(t));
        let (mut at, mut rot) = (p, s.rotation);
        if self.maquette && !small {
            if let Some((q, r)) = crate::placing::against_wall(self.ed.model(), self.ed.level, p) {
                (at, rot) = (q, r);
            }
        }
        // Beside another piece that joins: touching it, to become one.
        if let Some(t) = s.object_type {
            if let Some(q) =
                crate::placing::snap_alongside(self.ed.model(), self.ed.level, t, at, rot, None)
            {
                at = q;
            }
        }
        (at, rot, s.base_offset)
    }

    fn level_z_3d(&self) -> f32 {
        self.ed
            .model()
            .level(self.ed.level)
            .map_or(0.0, |l| l.elevation) as f32
    }
}

/// A piece of furniture dragged along the floor in the Maqueta's 3D view:
/// what it is, where it was taken from (its position minus the point of
/// the floor under the mouse) and where it would go now (position,
/// rotation and height over the level).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectMove3d {
    pub object: archisim_core::ObjectId,
    grab: archisim_core::Vec2,
    pub now: (Point2, f64, f64),
    /// Its back is against a wall.
    pub on_wall: bool,
}

impl ArchiSimApp {
    /// The point of the floor (at height `z` over the level) under a
    /// point of the 3D view.
    fn floor_at(&self, rect: Rect, pos: Pos2, z: f64) -> Option<Point2> {
        let local = pos - rect.min;
        let ray = self.v3d.camera.ray(
            glam::Vec2::new(local.x, local.y),
            glam::Vec2::new(rect.width(), rect.height()),
        );
        let level_z = self.level_z_3d() as f64 + z;
        let (o, d) = (ray.origin.as_dvec3(), ray.dir.as_dvec3());
        if d.z.abs() < 1e-9 {
            return None;
        }
        let t = (level_z - o.z) / d.z;
        (t > 0.0).then(|| {
            let q = o + d * t;
            Point2::new(q.x, q.y)
        })
    }

    /// Dragging furniture (in both environments), with nothing in hand: it slides
    /// along the floor as it will be, its back goes against a wall that it
    /// comes near (turned to face the room), small things rest on what is
    /// under them, and on letting go it stays there (joining another of
    /// its kind beside it). Returns whether one is being dragged (the
    /// camera then stays still).
    pub(super) fn object_drag_3d(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) -> bool {
        if self.tool != Tool::Arrow || self.object_grip.is_some() {
            self.object_move = None;
            return false;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let taken = ctx.input(|i| i.pointer.press_origin()).and_then(|p| {
                let local = p - rect.min;
                match self.v3d.pick(local, rect.size())? {
                    Owner::Model(Element::Object(id)) => Some((id, p)),
                    _ => None,
                }
            });
            self.object_move = taken.and_then(|(id, p)| {
                let o = self.ed.model().object(id)?.clone();
                let floor = self.floor_at(rect, p, 0.0)?;
                Some(ObjectMove3d {
                    object: id,
                    grab: o.position - floor,
                    now: (o.position, o.rotation, o.base_offset),
                    on_wall: false,
                })
            });
            if let Some(mv) = self.object_move {
                self.ed.selection = [Element::Object(mv.object)].into_iter().collect();
                let z = self.level_z_3d() as f64 + mv.now.2 + 0.01;
                self.take_for_move(z);
            }
        }
        let Some(mut mv) = self.object_move else {
            return false;
        };
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(floor) = self.floor_at(rect, pos, 0.0) {
                    mv.now = self.object_spot(&mv, floor + mv.grab);
                    mv.now.0 = self.object_near_walls(&mv);
                    mv.on_wall = {
                        let m = self.ed.model();
                        m.object(mv.object).is_some_and(|o| {
                            !self.rests_on_surfaces(o.object_type)
                                && crate::placing::against_wall(m, o.level, mv.now.0).is_some()
                        })
                    };
                    self.object_move = Some(mv);
                }
            }
            self.object_move_preview(&mv);
            ctx.request_repaint();
        }
        if response.drag_stopped() {
            self.object_move = None;
            self.move_clear.clear();
            self.v3d.set_overlay(None, || None);
            let (at, rotation, base) = mv.now;
            let id = mv.object;
            let r = self.ed.transact("Mover objeto", |m| {
                m.edit_object(id, |o| {
                    o.position = at;
                    o.rotation = rotation;
                    o.base_offset = base;
                })?;
                Ok(crate::placing::join_alongside(m, id).unwrap_or(id))
            });
            match r {
                Ok(kept) => {
                    self.ed.selection = [Element::Object(kept)].into_iter().collect();
                    if kept != id {
                        self.say("Unido al de al lado: ahora son una sola pieza.");
                    } else if mv.on_wall {
                        self.say("Contra la pared.");
                    }
                }
                Err(e) => self.complain(format!("No se puede mover: {e}.")),
            }
        }
        true
    }

    /// Where a dragged object goes with its point at `p`: against a wall
    /// near it (furniture), on the surface under it (small things), or on
    /// the grid.
    fn object_spot(&self, mv: &ObjectMove3d, p: Point2) -> (Point2, f64, f64) {
        let m = self.ed.model();
        let Some(o) = m.object(mv.object) else {
            return mv.now;
        };
        if self.rests_on_surfaces(o.object_type) {
            let top = crate::placing::plan_surface_at(m, o.level, p, Some(o.id)).unwrap_or(0.0);
            return (p, o.rotation, top);
        }
        let (at, rot) = match crate::placing::against_wall(m, o.level, p) {
            Some(x) => x,
            None if self.ground_grid => (super::maquette_ui::snap_to_grid(p), o.rotation),
            None => (p, o.rotation),
        };
        let at = crate::placing::snap_alongside(m, o.level, o.object_type, at, rot, Some(o.id))
            .unwrap_or(at);
        (at, rot, o.base_offset)
    }

    /// Where a dragged object goes once pulled to the faces of the walls
    /// near it (its sides into a corner, as well as its back against the
    /// wall); keeps its clear distances to draw them. Small things that
    /// rest on others are not pulled.
    fn object_near_walls(&mut self, mv: &ObjectMove3d) -> Point2 {
        let (at, rot, _) = mv.now;
        let m = self.ed.model();
        let Some(o) = m.object(mv.object) else {
            return at;
        };
        if self.rests_on_surfaces(o.object_type) || self.move_points.is_empty() {
            self.move_clear.clear();
            return at;
        }
        // Its outline where it would go: as it is now, moved and turned.
        let (c0, s0) = (o.rotation.cos(), o.rotation.sin());
        let (c1, s1) = (rot.cos(), rot.sin());
        let pts: Vec<Point2> = self
            .move_points
            .iter()
            .map(|p| {
                let d = *p - o.position;
                let local = Point2::new(d.x * c0 + d.y * s0, -d.x * s0 + d.y * c0);
                at + Point2::new(local.x * c1 - local.y * s1, local.x * s1 + local.y * c1)
            })
            .collect();
        let mut d = crate::wall_faces::snap_delta(&self.move_faces, &pts, Point2::new(0.0, 0.0));
        // Not pulled by a wall: its corners, middles and centre to those of
        // the objects near it (E15).
        if d.length() < 1e-9 {
            d = object_points_delta(m, o.id, o.level, &pts);
        }
        let moved: Vec<Point2> = pts.iter().map(|p| *p + d).collect();
        self.move_clear =
            crate::wall_faces::clearances(&self.move_faces, &moved, Point2::new(c1, s1));
        at + d
    }

    /// The object drawn where it would go, while dragged.
    fn object_move_preview(&mut self, mv: &ObjectMove3d) {
        let (at, rotation, base) = mv.now;
        let key = format!(
            "move:{}:{}:{:.3}:{:.3}:{:.4}:{:.3}",
            self.ed.revision(),
            mv.object.0,
            at.x,
            at.y,
            rotation,
            base
        );
        let model = self.ed.model();
        let id = mv.object;
        self.v3d.set_overlay(Some(key), || {
            let mut m = model.clone();
            m.edit_object(id, |o| {
                o.position = at;
                o.rotation = rotation;
                o.base_offset = base;
            })
            .ok()?;
            let solids = archisim_geom::solid::object_solids(&m, id);
            Some(crate::view3d::scene::Scene::of_solids(&m, solids))
        });
    }
}

/// How far to move an object whose box has the points `pts` so that one
/// of them (or its centre) falls on a corner, the middle of a side or the
/// centre of the box of another object near it (within 5 cm); none if
/// none is that near (E15, in 3D).
fn object_points_delta(
    m: &archisim_core::Model,
    own: archisim_core::ObjectId,
    level: archisim_core::LevelId,
    pts: &[Point2],
) -> Point2 {
    const REACH: f64 = 0.05;
    if pts.is_empty() {
        return Point2::new(0.0, 0.0);
    }
    let centre = pts.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / pts.len() as f64);
    let mut mine: Vec<Point2> = pts.to_vec();
    let n = pts.len();
    for i in 0..n {
        mine.push((pts[i] + pts[(i + 1) % n]) * 0.5);
    }
    mine.push(centre);
    let mut best: Option<(f64, Point2)> = None;
    for o in m.objects_on_level(level) {
        if o.id == own || o.position.distance(centre) > 6.0 || m.plant_look(o.id).is_some() {
            continue;
        }
        let outline = m.object_outline(o.id);
        if outline.len() < 3 {
            continue;
        }
        let k = outline.len();
        let mut theirs: Vec<Point2> = outline.clone();
        for i in 0..k {
            theirs.push((outline[i] + outline[(i + 1) % k]) * 0.5);
        }
        if let Some(c) = crate::drag::object_middle(m, o.id) {
            theirs.push(c);
        }
        for a in &mine {
            for b in &theirs {
                let dist = a.distance(*b);
                if dist < REACH && best.is_none_or(|(d, _)| dist < d) {
                    best = Some((dist, *b - *a));
                }
            }
        }
    }
    best.map_or(Point2::new(0.0, 0.0), |(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// E15 in 3D: a dragged object's corner is pulled to the corner of
    /// one near it.
    #[test]
    fn a_moved_object_falls_on_the_corner_of_another() {
        let mut m = archisim_core::Model::new("x");
        let level = m.add_level("PB", 0.0);
        archisim_library::add_library(&mut m).unwrap();
        let t = m
            .object_types()
            .find(|t| !m.type_grows(t.id) && t.name.contains("Mesa"))
            .map(|t| t.id)
            .unwrap();
        let place = |m: &mut archisim_core::Model, x: f64| {
            m.add_object(archisim_core::NewObject {
                object_type: t,
                level,
                position: Point2::new(x, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap()
        };
        let a = place(&mut m, 0.0);
        let b = place(&mut m, 3.0);
        let theirs = m.object_outline(b);
        let mine = m.object_outline(a);
        // A's box with its right side 3 cm short of B's left side.
        let gap = theirs[0].x - mine[1].x - 0.03;
        let pts: Vec<Point2> = mine.iter().map(|p| *p + Point2::new(gap, 0.0)).collect();
        let d = object_points_delta(&m, a, level, &pts);
        assert!((d.x - 0.03).abs() < 1e-6 && d.y.abs() < 1e-6, "{d:?}");
        // Far from it, nothing.
        let far: Vec<Point2> = mine.iter().map(|p| *p + Point2::new(0.5, 0.0)).collect();
        assert_eq!(
            object_points_delta(&m, a, level, &far),
            Point2::new(0.0, 0.0)
        );
    }
}
