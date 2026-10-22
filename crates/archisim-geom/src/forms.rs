//! The solids of the concept phase's volumes: a contour pushed up (with
//! its corners and its top and base edges chamfered or rounded, if asked),
//! a sphere, cylinder, cone or profile turned round an axis, or any solid
//! made by joining or cutting them. Each solid is a closed mesh whose
//! triangles say which face they are on, so faces can be painted one by
//! one: bottom 0, top 1, sides (or the curved surface, or each piece of a
//! profile) 2 on.

use archisim_core::{
    offset_contour, signed_area, EdgeFinish, EdgeKind, FaceRef, Mass, MassId, MassMesh, MassShape,
    MaterialId, Model, ModelError, Point2, Primitive, ROUND_SEGMENTS,
};
use std::collections::BTreeMap;

use crate::solid::triangulate;

pub const BOTTOM: u32 = 0;
pub const TOP: u32 = 1;
pub const SIDE: u32 = 2;

/// Steps of a rounded edge (a quarter of a circle).
const ROUND_STEPS: usize = 6;

/// Builds a mesh point by point.
#[derive(Default)]
struct Builder {
    mesh: MassMesh,
}

impl Builder {
    fn point(&mut self, x: f64, y: f64, z: f64) -> u32 {
        self.mesh.positions.push([x, y, z]);
        (self.mesh.positions.len() - 1) as u32
    }

    fn tri(&mut self, a: u32, b: u32, c: u32, face: u32) {
        if a == b || b == c || a == c {
            return;
        }
        self.mesh.triangles.push([a, b, c]);
        self.mesh.faces.push(face);
    }

    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32, face: u32) {
        self.tri(a, b, c, face);
        self.tri(a, c, d, face);
    }
}

/// The solid of a mass, its `z` over the mass's base.
pub fn mass_body(m: &Mass) -> MassMesh {
    let h = m.height.max(0.01);
    let body = match &m.shape {
        MassShape::Sweep {
            path,
            width,
            profile: Some(p),
        } => crate::sculpt::sweep(path, &fit_profile(p, *width, h)),
        // Twisted or bent: in level bands from the start, so its sides
        // read as smooth surfaces.
        MassShape::Prism(_) | MassShape::Sweep { .. } => {
            let bands = m
                .deform
                .filter(|d| !d.is_none())
                .map_or(1, |d| crate::sculpt::deform_steps(&d));
            prism_body_banded(&m.outline(), h, m.finish, bands)
        }
        MassShape::Primitive(p) => primitive_body(p, h, m.finish),
        MassShape::Mesh(mesh) => mesh.clone(),
    };
    match m.deform {
        Some(d) if !d.is_none() => crate::sculpt::deform(&body, &d, body_centre(&body)),
        _ => body,
    }
}

/// A sweep's profile stretched to its width and height (its points keep
/// where the path runs: 0 across).
pub fn fit_profile(profile: &[Point2], width: f64, height: f64) -> Vec<Point2> {
    let (lo, hi) = profile.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let kx = if hi.x - lo.x > 1e-9 {
        width / (hi.x - lo.x)
    } else {
        1.0
    };
    let ky = if hi.y - lo.y > 1e-9 {
        height / (hi.y - lo.y)
    } else {
        1.0
    };
    profile
        .iter()
        .map(|p| Point2::new(p.x * kx, (p.y - lo.y) * ky))
        .collect()
}

/// The middle of a solid's base in plan, round which it twists.
fn body_centre(m: &MassMesh) -> Point2 {
    let base: Vec<&[f64; 3]> = m.positions.iter().filter(|p| p[2].abs() < 1e-6).collect();
    let pts = if base.is_empty() {
        m.positions.iter().collect()
    } else {
        base
    };
    let n = pts.len().max(1) as f64;
    let (x, y) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p[0], a.1 + p[1]));
    Point2::new(x / n, y / n)
}

