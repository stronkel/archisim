//! The load takedown: the vertical loads of every case, from the floors
//! and roofs that carry them to the walls, beams and columns under them,
//! and down to the ground.
//!
//! Each floor or roof is cut into squares of `STEP`; each square's load
//! goes to what holds it up: in a light frame, along its joists or
//! rafters to the two supports they cross (one-way, by the lever rule);
//! otherwise to the nearest support (two-way, by tributary areas). The
//! walls, beams and columns add their own weight and pass all of it on,
//! from the top down: a wall to the floor, beam or wall it stands on, a
//! beam to its supports (lever rule between them), a column to what is
//! under its base. What reaches no element below reaches the ground (the
//! foundation).

use std::collections::{BTreeMap, HashMap};

use archisim_core::{
    BeamId, ColumnId, LevelId, LoadCase, LoadShape, Model, Point2, SlabFunction, Vec2, WallId,
};

use crate::actions::{roof_snow, room_category, UseCategory};
use crate::cases::Case;
use crate::geo;
use crate::weights::{self, TILES};

/// The side of the squares the floors are cut into, m.
pub const STEP: f64 = 0.25;
/// The length of the stretches the walls and beams are cut into, m.
pub const BIN: f64 = 0.25;

/// Something that carries loads down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Carrier {
    Wall(WallId),
    Beam(BeamId),
    Column(ColumnId),
}

/// What a carrier receives and passes on.
#[derive(Debug, Clone, PartialEq)]
pub struct Carried {
    /// Its line in plan (one point for a column).
    pub path: Vec<Point2>,
    pub length: f64,
    /// Height of its top and base above project zero.
    pub top: f64,
    pub base: f64,
    /// The loads it receives, kN per stretch of `BIN` along it (one
    /// stretch for a column), by case.
    pub received: BTreeMap<Case, Vec<f64>>,
    /// Its own weight, kN per stretch.
    pub own: Vec<f64>,
    /// The part of `received` that comes from beams and columns (which
    /// the frame analysis takes from them itself), kN per stretch.
    pub from_frame: BTreeMap<Case, Vec<f64>>,
    /// What it passes on at its base (received + own), kN per stretch.
    pub delivered: BTreeMap<Case, Vec<f64>>,
}

impl Carried {
    fn new(path: Vec<Point2>, top: f64, base: f64) -> Self {
        let length = geo::path_length(&path);
        let n = bins(length);
        Carried {
            path,
            length,
            top,
            base,
            received: BTreeMap::new(),
            own: vec![0.0; n],
            from_frame: BTreeMap::new(),
            delivered: BTreeMap::new(),
        }
    }

    fn add(&mut self, case: Case, along: f64, kn: f64) {
        let n = self.own.len().max(1);
        let i = ((along / BIN).floor().max(0.0) as usize).min(n - 1);
        self.received.entry(case).or_insert_with(|| vec![0.0; n])[i] += kn;
    }

    fn add_from_frame(&mut self, case: Case, along: f64, kn: f64) {
        let n = self.own.len().max(1);
        let i = ((along / BIN).floor().max(0.0) as usize).min(n - 1);
        self.from_frame.entry(case).or_insert_with(|| vec![0.0; n])[i] += kn;
    }

    /// What it receives from outside the frame of beams and columns, kN
    /// per stretch.
    pub fn received_outside_frame(&self, case: Case) -> Vec<f64> {
        let mut v = self
            .received
            .get(&case)
            .cloned()
            .unwrap_or_else(|| vec![0.0; self.own.len().max(1)]);
        if let Some(f) = self.from_frame.get(&case) {
            for (a, b) in v.iter_mut().zip(f) {
                *a -= b;
            }
        }
        v
    }

    /// The total it receives in a case, kN.
    pub fn received_total(&self, case: Case) -> f64 {
        self.received.get(&case).map_or(0.0, |v| v.iter().sum())
    }

    /// The total it passes on in a case, kN.
    pub fn delivered_total(&self, case: Case) -> f64 {
        self.delivered.get(&case).map_or(0.0, |v| v.iter().sum())
    }

    /// The load per metre it receives at `along` in a case, kN/m
    /// (averaged over `width` either side).
    pub fn received_per_m(&self, case: Case, along: f64, width: f64) -> f64 {
        per_m(self.received.get(&case), self.length, along, width)
    }

    /// The load per metre it passes on at `along`, kN/m.
    pub fn delivered_per_m(&self, case: Case, along: f64, width: f64) -> f64 {
        per_m(self.delivered.get(&case), self.length, along, width)
    }

