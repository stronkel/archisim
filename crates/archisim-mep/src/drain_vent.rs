//! The ventilation of the drainage and its small evacuation network
//! (DB-HS 5, third round), on the waste networks of [`crate::drain`]:
//!
//! - **Primary ventilation** (3.3.3.1): every downpipe column (the
//!   downpipes one over another at a place) goes up through the roof over
//!   it, 1,30 m over a roof not walked on and 2,00 m over one walked on;
//!   within 6 m of an air intake it is higher than the intake, and within
//!   6 m of a window it is 50 cm over the window's head. An air admittance
//!   valve at its head stands for it (3.3.3.4).
//! - **Secondary ventilation** (3.3.3.2): a column serving 7 floors or
//!   more (11 if drawn a size larger than asked) has its vent column
//!   beside it (a downpipe drawn with no water in it), of half its
//!   diameter at least, or a valve on every floor's branch.
//! - **Tertiary ventilation** (3.3.3.3): a floor's branch longer than
//!   5 m, or every branch of a column serving more than 14 floors, takes
//!   a valve or a vent column at its end.
//! - **Traps** (3.3.1.1): each appliance has its own trap or drains
//!   through a trap box (`DeviceKind::TrapBox`); the seal is the
//!   project's (`WaterSite::trap_seal`, 50 to 100 mm). A trap box takes
//!   only appliances of its room and no WC, its branches up to 2,50 m at
//!   2 to 4 %, and is 2,00 m at most from the downpipe.
//! - **Each appliance to its downpipe** (3.3.1.4): basins, bidets, sinks,
//!   dishwashers and washing machines at 4 m at most and 2,5 to 5 %;
//!   baths and showers at 4 m and 10 % at most; a WC 1 m at most. With no
//!   downpipe on its way (the lowest floor), to the first collector.
//!
//! The distances are along the pipes drawn, from the end where the
//! appliance drains.

use std::collections::{BTreeMap, BTreeSet};

use archisim_core::{
    Appliance, DeviceId, DeviceKind, LevelId, Model, ObjectId, OpeningKind, Point2, RoofAccess,
    RoomId,
};

use crate::drain::{inside, pvc_at_least, Drainage, Kind};
use crate::water::level_z;

// ---------------------------------------------------------------------
// Values of DB-HS 5 written from memory of its text, to be checked
// against the official document («para revisar»).
// ---------------------------------------------------------------------

/// 3.3.3.1.2: how far the primary ventilation goes over a roof not walked
/// on, and over one walked on, metres (para revisar).
pub const ABOVE_ROOF: f64 = 1.30;
pub const ABOVE_WALKED_ROOF: f64 = 2.00;
/// 3.3.3.1.3 and 3.3.3.1.4: within this of an air intake the outlet is
/// above it, and above the head of a window by `OVER_WINDOWS`, metres
/// (para revisar).
pub const INTAKE_GAP: f64 = 6.0;
pub const OVER_WINDOWS: f64 = 0.50;
/// 3.3.3.1.1: the primary ventilation is enough under this many floors,
/// or under `PRIMARY_FLOORS_OVERSIZED` with the downpipe a size larger
/// than asked (para revisar: the brief says 14; the text read, 11).
pub const PRIMARY_FLOORS: usize = 7;
pub const PRIMARY_FLOORS_OVERSIZED: usize = 11;
/// 3.3.3.3.1: tertiary ventilation for branches longer than this, metres,
/// or columns serving more than `TERTIARY_FLOORS` floors (para revisar).
pub const LONG_BRANCH: f64 = 5.0;
pub const TERTIARY_FLOORS: usize = 14;
/// 4.4.2 and 4.4.3: a vent column of half the downpipe's diameter at least
/// and a branch's vent of half the branch's; never less than `LEAST_VENT`
/// (para revisar: tablas 4.10 and 4.11, by the length, are not applied).
pub const VENT_FRACTION: f64 = 0.5;
pub const LEAST_VENT: f64 = 0.032;
/// 3.3.1.1.2 e): the water seal of a trap, metres (para revisar; 70 mm in
/// discontinuous use).
pub const SEAL_MIN: f64 = 0.05;
pub const SEAL_MAX: f64 = 0.10;
/// 3.3.1.4 c) and d): a trap box at 2,00 m at most from the downpipe, its
/// branches up to 2,50 m at 2 to 4 % (para revisar).
pub const BOX_TO_DOWNPIPE: f64 = 2.0;
pub const BOX_BRANCH: f64 = 2.5;
pub const BOX_SLOPE: (f64, f64) = (0.02, 0.04);

/// 3.3.1.4 e): how far an appliance with its own trap may be from the
/// downpipe (metres) and the slope of its branch (para revisar; the 4 m
/// of baths and showers comes from the brief, the text gives only the
/// slope).
pub fn limits(a: Appliance) -> (f64, f64, f64) {
    match a {
        Appliance::Wc => (1.0, 0.0, f64::INFINITY),
        Appliance::Bath | Appliance::Shower => (4.0, 0.02, 0.10),
        Appliance::Washbasin
        | Appliance::Bidet
        | Appliance::Sink
        | Appliance::Dishwasher
        | Appliance::Washer => (4.0, 0.025, 0.05),
    }
}

/// How near (in plan) a valve, a trap box or a vent column is looked for:
/// at a column's head, on a branch, beside a column, metres.
const HEAD_REACH: f64 = 0.3;
const ON_PIPE: f64 = 0.5;
const VENT_REACH: f64 = 1.0;
/// Downpipes this near in plan are one column, metres.
const COLUMN_REACH: f64 = 0.2;

