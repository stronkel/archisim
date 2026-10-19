//! Closed areas found from a point inside them (AutoCAD's CONTORNO, and the
//! "pick internal point" of SOMBCONT): the smallest region bounded by the
//! given lines and arcs that contains the point, with the closed islands
//! inside it as holes.
//!
//! The segments are cut at every crossing into a planar graph; dangling
//! pieces are pruned; then each face is walked keeping it on the left.

use std::collections::HashMap;

use archisim_core::{Point2, Polyline};

use crate::seg::{angle_of, intersect, Seg, TOL};
use crate::{inside_loops, polyline_area, polyline_from_segs, CURVE_TOL};

/// Points closer than this are the same vertex.
const MERGE: f64 = 1e-6;

/// Why no boundary was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionError {
    /// The point is not inside any closed area.
    NotClosed,
}

impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("el punto no está dentro de un recinto cerrado")
    }
}

struct Graph {
    points: Vec<Point2>,
    /// Edges: from vertex, to vertex, the segment between them.
    edges: Vec<(usize, usize, Seg)>,
}

fn bbox(s: &Seg) -> (Point2, Point2) {
    let pts = s.tessellate(CURVE_TOL * 50.0);
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    // Arcs bulge between their tessellation points: a little slack.
    (lo - Point2::new(1e-3, 1e-3), hi + Point2::new(1e-3, 1e-3))
}

fn vertex(
    points: &mut Vec<Point2>,
    index: &mut HashMap<(i64, i64), Vec<usize>>,
    p: Point2,
) -> usize {
    let key = ((p.x / MERGE).round() as i64, (p.y / MERGE).round() as i64);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(v) = index.get(&(key.0 + dx, key.1 + dy)) {
                for i in v {
                    if points[*i].distance(p) <= MERGE * 2.0 {
                        return *i;
                    }
                }
            }
        }
    }
    points.push(p);
    let i = points.len() - 1;
    index.entry(key).or_default().push(i);
    i
}

fn build(segs: &[Seg]) -> Graph {
    let boxes: Vec<(Point2, Point2)> = segs.iter().map(bbox).collect();
    let mut cuts: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; segs.len()];
    for i in 0..segs.len() {
        if segs[i].is_full_circle() {
            // A circle is split at its start so it has a vertex.
            cuts[i].push(0.5);
        }
        for j in i + 1..segs.len() {
            let (a, b) = (boxes[i], boxes[j]);
            if a.0.x > b.1.x || b.0.x > a.1.x || a.0.y > b.1.y || b.0.y > a.1.y {
                continue;
            }
            for (ti, tj, _) in intersect(&segs[i], &segs[j]) {
                cuts[i].push(ti.clamp(0.0, 1.0));
                cuts[j].push(tj.clamp(0.0, 1.0));
            }
        }
    }
    let mut points = Vec::new();
    let mut index = HashMap::new();
    let mut edges = Vec::new();
    for (s, c) in segs.iter().zip(cuts.iter_mut()) {
        c.sort_by(f64::total_cmp);
        c.dedup_by(|a, b| (*a - *b).abs() * s.length() < MERGE);
        for w in c.windows(2) {
            let piece = s.sub(w[0], w[1]);
            if piece.length() < MERGE * 2.0 {
                continue;
            }
            let a = vertex(&mut points, &mut index, piece.start());
            let b = vertex(&mut points, &mut index, piece.end());
            if a != b {
                edges.push((a, b, piece));
            }
        }
    }
    Graph { points, edges }
}

/// Removes edges hanging from a vertex with no other edge, repeatedly.
fn prune(g: &mut Graph) {
    loop {
        let mut degree = vec![0usize; g.points.len()];
        for (a, b, _) in &g.edges {
            degree[*a] += 1;
            degree[*b] += 1;
        }
        let before = g.edges.len();
        g.edges.retain(|(a, b, _)| degree[*a] > 1 && degree[*b] > 1);
        if g.edges.len() == before {
            break;
        }
    }
}

/// Every face of the graph as a closed list of segments, walked with the
/// face on the left.
fn faces(g: &Graph) -> Vec<Vec<Seg>> {
    // Half-edges: 2e is edge e forwards, 2e+1 backwards.
    let n = g.edges.len();
    let half = |h: usize| -> (usize, usize, Seg) {
        let (a, b, s) = g.edges[h / 2];
        if h.is_multiple_of(2) {
            (a, b, s)
        } else {
            (b, a, s.reversed())
        }
    };
    let mut out_of: Vec<Vec<(f64, usize)>> = vec![Vec::new(); g.points.len()];
    for h in 0..2 * n {
        let (a, _, s) = half(h);
        out_of[a].push((angle_of(s.tangent(0.0)), h));
    }
    for v in &mut out_of {
        v.sort_by(|x, y| x.0.total_cmp(&y.0));
    }
    let mut used = vec![false; 2 * n];
    let mut result = Vec::new();
    for start in 0..2 * n {
        if used[start] {
            continue;
        }
        let mut face = Vec::new();
        let mut h = start;
        let mut guard = 0;
        loop {
            used[h] = true;
            let (_, b, s) = half(h);
            face.push(s);
            // The twin, leaving b; the next edge is the one just clockwise
            // of it.
            let twin = h ^ 1;
            let list = &out_of[b];
            let pos = list.iter().position(|(_, x)| *x == twin).expect("twin");
            let next = if pos == 0 {
                list[list.len() - 1].1
            } else {
                list[pos - 1].1
            };
            h = next;
            guard += 1;
            if h == start || used[h] || guard > 4 * n + 4 {
                break;
            }
        }
        if h == start {
            result.push(face);
        }
    }
    result
}

