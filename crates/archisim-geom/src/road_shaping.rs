//! Shaping the street network by hand, as in a city builder («calles
//! plásticas»): its nodes, bending a road between them, pulling a loose
//! end onto another street and joining it, fusing crossings that crowd
//! each other and widening a stretch by its kerb.

use archisim_core::{shape_axis, Model, ModelError, Point2, Road, RoadId, Vec2};

use crate::crossroads::{junctions, shared_junctions, Junction};

/// Parts of a road shorter than this get no handle to bend them.
pub const BEND_MIN: f64 = 6.0;

/// How far a loose end is pulled onto another street's end, crossing or
/// axis when it is dropped near it (at least: a wide street pulls from
/// its own half width).
pub const NODE_REACH: f64 = 8.0;

/// Two streets meeting end to end with the same section join into one if
/// they turn less than this there (radians, 35°).
pub const JOIN_TURN: f64 = 0.61;

/// Where the nodes of the street network are along a road: its ends and
/// where other streets meet it (its crossings), metres from its start,
/// in order.
pub fn road_nodes(model: &Model, road: RoadId) -> Vec<f64> {
    let Some(r) = model.road(road) else {
        return Vec::new();
    };
    let len = r.length();
    let mut s = vec![0.0, len];
    for j in shared_junctions(model, r.level).iter() {
        for a in &j.arms {
            if a.road == road {
                s.push(a.station.clamp(0.0, len));
            }
        }
    }
    s.sort_by(f64::total_cmp);
    s.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    s
}

/// The nodes on each side of `s` along `road` (its ends where there are
/// none).
pub fn nodes_around(model: &Model, road: RoadId, s: f64) -> (f64, f64) {
    let nodes = road_nodes(model, road);
    let len = model.road(road).map_or(0.0, Road::length);
    let from = nodes
        .iter()
        .copied()
        .filter(|&n| n < s - 0.5)
        .fold(0.0, f64::max);
    let to = nodes
        .iter()
        .copied()
        .filter(|&n| n > s + 0.5)
        .fold(len, f64::min);
    (from, to)
}

/// The streets at the node `s` metres along `road`: each with where the
/// node is along it (an end of it, or a point it passes).
pub fn node_roads(model: &Model, road: RoadId, s: f64) -> Vec<(RoadId, f64)> {
    let Some(r) = model.road(road) else {
        return Vec::new();
    };
    let at = r.axis_at(s).0;
    let mut out = vec![(road, s)];
    for o in model.roads_on_level(r.level) {
        if o.id == road || o.free {
            continue;
        }
        let len = o.length();
        let st = if o.axis[0].distance(at) < 1.0 {
            0.0
        } else if o.axis[o.axis.len() - 1].distance(at) < 1.0 {
            len
        } else {
            let st = o.station_of(at);
            if o.axis_at(st).0.distance(at) > 1.0 {
                continue;
            }
            st
        };
        out.push((o.id, st));
    }
    out
}

/// A road pulled by `d` at `s` metres along it: the part between the
/// nodes on each side of `s` bends smoothly (`shape_axis`); an end moves
/// whole.
pub fn bend_road(model: &mut Model, road: RoadId, s: f64, d: Vec2) -> Result<(), ModelError> {
    let (from, to) = nodes_around(model, road, s);
    let Some(r) = model.road(road) else {
        return Err(ModelError::UnknownElement(road.0));
    };
    let from = if s < 0.5 { 0.0 } else { from };
    let axis = shape_axis(&r.axis, from, s, to, d, 2.0);
    model.edit_road(road, |r| r.axis = axis)
}

/// Whether the node `s` metres along `road` is a loose end: an end of it
/// that no other street reaches.
pub fn is_loose_end(model: &Model, road: RoadId, s: f64) -> bool {
    let Some(r) = model.road(road) else {
        return false;
    };
    (s < 0.5 || s > r.length() - 0.5) && node_roads(model, road, s).len() == 1
}

