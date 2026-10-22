//! Joining, cutting and intersecting solids (constructive solid
//! geometry), with binary space partitioning trees: each solid's polygons
//! are sorted by the planes of its own faces, and each solid clips the
//! other's polygons against those planes, keeping what is outside (union),
//! inside (intersection) or outside of one and inside the other
//! (subtraction). The classic method for polygon soups, written for
//! ArchiSim. Faces keep their number, so a cut face can still be painted
//! as it was, and the other solid's faces come with theirs.
//!
//! Also the lines where the faces of two solids cross ("intersecar
//! caras").

use archisim_core::MassMesh;
use std::collections::HashMap;

type P3 = [f64; 3];

/// Points closer than this to a plane are on it (metres).
const EPS: f64 = 1e-6;

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn lerp(a: P3, b: P3, t: f64) -> P3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[derive(Debug, Clone, Copy)]
struct Plane {
    n: P3,
    w: f64,
}

impl Plane {
    fn of(a: P3, b: P3, c: P3) -> Option<Plane> {
        let n = cross(sub(b, a), sub(c, a));
        let len = dot(n, n).sqrt();
        if len < 1e-12 {
            return None;
        }
        let n = [n[0] / len, n[1] / len, n[2] / len];
        Some(Plane { n, w: dot(n, a) })
    }

    fn flip(&mut self) {
        self.n = [-self.n[0], -self.n[1], -self.n[2]];
        self.w = -self.w;
    }
}

/// A convex polygon, counter-clockwise seen from outside.
#[derive(Debug, Clone)]
struct Poly {
    v: Vec<P3>,
    plane: Plane,
    face: u32,
}

impl Poly {
    fn flip(&mut self) {
        self.v.reverse();
        self.plane.flip();
    }
}

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;

/// Puts `p` (split if it spans the plane) in the lists it belongs to.
fn split(
    plane: &Plane,
    p: Poly,
    co_front: &mut Vec<Poly>,
    co_back: &mut Vec<Poly>,
    front: &mut Vec<Poly>,
    back: &mut Vec<Poly>,
) {
    let mut kind = 0u8;
    let types: Vec<u8> =
        p.v.iter()
            .map(|v| {
                let t = dot(plane.n, *v) - plane.w;
                let k = if t < -EPS {
                    BACK
                } else if t > EPS {
                    FRONT
                } else {
                    COPLANAR
                };
                kind |= k;
                k
            })
            .collect();
    match kind {
        COPLANAR => {
            if dot(plane.n, p.plane.n) > 0.0 {
                co_front.push(p);
            } else {
                co_back.push(p);
            }
        }
        FRONT => front.push(p),
        BACK => back.push(p),
        _ => {
            let (mut f, mut b) = (Vec::new(), Vec::new());
            let n = p.v.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let (ti, tj) = (types[i], types[j]);
                let (vi, vj) = (p.v[i], p.v[j]);
                if ti != BACK {
                    f.push(vi);
                }
                if ti != FRONT {
                    b.push(vi);
                }
                if (ti | tj) == SPANNING {
                    let t = (plane.w - dot(plane.n, vi)) / dot(plane.n, sub(vj, vi));
                    let v = lerp(vi, vj, t);
                    f.push(v);
                    b.push(v);
                }
            }
            if f.len() >= 3 {
                front.push(Poly {
                    v: f,
                    plane: p.plane,
                    face: p.face,
                });
            }
            if b.len() >= 3 {
                back.push(Poly {
                    v: b,
                    plane: p.plane,
                    face: p.face,
                });
            }
        }
    }
}

/// A node of the tree: a plane, the polygons on it, and what is in front
/// of it and behind it.
#[derive(Default)]
struct Node {
    plane: Option<Plane>,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
    polys: Vec<Poly>,
}

impl Node {
    fn new(polys: Vec<Poly>) -> Node {
        let mut n = Node::default();
        n.build(polys);
        n
    }

