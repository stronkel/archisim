//! Stairs by flights and landings ("escaleras por tramos y mesetas"),
//! checked against the Spanish building code (CTE DB-SUA 1, 4.2).
//!
//! A stair is drawn by its walking line ("línea de huella"): a polyline
//! from the foot of the stair to its head. Each piece long enough for a
//! tread is a flight; at every turn there is a landing as wide as the
//! stair (in a U-shaped stair the short piece across is part of the
//! landing). The stair climbs from its level to the level above (or by a
//! height): the risers are all equal, the treads of each flight fill it
//! exactly, as near as possible to the going asked for.
//!
//! IFC: `IfcStair` made of `IfcStairFlight`s and landings (`IfcSlab`
//! `LANDING`).

use super::*;

/// What the stair serves, for the code checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairUse {
    /// Any use (CTE "uso general"): treads ≥ 28 cm, risers 13–18,5 cm,
    /// width ≥ 1 m, flights up to 2,25 m in public areas.
    #[default]
    General,
    /// Inside a dwelling (CTE "uso restringido"): treads ≥ 22 cm, risers
    /// ≤ 20 cm, width ≥ 0,80 m.
    Restricted,
}

/// A stair.
#[derive(Debug, Clone, PartialEq)]
pub struct Stair {
    pub id: StairId,
    pub guid: GlobalId,
    pub level: LevelId,
    /// The walking line, from the foot to the head (at least two points).
    pub path: Vec<Point2>,
    /// Clear width of flights and landings.
    pub width: f64,
    /// The tread depth aimed at ("huella"); each flight's treads are
    /// adjusted to fill it.
    pub going: f64,
    /// Height of the foot above the level.
    pub base_offset: f64,
    /// Up to a level, or by `height`.
    pub top: WallTop,
    /// Height climbed (worked out from `top` if it is tied to a level).
    pub height: f64,
    /// Thickness of the concrete slab under the steps and landings
    /// ("losa").
    pub waist: f64,
    pub material: MaterialId,
    pub usage: StairUse,
}

/// Data needed to create a stair.
#[derive(Debug, Clone, PartialEq)]
pub struct NewStair {
    pub level: LevelId,
    pub path: Vec<Point2>,
    pub width: f64,
    pub going: f64,
    pub base_offset: f64,
    pub top: WallTop,
    pub height: f64,
    pub waist: f64,
    pub material: MaterialId,
    pub usage: StairUse,
}

/// One flight: its walking line (from its first riser to its last), how
/// many treads and risers it has, how deep its treads are and the height
/// of its foot above the level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    pub start: Point2,
    pub end: Point2,
    pub treads: usize,
    pub risers: usize,
    pub going: f64,
    pub base: f64,
}

impl Flight {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    pub fn direction(&self) -> Vec2 {
        (self.end - self.start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0))
    }

    /// The outline in plan of the flight, `width` wide, counter-clockwise.
    pub fn outline(&self, width: f64) -> Vec<Point2> {
        let u = self.direction();
        let n = u.perp_left() * (width / 2.0);
        let mut v = vec![self.start - n, self.end - n, self.end + n, self.start + n];
        if signed_area(&v) < 0.0 {
            v.reverse();
        }
        v
    }
}

/// A landing: its outline in plan and the height of its top above the
/// level.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    pub outline: Vec<Point2>,
    pub top: f64,
}

/// How a stair is laid out: flights, landings and the riser height.
#[derive(Debug, Clone, PartialEq)]
pub struct StairLayout {
    pub flights: Vec<Flight>,
    pub landings: Vec<Landing>,
    pub riser: f64,
}

impl StairLayout {
    pub fn risers(&self) -> usize {
        self.flights.iter().map(|f| f.risers).sum()
    }

    pub fn treads(&self) -> usize {
        self.flights.iter().map(|f| f.treads).sum()
    }
}

