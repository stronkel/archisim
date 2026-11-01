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
    offset_path, strip, LevelId, MaterialId, Model, Point2, Road, RoadBand, RoadId, Vec2,
};

/// A road going out of a junction.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    pub road: RoadId,
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
}

fn corner(x: Point2, a: &Arm, b: &Arm, radius: f64) -> Corner {
    let (na, nb) = (a.dir.perp_left(), b.dir.perp_left());
    let pa = x + na * a.kerb_left;
    let pb = x - nb * b.kerb_right;
    let theta = {
        let t = b.dir.y.atan2(b.dir.x) - a.dir.y.atan2(a.dir.x);
        t.rem_euclid(std::f64::consts::TAU)
    };
    let outer = meet(x + na * a.edge_left, a.dir, x - nb * b.edge_right, b.dir);
    if (theta - std::f64::consts::PI).abs() < STRAIGHT {
        return Corner {
            t_a: 0.0,
            t_b: 0.0,
            line: Vec::new(),
            outer: None,
        };
    }
    let Some(c) = meet(pa, a.dir, pb, b.dir) else {
        return Corner {
            t_a: 0.0,
            t_b: 0.0,
            line: Vec::new(),
            outer: None,
        };
    };
    if theta > std::f64::consts::PI {
        // The outside of a bend: the kerbs meet in a point.
        return Corner {
            t_a: (c - x).dot(a.dir),
            t_b: (c - x).dot(b.dir),
            line: vec![c],
            outer,
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
        t_a: (ta - x).dot(a.dir),
        t_b: (tb - x).dot(b.dir),
        line,
        outer,
    }
}

/// The junctions of the roads of a level.
pub fn junctions(model: &Model, level: LevelId) -> Vec<Junction> {
    let mut roads: Vec<&Road> = model.roads_on_level(level).collect();
    roads.sort_by_key(|r| r.id.0);
    let mut out = Vec::new();
    for (x, met) in meeting_points(&roads) {
        let mut arms: Vec<Arm> = Vec::new();
        for &(i, s) in &met {
            let r = roads[i];
            let len = length(&r.axis);
            let (_, u) = along(&r.axis, s.clamp(0.0, len));
            let (kl, kr, wl, wr, hl, hr) = reach(r, s.clamp(0.0, len));
            for forward in [true, false] {
                let room = if forward { len - s } else { s };
                if room <= SAME {
                    continue;
                }
                // Looking back along the road, its left is its right.
                let (dir, kerb_left, kerb_right, kw_l, kw_r, e_l, e_r) = if forward {
                    (u, kl, kr, wl, wr, hl, hr)
                } else {
                    (u * -1.0, kr, kl, wr, wl, hr, hl)
                };
                arms.push(Arm {
                    road: r.id,
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
                });
            }
        }
        if arms.len() < 2 || arms.iter().all(|a| a.road == arms[0].road) {
            continue;
        }
        arms.sort_by(|a, b| a.dir.y.atan2(a.dir.x).total_cmp(&b.dir.y.atan2(b.dir.x)));
        let n = arms.len();
        let owner = roads
            .iter()
            .find(|r| met.iter().any(|&(i, _)| roads[i].id == r.id))
            .copied()
            .expect("a road meets there");
        let radius_of = |a: &Arm, b: &Arm| {
            let ra = model.road(a.road).map_or(4.0, |r| r.section.corner_radius);
            let rb = model.road(b.road).map_or(4.0, |r| r.section.corner_radius);
            ra.min(rb).max(0.0)
        };
        // The corners first, which set where the arms are cut: the cut of
        // an arm reaches past both its corners and past the other roads'
        // carriageways.
        let mut corners: Vec<Corner> = (0..n)
            .map(|i| {
                let (a, b) = (&arms[i], &arms[(i + 1) % n]);
                corner(x, a, b, radius_of(a, b))
            })
            .collect();
        for i in 0..n {
            let prev = &corners[(i + n - 1) % n];
            let next = &corners[i];
            let wide = arms
                .iter()
                .filter(|o| o.road != arms[i].road)
                .map(|o| o.kerb_left.max(o.kerb_right) + o.kerb_width_left.max(o.kerb_width_right))
                .fold(0.0, f64::max);
            arms[i].cut = next.t_a.max(prev.t_b).max(wide).max(0.5);
        }
        // Again, now that the cuts are known (the kerb line runs to them).
        corners = (0..n)
            .map(|i| {
                let (a, b) = (&arms[i], &arms[(i + 1) % n]);
                corner(x, a, b, radius_of(a, b))
            })
            .collect();
        let mut plate = Vec::new();
        let mut kerbs = Vec::new();
        let mut sidewalks = Vec::new();
        let kerb_height = met
            .iter()
            .map(|&(i, _)| {
                let s = &roads[i].section;
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
            let right_cut = x + a.dir * a.cut - na * a.kerb_right;
            let left_cut = x + a.dir * a.cut + na * a.kerb_left;
            plate.push(right_cut);
            plate.push(left_cut);
            // The kerb line of the corner, from the cut of `a` to the cut
            // of `b`.
            let mut line = vec![left_cut];
            line.extend(corners[i].line.iter().copied());
            line.push(x + b.dir * b.cut - nb * b.kerb_right);
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
            let kw = a.kerb_width_left.max(b.kerb_width_right);
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
            side.push(x + b.dir * b.cut - nb * b.edge_right);
            if let Some(e) = corners[i].outer {
                side.push(e);
            }
            side.push(x + a.dir * a.cut + na * a.edge_left);
            side.dedup_by(|p, q| p.distance(*q) < 1e-6);
            if side.len() >= 3 && archisim_core::signed_area(&side).abs() > 1e-4 {
                sidewalks.push(side);
            }
        }
        plate.dedup_by(|p, q| p.distance(*q) < 1e-6);
        if archisim_core::signed_area(&plate) < 0.0 {
            plate.reverse();
        }
        out.push(Junction {
            at: x,
            owner: owner.id,
            top: owner.top_under(x),
            kerb_height,
            pavement: owner.pavement,
            kerb_material: owner.kerb_material,
            sidewalk_material: owner.sidewalk_material,
            depth: owner.section.depth,
            arms,
            plate,
            kerbs,
            sidewalks,
        });
    }
    out
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
}