    fn invert(&mut self) {
        for p in &mut self.polys {
            p.flip();
        }
        if let Some(pl) = &mut self.plane {
            pl.flip();
        }
        if let Some(f) = &mut self.front {
            f.invert();
        }
        if let Some(b) = &mut self.back {
            b.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    /// The polygons of `polys` outside this solid.
    fn clip_polys(&self, polys: Vec<Poly>) -> Vec<Poly> {
        let Some(plane) = self.plane else {
            return polys;
        };
        let (mut front, mut back) = (Vec::new(), Vec::new());
        for p in polys {
            let (mut cf, mut cb) = (Vec::new(), Vec::new());
            split(&plane, p, &mut cf, &mut cb, &mut front, &mut back);
            front.extend(cf);
            back.extend(cb);
        }
        let mut front = match &self.front {
            Some(f) => f.clip_polys(front),
            None => front,
        };
        let back = match &self.back {
            Some(b) => b.clip_polys(back),
            None => Vec::new(),
        };
        front.extend(back);
        front
    }

    /// Takes out of this tree what is inside `other`.
    fn clip_to(&mut self, other: &Node) {
        self.polys = other.clip_polys(std::mem::take(&mut self.polys));
        if let Some(f) = &mut self.front {
            f.clip_to(other);
        }
        if let Some(b) = &mut self.back {
            b.clip_to(other);
        }
    }

    fn all(&self) -> Vec<Poly> {
        let mut out = self.polys.clone();
        if let Some(f) = &self.front {
            out.extend(f.all());
        }
        if let Some(b) = &self.back {
            out.extend(b.all());
        }
        out
    }

    fn build(&mut self, polys: Vec<Poly>) {
        if polys.is_empty() {
            return;
        }
        let plane = *self.plane.get_or_insert(polys[0].plane);
        let (mut front, mut back) = (Vec::new(), Vec::new());
        let mut on = Vec::new();
        for p in polys {
            let mut cb = Vec::new();
            split(&plane, p, &mut on, &mut cb, &mut front, &mut back);
            on.extend(cb);
        }
        self.polys.extend(on);
        if !front.is_empty() {
            self.front.get_or_insert_with(Default::default).build(front);
        }
        if !back.is_empty() {
            self.back.get_or_insert_with(Default::default).build(back);
        }
    }
}

/// What to do with two solids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Union,
    Subtract,
    Intersect,
}

fn polys_of(m: &MassMesh, dz: f64, face_offset: u32) -> Vec<Poly> {
    m.triangles
        .iter()
        .zip(&m.faces)
        .filter_map(|(t, f)| {
            let v: Vec<P3> = t
                .iter()
                .map(|i| {
                    let p = m.positions[*i as usize];
                    [p[0], p[1], p[2] + dz]
                })
                .collect();
            let plane = Plane::of(v[0], v[1], v[2])?;
            Some(Poly {
                v,
                plane,
                face: f + face_offset,
            })
        })
        .collect()
}

fn mesh_of(polys: &[Poly]) -> MassMesh {
    let mut out = MassMesh::default();
    let mut index: HashMap<[i64; 3], u32> = HashMap::new();
    let mut id = |p: P3, out: &mut MassMesh| -> u32 {
        let key = p.map(|c| (c * 1e6).round() as i64);
        *index.entry(key).or_insert_with(|| {
            out.positions.push(p);
            (out.positions.len() - 1) as u32
        })
    };
    for p in polys {
        let ids: Vec<u32> = p.v.iter().map(|v| id(*v, &mut out)).collect();
        for k in 1..ids.len() - 1 {
            let t = [ids[0], ids[k], ids[k + 1]];
            if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] {
                out.triangles.push(t);
                out.faces.push(p.face);
            }
        }
    }
    out
}

