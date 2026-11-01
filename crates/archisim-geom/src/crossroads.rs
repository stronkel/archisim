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

use archisim_core::{
    offset_path, strip, IslandFinish, LevelId, MaterialId, Model, Point2, Road, RoadBand, RoadId,
    Roundabout, RoundaboutId, Vec2,
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
    /// What else it has: a roundabout's island and apron.
    pub parts: Vec<JunctionPart>,
}

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
    for i in 0..roads.len() {
        for j in i + 1..roads.len() {
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
}

fn corner(a: &Arm, b: &Arm, radius: f64) -> Corner {
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let (xa, xb) = (a.origin, b.origin);
    let pa = xa + na * a.kerb_left;
    let pb = xb - nb * b.kerb_right;
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
        };
    }
    let Some(c) = meet(pa, a.dir, pb, b.dir) else {
        return Corner {
            t_a: 0.0,
            t_b: 0.0,
            line: Vec::new(),
            outer: None,
            outer_line: None,
        };
    };
    if theta > std::f64::consts::PI {
        // The outside of a bend: the kerbs meet in a point.
        return Corner {
            t_a: (c - xa).dot(a.dir),
            t_b: (c - xb).dot(b.dir),
            line: vec![c],
            outer,
            outer_line: None,
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
    }
}

/// An arm of road `r` from `origin`, at `s` along it, going on along it
/// (`forward`) or back.
fn arm_of(r: &Road, origin: Point2, s: f64, forward: bool) -> Arm {
    let len = length(&r.axis);
    let (_, u) = along(&r.axis, s.clamp(0.0, len));
    let (kl, kr, wl, wr, hl, hr) = reach(r, s.clamp(0.0, len));
    // Looking back along the road, its left is its right.
    let (dir, kerb_left, kerb_right, kw_l, kw_r, e_l, e_r) = if forward {
        (u, kl, kr, wl, wr, hl, hr)
    } else {
        (u * -1.0, kr, kl, wr, wl, hr, hl)
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

/// The junctions of the roads of a level: at its roundabouts and squares
/// and wherever roads meet away from them.
pub fn junctions(model: &Model, level: LevelId) -> Vec<Junction> {
    let mut roads: Vec<&Road> = model.roads_on_level(level).collect();
    roads.sort_by_key(|r| r.id.0);
    let mut out = Vec::new();
    let rings: Vec<&Roundabout> = model.roundabouts_on_level(level).collect();
    for c in &rings {
        let arms = ring_arms(&roads, c);
        if let Some(j) = assemble(model, &roads, arms, c.centre, Some(c)) {
            out.push(j);
        }
    }
    for (x, met) in meeting_points(&roads) {
        if rings.iter().any(|c| c.centre.distance(x) < c.radius + 1.0) {
            continue;
        }
        let mut arms: Vec<Arm> = Vec::new();
        for &(i, s) in &met {
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
        if arms.len() < 2 || arms.iter().all(|a| a.road == arms[0].road) {
            continue;
        }
        arms.sort_by(|a, b| a.dir.y.atan2(a.dir.x).total_cmp(&b.dir.y.atan2(b.dir.x)));
        if let Some(j) = assemble(model, &roads, arms, x, None) {
            out.push(j);
        }
    }
    out
}

/// A junction at `x` of `arms` (sorted round it), round a roundabout's
/// circle if `ring`.
fn assemble(
    model: &Model,
    roads: &[&Road],
    mut arms: Vec<Arm>,
    x: Point2,
    ring: Option<&Roundabout>,
) -> Option<Junction> {
    if arms.is_empty() {
        return None;
    }
    let n = arms.len();
    let owner = roads.iter().find(|r| arms.iter().any(|a| a.road == r.id))?;
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
    let make = |arms: &[Arm]| -> Vec<Corner> {
        (0..n)
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
                    None => corner(a, b, radius_of(a.road).min(radius_of(b.road))),
                }
            })
            .collect()
    };
    // The corners first, which set where the arms are cut: the cut of an
    // arm reaches past both its corners and past the other roads'
    // carriageways.
    let corners = make(&arms);
    for i in 0..n {
        let prev = &corners[(i + n - 1) % n];
        let next = &corners[i];
        let wide = if ring.is_some() {
            0.0
        } else {
            arms.iter()
                .filter(|o| o.road != arms[i].road)
                .map(|o| o.kerb_left.max(o.kerb_right) + o.kerb_width_left.max(o.kerb_width_right))
                .fold(0.0, f64::max)
        };
        arms[i].cut = next.t_a.max(prev.t_b).max(wide).max(0.5);
    }
    // Again, now that the cuts are known (the kerb line runs to them).
    let corners = make(&arms);
    let mut plate = Vec::new();
    let mut kerbs = Vec::new();
    let mut sidewalks = Vec::new();
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
    for i in 0..n {
        let (a, b) = (&arms[i], &arms[(i + 1) % n]);
        let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
        let right_cut = a.origin + a.dir * a.cut - na * a.kerb_right;
        let left_cut = a.origin + a.dir * a.cut + na * a.kerb_left;
        plate.push(right_cut);
        plate.push(left_cut);
        // The kerb line of the corner, from the cut of `a` to the cut of
        // `b`.
        let mut line = vec![left_cut];
        line.extend(corners[i].line.iter().copied());
        line.push(b.origin + b.dir * b.cut - nb * b.kerb_right);
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
        if kw > 1e-6 {
            let k = strip(&line, -kw, 0.0);
            if k.len() >= 3 {
                kerbs.push(k);
            }
        }
        let Some(inner) = offset_path(&line, -kw) else {
            continue;
        };
        let mut side = inner;
        match (&corners[i].outer_line, ring) {
            (Some(outer), Some(_)) => {
                // Round a roundabout: out to the roads' outer edges at
                // their cuts, along the circle's offset.
                let mut far = outer.clone();
                far[0] = a.origin + a.dir * a.cut + na * a.edge_left;
                let last = far.len() - 1;
                far[last] = b.origin + b.dir * b.cut - nb * b.edge_right;
                far.reverse();
                side.extend(far);
            }
            _ => {
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
        None => (owner.top_under(x), owner.pavement, owner.kerb_material),
    };
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
    })
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
}
