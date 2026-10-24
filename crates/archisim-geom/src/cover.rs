//! What a group of triangles covers once flattened onto a drawing: the
//! silhouette of a body seen from one side (a person, a car in an
//! elevation), as outlines.
//!
//! The triangles are painted on a fine grid and the edge of what they
//! cover is traced (marching squares): thousands of small, overlapping
//! triangles, as a rounded body has, would make a polygon union slow.

use crate::junctions::Shape;
use archisim_core::Point2;
use std::collections::{HashMap, HashSet};

/// The region the triangles cover, traced on a grid of `cell` metres:
/// closed outlines, each with no holes of its own (a hole is another
/// outline inside it, turning the other way: fill them even-odd).
pub fn cover(triangles: &[[Point2; 3]], cell: f64) -> Vec<Shape> {
    let cell = cell.max(1e-4);
    if triangles.is_empty() {
        return Vec::new();
    }
    let mut lo = Point2::new(f64::MAX, f64::MAX);
    let mut hi = Point2::new(f64::MIN, f64::MIN);
    for t in triangles {
        for p in t {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    // A cell of margin all round, so every outline closes.
    let origin = Point2::new(lo.x - cell, lo.y - cell);
    let nx = ((hi.x - lo.x) / cell).ceil() as usize + 3;
    let ny = ((hi.y - lo.y) / cell).ceil() as usize + 3;
    if nx * ny > 40_000_000 {
        return Vec::new();
    }
    let mut on = vec![false; nx * ny];
    // A grid point is covered if it is inside a triangle.
    for t in triangles {
        let (a, b, c) = (t[0], t[1], t[2]);
        let area = (b - a).cross(c - a);
        if area.abs() < 1e-14 {
            continue;
        }
        let i0 = ((a.x.min(b.x).min(c.x) - origin.x) / cell).floor().max(0.0) as usize;
        let i1 = (((a.x.max(b.x).max(c.x) - origin.x) / cell).ceil() as usize).min(nx - 1);
        let j0 = ((a.y.min(b.y).min(c.y) - origin.y) / cell).floor().max(0.0) as usize;
        let j1 = (((a.y.max(b.y).max(c.y) - origin.y) / cell).ceil() as usize).min(ny - 1);
        let s = area.signum();
        // A hair of slack, so triangles meeting along an edge leave no
        // crack between them.
        let slack = -1e-9 * area.abs();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let p = Point2::new(origin.x + i as f64 * cell, origin.y + j as f64 * cell);
                let w0 = (b - a).cross(p - a) * s;
                let w1 = (c - b).cross(p - b) * s;
                let w2 = (a - c).cross(p - c) * s;
                if w0 >= slack && w1 >= slack && w2 >= slack {
                    on[j * nx + i] = true;
                }
            }
        }
    }
    trace(&on, nx, ny)
        .into_iter()
        .map(|l| {
            let pts: Vec<Point2> = l
                .into_iter()
                .map(|[x, y]| Point2::new(origin.x + x * cell, origin.y + y * cell))
                .collect();
            (straighten(&pts, cell * 0.35), Vec::new())
        })
        .filter(|(o, _): &Shape| o.len() >= 3)
        .collect()
}