/// The roof over a downpipe column.
#[derive(Debug, Clone, PartialEq)]
pub struct Cover {
    pub name: String,
    pub level: LevelId,
    /// Its top at the column, absolute, metres.
    pub z: f64,
    pub walked: bool,
}

/// A floor's branch into a column: the drains of a floor that reach the
/// column by the same pipe.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorBranch {
    pub level: LevelId,
    /// Its stretch (the last one before the downpipe).
    pub stretch: usize,
    pub appliances: usize,
    /// Its farthest appliance from the downpipe, along the pipes, metres.
    pub length: f64,
    /// It needs tertiary ventilation.
    pub needed: bool,
    /// A valve on it.
    pub valve: Option<DeviceId>,
    /// A vent column at its end, and its diameter.
    pub vent: Option<(DeviceId, f64)>,
    /// The vent's diameter asked, metres.
    pub vent_sized: f64,
}

impl FloorBranch {
    pub fn vented(&self) -> bool {
        self.valve.is_some()
            || self
                .vent
                .is_some_and(|(_, d)| d + 0.0015 >= self.vent_sized)
    }

    pub fn fails(&self) -> bool {
        self.needed && !self.vented()
    }
}

/// A downpipe column of a waste network and its ventilation.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub network: usize,
    /// Its downpipes, the lowest first.
    pub devices: Vec<DeviceId>,
    /// The one at the top.
    pub head: DeviceId,
    pub at: Point2,
    /// Its foot and head, absolute, metres.
    pub bottom: f64,
    pub top: f64,
    /// Diameter drawn and asked, metres.
    pub drawn: f64,
    pub sized: f64,
    /// Drawn a size larger than asked (3.3.3.1.1).
    pub oversized: bool,
    /// The floors that drain into it.
    pub floors: usize,
    pub cover: Option<Cover>,
    /// An air admittance valve at its head.
    pub valve: Option<DeviceId>,
    /// What asks the outlet the highest besides the roof: an intake or a
    /// window within 6 m (what, the height asked, absolute).
    pub near: Option<(String, f64)>,
    /// The vent column beside it, and its diameter.
    pub vent: Option<(DeviceId, f64)>,
    /// The vent column's diameter asked, metres.
    pub vent_sized: f64,
    pub branches: Vec<FloorBranch>,
}

impl Column {
    /// How far over the roof it must go, metres.
    pub fn above_required(&self) -> f64 {
        match &self.cover {
            Some(c) if c.walked => ABOVE_WALKED_ROOF,
            _ => ABOVE_ROOF,
        }
    }

    /// How high its head must be, absolute (none: no roof over it).
    pub fn required_top(&self) -> Option<f64> {
        let roof = self.cover.as_ref()?.z + self.above_required();
        Some(match &self.near {
            Some((_, z)) => roof.max(*z),
            None => roof,
        })
    }

    /// How far it goes over its roof, metres.
    pub fn above(&self) -> Option<f64> {
        self.cover.as_ref().map(|c| self.top - c.z)
    }

    /// The primary ventilation fails: no valve and its head too low.
    pub fn primary_fails(&self) -> bool {
        self.valve.is_none() && self.required_top().is_some_and(|z| self.top + 1e-6 < z)
    }

    /// It needs secondary ventilation (3.3.3.1.1).
    pub fn secondary_needed(&self) -> bool {
        let most = if self.oversized {
            PRIMARY_FLOORS_OVERSIZED
        } else {
            PRIMARY_FLOORS
        };
        self.floors >= most
    }

    /// Every floor's branch has its valve (3.3.3.4).
    pub fn valves_on_branches(&self) -> bool {
        !self.branches.is_empty() && self.branches.iter().all(|b| b.valve.is_some())
    }

    pub fn secondary_fails(&self) -> bool {
        self.secondary_needed()
            && !self.valves_on_branches()
            && !self
                .vent
                .is_some_and(|(_, d)| d + 0.0015 >= self.vent_sized)
    }
}

/// An appliance on its way to the downpipe.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplianceWay {
    pub object: ObjectId,
    pub kind: Appliance,
    pub level: LevelId,
    pub network: usize,
    /// Along the pipes to the downpipe (or the first collector), metres;
    /// none if the way has neither.
    pub length: Option<f64>,
    /// To a downpipe (false: to a collector).
    pub to_downpipe: bool,
    /// The least and the most slope of its own branch (the shared
    /// branches go by tabla 4.3).
    pub slopes: Option<(f64, f64)>,
    /// The trap box it drains through.
    pub trap_box: Option<DeviceId>,
}

impl ApplianceWay {
    pub fn limits(&self) -> (f64, f64, f64) {
        limits(self.kind)
    }

    pub fn too_far(&self) -> bool {
        self.trap_box.is_none() && self.length.is_some_and(|l| l > self.limits().0 + 1e-6)
    }

    pub fn bad_slope(&self) -> bool {
        let (_, lo, hi) = self.limits();
        self.trap_box.is_none()
            && self
                .slopes
                .is_some_and(|(a, b)| a + 1e-6 < lo || b > hi + 1e-6)
    }
}

/// An appliance on a trap box: which, how far from the box along the
/// pipes (metres) and the slopes of its branch.
pub type BoxedAppliance = (ObjectId, Appliance, f64, Option<(f64, f64)>);