/// What the code (CTE DB-SUA 1, 4.2) asks of a stair that this one does
/// not meet. The values are the stair's, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StairIssue {
    /// Treads shallower than the minimum (0,28 m; 0,22 m in a dwelling).
    TreadTooShort { going: f64, min: f64 },
    /// Risers outside 0,13–0,185 m (up to 0,20 m in a dwelling).
    RiserOutOfRange { riser: f64, min: f64, max: f64 },
    /// Two risers and a tread outside 0,54–0,70 m.
    StepRule { value: f64 },
    /// Narrower than 1,00 m (0,80 m in a dwelling).
    TooNarrow { width: f64, min: f64 },
    /// A flight of fewer than 3 risers.
    ShortFlight { risers: usize },
    /// A flight climbing more than 3,20 m (2,25 m in public areas).
    FlightTooHigh { height: f64, max: f64 },
}

/// Extreme values a stair may take at all (not the code's limits).
const MIN_WIDTH: f64 = 0.4;
const MIN_GOING: f64 = 0.1;

impl Stair {
    /// Flights, landings and riser height.
    pub fn layout(&self) -> StairLayout {
        layout(
            &self.path,
            self.width,
            self.going,
            self.height,
            self.base_offset,
        )
    }

    /// The outline of the whole stair in plan: flights and landings.
    pub fn outlines(&self) -> Vec<Vec<Point2>> {
        let l = self.layout();
        l.flights
            .iter()
            .map(|f| f.outline(self.width))
            .chain(l.landings.iter().map(|x| x.outline.clone()))
            .collect()
    }

    /// What the code asks that this stair does not meet.
    pub fn issues(&self) -> Vec<StairIssue> {
        let l = self.layout();
        let (min_going, min_riser, max_riser, min_width) = match self.usage {
            StairUse::General => (0.28, 0.13, 0.185, 1.0),
            StairUse::Restricted => (0.22, 0.0, 0.20, 0.8),
        };
        let mut v = Vec::new();
        let e = 1e-6;
        let going = l
            .flights
            .iter()
            .map(|f| f.going)
            .fold(f64::INFINITY, f64::min);
        if going.is_finite() && going < min_going - e {
            v.push(StairIssue::TreadTooShort {
                going,
                min: min_going,
            });
        }
        if l.riser < min_riser - e || l.riser > max_riser + e {
            v.push(StairIssue::RiserOutOfRange {
                riser: l.riser,
                min: min_riser,
                max: max_riser,
            });
        }
        if self.usage == StairUse::General && going.is_finite() {
            let rule = 2.0 * l.riser + going;
            if !(0.54 - e..=0.70 + e).contains(&rule) {
                v.push(StairIssue::StepRule { value: rule });
            }
        }
        if self.width < min_width - e {
            v.push(StairIssue::TooNarrow {
                width: self.width,
                min: min_width,
            });
        }
        for f in &l.flights {
            if f.risers < 3 {
                v.push(StairIssue::ShortFlight { risers: f.risers });
            }
            let h = f.risers as f64 * l.riser;
            if h > 3.2 + e {
                v.push(StairIssue::FlightTooHigh {
                    height: h,
                    max: 3.2,
                });
            }
        }
        v
    }
}

