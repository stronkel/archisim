//! The longitudinal profile of a road («rasante», U1): the height of the
//! carriageway on its axis at points along it, straight grades between
//! them and, at each point, a parabolic vertical curve («acuerdo») of a
//! length, as in road design. With no points the road is level at its
//! `top_offset`; with them, its height at a distance along the axis is
//! `top_offset` plus the profile's height there.
//!
//! The pieces of a road are cut where the profile turns (and every few
//! metres inside a curve) so that each is a plane: [`Road::profile_cuts`].
//! Points of a piece take their height from where they fall on the axis,
//! [`Road::station_of`] and [`Road::top_at`].

use super::*;

/// A point of a road's profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfilePoint {
    /// Distance along the axis from its start, metres.
    pub at: f64,
    /// Height of the carriageway on the axis there, over the road's
    /// `top_offset`.
    pub z: f64,
    /// Length of the parabolic vertical curve centred there (0: a sharp
    /// change of grade). Ignored at the ends.
    pub curve: f64,
}

/// How long the stretches of a vertical curve are when it is cut into
/// planes.
pub const CURVE_STEP: f64 = 2.5;

impl Road {
    /// The profile's points in order along the axis (copied, sorted).
    fn sorted_profile(&self) -> Vec<ProfilePoint> {
        let mut p = self.profile.clone();
        p.sort_by(|a, b| a.at.total_cmp(&b.at));
        p.dedup_by(|a, b| (a.at - b.at).abs() < 1e-6);
        p
    }

    /// The vertical curves that fit: each point's curve made shorter if it
    /// would run past the middle of its neighbouring grades. (start,
    /// vertex, end, grade in, grade out).
    fn curves(&self) -> Vec<(f64, ProfilePoint, f64, f64, f64)> {
        let p = self.sorted_profile();
        let mut out = Vec::new();
        for i in 1..p.len().saturating_sub(1) {
            let (a, v, b) = (p[i - 1], p[i], p[i + 1]);
            let (la, lb) = (v.at - a.at, b.at - v.at);
            if la < EPSILON || lb < EPSILON || v.curve <= EPSILON {
                continue;
            }
            let half = (v.curve / 2.0).min(la / 2.0).min(lb / 2.0);
            let (g0, g1) = ((v.z - a.z) / la, (b.z - v.z) / lb);
            out.push((v.at - half, v, v.at + half, g0, g1));
        }
        out
    }

    /// Height of the profile at `s` along the axis (0 with no profile).
    pub fn profile_at(&self, s: f64) -> f64 {
        let p = self.sorted_profile();
        match p.len() {
            0 => return 0.0,
            1 => return p[0].z,
            _ => {}
        }
        // Inside a vertical curve: the parabola tangent to both grades.
        for (s0, v, s1, g0, g1) in self.curves() {
            if s > s0 && s < s1 {
                let x = s - s0;
                let z0 = v.z - g0 * (v.at - s0);
                return z0 + g0 * x + (g1 - g0) * x * x / (2.0 * (s1 - s0));
            }
        }
        // On a straight grade (and level beyond the ends).
        if s <= p[0].at {
            return p[0].z;
        }
        for w in p.windows(2) {
            if s <= w[1].at {
                let t = (s - w[0].at) / (w[1].at - w[0].at).max(EPSILON);
                return w[0].z + (w[1].z - w[0].z) * t;
            }
        }
        p[p.len() - 1].z
    }

    /// Height of the carriageway on the axis at `s`, over the road's level.
    pub fn top_at(&self, s: f64) -> f64 {
        self.top_offset + self.profile_at(s)
    }

    /// Whether the road has a profile (it is not level).
    pub fn sloped(&self) -> bool {
        let p = self.sorted_profile();
        p.windows(2).any(|w| (w[0].z - w[1].z).abs() > 1e-6)
    }

    /// The grades between the profile's points: (from, to, grade as a
    /// fraction, positive uphill along the axis).
    pub fn grades(&self) -> Vec<(f64, f64, f64)> {
        self.sorted_profile()
            .windows(2)
            .filter(|w| w[1].at - w[0].at > EPSILON)
            .map(|w| (w[0].at, w[1].at, (w[1].z - w[0].z) / (w[1].at - w[0].at)))
            .collect()
    }

    /// Where the pieces of the road are cut so that each is a plane: the
    /// corners of its axis and, with a profile, its points and every
    /// [`CURVE_STEP`] inside its vertical curves.
    pub fn profile_cuts(&self) -> Vec<f64> {
        if !self.sloped() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut s = 0.0;
        for w in self.axis.windows(2) {
            s += w[0].distance(w[1]);
            out.push(s);
        }
        out.pop();
        for p in self.sorted_profile() {
            out.push(p.at);
        }
        for (s0, _, s1, _, _) in self.curves() {
            let n = ((s1 - s0) / CURVE_STEP).ceil().max(1.0) as usize;
            for k in 0..=n {
                out.push(s0 + (s1 - s0) * k as f64 / n as f64);
            }
        }
        let len = self.length();
        out.retain(|s| *s > 1e-6 && *s < len - 1e-6);
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        out
    }

    /// The point of the axis at `s` along it and its direction there.
    pub fn axis_at(&self, s: f64) -> (Point2, Vec2) {
        super::urban::axis_point(&self.axis, s)
    }

