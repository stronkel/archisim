//! Fitting the surroundings to the project ("encajar el entorno"): what
//! was brought around the building (the ground with its orthophoto, the
//! buildings of the surroundings, the streets, the plots, the trees and
//! the street furniture) moved, turned and raised as one, without the
//! building. When the coordinates or the angle of the project are not at
//! hand, the surroundings come in somewhere near and are fitted by hand:
//! by two points (this corner of the cadastre goes on this corner of the
//! building), or dragged and turned.
//!
//! The project stays where it is. What changes is where it is on the map:
//! the georeference and the true north follow the surroundings (if asked),
//! so that the IFC still puts the building in its real place.

use std::collections::BTreeSet;

use archisim_core::{ElementDiscipline, LevelId, Model, ModelError, Point2, Vec2};
use archisim_draft::Transform;

use crate::pick::Element;

/// What of the surroundings moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    /// The ground, with its orthophoto and its platforms.
    pub terrain: bool,
    /// The buildings of the surroundings (the dressed volumes).
    pub buildings: bool,
    /// Streets, roundabouts, road markings and urban surfaces.
    pub streets: bool,
    /// The plots.
    pub plots: bool,
    /// Trees, plants and street furniture, cars and people.
    pub trees: bool,
    /// The georeference and the true north follow (the project changes its
    /// place on the map, not on the plan).
    pub map: bool,
}

impl Default for Parts {
    fn default() -> Self {
        Parts {
            terrain: true,
            buildings: true,
            streets: true,
            plots: true,
            trees: true,
            map: true,
        }
    }
}

/// How many of each there are: for the panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub terrains: usize,
    pub buildings: usize,
    pub streets: usize,
    pub plots: usize,
    pub trees: usize,
}

impl Counts {
    pub fn any(&self) -> bool {
        self.terrains + self.buildings + self.streets + self.plots + self.trees > 0
    }
}

/// A rigid move of the surroundings: turned `angle` (radians,
/// counter-clockwise) about `about`, then shifted `shift`, and raised
/// `dz`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub about: Point2,
    pub angle: f64,
    pub shift: Vec2,
    pub dz: f64,
}

impl Default for Fit {
    fn default() -> Self {
        Fit {
            about: Point2::new(0.0, 0.0),
            angle: 0.0,
            shift: Vec2::new(0.0, 0.0),
            dz: 0.0,
        }
    }
}

impl Fit {
    /// Moved from `a` to `b`.
    pub fn shift(a: Point2, b: Point2) -> Fit {
        Fit {
            about: a,
            shift: b - a,
            ..Fit::default()
        }
    }

    /// Turned about `centre` by `angle`.
    pub fn turn(centre: Point2, angle: f64) -> Fit {
        Fit {
            about: centre,
            angle,
            ..Fit::default()
        }
    }

    /// By two pairs of points: `a` of the surroundings goes on `a2` of the
    /// project, and the line from `a` to `b` turns to lie along the line
    /// from `a2` to `b2` (the distances need not be the same: only the
    /// direction counts; the surroundings keep their size).
    pub fn by_two_points(a: Point2, a2: Point2, b: Point2, b2: Point2) -> Fit {
        let (u, v) = (b - a, b2 - a2);
        let angle = if u.length() < 1e-9 || v.length() < 1e-9 {
            0.0
        } else {
            v.y.atan2(v.x) - u.y.atan2(u.x)
        };
        Fit {
            about: a,
            angle: normal_angle(angle),
            shift: a2 - a,
            dz: 0.0,
        }
    }

    /// The plan transform: turn about `about`, then shift.
    pub fn transform(&self) -> Transform {
        Transform::rotate(self.about, self.angle).then(&Transform::translate(self.shift))
    }

    pub fn apply(&self, p: Point2) -> Point2 {
        self.transform().apply(p)
    }

    /// Whether it moves anything in plan.
    pub fn moves(&self) -> bool {
        self.angle.abs() > 1e-12 || self.shift.length() > 1e-9
    }

    pub fn is_identity(&self) -> bool {
        !self.moves() && self.dz.abs() < 1e-9
    }

    /// First `self`, then `other`.
    #[cfg(test)]
    pub fn then(&self, other: &Fit) -> Fit {
        // Both are turns and shifts: turning about the origin by the sum,
        // then the shift that puts the origin where both put it.
        let o = Point2::new(0.0, 0.0);
        let t = self.transform().then(&other.transform());
        let angle = normal_angle(self.angle + other.angle);
        let at = t.apply(o);
        Fit {
            about: o,
            angle,
            shift: at - o,
            dz: self.dz + other.dz,
        }
    }
}

fn normal_angle(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    let mut a = a.rem_euclid(t);
    if a > std::f64::consts::PI {
        a -= t;
    }
    a
}