/// The corners of a polygon chamfered or rounded (`which` says which
/// corners), each by `size` at most (less where its sides are short).
/// Returns the new polygon and, for each of its edges, the edge of the
/// old one it comes from (a corner goes with the edge after it).
pub fn finish_corners(
    poly: &[Point2],
    size: f64,
    kind: EdgeKind,
    which: &dyn Fn(usize) -> bool,
) -> (Vec<Point2>, Vec<usize>) {
    let n = poly.len();
    let mut out = Vec::new();
    let mut side = Vec::new();
    for i in 0..n {
        let v = poly[i];
        let a = poly[(i + n - 1) % n];
        let b = poly[(i + 1) % n];
        let (la, lb) = (v.distance(a), v.distance(b));
        let (ua, ub) = ((a - v).normalized(), (b - v).normalized());
        let s = size.min(0.45 * la).min(0.45 * lb);
        let straight = match (ua, ub) {
            (Some(ua), Some(ub)) => ua.x * ub.x + ua.y * ub.y < -0.999,
            _ => true,
        };
        if !which(i) || s <= 1e-6 || straight {
            out.push(v);
            side.push(i);
            continue;
        }
        let (ua, ub) = (ua.expect("checked"), ub.expect("checked"));
        let p1 = v + ua * s;
        let p2 = v + ub * s;
        match kind {
            EdgeKind::Chamfer => {
                out.push(p1);
                side.push(i);
                out.push(p2);
                side.push(i);
            }
            EdgeKind::Round => {
                // A quadratic curve from p1 to p2, pulled towards the corner:
                // tangent to both sides.
                for k in 0..=ROUND_STEPS {
                    let t = k as f64 / ROUND_STEPS as f64;
                    let q = p1 * ((1.0 - t) * (1.0 - t)) + v * (2.0 * t * (1.0 - t)) + p2 * (t * t);
                    out.push(q);
                    side.push(i);
                }
            }
        }
    }
    (out, side)
}

/// A contour pushed up `h`, with its edges finished.
#[cfg(test)]
pub(crate) fn prism_body(outline: &[Point2], h: f64, finish: Option<EdgeFinish>) -> MassMesh {
    prism_body_banded(outline, h, finish, 1)
}

/// [`prism_body`] with its straight sides in `bands` level bands.
fn prism_body_banded(
    outline: &[Point2],
    h: f64,
    finish: Option<EdgeFinish>,
    bands: usize,
) -> MassMesh {
    let mut poly = outline.to_vec();
    if poly.len() < 3 {
        return MassMesh::default();
    }
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let n0 = poly.len();
    let (poly, side_of) = match finish {
        Some(f) if f.vertical && f.size > 0.0 => finish_corners(&poly, f.size, f.kind, &|_| true),
        _ => (poly.clone(), (0..n0).collect()),
    };
    // Rings up the sides: (height, how far in).
    let mut rings: Vec<(f64, f64)> = Vec::new();
    let s = finish.map_or(0.0, |f| f.size.min(h * 0.49).max(0.0));
    let arc = |kind: EdgeKind| -> Vec<f64> {
        match kind {
            EdgeKind::Chamfer => vec![0.0, 1.0],
            EdgeKind::Round => (0..=ROUND_STEPS)
                .map(|k| k as f64 / ROUND_STEPS as f64)
                .collect(),
        }
    };
    match finish {
        Some(f) if f.bottom && s > 1e-6 => {
            for t in arc(f.kind) {
                let (z, inset) = match f.kind {
                    EdgeKind::Chamfer => (s * t, s * (1.0 - t)),
                    EdgeKind::Round => {
                        let a = t * std::f64::consts::FRAC_PI_2;
                        (s - s * a.cos(), s - s * a.sin())
                    }
                };
                rings.push((z, inset));
            }
        }
        _ => rings.push((0.0, 0.0)),
    }
    match finish {
        Some(f) if f.top && s > 1e-6 => {
            for t in arc(f.kind) {
                let (z, inset) = match f.kind {
                    EdgeKind::Chamfer => (h - s + s * t, s * t),
                    EdgeKind::Round => {
                        let a = t * std::f64::consts::FRAC_PI_2;
                        (h - s + s * a.sin(), s - s * a.cos())
                    }
                };
                rings.push((z, inset));
            }
        }
        _ => rings.push((h, 0.0)),
    }
    rings.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    if bands > 1 {
        // Between the finished edges, where the sides run straight.
        let low = rings
            .iter()
            .filter(|r| r.0 < h / 2.0)
            .map(|r| r.0)
            .fold(0.0, f64::max);
        let high = rings
            .iter()
            .filter(|r| r.0 >= h / 2.0)
            .map(|r| r.0)
            .fold(h, f64::min);
        for k in 1..bands {
            let z = h * k as f64 / bands as f64;
            if z > low + 1e-9 && z < high - 1e-9 {
                rings.push((z, 0.0));
            }
        }
        rings.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    let n = poly.len();
    let mut b = Builder::default();
    let mut ids: Vec<Vec<u32>> = Vec::new();
    let mut shapes: Vec<Vec<Point2>> = Vec::new();
    for &(z, inset) in &rings {
        let ring = if inset > 1e-9 {
            offset_contour(&poly, -inset)
                .filter(|r| r.len() == n)
                .unwrap_or_else(|| shrink(&poly, inset))
        } else {
            poly.clone()
        };
        ids.push(ring.iter().map(|p| b.point(p.x, p.y, z)).collect());
        shapes.push(ring);
    }
    for r in 0..rings.len() - 1 {
        for j in 0..n {
            let k = (j + 1) % n;
            let face = SIDE + side_of[j] as u32;
            b.quad(ids[r][j], ids[r][k], ids[r + 1][k], ids[r + 1][j], face);
        }
    }
    let last = rings.len() - 1;
    for [p, q, r] in triangulate(&shapes[last]) {
        b.tri(ids[last][p], ids[last][q], ids[last][r], TOP);
    }
    for [p, q, r] in triangulate(&shapes[0]) {
        b.tri(ids[0][p], ids[0][r], ids[0][q], BOTTOM);
    }
    b.mesh
}

/// A polygon shrunk towards its middle by about `d` (when an exact
/// offset folds over).
fn shrink(poly: &[Point2], d: f64) -> Vec<Point2> {
    let c = poly.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p) * (1.0 / poly.len() as f64);
    poly.iter()
        .map(|p| {
            let r = p.distance(c);
            if r < 1e-9 {
                *p
            } else {
                c + (*p - c) * ((r - d).max(r * 0.1) / r)
            }
        })
        .collect()
}

