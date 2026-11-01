//! Looking for what a model has in the air: the pieces it is made of (each
//! a set of triangles that share their corners), which touch which, and
//! the pieces that no chain of touching pieces joins to what holds the
//! model up (the floor; the wall behind it or the ceiling above for what
//! hangs). A leg that stops short of the seat, a scroll that does not
//! reach its bar, a foot under nothing: each is a loose piece.
//!
//! Used by the workshop's review (`archisim_library::workshop`), not by
//! the models themselves.

use crate::build::P3;
use crate::PropMesh;

/// What holds a model up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchors {
    /// The floor: the lowest height of the model.
    pub floor: bool,
    /// The wall behind it: `y = 0` (or its lowest `y`, if further
    /// forward).
    pub back: bool,
    /// The ceiling: its highest height.
    pub ceiling: bool,
}

impl Default for Anchors {
    fn default() -> Self {
        Self {
            floor: true,
            back: false,
            ceiling: false,
        }
    }
}

/// What holds up a model, by what it makes: what hangs on the wall is
/// held by it, lamps that hang by the ceiling; the rest stand.
pub fn anchors_for(kind: &str) -> Anchors {
    let back = matches!(
        kind,
        "wall_cabinet"
            | "mirror"
            | "towel_rail"
            | "wall_decor"
            | "wall_light"
            | "hood"
            | "fine_back_bar"
            | "wc"
            | "bidet"
            | "washbasin"
            | "shower"
            | "bath"
            | "fine_pool_shower"
            | "dressing_table"
            | "curtain"
            | "radiator"
            | "ac_unit"
            | "built_in_wardrobe"
            | "wall_climber"
    );
    // (a shower's rain head may hang from the ceiling)
    let ceiling = matches!(kind, "pendant" | "ceiling_light" | "shower");
    Anchors {
        floor: true,
        back,
        ceiling,
    }
}

/// A piece of a model.
#[derive(Debug, Clone)]
pub struct Piece {
    pub role: &'static str,
    pub triangles: Vec<[P3; 3]>,
    pub lo: P3,
    pub hi: P3,
}