/// The edges between covered and uncovered grid points, through the
/// middle of each grid step, joined into closed loops (grid units).
fn trace(on: &[bool], nx: usize, ny: usize) -> Vec<Vec<[f64; 2]>> {
    // Edges of the grid's squares: (i, j, 0) from (i, j) to (i + 1, j);
    // (i, j, 1) from (i, j) to (i, j + 1).
    type E = (usize, usize, u8);
    let v = |i: usize, j: usize| on[j * nx + i];
    let point = |e: E| -> [f64; 2] {
        let (i, j, k) = e;
        if k == 0 {
            [i as f64 + 0.5, j as f64]
        } else {
            [i as f64, j as f64 + 0.5]
        }
    };
    let mut next: HashMap<E, E> = HashMap::new();
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let c = [v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)];
            let mut case = 0;
            for (k, x) in c.iter().enumerate() {
                if *x {
                    case |= 1 << k;
                }
            }
            let bottom = (i, j, 0);
            let right = (i + 1, j, 1);
            let top = (i, j + 1, 0);
            let left = (i, j, 1);
            // Inside on the left of each segment.
            let segs: &[(E, E)] = match case {
                1 => &[(left, bottom)],
                2 => &[(bottom, right)],
                3 => &[(left, right)],
                4 => &[(right, top)],
                5 => &[(left, top), (right, bottom)],
                6 => &[(bottom, top)],
                7 => &[(left, top)],
                8 => &[(top, left)],
                9 => &[(top, bottom)],
                10 => &[(bottom, left), (top, right)],
                11 => &[(top, right)],
                12 => &[(right, left)],
                13 => &[(right, bottom)],
                14 => &[(bottom, left)],
                _ => &[],
            };
            for &(a, b) in segs {
                next.insert(b, a);
            }
        }
    }
    let mut loops = Vec::new();
    let mut used: HashSet<E> = HashSet::new();
    let mut keys: Vec<E> = next.keys().copied().collect();
    keys.sort();
    for start in keys {
        if used.contains(&start) {
            continue;
        }
        let mut l = Vec::new();
        let mut e = start;
        loop {
            if !used.insert(e) {
                break;
            }
            l.push(point(e));
            match next.get(&e) {
                Some(n) => e = *n,
                None => break,
            }
        }
        if l.len() >= 3 {
            loops.push(l);
        }
    }
    loops
}

/// A traced loop with the steps of the grid taken out: points closer
/// than `tolerance` to the line through their neighbours go.
fn straighten(points: &[Point2], tolerance: f64) -> Vec<Point2> {
    let n = points.len();
    if n < 4 {
        return points.to_vec();
    }
    // Douglas–Peucker on the loop, cut at its first point and the point
    // farthest from it.
    let far = (1..n)
        .max_by(|&a, &b| {
            points[a]
                .distance(points[0])
                .total_cmp(&points[b].distance(points[0]))
        })
        .unwrap_or(n / 2);
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    fn dp(p: &[Point2], keep: &mut [bool], a: usize, b: usize, tol: f64) {
        if b <= a + 1 {
            return;
        }
        let (pa, pb) = (p[a], p[b % p.len()]);
        let d = pb - pa;
        let len = d.length().max(1e-12);
        let (mut worst, mut at) = (0.0, a);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let dist = (d.cross(*q - pa) / len).abs();
            if dist > worst {
                worst = dist;
                at = i;
            }
        }
        if worst > tol {
            keep[at] = true;
            dp(p, keep, a, at, tol);
            dp(p, keep, at, b, tol);
        }
    }
    let mut ring = points.to_vec();
    ring.push(points[0]);
    let mut k2 = keep.clone();
    k2.push(true);
    dp(&ring, &mut k2, 0, far, tolerance);
    dp(&ring, &mut k2, far, n, tolerance);
    (0..n).filter(|&i| k2[i]).map(|i| points[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles making a square cover the square, one outline with
    /// its four corners.
    #[test]
    fn two_halves_make_one_square() {
        let p = |x: f64, y: f64| Point2::new(x, y);
        let s = cover(
            &[
                [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
                [p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
            ],
            0.01,
        );
        assert_eq!(s.len(), 1);
        let a = crate::junctions::area(&s);
        assert!((a - 1.0).abs() < 0.03, "{a}");
        assert!(s[0].0.len() <= 8, "{}", s[0].0.len());
    }

    /// A ring of triangles leaves its hole as a second outline.
    #[test]
    fn a_frame_has_a_hole() {
        let p = |x: f64, y: f64| Point2::new(x, y);
        let q = |a: Point2, b: Point2, c: Point2, d: Point2| [[a, b, c], [a, c, d]];
        let mut t = Vec::new();
        t.extend(q(p(0.0, 0.0), p(3.0, 0.0), p(3.0, 1.0), p(0.0, 1.0)));
        t.extend(q(p(0.0, 2.0), p(3.0, 2.0), p(3.0, 3.0), p(0.0, 3.0)));
        t.extend(q(p(0.0, 1.0), p(1.0, 1.0), p(1.0, 2.0), p(0.0, 2.0)));
        t.extend(q(p(2.0, 1.0), p(3.0, 1.0), p(3.0, 2.0), p(2.0, 2.0)));
        let s = cover(&t, 0.02);
        assert_eq!(s.len(), 2);
    }
}
