//! The streets of OpenStreetMap made into a street network, as a city
//! builder would: OpenStreetMap cuts a street into a new way at every
//! change of a tag and draws each carriageway of an avenue, and each
//! piece of a roundabout's ring, on its own. Brought one way to one
//! street, that is a heap of short pieces that overlap and meet where
//! they should not. Here:
//!
//! - the ways marked as a roundabout (`junction=roundabout|circular`) that
//!   close a ring become a roundabout (its circle), and the streets that
//!   reached it end on it or, if they cross its middle, pass through;
//! - the two one-way carriageways of an avenue, side by side and going
//!   opposite ways, become one avenue on the line between them, as wide
//!   apart as they are;
//! - the pieces of a street that only meet each other are joined into one
//!   street, while it goes on more or less straight.
//!
//! Pure geometry on the plan, without the model: [`network`].

use archisim_core::{Point2, Vec2};

/// A way of OpenStreetMap that is a street, on the plan.
#[derive(Debug, Clone)]
pub struct OsmStreet {
    pub points: Vec<Point2>,
    pub class: String,
    pub name: Option<String>,
    pub oneway: bool,
    pub roundabout: bool,
    pub lanes: Option<u32>,
}

/// A street of the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Street {
    pub axis: Vec<Point2>,
    pub class: String,
    pub name: Option<String>,
    pub oneway: bool,
    /// Two carriageways made one: how far apart their axes were.
    pub paired: Option<f64>,
}

/// A roundabout found in the ways: the circle of its ring's axis and the
/// width of its ring's carriageway.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub centre: Point2,
    /// Radius of the ring's axis.
    pub radius: f64,
    pub width: f64,
}

/// Rings smaller or larger than these are left as streets.
pub const RING_MIN: f64 = 4.0;
pub const RING_MAX: f64 = 70.0;
/// Carriageways further apart than this are not one avenue.
pub const PAIR_MAX: f64 = 40.0;
/// Pieces of a street turning more than this where they meet stay apart,
/// degrees.
pub const JOIN_TURN: f64 = 55.0;
/// Ends closer than this meet, metres.
const MEET: f64 = 1.5;

/// The network of the streets of OpenStreetMap: its streets and
/// roundabouts.
pub fn network(ways: &[OsmStreet]) -> (Vec<Street>, Vec<Ring>) {
    let (rings, ring_ways) = rings_of(ways);
    let mut streets: Vec<Street> = ways
        .iter()
        .enumerate()
        .filter(|(i, w)| !ring_ways.contains(i) && w.points.len() >= 2)
        .map(|(_, w)| Street {
            axis: w.points.clone(),
            class: w.class.clone(),
            name: w.name.clone(),
            oneway: w.oneway,
            paired: None,
        })
        .collect();
    // What is left inside a roundabout's island, out.
    streets.retain(|s| {
        !rings.iter().any(|r| {
            s.axis
                .iter()
                .all(|p| p.distance(r.centre) < r.radius - r.width / 2.0)
        })
    });
    let streets = join(streets);
    let streets = pair(streets);
    let mut streets = join(streets);
    // Their bends rounded, as a street is laid out (not where another
    // street reaches it: that is a crossing).
    let ends: Vec<Point2> = streets
        .iter()
        .flat_map(|s| [s.axis[0], s.axis[s.axis.len() - 1]])
        .collect();
    for s in &mut streets {
        s.axis = round_bends(&s.axis, bend_radius(&s.class), &ends);
    }
    (streets, rings)
}

/// The radius the bends of a street are rounded to, by its class.
pub fn bend_radius(class: &str) -> f64 {
    match class {
        "trunk" | "primary" => 40.0,
        "secondary" => 30.0,
        "tertiary" => 20.0,
        "living_street" | "pedestrian" => 6.0,
        _ => 12.0,
    }
}