/// A contour without the points on a straight run (left where the
/// pieces of a face met), so it triangulates without slivers.
fn straight(c: &[archisim_core::Point2]) -> Vec<archisim_core::Point2> {
    let mut out = c.to_vec();
    let mut i = 0;
    while out.len() > 3 && i < out.len() {
        let n = out.len();
        let (a, p, b) = (out[(i + n - 1) % n], out[i], out[(i + 1) % n]);
        let len = a.distance(b);
        let off = if len < 1e-12 {
            p.distance(a)
        } else {
            ((b - a).cross(p - a) / len).abs()
        };
        // On the line between its neighbours (and between them, not past).
        let along = (p - a).dot(b - a);
        if off < 2e-5 && along >= -1e-12 && along <= len * len + 1e-12 {
            out.remove(i);
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    out
}

/// `a` joined with, cut by or intersected with `b`. `b` is `dz` above
/// `a`'s base (both in the same plan coordinates); its faces are numbered
/// from `b_faces` on. Returns `None` if nothing is left.
pub fn combine(a: &MassMesh, b: &MassMesh, dz: f64, b_faces: u32, op: Op) -> Option<MassMesh> {
    let mut na = Node::new(polys_of(a, 0.0, 0));
    let mut nb = Node::new(polys_of(b, dz, b_faces));
    match op {
        Op::Union => {
            na.clip_to(&nb);
            nb.clip_to(&na);
            nb.invert();
            nb.clip_to(&na);
            nb.invert();
            na.build(nb.all());
        }
        Op::Subtract => {
            na.invert();
            na.clip_to(&nb);
            nb.clip_to(&na);
            nb.invert();
            nb.clip_to(&na);
            nb.invert();
            na.build(nb.all());
            na.invert();
        }
        Op::Intersect => {
            na.invert();
            nb.clip_to(&na);
            nb.invert();
            na.clip_to(&nb);
            nb.clip_to(&na);
            na.build(nb.all());
            na.invert();
        }
    }
    let m = clean(&na.all());
    (m.triangles.len() >= 4).then_some(m)
}

/// The pieces left by clipping, each face made whole again: the pieces of
/// a face on one plane are joined (in the plane) and triangulated anew,
/// so no slivers or lines inside a face are left.
fn clean(polys: &[Poly]) -> MassMesh {
    use archisim_core::Point2;
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;
    // Pieces by face and plane.
    let mut groups: HashMap<(u32, [i64; 4]), Vec<&Poly>> = HashMap::new();
    for p in polys {
        let n = p.plane.n;
        let key = [
            (n[0] * 1e4).round() as i64,
            (n[1] * 1e4).round() as i64,
            (n[2] * 1e4).round() as i64,
            (p.plane.w * 1e4).round() as i64,
        ];
        groups.entry((p.face, key)).or_default().push(p);
    }
    let mut keys: Vec<_> = groups.keys().copied().collect();
    keys.sort();
    let mut out: Vec<Poly> = Vec::new();
    for key in keys {
        let group = &groups[&key];
        if group.len() == 1 {
            out.push(group[0].clone());
            continue;
        }
        let plane = group[0].plane;
        let n = plane.n;
        // A frame on the plane: u, v and n turning counter-clockwise.
        let helper = if n[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let u = {
            let c = cross(helper, n);
            let l = dot(c, c).sqrt();
            [c[0] / l, c[1] / l, c[2] / l]
        };
        let v = cross(n, u);
        let origin = [n[0] * plane.w, n[1] * plane.w, n[2] * plane.w];
        let flat = |q: P3| {
            let d = sub(q, origin);
            [dot(d, u), dot(d, v)]
        };
        let lift = |p: Point2| {
            [
                origin[0] + u[0] * p.x + v[0] * p.y,
                origin[1] + u[1] * p.x + v[1] * p.y,
                origin[2] + u[2] * p.x + v[2] * p.y,
            ]
        };
        let contours: Vec<Vec<[f64; 2]>> = group
            .iter()
            .map(|p| p.v.iter().map(|q| flat(*q)).collect())
            .collect();
        let shapes = contours.overlay(
            &Vec::<Vec<[f64; 2]>>::new(),
            OverlayRule::Union,
            FillRule::NonZero,
        );
        let mut made = Vec::new();
        for shape in shapes {
            let mut it = shape.into_iter();
            let Some(outer) = it.next() else { continue };
            let to = |c: Vec<[f64; 2]>| -> Vec<Point2> {
                c.into_iter().map(|p| Point2::new(p[0], p[1])).collect()
            };
            // Holes of a hair's area are left by rounding, not by the solids.
            let holes: Vec<Vec<Point2>> = it
                .map(|h| straight(&to(h)))
                .filter(|h| h.len() >= 3 && archisim_core::signed_area(h).abs() > 1e-6)
                .collect();
            let outer = straight(&to(outer));
            if outer.len() < 3 {
                continue;
            }
            let poly = crate::solid::with_holes(&outer, &holes);
            for [a, b, c] in crate::solid::triangulate(&poly) {
                made.push(Poly {
                    v: vec![lift(poly[a]), lift(poly[b]), lift(poly[c])],
                    plane,
                    face: key.0,
                });
            }
        }
        // If the join went wrong, the pieces as they were.
        let area = |ps: &[&Poly]| -> f64 {
            ps.iter()
                .map(|p| {
                    (1..p.v.len() - 1)
                        .map(|k| {
                            let c = cross(sub(p.v[k], p.v[0]), sub(p.v[k + 1], p.v[0]));
                            dot(c, c).sqrt() / 2.0
                        })
                        .sum::<f64>()
                })
                .sum()
        };
        let before = area(group);
        let after = area(&made.iter().collect::<Vec<_>>());
        if made.is_empty() || (after - before).abs() > 1e-4 * before.max(1e-6) + 1e-6 {
            out.extend(group.iter().map(|p| (*p).clone()));
        } else {
            out.extend(made);
        }
    }
    mesh_of(&out)
}

/// The lines where the faces of `a` and of `b` (`dz` above it) cross, as
/// polylines of points in `a`'s coordinates.
pub fn crossings(a: &MassMesh, b: &MassMesh, dz: f64) -> Vec<Vec<P3>> {
    let ta = polys_of(a, 0.0, 0);
    let tb = polys_of(b, dz, 0);
    let bbox = |p: &Poly| {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for v in &p.v {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        (lo, hi)
    };
    let boxes_b: Vec<_> = tb.iter().map(bbox).collect();
    let mut segments: Vec<(P3, P3)> = Vec::new();
    for pa in &ta {
        let (la, ha) = bbox(pa);
        for (pb, (lb, hb)) in tb.iter().zip(&boxes_b) {
            if (0..3).any(|k| ha[k] < lb[k] - EPS || hb[k] < la[k] - EPS) {
                continue;
            }
            if let Some(s) = tri_cross(pa, pb) {
                segments.push(s);
            }
        }
    }
    chain(segments)
}

/// Where a polygon meets a plane: the stretch of the line of the two
/// planes inside it, as parameters along `dir` from `origin`.
fn span_on(p: &Poly, plane: &Plane, origin: P3, dir: P3) -> Option<(f64, f64)> {
    let d: Vec<f64> = p.v.iter().map(|v| dot(plane.n, *v) - plane.w).collect();
    let mut ts = Vec::new();
    let n = p.v.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if d[i].abs() <= EPS {
            ts.push(dot(sub(p.v[i], origin), dir));
        }
        if (d[i] > EPS && d[j] < -EPS) || (d[i] < -EPS && d[j] > EPS) {
            let t = d[i] / (d[i] - d[j]);
            ts.push(dot(sub(lerp(p.v[i], p.v[j], t), origin), dir));
        }
    }
    if ts.len() < 2 {
        return None;
    }
    let lo = ts.iter().copied().fold(f64::MAX, f64::min);
    let hi = ts.iter().copied().fold(f64::MIN, f64::max);
    Some((lo, hi))
}

fn tri_cross(a: &Poly, b: &Poly) -> Option<(P3, P3)> {
    let dir = cross(a.plane.n, b.plane.n);
    let len = dot(dir, dir).sqrt();
    if len < 1e-9 {
        return None; // parallel or on one plane
    }
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    let origin = a.v[0];
    let (a0, a1) = span_on(a, &b.plane, origin, dir)?;
    let (b0, b1) = span_on(b, &a.plane, origin, dir)?;
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    if hi - lo < 1e-5 {
        return None;
    }
    // Back to points: on the line through a point of both planes.
    let p0 = point_on_both(&a.plane, &b.plane, dir)?;
    let shift = dot(sub(p0, origin), dir);
    let at = |t: f64| {
        let s = t - shift;
        [p0[0] + dir[0] * s, p0[1] + dir[1] * s, p0[2] + dir[2] * s]
    };
    Some((at(lo), at(hi)))
}

/// A point on the line where two planes meet (`dir` along it).
fn point_on_both(a: &Plane, b: &Plane, dir: P3) -> Option<P3> {
    // p = (a.w (b.n × dir) + b.w (dir × a.n)) / (dir · (a.n × b.n))
    let den = dot(dir, cross(a.n, b.n));
    if den.abs() < 1e-12 {
        return None;
    }
    let u = cross(b.n, dir);
    let v = cross(dir, a.n);
    Some([
        (a.w * u[0] + b.w * v[0]) / den,
        (a.w * u[1] + b.w * v[1]) / den,
        (a.w * u[2] + b.w * v[2]) / den,
    ])
}

/// Segments joined end to end into polylines.
fn chain(mut segs: Vec<(P3, P3)>) -> Vec<Vec<P3>> {
    let near = |a: P3, b: P3| dot(sub(a, b), sub(a, b)) < 1e-8;
    let mut out = Vec::new();
    while let Some((a, b)) = segs.pop() {
        let mut line = vec![a, b];
        loop {
            let end = line[line.len() - 1];
            let start = line[0];
            if let Some(i) = segs
                .iter()
                .position(|(p, q)| near(*p, end) || near(*q, end))
            {
                let (p, q) = segs.swap_remove(i);
                line.push(if near(p, end) { q } else { p });
            } else if let Some(i) = segs
                .iter()
                .position(|(p, q)| near(*p, start) || near(*q, start))
            {
                let (p, q) = segs.swap_remove(i);
                line.insert(0, if near(p, start) { q } else { p });
            } else {
                break;
            }
        }
        // Points along a straight run are left out.
        let mut clean: Vec<P3> = Vec::new();
        for p in line {
            if clean.last().is_some_and(|q| near(*q, p)) {
                continue;
            }
            if clean.len() >= 2 {
                let (a, b) = (clean[clean.len() - 2], clean[clean.len() - 1]);
                let c = cross(sub(b, a), sub(p, b));
                if dot(c, c) < 1e-12 && dot(sub(b, a), sub(p, b)) > 0.0 {
                    clean.pop();
                }
            }
            clean.push(p);
        }
        if clean.len() >= 2 {
            out.push(clean);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::mesh_volume;

    fn cube(x: f64, y: f64, z: f64, s: f64) -> MassMesh {
        let mut m = MassMesh::default();
        for k in 0..8 {
            m.positions.push([
                x + s * (k & 1) as f64,
                y + s * ((k >> 1) & 1) as f64,
                z + s * ((k >> 2) & 1) as f64,
            ]);
        }
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        for (f, q) in quads.iter().enumerate() {
            m.triangles.push([q[0], q[1], q[2]]);
            m.triangles.push([q[0], q[2], q[3]]);
            m.faces.push(f as u32);
            m.faces.push(f as u32);
        }
        m
    }

    #[test]
    fn cubes_joined_cut_and_intersected() {
        let a = cube(0.0, 0.0, 0.0, 2.0);
        let b = cube(1.0, 1.0, 1.0, 2.0);
        assert!((mesh_volume(&a) - 8.0).abs() < 1e-9);
        let u = combine(&a, &b, 0.0, 10, Op::Union).unwrap();
        assert!((mesh_volume(&u) - 15.0).abs() < 1e-6, "{}", mesh_volume(&u));
        let s = combine(&a, &b, 0.0, 10, Op::Subtract).unwrap();
        assert!((mesh_volume(&s) - 7.0).abs() < 1e-6, "{}", mesh_volume(&s));
        let i = combine(&a, &b, 0.0, 10, Op::Intersect).unwrap();
        assert!((mesh_volume(&i) - 1.0).abs() < 1e-6, "{}", mesh_volume(&i));
        // The cut faces come from the other solid, with its numbers.
        assert!(s.faces.iter().any(|f| *f >= 10) && s.faces.iter().any(|f| *f < 10));
        // `b` given by its base: the same cut.
        let b0 = cube(1.0, 1.0, 0.0, 2.0);
        let s2 = combine(&a, &b0, 1.0, 10, Op::Subtract).unwrap();
        assert!((mesh_volume(&s2) - 7.0).abs() < 1e-6);
        // Apart: nothing in common.
        let far = cube(5.0, 5.0, 5.0, 1.0);
        assert!(combine(&a, &far, 0.0, 10, Op::Intersect).is_none());
    }

    #[test]
    fn lines_where_faces_cross() {
        let a = cube(0.0, 0.0, 0.0, 2.0);
        let b = cube(1.0, 1.0, 1.0, 2.0);
        let lines = crossings(&a, &b, 0.0);
        let total: f64 = lines
            .iter()
            .flat_map(|l| l.windows(2))
            .map(|w| dot(sub(w[1], w[0]), sub(w[1], w[0])).sqrt())
            .sum();
        // Six unit edges where the two cubes' faces cross.
        assert!((total - 6.0).abs() < 1e-6, "{total} {lines:?}");
    }
}

#[cfg(test)]
mod finished_tests {
    use super::*;
    use crate::forms::{mass_body, mesh_volume};
    use archisim_core::{EdgeFinish, EdgeKind, Mass, MassShape, Point2, Primitive};

    fn mass(shape: MassShape, h: f64, finish: Option<EdgeFinish>) -> Mass {
        Mass {
            id: archisim_core::MassId(1),
            guid: Default::default(),
            name: String::new(),
            level: archisim_core::LevelId(1),
            shape,
            base_offset: 0.0,
            height: h,
            floor_height: None,
            material: archisim_core::MaterialId(1),
            building: None,
            finish,
        }
    }

    #[test]
    fn a_rounded_block_cut_by_a_cylinder() {
        let block = mass(
            MassShape::Prism(vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 8.0),
                Point2::new(0.0, 8.0),
            ]),
            6.0,
            Some(EdgeFinish {
                kind: EdgeKind::Round,
                size: 0.6,
                vertical: true,
                top: true,
                bottom: false,
            }),
        );
        let cyl = mass(
            MassShape::Primitive(Primitive::Cylinder {
                center: Point2::new(10.0, 8.0),
                radius: 3.0,
            }),
            8.0,
            None,
        );
        let a = mass_body(&block);
        let b = mass_body(&cyl);
        let va = mesh_volume(&a);
        let cut = combine(&a, &b, 0.0, 100, Op::Subtract).unwrap();
        let v = mesh_volume(&cut);
        // A quarter of the cylinder, 6 m high, comes out (a little less,
        // where the corner was already rounded).
        let quarter = std::f64::consts::PI * 9.0 / 4.0 * 6.0;
        assert!((va - v - quarter).abs() < 1.5, "{va} {v} {quarter}");
        let mut m = block.clone();
        m.shape = MassShape::Mesh(cut.clone());
        // Its footprint: one outline, without slivers or holes.
        let f = crate::forms::footprint(&m);
        assert_eq!(f.len(), 1);
        assert!(f[0].1.is_empty());
        let area = archisim_core::signed_area(&f[0].0);
        // 10 × 8, less a quarter of a circle of 3 m and three rounded
        // corners.
        let expected = 80.0
            - std::f64::consts::PI * 9.0 / 4.0
            - 3.0 * 0.36 * (1.0 - std::f64::consts::FRAC_PI_4);
        assert!((area - expected).abs() < 0.3, "{area} {expected}");
        assert!(f[0].0.len() < 80, "{}", f[0].0.len());
        // Every point of the result is outside the cylinder (or on it).
        for p in &cut.positions {
            let d = ((p[0] - 10.0).powi(2) + (p[1] - 8.0).powi(2)).sqrt();
            assert!(d > 3.0 * 0.98 - 1e-6, "{p:?} at {d}");
        }
    }
}

#[cfg(test)]
mod joined_tests {
    use super::*;
    use crate::forms::mass_body;
    use archisim_core::{EdgeFinish, EdgeKind, Mass, MassShape, Point2, Primitive};

    /// A ball joined on a chamfered base: the base's sides, untouched by
    /// the ball, come out whole (a few triangles, none of them slivers).
    #[test]
    fn faces_come_out_whole() {
        let mk = |shape, h, finish| Mass {
            id: archisim_core::MassId(1),
            guid: Default::default(),
            name: String::new(),
            level: archisim_core::LevelId(1),
            shape,
            base_offset: 0.0,
            height: h,
            floor_height: None,
            material: archisim_core::MaterialId(1),
            building: None,
            finish,
        };
        let base = mk(
            MassShape::Prism(vec![
                Point2::new(14.0, 0.0),
                Point2::new(20.0, 0.0),
                Point2::new(20.0, 6.0),
                Point2::new(14.0, 6.0),
            ]),
            3.0,
            Some(EdgeFinish {
                kind: EdgeKind::Chamfer,
                size: 0.4,
                vertical: true,
                top: true,
                bottom: false,
            }),
        );
        let ball = mk(
            MassShape::Primitive(Primitive::Sphere {
                center: Point2::new(17.0, 3.0),
                radius: 2.5,
            }),
            5.0,
            None,
        );
        let r = combine(&mass_body(&base), &mass_body(&ball), 1.5, 100, Op::Union).unwrap();
        for side in 2..6 {
            let n = r.faces.iter().filter(|f| **f == side).count();
            assert!(n <= 40, "side {side}: {n} triangles");
        }
    }
}
