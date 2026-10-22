//! Pintar (R18–R20): a material for the face clicked, as in SketchUp. In
//! 3D, the face under the cursor: a side of a wall (or, when edges drawn
//! on it close a zone there, only that zone) or one face of a volume; in
//! plan, the side of the wall clicked or the top of a volume. «Quitar
//! pintura» takes it off, leaving the element's own material in sight.

use archisim_core::{FaceRef, FaceSide, MaterialId, Point2, WallId};
use archisim_draft::Seg;

use super::*;
use crate::view3d::scene::Owner;

/// How near the face of a wall an edge must lie to divide it, metres.
const ON_FACE: f64 = 0.03;
/// How near the border of a face an edge's end meets it, metres.
const NEAR_BORDER: f64 = 0.15;

impl ArchiSimApp {
    /// A click of Pintar at `p` (in plan; in 3D, with the face under the
    /// cursor).
    pub(super) fn paint_click(&mut self, p: Point2) {
        let erase = self.paint_erase;
        let material = (!erase).then_some(self.mass_settings.material);
        let in_3d = self.mode == ViewMode::ThreeD;
        let hit = self
            .cursor3d
            .filter(|_| in_3d)
            .and_then(|c| Some((c.owner?, c.world, c.normal?)));
        match hit {
            Some((Owner::Model(Element::Wall(w)), world, n)) => {
                let probe = p + archisim_core::Vec2::new(n.x as f64, n.y as f64) * 0.05;
                let side = self.wall_side(w, probe);
                self.paint_wall(w, side, Some(world), material);
            }
            Some((Owner::Model(Element::Mass(id)), world, n)) => {
                let face = archisim_geom::solid::face_of(
                    self.ed.model(),
                    archisim_geom::solid::SolidOwner::Mass(id),
                    [world.x as f64, world.y as f64, world.z as f64],
                    [n.x as f64, n.y as f64, n.z as f64],
                );
                match face {
                    Some(f) => self.paint_face(id.0, f, material),
                    None => self.complain("Esa cara no se puede pintar."),
                }
            }
            Some(_) if in_3d => {
                self.complain("Se pintan las caras de los muros y de los volúmenes.");
            }
            _ => {
                // In plan: the top of a volume, or the side of a wall.
                // The face looking up under the click (a part of a divided
                // top, too).
                if let Some(pick) = self.face_under(p, false) {
                    let face = self
                        .ed
                        .model()
                        .mass(pick.mass)
                        .map_or(FaceRef::Top, |m| m.face_ref(pick.face));
                    self.paint_face(pick.mass.0, face, material);
                    return;
                }
                if let Some(id) = self.mass_under(p) {
                    self.paint_face(id.0, FaceRef::Top, material);
                    return;
                }
                let reach = self.view.to_metres(REACH);
                match crate::pick::pick(&self.ed, self.ed.level, p, reach) {
                    Some(Element::Wall(w)) => {
                        let side = self.wall_side(w, p);
                        self.paint_wall(w, side, None, material);
                    }
                    _ => self.complain(
                        "Haz clic en la cara de un muro o de un volumen (mejor en 3D) para pintarla.",
                    ),
                }
            }
        }
    }

    fn paint_face(&mut self, element: u64, face: FaceRef, material: Option<MaterialId>) {
        let label = if material.is_some() {
            "Pintar cara"
        } else {
            "Quitar pintura"
        };
        let r = self
            .ed
            .transact(label, |m| m.set_face_paint(element, face, material));
        self.report(
            r,
            if material.is_some() {
                "Cara pintada."
            } else {
                "Pintura quitada: se ve el material del elemento."
            },
        );
    }

    /// Paints a side of a wall: the zone around the point clicked, if
    /// edges drawn on that face close one there; else the whole side.
    fn paint_wall(
        &mut self,
        w: WallId,
        side: FaceSide,
        at: Option<glam::Vec3>,
        material: Option<MaterialId>,
    ) {
        let zone = at.and_then(|a| self.wall_zone(w, side, a));
        let name = match side {
            FaceSide::Exterior => "exterior",
            FaceSide::Interior => "interior",
        };
        match zone {
            Some(outline) => {
                let r = self.ed.transact(
                    if material.is_some() {
                        "Pintar zona"
                    } else {
                        "Quitar pintura"
                    },
                    |m| m.set_paint_zone(w, side, outline, material),
                );
                self.report(
                    r,
                    if material.is_some() {
                        "Zona de la fachada pintada (entre las aristas dibujadas en ella)."
                    } else {
                        "Pintura de la zona quitada."
                    },
                );
            }
            None => {
                let r = self.ed.transact(
                    if material.is_some() {
                        "Pintar cara"
                    } else {
                        "Quitar pintura"
                    },
                    |m| {
                        if material.is_none() {
                            m.clear_paint_zones(w, side);
                        }
                        m.set_face_paint(w.0, FaceRef::Wall(side), material)
                    },
                );
                let text = if material.is_some() {
                    format!("Cara {name} del muro pintada.")
                } else {
                    format!("Pintura de la cara {name} quitada: se ve el material del muro.")
                };
                self.report(r, &text);
            }
        }
    }