/// A line with each bend rounded by an arc of `radius` (smaller where the
/// pieces each side are short), but not the bends at `keep` (where other
/// streets meet it) nor the sharp ones (more than 100°: a corner).
pub fn round_bends(line: &[Point2], radius: f64, keep: &[Point2]) -> Vec<Point2> {
    if line.len() < 3 {
        return line.to_vec();
    }
    let mut out = vec![line[0]];
    for k in 1..line.len() - 1 {
        let (a, b, c) = (line[k - 1], line[k], line[k + 1]);
        let (u, v) = (b - a, c - b);
        let (lu, lv) = (u.length(), v.length());
        if lu < 1e-6 || lv < 1e-6 {
            continue;
        }
        let (u, v) = (u * (1.0 / lu), v * (1.0 / lv));
        let turn = u.dot(v).clamp(-1.0, 1.0).acos();
        let kept = keep.iter().any(|q| q.distance(b) < MEET);
        if kept || turn < 2f64.to_radians() || turn > 100f64.to_radians() {
            out.push(b);
            continue;
        }
        // Where the arc leaves each piece: no more than near half of it.
        let mut t = radius * (turn / 2.0).tan();
        t = t.min(lu * 0.45).min(lv * 0.45);
        let r = t / (turn / 2.0).tan();
        let p0 = b - u * t;
        let p1 = b + v * t;
        // The arc's centre, on the inside of the bend.
        let left = u.cross(v) > 0.0;
        let n = if left {
            Vec2::new(-u.y, u.x)
        } else {
            Vec2::new(u.y, -u.x)
        };
        let centre = p0 + n * r;
        let a0 = (p0.y - centre.y).atan2(p0.x - centre.x);
        let sweep = if left { turn } else { -turn };
        let steps = ((r * turn) / 2.0).ceil().max(2.0) as usize;
        for i in 0..=steps {
            let ang = a0 + sweep * i as f64 / steps as f64;
            out.push(Point2::new(
                centre.x + r * ang.cos(),
                centre.y + r * ang.sin(),
            ));
        }
        let _ = p1;
    }
    out.push(line[line.len() - 1]);
    out
}

// ---------------------------------------------------------------------
// Roundabouts.

fn rings_of(ways: &[OsmStreet]) -> (Vec<Ring>, Vec<usize>) {
    let marked: Vec<usize> = (0..ways.len())
        .filter(|&i| ways[i].roundabout && ways[i].points.len() >= 2)
        .collect();
    // Join the marked ways that share an end into groups.
    let mut group: Vec<usize> = (0..marked.len()).collect();
    fn find(g: &mut [usize], i: usize) -> usize {
        let mut i = i;
        while g[i] != i {
            g[i] = g[g[i]];
            i = g[i];
        }
        i
    }
    for a in 0..marked.len() {
        for b in a + 1..marked.len() {
            let (wa, wb) = (&ways[marked[a]].points, &ways[marked[b]].points);
            let touch = [wa[0], wa[wa.len() - 1]].iter().any(|p| {
                [wb[0], wb[wb.len() - 1]]
                    .iter()
                    .any(|q| p.distance(*q) < MEET)
            });
            if touch {
                let (ra, rb) = (find(&mut group, a), find(&mut group, b));
                group[ra] = rb;
            }
        }
    }
    let mut rings = Vec::new();
    let mut used = Vec::new();
    let mut roots: Vec<usize> = (0..marked.len()).map(|i| find(&mut group, i)).collect();
    roots.sort_unstable();
    roots.dedup();
    for root in roots {
        let members: Vec<usize> = (0..marked.len())
            .filter(|&i| find(&mut group, i) == root)
            .map(|i| marked[i])
            .collect();
        let pts: Vec<Point2> = members
            .iter()
            .flat_map(|&i| ways[i].points.iter().copied())
            .collect();
        let Some((centre, radius, spread)) = circle_of(&pts) else {
            continue;
        };
        // It must go round its centre, nearly closed, and be a circle.
        let turn = sweep(&pts, centre);
        if !(RING_MIN..=RING_MAX).contains(&radius)
            || spread > 0.25 * radius
            || turn < 1.6 * std::f64::consts::PI
        {
            continue;
        }
        let lanes = members
            .iter()
            .filter_map(|&i| ways[i].lanes)
            .max()
            .unwrap_or(if radius < 14.0 { 1 } else { 2 });
        let width = (lanes as f64 * 4.0).min(radius * 0.9).max(4.0);
        rings.push(Ring {
            centre,
            radius,
            width,
        });
        used.extend(members);
    }
    (rings, used)
}

