//! Clash detection between two sets of elements (the project and a linked
//! model, or two linked models): which elements get into each other more
//! than a tolerance, where, and how deep.
//!
//! Each element is a closed triangle mesh. Points along the edges of one
//! element (and its corners) are tested against the other: a point inside
//! it, farther than the tolerance from its surface and not inside one of
//! its openings, is a penetration. Elements that only touch (a wall on a
//! slab, a beam against a column face) never clash; a pipe through an
//! opening that was left for it does not either.

use std::collections::{HashMap, HashSet};

use archisim_ifc::import::geometry::{cross, dot, sub, P3};

use crate::elements::{Element, ModelElements};
use crate::mesh::Tri;

/// A clash between an element of one set and one of the other.
#[derive(Debug, Clone)]
pub struct Clash {
    pub a: usize,
    pub b: usize,
    /// The deepest point found, in metres.
    pub point: P3,
    /// How far that point is inside the other element.
    pub depth: f64,
}

#[derive(Debug, Clone)]
pub struct ClashOptions {
    /// Penetrations up to this depth are ignored (contact, rounding).
    pub tolerance: f64,
    /// Classes never checked (spaces, furniture…), upper case.
    pub ignore_classes: HashSet<String>,
}

impl Default for ClashOptions {
    fn default() -> ClashOptions {
        ClashOptions {
            tolerance: 0.01,
            ignore_classes: [
                "IFCFURNISHINGELEMENT",
                "IFCFURNITURE",
                "IFCBUILDINGELEMENTPROXY",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

/// A body prepared for the tests: its triangles, bounds and the sample
/// points of its edges.
pub(crate) struct Prepared<'e> {
    el: &'e Element,
    lo: P3,
    hi: P3,
    samples: Vec<P3>,
}

pub(crate) fn prepare(el: &Element, step: f64) -> Option<Prepared<'_>> {
    let (lo, hi) = el.bounds?;
    let mut seen: HashSet<([u64; 3], [u64; 3])> = HashSet::new();
    let mut samples = Vec::new();
    let key = |p: P3| p.map(f64::to_bits);
    for t in &el.mesh.triangles {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            let k = if ka <= kb { (ka, kb) } else { (kb, ka) };
            if !seen.insert(k) {
                continue;
            }
            let d = sub(b, a);
            let len = dot(d, d).sqrt();
            let n = ((len / step).ceil() as usize).clamp(1, 2000);
            for s in 0..n {
                let f = s as f64 / n as f64;
                samples.push([a[0] + d[0] * f, a[1] + d[1] * f, a[2] + d[2] * f]);
            }
        }
    }
    Some(Prepared {
        el,
        lo,
        hi,
        samples,
    })
}

fn overlap(a: &Prepared, b: &Prepared, tol: f64) -> bool {
    (0..3).all(|i| a.lo[i] < b.hi[i] - tol && b.lo[i] < a.hi[i] - tol)
}

/// Möller–Trumbore: distance along the ray to the triangle, if hit.
fn ray_hit(o: P3, d: P3, t: &Tri) -> Option<f64> {
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-14 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(o, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    (t > 1e-12).then_some(t)
}

/// Whether a point is inside a closed mesh: three rays in skew directions,
/// majority of odd crossings (robust against rays grazing an edge).
pub fn inside(p: P3, tris: &[Tri]) -> bool {
    const DIRS: [P3; 3] = [
        [0.577_350_3, 0.577_350_2, 0.577_350_3],
        [-0.693_361_3, 0.196_116_1, 0.693_361_3],
        [0.110_431_2, -0.883_449_5, 0.455_642_1],
    ];
    let mut votes = 0;
    for d in DIRS {
        let n = tris.iter().filter(|t| ray_hit(p, d, t).is_some()).count();
        if n % 2 == 1 {
            votes += 1;
        }
    }
    votes >= 2
}

/// Distance from a point to a triangle.
pub(crate) fn point_triangle(p: P3, t: &Tri) -> f64 {
    // Ericson, Real-Time Collision Detection 5.1.5.
    let (a, b, c) = (t[0], t[1], t[2]);
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    let closest = if d1 <= 0.0 && d2 <= 0.0 {
        a
    } else {
        let bp = sub(p, b);
        let d3 = dot(ab, bp);
        let d4 = dot(ac, bp);
        if d3 >= 0.0 && d4 <= d3 {
            b
        } else {
            let vc = d1 * d4 - d3 * d2;
            if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
                let v = d1 / (d1 - d3);
                [a[0] + ab[0] * v, a[1] + ab[1] * v, a[2] + ab[2] * v]
            } else {
                let cp = sub(p, c);
                let d5 = dot(ab, cp);
                let d6 = dot(ac, cp);
                if d6 >= 0.0 && d5 <= d6 {
                    c
                } else {
                    let vb = d5 * d2 - d1 * d6;
                    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
                        let w = d2 / (d2 - d6);
                        [a[0] + ac[0] * w, a[1] + ac[1] * w, a[2] + ac[2] * w]
                    } else {
                        let va = d3 * d6 - d5 * d4;
                        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
                            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
                            let bc = sub(c, b);
                            [b[0] + bc[0] * w, b[1] + bc[1] * w, b[2] + bc[2] * w]
                        } else {
                            let denom = 1.0 / (va + vb + vc);
                            let v = vb * denom;
                            let w = vc * denom;
                            [
                                a[0] + ab[0] * v + ac[0] * w,
                                a[1] + ab[1] * v + ac[1] * w,
                                a[2] + ab[2] * v + ac[2] * w,
                            ]
                        }
                    }
                }
            }
        }
    };
    let d = sub(p, closest);
    dot(d, d).sqrt()
}

