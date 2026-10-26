//! Stairs in 3D, from their layout (`archisim_core::StairLayout`):
//! - on a slab ("losa"): straight flights as their stepped profile on a
//!   sloping slab, exact; winders and spirals tread by tread, the slab
//!   under them following the line of their nosings (a helicoidal slab
//!   under a spiral); landings as slabs; a finish of its own on the
//!   treads if they have another material;
//! - on stringers, on a central stringer, built into a wall or round a
//!   newel: the treads as plates, their risers (if closed), and the
//!   stringers along the sides (or the middle) following the nosings;
//! - railings: a handrail and a bottom rail along each side's line of
//!   nosings at the code's height, posts about every metre and balusters
//!   less than 10 cm apart.

use archisim_core::{
    offset_path, Layer, LayerFunction, MaterialId, Model, Point2, SidePoint, Stair, StairForm,
    StairId, StairLayout, StairStructure, Step, Vec2, EPSILON, RAILING_HEIGHT,
};

use crate::solid::{
    flight_profile, layered_plate_pub, prism, triangulate, vertical_prism_pub, Bar, Solid,
    SolidOwner, P3,
};

/// What a piece of a stair is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairPartKind {
    /// The treads of a run (on a slab, the slab with its steps; otherwise
    /// the tread plates and risers).
    Run(usize),
    Landing(usize),
    /// The finish laid on a concrete stair's treads and landings.
    Finish,
    /// A side stringer (left or right, walking up).
    Stringer(bool),
    /// The central stringer.
    Spine,
    /// A spiral's central post.
    Newel,
    Railing,
}

/// A piece of a stair: what it is, its material and its closed skin.
#[derive(Debug, Clone, PartialEq)]
pub struct StairPart {
    pub kind: StairPartKind,
    pub material: MaterialId,
    pub triangles: Vec<[P3; 3]>,
}

/// How far the side stringers rise over the nosings.
const STRINGER_RISE: f64 = 0.04;
/// Thickness of the risers of a stair that is not of concrete.
const RISER: f64 = 0.02;

/// A prism whose every corner has its own top and bottom height (a
/// counter-clockwise or clockwise outline): its top and bottom faces
/// triangulated the same way, so they may be warped (the soffit of a
/// winder, of a spiral).
pub fn warped_prism(corners: &[(Point2, f64, f64)]) -> Vec<[P3; 3]> {
    let mut c: Vec<(Point2, f64, f64)> = corners.to_vec();
    c.dedup_by(|a, b| a.0.approx_eq(b.0));
    while c.len() > 1 && c[0].0.approx_eq(c[c.len() - 1].0) {
        c.pop();
    }
    if c.len() < 3 {
        return Vec::new();
    }
    let pts: Vec<Point2> = c.iter().map(|x| x.0).collect();
    if archisim_core::signed_area(&pts) < 0.0 {
        c.reverse();
    }
    let pts: Vec<Point2> = c.iter().map(|x| x.0).collect();
    let top = |i: usize| [c[i].0.x, c[i].0.y, c[i].1];
    let bot = |i: usize| [c[i].0.x, c[i].0.y, c[i].2.min(c[i].1 - 1e-6)];
    let mut tris = Vec::new();
    for [a, b, d] in triangulate(&pts) {
        tris.push([top(a), top(b), top(d)]);
        tris.push([bot(a), bot(d), bot(b)]);
    }
    let n = c.len();
    for i in 0..n {
        let j = (i + 1) % n;
        tris.push([bot(i), bot(j), top(j)]);
        tris.push([bot(i), top(j), top(i)]);
    }
    tris
}

/// The vertical height of a slab `waist` thick square to the slope of a
/// stair's nosings.
fn vertical(waist: f64, riser: f64, going: f64) -> f64 {
    let k = if going > EPSILON { riser / going } else { 0.0 };
    waist * (1.0 + k * k).sqrt()
}