/// A trap box and what drains through it.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapBoxCheck {
    pub device: DeviceId,
    pub level: LevelId,
    pub room: Option<RoomId>,
    /// Its appliances, how far each is from it and the slopes of its
    /// branch.
    pub appliances: Vec<BoxedAppliance>,
    /// Appliances of another room.
    pub other_rooms: usize,
    /// Along the pipes to the downpipe, metres.
    pub to_downpipe: Option<f64>,
}

impl TrapBoxCheck {
    pub fn has_wc(&self) -> bool {
        self.appliances.iter().any(|a| a.1 == Appliance::Wc)
    }

    pub fn long_branches(&self) -> usize {
        self.appliances
            .iter()
            .filter(|a| a.2 > BOX_BRANCH + 1e-6)
            .count()
    }

    pub fn bad_slopes(&self) -> usize {
        self.appliances
            .iter()
            .filter(|a| {
                a.3.is_some_and(|(lo, hi)| lo + 1e-6 < BOX_SLOPE.0 || hi > BOX_SLOPE.1 + 1e-6)
            })
            .count()
    }

    pub fn too_far(&self) -> bool {
        self.to_downpipe.is_some_and(|d| d > BOX_TO_DOWNPIPE + 1e-6)
    }

    pub fn fails(&self) -> bool {
        self.has_wc()
            || self.other_rooms > 0
            || self.long_branches() > 0
            || self.bad_slopes() > 0
            || self.too_far()
    }
}

/// The ventilation and the small evacuation network of the drainage.
#[derive(Debug, Clone, PartialEq)]
pub struct Venting {
    pub columns: Vec<Column>,
    pub ways: Vec<ApplianceWay>,
    pub trap_boxes: Vec<TrapBoxCheck>,
    /// Trap boxes on no appliance's way.
    pub loose_boxes: Vec<DeviceId>,
    /// The traps' water seal, metres.
    pub seal: f64,
}

impl Venting {
    pub fn seal_fails(&self) -> bool {
        self.seal + 1e-6 < SEAL_MIN || self.seal > SEAL_MAX + 1e-6
    }

    /// How many things fail, all told.
    pub fn fails(&self) -> usize {
        self.columns
            .iter()
            .map(|c| {
                usize::from(c.primary_fails())
                    + usize::from(c.secondary_fails())
                    + c.branches.iter().filter(|b| b.fails()).count()
            })
            .sum::<usize>()
            + self
                .ways
                .iter()
                .filter(|w| w.too_far() || w.bad_slope())
                .count()
            + self.trap_boxes.iter().filter(|b| b.fails()).count()
            + usize::from(self.seal_fails())
    }
}

fn to_segment(p: Point2, a: Point2, b: Point2) -> (f64, f64) {
    let d = b - a;
    let l2 = d.dot(d);
    let t = if l2 < 1e-12 {
        0.0
    } else {
        ((p - a).dot(d) / l2).clamp(0.0, 1.0)
    };
    ((a + d * t).distance(p), t)
}

/// The highest roof or slab over a point, from `above` up.
fn cover_at(model: &Model, p: Point2, above: f64) -> Option<Cover> {
    let access = model.water_site().roof_access;
    let lowest = model.levels().first().map(|l| l.id);
    let walked_level = |l: LevelId| {
        model.railings_on_level(l).next().is_some() || model.rooms_on_level(l).next().is_some()
    };
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut best: Option<Cover> = None;
    let mut take = |c: Cover| {
        if c.z >= above && best.as_ref().is_none_or(|b| c.z > b.z) {
            best = Some(c);
        }
    };
    for s in model.slabs() {
        // A downpipe in a shaft (a hole of the slabs) goes out through
        // the roof all the same: the holes do not count.
        if Some(s.level) == lowest || s.outline.len() < 3 || !inside(p, &s.outline) {
            continue;
        }
        take(Cover {
            name: format!("el forjado de {}", level_name(s.level)),
            level: s.level,
            z: level_z(model, s.level) + s.top_at(p),
            walked: match access {
                RoofAccess::Auto => walked_level(s.level),
                RoofAccess::Walked => true,
                RoofAccess::NotWalked => false,
            },
        });
    }
    for r in model.roofs() {
        if r.outline.len() < 3 || !inside(p, &r.outline) {
            continue;
        }
        take(Cover {
            name: format!("la cubierta de {}", level_name(r.level)),
            level: r.level,
            z: level_z(model, r.level) + r.top_at(p).max(r.base_offset),
            walked: access == RoofAccess::Walked,
        });
    }
    best
}

/// The intake or window within 6 m that asks an outlet at `at` the
/// highest: what it is and the height asked, absolute.
fn near_intakes(model: &Model, at: Point2) -> Option<(String, f64)> {
    let level_name = |l: LevelId| model.level(l).map_or(String::new(), |l| l.name.clone());
    let mut best: Option<(String, f64)> = None;
    let mut take = |what: String, z: f64| {
        if best.as_ref().is_none_or(|b| z > b.1) {
            best = Some((what, z));
        }
    };
    for d in model.devices().filter(|d| d.kind == DeviceKind::AirInlet) {
        if d.position.distance(at) <= INTAKE_GAP {
            let top = level_z(model, d.level) + d.elevation + d.height;
            take(format!("aireador ({})", level_name(d.level)), top + 0.01);
        }
    }
    for o in model.openings() {
        let (Some(t), Some(wall)) = (model.opening_type(o.opening_type), model.wall(o.wall)) else {
            continue;
        };
        if !matches!(t.kind, OpeningKind::Window(_)) {
            continue;
        }
        let Some(g) = archisim_geom::opening_geometry(model, o.id) else {
            continue;
        };
        let mid = (g.exterior_offset + g.interior_offset) / 2.0;
        let (a, b) = (g.point(g.t0, mid), g.point(g.t1, mid));
        if to_segment(at, a, b).0 > INTAKE_GAP {
            continue;
        }
        let head = level_z(model, wall.level) + o.sill + t.height;
        take(
            format!("ventana «{}» ({})", t.name, level_name(wall.level)),
            head + OVER_WINDOWS,
        );
    }
    best
}