/// Where a loose end of `road` dropped at `to` goes: onto the end of
/// another street, the middle of a crossing or the axis of a street near
/// it, so that it meets it; or `to` itself.
pub fn pull_end(model: &Model, road: RoadId, to: Point2) -> Point2 {
    let Some(r) = model.road(road) else {
        return to;
    };
    let half = r.section.width() / 2.0;
    let others: Vec<&Road> = model
        .roads_on_level(r.level)
        .filter(|o| o.id != road && !o.free)
        .collect();
    let mut best: Option<(f64, Point2)> = None;
    fn take(best: &mut Option<(f64, Point2)>, d: f64, p: Point2, reach: f64) {
        if d < reach && best.is_none_or(|(b, _)| d < b) {
            *best = Some((d, p));
        }
    }
    // Ends and crossings first.
    for o in &others {
        let reach = NODE_REACH.max(o.section.width() / 2.0).max(half);
        for e in [o.axis[0], o.axis[o.axis.len() - 1]] {
            take(&mut best, e.distance(to), e, reach);
        }
    }
    for j in shared_junctions(model, r.level).iter() {
        if j.ring.is_some() || j.arms.len() < 3 || j.arms.iter().any(|a| a.road == road) {
            continue;
        }
        take(&mut best, j.at.distance(to), j.at, NODE_REACH.max(half));
    }
    if let Some((_, p)) = best {
        return p;
    }
    // Then, the nearest axis.
    for o in &others {
        let q = o.axis_at(o.station_of(to)).0;
        take(&mut best, q.distance(to), q, o.section.width() / 2.0 + 1.0);
    }
    best.map_or(to, |(_, p)| p)
}

/// The road's axis and its section as if it ran the other way: its strips
/// from right to left, its build-outs and crossings measured from its end.
fn reversed(r: &Road) -> (Vec<Point2>, archisim_core::RoadSection) {
    let len = r.length();
    let mut axis = r.axis.clone();
    axis.reverse();
    let mut sec = r.section.clone();
    sec.strips.reverse();
    for b in &mut sec.buildouts {
        b.left = !b.left;
        let (f, t) = (len - b.to, len - b.from);
        b.from = f;
        b.to = t;
    }
    for c in &mut sec.crossings {
        c.at = len - c.at;
    }
    (axis, sec)
}

/// Joins `road` with a street that ends where one of its ends is, if they
/// go on in line (less than [`JOIN_TURN`]) and are alike: the same
/// section and materials, both level and with nothing of their own along
/// them (changes of section, stretches, hidden pieces). Returns the road
/// that is left, if they were joined: `road`, longer.
pub fn join_ends(model: &mut Model, road: RoadId) -> Result<Option<RoadId>, ModelError> {
    let Some(a) = model.road(road).cloned() else {
        return Ok(None);
    };
    let plain = |r: &Road| {
        !r.free
            && r.changes.is_empty()
            && r.profile.is_empty()
            && r.skins.is_empty()
            && r.hidden.is_empty()
    };
    if !plain(&a) {
        return Ok(None);
    }
    let ends = |r: &Road| [r.axis[0], r.axis[r.axis.len() - 1]];
    let others: Vec<Road> = model.roads_on_level(a.level).cloned().collect();
    for b in &others {
        if b.id == road || !plain(b) {
            continue;
        }
        if (a.top_offset - b.top_offset).abs() > 1e-6
            || a.pavement != b.pavement
            || a.kerb_material != b.kerb_material
            || a.sidewalk_material != b.sidewalk_material
        {
            continue;
        }
        for (ia, pa) in ends(&a).into_iter().enumerate() {
            for (ib, pb) in ends(b).into_iter().enumerate() {
                if pa.distance(pb) > 0.05 {
                    continue;
                }
                // `a` running into the point, `b` out of it.
                let (axis_a, sec_a) = if ia == 0 {
                    reversed(&a)
                } else {
                    (a.axis.clone(), a.section.clone())
                };
                let (axis_b, sec_b) = if ib == 1 {
                    reversed(b)
                } else {
                    (b.axis.clone(), b.section.clone())
                };
                let bare = |s: &archisim_core::RoadSection| {
                    let mut s = s.clone();
                    s.buildouts.clear();
                    s.crossings.clear();
                    s
                };
                if bare(&sec_a) != bare(&sec_b) {
                    continue;
                }
                let n = axis_a.len();
                let din = (axis_a[n - 1] - axis_a[n - 2]).normalized();
                let dout = (axis_b[1] - axis_b[0]).normalized();
                let (Some(din), Some(dout)) = (din, dout) else {
                    continue;
                };
                if din.dot(dout) < JOIN_TURN.cos() {
                    continue;
                }
                // Not where a third street meets them.
                if node_roads(model, road, if ia == 0 { 0.0 } else { a.length() }).len() > 2 {
                    continue;
                }
                let la: f64 = axis_a.windows(2).map(|w| w[0].distance(w[1])).sum();
                let mut axis = axis_a;
                axis.extend(axis_b.iter().skip(1).copied());
                let mut sec = sec_a;
                for mut x in sec_b.buildouts {
                    x.from += la;
                    x.to += la;
                    sec.buildouts.push(x);
                }
                for mut x in sec_b.crossings {
                    x.at += la;
                    sec.crossings.push(x);
                }
                let other = b.id;
                model.remove_road(other)?;
                model.edit_road(road, |r| {
                    r.axis = axis;
                    r.section = sec;
                })?;
                return Ok(Some(road));
            }
        }
    }
    Ok(None)
}