/// The pieces of a stair, in model space.
pub fn stair_parts(model: &Model, id: StairId) -> Vec<StairPart> {
    let Some(stair) = model.stair(id) else {
        return Vec::new();
    };
    let z0 = model.level(stair.level).map_or(0.0, |l| l.elevation);
    let l = stair.layout();
    let d = stair.design;
    let treads_material = d.tread_material.unwrap_or(stair.material);
    let base = z0 + stair.base_offset;
    let r = l.riser;
    let mut out = Vec::new();
    let slab = d.structure == StairStructure::Slab;
    let finish = d.finish();
    // How thick a landing is: as the slab under the steps, or a plate
    // on a frame.
    let landing_depth = match d.structure {
        StairStructure::Slab => stair.waist + r,
        StairStructure::Cantilever | StairStructure::Newel => d.tread,
        StairStructure::Stringers | StairStructure::Spine => d.tread + 0.08,
    };
    if slab && d.form == StairForm::Landings && !d.split {
        // Straight flights: their profile, exact.
        for (i, f) in l.flights.iter().enumerate() {
            let profile = flight_profile(f, r, stair.waist, z0 - finish);
            let s = vertical_prism_pub(
                SolidOwner::Stair(id),
                stair.material,
                &profile,
                f.start,
                f.direction(),
                stair.width / 2.0,
            );
            out.push(StairPart {
                kind: StairPartKind::Run(i),
                material: stair.material,
                triangles: s.triangles,
            });
        }
    } else if slab {
        // Tread by tread, the slab under them following the nosings.
        for (ri, run) in l.runs.iter().enumerate() {
            let mut tris = Vec::new();
            for st in &l.steps[run.steps.clone()] {
                let wv = vertical(stair.waist, r, st.going);
                let corners: Vec<(Point2, f64, f64)> = st
                    .outline_rising()
                    .into_iter()
                    .map(|(p, f)| {
                        let nosing = z0 + st.top + f * r;
                        (p, z0 + st.top - finish, (nosing - r - wv).max(base))
                    })
                    .collect();
                tris.extend(warped_prism(&corners));
            }
            out.push(StairPart {
                kind: StairPartKind::Run(ri),
                material: stair.material,
                triangles: tris,
            });
        }
    } else {
        // Tread plates and, if closed, risers.
        for (ri, run) in l.runs.iter().enumerate() {
            let mut tris = Vec::new();
            for k in run.steps.clone() {
                let st = &l.steps[k];
                tris.extend(prism(&st.outline(), z0 + st.top - d.tread, z0 + st.top));
                if d.closed_risers {
                    let below = if k == run.steps.start {
                        run.base
                    } else {
                        l.steps[k - 1].top
                    };
                    tris.extend(riser_plate(st.front, z0 + below, z0 + st.top - d.tread));
                }
            }
            if d.closed_risers {
                // The last riser, up to the landing or the floor above.
                if let Some(last) = run.steps.clone().last().map(|k| &l.steps[k]) {
                    let next = l
                        .landings
                        .get(ri)
                        .map_or(stair.height + stair.base_offset, |x| x.top);
                    let depth = if l.landings.get(ri).is_some() {
                        landing_depth
                    } else {
                        d.tread
                    };
                    let back = [last.back[1], last.back[0]];
                    tris.extend(riser_plate(back, z0 + last.top, z0 + next - depth));
                }
            }
            out.push(StairPart {
                kind: StairPartKind::Run(ri),
                material: treads_material,
                triangles: tris,
            });
        }
    }
    // Landings.
    for (i, landing) in l.landings.iter().enumerate() {
        let material = if slab {
            stair.material
        } else {
            treads_material
        };
        let layers = [Layer {
            material,
            thickness: landing_depth,
            function: LayerFunction::Structure,
        }];
        let top = z0 + landing.top - if slab { finish } else { 0.0 };
        let tris: Vec<[P3; 3]> = layered_plate_pub(
            SolidOwner::Stair(id),
            &landing.outline,
            &[],
            &|_| top,
            &|t| t,
            &layers,
        )
        .into_iter()
        .flat_map(|s| s.triangles)
        .collect();
        out.push(StairPart {
            kind: StairPartKind::Landing(i),
            material,
            triangles: tris,
        });
    }
    // The finish on a concrete stair.
    if finish > 0.0 {
        let mut tris = Vec::new();
        for st in &l.steps {
            tris.extend(prism(&st.outline(), z0 + st.top - finish, z0 + st.top));
        }
        for landing in &l.landings {
            tris.extend(prism(
                &landing.outline,
                z0 + landing.top - finish,
                z0 + landing.top,
            ));
        }
        out.push(StairPart {
            kind: StairPartKind::Finish,
            material: treads_material,
            triangles: tris,
        });
    }
    // What carries the treads.
    let going = l.going().min(stair.going).max(0.1);
    let [sw, sd] = d.stringer;
    match d.structure {
        StairStructure::Stringers => {
            for left in [true, false] {
                let side = l.side(left);
                let dv = vertical(sd, r, going);
                let tris = band(&side, if left { sw } else { -sw }, |p| {
                    (z0 + p.z + STRINGER_RISE, (z0 + p.z - r - dv).max(base))
                });
                out.push(StairPart {
                    kind: StairPartKind::Stringer(left),
                    material: stair.material,
                    triangles: tris,
                });
            }
        }
        StairStructure::Spine => {
            let dv = vertical(sd, r, going);
            let mut tris = Vec::new();
            for st in &l.steps {
                let corners = centre_strip(st, sw, |f| {
                    let nosing = z0 + st.top + f * r;
                    (z0 + st.top - d.tread, (nosing - r - d.tread - dv).max(base))
                });
                tris.extend(warped_prism(&corners));
            }
            for landing in &l.landings {
                let top = z0 + landing.top - landing_depth;
                let pts: Vec<SidePoint> = landing
                    .centre
                    .iter()
                    .map(|p| SidePoint {
                        p: *p,
                        z: landing.top,
                        floor: landing.top,
                    })
                    .collect();
                tris.extend(band_centred(&pts, sw, |_| (top, top - sd)));
            }
            out.push(StairPart {
                kind: StairPartKind::Spine,
                material: stair.material,
                triangles: tris,
            });
        }
        StairStructure::Newel if d.form == StairForm::Spiral => {
            let c = stair.path[0];
            let radius = d.core.clamp(0.03, 0.3);
            let ring: Vec<Point2> = (0..24)
                .map(|i| {
                    let a = i as f64 / 24.0 * std::f64::consts::TAU;
                    c + Vec2::new(a.cos(), a.sin()) * radius
                })
                .collect();
            let top = z0 + stair.base_offset + stair.height + RAILING_HEIGHT;
            out.push(StairPart {
                kind: StairPartKind::Newel,
                material: stair.material,
                triangles: prism(&ring, base, top),
            });
        }
        _ => {}
    }
    let bars = railing_bars(model, id);
    if !bars.is_empty() {
        out.push(StairPart {
            kind: StairPartKind::Railing,
            material: railing_material(model, stair),
            triangles: bars.iter().flat_map(Bar::triangles).collect(),
        });
    }
    out.retain(|p| !p.triangles.is_empty());
    out
}