/// A device's foot and head, absolute.
fn span(model: &Model, d: &archisim_core::ServiceDevice) -> (f64, f64) {
    let b = level_z(model, d.level) + d.elevation;
    (b, b + d.height)
}

/// Groups devices by their place in plan (one over another).
fn clusters<'a>(
    devices: impl Iterator<Item = &'a archisim_core::ServiceDevice>,
) -> Vec<Vec<&'a archisim_core::ServiceDevice>> {
    let mut out: Vec<Vec<&archisim_core::ServiceDevice>> = Vec::new();
    for d in devices {
        match out
            .iter_mut()
            .find(|c| c[0].position.distance(d.position) < COLUMN_REACH)
        {
            Some(c) => c.push(d),
            None => out.push(vec![d]),
        }
    }
    out
}

/// The ventilation and small evacuation of the waste networks.
pub fn venting(model: &Model, d: &Drainage) -> Venting {
    let devices: Vec<&archisim_core::ServiceDevice> = model.devices().collect();
    let valves: Vec<&archisim_core::ServiceDevice> = devices
        .iter()
        .copied()
        .filter(|x| x.kind == DeviceKind::AirValve)
        .collect();
    // Downpipes with water in them, and the dry ones (vent columns).
    let wet: BTreeSet<DeviceId> = d
        .networks
        .iter()
        .flat_map(|n| n.stretches.iter().filter_map(|s| s.downpipe))
        .collect();
    let dry = clusters(
        devices
            .iter()
            .copied()
            .filter(|x| x.kind == DeviceKind::Downpipe && !wet.contains(&x.id)),
    );
    let dry_columns: Vec<(DeviceId, Point2, f64, f64, f64)> = dry
        .iter()
        .map(|c| {
            let (lo, hi) = c.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, x| {
                let (b, t) = span(model, x);
                (a.0.min(b), a.1.max(t))
            });
            let w = c.iter().map(|x| x.width).fold(0.0, f64::max);
            (c[0].id, c[0].position, lo, hi, w)
        })
        .collect();
    let run_of = |r| model.runs().find(|x| Some(x.id) == r);
    let mut columns = Vec::new();
    let mut ways = Vec::new();
    let mut boxes: Vec<TrapBoxCheck> = Vec::new();
    let all_boxes: Vec<&archisim_core::ServiceDevice> = devices
        .iter()
        .copied()
        .filter(|x| x.kind == DeviceKind::TrapBox)
        .collect();
    let mut used_boxes: BTreeSet<DeviceId> = BTreeSet::new();
    let room_at = |l: LevelId, p: Point2| {
        model
            .rooms_on_level(l)
            .find(|r| r.contains(p))
            .map(|r| r.id)
    };
    for (ni, n) in d.networks.iter().enumerate() {
        if n.system != archisim_core::ServiceSystem::Drainage {
            continue;
        }
        // Each appliance's way: where its downpipe (or collector) starts.
        for dr in &n.drains {
            let gap = model
                .appliances_on(dr.level)
                .iter()
                .find(|a| a.object == dr.object)
                .map_or(0.0, |a| a.point.distance(dr.point));
            let end = dr.route.iter().position(|r| {
                r.downpipe
                    || r.stretch
                        .is_some_and(|s| n.stretches[s].kind == Kind::Collector)
            });
            let (length, to_downpipe) = match end {
                Some(k) => (Some(dr.route[k].along + gap), dr.route[k].downpipe),
                None => (None, false),
            };
            let before = end.unwrap_or(dr.route.len());
            let slope_of = |upto: f64| {
                let v: Vec<f64> = dr.route[..before]
                    .iter()
                    .filter(|r| r.along < upto - 1e-6)
                    .filter_map(|r| r.stretch)
                    .filter(|s| n.stretches[*s].kind == Kind::Branch)
                    .map(|s| n.stretches[s].slope)
                    .collect();
                (!v.is_empty()).then(|| {
                    (
                        v.iter().copied().fold(f64::INFINITY, f64::min),
                        v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    )
                })
            };
            // A trap box on its way, before the downpipe.
            let mut on_box: Option<(DeviceId, f64)> = None;
            for b in all_boxes.iter().filter(|b| b.level == dr.level) {
                let mut best: Option<f64> = None;
                let upto = (before + 1).min(dr.route.len());
                for w in dr.route[..upto].windows(2) {
                    if w[0].level != dr.level || w[1].level != dr.level {
                        continue;
                    }
                    let (dist, t) = to_segment(b.position, w[0].p, w[1].p);
                    if dist <= ON_PIPE {
                        let along = w[0].along + (w[1].along - w[0].along) * t;
                        best = Some(best.map_or(along, |x: f64| x.min(along)));
                    }
                }
                if dr.route.len() == 1 && dr.route[0].p.distance(b.position) <= ON_PIPE {
                    best = Some(0.0);
                }
                if let Some(a) = best {
                    if on_box.is_none_or(|(_, x)| a < x) {
                        on_box = Some((b.id, a));
                    }
                }
            }
            if let Some((bid, along)) = on_box {
                used_boxes.insert(bid);
                let bdev = all_boxes.iter().find(|b| b.id == bid).expect("a box");
                let at = match boxes.iter().position(|x| x.device == bid) {
                    Some(i) => i,
                    None => {
                        boxes.push(TrapBoxCheck {
                            device: bid,
                            level: bdev.level,
                            room: room_at(bdev.level, bdev.position),
                            appliances: Vec::new(),
                            other_rooms: 0,
                            to_downpipe: None,
                        });
                        boxes.len() - 1
                    }
                };
                let bx = &mut boxes[at];
                let point = model
                    .appliances_on(dr.level)
                    .iter()
                    .find(|a| a.object == dr.object)
                    .map(|a| a.point);
                let room = point.and_then(|p| room_at(dr.level, p));
                if bx.room.is_some() && room != bx.room {
                    bx.other_rooms += 1;
                }
                bx.appliances
                    .push((dr.object, dr.kind, along + gap, slope_of(along)));
                if let (Some(l), true) = (length, to_downpipe) {
                    let rest = (l - gap - along).max(0.0);
                    bx.to_downpipe = Some(bx.to_downpipe.map_or(rest, |x| x.max(rest)));
                }
            }
            ways.push(ApplianceWay {
                object: dr.object,
                kind: dr.kind,
                level: dr.level,
                network: ni,
                length,
                to_downpipe,
                slopes: length.and_then(|l| slope_of(l - gap + 1e-6)),
                trap_box: on_box.map(|b| b.0),
            });
        }
        // The columns.
        let mine: Vec<&archisim_core::ServiceDevice> = devices
            .iter()
            .copied()
            .filter(|x| n.stretches.iter().any(|s| s.downpipe == Some(x.id)))
            .collect();
        for c in clusters(mine.into_iter()) {
            let ids: BTreeSet<DeviceId> = c.iter().map(|x| x.id).collect();
            let mut sorted = c.clone();
            sorted.sort_by(|a, b| span(model, a).0.total_cmp(&span(model, b).0));
            let head = *sorted
                .iter()
                .max_by(|a, b| span(model, a).1.total_cmp(&span(model, b).1))
                .expect("a downpipe");
            let (bottom, top) = (span(model, sorted[0]).0, span(model, head).1);
            let drawn = c.iter().map(|x| x.width).fold(0.0, f64::max);
            let sized = n
                .stretches
                .iter()
                .filter(|s| s.downpipe.is_some_and(|x| ids.contains(&x)))
                .map(|s| s.sized)
                .fold(0.0, f64::max);
            let oversized = sized > 0.0 && drawn > sized + 0.001;
            // The floors' branches into it.
            let mut floors: BTreeSet<LevelId> = BTreeSet::new();
            let mut branches: BTreeMap<usize, FloorBranch> = BTreeMap::new();
            let mut branch_runs: BTreeMap<usize, BTreeSet<archisim_core::RunId>> = BTreeMap::new();
            for dr in &n.drains {
                let Some(k) = dr.route.iter().position(|r| r.downpipe) else {
                    continue;
                };
                // It reaches this column there (a floor's tee or its foot).
                let first = c
                    .iter()
                    .any(|x| x.position.distance(dr.route[k].p) < COLUMN_REACH);
                if !first || k == 0 {
                    continue;
                }
                let Some(st) = dr.route[k - 1].stretch else {
                    continue;
                };
                floors.insert(dr.level);
                let gap = model
                    .appliances_on(dr.level)
                    .iter()
                    .find(|a| a.object == dr.object)
                    .map_or(0.0, |a| a.point.distance(dr.point));
                let len = dr.route[k].along + gap;
                let b = branches.entry(st).or_insert(FloorBranch {
                    level: dr.level,
                    stretch: st,
                    appliances: 0,
                    length: 0.0,
                    needed: false,
                    valve: None,
                    vent: None,
                    vent_sized: 0.0,
                });
                b.appliances += 1;
                b.length = b.length.max(len);
                let runs = branch_runs.entry(st).or_default();
                for r in dr.route[..k].iter().filter_map(|r| r.stretch) {
                    if let Some(id) = n.stretches[r].run {
                        runs.insert(id);
                    }
                }
            }
            let floors = floors.len();
            let mut branches: Vec<FloorBranch> = branches.into_values().collect();
            for b in &mut branches {
                b.needed = b.length > LONG_BRANCH + 1e-6 || floors > TERTIARY_FLOORS;
                let runs = &branch_runs[&b.stretch];
                let near = |p: Point2, level: LevelId| {
                    runs.iter().any(|r| {
                        run_of(Some(*r))
                            .is_some_and(|x| x.level == level && x.locate(p).1 <= ON_PIPE)
                    })
                };
                b.valve = valves
                    .iter()
                    .find(|v| near(v.position, v.level))
                    .map(|v| v.id);
                let floor_z = level_z(model, b.level);
                b.vent = dry_columns
                    .iter()
                    .find(|(_, p, lo, hi, _)| {
                        *lo <= floor_z + 0.5 && *hi >= floor_z && near(*p, b.level)
                    })
                    .map(|(id, _, _, _, w)| (*id, *w));
                let branch_d = n.stretches[b.stretch].drawn;
                b.vent_sized = pvc_at_least((branch_d * VENT_FRACTION).max(LEAST_VENT));
            }
            let at = head.position;
            let valve = valves
                .iter()
                .find(|v| {
                    let (vb, _) = span(model, v);
                    v.position.distance(at) <= HEAD_REACH && (vb - top).abs() <= 0.6
                })
                .map(|v| v.id);
            // The floors it serves, for its vent column.
            let served: Vec<f64> = branches.iter().map(|b| level_z(model, b.level)).collect();
            let (low, high) = served
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |a, z| {
                    (a.0.min(*z), a.1.max(*z))
                });
            let vent = dry_columns
                .iter()
                .filter(|(_, p, lo, hi, _)| {
                    p.distance(at) <= VENT_REACH && *lo <= low + 0.5 && *hi >= high
                })
                .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
                .map(|(id, _, _, _, w)| (*id, *w));
            columns.push(Column {
                network: ni,
                devices: sorted.iter().map(|x| x.id).collect(),
                head: head.id,
                at,
                bottom,
                top,
                drawn,
                sized,
                oversized,
                floors,
                cover: cover_at(model, at, bottom + 0.5),
                valve,
                near: near_intakes(model, at),
                vent,
                vent_sized: pvc_at_least((drawn * VENT_FRACTION).max(LEAST_VENT)),
                branches,
            });
        }
    }
    let loose_boxes = all_boxes
        .iter()
        .filter(|b| !used_boxes.contains(&b.id))
        .map(|b| b.id)
        .collect();
    Venting {
        columns,
        ways,
        trap_boxes: boxes,
        loose_boxes,
        seal: model.water_site().trap_seal,
    }
}

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

