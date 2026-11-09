//! Modular boxes (M1, `docs/MODULAR.md`): the volumetric module made in a
//! factory, carried on a lorry and craned onto its supports. A box is a
//! rectangle in plan with its height, its mark («MOD-03») and the state of
//! each of its four sides; what it holds (walls, floor, ceiling, roof,
//! columns, doors and windows, furniture) are the elements of always, and
//! the box does not keep a list of them: [`Model::box_members`] works them
//! out from where they are, so moving a wall into or out of a box needs no
//! bookkeeping. Each joint between two boxes has an owner (§5.2): the side
//! that carries the wall is [`SideState::Wall`], the other one
//! [`SideState::Open`]; a side on the outside is [`SideState::Free`], with
//! its façade and its parapet. IFC: `IfcElementAssembly` «Caja modular»
//! that aggregates its members.

use super::*;

/// Length of the box most of the system's houses use, in metres.
pub const BOX_LENGTH: f64 = 6.20;
/// Width of the system's boxes.
pub const BOX_WIDTH: f64 = 3.60;
/// Height of a whole box, from the underside of its floor chassis to the
/// top of its roof (27 cm of floor, 2,93 m clear, 38 cm of roof).
pub const BOX_HEIGHT: f64 = 3.58;
/// Underside of the floor chassis under the level (the finished floor).
pub const BOX_BOTTOM: f64 = -0.27;
/// How far from a side a wall may lie and still be that side's wall: the
/// whole thickness of the thickest envelope, with room to spare.
pub const SIDE_BAND: f64 = 0.35;
/// The joint between two boxes (§5.2): the tolerance of making and
/// placing them, filled with compressed mineral wool.
pub const BOX_JOINT: f64 = 0.02;
/// Two sides of two boxes closer than this, and facing, touch: the joint
/// (2 cm in the proposal) with its tolerance.
pub const TOUCH_GAP: f64 = 0.10;
/// Two touching sides must share at least this much to be a joint.
pub const TOUCH_OVERLAP: f64 = 0.50;

/// What a side of a box is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SideState {
    /// On the outside: façade wall and parapet on the roof.
    #[default]
    Free,
    /// Against another box, and this box carries the joint's wall.
    Wall,
    /// Against another box with nothing in between: the edge beam and the
    /// middle post stay, the room goes on into the next box.
    Open,
}

impl SideState {
    pub const ALL: [SideState; 3] = [SideState::Free, SideState::Wall, SideState::Open];

    pub fn label(self) -> &'static str {
        match self {
            SideState::Free => "Libre (fachada y pretil)",
            SideState::Wall => "Unido, con el muro de la junta",
            SideState::Open => "Unido, abierto",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SideState::Free => "free",
            SideState::Wall => "wall",
            SideState::Open => "open",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }

    /// Whether the side is against another box.
    pub fn joined(self) -> bool {
        self != SideState::Free
    }
}

/// The names of a box's sides: A and C are the long ones, B and D the
/// ends, A from the box's origin along its length and the rest going round
/// counter-clockwise.
pub const SIDE_NAMES: [&str; 4] = ["A", "B", "C", "D"];

/// A modular box.
#[derive(Debug, Clone, PartialEq)]
pub struct ModularBox {
    pub id: BoxId,
    pub guid: GlobalId,
    /// Its mark on the drawings and in the factory: «MOD-03».
    pub mark: String,
    /// What box of the system it is («Caja 6,20 · dormitorio doble»); free
    /// text until the catalogue (M8).
    pub kind: String,
    pub level: LevelId,
    /// The corner its sides start from: the outer face of its structure.
    pub origin: Point2,
    /// Direction of its length from the project's `x`, in radians.
    pub angle: f64,
    /// Along its long sides (A and C).
    pub length: f64,
    /// Along its ends (B and D).
    pub width: f64,
    /// Underside of the floor chassis over the level (negative: under the
    /// finished floor).
    pub bottom: f64,
    /// From the underside of the chassis to the top of the roof.
    pub height: f64,
    /// A, B, C and D.
    pub sides: [SideState; 4],
}

/// Data needed to make a box.
#[derive(Debug, Clone, PartialEq)]
pub struct NewBox {
    pub mark: Option<String>,
    pub kind: String,
    pub level: LevelId,
    pub origin: Point2,
    pub angle: f64,
    pub length: f64,
    pub width: f64,
    pub bottom: f64,
    pub height: f64,
    pub sides: [SideState; 4],
}