/// Solids of a stair: one per piece.
pub fn stair_solids(model: &Model, id: StairId) -> Vec<Solid> {
    stair_parts(model, id)
        .into_iter()
        .map(|p| Solid {
            owner: SolidOwner::Stair(id),
            material: p.material,
            triangles: p.triangles,
        })
        .collect()
}

/// A riser: a plate behind the edge `edge` (left end, right end walking
/// up), between two heights.
fn riser_plate(edge: [Point2; 2], z0: f64, z1: f64) -> Vec<[P3; 3]> {
    let Some(u) = (edge[1] - edge[0]).normalized() else {
        return Vec::new();
    };
    // Into the tread: left of the edge walked from its right end.
    let m = u.perp_left() * RISER;
    let quad = [edge[0], edge[1], edge[1] + m, edge[0] + m];
    prism(&quad, z0, z1)
}

/// A band along a line of nosings, `width` to its left (negative: to its
/// right), each corner's top and bottom from `z`: a stringer.
fn band(line: &[SidePoint], width: f64, z: impl Fn(&SidePoint) -> (f64, f64)) -> Vec<[P3; 3]> {
    let mut line = line.to_vec();
    line.dedup_by(|a, b| a.p.approx_eq(b.p));
    if line.len() < 2 {
        return Vec::new();
    }
    let pts: Vec<Point2> = line.iter().map(|x| x.p).collect();
    let Some(far) = offset_path(&pts, width) else {
        return Vec::new();
    };
    let mut tris = Vec::new();
    for i in 0..line.len() - 1 {
        let (za, zb) = (z(&line[i]), z(&line[i + 1]));
        tris.extend(warped_prism(&[
            (pts[i], za.0, za.1),
            (pts[i + 1], zb.0, zb.1),
            (far[i + 1], zb.0, zb.1),
            (far[i], za.0, za.1),
        ]));
    }
    tris
}

