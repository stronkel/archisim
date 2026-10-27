//! Sections of meshes by a plane, as a section box shows what it cuts of
//! an object: a kitchen unit cut at 60 cm reads as one solid block, not as
//! the empty box its panels make; a sofa cut through its back and arms
//! keeps the seat seen between them.

use std::collections::HashMap;

use archisim_core::Point2;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::{ensure_ccw, signed_area};

/// A plane: a point of it and two directions along it at right angles
/// (its normal is `u × v`).
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub origin: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}

/// One region of a section: its outline in the plane's `u`, `v`
/// (counter-clockwise) and the tag of the faces most of it was cut from.
#[derive(Debug, Clone, PartialEq)]
pub struct Region<T> {
    pub outline: Vec<Point2>,
    pub tag: T,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Where the triangles cross the plane, filled: every closed loop the cut
/// makes, joined with the others, and the room they enclose filled too
/// (the inside of a cupboard, of a car's body). Each region takes the tag
/// of the largest loop inside it. Loops that do not close (open meshes)
/// are left out.
pub fn filled_section<T: Copy + PartialEq>(
    triangles: &[([[f64; 3]; 3], T)],
    plane: Plane,
) -> Vec<Region<T>> {
    let n = cross(plane.u, plane.v);
    let to_2d = |p: [f64; 3]| {
        let d = sub(p, plane.origin);
        [dot(d, plane.u), dot(d, plane.v)]
    };
    // The segments where each triangle crosses the plane.
    let mut segments: Vec<([[f64; 2]; 2], T)> = Vec::new();
    for (t, tag) in triangles {
        let d = t.map(|p| dot(sub(p, plane.origin), n));
        // A corner on the plane counts as above it: a segment through it
        // is found once, from the triangles on either side.
        let above = d.map(|x| x >= 0.0);
        if above.iter().all(|a| *a) || above.iter().all(|a| !*a) {
            continue;
        }
        let mut ends = Vec::with_capacity(2);
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            if above[i] != above[j] {
                let s = d[i] / (d[i] - d[j]);
                let p = [
                    t[i][0] + (t[j][0] - t[i][0]) * s,
                    t[i][1] + (t[j][1] - t[i][1]) * s,
                    t[i][2] + (t[j][2] - t[i][2]) * s,
                ];
                ends.push(to_2d(p));
            }
        }
        if ends.len() == 2 && (ends[0][0] - ends[1][0]).hypot(ends[0][1] - ends[1][1]) > 1e-9 {
            segments.push(([ends[0], ends[1]], *tag));
        }
    }
    // Chained into loops by their ends (to a hundredth of a millimetre:
    // the triangles' corners come from single precision).
    let key = |p: [f64; 2]| ((p[0] * 1e5).round() as i64, (p[1] * 1e5).round() as i64);
    let mut at: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, (s, _)) in segments.iter().enumerate() {
        for p in s {
            at.entry(key(*p)).or_default().push(i);
        }
    }
    let mut used = vec![false; segments.len()];
    let mut loops: Vec<(Vec<[f64; 2]>, T)> = Vec::new();
    for first in 0..segments.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let start = key(segments[first].0[0]);
        let mut pts = vec![segments[first].0[0]];
        let mut cur = segments[first].0[1];
        let mut tags = vec![(segments[first].1, 1usize)];
        let mut closed = false;
        for _ in 0..segments.len() {
            if key(cur) == start {
                closed = true;
                break;
            }
            pts.push(cur);
            let next = at
                .get(&key(cur))
                .and_then(|list| list.iter().copied().find(|j| !used[*j]));
            let Some(j) = next else { break };
            used[j] = true;
            let (s, tag) = segments[j];
            cur = if key(s[0]) == key(cur) { s[1] } else { s[0] };
            match tags.iter_mut().find(|(t, _)| *t == tag) {
                Some(c) => c.1 += 1,
                None => tags.push((tag, 1)),
            }
        }
        if closed && pts.len() >= 3 {
            let tag = tags.iter().max_by_key(|(_, c)| *c).map(|(t, _)| *t);
            if let Some(tag) = tag {
                loops.push((pts, tag));
            }
        }
    }
    if loops.is_empty() {
        return Vec::new();
    }
    // Every loop filled (counter-clockwise, so that "non-zero" fills each
    // one whatever way round its triangles went), joined together.
    let mut sized: Vec<(f64, Vec<Point2>, T)> = loops
        .into_iter()
        .map(|(pts, tag)| {
            let poly = ensure_ccw(pts.iter().map(|p| Point2::new(p[0], p[1])).collect());
            (signed_area(&poly).abs(), poly, tag)
        })
        .filter(|(a, _, _)| *a > 1e-10)
        .collect();
    sized.sort_by(|a, b| b.0.total_cmp(&a.0));
    let contours: Vec<Vec<[f64; 2]>> = sized
        .iter()
        .map(|(_, p, _)| p.iter().map(|q| [q.x, q.y]).collect())
        .collect();
    let joined = contours.overlay(
        &Vec::<Vec<[f64; 2]>>::new(),
        OverlayRule::Union,
        FillRule::NonZero,
    );
    joined
        .into_iter()
        .filter_map(|shape| {
            // Only the outline: what it encloses is filled.
            let outer = shape.into_iter().next()?;
            let outline = ensure_ccw(outer.iter().map(|p| Point2::new(p[0], p[1])).collect());
            if outline.len() < 3 || signed_area(&outline).abs() < 1e-10 {
                return None;
            }
            // The largest loop inside it gives its tag.
            let tag = sized
                .iter()
                .find(|(_, p, _)| point_in(&outline, centre_of(p)))
                .or_else(|| sized.first())
                .map(|(_, _, t)| *t)?;
            Some(Region { outline, tag })
        })
        .collect()
}