/// Whether an object belongs to the surroundings: a tree, a plant, a
/// piece of street furniture, a car or a person (of the discipline of the
/// site, as the categories say).
fn outdoor(model: &Model, id: u64) -> bool {
    model.discipline(id) == Some(ElementDiscipline::Site)
}

/// The elements of the surroundings of the chosen parts.
pub fn context_elements(model: &Model, parts: Parts) -> BTreeSet<Element> {
    let mut out = BTreeSet::new();
    if parts.terrain {
        out.extend(model.terrains().map(|t| Element::Terrain(t.id)));
    }
    if parts.buildings {
        out.extend(
            model
                .masses()
                .filter(|m| m.dress.is_some())
                .map(|m| Element::Mass(m.id)),
        );
    }
    if parts.streets {
        out.extend(model.roads().map(|r| Element::Road(r.id)));
        out.extend(model.roundabouts().map(|r| Element::Roundabout(r.id)));
        out.extend(model.markings().map(|r| Element::Marking(r.id)));
        out.extend(model.surfaces().map(|r| Element::Surface(r.id)));
    }
    if parts.plots {
        out.extend(model.plots().map(|r| Element::Plot(r.id)));
    }
    if parts.trees {
        out.extend(
            model
                .objects()
                .filter(|o| outdoor(model, o.id.0))
                .map(|o| Element::Object(o.id)),
        );
        out.extend(model.plantings().map(|p| Element::Planting(p.id)));
    }
    out
}

/// How many of each part there are.
pub fn counts(model: &Model) -> Counts {
    Counts {
        terrains: model.terrains().count() + usize::from(model.orthophoto().is_some()),
        buildings: model.masses().filter(|m| m.dress.is_some()).count(),
        streets: model.roads().count()
            + model.roundabouts().count()
            + model.markings().count()
            + model.surfaces().count(),
        plots: model.plots().count(),
        trees: model.objects().filter(|o| outdoor(model, o.id.0)).count()
            + model.plantings().count(),
    }
}

/// Moves the surroundings: the chosen parts by `fit`, and the georeference
/// and the true north after them if `parts.map`. The number of elements
/// moved.
pub fn fit_context(model: &mut Model, parts: Parts, fit: &Fit) -> Result<usize, ModelError> {
    let items = context_elements(model, parts);
    let t = fit.transform();
    if fit.moves() {
        crate::xform::transform(model, &items, &t, false)?;
        if parts.terrain {
            if let Some(mut o) = model.orthophoto().cloned() {
                let origin = t.apply(Point2::new(o.origin[0], o.origin[1]));
                let u = t.linear(Vec2::new(o.u[0], o.u[1]));
                let v = t.linear(Vec2::new(o.v[0], o.v[1]));
                o.origin = [origin.x, origin.y];
                o.u = [u.x, u.y];
                o.v = [v.x, v.y];
                model.set_orthophoto(Some(o))?;
            }
        }
        if parts.terrain || parts.streets {
            if let Some(area) = model.surroundings().map(|a| a.to_vec()) {
                model.set_surroundings(Some(area.iter().map(|p| t.apply(*p)).collect()))?;
            }
        }
    }
    if fit.dz.abs() > 1e-9 {
        raise(model, &items, fit.dz)?;
    }
    if parts.map {
        follow_on_the_map(model, fit)?;
    }
    Ok(items.len() + usize::from(parts.terrain && model.orthophoto().is_some() && fit.moves()))
}

/// Raises the surroundings `dz`: the ground, the streets, the buildings
/// and what stands on the ground.
fn raise(model: &mut Model, items: &BTreeSet<Element>, dz: f64) -> Result<(), ModelError> {
    for e in items {
        match *e {
            Element::Terrain(id) => model.edit_terrain(id, |t| {
                for p in t.points.iter_mut() {
                    p.z += dz;
                }
                if let Some(g) = t.ground.as_mut() {
                    for p in g.iter_mut() {
                        p.z += dz;
                    }
                }
                for pad in t.pads.iter_mut() {
                    pad.z += dz;
                }
            })?,
            Element::Road(id) => model.edit_road(id, |r| r.top_offset += dz)?,
            Element::Roundabout(id) => model.edit_roundabout(id, |r| r.top_offset += dz)?,
            Element::Mass(id) => model.edit_mass(id, |m| m.base_offset += dz)?,
            Element::Object(id) => model.edit_object(id, |o| o.base_offset += dz)?,
            _ => {}
        }
    }
    Ok(())
}