/// Lays out a stair along a walking line.
pub fn layout(path: &[Point2], width: f64, going: f64, height: f64, base: f64) -> StairLayout {
    let k = path.len();
    let half = width / 2.0;
    // Each piece of the walking line, shortened by half the width at the
    // turns (the landing takes it); those still long enough for a tread
    // are flights, the others belong to the landing.
    struct Piece {
        start: Point2,
        end: Point2,
        flight: bool,
    }
    let mut pieces = Vec::new();
    for i in 0..k.saturating_sub(1) {
        let (a, b) = (path[i], path[i + 1]);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let start = if i > 0 { a + u * half } else { a };
        let end = if i + 2 < k { b - u * half } else { b };
        let long = (end - start).dot(u) >= going * 0.999;
        pieces.push(Piece {
            start,
            end,
            flight: long,
        });
    }
    // Treads per flight, and the risers: one more than the treads per
    // flight (the last rises onto the landing or the floor).
    let mut flights: Vec<Flight> = pieces
        .iter()
        .filter(|p| p.flight)
        .map(|p| {
            let len = p.start.distance(p.end);
            let treads = ((len / going) + 1e-6).floor().max(1.0) as usize;
            Flight {
                start: p.start,
                end: p.end,
                treads,
                risers: treads + 1,
                going: len / treads as f64,
                base: 0.0,
            }
        })
        .collect();
    let risers: usize = flights.iter().map(|f| f.risers).sum();
    let riser = if risers > 0 {
        height / risers as f64
    } else {
        0.0
    };
    let mut z = base;
    for f in &mut flights {
        f.base = z;
        z += f.risers as f64 * riser;
    }
    // Landings: between consecutive flights, everything the walking line
    // turns through, as wide as the stair.
    let mut landings = Vec::new();
    let flight_pieces: Vec<usize> = (0..pieces.len()).filter(|i| pieces[*i].flight).collect();
    for (fi, w) in flight_pieces.windows(2).enumerate() {
        let (i, j) = (w[0], w[1]);
        let mut pts: Vec<Point2> = Vec::new();
        let edge = |p: Point2, u: Vec2| {
            let n = u.perp_left() * half;
            [p - n, p + n]
        };
        let ui = (pieces[i].end - pieces[i].start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0));
        let uj = (pieces[j].end - pieces[j].start)
            .normalized()
            .unwrap_or(Vec2::new(1.0, 0.0));
        pts.extend(edge(pieces[i].end, ui));
        pts.extend(edge(pieces[j].start, uj));
        // The square of the landing round every turn in between.
        for v in &path[i + 1..=j] {
            let n = ui.perp_left() * half;
            let t = ui * half;
            pts.extend([*v - t - n, *v + t - n, *v + t + n, *v - t + n]);
            let m = uj.perp_left() * half;
            let s = uj * half;
            pts.extend([*v - s - m, *v + s - m, *v + s + m, *v - s + m]);
        }
        let hull = convex_hull(&pts);
        if hull.len() >= 3 {
            landings.push(Landing {
                outline: hull,
                top: flights[fi + 1].base,
            });
        }
    }
    StairLayout {
        flights,
        landings,
        riser,
    }
}