fn mm(v: f64) -> String {
    format!("Ø{:.0}", v * 1000.0)
}

/// What a column is called: its head downpipe's name.
pub fn column_name(model: &Model, c: &Column) -> String {
    model
        .devices()
        .find(|d| d.id == c.head)
        .map_or_else(|| format!("Bajante {}", c.head.0), |d| d.name.clone())
}

/// A column's primary ventilation, in words.
pub fn primary_text(c: &Column) -> String {
    if c.valve.is_some() {
        return "válvula de aireación en su cabeza (3.3.3.4)".into();
    }
    match (&c.cover, c.required_top()) {
        (Some(cv), Some(req)) => {
            let mut s = format!(
                "{} m sobre {} ({}; exige {} m)",
                dec(c.top - cv.z, 2),
                cv.name,
                if cv.walked {
                    "transitable"
                } else {
                    "no transitable"
                },
                dec(c.above_required(), 2)
            );
            if let Some((what, z)) = &c.near {
                if *z > cv.z + c.above_required() {
                    s.push_str(&format!(
                        "; por la {what}, a menos de {} m, su boca a {} m al menos (sobre {} m)",
                        dec(INTAKE_GAP, 0),
                        dec(*z, 2),
                        dec(req, 2)
                    ));
                }
            }
            if c.primary_fails() {
                s.push_str(&format!(
                    " — NO CUMPLE: prolongarla {} m",
                    dec(req - c.top, 2)
                ));
            }
            s
        }
        _ => "no está bajo ninguna cubierta ni forjado del dibujo: no se comprueba".into(),
    }
}