/// The georeference and the true north after the surroundings: the
/// surroundings keep their place on the map, so the project (which stays
/// on the plan) takes another. With `W(p) = E + R(a)·p` the map point of a
/// plan point, and the surroundings moved by `T(p) = R(α)(p − o) + o + d`,
/// the new map is `W'(q) = W(T⁻¹ q)`: turned `a − α`, with its origin at
/// `E + R(a)·o − R(a − α)(o + d)`; and raised `dz`, the height of the
/// ±0,00 goes down `dz`.
fn follow_on_the_map(model: &mut Model, fit: &Fit) -> Result<(), ModelError> {
    if fit.moves() {
        let north = model.true_north();
        if let Some(g) = model.georeference().cloned() {
            let a = north + g.convergence();
            let r = |angle: f64, v: Vec2| {
                let (s, c) = angle.sin_cos();
                Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
            };
            let o = fit.about - Point2::new(0.0, 0.0);
            let od = o + fit.shift;
            let e = Vec2::new(g.eastings, g.northings) + r(a, o) - r(a - fit.angle, od);
            let mut g2 = g.clone();
            g2.eastings = e.x;
            g2.northings = e.y;
            g2.height -= fit.dz;
            model.set_georeference(Some(g2))?;
        }
        model.set_true_north(north - fit.angle)?;
    } else if let Some(mut g) = model.georeference().cloned() {
        g.height -= fit.dz;
        model.set_georeference(Some(g))?;
    }
    Ok(())
}

/// The height of the ground at a point of the plan, over the level's ±0,00
/// (the highest terrain there): for «Cota».
pub fn ground_height(model: &Model, p: Point2) -> Option<f64> {
    model
        .terrains()
        .filter_map(|t| t.height_at(p))
        .max_by(|a, b| a.total_cmp(b))
}

/// The buildings of the surroundings that stand on the project: over its
/// slabs, its walls or its roofs on any level (they came from the cadastre
/// with the building itself, or the surroundings fell on it).
pub fn buildings_on_the_project(model: &Model) -> Vec<archisim_core::MassId> {
    let own = own_outlines(model);
    if own.is_empty() {
        return Vec::new();
    }
    model
        .masses()
        .filter(|m| m.dress.is_some())
        .filter(|m| {
            archisim_geom::forms::footprint(m)
                .iter()
                .any(|(outline, _)| own.iter().any(|o| overlap(outline, o)))
        })
        .map(|m| m.id)
        .collect()
}

/// The outlines of the building in plan: its slabs, roofs and walls.
fn own_outlines(model: &Model) -> Vec<Vec<Point2>> {
    let mut out: Vec<Vec<Point2>> = model
        .slabs()
        .map(|s| s.outline.clone())
        .chain(model.roofs().map(|r| r.outline.clone()))
        .filter(|o| o.len() >= 3)
        .collect();
    for w in model.walls() {
        let d = w.end - w.start;
        let l = d.length();
        if l < 1e-6 {
            continue;
        }
        let thick = model.wall_type(w.wall_type).map_or(0.2, |t| t.thickness());
        let n = Vec2::new(-d.y, d.x) * (thick.max(0.05) / 2.0 / l);
        out.push(vec![w.start + n, w.end + n, w.end - n, w.start - n]);
    }
    out
}

/// Whether two outlines overlap in plan (more than touch).
fn overlap(a: &[Point2], b: &[Point2]) -> bool {
    let inside = |p: Point2, poly: &[Point2]| archisim_core::point_in_polygon(p, poly);
    if a.iter().any(|p| inside(*p, b)) || b.iter().any(|p| inside(*p, a)) {
        return true;
    }
    let edges = |p: &[Point2]| -> Vec<(Point2, Point2)> {
        (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect()
    };
    let cross = |p: Point2, q: Point2, r: Point2, s: Point2| {
        let d = |a: Point2, b: Point2, c: Point2| (b - a).cross(c - a);
        d(r, s, p) * d(r, s, q) < 0.0 && d(p, q, r) * d(p, q, s) < 0.0
    };
    let eb = edges(b);
    edges(a)
        .iter()
        .any(|(p, q)| eb.iter().any(|(r, s)| cross(*p, *q, *r, *s)))
}

/// The middle of the building in plan (to turn the surroundings about it
/// when an angle is typed): the middle of the box of its outlines, or the
/// origin.
pub fn project_middle(model: &Model) -> Point2 {
    let own = own_outlines(model);
    let pts: Vec<Point2> = own.into_iter().flatten().collect();
    if pts.is_empty() {
        return Point2::new(0.0, 0.0);
    }
    let (mut lo, mut hi) = (pts[0], pts[0]);
    for p in &pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    Point2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0)
}