/// How deep the points of `a` get into `b` (the deepest one), counting
/// only points farther than `tol` from its surface and outside its openings.
pub(crate) fn penetration(a: &Prepared, b: &Prepared, tol: f64) -> Option<(P3, f64)> {
    let tris = &b.el.mesh.triangles;
    let mut best: Option<(P3, f64)> = None;
    for &p in &a.samples {
        if (0..3).any(|i| p[i] <= b.lo[i] + tol || p[i] >= b.hi[i] - tol) {
            continue;
        }
        if !inside(p, tris) {
            continue;
        }
        let depth = tris
            .iter()
            .map(|t| point_triangle(p, t))
            .fold(f64::INFINITY, f64::min);
        if depth <= tol || best.is_some_and(|(_, d)| d >= depth) {
            continue;
        }
        // In an opening, or on its edge (a door frame along its head).
        let in_void = |v: &Vec<Tri>| {
            inside(p, v)
                || v.iter()
                    .map(|t| point_triangle(p, t))
                    .fold(f64::INFINITY, f64::min)
                    <= tol
        };
        if b.el.voids.iter().any(in_void) || a.el.voids.iter().any(in_void) {
            continue;
        }
        best = Some((p, depth));
    }
    best
}

/// Clashes between every element of `left` and every element of `right`.
pub fn find_clashes(
    left: &ModelElements,
    right: &ModelElements,
    opts: &ClashOptions,
) -> Vec<Clash> {
    // Below half a millimetre, contact would count as a clash.
    let opts = &ClashOptions {
        tolerance: opts.tolerance.max(0.0005),
        ignore_classes: opts.ignore_classes.clone(),
    };
    let step = (opts.tolerance * 2.0).clamp(0.01, 0.05);
    let keep = |e: &Element| !opts.ignore_classes.contains(&e.class);
    let a: Vec<(usize, Prepared)> = left
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| keep(e))
        .filter_map(|(i, e)| prepare(e, step).map(|p| (i, p)))
        .collect();
    let b: Vec<(usize, Prepared)> = right
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| keep(e))
        .filter_map(|(i, e)| prepare(e, step).map(|p| (i, p)))
        .collect();
    // Broad phase: a grid of 2 m cells in plan.
    const CELL: f64 = 2.0;
    let cell = |v: f64| (v / CELL).floor() as i64;
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (k, (_, p)) in b.iter().enumerate() {
        for x in cell(p.lo[0])..=cell(p.hi[0]) {
            for y in cell(p.lo[1])..=cell(p.hi[1]) {
                grid.entry((x, y)).or_default().push(k);
            }
        }
    }
    let mut out = Vec::new();
    for (ia, pa) in &a {
        let mut cands: Vec<usize> = Vec::new();
        for x in cell(pa.lo[0])..=cell(pa.hi[0]) {
            for y in cell(pa.lo[1])..=cell(pa.hi[1]) {
                if let Some(v) = grid.get(&(x, y)) {
                    cands.extend(v);
                }
            }
        }
        cands.sort_unstable();
        cands.dedup();
        for k in cands {
            let (ib, pb) = &b[k];
            // The same element in both (a copy of the model linked): not a clash.
            if pa.el.global_id == pb.el.global_id || !overlap(pa, pb, opts.tolerance) {
                continue;
            }
            let hit = match (
                penetration(pa, pb, opts.tolerance),
                penetration(pb, pa, opts.tolerance),
            ) {
                (Some(x), Some(y)) => Some(if x.1 >= y.1 { x } else { y }),
                (x, y) => x.or(y),
            };
            if let Some((point, depth)) = hit {
                out.push(Clash {
                    a: *ia,
                    b: *ib,
                    point,
                    depth,
                });
            }
        }
    }
    out.sort_by(|x, y| y.depth.total_cmp(&x.depth));
    out
}
