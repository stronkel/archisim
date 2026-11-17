//! Crossroads (U1): where two roads of a level meet — they cross, one ends
//! on the other (a T) or both end at the same point (an L) — the
//! carriageways join in one plate and the kerbs and sidewalks turn the
//! corner with their radius, instead of the two roads lying one over the
//! other.
//!
//! A junction is found from the axes. Each road that meets there gives one
//! or two *arms* (the road going out of the junction one way or the
//! other). Between two arms next to each other round the junction, the
//! kerb lines of their carriageways (the edges of their lanes, parking and
//! bike lanes) are joined by an arc of the road's corner radius; nearly in
//! line, they go straight on; turning back (the outside of an L), they
//! meet in a point. Each arm is cut where its corners end, and the roads
//! leave their pieces out over those stretches ([`road_gaps`]): the
//! junction's plate, kerbs and sidewalk corners take their place.
//!
//! Nothing here is stored: it is worked out from the roads each time, so
//! it follows them as they move.

use std::sync::{Arc, Mutex};

use archisim_core::{
    offset_path, strip, ArmRef, ArmSign, CornerShape, GlobalId, IslandFinish, LevelId, MaterialId,
    Model, NodeEnd, NodeKey, NodeKind, NodeSettings, Point2, Road, RoadBand, RoadId, Roundabout,
    RoundaboutId, Vec2,
};

/// A road going out of a junction.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    pub road: RoadId,
    /// The point of its axis its distances out are measured from (where
    /// the junction is on the road, or where its axis would reach it).
    pub origin: Point2,
    /// Where the junction is along the road.
    pub station: f64,
    /// Whether the arm goes on along the road (its distances growing) or
    /// back.
    pub forward: bool,
    /// Its direction out of the junction.
    pub dir: Vec2,
    /// How far its carriageway reaches to each side (looking out along the
    /// arm), its kerbs' widths and its whole width to each side.
    pub kerb_left: f64,
    pub kerb_right: f64,
    pub kerb_width_left: f64,
    pub kerb_width_right: f64,
    pub edge_left: f64,
    pub edge_right: f64,
    /// How far its lanes for vehicles (not the parking) reach to each side
    /// of its axis: where a stop line ends.
    pub lanes_left: f64,
    pub lanes_right: f64,
    /// Where it is cut, from the junction.
    pub cut: f64,
}

/// A junction of roads and what it is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Junction {
    pub at: Point2,
    /// The road it belongs to (the first one): its level, its materials.
    pub owner: RoadId,
    pub arms: Vec<Arm>,
    /// Its carriageway, counter-clockwise.
    pub plate: Vec<Point2>,
    /// The kerbs round its corners.
    pub kerbs: Vec<Vec<Point2>>,
    /// The sidewalk at each corner.
    pub sidewalks: Vec<Vec<Point2>>,
    /// Height of its carriageway over the level.
    pub top: f64,
    /// How far its kerbs stand (0: a single-level street meets it).
    pub kerb_height: f64,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
    pub depth: f64,
    /// The roundabout or square it is (U3).
    pub ring: Option<RoundaboutId>,
    /// What else it has: a roundabout's island and apron, a shared
    /// surface's bollards.
    pub parts: Vec<JunctionPart>,
    /// Which node it is (N1; none: a roundabout's).
    pub key: Option<NodeKey>,
    /// What node it is (N1).
    pub kind: NodeKind,
    /// How far its carriageway rises over the streets' (a raised crossing
    /// or a shared surface: to the sidewalk), with ramps [`RAMP`] long at
    /// the ends of its arms.
    pub lift: f64,
    /// The ramps from its plateau down to each arm's carriageway.
    pub ramps: Vec<Vec<Point2>>,
    /// Flat at the height of its centre (N1).
    pub flat: bool,
    /// The street whose height it follows (N1; none: the nearest).
    pub main: Option<RoadId>,
    /// Its road markings: stop and give-way lines (N1), as thin polygons on
    /// its arms' carriageways.
    pub marks: Vec<Vec<Point2>>,
    /// Its signs: where each post stands, which way it faces (towards the
    /// traffic coming in) and what it is.
    pub signs: Vec<(Point2, Vec2, ArmSign)>,
    /// Stretches of its roads (road, from, to along the axis) where their
    /// own markings are left out (a node without markings).
    pub quiet: Vec<(RoadId, f64, f64)>,
    /// The kerbs across the parking where its bump-outs end (N1): on its
    /// roads, just past its arms' ends.
    pub bump_kerbs: Vec<Vec<Point2>>,
}

/// The length of the ramps of a raised node, metres.
pub const RAMP: f64 = 2.0;

/// A piece of a junction besides its plate, kerbs and sidewalks: its
/// outline, what it is, its material and how far it reaches under and
/// over the junction's carriageway.
#[derive(Debug, Clone, PartialEq)]
pub struct JunctionPart {
    pub band: RoadBand,
    pub outline: Vec<Point2>,
    pub material: MaterialId,
    pub z0: f64,
    pub z1: f64,
}

impl Junction {
    /// The height of its pavement at `p`, over the level: a roundabout's
    /// own, or that of the nearest of the roads that meet there (so that
    /// its plate follows them when they climb, U4).
    pub fn top_under(&self, model: &Model, p: Point2) -> f64 {
        if self.ring.is_some() || self.flat {
            return self.top;
        }
        // The main street rules: the others come to its height (N1).
        if let Some(r) = self.main.and_then(|m| model.road(m)) {
            return r.top_under(p);
        }
        let mut best: Option<(f64, f64)> = None;
        for a in &self.arms {
            let Some(r) = model.road(a.road) else {
                continue;
            };
            if !r.sloped() {
                continue;
            }
            // How far from the arm's line, and the road's height there.
            let off = (p - a.origin).cross(a.dir).abs();
            let h = r.top_under(p);
            if best.is_none_or(|(d, _)| off < d) {
                best = Some((off, h));
            }
        }
        match best {
            Some((_, h)) => h,
            None => self.top,
        }
    }

    /// The height of its carriageway at `p`, over the level: as
    /// [`Junction::top_under`], raised by its lift on the plateau and
    /// down its ramps at the ends of the arms (N1).
    pub fn plate_top(&self, model: &Model, p: Point2) -> f64 {
        self.top_under(model, p) + self.lift_at(p)
    }

    /// How much its carriageway is raised at `p`.
    pub fn lift_at(&self, p: Point2) -> f64 {
        if self.lift <= 0.0 {
            return 0.0;
        }
        let mut f: f64 = 1.0;
        for a in &self.arms {
            let s = (p - a.origin).dot(a.dir);
            let side = (p - a.origin).cross(a.dir);
            // Within the arm's carriageway (left is negative cross).
            let inside = -side <= a.kerb_left + 0.01 && side <= a.kerb_right + 0.01;
            if inside && s > a.cut - RAMP - 1e-6 {
                f = f.min(((a.cut - s) / RAMP).clamp(0.0, 1.0));
            }
        }
        self.lift * f
    }

    /// Its arm as the node's settings name it.
    pub fn arm_ref(a: &Arm) -> ArmRef {
        ArmRef {
            road: a.road,
            forward: a.forward,
        }
    }
}

/// Points closer than this are one junction.
const SAME: f64 = 1.0;
/// Segments of the arc of a corner.
const ARC_STEPS: usize = 8;
/// Nearly in line: the kerb goes straight on.
const STRAIGHT: f64 = 10.0 * std::f64::consts::PI / 180.0;

fn length(path: &[Point2]) -> f64 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point at `s` along a path and its direction there.
fn along(path: &[Point2], s: f64) -> (Point2, Vec2) {
    let mut left = s.max(0.0);
    for w in path.windows(2) {
        let l = w[0].distance(w[1]);
        let u = (w[1] - w[0]) * (1.0 / l.max(1e-12));
        if left <= l + 1e-9 {
            return (w[0] + u * left, u);
        }
        left -= l;
    }
    let n = path.len();
    let u = (path[n - 1] - path[n - 2]) * (1.0 / path[n - 2].distance(path[n - 1]).max(1e-12));
    (path[n - 1], u)
}