/// A loose piece: its role, its box and how far it is from the nearest
/// piece that holds (metres, measured between their boxes: a lower bound).
#[derive(Debug, Clone)]
pub struct Loose {
    pub role: &'static str,
    pub lo: P3,
    pub hi: P3,
    pub triangles: usize,
    pub gap: f64,
    /// The role and box of the held piece it is nearest to.
    pub near: (&'static str, P3, P3),
    /// Loose pieces that touch each other share their group: a chair's
    /// whole back in the air is one group.
    pub group: usize,
}

fn box_gap(alo: P3, ahi: P3, blo: P3, bhi: P3) -> f64 {
    let mut d2 = 0.0;
    for k in 0..3 {
        let d = (blo[k] - ahi[k]).max(alo[k] - bhi[k]).max(0.0);
        d2 += d * d;
    }
    d2.sqrt()
}

/// The pieces of a model: triangles joined by shared corners.
pub fn pieces(m: &PropMesh) -> Vec<Piece> {
    let n = m.positions.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(p: &mut [u32], mut i: u32) -> u32 {
        while p[i as usize] != i {
            p[i as usize] = p[p[i as usize] as usize];
            i = p[i as usize];
        }
        i
    }
    // Corners at the same place are one (a box's faces have their own).
    let mut at: std::collections::HashMap<[i64; 3], u32> = Default::default();
    for (i, p) in m.positions.iter().enumerate() {
        let k = p.map(|c| (c as f64 * 1e6).round() as i64);
        let first = *at.entry(k).or_insert(i as u32);
        if first != i as u32 {
            let a = find(&mut parent, first);
            let b = find(&mut parent, i as u32);
            if a != b {
                parent[b as usize] = a;
            }
        }
    }
    for (_, tris) in &m.parts {
        for t in tris {
            let a = find(&mut parent, t[0]);
            for &v in &t[1..] {
                let b = find(&mut parent, v);
                if a != b {
                    parent[b as usize] = a;
                }
            }
        }
    }
    let mut by_root: std::collections::BTreeMap<(u32, &'static str), Piece> = Default::default();
    for (role, tris) in &m.parts {
        for t in tris {
            let root = find(&mut parent, t[0]);
            let tri = t.map(|i| {
                let p = m.positions[i as usize];
                [p[0] as f64, p[1] as f64, p[2] as f64]
            });
            let piece = by_root.entry((root, role)).or_insert_with(|| Piece {
                role,
                triangles: Vec::new(),
                lo: [f64::MAX; 3],
                hi: [f64::MIN; 3],
            });
            let (lo, hi) = tri_box(&tri);
            for k in 0..3 {
                piece.lo[k] = piece.lo[k].min(lo[k]);
                piece.hi[k] = piece.hi[k].max(hi[k]);
            }
            piece.triangles.push(tri);
        }
    }
    by_root.into_values().collect()
}

fn boxes_meet(alo: P3, ahi: P3, blo: P3, bhi: P3, tol: f64) -> bool {
    (0..3).all(|k| alo[k] <= bhi[k] + tol && blo[k] <= ahi[k] + tol)
}

fn tri_box(t: &[P3; 3]) -> (P3, P3) {
    let mut lo = t[0];
    let mut hi = t[0];
    for p in &t[1..] {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn add_s(a: P3, b: P3, s: f64) -> P3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// The nearest point of triangle `t` to `p` (Ericson, Real-Time Collision
/// Detection, 5.1.5).
fn closest_on_tri(p: P3, t: &[P3; 3]) -> P3 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return add_s(a, ab, v);
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return add_s(a, ac, w);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add_s(b, sub(c, b), w);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    add_s(add_s(a, ab, v), ac, w)
}

fn dist2(a: P3, b: P3) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

/// The squared distance between segments `p1 q1` and `p2 q2` (Ericson
/// 5.1.9).
fn seg_seg2(p1: P3, q1: P3, p2: P3, q2: P3) -> f64 {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let eps = 1e-14;
    let (s, t);
    if a <= eps && e <= eps {
        return dot(r, r);
    }
    if a <= eps {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = dot(d1, r);
        if e <= eps {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut ss = if denom > eps {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut tt = (b * ss + f) / e;
            if tt < 0.0 {
                tt = 0.0;
                ss = (-c / a).clamp(0.0, 1.0);
            } else if tt > 1.0 {
                tt = 1.0;
                ss = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = ss;
            t = tt;
        }
    }
    dist2(add_s(p1, d1, s), add_s(p2, d2, t))
}

/// Whether segment `p q` crosses triangle `t`.
fn seg_hits_tri(p: P3, q: P3, t: &[P3; 3]) -> bool {
    let d = sub(q, p);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let h = cross(d, e2);
    let a = dot(e1, h);
    if a.abs() < 1e-14 {
        return false;
    }
    let f = 1.0 / a;
    let s = sub(p, t[0]);
    let u = f * dot(s, h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let qv = cross(s, e1);
    let v = f * dot(d, qv);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let k = f * dot(e2, qv);
    (0.0..=1.0).contains(&k)
}

/// Whether two triangles come within `tol` of each other.
fn tris_meet(a: &[P3; 3], b: &[P3; 3], tol: f64) -> bool {
    let t2 = tol * tol;
    for p in a {
        if dist2(*p, closest_on_tri(*p, b)) <= t2 {
            return true;
        }
    }
    for p in b {
        if dist2(*p, closest_on_tri(*p, a)) <= t2 {
            return true;
        }
    }
    for i in 0..3 {
        let (p, q) = (a[i], a[(i + 1) % 3]);
        if seg_hits_tri(p, q, b) {
            return true;
        }
        for j in 0..3 {
            if seg_seg2(p, q, b[j], b[(j + 1) % 3]) <= t2 {
                return true;
            }
        }
    }
    (0..3).any(|j| seg_hits_tri(b[j], b[(j + 1) % 3], a))
}

/// Whether point `p` is inside the closed piece (a ray along a slightly
/// skewed direction crosses it an odd number of times).
fn inside(p: P3, piece: &Piece) -> bool {
    if (0..3).any(|k| p[k] < piece.lo[k] || p[k] > piece.hi[k]) {
        return false;
    }
    let far = add_s(p, [0.577, 0.5771, 0.5779], 1e3);
    piece
        .triangles
        .iter()
        .filter(|t| seg_hits_tri(p, far, t))
        .count()
        % 2
        == 1
}

/// Whether two pieces touch (within `tol`), cross or one holds the other.
pub fn touch(a: &Piece, b: &Piece, tol: f64) -> bool {
    if !boxes_meet(a.lo, a.hi, b.lo, b.hi, tol) {
        return false;
    }
    // Only the triangles in the other's box can meet.
    let near = |p: &Piece, o: &Piece| -> Vec<([P3; 3], P3, P3)> {
        p.triangles
            .iter()
            .filter_map(|t| {
                let (lo, hi) = tri_box(t);
                boxes_meet(lo, hi, o.lo, o.hi, tol).then_some((*t, lo, hi))
            })
            .collect()
    };
    let na = near(a, b);
    let nb = near(b, a);
    for (ta, alo, ahi) in &na {
        for (tb, blo, bhi) in &nb {
            if boxes_meet(*alo, *ahi, *blo, *bhi, tol) && tris_meet(ta, tb, tol) {
                return true;
            }
        }
    }
    inside(a.triangles[0][0], b) || inside(b.triangles[0][0], a)
}

/// The pieces of `m` that nothing joins to what holds it up.
pub fn loose_pieces(m: &PropMesh, anchors: Anchors, tol: f64) -> Vec<Loose> {
    let ps = pieces(m);
    if ps.is_empty() {
        return Vec::new();
    }
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in &ps {
        for k in 0..3 {
            lo[k] = lo[k].min(p.lo[k]);
            hi[k] = hi[k].max(p.hi[k]);
        }
    }
    // Leaves, stalks and flowers (`fine`, and the roles named so) are not
    // looked at: a leaf a little off its stalk does not show.
    let held = |p: &Piece| {
        m.fine.contains(&p.role)
            || p.role.starts_with("leaves")
            || p.role.starts_with("flowers")
            || (anchors.floor && p.lo[2] <= lo[2] + tol)
            || (anchors.back && p.lo[1] <= lo[1].max(0.0) + tol)
            || (anchors.ceiling && p.hi[2] >= hi[2] - tol)
    };
    let n = ps.len();
    let mut reached = vec![false; n];
    let mut stack: Vec<usize> = (0..n).filter(|&i| held(&ps[i])).collect();
    for &i in &stack {
        reached[i] = true;
    }
    while let Some(i) = stack.pop() {
        for j in 0..n {
            if !reached[j] && touch(&ps[i], &ps[j], tol) {
                reached[j] = true;
                stack.push(j);
            }
        }
    }
    // How far: from the corners of one to the faces of the other, over
    // the held pieces near enough to matter.
    let gap = |p: &Piece| {
        let mut best = (f64::MAX, 0usize);
        for (qi, q) in ps.iter().enumerate().filter(|(i, _)| reached[*i]) {
            let g = box_gap(p.lo, p.hi, q.lo, q.hi);
            if g >= best.0 {
                continue;
            }
            if g > 0.3 {
                best = (g, qi);
                continue;
            }
            for (a, b) in [(p, q), (q, p)] {
                for t in &b.triangles {
                    let (lo, hi) = tri_box(t);
                    if box_gap(lo, hi, a.lo, a.hi) >= best.0 {
                        continue;
                    }
                    for tri in &a.triangles {
                        for v in tri {
                            let d = dist2(*v, closest_on_tri(*v, t)).sqrt();
                            if d < best.0 {
                                best = (d, qi);
                            }
                        }
                    }
                }
            }
        }
        best
    };
    let loose: Vec<usize> = (0..n).filter(|&i| !reached[i]).collect();
    let mut group = vec![usize::MAX; n];
    let mut groups = 0;
    for &i in &loose {
        if group[i] != usize::MAX {
            continue;
        }
        group[i] = groups;
        let mut stack = vec![i];
        while let Some(a) = stack.pop() {
            for &b in &loose {
                if group[b] == usize::MAX && touch(&ps[a], &ps[b], tol) {
                    group[b] = groups;
                    stack.push(b);
                }
            }
        }
        groups += 1;
    }
    loose
        .into_iter()
        .map(|i| {
            let p = &ps[i];
            let (g, near) = gap(p);
            let q = &ps[near.min(n - 1)];
            Loose {
                role: p.role,
                lo: p.lo,
                hi: p.hi,
                triangles: p.triangles.len(),
                gap: g,
                near: (q.role, q.lo, q.hi),
                group: group[i],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::Builder;

    fn mesh(b: Builder) -> PropMesh {
        PropMesh {
            positions: b.positions,
            parts: b.parts.into_iter().collect(),
            soft: Vec::new(),
            fine: Vec::new(),
        }
    }

    #[test]
    fn a_table_stands_and_a_floating_box_does_not() {
        let mut b = Builder::default();
        // A top on four legs, a rod through a leg, a box in the air.
        b.rbox("top", [0.0, 0.0, 0.7], [1.0, 1.0, 0.74], 0.0);
        for (x, y) in [(0.0, 0.0), (0.95, 0.0), (0.0, 0.95), (0.95, 0.95)] {
            b.rbox("leg", [x, y, 0.0], [x + 0.05, y + 0.05, 0.7], 0.0);
        }
        b.cyl("rod", [-0.1, 0.025, 0.3], [0.2, 0.025, 0.3], 0.01);
        b.rbox("ghost", [0.4, 0.4, 1.0], [0.5, 0.5, 1.1], 0.0);
        // A small block inside the top, touching nothing but held by it.
        b.rbox("inlay", [0.4, 0.4, 0.71], [0.5, 0.5, 0.72], 0.0);
        let loose = loose_pieces(&mesh(b), Anchors::default(), 0.002);
        let roles: Vec<_> = loose.iter().map(|l| l.role).collect();
        assert_eq!(roles, ["ghost"]);
    }

    #[test]
    fn what_hangs_from_the_wall_is_held_by_it() {
        let mut b = Builder::default();
        b.rbox("mirror", [0.0, 0.0, 1.0], [0.6, 0.02, 1.8], 0.0);
        b.rbox("shelf", [0.0, 0.0, 0.0], [0.6, 0.3, 0.02], 0.0);
        let m = mesh(b);
        assert_eq!(loose_pieces(&m, Anchors::default(), 0.002).len(), 1);
        let wall = Anchors {
            back: true,
            ..Anchors::default()
        };
        assert!(loose_pieces(&m, wall, 0.002).is_empty());
    }
}