    /// The point along it `along` metres from its start.
    pub fn point(&self, along: f64) -> Point2 {
        geo::along_path(&self.path, along)
    }
}

fn per_m(v: Option<&Vec<f64>>, length: f64, along: f64, width: f64) -> f64 {
    let Some(v) = v else {
        return 0.0;
    };
    let (a, b) = ((along - width).max(0.0), (along + width).min(length));
    if b <= a {
        return 0.0;
    }
    let mut sum = 0.0;
    for (i, x) in v.iter().enumerate() {
        let (s0, s1) = (i as f64 * BIN, ((i + 1) as f64 * BIN).min(length));
        let over = (s1.min(b) - s0.max(a)).max(0.0);
        if s1 > s0 {
            sum += x * over / (s1 - s0);
        }
    }
    sum / (b - a)
}

fn bins(length: f64) -> usize {
    ((length / BIN).ceil() as usize).max(1)
}

/// A load that reaches the ground.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundLoad {
    pub at: Point2,
    pub case: Case,
    pub kn: f64,
    /// What brings it down (`None`: a slab on the ground).
    pub from: Option<Carrier>,
}

/// A floor or roof, as the report shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub name: String,
    pub level: LevelId,
    /// Plan area, m².
    pub area: f64,
    /// Weight of its build-up per m² of plan, kN/m².
    pub dead: f64,
    /// Area of each use category on it (m²) and its live load (kN).
    pub uses: BTreeMap<UseCategory, (f64, f64)>,
    /// Snow per m² of plan where it is open to the sky, kN/m².
    pub snow: f64,
    /// Area open to the sky, m².
    pub exposed: f64,
    /// One-way (light frame) or two-way.
    pub one_way: bool,
    /// Whether it lies on the ground.
    pub on_ground: bool,
    /// Whether it found nothing to stand on.
    pub unsupported: bool,
}

/// The whole takedown.
#[derive(Debug, Clone, Default)]
pub struct Takedown {
    pub cases: Vec<Case>,
    pub carriers: BTreeMap<Carrier, Carried>,
    pub ground: Vec<GroundLoad>,
    pub surfaces: Vec<Surface>,
    pub notes: Vec<String>,
    /// The loads put on the floors and roofs and by hand, by case, kN.
    pub applied: BTreeMap<Case, f64>,
    /// What reached each plate (of `plates`) from above: walls, columns
    /// and loads placed by hand (point, case, kN).
    pub plate_incoming: Vec<Vec<(Point2, Case, f64)>>,
}

impl Takedown {
    /// What was put on: the floors' and roofs' loads, the hand loads and
    /// the carriers' own weight, kN.
    pub fn applied_total(&self, case: Case) -> f64 {
        let mut t = self.applied.get(&case).copied().unwrap_or(0.0);
        if case == Case::Dead {
            t += self
                .carriers
                .values()
                .map(|c| c.own.iter().sum::<f64>())
                .sum::<f64>();
        }
        t
    }

    /// The total of a case that reaches the ground, kN.
    pub fn ground_total(&self, case: Case) -> f64 {
        self.ground
            .iter()
            .filter(|g| g.case == case)
            .map(|g| g.kn)
            .sum()
    }
}

/// Where a plate comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateSource {
    Slab(archisim_core::SlabId),
    /// A roof and the side of its footprint the plane rises from.
    Roof(archisim_core::RoofId, usize),
}

/// A floor or roof plane to take down.
pub struct Plate {
    pub source: PlateSource,
    pub name: String,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
    /// Height of its top face at a point, above project zero.
    pub top: Box<dyn Fn(Point2) -> f64>,
    pub thickness: f64,
    /// Weight per m² of plan, tiles included, kN/m².
    pub dead: f64,
    /// Pitch, radians.
    pub pitch: f64,
    pub roof: bool,
    pub walkable: bool,
    /// One-way span direction (light frame).
    pub span: Option<Vec2>,
    pub tiles: bool,
    /// A slab on the ground (solera).
    pub base: bool,
}

