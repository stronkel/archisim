//! The weighted straight skeleton of a polygon: how a roof of planes
//! covers any footprint (L, T, U shapes…), with hips over convex corners
//! and valleys from re-entrant ones.
//!
//! Each side of the footprint moves inwards, parallel to itself, at its
//! own speed (the run per unit of rise: `1 / slope`; a gable does not
//! move); the corners slide where the sides meet. Each side sweeps the part
//! of the roof its plane covers (its "face"), and the lines its corners
//! trace are the hips, valleys and ridges. As the sides move, a side can
//! shrink to nothing (its neighbours then meet: an "edge event"), a
//! re-entrant corner can reach another side (the shape splits in two: a
//! "split event"), and a piece can close up into a line (a ridge). The
//! simulation jumps from event to event.

use crate::math::{signed_area, Point2, Vec2};

/// A corner of the moving outline.
#[derive(Debug, Clone, Copy)]
struct Corner {
    /// Where it is now.
    p: Point2,
    /// How it moves per unit of time.
    v: Vec2,
    /// The sides before and after it (indices into the polygon's sides).
    side_in: usize,
    side_out: usize,
    /// The node its present stretch started from.
    from: usize,
}

/// What the skeleton found: the nodes (corners of the footprint first)
/// and, for each side, the lines that bound its face (pairs of nodes).
struct Run {
    nodes: Vec<Point2>,
    bounds: Vec<Vec<(usize, usize)>>,
}

/// The faces of the weighted straight skeleton of a counter-clockwise
/// polygon whose side `i` (from corner `i` to the next) moves inwards at
/// `speeds[i]` (0: does not move). For each side, its face: a
/// counter-clockwise polygon starting with the side's two corners; empty
/// for a side that does not move. `None` if the polygon cannot be resolved
/// (it crosses itself, all speeds are zero…).
pub fn faces(poly: &[Point2], speeds: &[f64]) -> Option<Vec<Vec<Point2>>> {
    let n = poly.len();
    if n < 3 || speeds.len() != n || speeds.iter().any(|s| !(s.is_finite() && *s >= 0.0)) {
        return None;
    }
    if speeds.iter().all(|s| *s <= 0.0) || signed_area(poly) <= 0.0 {
        return None;
    }
    let run = simulate(poly, speeds)?;
    let mut out = Vec::with_capacity(n);
    for (i, speed) in speeds.iter().enumerate() {
        if *speed <= 0.0 {
            out.push(Vec::new());
            continue;
        }
        out.push(chain(&run, i, n)?);
    }
    Some(out)
}

/// The inward normal and a point of side `i`.
fn side(poly: &[Point2], i: usize) -> (Point2, Vec2) {
    let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
    let d = b - a;
    let l = d.length().max(1e-300);
    (a, Point2::new(-d.y / l, d.x / l))
}

/// How a corner between two sides moves: the point on both moving lines.
fn velocity(poly: &[Point2], speeds: &[f64], side_in: usize, side_out: usize) -> Vec2 {
    let (_, n1) = side(poly, side_in);
    let (_, n2) = side(poly, side_out);
    let (w1, w2) = (speeds[side_in], speeds[side_out]);
    let det = n1.cross(n2);
    if det.abs() < 1e-9 {
        // In line: the corner moves square to them (the faster one).
        let w = w1.max(w2);
        return n1 * w;
    }
    // n1·v = w1, n2·v = w2.
    Point2::new((w1 * n2.y - w2 * n1.y) / det, (n1.x * w2 - n2.x * w1) / det)
}