/// The ventilation and small evacuation, as text for the report.
pub fn report(model: &Model, d: &Drainage, v: &Venting) -> String {
    let mut s = String::new();
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |l| l.name.clone());
    let object_name = |o: ObjectId| {
        model
            .objects()
            .find(|x| x.id == o)
            .and_then(|x| model.object_type(x.object_type))
            .map_or_else(|| format!("objeto {}", o.0), |t| t.name.clone())
    };
    if v.columns.is_empty() && v.ways.is_empty() {
        return s;
    }
    s.push_str("\nVentilación de la red (3.3.3)\n");
    for c in &v.columns {
        s.push_str(&format!(
            "  {} ({} dibujada, {} exigida{}): {} planta{} — primaria: {}\n",
            column_name(model, c),
            mm(c.drawn),
            mm(c.sized),
            if c.oversized {
                ", sobredimensionada"
            } else {
                ""
            },
            c.floors,
            if c.floors == 1 { "" } else { "s" },
            primary_text(c)
        ));
        if c.secondary_needed() {
            s.push_str(&format!(
                "    secundaria (desde {} plantas): {}\n",
                if c.oversized {
                    PRIMARY_FLOORS_OVERSIZED
                } else {
                    PRIMARY_FLOORS
                },
                match (c.vent, c.valves_on_branches()) {
                    (_, true) => "válvulas de aireación en todos los ramales".to_string(),
                    (Some((_, dv)), _) => format!(
                        "columna de ventilación {} (exige {}){}",
                        mm(dv),
                        mm(c.vent_sized),
                        if c.secondary_fails() { " — NO CUMPLE" } else { "" }
                    ),
                    (None, _) => format!(
                        "NO CUMPLE: falta la columna de ventilación ({} al menos) junto a la bajante",
                        mm(c.vent_sized)
                    ),
                }
            ));
        }
        for b in c.branches.iter().filter(|b| b.needed) {
            s.push_str(&format!(
                "    terciaria en el ramal {} de {} ({} m): {}\n",
                d.networks[c.network].stretches[b.stretch].name,
                level_name(b.level),
                dec(b.length, 2),
                if b.valve.is_some() {
                    "válvula de aireación".to_string()
                } else if let Some((_, dv)) = b.vent {
                    format!(
                        "ventilación {} (exige {}){}",
                        mm(dv),
                        mm(b.vent_sized),
                        if b.fails() { " — NO CUMPLE" } else { "" }
                    )
                } else {
                    "NO CUMPLE: falta su ventilación o una válvula de aireación".to_string()
                }
            ));
        }
    }
    s.push_str(&format!(
        "\nCierres hidráulicos (3.3.1.1): altura del cierre {} mm (entre {} y {}){}\n",
        dec(v.seal * 1000.0, 0),
        dec(SEAL_MIN * 1000.0, 0),
        dec(SEAL_MAX * 1000.0, 0),
        if v.seal_fails() { " — NO CUMPLE" } else { "" }
    ));
    for b in &v.trap_boxes {
        let mut why = Vec::new();
        if b.has_wc() {
            why.push("lleva un inodoro".to_string());
        }
        if b.other_rooms > 0 {
            why.push(format!("{} aparato(s) de otro local", b.other_rooms));
        }
        if b.long_branches() > 0 {
            why.push(format!(
                "{} derivación(es) de más de {} m",
                b.long_branches(),
                dec(BOX_BRANCH, 2)
            ));
        }
        if b.bad_slopes() > 0 {
            why.push(format!(
                "{} derivación(es) fuera del {} al {} %",
                b.bad_slopes(),
                dec(BOX_SLOPE.0 * 100.0, 0),
                dec(BOX_SLOPE.1 * 100.0, 0)
            ));
        }
        if b.too_far() {
            why.push(format!(
                "a más de {} m de la bajante",
                dec(BOX_TO_DOWNPIPE, 2)
            ));
        }
        s.push_str(&format!(
            "  Bote sifónico ({}): {} aparato{}, a {} de la bajante{}\n",
            level_name(b.level),
            b.appliances.len(),
            if b.appliances.len() == 1 { "" } else { "s" },
            b.to_downpipe
                .map_or("—".to_string(), |x| format!("{} m", dec(x, 2))),
            if why.is_empty() {
                String::new()
            } else {
                format!(" — NO CUMPLE: {}", why.join(", "))
            }
        ));
    }
    if !v.loose_boxes.is_empty() {
        s.push_str(&format!(
            "  {} bote{} sifónico{} sin aparatos en su red\n",
            v.loose_boxes.len(),
            if v.loose_boxes.len() == 1 { "" } else { "s" },
            if v.loose_boxes.len() == 1 { "" } else { "s" }
        ));
    }
    let own = v.ways.iter().filter(|w| w.trap_box.is_none()).count();
    s.push_str(&format!(
        "  {own} aparato{} con sifón individual\n",
        if own == 1 { "" } else { "s" }
    ));
    let bad: Vec<&ApplianceWay> = v
        .ways
        .iter()
        .filter(|w| w.too_far() || w.bad_slope())
        .collect();
    s.push_str(&format!(
        "\nDerivaciones de los aparatos (3.3.1.4): {} de {} cumplen\n",
        v.ways.len() - bad.len(),
        v.ways.len()
    ));
    for w in bad {
        let (reach, lo, hi) = w.limits();
        let mut why = Vec::new();
        if w.too_far() {
            why.push(format!(
                "a {} m de la {} (≤ {} m)",
                dec(w.length.unwrap_or(0.0), 2),
                if w.to_downpipe { "bajante" } else { "red" },
                dec(reach, 2)
            ));
        }
        if w.bad_slope() {
            let (a, b) = w.slopes.unwrap_or((0.0, 0.0));
            why.push(format!(
                "pendiente {} % ({} a {} %)",
                if (a - b).abs() < 1e-6 {
                    dec(a * 100.0, 1)
                } else {
                    format!("{}–{}", dec(a * 100.0, 1), dec(b * 100.0, 1))
                },
                dec(lo * 100.0, 1),
                if hi.is_finite() {
                    dec(hi * 100.0, 0)
                } else {
                    "—".into()
                }
            ));
        }
        s.push_str(&format!(
            "  {} «{}» ({}): {}\n",
            w.kind.name(),
            object_name(w.object),
            level_name(w.level),
            why.join("; ")
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drain::drainage;
    use archisim_core::{NewDevice, WaterSite};

    fn device(m: &mut Model, level: LevelId, kind: DeviceKind, at: Point2, z: f64) -> DeviceId {
        let (w, d, h, _) = kind.defaults();
        m.add_device(NewDevice {
            level,
            kind,
            name: kind.name().into(),
            position: at,
            rotation: 0.0,
            elevation: z,
            width: w,
            depth: d,
            height: h,
        })
        .unwrap()
    }

    #[test]
    fn a_house_vents_its_downpipe() {
        let mut m = crate::drain::tests::house();
        let v = venting(&m, &drainage(&m));
        assert_eq!(v.columns.len(), 1, "{:#?}", v.columns);
        let c = v.columns[0].clone();
        // Two floors drain into it: the upper one by its branch, the
        // ground floor at its foot.
        assert_eq!(c.floors, 2);
        assert!(!c.secondary_needed());
        assert!(c.branches.iter().all(|b| !b.needed), "{:#?}", c.branches);
        // It stops at the first floor, under the roof at 6 m (not walked
        // on): its head must reach 7,30 m.
        let cover = c.cover.clone().expect("the roof over it");
        assert!((cover.z - 6.0).abs() < 1e-6 && !cover.walked);
        assert!((c.required_top().unwrap() - 7.3).abs() < 1e-6);
        assert!(c.primary_fails());
        // Taken up to 7,40 m it passes; on a walked roof it asks 8,00 m.
        m.edit_device(c.head, |d| d.height = 7.4).unwrap();
        let v = venting(&m, &drainage(&m));
        assert!(!v.columns[0].primary_fails(), "{:#?}", v.columns[0]);
        let site = WaterSite {
            roof_access: RoofAccess::Walked,
            ..*m.water_site()
        };
        m.set_water_site(site).unwrap();
        let v = venting(&m, &drainage(&m));
        assert!((v.columns[0].required_top().unwrap() - 8.0).abs() < 1e-6);
        assert!(v.columns[0].primary_fails());
        // A valve at its head stands for it.
        let l0 = m.levels()[0].id;
        device(&mut m, l0, DeviceKind::AirValve, Point2::new(1.0, 0.0), 7.4);
        let v = venting(&m, &drainage(&m));
        assert!(v.columns[0].valve.is_some() && !v.columns[0].primary_fails());
        let text = report(&m, &drainage(&m), &v);
        assert!(text.contains("válvula de aireación"), "{text}");
    }

    #[test]
    fn each_appliance_to_its_downpipe() {
        let m = crate::drain::tests::house();
        let v = venting(&m, &drainage(&m));
        let l1 = m.levels()[1].id;
        let on = |k: Appliance| {
            v.ways
                .iter()
                .find(|w| w.level == l1 && w.kind == k)
                .unwrap_or_else(|| panic!("{k:?}: {:#?}", v.ways))
                .clone()
        };
        // WC at x 2: 0,8 m down its own drain and 1 m along the branch.
        let wc = on(Appliance::Wc);
        assert!((wc.length.unwrap() - 1.8).abs() < 0.15, "{wc:?}");
        assert!(wc.to_downpipe && wc.too_far());
        // Basin at x 3,5: 3,3 m, within 4 m, but its pipes are at 2 %.
        let basin = on(Appliance::Washbasin);
        assert!((basin.length.unwrap() - 3.3).abs() < 0.15, "{basin:?}");
        assert!(!basin.too_far() && basin.bad_slope());
        // Shower at x 5: 4,8 m, too far; 2 % is within its slopes.
        let shower = on(Appliance::Shower);
        assert!(shower.too_far() && !shower.bad_slope(), "{shower:?}");
        assert!(!v.seal_fails());
    }

    #[test]
    fn trap_boxes_take_their_room_only() {
        let mut m = crate::drain::tests::house();
        let l1 = m.levels()[1].id;
        // A box on the branch between the shower and the basin: the
        // shower drains through it, 3,25 m from the downpipe.
        let b = device(
            &mut m,
            l1,
            DeviceKind::TrapBox,
            Point2::new(4.25, 0.0),
            -0.15,
        );
        let v = venting(&m, &drainage(&m));
        let tb = v
            .trap_boxes
            .iter()
            .find(|x| x.device == b)
            .expect("its box");
        assert_eq!(tb.appliances.len(), 1, "{tb:#?}");
        assert_eq!(tb.appliances[0].1, Appliance::Shower);
        assert!((tb.appliances[0].2 - 1.55).abs() < 0.15, "{tb:#?}");
        assert!((tb.to_downpipe.unwrap() - 3.25).abs() < 0.15);
        assert!(tb.too_far() && !tb.has_wc() && tb.fails());
        // A box by the WC takes it: not allowed.
        let w = device(
            &mut m,
            l1,
            DeviceKind::TrapBox,
            Point2::new(2.0, 0.4),
            -0.15,
        );
        let v = venting(&m, &drainage(&m));
        let tb = v
            .trap_boxes
            .iter()
            .find(|x| x.device == w)
            .expect("the WC's box");
        assert!(tb.has_wc() && tb.fails());
        // One on no drain is loose.
        let lone = device(
            &mut m,
            l1,
            DeviceKind::TrapBox,
            Point2::new(9.0, 3.0),
            -0.15,
        );
        let v = venting(&m, &drainage(&m));
        assert!(v.loose_boxes.contains(&lone));
    }

    #[test]
    fn tall_columns_need_their_vent_column() {
        let mut m = crate::drain::tests::house();
        let v = venting(&m, &drainage(&m));
        let mut c = v.columns[0].clone();
        c.floors = PRIMARY_FLOORS;
        assert!(c.secondary_needed() && c.secondary_fails());
        c.oversized = true;
        assert!(!c.secondary_needed());
        c.floors = PRIMARY_FLOORS_OVERSIZED;
        assert!(c.secondary_fails());
        // A dry downpipe beside it, all its height, half its diameter:
        // its vent column.
        let l0 = m.levels()[0].id;
        let dry = m
            .add_device(NewDevice {
                level: l0,
                kind: DeviceKind::Downpipe,
                name: "Ventilación secundaria".into(),
                position: Point2::new(1.0, 0.4),
                rotation: 0.0,
                elevation: 0.0,
                width: 0.063,
                depth: 0.063,
                height: 7.5,
            })
            .unwrap();
        let v = venting(&m, &drainage(&m));
        let mut c = v.columns[0].clone();
        assert_eq!(c.vent.map(|x| x.0), Some(dry), "{c:#?}");
        c.floors = PRIMARY_FLOORS;
        assert!(c.secondary_needed() && !c.secondary_fails(), "{c:#?}");
        // Short branches on a low column: no tertiary ventilation.
        assert!(c.branches.iter().all(|b| !b.needed));
    }
}