/// The loads per m² of plan at a point of plate `i`: dead, the live load
/// of the use there and the snow if it is open to the sky; with the
/// snow per m² of the plate.
pub fn area_loads(
    model: &Model,
    plates: &[Plate],
    i: usize,
    rooms: &[&archisim_core::Room],
    p: Point2,
) -> Vec<(Case, f64)> {
    let pl = &plates[i];
    let building = model.code_settings().building_use;
    let site = model.structural_site();
    let snow = roof_snow(site, model.site_altitude(), pl.pitch);
    let mut out = vec![(Case::Dead, pl.dead)];
    let z = (pl.top)(p);
    let covered = plates
        .iter()
        .enumerate()
        .any(|(j, o)| j != i && (o.top)(p) > z + 0.5 && geo::inside(p, &o.outline));
    let room = rooms.iter().find(|r| geo::inside(p, &r.outline));
    let (cat, q) = match room {
        Some(r) => {
            let c = room_category(r, building);
            (c, c.uniform())
        }
        None if pl.roof || (!covered && !pl.base) => {
            weights::roof_live(pl.pitch.to_degrees(), pl.walkable)
        }
        None => {
            let c = crate::actions::category_of_use(building);
            (c, c.uniform())
        }
    };
    if q > 0.0 {
        out.push((Case::Live(cat), q));
    }
    if !covered && snow > 0.0 {
        out.push((Case::Snow, snow));
    }
    out
}

/// Something that sends its load down: what the order of the takedown
/// goes by.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Item {
    Plate(usize),
    Carrier(Carrier),
}

/// A support of a plate: a carrier along a line (or at a point).
pub struct Support {
    pub carrier: Carrier,
    pub path: Vec<Point2>,
}

/// What holds a plate up (every wall, beam and column under it).
pub fn plate_supports(model: &Model, td: &Takedown, pl: &Plate) -> Vec<Support> {
    supports_of(model, td, pl, &Default::default())
}

fn level_z(model: &Model, l: LevelId) -> f64 {
    model.level(l).map_or(0.0, |l| l.elevation)
}

pub fn plates(model: &Model) -> Vec<Plate> {
    let mut out = Vec::new();
    for s in model.slabs() {
        let Some(t) = model.slab_type(s.slab_type) else {
            continue;
        };
        let z0 = level_z(model, s.level);
        let slope = s.slope;
        let off = s.top_offset;
        let span = model.type_framing(s.slab_type.0).and_then(|_| {
            let mut members = archisim_geom::framing::slab_members(model, s.id);
            members.retain(|m| m.role == archisim_geom::framing::Role::Joist);
            members
                .first()
                .and_then(|m| Vec2::new(m.end[0] - m.start[0], m.end[1] - m.start[1]).normalized())
        });
        let pitch = slope.map_or(0.0, |sl| sl.slope.abs().atan());
        let name_l = t.name.to_lowercase();
        out.push(Plate {
            source: PlateSource::Slab(s.id),
            name: t.name.clone(),
            level: s.level,
            outline: s.outline.clone(),
            holes: s.holes.clone(),
            top: Box::new(move |p| z0 + off + slope.map_or(0.0, |sl| sl.rise_at(p))),
            thickness: t.thickness(),
            dead: weights::slab_type_weight(model, s.slab_type)
                * (1.0 + pitch.tan().powi(2)).sqrt(),
            pitch,
            roof: t.function == SlabFunction::Roof,
            base: t.function == SlabFunction::BaseSlab,
            walkable: name_l.contains("transitable") && !name_l.contains("no transitable"),
            span,
            tiles: false,
        });
    }
    for r in model.roofs() {
        let Some(t) = model.slab_type(r.roof_type) else {
            continue;
        };
        let z0 = level_z(model, r.level);
        let framed = model.type_framing(r.roof_type.0).is_some();
        for plane in r.planes() {
            let slope = plane.slope;
            let off = r.base_offset;
            let pitch = slope.slope.abs().atan();
            out.push(Plate {
                source: PlateSource::Roof(r.id, plane.side),
                name: t.name.clone(),
                level: r.level,
                outline: plane.outline.clone(),
                holes: Vec::new(),
                top: Box::new(move |p| z0 + off + slope.rise_at(p)),
                thickness: t.thickness(),
                dead: weights::slab_type_weight(model, r.roof_type)
                    * (1.0 + pitch.tan().powi(2)).sqrt()
                    + if r.covering.tiles.is_some() {
                        TILES
                    } else {
                        0.0
                    },
                pitch,
                roof: true,
                walkable: false,
                span: if framed { slope.direction() } else { None },
                tiles: r.covering.tiles.is_some(),
                base: false,
            });
        }
    }
    out
}

fn member_weight(model: &Model, t: archisim_core::MemberTypeId) -> (f64, f64) {
    model.member_type(t).map_or((0.0, 0.3), |mt| {
        let gamma = model
            .material(mt.material)
            .map_or(25.0, weights::specific_weight);
        (mt.profile.area() * gamma, mt.profile.size().1)
    })
}