/// Half the width of the widest street of a crossing.
fn half(model: &Model, j: &Junction) -> f64 {
    j.arms
        .iter()
        .filter_map(|a| model.road(a.road))
        .map(|r| r.section.width() / 2.0)
        .fold(0.0, f64::max)
}

/// Whether two crossings are too near for both to be built: they would
/// overlap (closer than their half widths) or one is inside a
/// roundabout's circle.
pub fn too_near(model: &Model, a: &Junction, b: &Junction) -> bool {
    let room = match b.ring.and_then(|r| model.roundabout(r)) {
        Some(r) => r.radius + half(model, a) * 0.9,
        None => match a.ring.and_then(|r| model.roundabout(r)) {
            Some(r) => r.radius + half(model, b) * 0.9,
            None => (half(model, a) + half(model, b)) * 0.9,
        },
    };
    a.at.distance(b.at) < room
}

/// Whether any crossing of the streets `roads` is too near another one
/// ([`too_near`]).
pub fn crowded(model: &Model, roads: &[RoadId]) -> bool {
    let Some(level) = roads.first().and_then(|r| model.road(*r)).map(|r| r.level) else {
        return false;
    };
    let js = shared_junctions(model, level);
    js.iter().enumerate().any(|(i, a)| {
        a.ring.is_none()
            && a.arms.iter().any(|x| roads.contains(&x.road))
            && js
                .iter()
                .enumerate()
                .any(|(k, b)| k != i && too_near(model, a, b))
    })
}

/// Fuses the crossings of `road` that crowd another crossing into it: the
/// road bends (between its neighbouring nodes) so that it meets the other
/// streets there, in one crossing. Returns how many it fused.
pub fn fuse_crowded(model: &mut Model, road: RoadId) -> Result<usize, ModelError> {
    let mut n = 0;
    for _ in 0..4 {
        let Some(r) = model.road(road) else {
            break;
        };
        if r.free {
            break;
        }
        let js = junctions(model, r.level);
        let mut pull = None;
        'find: for a in &js {
            let Some(arm) = a.arms.iter().find(|x| x.road == road) else {
                continue;
            };
            if a.ring.is_some() {
                continue;
            }
            for b in &js {
                if std::ptr::eq(a, b)
                    || b.ring.is_some()
                    || b.arms.iter().any(|x| x.road == road)
                    || !too_near(model, a, b)
                {
                    continue;
                }
                // Its other crossings with the streets it is fused with
                // go: it meets them there, not twice.
                let theirs: Vec<RoadId> = b.arms.iter().map(|x| x.road).collect();
                let keep: Vec<f64> = js
                    .iter()
                    .filter(|j| {
                        j.ring.is_some() || !j.arms.iter().any(|x| theirs.contains(&x.road))
                    })
                    .flat_map(|j| j.arms.iter().filter(|x| x.road == road).map(|x| x.station))
                    .collect();
                let len = r.length();
                let from = keep
                    .iter()
                    .copied()
                    .filter(|&n| n < arm.station - 0.5)
                    .fold(0.0, f64::max);
                let to = keep
                    .iter()
                    .copied()
                    .filter(|&n| n > arm.station + 0.5)
                    .fold(len, f64::min);
                pull = Some((from, arm.station, to, b.at - a.at));
                break 'find;
            }
        }
        let Some((from, s, to, d)) = pull else {
            break;
        };
        let axis = shape_axis(&r.axis, from, s, to, d, 2.0);
        model.edit_road(road, |r| r.axis = axis)?;
        n += 1;
    }
    Ok(n)
}

/// The point of the edge of `road` (its left or right outer edge) at `s`
/// along it.
pub fn edge_point(r: &Road, s: f64, left: bool) -> Point2 {
    let (p, dir) = r.axis_at(s);
    let n = dir.normalized().unwrap_or(Vec2::new(1.0, 0.0)).perp_left();
    let (l, rr) = r.extent_at(s);
    if left {
        p + n * l
    } else {
        p - n * rr
    }
}