    /// The distance along the axis of the point of the axis nearest to `p`.
    pub fn station_of(&self, p: Point2) -> f64 {
        let mut best = (f64::INFINITY, 0.0);
        let mut s = 0.0;
        for w in self.axis.windows(2) {
            let (a, b) = (w[0], w[1]);
            let l = a.distance(b);
            let t = if l > EPSILON {
                ((p - a).dot(b - a) / (l * l)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let q = a + (b - a) * t;
            let d = q.distance(p);
            if d < best.0 {
                best = (d, s + l * t);
            }
            s += l;
        }
        best.1
    }

    /// Height of the carriageway at a point of the road in plan, over the
    /// road's level: the profile where the point falls on the axis (the
    /// section has no crossfall).
    pub fn top_under(&self, p: Point2) -> f64 {
        if self.profile.is_empty() {
            return self.top_offset;
        }
        self.top_at(self.station_of(p))
    }

    /// The profile's text, for IFC: `at z curve;…`.
    pub fn encode_profile(&self) -> String {
        self.sorted_profile()
            .iter()
            .map(|p| format!("{} {} {}", p.at, p.z, p.curve))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// A profile of [`Road::encode_profile`]'s text.
    pub fn decode_profile(text: &str) -> Option<Vec<ProfilePoint>> {
        let mut out = Vec::new();
        for x in text.split(';').filter(|x| !x.trim().is_empty()) {
            let v: Vec<f64> = x
                .split_whitespace()
                .map(|t| t.parse::<f64>().ok())
                .collect::<Option<_>>()?;
            if v.len() != 3 {
                return None;
            }
            out.push(ProfilePoint {
                at: v[0],
                z: v[1],
                curve: v[2],
            });
        }
        Some(out)
    }
}

impl Model {
    /// A profile for a road taken from the terrain of its level under its
    /// axis: a point every `step` metres (and at both ends), each with a
    /// vertical curve of `step` metres, heights over the road's
    /// `top_offset` and rounded to the centimetre. None if there is no
    /// terrain under it.
    pub fn road_profile_from_terrain(&self, id: RoadId, step: f64) -> Option<Vec<ProfilePoint>> {
        let r = self.road(id)?;
        let len = r.length();
        let step = step.max(1.0);
        let n = (len / step).ceil().max(1.0) as usize;
        let terrains: Vec<&Terrain> = self.terrains_on_level(r.level).collect();
        let mut out = Vec::new();
        for k in 0..=n {
            let s = len * k as f64 / n as f64;
            let p = super::urban::point_along(&r.axis, s);
            let Some(z) = terrains.iter().find_map(|t| t.height_at(p)) else {
                continue;
            };
            out.push(ProfilePoint {
                at: (s * 100.0).round() / 100.0,
                z: ((z - r.top_offset) * 100.0).round() / 100.0,
                curve: if k == 0 || k == n { 0.0 } else { step },
            });
        }
        (out.len() >= 2).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn road(profile: Vec<ProfilePoint>) -> Road {
        Road {
            id: RoadId(1),
            guid: GlobalId::new(),
            name: "R".into(),
            level: LevelId(0),
            axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
            section: RoadSection::default(),
            top_offset: 0.5,
            profile,
            changes: Vec::new(),
            pavement: MaterialId(0),
            kerb_material: MaterialId(0),
            sidewalk_material: MaterialId(0),
        }
    }

    fn pt(at: f64, z: f64, curve: f64) -> ProfilePoint {
        ProfilePoint { at, z, curve }
    }

    #[test]
    fn grades_and_a_vertical_curve() {
        let r = road(vec![
            pt(0.0, 0.0, 0.0),
            pt(50.0, 2.0, 20.0),
            pt(100.0, 1.0, 0.0),
        ]);
        assert!(r.sloped());
        let g = r.grades();
        assert!((g[0].2 - 0.04).abs() < 1e-12 && (g[1].2 + 0.02).abs() < 1e-12);
        // On the straight grades.
        assert!((r.top_at(20.0) - (0.5 + 0.8)).abs() < 1e-9);
        assert!((r.top_at(80.0) - (0.5 + 1.4)).abs() < 1e-9);
        // The curve is tangent at its ends and lower than the vertex.
        assert!((r.profile_at(40.0) - 1.6).abs() < 1e-9);
        assert!((r.profile_at(60.0) - 1.8).abs() < 1e-9);
        let top = r.profile_at(50.0);
        assert!(top < 2.0 && top > 1.6, "{top}");
        // A crest: the middle ordinate is (g1 − g0)·L/8 below the vertex.
        assert!((top - (2.0 - 0.06 * 20.0 / 8.0)).abs() < 1e-9, "{top}");
        // Cut at the vertex, at the curve's ends and inside it.
        let cuts = r.profile_cuts();
        assert!(cuts.contains(&40.0) && cuts.contains(&50.0) && cuts.contains(&60.0));
        assert!(cuts.len() >= 9, "{cuts:?}");
        // A point beside the axis takes the axis's height there.
        assert!((r.top_under(Point2::new(20.0, 4.0)) - 1.3).abs() < 1e-9);
        // Its text, back.
        let back = Road::decode_profile(&r.encode_profile()).unwrap();
        assert_eq!(back.len(), 3);
        assert_eq!(back[1], pt(50.0, 2.0, 20.0));
    }

    #[test]
    fn a_level_road_has_no_cuts() {
        let r = road(Vec::new());
        assert!(!r.sloped());
        assert!(r.profile_cuts().is_empty());
        assert!((r.top_under(Point2::new(30.0, 2.0)) - 0.5).abs() < 1e-12);
    }
}