/// The takedown of the model.
pub fn takedown(model: &Model) -> Takedown {
    let site = *model.structural_site();
    let altitude = model.site_altitude();
    let building = model.code_settings().building_use;
    let lowest = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min);
    let lowest = if lowest.is_finite() { lowest } else { 0.0 };
    let mut td = Takedown::default();
    let plates = plates(model);
    td.plate_incoming = vec![Vec::new(); plates.len()];
    // The carriers.
    for w in model.walls() {
        let base = level_z(model, w.level) + w.base_offset;
        let c = Carried::new(geo::wall_path(w), base + w.height, base);
        let mut c = c;
        let g = weights::wall_type_weight(model, w.wall_type) * w.height;
        let n = c.own.len();
        for (i, own) in c.own.iter_mut().enumerate() {
            let l = if i + 1 == n {
                c.length - BIN * (n - 1) as f64
            } else {
                BIN
            };
            *own = g * l.max(0.0);
        }
        td.carriers.insert(Carrier::Wall(w.id), c);
    }
    let mut beam_depth: HashMap<BeamId, f64> = HashMap::new();
    for b in model.beams() {
        let top = level_z(model, b.level) + b.top_offset;
        let (wpm, depth) = member_weight(model, b.member_type);
        beam_depth.insert(b.id, depth);
        let mut c = Carried::new(vec![b.start, b.end], top, top - depth);
        let n = c.own.len();
        let len = c.length;
        for (i, own) in c.own.iter_mut().enumerate() {
            let l = if i + 1 == n {
                len - BIN * (n - 1) as f64
            } else {
                BIN
            };
            *own = wpm * l.max(0.0);
        }
        td.carriers.insert(Carrier::Beam(b.id), c);
    }
    for col in model.columns() {
        let base = level_z(model, col.level) + col.base_offset;
        let (wpm, _) = member_weight(model, col.member_type);
        let mut c = Carried::new(vec![col.position], base + col.height, base);
        c.length = 0.0;
        c.own = vec![wpm * col.height];
        td.carriers.insert(Carrier::Column(col.id), c);
    }
    // The order: from the highest point where each thing receives.
    let mut order: Vec<(f64, u8, Item)> = Vec::new();
    for (i, p) in plates.iter().enumerate() {
        let z = p
            .outline
            .iter()
            .map(|q| (p.top)(*q))
            .fold(f64::NEG_INFINITY, f64::max);
        order.push((z, 0, Item::Plate(i)));
    }
    // Beams that land on other beams go before them.
    let beam_rank = |id: BeamId| -> u8 {
        let mut rank = 0u8;
        let mut cur = id;
        for _ in 0..8 {
            let Some(b) = model.beam(cur) else { break };
            let top = level_z(model, b.level) + b.top_offset;
            let next = model.beams().find(|o| {
                o.id != cur
                    && (level_z(model, o.level) + o.top_offset - top).abs() < 0.3
                    && [b.start, b.end]
                        .iter()
                        .any(|e| geo::to_segment(*e, o.start, o.end).0 < 0.3)
                    && geo::to_segment(o.start, b.start, b.end).0 > 0.3
                    && geo::to_segment(o.end, b.start, b.end).0 > 0.3
            });
            match next {
                Some(o) => {
                    rank += 1;
                    cur = o.id;
                }
                None => break,
            }
        }
        rank
    };
    for (k, c) in &td.carriers {
        let (z, rank) = match k {
            Carrier::Beam(id) => (c.top - 0.001, 1 + 10u8.saturating_sub(beam_rank(*id))),
            Carrier::Column(_) => (c.top - 0.002, 30),
            Carrier::Wall(_) => (c.top - 0.003, 40),
        };
        order.push((z, rank, Item::Carrier(*k)));
    }
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    // Loads arriving on plates from above (point, case, kN).
    let mut incoming: Vec<Vec<(Point2, Case, f64)>> = vec![Vec::new(); plates.len()];
    let mut processed: std::collections::HashSet<Carrier> = Default::default();
    let mut cases: std::collections::BTreeSet<Case> = Default::default();
    cases.insert(Case::Dead);
    // The hand loads go to the plate they lie on, or the ground.
    for l in model.structural_loads() {
        let case = match l.case {
            LoadCase::Dead => Case::Dead,
            LoadCase::Live => Case::Live(crate::actions::category_of_use(building)),
            LoadCase::Snow => Case::Snow,
            LoadCase::Wind => continue,
        };
        if l.direction.is_some() {
            continue;
        }
        let z = level_z(model, l.level) + l.height;
        let pts: Vec<(Point2, f64)> = match &l.shape {
            LoadShape::Point { at } => vec![(*at, l.value)],
            LoadShape::Line { start, end } => {
                let len = start.distance(*end);
                let n = (len / STEP).ceil().max(1.0) as usize;
                (0..n)
                    .map(|i| {
                        let t = (i as f64 + 0.5) / n as f64;
                        (*start + (*end - *start) * t, l.value * len / n as f64)
                    })
                    .collect()
            }
            LoadShape::Area { outline } => geo::grid(outline, &[], STEP)
                .into_iter()
                .map(|p| (p, l.value * STEP * STEP))
                .collect(),
        };
        for (p, kn) in pts {
            cases.insert(case);
            *td.applied.entry(case).or_default() += kn;
            let on = plates.iter().position(|pl| {
                ((pl.top)(p) - z).abs() < 0.4 && geo::near_polygon(p, &pl.outline, 0.05)
            });
            match on {
                Some(i) => incoming[i].push((p, case, kn)),
                None => td.ground.push(GroundLoad {
                    at: p,
                    case,
                    kn,
                    from: None,
                }),
            }
        }
    }
    for (_, _, item) in order {
        match item {
            Item::Plate(i) => {
                let pl = &plates[i];
                let supports = supports_of(model, &td, pl, &processed);
                let mut surface = Surface {
                    name: pl.name.clone(),
                    level: pl.level,
                    area: 0.0,
                    dead: pl.dead,
                    uses: BTreeMap::new(),
                    snow: roof_snow(&site, altitude, pl.pitch),
                    exposed: 0.0,
                    one_way: pl.span.is_some(),
                    on_ground: false,
                    unsupported: false,
                };
                let cell = STEP * STEP;
                let mut loads: Vec<(Point2, Case, f64)> = Vec::new();
                let rooms: Vec<_> = model.rooms_on_level(pl.level).collect();
                for p in geo::grid(&pl.outline, &pl.holes, STEP) {
                    surface.area += cell;
                    for (case, q) in area_loads(model, &plates, i, &rooms, p) {
                        loads.push((p, case, q * cell));
                        match case {
                            Case::Live(cat) => {
                                let u = surface.uses.entry(cat).or_default();
                                u.0 += cell;
                                u.1 += q * cell;
                            }
                            Case::Snow => surface.exposed += cell,
                            _ => {}
                        }
                    }
                }
                for (_, case, kn) in &loads {
                    *td.applied.entry(*case).or_default() += kn;
                }
                td.plate_incoming[i] = incoming[i].clone();
                loads.append(&mut incoming[i]);
                let on_ground = supports.is_empty()
                    && pl
                        .outline
                        .iter()
                        .map(|q| (pl.top)(*q))
                        .fold(f64::INFINITY, f64::min)
                        <= lowest + 0.6;
                surface.on_ground = on_ground;
                if supports.is_empty() && !on_ground {
                    surface.unsupported = true;
                    td.notes.push(format!(
                        "«{}» ({:.1} m²) no se apoya en ningún muro, viga ni pilar: su carga se lleva al terreno.",
                        pl.name, surface.area
                    ));
                }
                for (p, case, kn) in loads {
                    cases.insert(case);
                    if supports.is_empty() {
                        td.ground.push(GroundLoad {
                            at: p,
                            case,
                            kn,
                            from: None,
                        });
                        continue;
                    }
                    for (k, along, share) in distribute(p, pl.span, &supports) {
                        if let Some(c) = td.carriers.get_mut(&k) {
                            c.add(case, along, kn * share);
                        }
                    }
                }
                td.surfaces.push(surface);
            }
            Item::Carrier(k) => {
                deliver(
                    model,
                    &mut td,
                    &plates,
                    &mut incoming,
                    &processed,
                    k,
                    &beam_depth,
                );
                processed.insert(k);
            }
        }
    }
    td.cases = cases.into_iter().collect();
    td
}