/// The centre and mean radius of points round a circle, with how far
/// they stray from it (the largest difference of radius).
fn circle_of(pts: &[Point2]) -> Option<(Point2, f64, f64)> {
    if pts.len() < 4 {
        return None;
    }
    // Least squares (Kåsa): x² + y² + D·x + E·y + F = 0, about the mean.
    let n = pts.len() as f64;
    let (mx, my) = (
        pts.iter().map(|p| p.x).sum::<f64>() / n,
        pts.iter().map(|p| p.y).sum::<f64>() / n,
    );
    let (mut suu, mut svv, mut suv, mut suuu, mut svvv, mut suvv, mut svuu) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in pts {
        let (u, v) = (p.x - mx, p.y - my);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
    }
    let det = suu * svv - suv * suv;
    if det.abs() < 1e-9 {
        return None;
    }
    let (b1, b2) = ((suuu + suvv) / 2.0, (svvv + svuu) / 2.0);
    let uc = (b1 * svv - b2 * suv) / det;
    let vc = (suu * b2 - suv * b1) / det;
    let centre = Point2::new(mx + uc, my + vc);
    let r = pts.iter().map(|p| p.distance(centre)).sum::<f64>() / n;
    let spread = pts
        .iter()
        .map(|p| (p.distance(centre) - r).abs())
        .fold(0.0, f64::max);
    Some((centre, r, spread))
}

/// How much of a turn round `c` the points cover, radians.
fn sweep(pts: &[Point2], c: Point2) -> f64 {
    let mut a: Vec<f64> = pts.iter().map(|p| (p.y - c.y).atan2(p.x - c.x)).collect();
    a.sort_by(f64::total_cmp);
    a.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
    if a.len() < 2 {
        return 0.0;
    }
    let mut gap = a[0] + std::f64::consts::TAU - a[a.len() - 1];
    for w in a.windows(2) {
        gap = gap.max(w[1] - w[0]);
    }
    std::f64::consts::TAU - gap
}

// ---------------------------------------------------------------------
// Joining the pieces of a street.

fn same_street(a: &Street, b: &Street) -> bool {
    a.class == b.class
        && a.name == b.name
        && a.oneway == b.oneway
        && match (a.paired, b.paired) {
            (None, None) => true,
            (Some(x), Some(y)) => (x - y).abs() < 6.0,
            _ => false,
        }
}

/// Joins the streets whose ends meet two by two, with no other street
/// there, while they go on in the same direction.
fn join(mut streets: Vec<Street>) -> Vec<Street> {
    loop {
        let mut joined = false;
        'search: for i in 0..streets.len() {
            for end_i in [false, true] {
                let p = end_of(&streets[i].axis, end_i);
                // Whatever else is at this point.
                let mut here = Vec::new();
                for (j, s) in streets.iter().enumerate() {
                    if j == i {
                        continue;
                    }
                    for end_j in [false, true] {
                        if end_of(&s.axis, end_j).distance(p) < MEET {
                            here.push((j, end_j));
                        }
                    }
                    if passes_by(&s.axis, p) {
                        here.push((j, true));
                        here.push((j, false));
                    }
                }
                let [(j, end_j)] = here[..] else {
                    continue;
                };
                if j == i || !same_street(&streets[i], &streets[j]) {
                    continue;
                }
                // i …→ p →… j: i must end there and j start there (for one
                // way streets their directions must agree).
                let mut a = streets[i].axis.clone();
                let mut b = streets[j].axis.clone();
                if !end_i {
                    a.reverse();
                }
                if end_j {
                    b.reverse();
                }
                if streets[i].oneway && (end_i == end_j) {
                    continue;
                }
                let turn = heading(&a, true)
                    .dot(heading(&b, false))
                    .clamp(-1.0, 1.0)
                    .acos();
                if turn.to_degrees() > JOIN_TURN {
                    continue;
                }
                a.extend(b.into_iter().skip(1));
                let oneway_reversed = streets[i].oneway && !end_i;
                if oneway_reversed {
                    a.reverse();
                }
                streets[i].axis = a;
                streets.remove(j);
                joined = true;
                break 'search;
            }
        }
        if !joined {
            return streets;
        }
    }
}

fn end_of(axis: &[Point2], last: bool) -> Point2 {
    if last {
        axis[axis.len() - 1]
    } else {
        axis[0]
    }
}

