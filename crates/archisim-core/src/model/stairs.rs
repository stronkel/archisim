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

/// Which sides of a stair have a railing (walking up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Railings {
    None,
    Left,
    Right,
    #[default]
    Both,
}

impl Railings {
    pub fn left(self) -> bool {
        matches!(self, Railings::Left | Railings::Both)
    }

    pub fn right(self) -> bool {
        matches!(self, Railings::Right | Railings::Both)
    }
}

/// Height of a stair's railing above the nosings (CTE: 0,90 m).
pub const RAILING_HEIGHT: f64 = 0.9;

/// How a stair is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairForm {
    /// Straight flights along the walking line, a landing at every turn.
    #[default]
    Landings,
    /// One run along the walking line, its turns made with winders
    /// ("peldaños compensados").
    Winders,
    /// Round a centre ("de caracol"): the walking line's first point is
    /// the centre, its second gives the direction of the first riser.
    Spiral,
}

/// What carries the treads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairStructure {
    /// A concrete slab under the steps ("losa").
    #[default]
    Slab,
    /// A stringer on each side ("dos zancas laterales").
    Stringers,
    /// One stringer under the middle ("zanca central").
    Spine,
    /// Treads built into a wall, nothing under them ("peldaños volados").
    Cantilever,
    /// A spiral's treads round a central post ("mástil").
    Newel,
}

/// The form and build of a stair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairDesign {
    pub form: StairForm,
    /// Spiral: the angle turned from the first riser to the last, in
    /// radians; counter-clockwise if positive.
    pub sweep: f64,
    /// Spiral: the radius of the newel or of the eye, where the treads
    /// begin.
    pub core: f64,
    pub structure: StairStructure,
    /// Steps with risers ("tabica"); without, open treads.
    pub closed_risers: bool,
    /// Thickness of the treads (on a slab, of their finish, if it has a
    /// material of its own).
    pub tread: f64,
    /// The material of the treads; `None`, the stair's.
    pub tread_material: Option<MaterialId>,
    /// Width and depth of the stringers (or of the central one).
    pub stringer: [f64; 2],
    /// Flights and landings: each square turn's landing split along its
    /// diagonal into two treads at 45° ("meseta partida"; a U's, into
    /// three), so the stair is one run.
    pub split: bool,
    /// Spiral: the angle of the landing at its head ("meseta de
    /// llegada"), at the floor above, turning on the same way; 0, none.
    pub arrival: f64,
}

impl Default for StairDesign {
    fn default() -> Self {
        StairDesign {
            form: StairForm::Landings,
            sweep: 0.0,
            core: 0.1,
            structure: StairStructure::Slab,
            closed_risers: true,
            tread: 0.04,
            tread_material: None,
            stringer: [0.08, 0.3],
            split: false,
            arrival: 0.0,
        }
    }
}

impl StairDesign {
    /// Thickness of what is laid on the slab of a concrete stair (none
    /// without a material of its own).
    pub fn finish(&self) -> f64 {
        match (self.structure, self.tread_material) {
            (StairStructure::Slab, Some(_)) => self.tread,
            _ => 0.0,
        }
    }
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
    /// Railings along the flights (and round the landings).
    pub railings: Railings,
    /// Its form and build.
    pub design: StairDesign,
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
    pub design: StairDesign,
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
    /// Its left side, from the flight arriving to the one leaving.
    pub left: Vec<Point2>,
    /// Its right side, the same way.
    pub right: Vec<Point2>,
    /// The middle of the stair through it.
    pub centre: Vec<Point2>,
}

/// How a stair is laid out: its treads and landings in walking order and
/// the riser height (and, for flights and landings, the flights).
#[derive(Debug, Clone, PartialEq)]
pub struct StairLayout {
    /// The straight flights (a stair of flights and landings).
    pub flights: Vec<Flight>,
    pub landings: Vec<Landing>,
    /// Every tread, from the foot.
    pub steps: Vec<Step>,
    pub riser: f64,
    /// The runs of treads, a landing between each two.
    pub runs: Vec<Run>,
    /// The walking line drawn in plan, from the foot to the head.
    pub walk: Vec<Point2>,
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
    /// No handrail on a stair climbing 55 cm or more.
    NoHandrail,
    /// A handrail on one side only of a stair 1,20 m wide or more.
    HandrailBothSides,
    /// A curved tread narrower than 5 cm on its narrow side (dwelling).
    NarrowEdge { going: f64, min: f64 },
    /// A curved tread deeper than 44 cm on its wide side.
    WideEdge { going: f64, max: f64 },
    /// Steps without risers in a stair of general use (only allowed out of
    /// public areas and not for evacuation upwards).
    OpenRisers,
    /// Landings split into treads at 45° in a stair of general use (the
    /// code allows them only in stairs of restricted use).
    SplitLanding,
}

/// Extreme values a stair may take at all (not the code's limits).
const MIN_WIDTH: f64 = 0.4;
const MIN_GOING: f64 = 0.1;