/// Where two segments cross: the distances along each.
fn crossing(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<(f64, f64)> {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    ((-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u)).then_some((t, u))
}

/// Where two lines meet (each by a point and a direction).
fn meet(p: Point2, u: Vec2, q: Point2, v: Vec2) -> Option<Point2> {
    let den = u.cross(v);
    if den.abs() < 1e-9 {
        return None;
    }
    let t = (q - p).cross(v) / den;
    Some(p + u * t)
}

/// The nearest point of a road's axis to `p`: its distance along the axis
/// and how far it is.
fn nearest_on(r: &Road, p: Point2) -> (f64, f64) {
    let mut best = (0.0, f64::INFINITY);
    let mut s = 0.0;
    for w in r.axis.windows(2) {
        let (a, b) = (w[0], w[1]);
        let l = a.distance(b);
        let t = if l > 1e-12 {
            ((p - a).dot(b - a) / (l * l)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = (a + (b - a) * t).distance(p);
        if d < best.1 {
            best = (s + l * t, d);
        }
        s += l;
    }
    best
}

/// The points where roads meet, with the distance along each road there.
fn meeting_points(roads: &[&Road]) -> Vec<(Point2, Vec<(usize, f64)>)> {
    let mut found: Vec<(Point2, Vec<(usize, f64)>)> = Vec::new();
    let mut add = |p: Point2, i: usize, si: f64, j: usize, sj: f64| {
        let k = match found.iter().position(|(q, _)| q.distance(p) < SAME) {
            Some(k) => k,
            None => {
                found.push((p, Vec::new()));
                found.len() - 1
            }
        };
        for (r, s) in [(i, si), (j, sj)] {
            if !found[k].1.iter().any(|(x, _)| *x == r) {
                found[k].1.push((r, s));
            }
        }
    };
    // Only pairs whose boxes (their axes and their width round them)
    // touch can meet: swept along x, so a town of hundreds of streets does
    // not compare each with all the others.
    let boxes: Vec<(Point2, Point2)> = roads
        .iter()
        .map(|r| {
            let grow = r.widest() + 1.0;
            let (lo, hi) = r.axis.iter().fold(
                (
                    Point2::new(f64::INFINITY, f64::INFINITY),
                    Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |(lo, hi), p| {
                    (
                        Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                        Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
                    )
                },
            );
            (
                Point2::new(lo.x - grow, lo.y - grow),
                Point2::new(hi.x + grow, hi.y + grow),
            )
        })
        .collect();
    let mut by_x: Vec<usize> = (0..roads.len()).collect();
    by_x.sort_by(|&a, &b| boxes[a].0.x.total_cmp(&boxes[b].0.x));
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (k, &a) in by_x.iter().enumerate() {
        for &b in &by_x[k + 1..] {
            if boxes[b].0.x > boxes[a].1.x {
                break;
            }
            if boxes[b].0.y <= boxes[a].1.y && boxes[a].0.y <= boxes[b].1.y {
                pairs.push((a.min(b), a.max(b)));
            }
        }
    }
    // In the same order as comparing each with all, so the junctions come
    // out the same.
    pairs.sort_unstable();
    for (i, j) in pairs {
        {
            let (a, b) = (roads[i], roads[j]);
            // Axes that cross.
            let mut sa = 0.0;
            for wa in a.axis.windows(2) {
                let la = wa[0].distance(wa[1]);
                let mut sb = 0.0;
                for wb in b.axis.windows(2) {
                    let lb = wb[0].distance(wb[1]);
                    if let Some((t, u)) = crossing(wa[0], wa[1], wb[0], wb[1]) {
                        let p = wa[0] + (wa[1] - wa[0]) * t;
                        add(p, i, sa + la * t, j, sb + lb * u);
                    }
                    sb += lb;
                }
                sa += la;
            }
            // An end of one on the other (inside its width), or both ends
            // together.
            for (x, y, xi, yi) in [(a, b, i, j), (b, a, j, i)] {
                let lx = length(&x.axis);
                for (end, sx) in [(x.axis[0], 0.0), (x.axis[x.axis.len() - 1], lx)] {
                    let (sy, d) = nearest_on(y, end);
                    let (yl, yr) = y.extent_at(sy);
                    if d <= yl.max(yr) + 0.5 && d > 1e-6 {
                        // Its distance as if its axis went on to the
                        // other's axis.
                        let (p, _) = along(&y.axis, sy);
                        let sx = if sx == 0.0 { -d } else { sx + d };
                        add(p, xi, sx, yi, sy);
                    }
                }
            }
        }
    }
    found.retain(|(_, rs)| rs.len() >= 2);
    found
}

/// How far a road's carriageway (lanes, parking, bike lanes) reaches to
/// each side of its axis (left, right), the kerbs beside it and its whole
/// half-widths (left, right).
fn reach(r: &Road, at: f64) -> (f64, f64, f64, f64, f64, f64) {
    let offs = r.offsets_at(at);
    let traffic: Vec<&(RoadBand, f64, f64)> = offs.iter().filter(|o| o.0.traffic()).collect();
    let (el, er) = r.extent_at(at);
    let (left, right) = if traffic.is_empty() {
        (el, er)
    } else {
        (
            traffic
                .iter()
                .map(|o| o.2)
                .fold(f64::MIN, f64::max)
                .max(0.0),
            (-traffic.iter().map(|o| o.1).fold(f64::MAX, f64::min)).max(0.0),
        )
    };
    let kerb_beside = |edge: f64| {
        offs.iter()
            .find(|o| {
                o.0 == RoadBand::Kerb && ((o.1 - edge).abs() < 1e-6 || (o.2 - edge).abs() < 1e-6)
            })
            .map_or(0.0, |o| o.2 - o.1)
    };
    let kl = kerb_beside(left).max(if left < el { 0.12 } else { 0.0 });
    let kr = kerb_beside(-right).max(if right < er { 0.12 } else { 0.0 });
    (left, right, kl.min(el - left), kr.min(er - right), el, er)
}

/// A corner between two arms next to each other: the kerb line from the
/// cut of the first (on its left) to the cut of the second (on its right),
/// and where the sidewalk's outer edges meet. `a`'s cut and `b`'s are
/// worked out first.
struct Corner {
    /// Where the corner's kerb line leaves each arm, measured out along
    /// it (the tangents of the arc).
    t_a: f64,
    t_b: f64,
    /// The kerb line from the tangent on `a` to the tangent on `b`.
    line: Vec<Point2>,
    /// Where the outer edges meet (none: straight on).
    outer: Option<Point2>,
    /// The sidewalk's outer edge along the whole corner, when it follows
    /// the kerb line (round a roundabout) instead of meeting at `outer`.
    outer_line: Option<Vec<Point2>>,
    /// The centre of its arc (a bend's outside corner turns round it).
    centre: Option<Point2>,
}

/// `bump_a` and `bump_b`: how far the corner's sidewalk is brought out over
/// `a`'s and `b`'s carriageways (a bump-out, N1).
fn corner(a: &Arm, b: &Arm, radius: f64, shape: CornerShape, bump_a: f64, bump_b: f64) -> Corner {
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let (xa, xb) = (a.origin, b.origin);
    let pa = xa + na * (a.kerb_left - bump_a);
    let pb = xb - nb * (b.kerb_right - bump_b);
    let theta = {
        let t = b.dir.y.atan2(b.dir.x) - a.dir.y.atan2(a.dir.x);
        t.rem_euclid(std::f64::consts::TAU)
    };
    let outer = meet(xa + na * a.edge_left, a.dir, xb - nb * b.edge_right, b.dir);
    if (theta - std::f64::consts::PI).abs() < STRAIGHT {
        return Corner {
            t_a: 0.0,
            t_b: 0.0,
            line: Vec::new(),
            outer: None,
            outer_line: None,
            centre: None,
        };
    }
    let Some(c) = meet(pa, a.dir, pb, b.dir) else {
        return Corner {
            t_a: 0.0,
            t_b: 0.0,
            line: Vec::new(),
            outer: None,
            outer_line: None,
            centre: None,
        };
    };
    if theta > std::f64::consts::PI || shape == CornerShape::Sharp || radius <= 1e-6 {
        // The outside of a bend, or a corner without rounding: the kerbs
        // meet in a point.
        return Corner {
            t_a: (c - xa).dot(a.dir),
            t_b: (c - xb).dot(b.dir),
            line: vec![c],
            outer,
            outer_line: None,
            centre: None,
        };
    }
    let d = radius / (theta / 2.0).tan();
    let (ta, tb) = (c + a.dir * d, c + b.dir * d);
    let bis = (a.dir + b.dir).normalized().unwrap_or(na);
    let centre = c + bis * (radius / (theta / 2.0).sin());
    let (sa, sb) = (ta - centre, tb - centre);
    let (a0, mut a1) = (sa.y.atan2(sa.x), sb.y.atan2(sb.x));
    // The short way round, which passes nearest the corner point.
    let mut sweep = a1 - a0;
    while sweep > std::f64::consts::PI {
        sweep -= std::f64::consts::TAU;
    }
    while sweep < -std::f64::consts::PI {
        sweep += std::f64::consts::TAU;
    }
    a1 = a0 + sweep;
    if shape == CornerShape::Chamfer {
        // Straight across, from where the arc would start to where it
        // would end.
        return Corner {
            t_a: (ta - xa).dot(a.dir),
            t_b: (tb - xb).dot(b.dir),
            line: vec![ta, tb],
            outer,
            outer_line: None,
            centre: None,
        };
    }
    let line = (0..=ARC_STEPS)
        .map(|k| {
            let t = a0 + (a1 - a0) * k as f64 / ARC_STEPS as f64;
            centre + Point2::new(t.cos(), t.sin()) * radius
        })
        .collect();
    Corner {
        t_a: (ta - xa).dot(a.dir),
        t_b: (tb - xb).dot(b.dir),
        line,
        outer,
        outer_line: None,
        centre: Some(centre),
    }
}

/// An arc round `centre` from `p` to `q`, the short way (or, `ccw`, always
/// counter-clockwise), with its ends.
fn arc(centre: Point2, p: Point2, q: Point2, ccw: bool) -> Vec<Point2> {
    let (sp, sq) = (p - centre, q - centre);
    let r = sp.length();
    let a0 = sp.y.atan2(sp.x);
    let mut sweep = sq.y.atan2(sq.x) - a0;
    if ccw {
        sweep = sweep.rem_euclid(std::f64::consts::TAU);
    } else {
        while sweep > std::f64::consts::PI {
            sweep -= std::f64::consts::TAU;
        }
        while sweep < -std::f64::consts::PI {
            sweep += std::f64::consts::TAU;
        }
    }
    let steps = ((sweep.abs() / (std::f64::consts::PI / 24.0)).ceil() as usize).max(ARC_STEPS);
    (0..=steps)
        .map(|k| {
            let t = a0 + sweep * k as f64 / steps as f64;
            centre + Point2::new(t.cos(), t.sin()) * r
        })
        .collect()
}

/// Where a kerb line (from `from` along `dir`) is joined to a roundabout's
/// outer circle by a fillet of `radius` on the side `side` (its normal
/// towards the sidewalk): how far out along the line the fillet starts,
/// and the fillet from the line to the circle.
fn ring_fillet(
    from: Point2,
    dir: Vec2,
    side: Vec2,
    centre: Point2,
    ring: f64,
    radius: f64,
) -> (f64, Vec<Point2>) {
    let r = radius.max(0.5);
    let p0 = from + side * r - centre;
    let b = p0.dot(dir);
    let disc = b * b - p0.dot(p0) + (ring + r) * (ring + r);
    if disc < 0.0 {
        // The line misses the circle: it simply reaches its radius.
        return (ring, vec![from + dir * ring]);
    }
    let t = -b + disc.sqrt();
    let f = from + side * r + dir * t;
    let on_line = from + dir * t;
    let on_circle = centre + (f - centre) * (ring / (ring + r));
    (t, arc(f, on_line, on_circle, false))
}

/// A corner between two arms of a roundabout: from `a`'s kerb line,
/// round the outer circle counter-clockwise, to `b`'s.
fn ring_corner(a: &Arm, b: &Arm, centre: Point2, ring: f64, ra: f64, rb: f64, walk: f64) -> Corner {
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let (ta, fa) = ring_fillet(a.origin + na * a.kerb_left, a.dir, na, centre, ring, ra);
    let (tb, fb) = ring_fillet(
        b.origin - nb * b.kerb_right,
        b.dir,
        nb * -1.0,
        centre,
        ring,
        rb,
    );
    let mut line = fa.clone();
    let (ca, cb) = (fa[fa.len() - 1], fb[fb.len() - 1]);
    let round = arc(centre, ca, cb, true);
    line.extend(round.iter().skip(1));
    line.extend(fb.iter().rev().skip(1));
    line.dedup_by(|p, q| p.distance(*q) < 1e-6);
    // The sidewalk round it, as wide as the narrower of the two.
    let outer_line = offset_path(&line, -walk);
    Corner {
        t_a: ta,
        t_b: tb,
        line,
        outer: None,
        outer_line,
        centre: Some(centre),
    }
}

/// An arm of road `r` from `origin`, at `s` along it, going on along it
/// (`forward`) or back.
fn arm_of(r: &Road, origin: Point2, s: f64, forward: bool) -> Arm {
    let len = length(&r.axis);
    let (_, u) = along(&r.axis, s.clamp(0.0, len));
    let (kl, kr, wl, wr, hl, hr) = reach(r, s.clamp(0.0, len));
    // The lanes, without the parking and the bike lanes.
    let (ll, lr) = {
        let offs = r.offsets_at(s.clamp(0.0, len));
        let lanes: Vec<&(RoadBand, f64, f64)> = offs
            .iter()
            .filter(|o| {
                matches!(
                    o.0,
                    RoadBand::Carriageway | RoadBand::BusLane | RoadBand::Tram
                )
            })
            .collect();
        if lanes.is_empty() {
            (kl, kr)
        } else {
            (
                lanes.iter().map(|o| o.2).fold(f64::MIN, f64::max).max(0.0),
                (-lanes.iter().map(|o| o.1).fold(f64::MAX, f64::min)).max(0.0),
            )
        }
    };
    // Looking back along the road, its left is its right.
    let (dir, kerb_left, kerb_right, kw_l, kw_r, e_l, e_r, lanes_left, lanes_right) = if forward {
        (u, kl, kr, wl, wr, hl, hr, ll, lr)
    } else {
        (u * -1.0, kr, kl, wr, wl, hr, hl, lr, ll)
    };
    Arm {
        road: r.id,
        origin,
        station: s,
        forward,
        dir,
        kerb_left,
        kerb_right,
        kerb_width_left: kw_l,
        kerb_width_right: kw_r,
        edge_left: e_l,
        edge_right: e_r,
        lanes_left,
        lanes_right,
        cut: 0.5,
    }
}

/// The arms of the roads that reach a roundabout: those whose axis goes
/// through its circle (two arms) or ends in it or near it (one, measured
/// from where its axis would pass by the centre).
fn ring_arms(roads: &[&Road], c: &Roundabout) -> Vec<Arm> {
    let mut arms = Vec::new();
    for r in roads {
        let len = length(&r.axis);
        let (s, d) = nearest_on(r, c.centre);
        let reach = c.radius + 3.0;
        let at_end = s <= 1e-6 || s >= len - 1e-6;
        if !at_end {
            if d >= c.radius {
                continue;
            }
            let (o, _) = along(&r.axis, s);
            for forward in [true, false] {
                let room = if forward { len - s } else { s };
                if room > c.radius + 1.0 {
                    arms.push(arm_of(r, o, s, forward));
                }
            }
            continue;
        }
        // An end: where its line passes by the centre.
        let forward = s <= 1e-6;
        let (end, _) = along(&r.axis, s);
        if end.distance(c.centre) > reach {
            continue;
        }
        let (_, u) = along(&r.axis, if forward { 0.0 } else { len });
        let out = if forward { u } else { u * -1.0 };
        let back = (c.centre - end).dot(out);
        if back > 0.0 {
            continue;
        }
        let o = end + out * back;
        let station = if forward { back } else { len - back };
        if len > 1.0 {
            arms.push(arm_of(r, o, station, forward));
        }
    }
    arms.sort_by(|a, b| a.dir.y.atan2(a.dir.x).total_cmp(&b.dir.y.atan2(b.dir.x)));
    arms
}

/// Streets that cross with their tops further apart than this pass one
/// over the other, without a crossing, metres.
pub const OVERPASS: f64 = 3.5;

/// The junctions of the roads of a level: at its roundabouts and squares
/// and wherever roads meet away from them (but where one passes over the
/// other, `OVERPASS`).
///
/// They depend only on the level's roads and roundabouts, and the plan,
/// the 3D, the IFC and the tools ask for them many times between two
/// changes: the last few are kept with what they came from
/// ([`shared_junctions`]) and given back while those are the same.
pub fn junctions(model: &Model, level: LevelId) -> Vec<Junction> {
    shared_junctions(model, level).as_ref().clone()
}

/// What some junctions were worked out from, and them.
struct Kept {
    roads: Vec<Road>,
    rings: Vec<Roundabout>,
    nodes: Vec<NodeSettings>,
    junctions: Arc<Vec<Junction>>,
}

/// The junctions kept: one per level or so of the last models asked
/// about (the plan of one level and the 3D of all of them).
const KEPT: usize = 8;
static KEPT_JUNCTIONS: Mutex<Vec<Kept>> = Mutex::new(Vec::new());

/// [`junctions`] without copying them: the same ones while the level's
/// roads and roundabouts stay the same.
pub fn shared_junctions(model: &Model, level: LevelId) -> Arc<Vec<Junction>> {
    let mut roads: Vec<&Road> = model.roads_on_level(level).collect();
    roads.sort_by_key(|r| r.id.0);
    let mut rings: Vec<&Roundabout> = model.roundabouts_on_level(level).collect();
    rings.sort_by_key(|r| r.id.0);
    let nodes: Vec<&NodeSettings> = model.nodes().filter(|n| n.key.level == level).collect();
    let same = |k: &Kept| {
        k.nodes.len() == nodes.len()
            && k.nodes.iter().zip(&nodes).all(|(a, b)| a == *b)
            && k.roads.len() == roads.len()
            && k.rings.len() == rings.len()
            && k.roads.iter().zip(&roads).all(|(a, b)| a == *b)
            && k.rings.iter().zip(&rings).all(|(a, b)| a == *b)
    };
    if let Ok(mut kept) = KEPT_JUNCTIONS.lock() {
        if let Some(i) = kept.iter().position(same) {
            // The most recent last.
            let k = kept.remove(i);
            let out = k.junctions.clone();
            kept.push(k);
            return out;
        }
    }
    let out = Arc::new(work_out_junctions(model, &roads, &rings));
    if let Ok(mut kept) = KEPT_JUNCTIONS.lock() {
        if kept.len() >= KEPT {
            kept.remove(0);
        }
        kept.push(Kept {
            roads: roads.iter().map(|r| (*r).clone()).collect(),
            rings: rings.iter().map(|r| (*r).clone()).collect(),
            nodes: nodes.iter().map(|n| (*n).clone()).collect(),
            junctions: out.clone(),
        });
    }
    out
}

/// The junctions of these roads and roundabouts (of one level, roads
/// sorted by their ids).
/// A meeting point: where, which roads at which stations, its node's key
/// and the station along the first road (to number the same nodes).
type Meeting = (Point2, Vec<(usize, f64)>, NodeKey, f64);

fn work_out_junctions(model: &Model, roads: &[&Road], rings: &[&Roundabout]) -> Vec<Junction> {
    let roads = roads.to_vec();
    let mut out = Vec::new();
    for c in rings {
        let arms = ring_arms(&roads, c);
        if let Some(j) = assemble(model, &roads, arms, c.centre, Some(c), None) {
            out.push(j);
        }
    }
    // The meeting points and their nodes' keys: the same streets meeting
    // the same way, numbered along the first of them.
    let mut found: Vec<Meeting> = Vec::new();
    let Some(level) = roads.first().map(|r| r.level) else {
        return out;
    };
    let points = meeting_points(&roads);
    // The dead ends with a turning circle (N1).
    // (Only worked out when a node of one or two streets is changed.)
    let any_loose = model
        .nodes()
        .any(|n| n.key.level == level && n.key.roads.len() <= 2);
    let (ends, joined) = if any_loose {
        loose_ends(&roads, rings, &points)
    } else {
        (Vec::new(), Vec::new())
    };
    for (key, at, i, s) in ends {
        let Some(n) = model.node(&key) else { continue };
        if n.kind() != NodeKind::DeadEnd {
            continue;
        }
        if let Some(mut j) = dead_end(model, &roads, roads[i], at, s, n) {
            j.key = Some(key);
            out.push(j);
        }
    }
    // Ends together: a node only when it is changed by hand (else the
    // streets simply go on one into the other, as drawn).
    for (key, at, met) in joined {
        let Some(n) = model.node(&key) else { continue };
        if n.kind() == NodeKind::Open {
            continue;
        }
        let arms = arms_at(&roads, at, &met);
        if arms.len() < 2 {
            continue;
        }
        if let Some(mut j) = assemble(model, &roads, arms, at, None, Some(n)) {
            j.key = Some(key);
            out.push(j);
        }
    }
    for (x, met) in points {
        if rings.iter().any(|c| c.centre.distance(x) < c.radius + 1.0) {
            continue;
        }
        let ends = met_ends(&roads, &met);
        let first = ends[0].0;
        let along = met
            .iter()
            .find(|&&(i, _)| roads[i].id == first)
            .map_or(0.0, |m| m.1);
        found.push((
            x,
            met,
            NodeKey {
                level,
                roads: ends,
                nth: 0,
            },
            along,
        ));
    }
    let mut order: Vec<usize> = (0..found.len()).collect();
    order.sort_by(|&a, &b| {
        found[a]
            .2
            .roads
            .cmp(&found[b].2.roads)
            .then(found[a].3.total_cmp(&found[b].3))
    });
    for w in 1..order.len() {
        let (p, q) = (order[w - 1], order[w]);
        if found[p].2.roads == found[q].2.roads {
            found[q].2.nth = found[p].2.nth + 1;
        }
    }
    for (x, met, key, _) in found {
        let settings = model.node(&key);
        // No node by hand: the streets pass without one.
        if settings.is_some_and(|n| n.kind() == NodeKind::Open) {
            continue;
        }
        // One passes over the other (a bridge): no crossing.
        let heights: Vec<f64> = met.iter().map(|&(i, s)| roads[i].top_at(s)).collect();
        let (lo, hi) = heights
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(*z), b.max(*z)));
        if hi - lo > OVERPASS {
            continue;
        }
        let arms = arms_at(&roads, x, &met);
        if arms.len() < 2 || arms.iter().all(|a| a.road == arms[0].road) {
            continue;
        }
        if let Some(mut j) = assemble(model, &roads, arms, x, None, settings) {
            j.key = Some(key);
            out.push(j);
        }
    }
    out
}

/// How each road reaches a meeting point, sorted (the node's key).
fn met_ends(roads: &[&Road], met: &[(usize, f64)]) -> Vec<(RoadId, NodeEnd)> {
    let mut ends: Vec<(RoadId, NodeEnd)> = met
        .iter()
        .map(|&(i, s)| {
            let len = length(&roads[i].axis);
            let end = if s <= SAME {
                NodeEnd::Start
            } else if s >= len - SAME {
                NodeEnd::End
            } else {
                NodeEnd::Through
            };
            (roads[i].id, end)
        })
        .collect();
    ends.sort();
    ends
}

/// The ends of the roads that meet nothing (no other road, no
/// roundabout): their node's key, where, which road and how far along it.
/// Two such ends together (a street that goes on in line into another) are
/// one node of both, given as the second list: its key, where, and the
/// roads with their stations.
type FreeEnd = (NodeKey, Point2, usize, f64);
type Joint = (NodeKey, Point2, Vec<(usize, f64)>);

fn loose_ends(
    roads: &[&Road],
    rings: &[&Roundabout],
    points: &[(Point2, Vec<(usize, f64)>)],
) -> (Vec<FreeEnd>, Vec<Joint>) {
    let mut ends: Vec<FreeEnd> = Vec::new();
    for (i, r) in roads.iter().enumerate() {
        let len = length(&r.axis);
        if r.axis.len() < 2 || len < 2.0 * SAME {
            continue;
        }
        for (end, s, at) in [
            (NodeEnd::Start, 0.0, r.axis[0]),
            (NodeEnd::End, len, r.axis[r.axis.len() - 1]),
        ] {
            let met = points.iter().any(|(_, m)| {
                m.iter().any(|&(k, st)| {
                    k == i
                        && match end {
                            NodeEnd::Start => st <= SAME,
                            _ => st >= len - SAME,
                        }
                })
            });
            let ringed = rings
                .iter()
                .any(|c| c.centre.distance(at) < c.radius + r.widest() + 1.0);
            if met || ringed {
                continue;
            }
            ends.push((
                NodeKey {
                    level: r.level,
                    roads: vec![(r.id, end)],
                    nth: 0,
                },
                at,
                i,
                s,
            ));
        }
    }
    // Ends together.
    let mut joints: Vec<Joint> = Vec::new();
    let mut taken = vec![false; ends.len()];
    for a in 0..ends.len() {
        if taken[a] {
            continue;
        }
        let together: Vec<usize> = (a..ends.len())
            .filter(|&b| !taken[b] && ends[b].1.distance(ends[a].1) < SAME)
            .collect();
        if together.len() < 2 || together.iter().all(|&b| ends[b].2 == ends[a].2) {
            continue;
        }
        let mut roads_at: Vec<(RoadId, NodeEnd)> = Vec::new();
        let mut met = Vec::new();
        for &b in &together {
            taken[b] = true;
            roads_at.extend(ends[b].0.roads.iter().copied());
            met.push((ends[b].2, ends[b].3));
        }
        roads_at.sort();
        joints.push((
            NodeKey {
                level: ends[a].0.level,
                roads: roads_at,
                nth: 0,
            },
            ends[a].1,
            met,
        ));
    }
    let singles = ends
        .into_iter()
        .zip(taken)
        .filter(|(_, t)| !t)
        .map(|(e, _)| e)
        .collect();
    (singles, joints)
}

/// The arms of the roads that meet at `x` (sorted round it).
fn arms_at(roads: &[&Road], x: Point2, met: &[(usize, f64)]) -> Vec<Arm> {
    let mut arms: Vec<Arm> = Vec::new();
    for &(i, s) in met {
        let r = roads[i];
        let len = length(&r.axis);
        for forward in [true, false] {
            let room = if forward { len - s } else { s };
            if room <= SAME {
                continue;
            }
            arms.push(arm_of(r, x, s, forward));
        }
    }
    arms.sort_by(|a, b| a.dir.y.atan2(a.dir.x).total_cmp(&b.dir.y.atan2(b.dir.x)));
    arms
}

/// The free ends of a level's roads and the ends together (N1): their key
/// and where they are.
/// Nodes and where they are.
type Placed = Vec<(NodeKey, Point2)>;

fn level_loose_ends(model: &Model, level: LevelId) -> (Placed, Placed) {
    let mut roads: Vec<&Road> = model.roads_on_level(level).collect();
    roads.sort_by_key(|r| r.id.0);
    let rings: Vec<&Roundabout> = model.roundabouts_on_level(level).collect();
    let points = meeting_points(&roads);
    let (singles, joints) = loose_ends(&roads, &rings, &points);
    (
        singles.into_iter().map(|(k, p, _, _)| (k, p)).collect(),
        joints.into_iter().map(|(k, p, _)| (k, p)).collect(),
    )
}

/// The free ends of a level's roads (N1): their key and where they are.
pub fn dead_ends(model: &Model, level: LevelId) -> Placed {
    level_loose_ends(model, level).0
}

/// The ends of a level's roads that are together without making a junction
/// (a street that goes on in line into another, N1): their key and where.
pub fn joints(model: &Model, level: LevelId) -> Placed {
    level_loose_ends(model, level).1
}

/// The usual radius of a dead end's turning circle (its kerb), metres.
pub const TURNING_RADIUS: f64 = 9.0;

/// A dead end with a turning circle round the end of the road's axis: the
/// road's kerbs go round the circle as round a roundabout.
fn dead_end(
    model: &Model,
    roads: &[&Road],
    r: &Road,
    at: Point2,
    s: f64,
    n: &NodeSettings,
) -> Option<Junction> {
    let len = length(&r.axis);
    let forward = s < len / 2.0;
    let arm = arm_of(r, at, s, forward);
    let radius = n
        .end_radius
        .unwrap_or(TURNING_RADIUS)
        .max(arm.kerb_left.max(arm.kerb_right) + 0.5);
    // The circle is centred on the axis, a little back from its end so
    // that the carriageway's end falls inside it.
    let ring = Roundabout {
        id: RoundaboutId(0),
        guid: GlobalId::from_u128(0),
        name: String::new(),
        level: r.level,
        centre: at,
        radius,
        island: 0.0,
        apron: 0.0,
        top_offset: r.top_at(s),
        plaza: false,
        island_finish: IslandFinish::Paved,
        pavement: r.pavement,
        kerb_material: r.kerb_material,
        island_material: r.kerb_material,
    };
    let mut j = assemble(model, roads, vec![arm], at, Some(&ring), Some(n))?;
    j.ring = None;
    j.kind = NodeKind::DeadEnd;
    j.flat = true;
    j.top = r.top_at(s);
    Some(j)
}

/// A model whose nodes made [`NodeKind::Open`] on `level` are plain
/// crossings: to find where they are.
fn opened(model: &Model, level: LevelId) -> Option<Model> {
    let open: Vec<NodeKey> = model
        .nodes()
        .filter(|n| n.key.level == level && n.kind() == NodeKind::Open)
        .map(|n| n.key.clone())
        .collect();
    if open.is_empty() {
        return None;
    }
    let mut plain = model.clone();
    for k in open {
        if let Some(mut n) = plain.node(&k).cloned() {
            n.kind = None;
            let _ = plain.set_node(n);
        }
    }
    Some(plain)
}

/// The junction of a node, also if it is made [`NodeKind::Open`] (then as
/// a plain crossing, to show where it is).
pub fn node_junction(model: &Model, key: &NodeKey) -> Option<Junction> {
    if let Some(j) = shared_junctions(model, key.level)
        .iter()
        .find(|j| j.key.as_ref() == Some(key))
    {
        return Some(j.clone());
    }
    let plain = opened(model, key.level)?;
    let found = shared_junctions(&plain, key.level)
        .iter()
        .find(|j| j.key.as_ref() == Some(key))
        .cloned();
    found
}

/// The node at `p` of a level (N1): the one whose carriageway it is on, or
/// the nearest within `reach`; also those made [`NodeKind::Open`].
pub fn node_at(model: &Model, level: LevelId, p: Point2, reach: f64) -> Option<Junction> {
    let pick = |js: &[Junction]| -> Option<Junction> {
        let inside = js
            .iter()
            .filter(|j| j.key.is_some())
            .find(|j| archisim_core::point_in_polygon(p, &j.plate));
        if let Some(j) = inside {
            return Some(j.clone());
        }
        js.iter()
            .filter(|j| j.key.is_some() && j.at.distance(p) < reach)
            .min_by(|a, b| a.at.distance(p).total_cmp(&b.at.distance(p)))
            .cloned()
    };
    pick(&shared_junctions(model, level)).or_else(|| {
        let plain = opened(model, level)?;
        let js = shared_junctions(&plain, level);
        pick(&js).filter(|j| {
            j.key
                .as_ref()
                .and_then(|k| model.node(k))
                .is_some_and(|n| n.kind() == NodeKind::Open)
        })
    })
}

/// The node at `p` of a level (N1), also a street's free end (a dead end,
/// which has no junction while it has no turning circle).
pub fn node_key_at(model: &Model, level: LevelId, p: Point2, reach: f64) -> Option<NodeKey> {
    if let Some(k) = node_at(model, level, p, reach).and_then(|j| j.key) {
        return Some(k);
    }
    let (ends, joined) = level_loose_ends(model, level);
    ends.into_iter()
        .chain(joined)
        .filter(|(_, q)| q.distance(p) < reach)
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
        .map(|(k, _)| k)
}

/// The arms of a node, also of a dead end without its turning circle:
/// where the street goes out of it.
pub fn node_arms(model: &Model, key: &NodeKey) -> Vec<Arm> {
    if let Some(j) = node_junction(model, key) {
        return j.arms;
    }
    let mut arms = Vec::new();
    for &(id, end) in &key.roads {
        let Some(r) = model.road(id) else {
            return Vec::new();
        };
        let len = length(&r.axis);
        match end {
            NodeEnd::Start => arms.push(arm_of(r, r.axis[0], 0.0, true)),
            NodeEnd::End => arms.push(arm_of(r, r.axis[r.axis.len() - 1], len, false)),
            NodeEnd::Through => return Vec::new(),
        }
    }
    arms.sort_by(|a, b| a.dir.y.atan2(a.dir.x).total_cmp(&b.dir.y.atan2(b.dir.x)));
    arms
}

/// The keys of the nodes of a level (N1): what the streets make now.
pub fn node_keys(model: &Model, level: LevelId) -> Vec<NodeKey> {
    shared_junctions(model, level)
        .iter()
        .filter_map(|j| j.key.clone())
        .collect()
}

/// The keys of every node, also those made [`NodeKind::Open`] (which have
/// no junction): to forget only the settings of the nodes that are gone.
pub fn all_node_keys(model: &Model) -> Vec<NodeKey> {
    let mut keys = Vec::new();
    for l in model.levels() {
        keys.extend(node_keys(model, l.id));
    }
    // The dead ends and the ends together, while they are there.
    for l in model.levels() {
        let (ends, joined) = level_loose_ends(model, l.id);
        for (k, _) in ends.into_iter().chain(joined) {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
    }
    // The open ones are there while their streets still meet: their key is
    // worked out as if they were plain.
    for l in model.levels() {
        if let Some(plain) = opened(model, l.id) {
            for k in node_keys(&plain, l.id) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
    }
    keys
}

/// A junction at `x` of `arms` (sorted round it), round a roundabout's
/// circle if `ring` (or a dead end's turning circle).
fn assemble(
    model: &Model,
    roads: &[&Road],
    mut arms: Vec<Arm>,
    x: Point2,
    ring: Option<&Roundabout>,
    settings: Option<&NodeSettings>,
) -> Option<Junction> {
    if arms.is_empty() {
        return None;
    }
    let n = arms.len();
    let mut kind = settings.map_or(NodeKind::Crossing, |s| s.kind());
    if !kind.fits(n) {
        kind = NodeKind::Crossing;
    }
    let raised = matches!(kind, NodeKind::Raised | NodeKind::Shared) && ring.is_none();
    let marks_hidden = settings.and_then(|s| s.hide_marks).unwrap_or(false);
    // The main street, if it is one of the node's (N1); else the first.
    let main = settings
        .and_then(|s| s.main)
        .filter(|m| arms.iter().any(|a| a.road == *m));
    let owner = match main.and_then(|m| roads.iter().find(|r| r.id == m)) {
        Some(r) => r,
        None => roads.iter().find(|r| arms.iter().any(|a| a.road == r.id))?,
    };
    let radius_of = |id: RoadId| {
        model
            .road(id)
            .map_or(4.0, |r| r.section.corner_radius)
            .max(0.0)
    };
    // The sidewalk round a roundabout: as wide as the narrowest of its
    // roads' sidewalks.
    let walk = arms
        .iter()
        .map(|a| {
            (a.edge_left - a.kerb_left - a.kerb_width_left)
                .min(a.edge_right - a.kerb_right - a.kerb_width_right)
        })
        .fold(f64::INFINITY, f64::min)
        .max(0.0);
    let kw_ring = arms
        .iter()
        .map(|a| a.kerb_width_left.max(a.kerb_width_right))
        .fold(0.0, f64::max);
    let corner_of = |a: &Arm| {
        settings
            .map(|s| s.corner(Junction::arm_ref(a)))
            .unwrap_or_default()
    };
    // The bump-outs (N1): of each arm's left side (its corner's) and right
    // side (the corner before's), leaving at least a lane.
    let bump = |a: &Arm, kerb: f64| -> f64 {
        if ring.is_some() {
            return 0.0;
        }
        corner_of(a)
            .bump
            .unwrap_or(0.0)
            .clamp(0.0, (kerb - LANE_KEPT).max(0.0))
    };
    let bump_left: Vec<f64> = arms.iter().map(|a| bump(a, a.kerb_left)).collect();
    let bump_right: Vec<f64> = (0..n)
        .map(|i| {
            let prev = &arms[(i + n - 1) % n];
            bump(prev, arms[i].kerb_right)
        })
        .collect();
    let make = |arms: &[Arm]| -> Vec<Corner> {
        let mut corners: Vec<Corner> = (0..n)
            .map(|i| {
                let (a, b) = (&arms[i], &arms[(i + 1) % n]);
                match ring {
                    Some(c) => ring_corner(
                        a,
                        b,
                        c.centre,
                        c.radius,
                        radius_of(a.road),
                        radius_of(b.road),
                        kw_ring + walk,
                    ),
                    None if kind == NodeKind::Continuous => straight_corner(),
                    None => {
                        let c = corner_of(a);
                        let r = c
                            .radius
                            .unwrap_or_else(|| radius_of(a.road).min(radius_of(b.road)));
                        corner(
                            a,
                            b,
                            r,
                            c.shape.unwrap_or_default(),
                            bump_left[i],
                            bump_right[(i + 1) % n],
                        )
                    }
                }
            })
            .collect();
        // A bend: the outside corner round the inside one's centre.
        if kind == NodeKind::Bend && n == 2 {
            for i in 0..2 {
                let (a, b) = (&arms[i], &arms[1 - i]);
                let theta = (b.dir.y.atan2(b.dir.x) - a.dir.y.atan2(a.dir.x))
                    .rem_euclid(std::f64::consts::TAU);
                if theta <= std::f64::consts::PI {
                    continue;
                }
                if let Some(c) = corners[1 - i].centre {
                    corners[i] = bend_outside(a, b, c);
                }
            }
        }
        corners
    };
    // The corners first, which set where the arms are cut: the cut of an
    // arm reaches past both its corners and past the other roads'
    // carriageways.
    let corners = make(&arms);
    for i in 0..n {
        let prev = &corners[(i + n - 1) % n];
        let next = &corners[i];
        let wide = if ring.is_some() || matches!(kind, NodeKind::Bend | NodeKind::Continuous) {
            0.0
        } else {
            arms.iter()
                .filter(|o| o.road != arms[i].road)
                .map(|o| o.kerb_left.max(o.kerb_right) + o.kerb_width_left.max(o.kerb_width_right))
                .fold(0.0, f64::max)
        };
        let mut cut = next.t_a.max(prev.t_b).max(wide).max(0.5);
        if kind == NodeKind::Continuous {
            // The stretch over which the section changes.
            cut = cut.max(TRANSITION);
        }
        // Room for the ramp of a raised node past its corners.
        if raised {
            cut = cut.max(next.t_a.max(prev.t_b).max(0.0) + RAMP);
        }
        // Its setback, by hand.
        if let Some(b) = settings.and_then(|s| s.arm(Junction::arm_ref(&arms[i])).setback) {
            cut = cut.max(b);
        }
        arms[i].cut = cut;
    }
    // Again, now that the cuts are known (the kerb line runs to them).
    let corners = make(&arms);
    let mut plate = Vec::new();
    let mut kerbs = Vec::new();
    let mut sidewalks = Vec::new();
    let mut bollards: Vec<Point2> = Vec::new();
    let mut ramps: Vec<Vec<Point2>> = Vec::new();
    let mut bump_kerbs: Vec<Vec<Point2>> = Vec::new();
    let kerb_height = arms
        .iter()
        .filter_map(|a| model.road(a.road))
        .map(|r| {
            let s = &r.section;
            if s.single_level {
                0.0
            } else {
                s.kerb_height
            }
        })
        .fold(f64::MAX, f64::min);
    // How far its carriageway rises (a raised node: to the sidewalk).
    let lift = if raised {
        owner.section.kerb_height.max(kerb_height)
    } else {
        0.0
    };
    let left_at = |i: usize, s: f64| {
        let a = &arms[i];
        a.origin + a.dir * s + a.dir.perp_left() * (a.kerb_left - bump_left[i])
    };
    let right_at = |i: usize, s: f64| {
        let a = &arms[i];
        a.origin + a.dir * s - a.dir.perp_left() * (a.kerb_right - bump_right[i])
    };
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (&arms[i], &arms[j]);
        let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
        let right_cut = right_at(i, a.cut);
        let left_cut = left_at(i, a.cut);
        if lift > 0.0 {
            // The plateau ends a ramp's length back; the ramp down to the
            // street's carriageway.
            let (right_in, left_in) = (right_at(i, a.cut - RAMP), left_at(i, a.cut - RAMP));
            plate.push(right_in);
            plate.push(left_in);
            ramps.push(vec![right_in, right_cut, left_cut, left_in]);
        } else {
            plate.push(right_cut);
            plate.push(left_cut);
        }
        // The kerb across the parking where a bump-out ends (N1).
        let kw_a = a.kerb_width_left;
        if bump_left[i] > 1e-6 && kw_a > 1e-6 && kind != NodeKind::Shared {
            let full = a.origin + a.dir * a.cut + na * a.kerb_left;
            bump_kerbs.push(vec![
                left_cut,
                full,
                full + a.dir * kw_a,
                left_cut + a.dir * kw_a,
            ]);
        }
        let kw_b = b.kerb_width_right;
        if bump_right[j] > 1e-6 && kw_b > 1e-6 && kind != NodeKind::Shared {
            let full = b.origin + b.dir * b.cut - nb * b.kerb_right;
            let bumped = right_at(j, b.cut);
            bump_kerbs.push(vec![
                full,
                bumped,
                bumped + b.dir * kw_b,
                full + b.dir * kw_b,
            ]);
        }
        // The kerb line of the corner, from the cut of `a` to the cut of
        // `b`.
        let mut line = vec![left_cut];
        line.extend(corners[i].line.iter().copied());
        line.push(right_at(j, b.cut));
        line.dedup_by(|p, q| p.distance(*q) < 1e-6);
        plate.extend(
            line.iter()
                .skip(1)
                .take(line.len().saturating_sub(2))
                .copied(),
        );
        if line.len() < 2 {
            continue;
        }
        // The kerb and the sidewalk lie to the right of that line.
        let kw = if ring.is_some() {
            kw_ring
        } else {
            a.kerb_width_left.max(b.kerb_width_right)
        };
        if kw > 1e-6 && kind != NodeKind::Shared {
            let k = strip(&line, -kw, 0.0);
            if k.len() >= 3 {
                kerbs.push(k);
            }
        }
        // A shared surface's bollards, along the kerb line.
        if kind == NodeKind::Shared {
            bollards.extend(bollards_along(&line));
        }
        // No sidewalk at this corner, by hand.
        let walk_here = corner_of(a).sidewalk.unwrap_or(true);
        if !walk_here {
            continue;
        }
        let Some(inner) = offset_path(&line, -kw) else {
            continue;
        };
        let mut side = inner;
        match &corners[i].outer_line {
            Some(outer) => {
                // Round a roundabout or a bend: out to the roads' outer
                // edges at their cuts, along the kerb line's offset.
                let mut far = outer.clone();
                far[0] = a.origin + a.dir * a.cut + na * a.edge_left;
                let last = far.len() - 1;
                far[last] = b.origin + b.dir * b.cut - nb * b.edge_right;
                far.reverse();
                side.extend(far);
            }
            None => {
                side.push(b.origin + b.dir * b.cut - nb * b.edge_right);
                if let Some(e) = corners[i].outer {
                    side.push(e);
                }
                side.push(a.origin + a.dir * a.cut + na * a.edge_left);
            }
        }
        side.dedup_by(|p, q| p.distance(*q) < 1e-6);
        if side.len() >= 3 && archisim_core::signed_area(&side).abs() > 1e-4 {
            sidewalks.push(side);
        }
    }
    plate.dedup_by(|p, q| p.distance(*q) < 1e-6);
    if archisim_core::signed_area(&plate) < 0.0 {
        plate.reverse();
    }
    let mut parts = Vec::new();
    let (top, pavement, kerb_material) = match ring {
        Some(c) => {
            // Its island, raised and filled, and the apron round it.
            if c.island > 0.1 {
                let kh = kerb_height.max(0.15);
                parts.push(JunctionPart {
                    band: match c.island_finish {
                        IslandFinish::Garden => RoadBand::Planting,
                        IslandFinish::Paved => RoadBand::Median,
                    },
                    outline: archisim_core::circle_of(c.centre, c.island, 64),
                    material: c.island_material,
                    z0: -owner.section.depth,
                    z1: kh,
                });
                if c.apron > 0.05 {
                    let steps = 48;
                    let (r0, r1) = (c.island, c.island + c.apron);
                    for k in 0..steps {
                        let t0 = std::f64::consts::TAU * k as f64 / steps as f64;
                        let t1 = std::f64::consts::TAU * (k + 1) as f64 / steps as f64;
                        let at = |r: f64, t: f64| c.centre + Point2::new(t.cos(), t.sin()) * r;
                        parts.push(JunctionPart {
                            band: RoadBand::Kerb,
                            outline: vec![at(r0, t0), at(r1, t0), at(r1, t1), at(r0, t1)],
                            material: c.kerb_material,
                            z0: -owner.section.depth,
                            z1: 0.05,
                        });
                    }
                }
            }
            (c.top_offset, c.pavement, c.kerb_material)
        }
        None => (
            owner.top_under(x),
            if kind == NodeKind::Shared {
                owner.sidewalk_material
            } else {
                owner.pavement
            },
            owner.kerb_material,
        ),
    };
    for b in bollards {
        parts.push(JunctionPart {
            band: RoadBand::Kerb,
            outline: square(b, BOLLARD),
            material: kerb_material,
            z0: lift,
            z1: lift + BOLLARD_HEIGHT,
        });
    }
    // The signs and their marks on the road, for the traffic coming in by
    // each arm (N1). Traffic keeps to the right: it comes in on the arm's
    // left.
    let mut marks = Vec::new();
    let mut signs = Vec::new();
    if ring.is_none() {
        for (i, a) in arms.iter().enumerate() {
            let sign = settings
                .and_then(|s| s.arm(Junction::arm_ref(a)).sign)
                .unwrap_or_default();
            if sign == ArmSign::None {
                continue;
            }
            let na = a.dir.perp_left();
            let wide = (a.kerb_left - bump_left[i]).min(a.lanes_left).max(0.0);
            let s0 = a.cut + MARK_GAP;
            if !marks_hidden && wide > 0.5 {
                let (from, to) = (0.1, wide - 0.1);
                let bar = |l0: f64, l1: f64| {
                    let p = a.origin + a.dir * s0;
                    let q = p + a.dir * STOP_LINE;
                    vec![p + na * l0, q + na * l0, q + na * l1, p + na * l1]
                };
                match sign {
                    ArmSign::Yield => {
                        // As many dashes as fit, the gaps shared out.
                        let span = to - from;
                        let count = ((span + YIELD_GAP) / (YIELD_DASH + YIELD_GAP))
                            .round()
                            .max(1.0) as usize;
                        let gap = if count > 1 {
                            (span - count as f64 * YIELD_DASH) / (count - 1) as f64
                        } else {
                            0.0
                        };
                        for c in 0..count {
                            let l = from + c as f64 * (YIELD_DASH + gap.max(0.1));
                            marks.push(bar(l, (l + YIELD_DASH).min(to)));
                        }
                    }
                    _ => marks.push(bar(from, to)),
                }
            }
            // The post on the sidewalk to the right of the traffic.
            let out = if a.edge_left > a.kerb_left + a.kerb_width_left + 0.6 {
                a.kerb_left + a.kerb_width_left + 0.5
            } else {
                a.kerb_left + 0.3
            };
            let post = a.origin + a.dir * (s0 + 1.0) + na * out;
            signs.push((post, a.dir, sign));
            let up = kerb_height;
            parts.push(JunctionPart {
                band: RoadBand::Kerb,
                outline: square(post, POST),
                material: kerb_material,
                z0: up,
                z1: up + if sign == ArmSign::Lights { 3.2 } else { 2.6 },
            });
            let (half_w, half_t, z0, z1) = if sign == ArmSign::Lights {
                (0.18, 0.15, 2.3, 3.2)
            } else {
                (0.35, 0.02, 1.9, 2.6)
            };
            let c = post + a.dir * (POST + half_t);
            parts.push(JunctionPart {
                band: RoadBand::Kerb,
                outline: vec![
                    c - na * half_w - a.dir * half_t,
                    c + na * half_w - a.dir * half_t,
                    c + na * half_w + a.dir * half_t,
                    c - na * half_w + a.dir * half_t,
                ],
                material: kerb_material,
                z0: up + z0,
                z1: up + z1,
            });
        }
    }
    // The stretches of its roads without their own markings.
    let quiet = if marks_hidden {
        arms.iter()
            .map(|a| {
                let reach = a.cut + QUIET_REACH;
                if a.forward {
                    (a.road, a.station, a.station + reach)
                } else {
                    (a.road, a.station - reach, a.station)
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let flat = settings.and_then(|s| s.flat).unwrap_or(false);
    Some(Junction {
        at: x,
        owner: owner.id,
        top,
        kerb_height,
        pavement,
        kerb_material,
        sidewalk_material: owner.sidewalk_material,
        depth: owner.section.depth,
        ring: ring.map(|c| c.id),
        parts,
        arms,
        plate,
        kerbs,
        sidewalks,
        key: None,
        kind,
        lift,
        ramps,
        flat,
        main,
        marks,
        signs,
        quiet,
        bump_kerbs,
    })
}

/// A corner with nothing: the kerbs go straight from one arm to the next.
fn straight_corner() -> Corner {
    Corner {
        t_a: 0.0,
        t_b: 0.0,
        line: Vec::new(),
        outer: None,
        outer_line: None,
        centre: None,
    }
}

/// The outside corner of a bend from `a` to `b`: an arc round the inside
/// corner's `centre`, from `a`'s kerb line to `b`'s, and the sidewalk
/// following it.
fn bend_outside(a: &Arm, b: &Arm, centre: Point2) -> Corner {
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let pa = a.origin + na * a.kerb_left;
    let pb = b.origin - nb * b.kerb_right;
    let ta = pa + a.dir * (centre - pa).dot(a.dir);
    let tb = pb + b.dir * (centre - pb).dot(b.dir);
    let line = arc(centre, ta, tb, false);
    let walk = (a.edge_left - a.kerb_left).min(b.edge_right - b.kerb_right);
    Corner {
        t_a: (ta - a.origin).dot(a.dir),
        t_b: (tb - b.origin).dot(b.dir),
        outer_line: offset_path(&line, -walk),
        line,
        outer: None,
        centre: Some(centre),
    }
}

/// A square of half side `h` round `c`.
fn square(c: Point2, h: f64) -> Vec<Point2> {
    vec![
        c + Vec2::new(-h, -h),
        c + Vec2::new(h, -h),
        c + Vec2::new(h, h),
        c + Vec2::new(-h, h),
    ]
}

/// The lane kept beside a bump-out, metres.
const LANE_KEPT: f64 = 2.75;
/// How long a continuous node's change of section is, on each arm, metres.
pub const TRANSITION: f64 = 6.0;
/// From the end of an arm to its stop or give-way line, metres.
const MARK_GAP: f64 = 0.5;
/// How wide a stop line is (M-4.1) and a give-way line's dashes are long
/// across and apart (M-4.2), metres.
const STOP_LINE: f64 = 0.4;
const YIELD_DASH: f64 = 0.8;
const YIELD_GAP: f64 = 0.4;
/// Half the side of a sign's post, metres.
const POST: f64 = 0.04;
/// How far past an arm's end its road's own markings are left out when
/// a node hides them, metres (where the zebras stand).
const QUIET_REACH: f64 = 15.0;

/// Half the side of a bollard, metres.
const BOLLARD: f64 = 0.08;
/// How high a bollard stands, metres.
const BOLLARD_HEIGHT: f64 = 0.9;
/// How far apart the bollards stand, metres.
const BOLLARD_GAP: f64 = 1.5;

/// Where the bollards of a shared surface stand along a corner's kerb
/// line: every [`BOLLARD_GAP`], a little in from the line.
fn bollards_along(line: &[Point2]) -> Vec<Point2> {
    let len = length(line);
    if len < 0.5 {
        return Vec::new();
    }
    let n = ((len / BOLLARD_GAP).floor() as usize).max(1);
    (0..=n)
        .map(|k| {
            let (p, u) = along(line, len * k as f64 / n as f64);
            // The sidewalk is to the right of the line.
            p - u.perp_left() * 0.3
        })
        .collect()
}

/// The zebra crossing of a node's arm (N1): where along its road it is,
/// if the road has one within its first metres past the node.
pub fn arm_crossing(model: &Model, a: &Arm) -> Option<f64> {
    let r = model.road(a.road)?;
    let reach = a.cut + QUIET_REACH;
    r.section.crossings.iter().map(|c| c.at).find(|&at| {
        let d = if a.forward {
            at - a.station
        } else {
            a.station - at
        };
        d > 0.0 && d <= reach
    })
}

/// Lays or takes away the zebra crossing of a node's arm (N1): a crossing
/// of its road just past the node's end (its kerbs lowered with it).
pub fn set_arm_crossing(
    model: &mut Model,
    a: &Arm,
    on: bool,
) -> Result<(), archisim_core::ModelError> {
    let here = arm_crossing(model, a);
    match (here, on) {
        (Some(at), false) => model.edit_road(a.road, |r| {
            r.section.crossings.retain(|c| (c.at - at).abs() > 1e-6);
        }),
        (None, true) => {
            let w = archisim_core::ZEBRA_WIDTH;
            let off = a.cut + archisim_core::ZEBRA_SETBACK + w / 2.0;
            let at = if a.forward {
                a.station + off
            } else {
                a.station - off
            };
            let len = model.road(a.road).map_or(0.0, |r| r.length());
            if at < w / 2.0 || at > len - w / 2.0 {
                return Ok(());
            }
            model.edit_road(a.road, |r| {
                r.section.crossings.push(archisim_core::Crossing {
                    at: (at * 100.0).round() / 100.0,
                    width: w,
                });
            })
        }
        _ => Ok(()),
    }
}

/// Where a corner's handle is and how dragging it changes its radius (N1):
/// the point where its kerb lines meet, the way into the corner along its
/// bisector and half the angle between its arms. None if the arms go
/// straight on or turn back.
pub fn corner_frame(a: &Arm, b: &Arm) -> Option<(Point2, Vec2, f64)> {
    let theta = (b.dir.y.atan2(b.dir.x) - a.dir.y.atan2(a.dir.x)).rem_euclid(std::f64::consts::TAU);
    if (theta - std::f64::consts::PI).abs() < STRAIGHT || theta >= std::f64::consts::PI {
        return None;
    }
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let apex = meet(
        a.origin + na * a.kerb_left,
        a.dir,
        b.origin - nb * b.kerb_right,
        b.dir,
    )?;
    let bis = (a.dir + b.dir).normalized()?;
    Some((apex, bis, theta / 2.0))
}

/// The middle of a corner's arc of `radius` (its handle).
pub fn corner_middle(apex: Point2, bis: Vec2, half: f64, radius: f64) -> Point2 {
    apex + bis * (radius * (1.0 / half.sin() - 1.0))
}

/// The radius whose arc's middle is at `p` (dragging a corner's handle).
pub fn corner_radius_at(apex: Point2, bis: Vec2, half: f64, p: Point2) -> f64 {
    let d = (p - apex).dot(bis);
    (d / (1.0 / half.sin() - 1.0)).max(0.0)
}

/// The usual size of a chamfer where the project is (N1): the radius of
/// the corner it cuts, metres. Barcelona's Eixample, larger; elsewhere, the
/// chamfers of the Spanish ensanches.
pub fn standard_chamfer(model: &Model) -> f64 {
    let Some((lat, lon)) = model.georeference().and_then(|g| g.lat_lon()) else {
        return 5.0;
    };
    if (41.32..=41.47).contains(&lat) && (2.05..=2.25).contains(&lon) {
        // Barcelona: the Eixample's chamfers.
        return 9.0;
    }
    let place = archisim_core::world_places()
        .get(model.world_place() as usize)
        .copied()
        .unwrap_or("");
    if place.starts_with("Cataluña") || place.starts_with("Valencia") {
        7.0
    } else {
        5.0
    }
}

/// The stretches of a road's axis where its own markings are left out:
/// those its junctions take and those their nodes keep quiet (N1).
pub fn marking_gaps(junctions: &[Junction], road: RoadId) -> Vec<(f64, f64)> {
    let mut gaps = road_gaps(junctions, road);
    gaps.extend(
        junctions
            .iter()
            .flat_map(|j| j.quiet.iter())
            .filter(|q| q.0 == road)
            .map(|q| (q.1, q.2)),
    );
    gaps
}

/// The stretches of a road's axis that its junctions take: (from, to).
pub fn road_gaps(junctions: &[Junction], road: RoadId) -> Vec<(f64, f64)> {
    junctions
        .iter()
        .flat_map(|j| j.arms.iter())
        .filter(|a| a.road == road)
        .map(|a| {
            if a.forward {
                (a.station, a.station + a.cut)
            } else {
                (a.station - a.cut, a.station)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{signed_area, NewRoad, RoadSection};

    fn town() -> (Model, LevelId, RoadId, RoadId) {
        let mut m = Model::new("Cruce");
        let level = m.add_level("Planta baja", 0.0);
        let asphalt = m.add_material(
            "Asfalto",
            "asphalt",
            [60, 60, 60],
            archisim_core::CutPattern::Solid,
        );
        let stone = m.add_material(
            "Piedra",
            "stone",
            [200, 200, 200],
            archisim_core::CutPattern::Solid,
        );
        let add = |m: &mut Model, a: Point2, b: Point2| {
            m.add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![a, b],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),

                changes: Vec::new(),
                pavement: asphalt,
                kerb_material: stone,
                sidewalk_material: stone,
            })
            .unwrap()
        };
        let a = add(&mut m, Point2::new(-50.0, 0.0), Point2::new(50.0, 0.0));
        let b = add(&mut m, Point2::new(0.0, -50.0), Point2::new(0.0, 50.0));
        (m, level, a, b)
    }

    /// The junctions kept are given back only while the roads are the
    /// same: moving one gives new ones, and moving it back the first ones.
    #[test]
    fn kept_junctions_follow_the_roads() {
        let (mut m, level, a, _) = town();
        let first = junctions(&m, level);
        assert!(Arc::ptr_eq(
            &shared_junctions(&m, level),
            &shared_junctions(&m, level)
        ));
        let axis = m.road(a).unwrap().axis.clone();
        m.edit_road(a, |r| {
            for p in &mut r.axis {
                p.y += 3.0;
            }
        })
        .unwrap();
        let moved = junctions(&m, level);
        assert_ne!(first, moved);
        assert!((moved[0].at.y - first[0].at.y - 3.0).abs() < 1e-9);
        m.edit_road(a, |r| r.axis = axis).unwrap();
        assert_eq!(junctions(&m, level), first);
    }

    #[test]
    fn a_crossing_has_four_rounded_corners() {
        let (m, level, a, b) = town();
        let js = junctions(&m, level);
        assert_eq!(js.len(), 1);
        let j = &js[0];
        assert!(j.at.distance(Point2::new(0.0, 0.0)) < 1e-9);
        assert_eq!(j.arms.len(), 4);
        assert_eq!(j.kerbs.len(), 4);
        assert_eq!(j.sidewalks.len(), 4);
        // Carriageway 6 + parking 2·2,2 = 10,4 m: the plate reaches 5,2 m
        // each side plus the 4 m corners: each arm cut at 9,2 m.
        for arm in &j.arms {
            assert!((arm.cut - 9.2).abs() < 1e-6, "{}", arm.cut);
        }
        let area = signed_area(&j.plate);
        // The cross of four arms 10,4 × 9,2 and the middle square, less
        // the four corners' (4 − π) · r² / … : between the two.
        let square = 10.4 * 10.4 + 4.0 * 10.4 * (9.2 - 5.2);
        let full = square + 4.0 * 16.0;
        assert!(area > square && area < full, "{area}");
        // Each road leaves out 9,2 m each way of the crossing.
        let gaps = road_gaps(&js, a);
        assert_eq!(gaps.len(), 2);
        assert!(gaps
            .iter()
            .any(|g| (g.0 - 50.0).abs() < 1e-6 && (g.1 - 59.2).abs() < 1e-6));
        assert_eq!(road_gaps(&js, b).len(), 2);
    }

    #[test]
    fn a_t_and_an_l() {
        let (mut m, level, _, b) = town();
        // A road ending on the first one: a T.
        m.edit_road(b, |r| {
            r.axis = vec![Point2::new(0.0, 0.0), Point2::new(0.0, 50.0)]
        })
        .unwrap();
        let js = junctions(&m, level);
        assert_eq!(js.len(), 1);
        assert_eq!(js[0].arms.len(), 3);
        // Two rounded corners and the straight kerb across.
        assert_eq!(js[0].kerbs.len(), 3, "{:?}", js[0].kerbs.len());
        // Both ending at the same point: an L.
        m.edit_road(b, |r| {
            r.axis = vec![Point2::new(50.0, 0.0), Point2::new(50.0, 50.0)]
        })
        .unwrap();
        let js = junctions(&m, level);
        assert_eq!(js.len(), 1);
        assert_eq!(js[0].arms.len(), 2);
        assert_eq!(js[0].kerbs.len(), 2);
        assert!(signed_area(&js[0].plate) > 100.0);
    }

    #[test]
    fn a_roundabout_takes_the_roads_that_reach_it() {
        let (mut m, level, a, b) = town();
        let mat = m.road(a).unwrap().pavement;
        // A road ending near it from the north-east, too.
        m.add_road(NewRoad {
            name: "Diagonal".into(),
            level,
            axis: vec![Point2::new(12.0, 12.0), Point2::new(60.0, 60.0)],
            section: RoadSection::default(),
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: mat,
            kerb_material: mat,
            sidewalk_material: mat,
        })
        .unwrap();
        m.add_roundabout(archisim_core::NewRoundabout {
            name: "G".into(),
            level,
            centre: Point2::new(0.0, 0.0),
            radius: 15.0,
            island: 7.0,
            apron: 1.5,
            top_offset: 0.0,
            plaza: false,
            island_finish: IslandFinish::Garden,
            pavement: mat,
            kerb_material: mat,
            island_material: mat,
        })
        .unwrap();
        let js = junctions(&m, level);
        assert_eq!(js.len(), 1, "no other junction: the crossing is inside it");
        let j = &js[0];
        assert!(j.ring.is_some());
        assert_eq!(j.arms.len(), 5);
        for arm in &j.arms {
            assert!(arm.cut > 15.0, "{}", arm.cut);
        }
        // The disc and the arms' mouths.
        let area = signed_area(&j.plate);
        assert!(area > std::f64::consts::PI * 15.0 * 15.0, "{area}");
        assert_eq!(j.kerbs.len(), 5);
        assert_eq!(j.sidewalks.len(), 5);
        // The island and the apron's pieces.
        assert_eq!(j.parts.len(), 1 + 48);
        // Each road leaves out its stretch in it.
        let gaps = road_gaps(&js, b);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.iter().all(|g| g.1 - g.0 > 15.0));
    }

    /// The node of the crossing of `town` and its settings, changed by
    /// `f`.
    fn changed(m: &mut Model, level: LevelId, f: impl FnOnce(&mut NodeSettings, &Junction)) {
        let j = junctions(m, level).remove(0);
        let key = j.key.clone().expect("a node");
        let mut n = m
            .node(&key)
            .cloned()
            .unwrap_or_else(|| NodeSettings::new(key));
        f(&mut n, &j);
        m.set_node(n).unwrap();
    }

    /// A node keeps its key while its streets move, and its settings with
    /// it; taking a street away forgets them.
    #[test]
    fn a_node_is_the_same_while_its_streets_meet() {
        let (mut m, level, a, b) = town();
        let key = junctions(&m, level)[0].key.clone().unwrap();
        assert_eq!(key.roads.len(), 2);
        assert!(key.roads.iter().all(|r| r.1 == NodeEnd::Through));
        changed(&mut m, level, |n, _| n.kind = Some(NodeKind::Raised));
        m.edit_road(a, |r| {
            for p in &mut r.axis {
                p.y += 2.0;
            }
        })
        .unwrap();
        let j = &junctions(&m, level)[0];
        assert_eq!(j.key.as_ref(), Some(&key));
        assert_eq!(j.kind, NodeKind::Raised);
        // The crossing and the four free ends of its streets.
        let keys = all_node_keys(&m);
        assert_eq!(keys.len(), 5);
        assert!(keys.contains(&key));
        m.remove_road(b).unwrap();
        assert!(m.node(&key).is_none());
    }

    /// A raised crossing: its carriageway at the sidewalk's height, with a
    /// ramp down each arm; a shared surface also without kerbs and with
    /// bollards, paved as the sidewalk.
    #[test]
    fn raised_and_shared_nodes() {
        let (mut m, level, _, _) = town();
        let plain = junctions(&m, level).remove(0);
        changed(&mut m, level, |n, _| n.kind = Some(NodeKind::Raised));
        let j = junctions(&m, level).remove(0);
        assert!((j.lift - 0.15).abs() < 1e-9, "{}", j.lift);
        assert_eq!(j.ramps.len(), 4);
        assert_eq!(j.kerbs.len(), 4);
        assert!((j.lift_at(j.at) - j.lift).abs() < 1e-9);
        for a in &j.arms {
            // Up at the plateau, down at the arm's end.
            assert!(j.lift_at(a.origin + a.dir * (a.cut - RAMP)) > j.lift - 1e-6);
            assert!(j.lift_at(a.origin + a.dir * (a.cut - 1e-3)) < 0.01);
            assert!(a.cut >= plain.arms[0].cut + RAMP - 1e-6);
        }
        changed(&mut m, level, |n, _| n.kind = Some(NodeKind::Shared));
        let j = junctions(&m, level).remove(0);
        assert!(j.kerbs.is_empty());
        assert!(!j.parts.is_empty(), "bolardos");
        assert_eq!(j.pavement, j.sidewalk_material);
        // No node: the streets lie one on the other.
        changed(&mut m, level, |n, _| n.kind = Some(NodeKind::Open));
        assert!(junctions(&m, level).is_empty());
        let found = node_at(&m, level, Point2::new(0.5, 0.5), 15.0).unwrap();
        assert_eq!(found.key, plain.key);
    }

    /// Each arm's setback and each corner's radius, shape and sidewalk.
    #[test]
    fn arms_and_corners_by_hand() {
        let (mut m, level, _, _) = town();
        let plain = junctions(&m, level).remove(0);
        changed(&mut m, level, |n, j| {
            let a = Junction::arm_ref(&j.arms[0]);
            n.set_arm(
                a,
                archisim_core::ArmSettings {
                    setback: Some(20.0),
                    ..Default::default()
                },
            );
            n.set_corner(
                a,
                archisim_core::CornerSettings {
                    radius: Some(8.0),
                    shape: Some(CornerShape::Chamfer),
                    sidewalk: Some(false),
                    ..Default::default()
                },
            );
        });
        let j = junctions(&m, level).remove(0);
        assert!((j.arms[0].cut - 20.0).abs() < 1e-9);
        assert_eq!(j.sidewalks.len(), 3);
        // The chamfer is shorter than the arc it replaces, with its larger
        // radius reaching further out along the next arm.
        assert!(j.arms[1].cut > plain.arms[1].cut + 1.0);
        // A sharp corner: no rounding, the sidewalk reaches the kerbs' corner.
        let before = signed_area(&j.plate);
        changed(&mut m, level, |n, j| {
            let a = Junction::arm_ref(&j.arms[2]);
            n.set_corner(
                a,
                archisim_core::CornerSettings {
                    shape: Some(CornerShape::Sharp),
                    ..Default::default()
                },
            );
        });
        let j = junctions(&m, level).remove(0);
        assert!(signed_area(&j.plate) < before - 1.0);
        // Back as worked out.
        let key = j.key.clone().unwrap();
        m.set_node(NodeSettings::new(key.clone())).unwrap();
        assert!(m.node(&key).is_none());
        assert_eq!(junctions(&m, level)[0].plate, plain.plate);
    }

    /// A road like the town's.
    fn road(m: &mut Model, level: LevelId, a: Point2, b: Point2) -> RoadId {
        let mat = m.materials().next().unwrap().id;
        m.add_road(NewRoad {
            name: "Calle".into(),
            level,
            axis: vec![a, b],
            section: RoadSection::default(),
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: mat,
            kerb_material: mat,
            sidewalk_material: mat,
        })
        .unwrap()
    }

    /// The node at `p` (also a dead end or ends together) changed by `f`.
    fn changed_at(
        m: &mut Model,
        level: LevelId,
        p: Point2,
        f: impl FnOnce(&mut NodeSettings, &[Arm]),
    ) {
        let key = node_key_at(m, level, p, 3.0).unwrap();
        let arms = node_arms(m, &key);
        let mut n = m
            .node(&key)
            .cloned()
            .unwrap_or_else(|| NodeSettings::new(key));
        f(&mut n, &arms);
        m.set_node(n).unwrap();
    }

    /// The second round of N1: dead ends with a turning circle, ends
    /// together that go on without a seam, bends, bump-outs, signs with
    /// their marks, the main street and a flat node, and a node without
    /// markings.
    #[test]
    fn more_kinds_of_node() {
        let (mut m, level, a, b) = town();
        let p = Point2::new;
        // A free end: no junction until it is a dead end.
        let before = junctions(&m, level).len();
        assert!(dead_ends(&m, level)
            .iter()
            .any(|(_, q)| q.distance(p(50.0, 0.0)) < 1e-6));
        changed_at(&mut m, level, p(50.0, 0.0), |n, arms| {
            assert_eq!(arms.len(), 1);
            n.kind = Some(NodeKind::DeadEnd);
            n.end_radius = Some(10.0);
        });
        let js = junctions(&m, level);
        assert_eq!(js.len(), before + 1);
        let end = js.iter().find(|j| j.kind == NodeKind::DeadEnd).unwrap();
        // The turning circle reaches 10 m round the end.
        let arm = &end.arms[0];
        let far = end
            .plate
            .iter()
            .filter(|q| (**q - arm.origin).dot(arm.dir) < 0.0)
            .map(|q| q.distance(p(50.0, 0.0)))
            .fold(0.0, f64::max);
        assert!((far - 10.0).abs() < 0.2, "{far}");
        assert!(!end.sidewalks.is_empty() && !end.kerbs.is_empty());
        // Two streets in line, one after the other: a node only by hand.
        let c = road(&mut m, level, p(0.0, 50.0), p(0.0, 90.0));
        let _ = c;
        // (The second street of the town ends where the new one starts.)
        assert!(joints(&m, level)
            .iter()
            .any(|(_, q)| q.distance(p(0.0, 50.0)) < 1e-6));
        let n0 = junctions(&m, level).len();
        changed_at(&mut m, level, p(0.0, 50.0), |n, arms| {
            assert_eq!(arms.len(), 2);
            n.kind = Some(NodeKind::Continuous);
        });
        let js = junctions(&m, level);
        assert_eq!(js.len(), n0 + 1);
        let cont = js.iter().find(|j| j.kind == NodeKind::Continuous).unwrap();
        assert!(cont.arms.iter().all(|a| (a.cut - TRANSITION).abs() < 1e-9));
        assert_eq!(cont.kerbs.len(), 2);
        // A bend: the outside corner round the inside one's centre.
        let d = road(&mut m, level, p(100.0, 0.0), p(140.0, 0.0));
        let _e = road(&mut m, level, p(140.0, 0.0), p(170.0, 30.0));
        changed_at(&mut m, level, p(140.0, 0.0), |n, arms| {
            n.kind = Some(NodeKind::Bend);
            for a in arms {
                n.set_corner(
                    Junction::arm_ref(a),
                    archisim_core::CornerSettings {
                        radius: Some(10.0),
                        ..Default::default()
                    },
                );
            }
        });
        let bend = junctions(&m, level)
            .into_iter()
            .find(|j| j.kind == NodeKind::Bend)
            .unwrap();
        assert!(bend.arms.iter().any(|x| x.road == d));
        assert_eq!(bend.sidewalks.len(), 2);
        // The bend's plate has no corner point outside: all its points
        // are within the outer radius of the curve.
        assert!(bend.plate.len() > 10);
        // The crossing: the second street gives way to the first, which
        // rules its height, with bump-outs, flat and without markings.
        changed_at(&mut m, level, p(0.0, 0.0), |n, arms| {
            n.main = Some(a);
            n.flat = Some(true);
            n.hide_marks = Some(false);
            for x in arms {
                let r = Junction::arm_ref(x);
                if x.road == b {
                    n.set_arm(
                        r,
                        archisim_core::ArmSettings {
                            sign: Some(archisim_core::ArmSign::Stop),
                            ..Default::default()
                        },
                    );
                }
                n.set_corner(
                    r,
                    archisim_core::CornerSettings {
                        bump: Some(2.0),
                        ..Default::default()
                    },
                );
            }
        });
        let cross = junctions(&m, level)
            .into_iter()
            .find(|j| j.arms.len() == 4)
            .unwrap();
        assert_eq!(cross.main, Some(a));
        assert!(cross.flat);
        assert_eq!(cross.signs.len(), 2);
        assert_eq!(cross.marks.len(), 2);
        assert_eq!(cross.bump_kerbs.len(), 8);
        // The plate is narrower at the arms' ends by the bump-outs.
        let x = &cross.arms[0];
        let w = cross
            .plate
            .iter()
            .filter(|q| ((**q - x.origin).dot(x.dir) - x.cut).abs() < 1e-6)
            .map(|q| (*q - x.origin).cross(x.dir).abs())
            .fold(0.0, f64::max);
        assert!((w - (x.kerb_left - 2.0)).abs() < 1e-6, "{w}");
        // A stop line of each street coming in, across its lanes only.
        for mk in &cross.marks {
            let across = mk.iter().map(|q| q.distance(mk[0])).fold(0.0, f64::max);
            assert!(across < x.lanes_left + 0.5, "{across}");
        }
        // Without markings: the stop lines go and the roads keep quiet
        // near it.
        changed_at(&mut m, level, p(0.0, 0.0), |n, _| n.hide_marks = Some(true));
        let quiet = junctions(&m, level)
            .into_iter()
            .find(|j| j.arms.len() == 4)
            .unwrap();
        assert!(quiet.marks.is_empty() && !quiet.signs.is_empty());
        assert!(marking_gaps(std::slice::from_ref(&quiet), a).len() > road_gaps(&[quiet], a).len());
        // A zebra on an arm, laid and taken away on its road.
        let arm = cross.arms.iter().find(|x| x.road == b).unwrap().clone();
        assert!(arm_crossing(&m, &arm).is_none());
        set_arm_crossing(&mut m, &arm, true).unwrap();
        assert!(arm_crossing(&m, &arm).is_some());
        set_arm_crossing(&mut m, &arm, false).unwrap();
        assert!(arm_crossing(&m, &arm).is_none());
        // Every node's key is there to keep its settings.
        let keys = all_node_keys(&m);
        assert!(m.nodes().all(|n| keys.contains(&n.key)));
        assert_eq!(m.keep_nodes(&keys), 0);
    }

    /// A corner's handle: dragging it to where an arc's middle is gives
    /// that arc's radius back.
    #[test]
    fn corner_handles_give_the_radius() {
        let (m, level, _, _) = town();
        let j = junctions(&m, level).remove(0);
        let (apex, bis, half) = corner_frame(&j.arms[0], &j.arms[1]).unwrap();
        let mid = corner_middle(apex, bis, half, 6.5);
        assert!((corner_radius_at(apex, bis, half, mid) - 6.5).abs() < 1e-9);
        // An arc of the usual radius has its middle on the plate's edge.
        let usual = corner_middle(apex, bis, half, 4.0);
        let near = j
            .plate
            .iter()
            .map(|q| q.distance(usual))
            .fold(f64::MAX, f64::min);
        assert!(near < 0.1, "{near}");
    }
}
