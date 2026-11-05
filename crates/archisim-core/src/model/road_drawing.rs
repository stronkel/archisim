//! Drawing streets stretch by stretch, as in a city builder: each click
//! lays one stretch (straight or curved) of the street being drawn, which
//! goes on from the last one; a stretch that starts at the free end of a
//! street of the same section lengthens it, and one that starts or ends on
//! another street makes a crossing there (the crossings are not stored:
//! `archisim_geom::crossroads` finds them). The street follows the ground
//! with its grade kept under a limit, smoothed by vertical curves.

use super::*;

/// One stretch to lay.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadStretch {
    pub level: LevelId,
    /// Its axis: a straight stretch is two points, a curve many.
    pub points: Vec<Point2>,
    /// The street it goes on from, at its end (`true`) or its start.
    pub continues: Option<(RoadId, bool)>,
    pub section: RoadSection,
    pub top_offset: f64,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
    /// Following the ground, with the steepest grade allowed (0.08 is 8 %).
    pub follow_ground: Option<f64>,
}

/// Points of a curve from `a` to `b` pulled towards `control` (a
/// quadratic Bézier), one every `step` metres or so.
pub fn curve_points(a: Point2, control: Point2, b: Point2, step: f64) -> Vec<Point2> {
    let len = a.distance(control) + control.distance(b);
    let n = ((len / step.max(0.5)).ceil() as usize).clamp(2, 200);
    (0..=n)
        .map(|k| {
            let t = k as f64 / n as f64;
            let u = 1.0 - t;
            Point2::new(
                u * u * a.x + 2.0 * u * t * control.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * control.y + t * t * b.y,
            )
        })
        .collect()
}

/// A free end of a street near `p` (within `reach`): the street and
/// whether it is its end (`true`) or its start, and the end's point.
pub fn road_end_near(
    model: &Model,
    level: LevelId,
    p: Point2,
    reach: f64,
) -> Option<(RoadId, bool, Point2)> {
    let mut best: Option<(f64, RoadId, bool, Point2)> = None;
    for r in model.roads_on_level(level) {
        let (Some(first), Some(last)) = (r.axis.first(), r.axis.last()) else {
            continue;
        };
        for (at_end, q) in [(false, *first), (true, *last)] {
            let d = q.distance(p);
            if d <= reach && best.is_none_or(|b| d < b.0) {
                best = Some((d, r.id, at_end, q));
            }
        }
    }
    best.map(|(_, id, e, q)| (id, e, q))
}

/// The nearest point on the axis of a street near `p` (within the half
/// width of its carriageway and sidewalks, plus `reach`).
pub fn road_axis_near(
    model: &Model,
    level: LevelId,
    p: Point2,
    reach: f64,
) -> Option<(RoadId, Point2)> {
    let mut best: Option<(f64, RoadId, Point2)> = None;
    for r in model.roads_on_level(level) {
        let half = r.section.width() / 2.0 + reach;
        for w in r.axis.windows(2) {
            let q = closest_on_segment(w[0], w[1], p);
            let d = q.distance(p);
            if d <= half && best.is_none_or(|b| d < b.0) {
                best = Some((d, r.id, q));
            }
        }
    }
    best.map(|(_, id, q)| (id, q))
}

fn closest_on_segment(a: Point2, b: Point2, p: Point2) -> Point2 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return a;
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    a + ab * t
}

/// The direction a street leaves its end (`true`) or its start, outwards.
pub fn road_end_direction(model: &Model, id: RoadId, at_end: bool) -> Option<Vec2> {
    let r = model.road(id)?;
    let n = r.axis.len();
    if n < 2 {
        return None;
    }
    let (from, to) = if at_end {
        (r.axis[n - 2], r.axis[n - 1])
    } else {
        (r.axis[1], r.axis[0])
    };
    (to - from).normalized()
}