/// A round form.
fn primitive_body(p: &Primitive, h: f64, finish: Option<EdgeFinish>) -> MassMesh {
    let c = p.center();
    match p {
        Primitive::Sphere { radius, .. } => sphere(c, *radius),
        Primitive::Cylinder { radius, .. } => revolve(
            c,
            &[Point2::new(*radius, 0.0), Point2::new(*radius, h)],
            finish,
        ),
        Primitive::Cone {
            radius, top_radius, ..
        } => revolve(
            c,
            &[Point2::new(*radius, 0.0), Point2::new(*top_radius, h)],
            finish,
        ),
        Primitive::Revolution { profile, .. } => revolve(c, profile, finish),
    }
}

fn sphere(c: Point2, r: f64) -> MassMesh {
    let seg = ROUND_SEGMENTS;
    let rows = seg / 2;
    let mut b = Builder::default();
    let south = b.point(c.x, c.y, 0.0);
    let mut rings: Vec<Vec<u32>> = Vec::new();
    for i in 1..rows {
        let phi = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / rows as f64;
        let (z, rr) = (r + r * phi.sin(), r * phi.cos());
        rings.push(
            (0..seg)
                .map(|k| {
                    let a = std::f64::consts::TAU * k as f64 / seg as f64;
                    b.point(c.x + rr * a.cos(), c.y + rr * a.sin(), z)
                })
                .collect(),
        );
    }
    let north = b.point(c.x, c.y, 2.0 * r);
    for k in 0..seg {
        let k1 = (k + 1) % seg;
        b.tri(south, rings[0][k1], rings[0][k], SIDE);
        let top = rings.len() - 1;
        b.tri(north, rings[top][k], rings[top][k1], SIDE);
    }
    for i in 0..rings.len() - 1 {
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            b.quad(
                rings[i][k],
                rings[i][k1],
                rings[i + 1][k1],
                rings[i + 1][k],
                SIDE,
            );
        }
    }
    b.mesh
}