/// A band `width` wide centred on a line.
fn band_centred(
    line: &[SidePoint],
    width: f64,
    z: impl Fn(&SidePoint) -> (f64, f64),
) -> Vec<[P3; 3]> {
    let mut line = line.to_vec();
    line.dedup_by(|a, b| a.p.approx_eq(b.p));
    if line.len() < 2 {
        return Vec::new();
    }
    let pts: Vec<Point2> = line.iter().map(|x| x.p).collect();
    let (Some(near), Some(far)) = (
        offset_path(&pts, -width / 2.0),
        offset_path(&pts, width / 2.0),
    ) else {
        return Vec::new();
    };
    let mut tris = Vec::new();
    for i in 0..line.len() - 1 {
        let (za, zb) = (z(&line[i]), z(&line[i + 1]));
        tris.extend(warped_prism(&[
            (near[i], za.0, za.1),
            (near[i + 1], zb.0, zb.1),
            (far[i + 1], zb.0, zb.1),
            (far[i], za.0, za.1),
        ]));
    }
    tris
}

/// The strip `width` wide along the middle of a tread, its corners with
/// their heights from `z` (given how far from the front each is).
fn centre_strip(st: &Step, width: f64, z: impl Fn(f64) -> (f64, f64)) -> Vec<(Point2, f64, f64)> {
    let mut c = st.centre.clone();
    c.dedup_by(|a, b| a.approx_eq(*b));
    if c.len() < 2 {
        return Vec::new();
    }
    let (Some(near), Some(far)) = (offset_path(&c, -width / 2.0), offset_path(&c, width / 2.0))
    else {
        return Vec::new();
    };
    let mut acc = vec![0.0];
    for w in c.windows(2) {
        let last = *acc.last().unwrap_or(&0.0);
        acc.push(last + w[0].distance(w[1]));
    }
    let total = acc.last().copied().unwrap_or(1.0).max(EPSILON);
    let mut out: Vec<(Point2, f64, f64)> = Vec::new();
    for (p, a) in near.iter().zip(&acc) {
        let (t, b) = z(a / total);
        out.push((*p, t, b));
    }
    for (p, a) in far.iter().zip(&acc).rev() {
        let (t, b) = z(a / total);
        out.push((*p, t, b));
    }
    out
}