fn simulate(poly: &[Point2], speeds: &[f64]) -> Option<Run> {
    let n = poly.len();
    let size = poly
        .iter()
        .flat_map(|p| [p.x.abs(), p.y.abs()])
        .fold(0.0_f64, f64::max)
        .max(
            poly.iter()
                .map(|p| p.distance(poly[0]))
                .fold(0.0_f64, f64::max),
        )
        .max(1e-9);
    let eps = size * 1e-9;
    let mut nodes: Vec<Point2> = poly.to_vec();
    let mut bounds: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    let mut loops: Vec<Vec<Corner>> = vec![(0..n)
        .map(|i| Corner {
            p: poly[i],
            v: velocity(poly, speeds, (i + n - 1) % n, i),
            side_in: (i + n - 1) % n,
            side_out: i,
            from: i,
        })
        .collect()];
    let node_at = |nodes: &mut Vec<Point2>, p: Point2| -> usize {
        if let Some(k) = nodes.iter().position(|q| q.distance(p) <= eps * 10.0) {
            return k;
        }
        nodes.push(p);
        nodes.len() - 1
    };
    // A corner's stretch ends at `p`: the line from where it started
    // bounds the faces of both its sides.
    let end =
        |c: &Corner, nodes: &mut Vec<Point2>, bounds: &mut Vec<Vec<(usize, usize)>>| -> usize {
            let k = node_at(nodes, c.p);
            if k != c.from {
                bounds[c.side_in].push((c.from, k));
                bounds[c.side_out].push((c.from, k));
            }
            k
        };
    let mut guard = 0;
    while !loops.is_empty() {
        guard += 1;
        if guard > 50 * n + 200 {
            return None;
        }
        // Tidy every loop at the present time: corners that met, corners
        // on another side, loops closed up.
        let mut changed = true;
        while changed {
            changed = false;
            let mut next_loops = Vec::new();
            for mut lp in std::mem::take(&mut loops) {
                // Corners that met (a side shrank to nothing).
                let mut i = 0;
                while lp.len() > 1 && i < lp.len() {
                    let j = (i + 1) % lp.len();
                    if lp[i].p.distance(lp[j].p) <= eps * 10.0 {
                        let (a, b) = (lp[i], lp[j]);
                        let k = end(&a, &mut nodes, &mut bounds);
                        end(&b, &mut nodes, &mut bounds);
                        // The side between them is gone.
                        let merged = Corner {
                            p: nodes[k],
                            v: velocity(poly, speeds, a.side_in, b.side_out),
                            side_in: a.side_in,
                            side_out: b.side_out,
                            from: k,
                        };
                        if j == 0 {
                            lp.remove(i);
                            lp[0] = merged;
                        } else {
                            lp[i] = merged;
                            lp.remove(j);
                        }
                        changed = true;
                    } else {
                        i += 1;
                    }
                }
                // Closed up: a ridge (or a point).
                let pts: Vec<Point2> = lp.iter().map(|c| c.p).collect();
                if lp.len() <= 2 || signed_area(&pts).abs() <= eps * size {
                    let ks: Vec<usize> =
                        lp.iter().map(|c| end(c, &mut nodes, &mut bounds)).collect();
                    for (idx, c) in lp.iter().enumerate() {
                        let k2 = ks[(idx + 1) % ks.len()];
                        if ks[idx] != k2 {
                            bounds[c.side_out].push((ks[idx], k2));
                        }
                    }
                    changed = true;
                    continue;
                }
                // A re-entrant corner on another side: the loop splits.
                if let Some((vi, ej)) = split_at(&lp, eps * 10.0) {
                    let c = lp[vi];
                    let k = end(&c, &mut nodes, &mut bounds);
                    let side = lp[ej].side_out;
                    let m = lp.len();
                    // First: from the corner along its outgoing side round
                    // to the start of the side it hit, then back along it.
                    let mut a = vec![Corner {
                        p: nodes[k],
                        v: velocity(poly, speeds, side, c.side_out),
                        side_in: side,
                        side_out: c.side_out,
                        from: k,
                    }];
                    let mut t = (vi + 1) % m;
                    loop {
                        a.push(lp[t]);
                        if t == ej {
                            break;
                        }
                        t = (t + 1) % m;
                    }
                    // Second: from the end of that side round to the
                    // corner, in along its incoming side.
                    let mut b = vec![Corner {
                        p: nodes[k],
                        v: velocity(poly, speeds, c.side_in, side),
                        side_in: c.side_in,
                        side_out: side,
                        from: k,
                    }];
                    let mut t = (ej + 1) % m;
                    while t != vi {
                        b.push(lp[t]);
                        t = (t + 1) % m;
                    }
                    next_loops.push(a);
                    next_loops.push(b);
                    changed = true;
                    continue;
                }
                next_loops.push(lp);
            }
            loops = next_loops;
        }
        if loops.is_empty() {
            break;
        }
        // The next event.
        let mut dt = f64::INFINITY;
        for lp in &loops {
            let m = lp.len();
            for i in 0..m {
                let (a, b) = (lp[i], lp[(i + 1) % m]);
                // A side shrinking to nothing.
                let ab = b.p - a.p;
                let len = ab.length();
                if len > 1e-300 {
                    let d = ab * (1.0 / len);
                    let rate = (b.v - a.v).dot(d);
                    if rate < -1e-12 {
                        dt = dt.min(len / -rate);
                    }
                }
            }
            for (i, c) in lp.iter().enumerate() {
                if !reflex(lp, i) {
                    continue;
                }
                for j in 0..m {
                    if j == i || (j + 1) % m == i {
                        continue;
                    }
                    let (x, y) = (lp[j], lp[(j + 1) % m]);
                    let (_, nrm) = side(poly, x.side_out);
                    let w = speeds[x.side_out];
                    let gap = (c.p - x.p).dot(nrm);
                    let close = w - c.v.dot(nrm);
                    if gap < -eps || close <= 1e-12 {
                        continue;
                    }
                    let t = gap / close;
                    // Within the side then?
                    let at = c.p + c.v * t;
                    let (xa, ya) = (x.p + x.v * t, y.p + y.v * t);
                    let d = ya - xa;
                    let len2 = d.dot(d);
                    if len2 <= 0.0 {
                        continue;
                    }
                    let u = (at - xa).dot(d) / len2;
                    if (-1e-9..=1.0 + 1e-9).contains(&u) {
                        dt = dt.min(t);
                    }
                }
            }
        }
        if !dt.is_finite() {
            return None;
        }
        let dt = dt.max(0.0);
        for lp in loops.iter_mut() {
            for c in lp.iter_mut() {
                c.p = c.p + c.v * dt;
            }
        }
    }
    Some(Run { nodes, bounds })
}