/// A half-section turned round the vertical axis through `c`: each point
/// is (distance from the axis, height). Closed with flat faces where it
/// does not reach the axis.
fn revolve(c: Point2, profile: &[Point2], finish: Option<EdgeFinish>) -> MassMesh {
    let mut prof: Vec<Point2> = profile
        .iter()
        .map(|p| Point2::new(p.x.max(0.0), p.y))
        .collect();
    prof.dedup_by(|a, b| a.approx_eq(*b));
    if prof.len() < 2 {
        return MassMesh::default();
    }
    // With the axis, the profile goes round counter-clockwise: up the
    // outside. Drawn the other way, it is turned round.
    let closed = |p: &[Point2]| {
        let mut q = vec![Point2::new(0.0, p[0].y)];
        q.extend_from_slice(p);
        q.push(Point2::new(0.0, p[p.len() - 1].y));
        q
    };
    if signed_area(&closed(&prof)) < 0.0 {
        prof.reverse();
    }
    let segments = prof.len() - 1;
    // Faces of each piece of profile.
    let mut piece: Vec<u32> = (0..segments as u32).map(|j| SIDE + j).collect();
    if let Some(f) = finish.filter(|f| (f.top || f.bottom) && f.size > 0.0) {
        let poly = closed(&prof);
        let last = poly.len() - 2;
        let (fin, from) = finish_corners(&poly, f.size, f.kind, &|i| {
            (i == 1 && f.bottom && poly[1].x > 1e-6) || (i == last && f.top && poly[last].x > 1e-6)
        });
        // Back to a profile: without the points on the axis. Each edge of
        // the new polygon comes from an edge of the closed one: edge 0 is
        // the bottom cap, edge `i` (1…) the profile's piece `i − 1`.
        let m = fin.len();
        prof = fin[1..m - 1].to_vec();
        piece = (1..m - 2)
            .map(|e| match from[e] {
                0 => BOTTOM,
                k if k > segments => TOP,
                k => SIDE + k as u32 - 1,
            })
            .collect();
    }
    let seg = ROUND_SEGMENTS;
    let mut b = Builder::default();
    let rings: Vec<Vec<u32>> = prof
        .iter()
        .map(|p| {
            if p.x < 1e-9 {
                vec![b.point(c.x, c.y, p.y); 1]
            } else {
                (0..seg)
                    .map(|k| {
                        let a = std::f64::consts::TAU * k as f64 / seg as f64;
                        b.point(c.x + p.x * a.cos(), c.y + p.x * a.sin(), p.y)
                    })
                    .collect()
            }
        })
        .collect();
    let at = |r: &Vec<u32>, k: usize| if r.len() == 1 { r[0] } else { r[k % seg] };
    for j in 0..prof.len() - 1 {
        let (ra, rb) = (&rings[j], &rings[j + 1]);
        for k in 0..seg {
            b.quad(at(ra, k), at(ra, k + 1), at(rb, k + 1), at(rb, k), piece[j]);
        }
    }
    // The flat faces at the ends, where the profile is off the axis.
    let first = &rings[0];
    if first.len() > 1 {
        let mid = b.point(c.x, c.y, prof[0].y);
        for k in 0..seg {
            b.tri(mid, first[(k + 1) % seg], first[k], BOTTOM);
        }
    }
    let last = &rings[rings.len() - 1];
    if last.len() > 1 {
        let mid = b.point(c.x, c.y, prof[prof.len() - 1].y);
        for k in 0..seg {
            b.tri(mid, last[k], last[(k + 1) % seg], TOP);
        }
    }
    b.mesh
}