/// The bars of a stair's railings, in model space: along each side that
/// has one, just inside it, a handrail at the code's height over the
/// line of the nosings and a bottom rail just over it, posts at the ends
/// and about every metre, and balusters between them less than 10 cm
/// apart. A spiral round a newel has none on its inner side.
pub fn railing_bars(model: &Model, id: StairId) -> Vec<Bar> {
    let Some(stair) = model.stair(id) else {
        return Vec::new();
    };
    let (left, right) = (stair.railings.left(), stair.railings.right());
    if !left && !right {
        return Vec::new();
    }
    let z0 = model.level(stair.level).map_or(0.0, |l| l.elevation);
    let l = stair.layout();
    let h = RAILING_HEIGHT;
    let mut bars = Vec::new();
    for (on, is_left) in [(left, true), (right, false)] {
        if !on || skip_inner_rail(stair, is_left) {
            continue;
        }
        let line = railing_line(&l, is_left);
        if line.len() < 2 {
            continue;
        }
        bars.extend(rail_bars(&line, z0, h));
    }
    bars
}

/// Whether the railing on one side of a stair is left out: the inner
/// side of a spiral with a newel or a small eye.
pub fn skip_inner_rail(stair: &Stair, left: bool) -> bool {
    stair.design.form == StairForm::Spiral
        && (stair.design.sweep >= 0.0) == left
        && stair.design.core < 0.25
}

/// The line a railing follows: the side's line of nosings, 4 cm inside.
pub fn railing_line(l: &StairLayout, left: bool) -> Vec<SidePoint> {
    let mut side = l.side(left);
    side.dedup_by(|a, b| a.p.distance(b.p) < 1e-4);
    if side.len() < 2 {
        return side;
    }
    let pts: Vec<Point2> = side.iter().map(|s| s.p).collect();
    let Some(inner) = offset_path(&pts, if left { -0.04 } else { 0.04 }) else {
        return side;
    };
    side.iter()
        .zip(inner)
        .map(|(s, p)| SidePoint { p, ..*s })
        .collect()
}

/// The bars along one railing's line.
fn rail_bars(line: &[SidePoint], z0: f64, h: f64) -> Vec<Bar> {
    let up = [0.0, 0.0, 1.0];
    let mut bars = Vec::new();
    let post = |p: Point2, za: f64, zb: f64, along: Vec2, w: f64| Bar {
        a: [p.x, p.y, za],
        b: [p.x, p.y, zb],
        side: [along.x, along.y, 0.0],
        up: [-along.y, along.x, 0.0],
        w,
        d: w,
    };
    // Lengths along the line.
    let mut acc = vec![0.0];
    for w in line.windows(2) {
        let last = *acc.last().unwrap_or(&0.0);
        acc.push(last + w[0].p.distance(w[1].p));
    }
    let total = *acc.last().unwrap_or(&0.0);
    if total < 0.05 {
        return bars;
    }
    for i in 0..line.len() - 1 {
        let (a, b) = (line[i], line[i + 1]);
        let Some(u) = (b.p - a.p).normalized() else {
            continue;
        };
        let n = u.perp_left();
        for (lift, size) in [(h, 0.05), (0.05, 0.03)] {
            bars.push(Bar {
                a: [a.p.x, a.p.y, z0 + a.z + lift],
                b: [b.p.x, b.p.y, z0 + b.z + lift],
                side: [n.x, n.y, 0.0],
                up,
                w: size,
                d: size,
            });
        }
    }
    // A point `s` along the line: where, how high the nosings, what one
    // stands on and which way the line goes.
    let at = |s: f64| {
        let i = (0..line.len() - 1)
            .find(|&i| s <= acc[i + 1] + 1e-9)
            .unwrap_or(line.len() - 2);
        let (a, b) = (line[i], line[i + 1]);
        let len = (acc[i + 1] - acc[i]).max(EPSILON);
        let f = ((s - acc[i]) / len).clamp(0.0, 1.0);
        let u = (b.p - a.p).normalized().unwrap_or(Vec2::new(1.0, 0.0));
        let floor = if s >= total - 1e-9 { b.z } else { a.floor };
        (a.p + (b.p - a.p) * f, a.z + (b.z - a.z) * f, floor, u)
    };
    let posts = (total / 1.0).ceil().max(1.0) as usize;
    for i in 0..=posts {
        let s = total * i as f64 / posts as f64;
        let (p, z, floor, u) = at(s);
        bars.push(post(p, z0 + floor, z0 + z + h - 0.025, u, 0.04));
    }
    // Balusters, 11 cm apart (less than 10 cm clear between them).
    let count = (total / 0.11).ceil() as usize;
    for i in 1..count {
        let s = total * i as f64 / count as f64;
        let (p, z, _, u) = at(s);
        bars.push(post(p, z0 + z + 0.05, z0 + z + h - 0.025, u, 0.016));
    }
    bars
}