/// Heights of the ground along a polyline, every `step` metres: station
/// and height, where there is ground.
fn ground_along(model: &Model, level: LevelId, axis: &[Point2], step: f64) -> Vec<(f64, f64)> {
    let len: f64 = axis.windows(2).map(|w| w[0].distance(w[1])).sum();
    let n = (len / step.max(1.0)).ceil().max(1.0) as usize;
    let terrains: Vec<&Terrain> = model.terrains_on_level(level).collect();
    (0..=n)
        .filter_map(|k| {
            let s = len * k as f64 / n as f64;
            let p = super::urban::point_along(axis, s);
            terrains.iter().find_map(|t| t.height_at(p)).map(|z| (s, z))
        })
        .collect()
}

/// Heights along a street that keep its grade under `max_grade`, as close
/// to `ground` as that allows (each point raised or lowered least): a
/// pass forwards and one backwards bound each point by its neighbours.
pub fn graded_heights(ground: &[(f64, f64)], max_grade: f64) -> Vec<(f64, f64)> {
    let g = max_grade.max(0.005);
    let mut z: Vec<f64> = ground.iter().map(|p| p.1).collect();
    // The ground smoothed first: a street does not follow every bump.
    if z.len() > 2 {
        let raw = z.clone();
        for i in 1..z.len() - 1 {
            z[i] = (raw[i - 1] + 2.0 * raw[i] + raw[i + 1]) / 4.0;
        }
    }
    for _ in 0..4 {
        for i in 1..z.len() {
            let ds = ground[i].0 - ground[i - 1].0;
            z[i] = z[i].clamp(z[i - 1] - g * ds, z[i - 1] + g * ds);
        }
        for i in (0..z.len().saturating_sub(1)).rev() {
            let ds = ground[i + 1].0 - ground[i].0;
            z[i] = z[i].clamp(z[i + 1] - g * ds, z[i + 1] + g * ds);
        }
    }
    ground.iter().zip(z).map(|(p, z)| (p.0, z)).collect()
}

/// Spacing of the points of the profile of a street that follows the
/// ground, metres.
pub const PROFILE_STEP: f64 = 10.0;

/// The zebra crossings laid at the junctions: their width along the
/// street and how far back from the edge of the street crossed.
pub const ZEBRA_WIDTH: f64 = 4.0;
pub const ZEBRA_SETBACK: f64 = 1.5;

/// Whether a section has sidewalks with kerbs to cross between (not a
/// road without them, nor a pedestrian street).
fn crossable(s: &RoadSection) -> bool {
    s.strips.iter().any(|x| x.band == RoadBand::Carriageway)
        && s.strips.iter().any(|x| x.band == RoadBand::Sidewalk)
}

impl Model {
    /// The profile of a street that follows the ground with its grade
    /// under `max_grade` (none if there is no ground under it).
    pub fn road_profile_following(&self, id: RoadId, max_grade: f64) -> Option<Vec<ProfilePoint>> {
        let r = self.road(id)?;
        let ground = ground_along(self, r.level, &r.axis, PROFILE_STEP);
        if ground.len() < 2 {
            return None;
        }
        let graded = graded_heights(&ground, max_grade);
        let n = graded.len();
        Some(
            graded
                .into_iter()
                .enumerate()
                .map(|(k, (s, z))| ProfilePoint {
                    at: (s * 100.0).round() / 100.0,
                    z: ((z - r.top_offset) * 100.0).round() / 100.0,
                    curve: if k == 0 || k + 1 == n {
                        0.0
                    } else {
                        PROFILE_STEP
                    },
                })
                .collect(),
        )
    }