/// The footprint of a mass in plan: what its solid covers seen from
/// above (outlines counter-clockwise, with their holes).
pub fn footprint(m: &Mass) -> Vec<crate::junctions::Shape> {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;
    // A solid of triangles, or one twisted, bent or swept by a profile:
    // what its faces cover.
    let shaped = m.deform.is_some_and(|d| !d.is_none())
        || matches!(
            m.shape,
            MassShape::Sweep {
                profile: Some(_),
                ..
            }
        );
    let body;
    let shape = if shaped {
        body = MassShape::Mesh(mass_body(m));
        &body
    } else {
        &m.shape
    };
    match shape {
        MassShape::Mesh(mesh) => {
            // What a closed solid covers is what its faces looking up cover:
            // each of them, a hair larger so they overlap.
            let contours: Vec<Vec<[f64; 2]>> = mesh
                .triangles
                .iter()
                .filter_map(|t| {
                    let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
                    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let n = [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ];
                    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    if len < 1e-12 || n[2] / len < 0.02 {
                        return None;
                    }
                    let tri = [
                        Point2::new(a[0], a[1]),
                        Point2::new(b[0], b[1]),
                        Point2::new(c[0], c[1]),
                    ];
                    // Each corner a millimetre and a half out from the middle
                    // (an offset would shoot out at sharp corners).
                    let mid = (tri[0] + tri[1] + tri[2]) * (1.0 / 3.0);
                    let mut c: Vec<[f64; 2]> = tri
                        .iter()
                        .map(|p| {
                            let q = (*p - mid).normalized().map_or(*p, |d| *p + d * 0.0015);
                            [q.x, q.y]
                        })
                        .collect();
                    let area = (c[1][0] - c[0][0]) * (c[2][1] - c[0][1])
                        - (c[1][1] - c[0][1]) * (c[2][0] - c[0][0]);
                    if area < 0.0 {
                        c.reverse();
                    }
                    Some(c)
                })
                .collect();
            if contours.is_empty() {
                return Vec::new();
            }
            contours
                .overlay(
                    &Vec::<Vec<[f64; 2]>>::new(),
                    OverlayRule::Union,
                    FillRule::NonZero,
                )
                .into_iter()
                .filter_map(|shape| {
                    let mut it = shape.into_iter();
                    let to = |c: Vec<[f64; 2]>| -> Vec<Point2> {
                        c.into_iter().map(|p| Point2::new(p[0], p[1])).collect()
                    };
                    let outer = simplify(&crate::ensure_ccw(to(it.next()?)));
                    let holes = it
                        .map(|h| simplify(&to(h)))
                        .filter(|h| h.len() >= 3)
                        .collect();
                    (outer.len() >= 3).then_some((outer, holes))
                })
                .collect()
        }
        _ => {
            let o = m.outline();
            if o.len() < 3 {
                Vec::new()
            } else {
                vec![(crate::ensure_ccw(o), Vec::new())]
            }
        }
    }
}

/// A contour without the points that lie on a straight run (within a
/// couple of millimetres: what growing the faces left).
fn simplify(c: &[Point2]) -> Vec<Point2> {
    let mut out: Vec<Point2> = c.to_vec();
    let mut changed = true;
    while changed && out.len() > 3 {
        changed = false;
        let n = out.len();
        for i in 0..n {
            let (a, b, p) = (out[(i + n - 1) % n], out[(i + 1) % n], out[i]);
            let len = a.distance(b);
            let off = if len < 1e-12 {
                p.distance(a)
            } else {
                ((b - a).cross(p - a) / len).abs()
            };
            if off < 0.002 {
                out.remove(i);
                changed = true;
                break;
            }
        }
    }
    out
}

/// The face of a mass's solid at a point (`z` over the level's floor
/// raised by the mass's base), facing `normal`: the face of the triangle
/// it lies on.
pub fn face_at(m: &Mass, at: [f64; 3], normal: [f64; 3]) -> Option<u32> {
    let body = mass_body(m);
    let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if len < 1e-12 {
        return None;
    }
    let n = [normal[0] / len, normal[1] / len, normal[2] / len];
    let mut best: Option<(f64, u32)> = None;
    for (t, f) in body.triangles.iter().zip(&body.faces) {
        let [a, b, c] = t.map(|i| body.positions[i as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let tn = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let tl = (tn[0] * tn[0] + tn[1] * tn[1] + tn[2] * tn[2]).sqrt();
        if tl < 1e-12 {
            continue;
        }
        let tn = [tn[0] / tl, tn[1] / tl, tn[2] / tl];
        if tn[0] * n[0] + tn[1] * n[1] + tn[2] * n[2] < 0.5 {
            continue;
        }
        // Distance to the triangle's plane, and whether the point is over it.
        let w = [at[0] - a[0], at[1] - a[1], at[2] - a[2]];
        let d = (w[0] * tn[0] + w[1] * tn[1] + w[2] * tn[2]).abs();
        let (uu, uv, vv) = (dot3(u, u), dot3(u, v), dot3(v, v));
        let (wu, wv) = (dot3(w, u), dot3(w, v));
        let den = uv * uv - uu * vv;
        if den.abs() < 1e-18 {
            continue;
        }
        let s = (uv * wv - vv * wu) / den;
        let t2 = (uv * wu - uu * wv) / den;
        let inside = s >= -1e-3 && t2 >= -1e-3 && s + t2 <= 1.0 + 1e-3;
        if inside && best.is_none_or(|b| d < b.0) {
            best = Some((d, *f));
        }
    }
    best.filter(|b| b.0 < 0.05).map(|b| b.1)
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Why volumes could not be joined or cut.
#[derive(Debug, Clone, PartialEq)]
pub enum FormError {
    Missing,
    Building,
    NothingLeft,
    NothingInCommon,
    NoCrossing,
    NotDivided,
    NotFlat,
    Model(ModelError),
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FormError::Missing => "un volumen ya no existe",
            FormError::Building => "los volúmenes convertidos en edificio no se combinan",
            FormError::NothingLeft => "no queda nada del volumen",
            FormError::NothingInCommon => "los volúmenes no tienen nada en común",
            FormError::NoCrossing => "las caras de esos volúmenes no se cruzan",
            FormError::NotDivided => "la línea no divide esa cara",
            FormError::NotFlat => "solo se empujan caras planas",
            FormError::Model(e) => return write!(f, "{e}"),
        })
    }
}