impl NewBox {
    /// A box of the usual size, all its sides free.
    pub fn usual(level: LevelId, origin: Point2, angle: f64) -> Self {
        NewBox {
            mark: None,
            kind: String::new(),
            level,
            origin,
            angle,
            length: BOX_LENGTH,
            width: BOX_WIDTH,
            bottom: BOX_BOTTOM,
            height: BOX_HEIGHT,
            sides: [SideState::Free; 4],
        }
    }
}

impl ModularBox {
    /// Unit vector along its length (sides A and C).
    pub fn along(&self) -> Point2 {
        Point2::new(self.angle.cos(), self.angle.sin())
    }

    /// Unit vector across it, from A towards C.
    pub fn across(&self) -> Point2 {
        self.along().perp_left()
    }

    /// Its four corners, counter-clockwise from the origin: side `i` goes
    /// from corner `i` to corner `i + 1`.
    pub fn corners(&self) -> [Point2; 4] {
        let u = self.along() * self.length;
        let v = self.across() * self.width;
        let o = self.origin;
        [o, o + u, o + u + v, o + v]
    }

    pub fn outline(&self) -> Vec<Point2> {
        self.corners().to_vec()
    }

    /// Side `i` from its start to its end.
    pub fn side(&self, i: usize) -> (Point2, Point2) {
        let c = self.corners();
        (c[i % 4], c[(i + 1) % 4])
    }

    pub fn side_length(&self, i: usize) -> f64 {
        if i.is_multiple_of(2) {
            self.length
        } else {
            self.width
        }
    }

    pub fn centre(&self) -> Point2 {
        self.origin + self.along() * (self.length / 2.0) + self.across() * (self.width / 2.0)
    }

    /// A point in the box's own axes: along its length and across it.
    pub fn local(&self, p: Point2) -> (f64, f64) {
        let d = p - self.origin;
        (d.dot(self.along()), d.dot(self.across()))
    }

    /// How far `p` is inside the box (negative: outside) and the side it
    /// is nearest to.
    pub fn depth(&self, p: Point2) -> (f64, usize) {
        let (s, t) = self.local(p);
        let d = [t, self.length - s, self.width - t, s];
        let mut best = 0;
        for i in 1..4 {
            if d[i] < d[best] {
                best = i;
            }
        }
        // Outside past a corner, the depth is the distance to the box.
        let out_s = (-s).max(s - self.length).max(0.0);
        let out_t = (-t).max(t - self.width).max(0.0);
        if out_s > 0.0 && out_t > 0.0 {
            return (-(out_s.hypot(out_t)), best);
        }
        (d[best], best)
    }

    pub fn contains(&self, p: Point2, tolerance: f64) -> bool {
        self.depth(p).0 >= -tolerance
    }

    /// Bottom and top over the level.
    pub fn span(&self) -> (f64, f64) {
        (self.bottom, self.bottom + self.height)
    }

    pub fn area(&self) -> f64 {
        self.length * self.width
    }
}

/// What a box holds, worked out from where everything is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoxMembers {
    pub walls: Vec<WallId>,
    pub openings: Vec<OpeningId>,
    pub slabs: Vec<SlabId>,
    pub ceilings: Vec<CeilingId>,
    pub columns: Vec<ColumnId>,
    pub beams: Vec<BeamId>,
    pub objects: Vec<ObjectId>,
}

impl BoxMembers {
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn count(&self) -> usize {
        self.walls.len()
            + self.openings.len()
            + self.slabs.len()
            + self.ceilings.len()
            + self.columns.len()
            + self.beams.len()
            + self.objects.len()
    }

    /// The ids of all of them.
    pub fn ids(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = Vec::with_capacity(self.count());
        ids.extend(self.walls.iter().map(|i| i.0));
        ids.extend(self.openings.iter().map(|i| i.0));
        ids.extend(self.slabs.iter().map(|i| i.0));
        ids.extend(self.ceilings.iter().map(|i| i.0));
        ids.extend(self.columns.iter().map(|i| i.0));
        ids.extend(self.beams.iter().map(|i| i.0));
        ids.extend(self.objects.iter().map(|i| i.0));
        ids
    }
}