    /// Zebra crossings at the junction at `p`: on every street of `level`
    /// that passes through it (or ends at it), on each of its arms, set
    /// back from the edge of the widest of the others. Streets without
    /// sidewalks get none; where there is one already, none more. Returns
    /// how many it laid.
    pub fn crossings_at(&mut self, level: LevelId, p: Point2) -> Result<usize, ModelError> {
        let meeting: Vec<(RoadId, f64, f64, f64)> = self
            .roads_on_level(level)
            .filter_map(|r| {
                let s = r.station_of(p);
                (r.axis_at(s).0.distance(p) < 0.6)
                    .then(|| (r.id, s, r.length(), r.section.width() / 2.0))
            })
            .collect();
        if meeting.len() < 2 {
            return Ok(0);
        }
        let mut laid = 0;
        for &(id, s0, len, _) in &meeting {
            let other = meeting
                .iter()
                .filter(|m| m.0 != id)
                .map(|m| m.3)
                .fold(0.0, f64::max);
            let off = other + ZEBRA_SETBACK + ZEBRA_WIDTH / 2.0;
            let Some(r) = self.road(id) else { continue };
            if !crossable(&r.section) {
                continue;
            }
            let wanted: Vec<f64> = [s0 - off, s0 + off]
                .into_iter()
                .filter(|a| *a > ZEBRA_WIDTH / 2.0 && *a < len - ZEBRA_WIDTH / 2.0)
                .filter(|a| r.section.crossings.iter().all(|c| (c.at - a).abs() > 3.0))
                .collect();
            if wanted.is_empty() {
                continue;
            }
            laid += wanted.len();
            self.edit_road(id, |r| {
                for at in wanted {
                    r.section.crossings.push(Crossing {
                        at: (at * 100.0).round() / 100.0,
                        width: ZEBRA_WIDTH,
                    });
                }
            })?;
        }
        Ok(laid)
    }