fn face_polyline(face: &[Seg]) -> Option<Polyline> {
    polyline_from_segs(face, true, 0.0)
}

/// The boundary around `p`: the smallest closed area made by the segments
/// that contains it, counter-clockwise, followed by the islands inside it
/// (closed groups of segments that do not touch it) as holes.
pub fn boundary_at(segs: &[Seg], p: Point2) -> Result<Vec<Polyline>, RegionError> {
    let segs: Vec<Seg> = segs.iter().copied().filter(|s| s.length() > TOL).collect();
    let mut g = build(&segs);
    prune(&mut g);
    let faces = faces(&g);
    let mut bounded: Vec<(f64, Polyline, Vec<Point2>)> = Vec::new();
    let mut outers: Vec<(Polyline, Vec<Point2>)> = Vec::new();
    for f in &faces {
        let Some(pl) = face_polyline(f) else { continue };
        let a = polyline_area(&pl);
        let poly = crate::loop_polygon(&pl, CURVE_TOL);
        if a > 1e-12 {
            bounded.push((a, pl, poly));
        } else if a < -1e-12 {
            outers.push((pl, poly));
        }
    }
    let (area, outer, outer_poly) = bounded
        .into_iter()
        .filter(|(_, _, poly)| inside_loops(std::slice::from_ref(poly), p))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .ok_or(RegionError::NotClosed)?;
    let mut loops = vec![outer];
    // Islands: the outer outline of another group, inside this face and
    // smaller than it, not around the point.
    for (pl, poly) in outers {
        let inner_area = polyline_area(&pl).abs();
        if inner_area >= area - 1e-9 {
            continue;
        }
        let inside = poly
            .iter()
            .all(|q| inside_loops(std::slice::from_ref(&outer_poly), *q));
        if inside && !inside_loops(std::slice::from_ref(&poly), p) {
            loops.push(pl);
        }
    }
    Ok(loops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn l(a: Point2, b: Point2) -> Seg {
        Seg::Line { a, b }
    }

    #[test]
    fn room_made_of_loose_lines_that_overrun() {
        // Four lines crossing at the corners like a hand-drawn grid, plus a
        // dividing line: two rooms.
        let segs = vec![
            l(p(-1.0, 0.0), p(11.0, 0.0)),
            l(p(-1.0, 5.0), p(11.0, 5.0)),
            l(p(0.0, -1.0), p(0.0, 6.0)),
            l(p(10.0, -1.0), p(10.0, 6.0)),
            l(p(4.0, 0.0), p(4.0, 5.0)),
        ];
        let b = boundary_at(&segs, p(2.0, 2.0)).unwrap();
        assert_eq!(b.len(), 1);
        assert!((polyline_area(&b[0]) - 20.0).abs() < 1e-9);
        let b = boundary_at(&segs, p(7.0, 2.0)).unwrap();
        assert!((polyline_area(&b[0]) - 30.0).abs() < 1e-9);
        assert_eq!(
            boundary_at(&segs, p(20.0, 2.0)),
            Err(RegionError::NotClosed)
        );
    }

    #[test]
    fn islands_become_holes_and_arcs_stay_exact() {
        let mut segs = vec![
            l(p(0.0, 0.0), p(10.0, 0.0)),
            l(p(10.0, 0.0), p(10.0, 10.0)),
            l(p(10.0, 10.0), p(0.0, 10.0)),
            l(p(0.0, 10.0), p(0.0, 0.0)),
        ];
        // A column (a circle) in the middle of the room.
        segs.push(Seg::Arc {
            centre: p(5.0, 5.0),
            radius: 1.0,
            start: 0.0,
            sweep: TAU,
        });
        let b = boundary_at(&segs, p(2.0, 2.0)).unwrap();
        assert_eq!(b.len(), 2);
        let hatch = archisim_core::Shape::Hatch(archisim_core::Hatch {
            loops: b,
            pattern: archisim_core::HatchPattern::Solid,
            scale: 1.0,
            angle: 0.0,
            background: None,
        });
        let a = crate::area(&hatch).unwrap();
        assert!((a - (100.0 - std::f64::consts::PI)).abs() < 1e-9, "{a}");
        // Inside the column: the circle itself.
        let b = boundary_at(&segs, p(5.0, 5.2)).unwrap();
        assert!((polyline_area(&b[0]) - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn dangling_lines_are_ignored() {
        let segs = vec![
            l(p(0.0, 0.0), p(4.0, 0.0)),
            l(p(4.0, 0.0), p(4.0, 4.0)),
            l(p(4.0, 4.0), p(0.0, 4.0)),
            l(p(0.0, 4.0), p(0.0, 0.0)),
            // A loose line from a wall into the room.
            l(p(2.0, 0.0), p(2.0, 2.0)),
        ];
        let b = boundary_at(&segs, p(1.0, 3.0)).unwrap();
        assert!((polyline_area(&b[0]) - 16.0).abs() < 1e-9);
        assert_eq!(b[0].points.len(), 5);
    }
}