/// The limits a box must keep: those of the system (the factory and its
/// chassis) and those of the road.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModularSite {
    /// Longest box the system makes (9 m).
    pub max_length: f64,
    /// Widest box the system makes (3,60 m).
    pub max_width: f64,
    /// Widest load without a permit for special transport (2,55 m).
    pub road_width: f64,
    /// Highest load on the road, from the ground (4 m).
    pub road_height: f64,
    /// Height of the lorry's deck the box stands on (a low loader).
    pub deck_height: f64,
}

impl Default for ModularSite {
    fn default() -> Self {
        ModularSite {
            max_length: 9.0,
            max_width: 3.60,
            road_width: 2.55,
            road_height: 4.0,
            deck_height: 0.40,
        }
    }
}

/// Something about a box to look at.
#[derive(Debug, Clone, PartialEq)]
pub enum BoxIssue {
    /// Longer than the system makes.
    TooLong { length: f64, max: f64 },
    /// Wider than the system makes.
    TooWide { width: f64, max: f64 },
    /// Wider than the road allows without a permit: special transport.
    WideLoad { width: f64, max: f64 },
    /// Too high on the lorry for the road.
    TooTall { height: f64, max: f64 },
    /// A free side against another box.
    FreeAgainstBox { side: usize, other: BoxId },
    /// A joined side with no box against it.
    JoinedAlone { side: usize },
    /// Both boxes put a wall in the joint.
    DoubleWall { side: usize, other: BoxId },
    /// Neither box puts a wall in the joint and it is not meant to be open
    /// on both: it is, so it is only noted.
    OpenBothSides { side: usize, other: BoxId },
}

impl BoxIssue {
    /// Whether it must be put right (and not only noted).
    pub fn is_error(&self) -> bool {
        !matches!(
            self,
            BoxIssue::WideLoad { .. } | BoxIssue::OpenBothSides { .. }
        )
    }

    pub fn text(&self, model: &Model) -> String {
        let mark = |b: &BoxId| {
            model
                .modular_box(*b)
                .map_or_else(|| "otra caja".to_string(), |b| b.mark.clone())
        };
        let m = |v: f64| format!("{v:.2}").replace('.', ",");
        match self {
            BoxIssue::TooLong { length, max } => format!(
                "Mide {} m de largo: el sistema llega a {} m.",
                m(*length),
                m(*max)
            ),
            BoxIssue::TooWide { width, max } => format!(
                "Mide {} m de ancho: el sistema llega a {} m.",
                m(*width),
                m(*max)
            ),
            BoxIssue::WideLoad { width, max } => format!(
                "Con {} m de ancho pasa de los {} m de una carga normal: transporte especial con autorización.",
                m(*width),
                m(*max)
            ),
            BoxIssue::TooTall { height, max } => format!(
                "Sobre el camión llega a {} m de alto, y la carretera admite {} m.",
                m(*height),
                m(*max)
            ),
            BoxIssue::FreeAgainstBox { side, other } => format!(
                "El lado {} está libre, pero toca {}.",
                SIDE_NAMES[*side],
                mark(other)
            ),
            BoxIssue::JoinedAlone { side } => format!(
                "El lado {} está unido, pero no toca ninguna caja.",
                SIDE_NAMES[*side]
            ),
            BoxIssue::DoubleWall { side, other } => format!(
                "El lado {} y el de {} llevan los dos el muro de la junta: debe llevarlo solo una.",
                SIDE_NAMES[*side],
                mark(other)
            ),
            BoxIssue::OpenBothSides { side, other } => format!(
                "El lado {} y el de {} están abiertos: la junta no lleva muro.",
                SIDE_NAMES[*side],
                mark(other)
            ),
        }
    }
}

/// The types to fill a box with: its envelope walls on the free and wall
/// sides, its floor, its roof and its false ceiling. `None` leaves that
/// part out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxFill {
    pub wall_type: Option<WallTypeId>,
    pub floor_type: Option<SlabTypeId>,
    pub roof_type: Option<SlabTypeId>,
    pub ceiling_type: Option<SlabTypeId>,
    /// Height of the false ceiling's face over the level.
    pub ceiling_height: f64,
}

/// What [`Model::fill_box`] made.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoxFilled {
    pub walls: Vec<WallId>,
    pub floor: Option<SlabId>,
    pub roof: Option<SlabId>,
    pub ceiling: Option<CeilingId>,
}

