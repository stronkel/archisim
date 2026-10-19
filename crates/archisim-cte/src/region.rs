//! The floor of a room as a walkable region: an outline with holes, and
//! obstacles (furniture, stairs) where they count. Walking distances inside
//! it (the shortest path round its corners), and the largest free circle
//! (the turning circle of a wheelchair).

use archisim_core::{Point2, Vec2};

/// An outline with holes, both as closed rings without repeating the first
/// point.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub outline: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
}

const EPS: f64 = 1e-7;

fn ring_contains(ring: &[Point2], p: Point2) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Distance from `p` to the segment `ab`.
pub fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-18 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn cross(o: Point2, a: Point2, b: Point2) -> f64 {
    (a - o).cross(b - o)
}

/// Whether `ab` and `cd` cross at a point inside both (not just touch).
fn crosses(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let d1 = cross(a, b, c);
    let d2 = cross(a, b, d);
    let d3 = cross(c, d, a);
    let d4 = cross(c, d, b);
    ((d1 > EPS && d2 < -EPS) || (d1 < -EPS && d2 > EPS))
        && ((d3 > EPS && d4 < -EPS) || (d3 < -EPS && d4 > EPS))
}

impl Region {
    pub fn new(outline: Vec<Point2>, holes: Vec<Vec<Point2>>) -> Self {
        Self { outline, holes }
    }

    fn rings(&self) -> impl Iterator<Item = &Vec<Point2>> {
        std::iter::once(&self.outline).chain(self.holes.iter())
    }