/// The supports of a plate: walls, beams and columns just under it.
fn supports_of(
    model: &Model,
    td: &Takedown,
    pl: &Plate,
    processed: &std::collections::HashSet<Carrier>,
) -> Vec<Support> {
    let mut out = Vec::new();
    for (k, c) in &td.carriers {
        if processed.contains(k) {
            continue;
        }
        let reach = |p: Point2| geo::near_polygon(p, &pl.outline, 0.3);
        let under = |p: Point2| {
            let face = (pl.top)(p) - pl.thickness;
            c.top >= face - 0.45 && c.top <= (pl.top)(p) + 0.05
        };
        let ok =
            match k {
                Carrier::Wall(id) => {
                    let trimmed = model.wall_roof(*id).is_some() && pl.roof;
                    let mid = c.point(c.length / 2.0);
                    let touching =
                        c.path.iter().copied().chain(std::iter::once(mid)).any(|p| {
                            reach(p) && (under(p) || trimmed && ((pl.top)(p) - c.base) > 0.5)
                        });
                    touching
                }
                Carrier::Beam(_) => c.path.iter().any(|p| reach(*p) && under(*p)),
                Carrier::Column(_) => reach(c.path[0]) && under(c.path[0]),
            };
        if ok {
            out.push(Support {
                carrier: *k,
                path: c.path.clone(),
            });
        }
    }
    out
}