impl Model {
    fn check_box(&self, b: &ModularBox) -> Result<(), ModelError> {
        if !self.levels.contains_key(&b.level) {
            return Err(ModelError::UnknownLevel(b.level));
        }
        finite(b.origin.x, "box origin")?;
        finite(b.origin.y, "box origin")?;
        finite(b.angle, "box angle")?;
        finite(b.bottom, "box bottom")?;
        for (v, what) in [
            (b.length, "a box needs a length"),
            (b.width, "a box needs a width"),
            (b.height, "a box needs a height"),
        ] {
            if !(v.is_finite() && v > 0.05) {
                return Err(ModelError::InvalidShape(what));
            }
        }
        if b.mark.trim().is_empty() {
            return Err(ModelError::InvalidShape("a box needs a mark"));
        }
        Ok(())
    }

    /// The next free mark: «MOD-01», «MOD-02»…
    pub fn next_box_mark(&self) -> String {
        let mut n = self.boxes.len() + 1;
        loop {
            let mark = format!("MOD-{n:02}");
            if !self.boxes.values().any(|b| b.mark == mark) {
                return mark;
            }
            n += 1;
        }
    }

    pub fn add_box(&mut self, spec: NewBox) -> Result<BoxId, ModelError> {
        let mark = spec
            .mark
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| self.next_box_mark());
        let b = ModularBox {
            id: BoxId(0),
            guid: GlobalId::new(),
            mark,
            kind: spec.kind,
            level: spec.level,
            origin: spec.origin,
            angle: spec.angle,
            length: spec.length,
            width: spec.width,
            bottom: spec.bottom,
            height: spec.height,
            sides: spec.sides,
        };
        self.check_box(&b)?;
        let id = BoxId(self.take_id());
        self.boxes.insert(id, ModularBox { id, ..b });
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes a box, checked as when it was made. What it holds stays
    /// where it is: [`Model::move_box_with_members`] moves it all.
    pub fn edit_box(
        &mut self,
        id: BoxId,
        edit: impl FnOnce(&mut ModularBox),
    ) -> Result<(), ModelError> {
        let mut b = self
            .boxes
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut b);
        b.id = id;
        self.check_box(&b)?;
        self.boxes.insert(id, b);
        Ok(())
    }

    pub fn remove_box(&mut self, id: BoxId) -> Result<ModularBox, ModelError> {
        self.boxes
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn modular_box(&self, id: BoxId) -> Option<&ModularBox> {
        self.boxes.get(&id)
    }

    pub fn boxes(&self) -> impl Iterator<Item = &ModularBox> {
        self.boxes.values()
    }

    pub fn boxes_on_level(&self, level: LevelId) -> impl Iterator<Item = &ModularBox> {
        self.boxes.values().filter(move |b| b.level == level)
    }

    pub fn modular_site(&self) -> ModularSite {
        self.modular_site
    }

    pub fn set_modular_site(&mut self, site: ModularSite) -> Result<(), ModelError> {
        for v in [
            site.max_length,
            site.max_width,
            site.road_width,
            site.road_height,
            site.deck_height,
        ] {
            if !(v.is_finite() && v >= 0.0) {
                return Err(ModelError::InvalidShape(
                    "transport limits must be positive",
                ));
            }
        }
        self.modular_site = site;
        Ok(())
    }

    /// Which box each element of a level belongs to, if any. Walls and
    /// beams go by the middle of their axis: deep inside a box, to that
    /// box; near one of its sides, to the box whose side is not open (the
    /// owner of the joint). Floors, ceilings, columns and objects go by
    /// their centre; doors and windows with their wall.
    pub fn box_members_on(&self, level: LevelId) -> BTreeMap<BoxId, BoxMembers> {
        let boxes: Vec<&ModularBox> = self.boxes_on_level(level).collect();
        let mut out: BTreeMap<BoxId, BoxMembers> = BTreeMap::new();
        if boxes.is_empty() {
            return out;
        }
        // A line: the box it is deepest in, skipping open sides.
        let by_line = |p: Point2| -> Option<BoxId> {
            let mut best: Option<(f64, BoxId)> = None;
            for b in &boxes {
                let (d, side) = b.depth(p);
                if d < -SIDE_BAND {
                    continue;
                }
                if d <= SIDE_BAND && b.sides[side] == SideState::Open {
                    continue;
                }
                if best.is_none_or(|(bd, _)| d > bd + 1e-9) {
                    best = Some((d, b.id));
                }
            }
            best.map(|(_, id)| id)
        };
        // A point: the box it is in.
        let by_point = |p: Point2| -> Option<BoxId> {
            let mut best: Option<(f64, BoxId)> = None;
            for b in &boxes {
                let (d, _) = b.depth(p);
                if d < -0.05 {
                    continue;
                }
                if best.is_none_or(|(bd, _)| d > bd + 1e-9) {
                    best = Some((d, b.id));
                }
            }
            best.map(|(_, id)| id)
        };
        let mut wall_box: BTreeMap<WallId, BoxId> = BTreeMap::new();
        for w in self.walls.values().filter(|w| w.level == level) {
            let mid = (w.start + w.end) * 0.5;
            if let Some(b) = by_line(mid) {
                wall_box.insert(w.id, b);
                out.entry(b).or_default().walls.push(w.id);
            }
        }
        for o in self.openings.values() {
            if let Some(b) = wall_box.get(&o.wall) {
                out.entry(*b).or_default().openings.push(o.id);
            }
        }
        for s in self.slabs.values().filter(|s| s.level == level) {
            if let Some(b) = by_point(rooms_centroid(&s.outline)) {
                out.entry(b).or_default().slabs.push(s.id);
            }
        }
        for c in self.ceilings.values().filter(|c| c.level == level) {
            if let Some(b) = by_point(rooms_centroid(&c.outline)) {
                out.entry(b).or_default().ceilings.push(c.id);
            }
        }
        for c in self.columns().filter(|c| c.level == level) {
            if let Some(b) = by_point(c.position) {
                out.entry(b).or_default().columns.push(c.id);
            }
        }
        for beam in self.beams().filter(|b| b.level == level) {
            if let Some(b) = by_line((beam.start + beam.end) * 0.5) {
                out.entry(b).or_default().beams.push(beam.id);
            }
        }
        for o in self.objects().filter(|o| o.level == level) {
            if let Some(b) = by_point(o.position) {
                out.entry(b).or_default().objects.push(o.id);
            }
        }
        out
    }

    /// What a box holds.
    pub fn box_members(&self, id: BoxId) -> BoxMembers {
        let Some(b) = self.boxes.get(&id) else {
            return BoxMembers::default();
        };
        self.box_members_on(b.level).remove(&id).unwrap_or_default()
    }

    /// The box an element belongs to.
    pub fn box_of(&self, element: u64) -> Option<BoxId> {
        let levels: std::collections::BTreeSet<LevelId> =
            self.boxes.values().map(|b| b.level).collect();
        for level in levels {
            for (b, members) in self.box_members_on(level) {
                if members.ids().contains(&element) {
                    return Some(b);
                }
            }
        }
        None
    }

    /// For each side of a box, the box it touches, if any: a side of
    /// another box on the same level, facing it, at most [`TOUCH_GAP`]
    /// away and sharing at least [`TOUCH_OVERLAP`] with it.
    pub fn box_neighbours(&self, id: BoxId) -> [Option<(BoxId, usize)>; 4] {
        let mut out = [None; 4];
        let Some(b) = self.boxes.get(&id) else {
            return out;
        };
        for (i, slot) in out.iter_mut().enumerate() {
            let (a0, a1) = b.side(i);
            let dir = match (a1 - a0).normalized() {
                Some(d) => d,
                None => continue,
            };
            let outward = -dir.perp_left();
            let mut best: Option<(f64, BoxId, usize)> = None;
            for o in self.boxes_on_level(b.level).filter(|o| o.id != id) {
                for j in 0..4 {
                    let (b0, b1) = o.side(j);
                    let Some(odir) = (b1 - b0).normalized() else {
                        continue;
                    };
                    // Facing: the other side runs the other way.
                    if dir.dot(odir) > -0.999 {
                        continue;
                    }
                    let gap = (b0 - a0).dot(outward);
                    if !(-TOUCH_GAP..=TOUCH_GAP).contains(&gap) {
                        continue;
                    }
                    let (s0, s1) = ((b0 - a0).dot(dir), (b1 - a0).dot(dir));
                    let (lo, hi) = (s0.min(s1).max(0.0), s0.max(s1).min(b.side_length(i)));
                    let overlap = hi - lo;
                    if overlap < TOUCH_OVERLAP {
                        continue;
                    }
                    if best.is_none_or(|(bo, _, _)| overlap > bo) {
                        best = Some((overlap, o.id, j));
                    }
                }
            }
            *slot = best.map(|(_, o, j)| (o, j));
        }
        out
    }

    /// What is wrong with a box, or worth knowing.
    pub fn box_issues(&self, id: BoxId) -> Vec<BoxIssue> {
        let Some(b) = self.boxes.get(&id) else {
            return Vec::new();
        };
        let site = self.modular_site;
        let mut issues = Vec::new();
        let long = b.length.max(b.width);
        let wide = b.length.min(b.width);
        if long > site.max_length + 1e-6 {
            issues.push(BoxIssue::TooLong {
                length: long,
                max: site.max_length,
            });
        }
        if wide > site.max_width + 1e-6 {
            issues.push(BoxIssue::TooWide {
                width: wide,
                max: site.max_width,
            });
        } else if wide > site.road_width + 1e-6 {
            issues.push(BoxIssue::WideLoad {
                width: wide,
                max: site.road_width,
            });
        }
        let on_road = b.height + site.deck_height;
        if on_road > site.road_height + 1e-6 {
            issues.push(BoxIssue::TooTall {
                height: on_road,
                max: site.road_height,
            });
        }
        for (side, n) in self.box_neighbours(id).into_iter().enumerate() {
            let state = b.sides[side];
            match n {
                None if state.joined() => issues.push(BoxIssue::JoinedAlone { side }),
                None => {}
                Some((other, _)) if state == SideState::Free => {
                    issues.push(BoxIssue::FreeAgainstBox { side, other })
                }
                Some((other, j)) => {
                    let theirs = self.boxes[&other].sides[j];
                    if state == SideState::Wall && theirs == SideState::Wall {
                        issues.push(BoxIssue::DoubleWall { side, other });
                    } else if state == SideState::Open && theirs == SideState::Open {
                        issues.push(BoxIssue::OpenBothSides { side, other });
                    }
                }
            }
        }
        issues
    }

    /// Sets the sides of a box from the boxes around it: free where it
    /// touches nothing; against another box, open where the other one
    /// carries the wall and the wall where the other one is open. Against
    /// a free side of a box already there, the one already there keeps
    /// its façade wall as the joint's (its side becomes «wall») and this
    /// one stays open; between two boxes being laid out together, the
    /// first one carries it.
    pub fn set_box_sides_from_neighbours(&mut self, id: BoxId) -> Result<(), ModelError> {
        let neighbours = self.box_neighbours(id);
        let mut sides = self
            .boxes
            .get(&id)
            .ok_or(ModelError::UnknownElement(id.0))?
            .sides;
        let mut theirs: Vec<(BoxId, usize)> = Vec::new();
        for (i, n) in neighbours.into_iter().enumerate() {
            sides[i] = match n {
                None => SideState::Free,
                Some((other, j)) => match self.boxes[&other].sides[j] {
                    SideState::Wall => SideState::Open,
                    SideState::Open => SideState::Wall,
                    SideState::Free if id < other => SideState::Wall,
                    SideState::Free => {
                        theirs.push((other, j));
                        SideState::Open
                    }
                },
            };
        }
        self.edit_box(id, |b| b.sides = sides)?;
        for (other, j) in theirs {
            self.edit_box(other, |b| b.sides[j] = SideState::Wall)?;
        }
        Ok(())
    }

    /// Where a new box near `spec.origin` goes to stand against a box of
    /// its level: one of its sides facing one of the other's, a joint
    /// away, flush with one of its ends. The nearest place within `reach`
    /// of where it was, if any.
    pub fn box_snap(&self, spec: &NewBox, reach: f64) -> Option<Point2> {
        snap_against(spec, reach, self.boxes_on_level(spec.level))
    }

    /// Fills a box with its envelope: a wall on each free side and on each
    /// side that carries a joint's wall (outer face on the box's outline,
    /// from the underside of the chassis to the top of the roof), its
    /// floor (top at the level), its roof (top at the top of the box) and
    /// its false ceiling.
    pub fn fill_box(&mut self, id: BoxId, fill: BoxFill) -> Result<BoxFilled, ModelError> {
        let b = self
            .boxes
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        let mut made = BoxFilled::default();
        let outline = b.outline();
        if let Some(t) = fill.floor_type {
            made.floor = Some(self.add_slab(NewSlab {
                slab_type: t,
                level: b.level,
                outline: outline.clone(),
                top_offset: 0.0,
            })?);
        }
        if let Some(t) = fill.roof_type {
            made.roof = Some(self.add_slab(NewSlab {
                slab_type: t,
                level: b.level,
                outline: outline.clone(),
                top_offset: b.bottom + b.height,
            })?);
        }
        if let Some(t) = fill.ceiling_type {
            made.ceiling = Some(self.add_ceiling(NewCeiling {
                ceiling_type: t,
                level: b.level,
                outline: outline.clone(),
                height: fill.ceiling_height,
            })?);
        }
        if let Some(t) = fill.wall_type {
            for i in 0..4 {
                if b.sides[i] == SideState::Open {
                    continue;
                }
                // Clockwise round the box, so the exterior is on the left.
                let (s, e) = b.side(i);
                made.walls.push(self.add_wall(NewWall {
                    wall_type: t,
                    level: b.level,
                    start: e,
                    end: s,
                    bulge: 0.0,
                    reference: ReferenceLine::ExteriorFace,
                    height: b.height,
                    base_offset: b.bottom,
                    top: WallTop::Unconnected,
                })?);
            }
        }
        Ok(made)
    }

    pub(super) fn restore_boxes(
        &mut self,
        list: Vec<ModularBox>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for b in list {
            let id = b.id;
            claim(id.0, Some(&b.guid))
                .and_then(|_| self.check_box(&b))
                .map_err(|error| RestoreError {
                    element: format!("box {}", id.0),
                    error,
                })?;
            self.boxes.insert(id, b);
        }
        Ok(())
    }
}