/// The material railings are drawn in: the model's first steel, or the
/// stair's own.
pub fn railing_material(model: &Model, stair: &Stair) -> MaterialId {
    model
        .materials()
        .find(|m| m.category == "steel")
        .map_or(stair.material, |m| m.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewStair, StairDesign, StairUse, WallTop};

    fn closed(tris: &[[P3; 3]]) -> bool {
        use std::collections::HashMap;
        let key = |p: P3| {
            (
                (p[0] * 1e6).round() as i64,
                (p[1] * 1e6).round() as i64,
                (p[2] * 1e6).round() as i64,
            )
        };
        let mut edges: HashMap<_, i32> = HashMap::new();
        for t in tris {
            for i in 0..3 {
                let (a, b) = (key(t[i]), key(t[(i + 1) % 3]));
                if a == b {
                    continue;
                }
                *edges.entry((a, b)).or_default() += 1;
                *edges.entry((b, a)).or_default() -= 1;
            }
        }
        edges.values().all(|v| *v == 0)
    }

    fn volume(tris: &[[P3; 3]]) -> f64 {
        Solid {
            owner: SolidOwner::Stair(StairId(0)),
            material: MaterialId(0),
            triangles: tris.to_vec(),
        }
        .volume()
    }

    fn stair(path: Vec<Point2>, design: StairDesign) -> (Model, StairId) {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let ha = m.add_material("HA", "concrete", [200, 200, 200], CutPattern::Solid);
        m.add_material("Acero", "steel", [90, 90, 90], CutPattern::Solid);
        let wood = m.add_material("Roble", "wood", [160, 120, 80], CutPattern::Solid);
        let design = StairDesign {
            tread_material: design.tread_material.map(|_| wood),
            ..design
        };
        let id = m
            .add_stair(NewStair {
                level: pb,
                path,
                width: 1.0,
                going: 0.28,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 2.97,
                waist: 0.15,
                material: ha,
                usage: StairUse::Restricted,
                design,
            })
            .unwrap();
        (m, id)
    }

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    /// Every piece of every kind of stair is a closed skin with volume.
    #[test]
    fn every_stair_is_closed() {
        let l_path = vec![p(0.0, 0.0), p(2.3, 0.0), p(2.3, 2.3)];
        let u_path = vec![p(0.0, 0.0), p(1.7, 0.0), p(1.7, 2.1), p(0.0, 2.1)];
        let spiral = vec![p(0.0, 0.0), p(0.6, 0.0)];
        for structure in [
            StairStructure::Slab,
            StairStructure::Stringers,
            StairStructure::Spine,
            StairStructure::Cantilever,
            StairStructure::Newel,
        ] {
            for (form, path, sweep, split, arrival) in [
                (StairForm::Landings, l_path.clone(), 0.0, false, 0.0),
                (StairForm::Winders, l_path.clone(), 0.0, false, 0.0),
                (StairForm::Winders, u_path.clone(), 0.0, false, 0.0),
                (StairForm::Landings, u_path.clone(), 0.0, false, 0.0),
                (StairForm::Landings, l_path.clone(), 0.0, true, 0.0),
                (StairForm::Landings, u_path.clone(), 0.0, true, 0.0),
                (StairForm::Spiral, spiral.clone(), 7.5, false, 0.0),
                (StairForm::Spiral, spiral.clone(), 7.5, false, 1.5),
            ] {
                for finish in [None, Some(())] {
                    let design = StairDesign {
                        form,
                        sweep,
                        split,
                        arrival,
                        structure,
                        tread_material: finish.map(|_| MaterialId(0)),
                        ..Default::default()
                    };
                    let (m, id) = stair(path.clone(), design);
                    let parts = stair_parts(&m, id);
                    assert!(!parts.is_empty());
                    for part in &parts {
                        if part.kind == StairPartKind::Railing {
                            continue;
                        }
                        assert!(
                            closed(&part.triangles),
                            "{structure:?} {form:?} {split} {arrival} {:?}",
                            part.kind
                        );
                        assert!(
                            volume(&part.triangles) > 1e-4,
                            "{structure:?} {form:?} {:?}: {}",
                            part.kind,
                            volume(&part.triangles)
                        );
                    }
                }
            }
        }
    }

    /// A straight flight on its slab keeps its exact profile; with a
    /// finish, the slab goes down under it.
    #[test]
    fn a_slab_and_its_finish() {
        let path = vec![p(0.0, 0.0), p(4.48, 0.0)];
        let (m, id) = stair(path.clone(), StairDesign::default());
        let parts = stair_parts(&m, id);
        let l = m.stair(id).unwrap().layout();
        let profile = flight_profile(&l.flights[0], l.riser, 0.15, 0.0);
        let run = parts
            .iter()
            .find(|p| p.kind == StairPartKind::Run(0))
            .unwrap();
        assert!((volume(&run.triangles) - archisim_core::signed_area(&profile)).abs() < 1e-9);
        let (m, id) = stair(
            path,
            StairDesign {
                tread_material: Some(MaterialId(0)),
                tread: 0.03,
                ..Default::default()
            },
        );
        let parts = stair_parts(&m, id);
        let finish = parts
            .iter()
            .find(|p| p.kind == StairPartKind::Finish)
            .unwrap();
        // 16 treads of 28 cm by 1 m by 3 cm.
        assert!((volume(&finish.triangles) - 16.0 * 0.28 * 0.03).abs() < 1e-9);
        assert_ne!(finish.material, m.stair(id).unwrap().material);
    }

    /// Railings: a handrail and a bottom rail along each side, posts and
    /// balusters less than 10 cm apart; none on the inner side of a
    /// spiral round its newel.
    #[test]
    fn railings_and_balusters() {
        let (m, id) = stair(vec![p(0.0, 0.0), p(4.48, 0.0)], StairDesign::default());
        let bars = railing_bars(&m, id);
        // The handrails rise with the nosings, from the first tread to
        // the floor above, on both sides.
        let r: f64 = 2.97 / 17.0;
        let length: f64 = bars
            .iter()
            .filter(|b| b.w == 0.05)
            .map(|b| {
                let d = [b.b[0] - b.a[0], b.b[1] - b.a[1], b.b[2] - b.a[2]];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            })
            .sum();
        let expected = 2.0 * (4.48f64.powi(2) + (2.97f64 - r).powi(2)).sqrt();
        assert!((length - expected).abs() < 1e-6, "{length} {expected}");
        let balusters: Vec<&Bar> = bars.iter().filter(|b| b.w == 0.016).collect();
        assert!(balusters.len() > 2 * 35);
        // Clear gap between balusters below 10 cm.
        let gap = (balusters[1].a[0] - balusters[0].a[0]).abs() - 0.016;
        assert!(gap < 0.1, "{gap}");
        let (m, id) = stair(
            vec![p(0.0, 0.0), p(0.6, 0.0)],
            StairDesign {
                form: StairForm::Spiral,
                sweep: 7.5,
                structure: StairStructure::Newel,
                ..Default::default()
            },
        );
        let bars = railing_bars(&m, id);
        assert!(bars.iter().filter(|b| b.w == 0.05).count() > 10);
        // All on the outside, more than 1 m from the centre.
        assert!(bars
            .iter()
            .all(|b| Point2::new(b.a[0], b.a[1]).length() > 1.0));
        let parts = stair_parts(&m, id);
        assert!(parts.iter().any(|p| p.kind == StairPartKind::Newel));
    }
}