/// The stretch of `road` between the nodes round `s` made `delta` metres
/// wider (narrower, if negative) on its left or right ([`Road::widened`]).
pub fn widen_stretch(
    model: &mut Model,
    road: RoadId,
    s: f64,
    left: bool,
    delta: f64,
) -> Result<(), ModelError> {
    let (from, to) = nodes_around(model, road, s);
    let Some(r) = model.road(road) else {
        return Err(ModelError::UnknownElement(road.0));
    };
    let Some(changes) = r.widened(from, to, left, delta) else {
        return Err(ModelError::InvalidShape(
            "la calle no puede ser tan estrecha ahí",
        ));
    };
    model.edit_road(road, |r| r.changes = changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, LevelId, NewRoad, RoadSection};

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn town() -> (Model, LevelId) {
        let mut m = Model::new("Calles");
        let level = m.add_level("Planta baja", 0.0);
        m.add_material("Asfalto", "asphalt", [70, 70, 70], CutPattern::Solid);
        (m, level)
    }

    fn road(m: &mut Model, level: LevelId, axis: Vec<Point2>) -> RoadId {
        let mat = m.materials().next().unwrap().id;
        m.add_road(NewRoad {
            name: "Calle".into(),
            level,
            axis,
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

    #[test]
    fn a_loose_end_is_pulled_onto_the_street_near_it() {
        let (mut m, lv) = town();
        let a = road(&mut m, lv, vec![p(0.0, 0.0), p(100.0, 0.0)]);
        let b = road(&mut m, lv, vec![p(50.0, 60.0), p(50.0, 25.0)]);
        let len = m.road(b).unwrap().length();
        assert!(is_loose_end(&m, b, len));
        assert!(!is_loose_end(&m, a, 50.0));
        // Dropped on the side of the other street: onto its axis.
        let to = pull_end(&m, b, p(51.0, 3.0));
        assert!(to.distance(p(51.0, 0.0)) < 1e-6, "{to:?}");
        // Near its end: onto the end.
        let to = pull_end(&m, b, p(97.0, 4.0));
        assert!(to.distance(p(100.0, 0.0)) < 1e-6, "{to:?}");
        // Far from everything: where it was dropped.
        assert_eq!(pull_end(&m, b, p(50.0, 40.0)), p(50.0, 40.0));
        // Moved there, it meets the street: a T.
        bend_road(&mut m, b, len, p(51.0, 0.0) - p(50.0, 25.0)).unwrap();
        assert!(!is_loose_end(&m, b, m.road(b).unwrap().length()));
        assert_eq!(junctions(&m, lv).len(), 1);
    }

    #[test]
    fn streets_in_line_join_and_a_corner_does_not() {
        let (mut m, lv) = town();
        let a = road(&mut m, lv, vec![p(0.0, 0.0), p(50.0, 0.0)]);
        // Drawn the other way round: it still joins.
        let b = road(&mut m, lv, vec![p(100.0, 8.0), p(50.0, 0.0)]);
        assert_eq!(join_ends(&mut m, a).unwrap(), Some(a));
        assert!(m.road(b).is_none());
        let r = m.road(a).unwrap();
        assert!((r.length() - 50.0 - 8f64.hypot(50.0)).abs() < 1e-6);
        assert_eq!(r.axis[0], p(0.0, 0.0));
        // A corner of 90° stays two streets.
        let c = road(&mut m, lv, vec![p(0.0, 0.0), p(0.0, -40.0)]);
        assert_eq!(join_ends(&mut m, a).unwrap(), None);
        assert!(m.road(c).is_some());
    }

    #[test]
    fn crossings_too_near_are_fused_into_one() {
        let (mut m, lv) = town();
        road(&mut m, lv, vec![p(0.0, 0.0), p(120.0, 0.0)]);
        road(&mut m, lv, vec![p(40.0, -60.0), p(40.0, 60.0)]);
        let c = road(&mut m, lv, vec![p(20.0, -60.0), p(74.0, 60.0)]);
        assert!(crowded(&m, &[c]));
        assert_eq!(fuse_crowded(&mut m, c).unwrap(), 1);
        assert!(!crowded(&m, &[c]));
        let js = junctions(&m, lv);
        assert_eq!(js.len(), 1);
        assert!(js[0].at.distance(p(40.0, 0.0)) < 0.5, "{:?}", js[0].at);
        // Its ends stay where they were.
        let r = m.road(c).unwrap();
        assert_eq!(r.axis[0], p(20.0, -60.0));
        assert_eq!(*r.axis.last().unwrap(), p(74.0, 60.0));
    }

    #[test]
    fn a_stretch_is_widened_between_its_nodes() {
        let (mut m, lv) = town();
        let a = road(&mut m, lv, vec![p(0.0, 0.0), p(120.0, 0.0)]);
        road(&mut m, lv, vec![p(40.0, -60.0), p(40.0, 60.0)]);
        let r = m.road(a).unwrap().clone();
        let before = edge_point(&r, 80.0, true);
        widen_stretch(&mut m, a, 80.0, true, 3.0).unwrap();
        let r = m.road(a).unwrap();
        assert!((edge_point(r, 80.0, true).y - before.y - 3.0).abs() < 1e-9);
        // Not before the crossing.
        assert!((edge_point(r, 20.0, true).y - before.y).abs() < 1e-9);
        assert!(widen_stretch(&mut m, a, 80.0, true, -60.0).is_err());
    }
}