    /// Where a point of a wall's face is, in metres along the wall from its
    /// start and up from the level.
    fn on_wall_face(&self, w: WallId, at: (Point2, f64)) -> Option<Point2> {
        let model = self.ed.model();
        let wall = model.wall(w)?;
        let z0 = model.level(wall.level)?.elevation;
        let len = wall.length();
        let (p, z) = at;
        let along = match wall.arc() {
            Some(a) => {
                let ang = (p - a.centre).y.atan2((p - a.centre).x);
                let mut d = ang - a.start_angle;
                if a.sweep > 0.0 {
                    while d < 0.0 {
                        d += std::f64::consts::TAU;
                    }
                } else {
                    while d > 0.0 {
                        d -= std::f64::consts::TAU;
                    }
                }
                d / a.sweep * len
            }
            None => {
                let u = (wall.end - wall.start).normalized()?;
                (p - wall.start).x * u.x + (p - wall.start).y * u.y
            }
        };
        Some(Point2::new(along, z - z0))
    }

    /// The zone of a wall's face around a point, if edges drawn on that
    /// face close one there (with the face's own outline).
    fn wall_zone(&self, w: WallId, side: FaceSide, at: glam::Vec3) -> Option<Vec<Point2>> {
        let model = self.ed.model();
        let wall = model.wall(w)?;
        let t = model.wall_type(wall.wall_type)?;
        let (ext, int) = wall.face_offsets(t.thickness());
        let face_off = match side {
            FaceSide::Exterior => ext,
            FaceSide::Interior => int,
        };
        let z0 = model.level(wall.level)?.elevation;
        let len = wall.length();
        let (lo, hi) = (wall.base_offset, wall.base_offset + wall.height);
        // The edges on that face, in its own coordinates.
        let mut segs: Vec<Seg> = Vec::new();
        let lies_on_face = |p: Point2| -> bool {
            let off = match wall.arc() {
                Some(a) => {
                    let r = p.distance(a.centre) - a.radius;
                    if a.sweep > 0.0 {
                        -r
                    } else {
                        r
                    }
                }
                None => match (wall.end - wall.start).normalized() {
                    Some(u) => (p - wall.start).x * -u.y + (p - wall.start).y * u.x,
                    None => return false,
                },
            };
            (off - face_off).abs() <= ON_FACE
        };
        for e in model.edges() {
            let ez = model.level(e.level).map_or(0.0, |l| l.elevation);
            if !e.points.iter().all(|p| lies_on_face(*p)) {
                continue;
            }
            // Ends just short of the face's border meet it.
            let snap = |q: Point2| {
                let near = |v: f64, to: f64| if (v - to).abs() <= NEAR_BORDER { to } else { v };
                Point2::new(near(near(q.x, 0.0), len), near(near(q.y, lo), hi))
            };
            let pts: Vec<Point2> = (0..e.points.len())
                .filter_map(|i| {
                    let (p, h) = e.point_3d(i);
                    self.on_wall_face(w, (p, ez + h)).map(snap)
                })
                .collect();
            for q in pts.windows(2) {
                segs.push(Seg::from_bulge(q[0], q[1], 0.0));
            }
        }
        if segs.is_empty() {
            return None;
        }
        let corners = [
            Point2::new(0.0, lo),
            Point2::new(len, lo),
            Point2::new(len, hi),
            Point2::new(0.0, hi),
        ];
        for i in 0..4 {
            segs.push(Seg::from_bulge(corners[i], corners[(i + 1) % 4], 0.0));
        }
        let p = self.on_wall_face(w, (Point2::new(at.x as f64, at.y as f64), at.z as f64))?;
        let _ = z0;
        let rings = archisim_draft::region::boundary_at(&segs, p).ok()?;
        let outline = rings.first()?.points.clone();
        // The whole face is not a zone.
        let area = archisim_core::signed_area(&outline).abs();
        (area < len * (hi - lo) * 0.999).then_some(outline)
    }
}