impl std::error::Error for FormError {}

impl From<ModelError> for FormError {
    fn from(e: ModelError) -> Self {
        FormError::Model(e)
    }
}

/// Joins volumes to `target`, cuts them from it, or keeps what they have
/// in common: they go, and `target` becomes the solid made. Each face of
/// the result keeps the material it was seen with: the paint of the faces
/// painted, and the material of the volume it comes from (painted on it,
/// if it is not `target`'s).
pub fn combine_masses(
    model: &mut Model,
    target: MassId,
    others: &[MassId],
    op: crate::csg::Op,
) -> Result<(), FormError> {
    use crate::csg::{combine, Op};
    let t = model.mass(target).ok_or(FormError::Missing)?.clone();
    let level_z =
        |model: &Model, m: &Mass| model.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
    let paint_of = |model: &Model, m: &Mass, f: u32| {
        model
            .face_paint(m.id.0, m.face_ref(f))
            .unwrap_or(m.material)
    };
    if t.building.is_some() {
        return Err(FormError::Building);
    }
    let base = level_z(model, &t);
    let mut body = mass_body(&t);
    let mut materials: BTreeMap<u32, MaterialId> = body
        .face_ids()
        .into_iter()
        .map(|f| (f, paint_of(model, &t, f)))
        .collect();
    let mut next = materials.keys().max().map_or(0, |m| m + 1);
    for id in others {
        let o = model.mass(*id).ok_or(FormError::Missing)?;
        if o.building.is_some() {
            return Err(FormError::Building);
        }
        let ob = mass_body(o);
        for f in ob.face_ids() {
            materials.insert(f + next, paint_of(model, o, f));
        }
        let dz = level_z(model, o) - base;
        body = combine(&body, &ob, dz, next, op).ok_or(match op {
            Op::Intersect => FormError::NothingInCommon,
            _ => FormError::NothingLeft,
        })?;
        next += ob.face_ids().into_iter().max().map_or(0, |m| m + 1);
    }
    // The faces left, numbered 0, 1, 2…
    let renumber: BTreeMap<u32, u32> = body
        .face_ids()
        .into_iter()
        .enumerate()
        .map(|(i, f)| (f, i as u32))
        .collect();
    for f in &mut body.faces {
        *f = renumber[f];
    }
    let height = body.top();
    for id in others {
        model.remove_mass(*id)?;
    }
    model.edit_mass(target, |x| {
        x.shape = MassShape::Mesh(body);
        x.height = height;
        x.finish = None;
        x.floor_height = None;
    })?;
    // The old paint of `target` went with its old faces.
    let old: Vec<FaceRef> = model
        .face_paints()
        .filter(|(e, _, _)| *e == target.0)
        .map(|(_, f, _)| f)
        .collect();
    for f in old {
        let _ = model.set_face_paint(target.0, f, None);
    }
    for (old, new) in &renumber {
        let mat = materials.get(old).copied().unwrap_or(t.material);
        if mat != t.material {
            model.set_face_paint(target.0, FaceRef::Part(*new), Some(mat))?;
        }
    }
    Ok(())
}

/// Loose edges where the faces of the volumes cross each other
/// (SketchUp's «intersecar caras»): lines to divide the faces, to paint
/// them apart. Returns how many.
pub fn cross_masses(model: &mut Model, ids: &[MassId]) -> Result<usize, FormError> {
    let mut lines = Vec::new();
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            let (Some(ma), Some(mb)) = (model.mass(*a), model.mass(*b)) else {
                return Err(FormError::Missing);
            };
            let level_z = model.level(ma.level).map_or(0.0, |l| l.elevation);
            let za = level_z + ma.base_offset;
            let zb = model.level(mb.level).map_or(0.0, |l| l.elevation) + mb.base_offset;
            for line in crate::csg::crossings(&mass_body(ma), &mass_body(mb), zb - za) {
                let pts: Vec<Point2> = line.iter().map(|p| Point2::new(p[0], p[1])).collect();
                let hs: Vec<f64> = line.iter().map(|p| p[2] + za - level_z).collect();
                lines.push((ma.level, pts, hs));
            }
        }
    }
    if lines.is_empty() {
        return Err(FormError::NoCrossing);
    }
    let n = lines.len();
    for (level, pts, hs) in lines {
        model.add_edge_3d(level, pts, hs, 0.0)?;
    }
    Ok(n)
}