/// Where a load at `p` on a plate goes: (carrier, along it, share).
fn distribute(p: Point2, span: Option<Vec2>, supports: &[Support]) -> Vec<(Carrier, f64, f64)> {
    // Right over a support: straight into it.
    let over = supports
        .iter()
        .filter(|s| s.path.len() > 1)
        .map(|s| {
            let (d, a) = geo::to_path(p, &s.path);
            (d, a, s.carrier)
        })
        .filter(|(d, _, _)| *d < 0.12)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, a, k)) = over {
        return vec![(k, a, 1.0)];
    }
    if let Some(d) = span {
        let hit = |dir: Vec2| {
            supports
                .iter()
                .filter(|s| s.path.len() > 1)
                .filter_map(|s| geo::ray_hit(p, dir, &s.path, 30.0).map(|(t, a)| (t, a, s.carrier)))
                .min_by(|a, b| a.0.total_cmp(&b.0))
        };
        match (hit(d), hit(-d)) {
            (Some((ta, aa, ka)), Some((tb, ab, kb))) => {
                let l = ta + tb;
                return vec![(ka, aa, tb / l), (kb, ab, ta / l)];
            }
            (Some((_, a, k)), None) | (None, Some((_, a, k))) => return vec![(k, a, 1.0)],
            (None, None) => {}
        }
    }
    // Two-way: the nearest support.
    let best = supports
        .iter()
        .map(|s| {
            if s.path.len() == 1 {
                (p.distance(s.path[0]), 0.0, s.carrier)
            } else {
                let (d, a) = geo::to_path(p, &s.path);
                (d, a, s.carrier)
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    best.map(|(_, a, k)| vec![(k, a, 1.0)]).unwrap_or_default()
}

/// Passes a carrier's load on to what is under it.
fn deliver(
    model: &Model,
    td: &mut Takedown,
    plates: &[Plate],
    incoming: &mut [Vec<(Point2, Case, f64)>],
    processed: &std::collections::HashSet<Carrier>,
    k: Carrier,
    beam_depth: &HashMap<BeamId, f64>,
) {
    let Some(c) = td.carriers.get(&k).cloned() else {
        return;
    };
    let _ = beam_depth;
    // The loads to pass on: per stretch, by case.
    let mut total: BTreeMap<Case, Vec<f64>> = c.received.clone();
    let n = c.own.len();
    {
        let dead = total.entry(Case::Dead).or_insert_with(|| vec![0.0; n]);
        for (d, o) in dead.iter_mut().zip(&c.own) {
            *d += o;
        }
    }
    if let Some(cc) = td.carriers.get_mut(&k) {
        cc.delivered = total.clone();
    }
    let base = c.base;
    // A point at the base: where does it go?
    let target = |p: Point2, td: &Takedown| -> Target {
        // A plate whose top is at the base.
        if let Some(i) = plates.iter().position(|pl| {
            ((pl.top)(p) - base).abs() < 0.15 && geo::near_polygon(p, &pl.outline, 0.15)
        }) {
            return Target::Plate(i);
        }
        let mut best: Option<(f64, Carrier, f64)> = None;
        for (o, oc) in &td.carriers {
            if *o == k || processed.contains(o) {
                continue;
            }
            if !(oc.top >= base - 0.6 && oc.top <= base + 0.05) {
                continue;
            }
            let (d, along) = if oc.path.len() == 1 {
                (p.distance(oc.path[0]), 0.0)
            } else {
                geo::to_path(p, &oc.path)
            };
            let tol = if matches!(o, Carrier::Column(_)) {
                0.4
            } else {
                0.3
            };
            if d < tol && best.is_none_or(|b| d < b.0) {
                best = Some((d, *o, along));
            }
        }
        match best {
            Some((_, o, along)) => Target::Carrier(o, along),
            None => Target::Ground,
        }
    };
    match k {
        Carrier::Wall(_) | Carrier::Column(_) => {
            for (case, v) in &total {
                for (i, kn) in v.iter().enumerate() {
                    if *kn == 0.0 {
                        continue;
                    }
                    let along = if c.length > 0.0 {
                        ((i as f64 + 0.5) * BIN).min(c.length)
                    } else {
                        0.0
                    };
                    let p = c.point(along);
                    match target(p, td) {
                        Target::Plate(j) => incoming[j].push((p, *case, *kn)),
                        Target::Carrier(o, a) => {
                            let framed = is_frame(k) && is_frame(o);
                            if let Some(oc) = td.carriers.get_mut(&o) {
                                oc.add(*case, a, *kn);
                                if framed {
                                    oc.add_from_frame(*case, a, *kn);
                                }
                            }
                        }
                        Target::Ground => td.ground.push(GroundLoad {
                            at: p,
                            case: *case,
                            kn: *kn,
                            from: Some(k),
                        }),
                    }
                }
            }
        }
        Carrier::Beam(_) => {
            // Its supports along it: what is under its ends and under it.
            let mut sup: Vec<(f64, Target, Point2)> = Vec::new();
            let probe =
                |sup: &mut Vec<(f64, Target, Point2)>, along: f64, td: &Takedown, end: bool| {
                    let p = c.point(along);
                    let t = target(p, td);
                    let real = !matches!(t, Target::Ground) || end;
                    if real && !sup.iter().any(|(a, _, _)| (a - along).abs() < 0.3) {
                        sup.push((along, t, p));
                    }
                };
            probe(&mut sup, 0.0, td, false);
            probe(&mut sup, c.length, td, false);
            // Columns and walls under its middle.
            let mids: Vec<f64> = td
                .carriers
                .iter()
                .filter(|(o, oc)| {
                    **o != k
                        && !processed.contains(o)
                        && oc.top >= base - 0.6
                        && oc.top <= base + 0.05
                })
                .filter_map(|(_, oc)| {
                    let p = if oc.path.len() == 1 {
                        oc.path[0]
                    } else {
                        oc.point(oc.length / 2.0)
                    };
                    let (d, a) = geo::to_path(p, &c.path);
                    (d < 0.3 && a > 0.3 && a < c.length - 0.3).then_some(a)
                })
                .collect();
            for a in mids {
                probe(&mut sup, a, td, false);
            }
            if sup.is_empty() {
                // Nothing under it: to the ground at its ends.
                probe(&mut sup, 0.0, td, true);
                probe(&mut sup, c.length, td, true);
                td.notes.push(format!(
                    "Una viga de {:.2} m no encuentra apoyos: su carga se lleva al terreno.",
                    c.length
                ));
            }
            sup.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (case, v) in &total {
                for (i, kn) in v.iter().enumerate() {
                    if *kn == 0.0 {
                        continue;
                    }
                    let a = ((i as f64 + 0.5) * BIN).min(c.length);
                    for (j, share) in lever(&sup, a) {
                        let (_, t, p) = sup[j];
                        let part = kn * share;
                        match t {
                            Target::Plate(pi) => incoming[pi].push((p, *case, part)),
                            Target::Carrier(o, along) => {
                                let framed = is_frame(o);
                                if let Some(oc) = td.carriers.get_mut(&o) {
                                    oc.add(*case, along, part);
                                    if framed {
                                        oc.add_from_frame(*case, along, part);
                                    }
                                }
                            }
                            Target::Ground => td.ground.push(GroundLoad {
                                at: p,
                                case: *case,
                                kn: part,
                                from: Some(k),
                            }),
                        }
                    }
                }
            }
        }
    }
    let _ = model;
}

fn is_frame(k: Carrier) -> bool {
    matches!(k, Carrier::Beam(_) | Carrier::Column(_))
}

#[derive(Debug, Clone, Copy)]
enum Target {
    Plate(usize),
    Carrier(Carrier, f64),
    Ground,
}

/// The lever rule between the supports either side of `a` (supports
/// sorted along the beam): beyond the last support, all to it.
fn lever(sup: &[(f64, Target, Point2)], a: f64) -> Vec<(usize, f64)> {
    match sup.len() {
        0 => Vec::new(),
        1 => vec![(0, 1.0)],
        n => {
            if a <= sup[0].0 {
                return vec![(0, 1.0)];
            }
            if a >= sup[n - 1].0 {
                return vec![(n - 1, 1.0)];
            }
            for j in 0..n - 1 {
                let (s0, s1) = (sup[j].0, sup[j + 1].0);
                if a >= s0 && a <= s1 {
                    let t = (a - s0) / (s1 - s0);
                    return vec![(j, 1.0 - t), (j + 1, t)];
                }
            }
            vec![(n - 1, 1.0)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    fn wall(
        m: &mut Model,
        t: WallTypeId,
        level: LevelId,
        a: (f64, f64),
        b: (f64, f64),
        h: f64,
    ) -> WallId {
        m.add_wall(NewWall {
            wall_type: t,
            level,
            start: Point2::new(a.0, a.1),
            end: Point2::new(b.0, b.1),
            bulge: 0.0,
            reference: ReferenceLine::Center,
            height: h,
            base_offset: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap()
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    /// A box of four walls, 6 × 4 m, 3 m high, with a flat roof slab.
    fn hut() -> (Model, archisim_library::Library, [WallId; 4]) {
        let mut m = Model::new("Caseta");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let t = lib.bearing_wall;
        let w = [
            wall(&mut m, t, l0, (0.0, 0.0), (6.0, 0.0), 2.7),
            wall(&mut m, t, l0, (6.0, 0.0), (6.0, 4.0), 2.7),
            wall(&mut m, t, l0, (6.0, 4.0), (0.0, 4.0), 2.7),
            wall(&mut m, t, l0, (0.0, 4.0), (0.0, 0.0), 2.7),
        ];
        m.add_slab(NewSlab {
            slab_type: lib.flat_roof,
            level: l1,
            outline: rect(-0.15, -0.15, 6.15, 4.15),
            top_offset: 0.0,
        })
        .unwrap();
        (m, lib, w)
    }

    #[test]
    fn what_goes_up_comes_down() {
        let (m, _, walls) = hut();
        let td = takedown(&m);
        // Everything ends on the ground: the roof's weight, snow and
        // upkeep load, and the walls' weight.
        let roof: f64 = td.surfaces.iter().map(|s| s.area * s.dead).sum();
        let walls_g: f64 = walls
            .iter()
            .map(|w| td.carriers[&Carrier::Wall(*w)].own.iter().sum::<f64>())
            .sum();
        let down = td.ground_total(Case::Dead);
        assert!(
            (down - roof - walls_g).abs() < 1e-6 * down.max(1.0),
            "{down} vs {roof} + {walls_g}"
        );
        let snow = td.ground_total(Case::Snow);
        let area = td.surfaces[0].area;
        // Huelva: 0,2 kN/m² on a flat roof.
        assert!((snow - 0.2 * area).abs() < 1e-6, "{snow}");
        assert!(td.cases.contains(&Case::Live(UseCategory::G1)));
        // The long walls take more of the roof than the short ones.
        let long = td.carriers[&Carrier::Wall(walls[0])].received_total(Case::Dead);
        let short = td.carriers[&Carrier::Wall(walls[1])].received_total(Case::Dead);
        assert!(long > short * 1.3, "{long} vs {short}");
        assert!(td.notes.is_empty(), "{:?}", td.notes);
    }

    #[test]
    fn a_beam_passes_its_load_to_its_columns() {
        let mut m = Model::new("Pórtico");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut col = |x: f64| {
            m.add_column(NewColumn {
                member_type: lib.column,
                level: l0,
                position: Point2::new(x, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 2.5,
            })
            .unwrap()
        };
        let c0 = col(0.0);
        let c1 = col(5.0);
        m.add_beam(NewBeam {
            member_type: lib.beam,
            level: l1,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            top_offset: 0.0,
        })
        .unwrap();
        m.add_structural_load(StructuralLoad {
            id: 0,
            name: "Máquina".into(),
            level: l1,
            height: 0.0,
            shape: LoadShape::Point {
                at: Point2::new(1.0, 0.0),
            },
            case: LoadCase::Dead,
            value: 0.0,
            direction: None,
        })
        .unwrap();
        // A slab 1 m wide over the beam, 5 m long.
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: rect(0.0, -0.5, 5.0, 0.5),
            top_offset: 0.0,
        })
        .unwrap();
        let td = takedown(&m);
        let a = td.carriers[&Carrier::Column(c0)].received_total(Case::Dead);
        let b = td.carriers[&Carrier::Column(c1)].received_total(Case::Dead);
        assert!(a > 0.0 && (a - b).abs() < 0.05 * a, "{a} {b}");
        // The columns' own weight goes to the ground under them.
        let ground: f64 = td
            .ground
            .iter()
            .filter(|g| g.case == Case::Dead && g.from == Some(Carrier::Column(c0)))
            .map(|g| g.kn)
            .sum();
        assert!(ground > a, "{ground} {a}");
    }
}