/// The unit direction at an end of a line, along it (`last`: arriving).
fn heading(axis: &[Point2], last: bool) -> Vec2 {
    let (a, b) = if last {
        let n = axis.len();
        (back_from(axis, n - 1, 6.0), axis[n - 1])
    } else {
        let r: Vec<Point2> = axis.iter().rev().copied().collect();
        let n = r.len();
        (axis[0], back_from(&r, n - 1, 6.0))
    };
    let d = b - a;
    let l = d.length().max(1e-9);
    d * (1.0 / l)
}

/// A point some metres back from point `i` of a line.
fn back_from(axis: &[Point2], i: usize, metres: f64) -> Point2 {
    let mut left = metres;
    let mut k = i;
    while k > 0 {
        let d = axis[k].distance(axis[k - 1]);
        if d >= left {
            let t = left / d.max(1e-9);
            return axis[k] + (axis[k - 1] - axis[k]) * t;
        }
        left -= d;
        k -= 1;
    }
    axis[0]
}

/// Whether a line passes by a point away from its ends.
fn passes_by(axis: &[Point2], p: Point2) -> bool {
    let n = axis.len();
    if axis[0].distance(p) < MEET || axis[n - 1].distance(p) < MEET {
        return false;
    }
    axis.windows(2).any(|w| nearest(w[0], w[1], p).0 < MEET)
}