/// The volume of a closed mesh.
pub fn mesh_volume(m: &MassMesh) -> f64 {
    m.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{EdgeFinish, EdgeKind};

    fn square(s: f64) -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ]
    }

    /// Every edge is shared by two triangles, once each way.
    fn closed(m: &MassMesh) -> bool {
        use std::collections::HashMap;
        let mut count: HashMap<(u32, u32), i32> = HashMap::new();
        for t in &m.triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *count.entry((a, b)).or_default() += 1;
                *count.entry((b, a)).or_default() -= 1;
            }
        }
        count.values().all(|v| *v == 0)
    }

    #[test]
    fn prisms_with_finished_edges() {
        let plain = prism_body(&square(4.0), 3.0, None);
        assert!(closed(&plain));
        assert!((mesh_volume(&plain) - 48.0).abs() < 1e-9);
        for kind in [EdgeKind::Chamfer, EdgeKind::Round] {
            let f = EdgeFinish {
                kind,
                size: 0.5,
                vertical: true,
                top: true,
                bottom: false,
            };
            let m = prism_body(&square(4.0), 3.0, Some(f));
            assert!(closed(&m), "{kind:?}");
            let v = mesh_volume(&m);
            assert!(v > 40.0 && v < 48.0, "{kind:?} {v}");
            assert!(
                m.faces.contains(&TOP)
                    && m.faces.contains(&BOTTOM)
                    && m.faces.contains(&(SIDE + 3))
            );
        }
    }

    #[test]
    fn round_forms() {
        let r = 2.0;
        let s = sphere(Point2::new(0.0, 0.0), r);
        assert!(closed(&s));
        let exact = 4.0 / 3.0 * std::f64::consts::PI * r * r * r;
        assert!((mesh_volume(&s) - exact).abs() / exact < 0.05);
        let cyl = revolve(
            Point2::new(1.0, 1.0),
            &[Point2::new(r, 0.0), Point2::new(r, 3.0)],
            None,
        );
        assert!(closed(&cyl));
        let exact = std::f64::consts::PI * r * r * 3.0;
        assert!((mesh_volume(&cyl) - exact).abs() / exact < 0.02);
        // Drawn from the top down, it is the same solid.
        let down = revolve(
            Point2::new(1.0, 1.0),
            &[Point2::new(r, 3.0), Point2::new(r, 0.0)],
            None,
        );
        assert!((mesh_volume(&down) - mesh_volume(&cyl)).abs() < 1e-9);
        // A cone reaches the axis: no top face.
        let cone = revolve(
            Point2::new(0.0, 0.0),
            &[Point2::new(r, 0.0), Point2::new(0.0, 3.0)],
            None,
        );
        assert!(closed(&cone));
        assert!(!cone.faces.contains(&TOP));
        // A vase: three pieces of profile, three faces round it.
        let vase = revolve(
            Point2::new(0.0, 0.0),
            &[
                Point2::new(0.5, 0.0),
                Point2::new(1.0, 0.6),
                Point2::new(0.4, 1.2),
                Point2::new(0.6, 1.5),
            ],
            None,
        );
        assert!(closed(&vase));
        assert!(vase.faces.contains(&(SIDE + 2)));
        // A cylinder with its top edge rounded.
        let f = EdgeFinish {
            kind: EdgeKind::Round,
            size: 0.3,
            vertical: false,
            top: true,
            bottom: true,
        };
        let soft = revolve(
            Point2::new(0.0, 0.0),
            &[Point2::new(r, 0.0), Point2::new(r, 3.0)],
            Some(f),
        );
        assert!(closed(&soft));
        assert!(mesh_volume(&soft) < exact);
        assert!(soft.faces.contains(&TOP) && soft.faces.contains(&BOTTOM));
    }
}