    /// Every edge of the outline and the holes.
    pub fn edges(&self) -> impl Iterator<Item = (Point2, Point2)> + '_ {
        self.rings().flat_map(|r| {
            let n = r.len();
            (0..n).map(move |i| (r[i], r[(i + 1) % n]))
        })
    }

    /// Strictly inside: in the outline, not in a hole.
    pub fn contains(&self, p: Point2) -> bool {
        ring_contains(&self.outline, p) && !self.holes.iter().any(|h| ring_contains(h, p))
    }

    /// Distance from `p` to the nearest edge.
    pub fn boundary_distance(&self, p: Point2) -> f64 {
        self.edges()
            .map(|(a, b)| segment_distance(p, a, b))
            .fold(f64::INFINITY, f64::min)
    }

    fn inside_or_on(&self, p: Point2) -> bool {
        self.contains(p) || self.boundary_distance(p) < 1e-6
    }

    /// Whether one can walk straight from `a` to `b` without leaving the
    /// region (running along a wall is allowed).
    pub fn visible(&self, a: Point2, b: Point2) -> bool {
        if self.edges().any(|(c, d)| crosses(a, b, c, d)) {
            return false;
        }
        [0.25, 0.5, 0.75]
            .iter()
            .all(|t| self.inside_or_on(a + (b - a) * *t))
    }

    /// The corners a shortest path may turn round: the reflex corners of
    /// the outline and the convex corners of the holes.
    fn turning_points(&self) -> Vec<Point2> {
        let mut out = Vec::new();
        let area = archisim_core::signed_area(&self.outline);
        let ring_points = |ring: &[Point2], reflex_sign: f64, out: &mut Vec<Point2>| {
            let n = ring.len();
            for i in 0..n {
                let (p, c, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                if cross(p, c, q) * reflex_sign > EPS {
                    out.push(c);
                }
            }
        };
        // Reflex corners of the outline turn against its orientation.
        ring_points(&self.outline, -area.signum(), &mut out);
        for h in &self.holes {
            // The convex corners of a hole, seen from inside it.
            let ha = archisim_core::signed_area(h);
            ring_points(h, ha.signum(), &mut out);
        }
        out
    }

    /// Walking distances from `from` to each of `to` inside the region
    /// (infinite where it cannot be reached), with the path to each.
    pub fn paths(&self, from: Point2, to: &[Point2]) -> Vec<(f64, Vec<Point2>)> {
        let mut nodes = vec![from];
        nodes.extend(self.turning_points());
        let first_target = nodes.len();
        nodes.extend_from_slice(to);
        let n = nodes.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        dist[0] = 0.0;
        for _ in 0..n {
            let Some(u) = (0..n)
                .filter(|&i| !done[i] && dist[i].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
            else {
                break;
            };
            done[u] = true;
            // Paths do not go through other targets' points (they are on
            // the boundary, at doors).
            if u >= first_target {
                continue;
            }
            for v in 0..n {
                if done[v] || v == u {
                    continue;
                }
                let d = dist[u] + nodes[u].distance(nodes[v]);
                if d < dist[v] - 1e-12 && self.visible(nodes[u], nodes[v]) {
                    dist[v] = d;
                    prev[v] = u;
                }
            }
        }
        (first_target..n)
            .map(|t| {
                if !dist[t].is_finite() {
                    return (f64::INFINITY, Vec::new());
                }
                let mut path = vec![nodes[t]];
                let mut k = t;
                while prev[k] != usize::MAX {
                    k = prev[k];
                    path.push(nodes[k]);
                }
                path.reverse();
                (dist[t], path)
            })
            .collect()
    }

    /// Points to measure from: just inside every corner and on a grid
    /// across the floor (the farthest point of a room is near a corner).
    pub fn sample_points(&self, grid: f64) -> Vec<Point2> {
        let mut out = Vec::new();
        let n = self.outline.len();
        for i in 0..n {
            let (p, c, q) = (
                self.outline[(i + n - 1) % n],
                self.outline[i],
                self.outline[(i + 1) % n],
            );
            let a = (p - c).normalized();
            let b = (q - c).normalized();
            if let (Some(a), Some(b)) = (a, b) {
                let bis = a + b;
                for dir in [bis, -bis] {
                    if let Some(d) = dir.normalized() {
                        let x = c + d * 0.1;
                        if self.contains(x) {
                            out.push(x);
                            break;
                        }
                    }
                }
            }
        }
        let (lo, hi) = bounds(&self.outline);
        let step = grid.max((hi.x - lo.x).max(hi.y - lo.y) / 40.0);
        let mut y = lo.y + step / 2.0;
        while y < hi.y {
            let mut x = lo.x + step / 2.0;
            while x < hi.x {
                let p = Point2::new(x, y);
                if self.contains(p) {
                    out.push(p);
                }
                x += step;
            }
            y += step;
        }
        out
    }

    /// The largest circle on the floor clear of the walls and of the given
    /// obstacles (outlines), as (centre, radius).
    pub fn largest_free_circle(&self, obstacles: &[Vec<Point2>]) -> Option<(Point2, f64)> {
        let free = |p: Point2| -> Option<f64> {
            if !self.contains(p) || obstacles.iter().any(|o| ring_contains(o, p)) {
                return None;
            }
            let mut d = self.boundary_distance(p);
            for o in obstacles {
                let n = o.len();
                for i in 0..n {
                    d = d.min(segment_distance(p, o[i], o[(i + 1) % n]));
                }
            }
            Some(d)
        };
        let (lo, hi) = bounds(&self.outline);
        let size = (hi.x - lo.x).max(hi.y - lo.y);
        let mut step = (size / 60.0).max(0.05);
        let mut best: Option<(Point2, f64)> = None;
        let mut y = lo.y + step / 2.0;
        while y < hi.y {
            let mut x = lo.x + step / 2.0;
            while x < hi.x {
                let p = Point2::new(x, y);
                if let Some(d) = free(p) {
                    if best.is_none_or(|(_, b)| d > b) {
                        best = Some((p, d));
                    }
                }
                x += step;
            }
            y += step;
        }
        // Refine round the best point.
        let (mut c, mut r) = best?;
        for _ in 0..6 {
            let mut improved = (c, r);
            for i in -3..=3 {
                for j in -3..=3 {
                    let p = c + Vec2::new(i as f64, j as f64) * (step / 3.0);
                    if let Some(d) = free(p) {
                        if d > improved.1 {
                            improved = (p, d);
                        }
                    }
                }
            }
            (c, r) = improved;
            step /= 3.0;
        }
        Some((c, r))
    }

    /// Width of a corridor: the diameter of the largest circle in it.
    pub fn width(&self) -> f64 {
        self.largest_free_circle(&[]).map_or(0.0, |(_, r)| 2.0 * r)
    }
}

pub fn bounds(points: &[Point2]) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    /// Round the inside corner of an L, not across it.
    #[test]
    fn paths_turn_round_corners() {
        // An L: 10 × 2 along x, then 2 × 10 up at the right end.
        let l = Region::new(
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 10.0),
                Point2::new(8.0, 10.0),
                Point2::new(8.0, 2.0),
                Point2::new(0.0, 2.0),
            ],
            Vec::new(),
        );
        let from = Point2::new(1.0, 1.0);
        let to = Point2::new(9.0, 9.0);
        let r = l.paths(from, &[to]);
        let expected = from.distance(Point2::new(8.0, 2.0)) + Point2::new(8.0, 2.0).distance(to);
        assert!((r[0].0 - expected).abs() < 1e-9, "{} {expected}", r[0].0);
        assert_eq!(r[0].1.len(), 3);
        // Round a pillar.
        let room = Region::new(rect(0.0, 0.0, 10.0, 4.0), vec![rect(4.0, 1.0, 6.0, 3.0)]);
        let r = room.paths(Point2::new(1.0, 2.0), &[Point2::new(9.0, 2.0)]);
        assert!(r[0].0 > 8.0 + 0.1 && r[0].0 < 9.0, "{}", r[0].0);
    }

    #[test]
    fn free_circles_and_widths() {
        let room = Region::new(rect(0.0, 0.0, 3.0, 2.0), Vec::new());
        let (_, r) = room.largest_free_circle(&[]).unwrap();
        assert!((r - 1.0).abs() < 0.01, "{r}");
        // A wardrobe along one side leaves 1,4 m.
        let (_, r) = room
            .largest_free_circle(&[rect(0.0, 1.4, 3.0, 2.0)])
            .unwrap();
        assert!((r - 0.7).abs() < 0.01, "{r}");
        let corridor = Region::new(rect(0.0, 0.0, 8.0, 1.1), Vec::new());
        assert!((corridor.width() - 1.1).abs() < 0.01);
    }
}