/// Distance from `p` to segment `a b`, and the nearest point.
fn nearest(a: Point2, b: Point2, p: Point2) -> (f64, Point2, f64) {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let t = if l2 > 0.0 {
        ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let q = a + ab * t;
    (p.distance(q), q, t)
}

// ---------------------------------------------------------------------
// Two carriageways, one avenue.

fn samples(axis: &[Point2], step: f64) -> Vec<(Point2, Vec2)> {
    let mut out = Vec::new();
    for w in axis.windows(2) {
        let d = w[1] - w[0];
        let l = d.length();
        if l < 1e-9 {
            continue;
        }
        let u = d * (1.0 / l);
        let n = (l / step).ceil().max(1.0) as usize;
        for k in 0..n {
            out.push((w[0] + d * (k as f64 / n as f64), u));
        }
    }
    if let (Some(last), Some(w)) = (axis.last(), axis.windows(2).last()) {
        let d = w[1] - w[0];
        let l = d.length().max(1e-9);
        out.push((*last, d * (1.0 / l)));
    }
    out
}

/// The nearest point of a line to `p`, with the line's direction there.
fn nearest_on(axis: &[Point2], p: Point2) -> (f64, Point2, Vec2, bool) {
    let mut best = (f64::INFINITY, axis[0], Vec2::new(1.0, 0.0), true);
    let n = axis.len();
    for (k, w) in axis.windows(2).enumerate() {
        let (d, q, t) = nearest(w[0], w[1], p);
        if d < best.0 {
            let dir = w[1] - w[0];
            let l = dir.length().max(1e-9);
            let at_end = (k == 0 && t <= 1e-6) || (k == n - 2 && t >= 1.0 - 1e-6);
            best = (d, q, dir * (1.0 / l), at_end);
        }
    }
    best
}

/// How a one way street sits beside another: the share of its samples
/// with the other beside it going the other way, and their distances.
fn beside(a: &Street, b: &Street) -> (f64, Vec<f64>) {
    let s = samples(&a.axis, 4.0);
    let mut d = Vec::new();
    for (p, u) in &s {
        let (dist, _, v, at_end) = nearest_on(&b.axis, *p);
        if !at_end && dist <= PAIR_MAX && u.dot(v) < -0.8 {
            d.push(dist);
        }
    }
    (d.len() as f64 / s.len().max(1) as f64, d)
}

fn pair(streets: Vec<Street>) -> Vec<Street> {
    let mut taken = vec![false; streets.len()];
    let mut out = Vec::new();
    for i in 0..streets.len() {
        if taken[i] || !streets[i].oneway {
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for j in 0..streets.len() {
            if j == i || taken[j] || !streets[j].oneway {
                continue;
            }
            let (a, b) = (&streets[i], &streets[j]);
            if a.class != b.class || (a.name.is_some() && b.name.is_some() && a.name != b.name) {
                continue;
            }
            let (fa, da) = beside(a, b);
            let (fb, _) = beside(b, a);
            if fa < 0.6 || fb < 0.6 || da.is_empty() {
                continue;
            }
            let mut d = da;
            d.sort_by(f64::total_cmp);
            let med = d[d.len() / 2];
            if med < 3.0 {
                continue;
            }
            if best.is_none_or(|(_, m)| med < m) {
                best = Some((j, med));
            }
        }
        let Some((j, gap)) = best else {
            continue;
        };
        taken[i] = true;
        taken[j] = true;
        let (a, b) = (&streets[i], &streets[j]);
        // The line between them, along the first, and what of either is
        // left beyond the other.
        let mut mid = Vec::new();
        let mut alone_a: Vec<Vec<Point2>> = vec![Vec::new()];
        for (p, u) in samples(&a.axis, 4.0) {
            let (dist, q, v, at_end) = nearest_on(&b.axis, p);
            if !at_end && dist <= PAIR_MAX && u.dot(v) < -0.8 {
                mid.push(Point2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0));
                if !alone_a.last().is_some_and(Vec::is_empty) {
                    alone_a.push(Vec::new());
                }
            } else {
                alone_a.last_mut().unwrap().push(p);
            }
        }
        let mut alone_b: Vec<Vec<Point2>> = vec![Vec::new()];
        for (p, u) in samples(&b.axis, 4.0) {
            let (dist, _, v, at_end) = nearest_on(&a.axis, p);
            if !at_end && dist <= PAIR_MAX && u.dot(v) < -0.8 {
                if !alone_b.last().is_some_and(Vec::is_empty) {
                    alone_b.push(Vec::new());
                }
            } else {
                alone_b.last_mut().unwrap().push(p);
            }
        }
        let mid = thin(&mid, 0.3);
        if mid.len() >= 2 {
            out.push(Street {
                axis: mid,
                class: a.class.clone(),
                name: a.name.clone().or_else(|| b.name.clone()),
                oneway: false,
                paired: Some(gap),
            });
        }
        for (rest, s) in alone_a
            .into_iter()
            .map(|r| (r, a))
            .chain(alone_b.into_iter().map(|r| (r, b)))
        {
            if rest.len() >= 2 && line_length(&rest) >= 10.0 {
                out.push(Street {
                    axis: thin(&rest, 0.3),
                    ..s.clone()
                });
            }
        }
    }
    out.extend(
        streets
            .into_iter()
            .zip(taken)
            .filter(|(_, t)| !t)
            .map(|(s, _)| s),
    );
    out
}

fn line_length(l: &[Point2]) -> f64 {
    l.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Leaves out the points of a line that lie on the straight between
/// their neighbours (within `tol`).
fn thin(line: &[Point2], tol: f64) -> Vec<Point2> {
    if line.len() <= 2 {
        return line.to_vec();
    }
    let mut out = vec![line[0]];
    for k in 1..line.len() - 1 {
        let a = *out.last().unwrap();
        let (d, _, _) = nearest(a, line[k + 1], line[k]);
        if d > tol {
            out.push(line[k]);
        }
    }
    out.push(line[line.len() - 1]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn way(points: &[(f64, f64)], class: &str, oneway: bool, roundabout: bool) -> OsmStreet {
        OsmStreet {
            points: points.iter().map(|&(x, y)| Point2::new(x, y)).collect(),
            class: class.into(),
            name: Some("Avenida".into()),
            oneway,
            roundabout,
            lanes: None,
        }
    }

    fn arc(c: (f64, f64), r: f64, from: f64, to: f64) -> Vec<(f64, f64)> {
        (0..=12)
            .map(|k| {
                let a = (from + (to - from) * k as f64 / 12.0).to_radians();
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect()
    }

    /// A roundabout of 20 m drawn in four ways, a road crossing its
    /// middle (cut at the ring as OpenStreetMap does) and an avenue of
    /// two carriageways reaching it from the north, each cut in two.
    fn junction() -> Vec<OsmStreet> {
        let mut w = Vec::new();
        for q in 0..4 {
            let a = q as f64 * 90.0;
            w.push(way(
                &arc((0.0, 0.0), 20.0, a, a + 90.0),
                "primary",
                true,
                true,
            ));
        }
        w.push(way(
            &[(-150.0, 0.0), (-80.0, 0.0)],
            "secondary",
            false,
            false,
        ));
        w.push(way(
            &[(-80.0, 0.0), (-20.0, 0.0)],
            "secondary",
            false,
            false,
        ));
        w.push(way(&[(-20.0, 0.0), (20.0, 0.0)], "secondary", false, false));
        w.push(way(&[(20.0, 0.0), (150.0, 0.0)], "secondary", false, false));
        // Inbound west carriageway (down), outbound east one (up).
        w.push(way(&[(-6.0, 150.0), (-6.0, 80.0)], "primary", true, false));
        w.push(way(&[(-6.0, 80.0), (-6.0, 19.1)], "primary", true, false));
        w.push(way(&[(6.0, 19.1), (6.0, 90.0)], "primary", true, false));
        w.push(way(&[(6.0, 90.0), (6.0, 150.0)], "primary", true, false));
        w
    }

    #[test]
    fn a_roundabout_with_a_road_through_and_an_avenue() {
        let (streets, rings) = network(&junction());
        assert_eq!(rings.len(), 1);
        assert!(rings[0].centre.distance(Point2::new(0.0, 0.0)) < 0.5);
        assert!((rings[0].radius - 20.0).abs() < 0.5);
        // The road through: one street from end to end.
        let through: Vec<_> = streets.iter().filter(|s| s.class == "secondary").collect();
        assert_eq!(through.len(), 1, "{through:?}");
        assert!((line_length(&through[0].axis) - 300.0).abs() < 1.0);
        // The avenue: one street on its middle, 12 m between carriageways.
        let avenue: Vec<_> = streets.iter().filter(|s| s.class == "primary").collect();
        assert_eq!(avenue.len(), 1, "{avenue:?}");
        assert!(avenue[0].axis.iter().all(|p| p.x.abs() < 0.5));
        assert!((avenue[0].paired.unwrap() - 12.0).abs() < 0.5);
        assert!(!avenue[0].oneway);
    }

    #[test]
    fn streets_that_turn_or_branch_stay_apart() {
        let w = vec![
            way(&[(0.0, 0.0), (50.0, 0.0)], "residential", false, false),
            way(&[(50.0, 0.0), (50.0, 50.0)], "residential", false, false),
            way(&[(50.0, 0.0), (100.0, 0.0)], "residential", false, false),
        ];
        let (streets, rings) = network(&w);
        assert!(rings.is_empty());
        assert_eq!(streets.len(), 3);
        // A bend of 90° stays two streets; a gentle one joins.
        let w = vec![
            way(&[(0.0, 0.0), (50.0, 0.0)], "residential", false, false),
            way(&[(50.0, 0.0), (50.0, 50.0)], "residential", false, false),
        ];
        assert_eq!(network(&w).0.len(), 2);
        let w = vec![
            way(&[(0.0, 0.0), (50.0, 0.0)], "residential", false, false),
            way(&[(100.0, 20.0), (50.0, 0.0)], "residential", false, false),
        ];
        assert_eq!(network(&w).0.len(), 1);
    }

    #[test]
    fn bends_are_rounded_but_not_crossings() {
        let line = vec![
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(150.0, 50.0),
        ];
        let round = round_bends(&line, 30.0, &[]);
        assert!(round.len() > 5);
        // Smooth: no turn between samples sharper than a few degrees.
        for w in round.windows(3) {
            let (u, v) = (w[1] - w[0], w[2] - w[1]);
            let t = (u.dot(v) / (u.length() * v.length()))
                .clamp(-1.0, 1.0)
                .acos();
            assert!(t.to_degrees() < 10.0, "{}", t.to_degrees());
        }
        // It stays in the corner's two pieces' reach, ends where it did.
        assert_eq!(round[0], line[0]);
        assert_eq!(*round.last().unwrap(), line[2]);
        // Where another street reaches the bend, it is a crossing: kept.
        let kept = round_bends(&line, 30.0, &[Point2::new(100.0, 0.0)]);
        assert_eq!(kept, line);
    }

    #[test]
    fn a_road_bent_like_an_arc_is_not_a_roundabout() {
        let w = vec![way(
            &arc((0.0, 0.0), 30.0, 0.0, 180.0),
            "primary",
            true,
            true,
        )];
        let (streets, rings) = network(&w);
        assert!(rings.is_empty());
        assert_eq!(streets.len(), 1);
    }
}