/// Lays out a stair of any form.
#[allow(clippy::too_many_arguments)]
pub fn stair_layout(
    path: &[Point2],
    width: f64,
    going: f64,
    height: f64,
    base: f64,
    design: &StairDesign,
    edge: f64,
) -> StairLayout {
    match design.form {
        StairForm::Landings if design.split => split_layout(path, width, going, height, base),
        StairForm::Landings => layout(path, width, going, height, base),
        StairForm::Winders => winder_layout(path, width, going, height, base, edge),
        StairForm::Spiral => match path {
            [c, f, ..] => {
                let mut l = spiral_layout(
                    *c,
                    *f - *c,
                    design.sweep,
                    design.core,
                    width,
                    going,
                    height,
                    base,
                    edge,
                );
                spiral_arrival(
                    &mut l,
                    *c,
                    *f - *c,
                    design.sweep,
                    design.core,
                    width,
                    base + height,
                    design.arrival,
                );
                l
            }
            _ => layout(&[], width, going, height, base),
        },
    }
}

impl Stair {
    /// Treads, landings and riser height.
    pub fn layout(&self) -> StairLayout {
        stair_layout(
            &self.path,
            self.width,
            self.going,
            self.height,
            self.base_offset,
            &self.design,
            self.walking_edge(),
        )
    }

    /// The outline of the whole stair in plan: treads and landings.
    pub fn outlines(&self) -> Vec<Vec<Point2>> {
        let l = self.layout();
        l.steps
            .iter()
            .map(Step::outline)
            .chain(l.landings.iter().map(|x| x.outline.clone()))
            .collect()
    }

    /// Where one sets foot on the stair, and where one leaves it, on its
    /// walking line.
    pub fn foot(&self) -> Option<Point2> {
        match self.design.form {
            StairForm::Spiral => self.layout().walk.first().copied(),
            _ => self.path.first().copied(),
        }
    }

    pub fn head(&self) -> Option<Point2> {
        match self.design.form {
            StairForm::Spiral => self.layout().walk.last().copied(),
            _ => self.path.last().copied(),
        }
    }

    /// How far from the narrow side of a curved tread the code measures
    /// its going: 50 cm, or its middle in a dwelling's stair narrower
    /// than 1 m.
    pub fn walking_edge(&self) -> f64 {
        walking_edge(self.usage, self.width)
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
        let going = l.going();
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
        // Curved treads: not too narrow on one side, not too deep on the
        // other. The treads of a split landing are not curved treads: the
        // code allows them in a stair of restricted use, and not in any
        // other.
        let split = self.design.form == StairForm::Landings;
        if split && self.usage == StairUse::General && l.steps.iter().any(|s| s.turned) {
            v.push(StairIssue::SplitLanding);
        }
        let turned = || l.steps.iter().filter(|s| s.turned && !split);
        let narrow = turned()
            .map(|s| s.side_length(true).min(s.side_length(false)))
            .fold(f64::INFINITY, f64::min);
        let wide = turned()
            .map(|s| s.side_length(true).max(s.side_length(false)))
            .fold(0.0, f64::max);
        if self.usage == StairUse::Restricted && narrow < 0.05 - e {
            v.push(StairIssue::NarrowEdge {
                going: narrow,
                min: 0.05,
            });
        }
        if wide > 0.44 + e {
            v.push(StairIssue::WideEdge {
                going: wide,
                max: 0.44,
            });
        }
        if self.usage == StairUse::General
            && !self.design.closed_risers
            && self.design.structure != StairStructure::Slab
        {
            v.push(StairIssue::OpenRisers);
        }
        if l.risers() as f64 * l.riser >= 0.55 - e {
            match self.railings {
                Railings::None => v.push(StairIssue::NoHandrail),
                Railings::Left | Railings::Right if self.width >= 1.2 - e => {
                    v.push(StairIssue::HandrailBothSides)
                }
                _ => {}
            }
        }
        for run in &l.runs {
            let risers = run.steps.len() + 1;
            if risers < 3 {
                v.push(StairIssue::ShortFlight { risers });
            }
            let h = risers as f64 * l.riser;
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
        let d = &s.design;
        for (v, what) in [
            (d.sweep, "stair sweep"),
            (d.core, "stair core"),
            (d.tread, "stair tread"),
            (d.stringer[0], "stair stringer width"),
            (d.stringer[1], "stair stringer depth"),
        ] {
            finite(v, what)?;
        }
        if let Some(m) = d.tread_material {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        if s.width < MIN_WIDTH
            || s.going < MIN_GOING
            || s.waist <= EPSILON
            || !(0.005..=0.3).contains(&d.tread)
            || d.stringer[0] < 0.005
            || d.stringer[1] < 0.02
            || d.core < 0.0
        {
            return Err(ModelError::InvalidStair);
        }
        if d.form == StairForm::Spiral
            && (s.path.len() != 2
                || s.path[0].approx_eq(s.path[1])
                || !(0.2..=6.0 * std::f64::consts::TAU).contains(&d.sweep.abs())
                || d.core > 5.0)
        {
            return Err(ModelError::InvalidStair);
        }
        let height = self.stair_height(s.level, s.base_offset, s.top, s.height)?;
        if height <= 0.05 {
            return Err(ModelError::InvalidStair);
        }
        let l = stair_layout(
            &s.path,
            s.width,
            s.going,
            height,
            s.base_offset,
            d,
            walking_edge(s.usage, s.width),
        );
        if s.path.len() < 2 || l.steps.is_empty() {
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
                railings: Railings::default(),
                design: spec.design,
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
        design: s.design,
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
            design: StairDesign::default(),
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