/// Where a box near `spec.origin` goes to stand against one of `boxes`
/// (see [`Model::box_snap`]).
pub fn snap_against<'a>(
    spec: &NewBox,
    reach: f64,
    boxes: impl Iterator<Item = &'a ModularBox>,
) -> Option<Point2> {
    let probe = ModularBox {
        id: BoxId(0),
        guid: GlobalId::new(),
        mark: String::new(),
        kind: String::new(),
        level: spec.level,
        origin: Point2::new(0.0, 0.0),
        angle: spec.angle,
        length: spec.length,
        width: spec.width,
        bottom: spec.bottom,
        height: spec.height,
        sides: spec.sides,
    };
    // Its corners from its origin.
    let rel = probe.corners();
    let mut best: Option<(f64, Point2)> = None;
    for o in boxes {
        for j in 0..4 {
            let (b0, b1) = o.side(j);
            let Some(odir) = (b1 - b0).normalized() else {
                continue;
            };
            let out = -odir.perp_left();
            for i in 0..4 {
                let (s0, s1) = (rel[i], rel[(i + 1) % 4]);
                let Some(dir) = (s1 - s0).normalized() else {
                    continue;
                };
                if dir.dot(odir) > -0.999 {
                    continue;
                }
                for origin in [b1 + out * BOX_JOINT - s0, b0 + out * BOX_JOINT - s1] {
                    let d = origin.distance(spec.origin);
                    if d <= reach && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, origin));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

fn rooms_centroid(poly: &[Point2]) -> Point2 {
    crate::rooms::centroid(poly)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> (Model, LevelId) {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        (m, lv)
    }

    #[test]
    fn neighbours_and_issues() {
        let (mut m, lv) = model();
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        // The second one 2 cm further, across side C of the first.
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, BOX_WIDTH + 0.02), 0.0))
            .unwrap();
        assert_eq!(m.modular_box(a).unwrap().mark, "MOD-01");
        assert_eq!(m.modular_box(b).unwrap().mark, "MOD-02");
        let n = m.box_neighbours(a);
        assert_eq!(n[2], Some((b, 0)));
        assert_eq!(n[0], None);
        // Both free and touching: each says so.
        assert!(m
            .box_issues(a)
            .contains(&BoxIssue::FreeAgainstBox { side: 2, other: b }));
        // Set by their neighbours: the first carries the wall.
        m.set_box_sides_from_neighbours(a).unwrap();
        m.set_box_sides_from_neighbours(b).unwrap();
        assert_eq!(m.modular_box(a).unwrap().sides[2], SideState::Wall);
        assert_eq!(m.modular_box(b).unwrap().sides[0], SideState::Open);
        let issues = m.box_issues(a);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(matches!(issues[0], BoxIssue::WideLoad { .. }));
        // Too long, too tall.
        m.edit_box(a, |x| {
            x.length = 10.0;
            x.height = 3.80;
        })
        .unwrap();
        let issues = m.box_issues(a);
        assert!(issues.iter().any(|i| matches!(i, BoxIssue::TooLong { .. })));
        assert!(issues.iter().any(|i| matches!(i, BoxIssue::TooTall { .. })));
        assert!(issues.iter().all(|i| !i.text(&m).is_empty()));
    }

    #[test]
    fn a_filled_box_holds_its_envelope() {
        let (mut m, lv) = model();
        let mat = m.add_material("Placa", "board", [200, 200, 200], CutPattern::Solid);
        let layer = |t| Layer {
            material: mat,
            thickness: t,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("Muro", WallFunction::Exterior, vec![layer(0.2)])
            .unwrap();
        let ft = m
            .add_slab_type("Suelo", SlabFunction::Floor, vec![layer(0.27)])
            .unwrap();
        let rt = m
            .add_slab_type("Cubierta", SlabFunction::Roof, vec![layer(0.38)])
            .unwrap();
        let ct = m
            .add_slab_type("Techo", SlabFunction::Ceiling, vec![layer(0.015)])
            .unwrap();
        let fill = BoxFill {
            wall_type: Some(wt),
            floor_type: Some(ft),
            roof_type: Some(rt),
            ceiling_type: Some(ct),
            ceiling_height: 2.93,
        };
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, BOX_WIDTH + 0.02), 0.0))
            .unwrap();
        m.edit_box(a, |x| x.sides[2] = SideState::Wall).unwrap();
        m.edit_box(b, |x| x.sides[0] = SideState::Open).unwrap();
        let fa = m.fill_box(a, fill).unwrap();
        let fb = m.fill_box(b, fill).unwrap();
        assert_eq!(fa.walls.len(), 4);
        assert_eq!(fb.walls.len(), 3);
        // The joint's wall, outer face on A's side C, inside A.
        let ma = m.box_members(a);
        let mb = m.box_members(b);
        assert_eq!(ma.walls.len(), 4, "{ma:?}");
        assert_eq!(mb.walls.len(), 3, "{mb:?}");
        assert_eq!(ma.slabs.len(), 2);
        assert_eq!(ma.ceilings.len(), 1);
        assert_eq!(m.box_of(fa.walls[2].0), Some(a));
        // The walls go from the chassis to the top of the roof.
        let w = m.wall(fa.walls[0]).unwrap();
        assert!((w.base_offset - BOX_BOTTOM).abs() < 1e-9);
        assert!((w.height - BOX_HEIGHT).abs() < 1e-9);
        // An open side keeps its wall out even if it is the other box's.
        let shared = m.wall(fa.walls[2]).unwrap();
        let mid = (shared.start + shared.end) * 0.5;
        assert!(m.modular_box(b).unwrap().contains(mid, SIDE_BAND));
    }

    #[test]
    fn a_new_box_snaps_beside_another_and_takes_the_joint() {
        let (mut m, lv) = model();
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        // Clicked a little off the place beside side C of the first.
        let mut spec = NewBox::usual(lv, Point2::new(0.3, BOX_WIDTH + 0.4), 0.0);
        let at = m.box_snap(&spec, 1.0).unwrap();
        assert!(
            at.approx_eq(Point2::new(0.0, BOX_WIDTH + BOX_JOINT)),
            "{at:?}"
        );
        spec.origin = at;
        let b = m.add_box(spec).unwrap();
        m.set_box_sides_from_neighbours(b).unwrap();
        // The first one keeps its façade as the joint's wall.
        assert_eq!(m.modular_box(a).unwrap().sides[2], SideState::Wall);
        assert_eq!(m.modular_box(b).unwrap().sides[0], SideState::Open);
        assert!(m.box_issues(b).iter().all(|i| !i.is_error()));
        // Too far: nothing.
        let far = NewBox::usual(lv, Point2::new(20.0, 20.0), 0.0);
        assert!(m.box_snap(&far, 1.0).is_none());
    }

    #[test]
    fn depth_round_a_turned_box() {
        let (mut m, lv) = model();
        let id = m
            .add_box(NewBox::usual(
                lv,
                Point2::new(10.0, 5.0),
                std::f64::consts::FRAC_PI_2,
            ))
            .unwrap();
        let b = m.modular_box(id).unwrap();
        let c = b.corners();
        assert!(c[1].approx_eq(Point2::new(10.0, 5.0 + BOX_LENGTH)));
        assert!(c[3].approx_eq(Point2::new(10.0 - BOX_WIDTH, 5.0)));
        let (d, side) = b.depth(b.centre());
        assert!((d - BOX_WIDTH / 2.0).abs() < 1e-9);
        assert!(side == 0 || side == 2);
        assert!(!b.contains(Point2::new(11.0, 6.0), 0.01));
    }
}