/// Whether corner `i` of a loop turns right (re-entrant).
fn reflex(lp: &[Corner], i: usize) -> bool {
    let m = lp.len();
    let (a, b, c) = (lp[(i + m - 1) % m].p, lp[i].p, lp[(i + 1) % m].p);
    (b - a).cross(c - b) < 0.0
}

/// A re-entrant corner lying on a side of its loop that is not its own:
/// (corner, first corner of that side).
fn split_at(lp: &[Corner], tol: f64) -> Option<(usize, usize)> {
    let m = lp.len();
    for i in 0..m {
        if !reflex(lp, i) {
            continue;
        }
        let p = lp[i].p;
        for j in 0..m {
            if j == i || (j + 1) % m == i {
                continue;
            }
            let (x, y) = (lp[j].p, lp[(j + 1) % m].p);
            let d = y - x;
            let len2 = d.dot(d);
            if len2 <= tol * tol {
                continue;
            }
            let u = (p - x).dot(d) / len2;
            if !(-1e-9..=1.0 + 1e-9).contains(&u) {
                continue;
            }
            let off = (p - x).cross(d).abs() / len2.sqrt();
            if off <= tol {
                return Some((i, j));
            }
        }
    }
    None
}

/// The face of side `i`: its corners, then round its bounds back to the
/// start, counter-clockwise.
fn chain(run: &Run, i: usize, n: usize) -> Option<Vec<Point2>> {
    let (a, b) = (i, (i + 1) % n);
    let segs: Vec<(usize, usize)> = run.bounds[i].clone();
    let mut out = vec![a, b];
    let mut used = vec![false; segs.len()];
    let mut at = b;
    let mut came_from = a;
    for _ in 0..=segs.len() {
        if at == a {
            break;
        }
        // The bound from here not yet walked (the one turning most to the
        // left if several meet, keeping the face simple).
        let options: Vec<(usize, usize)> = segs
            .iter()
            .enumerate()
            .filter(|(k, _)| !used[*k])
            .filter_map(|(k, &(p, q))| {
                if p == at {
                    Some((k, q))
                } else if q == at {
                    Some((k, p))
                } else {
                    None
                }
            })
            .collect();
        let (k, next) = match options.len() {
            0 => return None,
            1 => options[0],
            _ => {
                let back = run.nodes[came_from] - run.nodes[at];
                *options
                    .iter()
                    .max_by(|x, y| {
                        let ang = |to: usize| {
                            let d = run.nodes[to] - run.nodes[at];
                            // Angle turned from the way back, clockwise.
                            let a = back.cross(d).atan2(back.dot(d));
                            if a <= 0.0 {
                                a + std::f64::consts::TAU
                            } else {
                                a
                            }
                        };
                        ang(y.1).total_cmp(&ang(x.1))
                    })
                    .expect("options")
            }
        };
        used[k] = true;
        came_from = at;
        at = next;
        if at != a {
            out.push(at);
        }
    }
    if at != a {
        return None;
    }
    let pts: Vec<Point2> = out.iter().map(|&k| run.nodes[k]).collect();
    Some(pts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn area(f: &[Point2]) -> f64 {
        signed_area(f)
    }

    /// A hip roof over a rectangle: two triangles and two trapezoids.
    #[test]
    fn rectangle() {
        let poly = [p(0.0, 0.0), p(12.0, 0.0), p(12.0, 8.0), p(0.0, 8.0)];
        let f = faces(&poly, &[1.0; 4]).unwrap();
        assert_eq!(f[0].len(), 4);
        assert_eq!(f[1].len(), 3);
        let total: f64 = f.iter().map(|x| area(x)).sum();
        assert!((total - 96.0).abs() < 1e-9, "{total}");
        assert!(f.iter().all(|x| area(x) > 0.0));
        // The ridge from (4, 4) to (8, 4).
        assert!(f[0].iter().any(|q| q.distance(p(8.0, 4.0)) < 1e-9));
        assert!(f[0].iter().any(|q| q.distance(p(4.0, 4.0)) < 1e-9));
        // Gables at the short ends: two planes meeting at the ridge.
        let g = faces(&poly, &[1.0, 0.0, 1.0, 0.0]).unwrap();
        assert!(g[1].is_empty() && g[3].is_empty());
        assert!((area(&g[0]) - 48.0).abs() < 1e-9 && (area(&g[2]) - 48.0).abs() < 1e-9);
    }

    /// An L: the valley from the re-entrant corner, faces covering it all.
    #[test]
    fn l_shape() {
        let poly = [
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 10.0),
            p(0.0, 10.0),
        ];
        let f = faces(&poly, &[1.0; 6]).unwrap();
        let total: f64 = f.iter().map(|x| area(x)).sum();
        assert!((total - (100.0 - 36.0)).abs() < 1e-9, "{total}");
        // The valley: the faces of the two sides at the re-entrant corner
        // share the line from (4, 4) to (2, 2).
        let has = |face: &[Point2], q: Point2| face.iter().any(|x| x.distance(q) < 1e-9);
        assert!(has(&f[2], p(2.0, 2.0)) && has(&f[3], p(2.0, 2.0)));
        assert!(has(&f[2], p(4.0, 4.0)) && has(&f[3], p(4.0, 4.0)));
        for face in &f {
            assert!(area(face) > 0.0, "{face:?}");
        }
    }

    /// A U and a T: splits; different pitches; and a crossing outline.
    #[test]
    fn u_and_t_shapes() {
        let u = [
            p(0.0, 0.0),
            p(12.0, 0.0),
            p(12.0, 10.0),
            p(8.0, 10.0),
            p(8.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 10.0),
            p(0.0, 10.0),
        ];
        let f = faces(&u, &[1.0; 8]).unwrap();
        let total: f64 = f.iter().map(|x| area(x)).sum();
        assert!((total - (120.0 - 24.0)).abs() < 1e-9, "{total}");
        let t = [
            p(0.0, 6.0),
            p(4.0, 6.0),
            p(4.0, 0.0),
            p(8.0, 0.0),
            p(8.0, 6.0),
            p(12.0, 6.0),
            p(12.0, 10.0),
            p(0.0, 10.0),
        ];
        let speeds = [1.0, 0.8, 1.0, 0.8, 1.0, 1.0, 0.5, 1.0];
        let f = faces(&t, &speeds).unwrap();
        let total: f64 = f.iter().map(|x| area(x)).sum();
        assert!((total - (48.0 + 24.0)).abs() < 1e-6, "{total}");
        assert!(f.iter().all(|x| area(x) > 0.0));
    }

    /// Faces tile the outline, and where planes meet they are at the same
    /// height (time): checked on many shapes and pitches.
    fn consistent(poly: &[Point2], speeds: &[f64]) {
        let f = faces(poly, speeds).unwrap_or_else(|| panic!("{poly:?} {speeds:?}"));
        let total: f64 = f.iter().filter(|x| !x.is_empty()).map(|x| area(x)).sum();
        let want = signed_area(poly);
        assert!(
            (total - want).abs() < 1e-6 * want.max(1.0),
            "{total} vs {want} {poly:?}"
        );
        let height = |i: usize, q: Point2| {
            let (a, nrm) = side(poly, i);
            (q - a).dot(nrm) / speeds[i]
        };
        for (i, fi) in f.iter().enumerate() {
            if fi.is_empty() {
                continue;
            }
            assert!(area(fi) > 0.0, "face {i} {fi:?}");
            for q in fi {
                for (j, fj) in f.iter().enumerate() {
                    if j != i && fj.iter().any(|r| r.distance(*q) < 1e-7) {
                        let (hi, hj) = (height(i, *q), height(j, *q));
                        assert!(
                            (hi - hj).abs() < 1e-6,
                            "{q:?}: {hi} vs {hj} ({i}, {j}) {poly:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn many_shapes() {
        let pts = |v: &[(f64, f64)]| v.iter().map(|(x, y)| p(*x, *y)).collect::<Vec<_>>();
        let shapes = [
            // Staircase.
            pts(&[
                (0.0, 0.0),
                (12.0, 0.0),
                (12.0, 4.0),
                (8.0, 4.0),
                (8.0, 8.0),
                (4.0, 8.0),
                (4.0, 12.0),
                (0.0, 12.0),
            ]),
            // Cross.
            pts(&[
                (4.0, 0.0),
                (8.0, 0.0),
                (8.0, 4.0),
                (12.0, 4.0),
                (12.0, 8.0),
                (8.0, 8.0),
                (8.0, 12.0),
                (4.0, 12.0),
                (4.0, 8.0),
                (0.0, 8.0),
                (0.0, 4.0),
                (4.0, 4.0),
            ]),
            // H.
            pts(&[
                (0.0, 0.0),
                (4.0, 0.0),
                (4.0, 5.0),
                (8.0, 5.0),
                (8.0, 0.0),
                (12.0, 0.0),
                (12.0, 14.0),
                (8.0, 14.0),
                (8.0, 9.0),
                (4.0, 9.0),
                (4.0, 14.0),
                (0.0, 14.0),
            ]),
            // L with wings of different widths.
            pts(&[
                (0.0, 0.0),
                (15.0, 0.0),
                (15.0, 6.0),
                (5.0, 6.0),
                (5.0, 11.0),
                (0.0, 11.0),
            ]),
            // A house with a bay.
            pts(&[
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 8.0),
                (7.0, 8.0),
                (7.0, 9.5),
                (3.0, 9.5),
                (3.0, 8.0),
                (0.0, 8.0),
            ]),
            // Not square corners.
            pts(&[
                (0.0, 0.0),
                (10.0, 0.0),
                (13.0, 5.0),
                (8.0, 4.0),
                (6.0, 9.0),
                (0.0, 7.0),
            ]),
        ];
        for poly in &shapes {
            let n = poly.len();
            consistent(poly, &vec![1.0; n]);
            let mixed: Vec<f64> = (0..n).map(|i| [1.0, 0.7, 1.3][i % 3]).collect();
            consistent(poly, &mixed);
        }
        // Gables: an L with gable ends, a cross with four.
        consistent(&shapes[3], &[1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
        consistent(
            &shapes[1],
            &[0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0],
        );
        // Convex polygons, all pitches the same and mixed.
        for k in 3..12 {
            let poly: Vec<Point2> = (0..k)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / k as f64 + 0.1;
                    let r = 5.0 + (i % 3) as f64;
                    p(r * a.cos(), r * a.sin())
                })
                .collect();
            if signed_area(&poly) > 0.0 {
                consistent(&poly, &vec![1.0; k]);
                let mixed: Vec<f64> = (0..k).map(|i| 0.6 + 0.2 * (i % 4) as f64).collect();
                consistent(&poly, &mixed);
            }
        }
    }
}