    /// Lays a stretch: lengthens the street it goes on from (if it has the
    /// same section) or makes a new one. Returns the street. Its section
    /// gets the white road paint if it has none (lane lines and zebras).
    pub fn add_road_stretch(&mut self, s: RoadStretch) -> Result<RoadId, ModelError> {
        let mut s = s;
        if s.section.markings.is_none()
            && s.section
                .strips
                .iter()
                .any(|x| x.band == RoadBand::Carriageway)
        {
            s.section.markings = Some(self.marking_material([245, 245, 240]));
        }
        let mut pts: Vec<Point2> = Vec::with_capacity(s.points.len());
        for p in &s.points {
            if pts.last().is_none_or(|l: &Point2| l.distance(*p) > 0.05) {
                pts.push(*p);
            }
        }
        if pts.len() < 2 {
            return Err(ModelError::NonPositive("stretch length"));
        }
        // The same section but for its crossings, which are the street's.
        let same = s.continues.filter(|(id, _)| {
            self.road(*id).is_some_and(|r| {
                let mut a = r.section.clone();
                a.crossings.clear();
                let mut b = s.section.clone();
                b.crossings.clear();
                a == b && r.changes.is_empty()
            })
        });
        let id = match same {
            Some((id, at_end)) => {
                self.edit_road(id, |r| {
                    if at_end {
                        // The first point is the end it goes on from.
                        r.axis.extend(pts.iter().skip(1).copied());
                    } else {
                        let mut a: Vec<Point2> = pts.iter().skip(1).rev().copied().collect();
                        // What was there moves along by what is added.
                        let added: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
                        for c in &mut r.section.crossings {
                            c.at += added;
                        }
                        a.extend(r.axis.iter().copied());
                        r.axis = a;
                        // Its stations moved: the profile is made again
                        // below, or dropped.
                        r.profile.clear();
                    }
                })?;
                id
            }
            None => self.add_road(NewRoad {
                name: self.next_road_name(),
                level: s.level,
                axis: pts,
                section: s.section.clone(),
                top_offset: s.top_offset,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: s.pavement,
                kerb_material: s.kerb_material,
                sidewalk_material: s.sidewalk_material,
            })?,
        };
        if let Some(g) = s.follow_ground {
            let profile = self.road_profile_following(id, g).unwrap_or_default();
            self.edit_road(id, |r| r.profile = profile)?;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with_slope() -> (Model, LevelId, MaterialId) {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let earth = m.add_material("Tierra", "earth", [120, 100, 80], CutPattern::Solid);
        // Rising 20 m in 100 m to the east: 20 %.
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        m.add_terrain(NewTerrain {
            level: l,
            points: vec![
                sp(-10.0, -50.0, -2.0),
                sp(110.0, -50.0, 22.0),
                sp(110.0, 50.0, 22.0),
                sp(-10.0, 50.0, -2.0),
            ],
            material: earth,
            depth: 1.0,
            contour: 1.0,
            lines: Vec::new(),
        })
        .unwrap();
        (m, l, earth)
    }

    fn stretch(l: LevelId, mat: MaterialId, pts: Vec<Point2>) -> RoadStretch {
        RoadStretch {
            level: l,
            points: pts,
            continues: None,
            section: RoadSection::default(),
            top_offset: 0.0,
            pavement: mat,
            kerb_material: mat,
            sidewalk_material: mat,
            follow_ground: Some(0.08),
        }
    }

    #[test]
    fn stretches_go_on_and_follow_the_ground() {
        let (mut m, l, mat) = model_with_slope();
        let a = m
            .add_road_stretch(stretch(
                l,
                mat,
                vec![Point2::new(0.0, 0.0), Point2::new(50.0, 0.0)],
            ))
            .unwrap();
        // The next stretch goes on from its end: the same street.
        let (id, at_end, q) = road_end_near(&m, l, Point2::new(50.5, 0.4), 2.0).unwrap();
        assert_eq!((id, at_end), (a, true));
        let mut s = stretch(
            l,
            mat,
            curve_points(q, Point2::new(80.0, 0.0), Point2::new(90.0, 20.0), 3.0),
        );
        s.continues = Some((id, at_end));
        let b = m.add_road_stretch(s).unwrap();
        assert_eq!(a, b);
        let r = m.road(a).unwrap();
        assert!(r.length() > 90.0, "{}", r.length());
        assert_eq!(m.roads().count(), 1);
        // The ground climbs at 20 %; the street at no more than 8 %.
        let grades: Vec<f64> = r
            .profile
            .windows(2)
            .map(|w| (w[1].z - w[0].z) / (w[1].at - w[0].at))
            .collect();
        assert!(!grades.is_empty());
        assert!(grades.iter().all(|g| *g <= 0.081), "{grades:?}");
        // Over the middle it stays near the ground (cut at the top, fill at
        // the bottom), never far off.
        let mid = r.profile[r.profile.len() / 2];
        let ground = -2.0 + (super::super::urban::point_along(&r.axis, mid.at).x + 10.0) * 0.2;
        assert!((mid.z - ground).abs() < 6.0, "{} {ground}", mid.z);
        // A stretch of another section starts a street of its own.
        let mut other = stretch(l, mat, vec![Point2::new(0.0, 0.0), Point2::new(0.0, 30.0)]);
        other.section.kerb_height += 0.02;
        other.continues = Some((a, false));
        let c = m.add_road_stretch(other).unwrap();
        assert_ne!(c, a);
        // The axis of a street, from its side.
        let (id, q) = road_axis_near(&m, l, Point2::new(25.0, 3.0), 0.5).unwrap();
        assert_eq!(id, a);
        assert!((q.y).abs() < 1e-9);
        assert!(road_end_direction(&m, a, true).is_some());
        // Painted: lane lines and zebras.
        assert!(m.road(a).unwrap().section.markings.is_some());
        // A street from the side of the first: zebras on its arms.
        let mut t = stretch(
            l,
            mat,
            vec![Point2::new(25.0, 0.0), Point2::new(25.0, -40.0)],
        );
        t.follow_ground = None;
        m.add_road_stretch(t).unwrap();
        let n = m.crossings_at(l, Point2::new(25.0, 0.0)).unwrap();
        assert!(n >= 2, "{n}");
        // Not twice.
        assert_eq!(m.crossings_at(l, Point2::new(25.0, 0.0)).unwrap(), 0);
    }

    #[test]
    fn the_grade_is_bound_both_ways() {
        let ground: Vec<(f64, f64)> = (0..=10)
            .map(|k| (k as f64 * 10.0, if k == 5 { 20.0 } else { 0.0 }))
            .collect();
        let z = graded_heights(&ground, 0.1);
        for w in z.windows(2) {
            assert!((w[1].1 - w[0].1).abs() <= 1.0 + 1e-9, "{z:?}");
        }
    }
}