/// The outlines to show where the surroundings would go: the footprints
/// of its buildings and the axes of its streets, as polylines (closed ones
/// repeat their first point). For the preview while fitting.
pub fn preview_lines(model: &Model, level: LevelId, parts: Parts) -> Vec<Vec<Point2>> {
    let _ = level;
    let mut out = Vec::new();
    if parts.buildings {
        for m in model.masses().filter(|m| m.dress.is_some()) {
            for (o, _) in archisim_geom::forms::footprint(m) {
                let mut o = o.clone();
                if let Some(f) = o.first().copied() {
                    o.push(f);
                }
                out.push(o);
            }
        }
    }
    if parts.streets {
        out.extend(model.roads().map(|r| r.axis.clone()));
    }
    if parts.plots {
        for p in model.plots() {
            let mut o = p.outline.clone();
            if let Some(f) = o.first().copied() {
                o.push(f);
            }
            out.push(o);
        }
    }
    if parts.terrain {
        for t in model.terrains() {
            if let Some(mut o) = t.outline.clone() {
                if let Some(f) = o.first().copied() {
                    o.push(f);
                }
                out.push(o);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{Georeference, NewRoad, NewWall, ReferenceLine, RoadSection, WallTop};

    fn close(a: Point2, b: Point2) -> bool {
        a.distance(b) < 1e-6
    }

    #[test]
    fn two_points_put_a_corner_on_a_corner_and_turn() {
        let fit = Fit::by_two_points(
            Point2::new(10.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 5.0),
            Point2::new(-5.0, 0.0),
        );
        assert!(close(
            fit.apply(Point2::new(10.0, 0.0)),
            Point2::new(0.0, 0.0)
        ));
        // The line went up; it goes left now: a quarter turn.
        assert!((fit.angle - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(close(
            fit.apply(Point2::new(10.0, 5.0)),
            Point2::new(-5.0, 0.0)
        ));
        let both = fit.then(&Fit::shift(Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)));
        assert!(close(
            both.apply(Point2::new(10.0, 0.0)),
            Point2::new(1.0, 1.0)
        ));
    }

    /// A project with a wall, a street of the surroundings and a georeference.
    fn project() -> (Model, archisim_core::RoadId) {
        let mut m = Model::new("Encajar");
        let level = m.add_level("Planta baja", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        m.add_wall(NewWall {
            wall_type: lib.bearing_wall,
            level,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(10.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let mat = m.materials().next().unwrap().id;
        let road = m
            .add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![Point2::new(-20.0, 20.0), Point2::new(40.0, 20.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        m.set_georeference(Some(Georeference {
            crs: "EPSG:25829".into(),
            eastings: 680_000.0,
            northings: 4_120_000.0,
            height: 5.0,
        }))
        .unwrap();
        (m, road)
    }

    /// Where a plan point is on the map.
    fn on_map(m: &Model, p: Point2) -> (f64, f64) {
        let g = m.georeference().unwrap();
        let a = m.true_north() + g.convergence();
        let (s, c) = a.sin_cos();
        (
            g.eastings + p.x * c - p.y * s,
            g.northings + p.x * s + p.y * c,
        )
    }

    #[test]
    fn the_surroundings_move_and_keep_their_place_on_the_map() {
        let (mut m, road) = project();
        let corner = Point2::new(-20.0, 20.0);
        let before = on_map(&m, corner);
        let wall = m.walls().next().unwrap().clone();
        let fit = Fit::by_two_points(
            corner,
            Point2::new(-3.0, 12.0),
            Point2::new(40.0, 20.0),
            Point2::new(50.0, 30.0),
        );
        let mut fit = fit;
        fit.dz = -1.5;
        let n = fit_context(&mut m, Parts::default(), &fit).unwrap();
        assert!(n >= 1);
        // The street went; the wall stayed.
        let axis = &m.road(road).unwrap().axis;
        assert!(close(axis[0], Point2::new(-3.0, 12.0)));
        assert_eq!(m.walls().next().unwrap().start, wall.start);
        assert!((m.road(road).unwrap().top_offset + 1.5).abs() < 1e-9);
        // The corner of the street is where it was on the map.
        let after = on_map(&m, axis[0]);
        assert!((after.0 - before.0).abs() < 1e-3, "{after:?} {before:?}");
        assert!((after.1 - before.1).abs() < 1e-3);
        assert!((m.georeference().unwrap().height - 6.5).abs() < 1e-9);
    }

    #[test]
    fn without_the_map_only_the_plan_changes() {
        let (mut m, _) = project();
        let g = m.georeference().cloned();
        let north = m.true_north();
        let parts = Parts {
            map: false,
            ..Parts::default()
        };
        fit_context(&mut m, parts, &Fit::turn(Point2::new(0.0, 0.0), 0.3)).unwrap();
        assert_eq!(m.georeference().cloned(), g);
        assert_eq!(m.true_north(), north);
    }
}