/// A point inside a loop, to tell which region it went into: its centre,
/// or (for a U, whose centre is outside it) a point just in from a corner.
fn centre_of(poly: &[Point2]) -> Point2 {
    let n = poly.len() as f64;
    let c = poly.iter().fold(Point2::new(0.0, 0.0), |a, p| {
        Point2::new(a.x + p.x / n, a.y + p.y / n)
    });
    if point_in(poly, c) {
        return c;
    }
    // A concave loop (a U): nudge from its first corner towards the
    // centre, which then lies inside it.
    let p = poly[0];
    Point2::new(p.x + (c.x - p.x) * 0.01, p.y + (c.y - p.y) * 0.01)
}

fn point_in(poly: &[Point2], p: Point2) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The twelve triangles of a box.
    fn cuboid(lo: [f64; 3], hi: [f64; 3], tag: u8) -> Vec<([[f64; 3]; 3], u8)> {
        let c = |i: usize| {
            [
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]
        };
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        quads
            .iter()
            .flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]])
            .map(|t| (t, tag))
            .collect()
    }

    const FLOOR_AT: fn(f64) -> Plane = |z| Plane {
        origin: [0.0, 0.0, z],
        u: [1.0, 0.0, 0.0],
        v: [0.0, 1.0, 0.0],
    };

    fn area<T>(r: &[Region<T>]) -> f64 {
        r.iter().map(|r| signed_area(&r.outline)).sum()
    }

    /// A cupboard of four panels cut through them is filled whole; a U of
    /// panels (a sofa's back and arms) keeps the room between its arms.
    #[test]
    fn cut_cupboard_is_solid_and_a_u_is_not() {
        let mut ring = Vec::new();
        ring.extend(cuboid([0.0, 0.0, 0.0], [1.0, 0.02, 0.9], 1));
        ring.extend(cuboid([0.0, 0.58, 0.0], [1.0, 0.6, 0.9], 1));
        ring.extend(cuboid([0.0, 0.0, 0.0], [0.02, 0.6, 0.9], 1));
        ring.extend(cuboid([0.98, 0.0, 0.0], [1.0, 0.6, 0.9], 2));
        let r = filled_section(&ring, FLOOR_AT(0.6));
        assert_eq!(r.len(), 1, "{r:?}");
        assert!((area(&r) - 0.6).abs() < 1e-6, "{}", area(&r));
        assert_eq!(r[0].tag, 1);
        let mut u = Vec::new();
        u.extend(cuboid([0.0, 0.0, 0.0], [2.0, 0.2, 0.9], 1));
        u.extend(cuboid([0.0, 0.0, 0.0], [0.2, 0.9, 0.7], 1));
        u.extend(cuboid([1.8, 0.0, 0.0], [2.0, 0.9, 0.7], 1));
        u.extend(cuboid([0.2, 0.2, 0.0], [1.8, 0.9, 0.45], 3));
        let r = filled_section(&u, FLOOR_AT(0.6));
        let want = 2.0 * 0.2 + 2.0 * 0.2 * 0.7;
        assert!((area(&r) - want).abs() < 1e-6, "{}", area(&r));
        // Nothing above or below.
        assert!(filled_section(&u, FLOOR_AT(1.0)).is_empty());
    }

    /// A side plane gives the outline in its own directions.
    #[test]
    fn vertical_planes_too() {
        let b = cuboid([0.0, 0.0, 0.0], [1.0, 2.0, 3.0], 7);
        let plane = Plane {
            origin: [0.5, 0.0, 0.0],
            u: [0.0, 1.0, 0.0],
            v: [0.0, 0.0, 1.0],
        };
        let r = filled_section(&b, plane);
        assert_eq!(r.len(), 1);
        assert!((area(&r) - 6.0).abs() < 1e-6);
    }
}