/// Convex hull, counter-clockwise (monotone chain).
fn convex_hull(points: &[Point2]) -> Vec<Point2> {
    let mut p: Vec<Point2> = points.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup_by(|a, b| a.approx_eq(*b));
    if p.len() < 3 {
        return p;
    }
    let cross =
        |o: Point2, a: Point2, b: Point2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Point2> = Vec::new();
    for q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *q) <= 1e-12
        {
            lower.pop();
        }
        lower.push(*q);
    }
    let mut upper: Vec<Point2> = Vec::new();
    for q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *q) <= 1e-12
        {
            upper.pop();
        }
        upper.push(*q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

impl Model {
    fn stair_height(
        &self,
        level: LevelId,
        base: f64,
        top: WallTop,
        height: f64,
    ) -> Result<f64, ModelError> {
        match top {
            WallTop::Unconnected => Ok(height),
            WallTop::Level { level: to, offset } => {
                let z0 = self
                    .levels
                    .get(&level)
                    .ok_or(ModelError::UnknownLevel(level))?
                    .elevation;
                let z1 = self
                    .levels
                    .get(&to)
                    .ok_or(ModelError::UnknownLevel(to))?
                    .elevation;
                Ok(z1 + offset - z0 - base)
            }
        }
    }

    fn validate_stair(&self, s: &NewStair) -> Result<f64, ModelError> {
        if !self.levels.contains_key(&s.level) {
            return Err(ModelError::UnknownLevel(s.level));
        }
        if !self.materials.contains_key(&s.material) {
            return Err(ModelError::UnknownMaterial(s.material));
        }
        for p in &s.path {
            finite_point(*p, "stair path point")?;
        }
        for (v, what) in [
            (s.width, "stair width"),
            (s.going, "stair going"),
            (s.base_offset, "stair base offset"),
            (s.height, "stair height"),
            (s.waist, "stair waist"),
        ] {
            finite(v, what)?;
        }
        if s.width < MIN_WIDTH || s.going < MIN_GOING || s.waist <= EPSILON {
            return Err(ModelError::InvalidStair);
        }
        let height = self.stair_height(s.level, s.base_offset, s.top, s.height)?;
        if height <= 0.05 {
            return Err(ModelError::InvalidStair);
        }
        let l = layout(&s.path, s.width, s.going, height, s.base_offset);
        if s.path.len() < 2 || l.flights.is_empty() {
            return Err(ModelError::InvalidStair);
        }
        Ok(height)
    }

    pub fn add_stair(&mut self, spec: NewStair) -> Result<StairId, ModelError> {
        let height = self.validate_stair(&spec)?;
        let id = StairId(self.take_id());
        self.stairs.insert(
            id,
            Stair {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                path: spec.path,
                width: spec.width,
                going: spec.going,
                base_offset: spec.base_offset,
                top: spec.top,
                height,
                waist: spec.waist,
                material: spec.material,
                usage: spec.usage,
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a stair; nothing changes if the result is not valid.
    pub fn edit_stair(
        &mut self,
        id: StairId,
        edit: impl FnOnce(&mut Stair),
    ) -> Result<(), ModelError> {
        let mut s = self
            .stairs
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownStair(id))?;
        edit(&mut s);
        s.height = self.validate_stair(&spec_of(&s))?;
        self.stairs.insert(id, s);
        Ok(())
    }

    pub fn remove_stair(&mut self, id: StairId) -> Result<Stair, ModelError> {
        self.stairs.remove(&id).ok_or(ModelError::UnknownStair(id))
    }

    pub fn stair(&self, id: StairId) -> Option<&Stair> {
        self.stairs.get(&id)
    }

    pub fn stairs(&self) -> impl Iterator<Item = &Stair> {
        self.stairs.values()
    }

    pub fn stairs_on_level(&self, level: LevelId) -> impl Iterator<Item = &Stair> {
        self.stairs.values().filter(move |s| s.level == level)
    }

    /// Recomputes the heights of stairs tied to a level after levels
    /// change; those that would be left without height keep theirs.
    pub(super) fn refresh_stair_heights(&mut self) {
        let ids: Vec<StairId> = self.stairs.keys().copied().collect();
        for id in ids {
            let s = self.stairs[&id].clone();
            if let Ok(h) = self.stair_height(s.level, s.base_offset, s.top, s.height) {
                if h > 0.05 {
                    if let Some(x) = self.stairs.get_mut(&id) {
                        x.height = h;
                    }
                }
            }
        }
    }

    pub(super) fn restore_stairs(
        &mut self,
        stairs: Vec<Stair>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for mut s in stairs {
            let name = format!("stair {} ({})", s.id.0, s.guid);
            claim(s.id.0, Some(&s.guid))
                .and_then(|_| self.validate_stair(&spec_of(&s)))
                .map(|h| s.height = h)
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.stairs.insert(s.id, s);
        }
        Ok(())
    }
}

fn spec_of(s: &Stair) -> NewStair {
    NewStair {
        level: s.level,
        path: s.path.clone(),
        width: s.width,
        going: s.going,
        base_offset: s.base_offset,
        top: s.top,
        height: s.height,
        waist: s.waist,
        material: s.material,
        usage: s.usage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Model, LevelId, LevelId, MaterialId) {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        let p1 = m.add_level("P1", 2.97);
        let ha = m.add_material("HA", "concrete", [200, 200, 200], CutPattern::Solid);
        (m, pb, p1, ha)
    }

    fn spec(level: LevelId, top: LevelId, material: MaterialId, path: Vec<Point2>) -> NewStair {
        NewStair {
            level,
            path,
            width: 1.0,
            going: 0.28,
            base_offset: 0.0,
            top: WallTop::Level {
                level: top,
                offset: 0.0,
            },
            height: 0.0,
            waist: 0.15,
            material,
            usage: StairUse::General,
        }
    }

    /// A straight stair of 16 treads up to the next floor: 17 risers of
    /// 17,47 cm, as the code wants.
    #[test]
    fn straight_stair() {
        let (mut m, pb, p1, ha) = setup();
        let id = m
            .add_stair(spec(
                pb,
                p1,
                ha,
                vec![Point2::new(0.0, 0.0), Point2::new(4.48, 0.0)],
            ))
            .unwrap();
        let s = m.stair(id).unwrap();
        let l = s.layout();
        assert_eq!(l.flights.len(), 1);
        assert!(l.landings.is_empty());
        assert_eq!(l.treads(), 16);
        assert_eq!(l.risers(), 17);
        assert!((l.riser - 2.97 / 17.0).abs() < 1e-12);
        assert!((l.flights[0].going - 0.28).abs() < 1e-12);
        assert!(s.issues().is_empty(), "{:?}", s.issues());
        // The floor above moves: the stair follows it.
        m.set_level_elevation(p1, 3.2).unwrap();
        assert!((m.stair(id).unwrap().height - 3.2).abs() < 1e-12);
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }

    /// An L stair: two flights and a square landing at the turn; a U
    /// stair: the short piece across is part of the landing.
    #[test]
    fn stairs_with_landings_and_the_code() {
        let (mut m, pb, p1, ha) = setup();
        let l_shape = m
            .add_stair(spec(
                pb,
                p1,
                ha,
                vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(3.0, 0.0),
                    Point2::new(3.0, 3.0),
                ],
            ))
            .unwrap();
        let l = m.stair(l_shape).unwrap().layout();
        assert_eq!(l.flights.len(), 2);
        assert_eq!(l.landings.len(), 1);
        assert!((signed_area(&l.landings[0].outline) - 1.0).abs() < 1e-9);
        assert!((l.landings[0].top - l.flights[1].base).abs() < 1e-12);
        let u = m
            .add_stair(spec(
                pb,
                p1,
                ha,
                vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(2.5, 0.0),
                    Point2::new(2.5, 1.1),
                    Point2::new(0.0, 1.1),
                ],
            ))
            .unwrap();
        let l = m.stair(u).unwrap().layout();
        assert_eq!(l.flights.len(), 2);
        assert_eq!(l.landings.len(), 1);
        // 1 m deep, 2,1 m wide (both flights and the gap).
        assert!((signed_area(&l.landings[0].outline) - 2.1).abs() < 1e-9);
        // Too steep for a public stair, fine in a house.
        m.edit_stair(u, |s| s.going = 0.24).unwrap();
        let issues = m.stair(u).unwrap().issues();
        assert!(issues
            .iter()
            .any(|i| matches!(i, StairIssue::TreadTooShort { .. })));
        m.edit_stair(u, |s| s.usage = StairUse::Restricted).unwrap();
        assert!(!m
            .stair(u)
            .unwrap()
            .issues()
            .iter()
            .any(|i| matches!(i, StairIssue::TreadTooShort { .. })));
        // Nothing to climb, no flight, too narrow: refused.
        assert_eq!(
            m.edit_stair(u, |s| s.path =
                vec![Point2::new(0.0, 0.0), Point2::new(0.1, 0.0)]),
            Err(ModelError::InvalidStair)
        );
        assert_eq!(
            m.edit_stair(u, |s| s.width = 0.1),
            Err(ModelError::InvalidStair)
        );
    }
}
