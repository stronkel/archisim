//! Drainage by DB-HS 5, on the pipes drawn.
//!
//! - **Waste water** (`ServiceSystem::Drainage`): every sanitary appliance
//!   drains into the nearest end of a drain; each network is followed from
//!   its lowest point (the connection to the sewer, an «arqueta») up the
//!   downpipes to every appliance. The units of discharge of each stretch
//!   (tabla 4.1, private or public use by the building's use) give its
//!   diameter: an appliance's own branch by tabla 4.1, a branch collecting
//!   several by tabla 4.3 (with its slope), a downpipe by tabla 4.4 (all it
//!   gathers, the most one floor brings and how many floors) and a
//!   collector by tabla 4.5 (with its slope). Nothing gets smaller
//!   downstream.
//! - **Rain water** (`ServiceSystem::Rainwater`): the roofs, terraces and
//!   balconies open to the sky are sampled on a grid and every point drains
//!   to the nearest inlet (a free upper end of a rain pipe, a downpipe's
//!   head) that is not above it; downpipes by tabla 4.8 and collectors by
//!   tabla 4.9, with the area scaled by the rainfall intensity (i/100).
//!   Every roof is checked for its number of outlets (tabla 4.6), and the
//!   gutters a roof carries (its covering) by tablas 4.7 and 4.8.
//! - **Mixed** networks: where a rain network ends on a waste drain, the
//!   waste stretches after it carry both, sized by tabla 4.9 with the units
//!   turned into an equivalent area (4.3: 90 m² up to 250 UD, 0,36 m² per
//!   UD above).
//!
//! Pipes are PVC, whose outside diameter is the nominal one of the tables.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use archisim_core::{
    Appliance, BuildingUse, DeviceId, LevelId, Model, ObjectId, Point2, RunId, ServiceSystem,
};

use crate::water::{graph, level_z, Graph};

/// PVC drain sizes, outside diameter, metres.
pub const PVC: [f64; 13] = [
    0.032, 0.040, 0.050, 0.063, 0.075, 0.090, 0.110, 0.125, 0.160, 0.200, 0.250, 0.315, 0.350,
];

/// How far from an appliance its drain may end, metres.
const APPLIANCE_REACH: f64 = 0.45;
/// The grid the rain is sampled on, metres.
const RAIN_STEP: f64 = 0.5;
/// An inlet takes the rain of a surface up to this much below it (a
/// gutter under its eave) and none from below it, metres.
const INLET_BELOW: f64 = 4.0;
const INLET_ABOVE: f64 = 1.0;
/// Smaller surfaces (balconies, a porch) are not checked for their
/// number of outlets, m².
pub const SMALL_SURFACE: f64 = 25.0;

/// HS 5, tabla 4.1: the units of discharge of an appliance and the least
/// diameter of its own drain (mm), in private or public use.
pub fn units(a: Appliance, public: bool) -> (u32, f64) {
    if !public {
        return a.drain();
    }
    match a {
        Appliance::Washbasin => (2, 40.0),
        Appliance::Bidet => (3, 40.0),
        Appliance::Shower => (3, 50.0),
        Appliance::Bath => (4, 50.0),
        Appliance::Wc => (5, 100.0),
        Appliance::Sink | Appliance::Dishwasher | Appliance::Washer => (6, 50.0),
    }
}

/// The column of a table by slope: 1, 2 or 4 %.
fn slope_column(slope: f64) -> usize {
    if slope >= 0.04 - 1e-6 {
        2
    } else if slope >= 0.02 - 1e-6 {
        1
    } else {
        0
    }
}

/// HS 5, tabla 4.3: branches collecting several appliances, the most UD
/// at 1, 2 and 4 % (none: not allowed at that slope).
const BRANCHES: [(f64, [u32; 3]); 10] = [
    (0.032, [0, 1, 1]),
    (0.040, [0, 2, 3]),
    (0.050, [0, 6, 8]),
    (0.063, [0, 11, 14]),
    (0.075, [0, 21, 28]),
    (0.090, [47, 60, 75]),
    (0.110, [123, 151, 181]),
    (0.125, [180, 234, 280]),
    (0.160, [438, 582, 800]),
    (0.200, [870, 1150, 1680]),
];

/// HS 5, tabla 4.4: downpipes, the most UD in all (up to 3 floors, more)
/// and from one floor (up to 3 floors, more).
const DOWNPIPES: [(f64, [u32; 4]); 10] = [
    (0.050, [10, 25, 6, 6]),
    (0.063, [19, 38, 11, 9]),
    (0.075, [27, 53, 21, 13]),
    (0.090, [135, 280, 70, 53]),
    (0.110, [360, 740, 181, 134]),
    (0.125, [540, 1100, 280, 200]),
    (0.160, [1208, 2240, 1120, 400]),
    (0.200, [2200, 3600, 1680, 600]),
    (0.250, [3800, 5600, 2500, 1000]),
    (0.315, [6000, 9240, 4320, 1650]),
];

/// HS 5, tabla 4.5: horizontal collectors, the most UD at 1, 2 and 4 %.
const COLLECTORS: [(f64, [u32; 3]); 11] = [
    (0.050, [0, 20, 25]),
    (0.063, [0, 24, 29]),
    (0.075, [0, 38, 57]),
    (0.090, [96, 130, 160]),
    (0.110, [264, 321, 382]),
    (0.125, [390, 480, 580]),
    (0.160, [880, 1056, 1300]),
    (0.200, [1600, 1920, 2300]),
    (0.250, [2900, 3500, 4200]),
    (0.315, [5710, 6920, 8290]),
    (0.350, [8300, 10000, 12000]),
];

/// HS 5, tabla 4.7: semicircular gutters, the most roof (m², at 100 mm/h)
/// at 0,5, 1, 2 and 4 %.
pub const GUTTERS: [(f64, [f64; 4]); 5] = [
    (0.100, [35.0, 45.0, 65.0, 95.0]),
    (0.125, [60.0, 80.0, 115.0, 165.0]),
    (0.150, [90.0, 125.0, 175.0, 255.0]),
    (0.200, [185.0, 260.0, 370.0, 520.0]),
    (0.250, [335.0, 475.0, 670.0, 930.0]),
];

/// HS 5, tabla 4.8: rain downpipes, the most roof (m², at 100 mm/h).
const RAIN_DOWNPIPES: [(f64, f64); 8] = [
    (0.050, 65.0),
    (0.063, 113.0),
    (0.075, 177.0),
    (0.090, 318.0),
    (0.110, 580.0),
    (0.125, 805.0),
    (0.160, 1544.0),
    (0.200, 2700.0),
];

/// HS 5, tabla 4.9: rain (and mixed) collectors, the most area (m², at
/// 100 mm/h) at 1, 2 and 4 %.
const RAIN_COLLECTORS: [(f64, [f64; 3]); 7] = [
    (0.090, [125.0, 178.0, 253.0]),
    (0.110, [229.0, 323.0, 458.0]),
    (0.125, [310.0, 440.0, 620.0]),
    (0.160, [614.0, 862.0, 1228.0]),
    (0.200, [1070.0, 1510.0, 2140.0]),
    (0.250, [1920.0, 2710.0, 3850.0]),
    (0.315, [3016.0, 4589.0, 6500.0]),
];

/// HS 5, tabla 4.6: the outlets a roof of this area needs.
pub fn outlets_needed(area: f64) -> usize {
    if area < 100.0 {
        2
    } else if area < 200.0 {
        3
    } else if area <= 500.0 {
        4
    } else {
        (area / 150.0).ceil() as usize
    }
}

/// HS 5, 4.3: the area that stands for `units` of waste water in a mixed
/// collector, m².
pub fn equivalent_area(units: u32) -> f64 {
    if units == 0 {
        0.0
    } else if units <= 250 {
        90.0
    } else {
        0.36 * f64::from(units)
    }
}

/// The smallest PVC size of at least `mm`.
pub(crate) fn pvc_at_least(m: f64) -> f64 {
    PVC.iter()
        .copied()
        .find(|d| *d + 1e-6 >= m)
        .unwrap_or(PVC[PVC.len() - 1])
}

/// The smallest size of a table that takes `want`; the largest (and
/// false) if none does.
fn pick<T: Copy + PartialOrd>(rows: &[(f64, T)], fits: impl Fn(T) -> bool) -> (f64, bool) {
    rows.iter()
        .find(|r| fits(r.1))
        .map_or((rows[rows.len() - 1].0, false), |r| (r.0, true))
}

/// What a stretch is, for the table that sizes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An appliance's own drain (tabla 4.1).
    Branch,
    /// A branch gathering several appliances (tabla 4.3).
    Gathering,
    /// A downpipe (tabla 4.4; rain, 4.8).
    Downpipe,
    /// A horizontal collector (tabla 4.5; rain and mixed, 4.9).
    Collector,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Branch => "derivación",
            Kind::Gathering => "ramal",
            Kind::Downpipe => "bajante",
            Kind::Collector => "colector",
        }
    }
}

/// A piece of the network: one pipe (or downpipe) between branchings.
#[derive(Debug, Clone)]
pub struct Stretch {
    /// «S1»… for waste water, «P1»… for rain.
    pub name: String,
    pub run: Option<RunId>,
    pub downpipe: Option<DeviceId>,
    pub level: LevelId,
    /// The stretch it drains into (`None`: it reaches the outlet).
    pub parent: Option<usize>,
    pub kind: Kind,
    pub length: f64,
    pub slope: f64,
    /// Units of discharge it carries (waste water).
    pub units: u32,
    /// Appliances it drains.
    pub appliances: usize,
    /// Roof it drains, in plan, m² (rain; mixed collectors too).
    pub area: f64,
    /// Outside diameter drawn, metres.
    pub drawn: f64,
    /// Outside diameter the code asks, metres.
    pub sized: f64,
    /// The rule that sized it.
    pub rule: &'static str,
    /// False if not even the largest size of its table is enough.
    pub fits: bool,
}

impl Stretch {
    /// Whether the pipe drawn is smaller than the one asked.
    pub fn short(&self) -> bool {
        self.drawn + 0.0015 < self.sized
    }
}

/// An appliance draining into the network.
#[derive(Debug, Clone)]
pub struct Drain {
    pub object: ObjectId,
    pub kind: Appliance,
    pub level: LevelId,
    pub units: u32,
    /// Its way to the outlet, first stretch first (the outlet's end).
    pub path: Vec<usize>,
    /// Where it joins the network (the end of a drain nearest it).
    pub point: Point2,
    /// The points of its way, from where it joins down to the outlet.
    pub route: Vec<RoutePoint>,
}

/// A point on an appliance's way to the outlet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutePoint {
    pub level: LevelId,
    pub p: Point2,
    /// Absolute height, metres.
    pub z: f64,
    /// How far along the pipes from the appliance's end, metres.
    pub along: f64,
    /// A downpipe starts or passes here.
    pub downpipe: bool,
    /// The stretch it goes on by towards the outlet (none at the outlet).
    pub stretch: Option<usize>,
}

/// A rain inlet: where a roof's water enters the network.
#[derive(Debug, Clone)]
pub struct Inlet {
    pub level: LevelId,
    pub point: Point2,
    /// Roof it takes, in plan, m².
    pub area: f64,
    pub path: Vec<usize>,
}

/// One network: from its outlet up.
#[derive(Debug, Clone)]
pub struct Network {
    pub system: ServiceSystem,
    /// The outlet: its level and point.
    pub outlet: (LevelId, Point2),
    pub stretches: Vec<Stretch>,
    pub drains: Vec<Drain>,
    pub inlets: Vec<Inlet>,
    /// For a rain network, the waste network it ends on (mixed).
    pub joins: Option<usize>,
}

impl Network {
    pub fn units(&self) -> u32 {
        self.drains.iter().map(|d| d.units).sum()
    }

    pub fn area(&self) -> f64 {
        self.inlets.iter().map(|i| i.area).sum()
    }

    /// Its stretches that are drawn smaller than asked.
    pub fn short(&self) -> impl Iterator<Item = &Stretch> {
        self.stretches.iter().filter(|s| s.short())
    }
}

/// A surface open to the sky: a roof, a terrace, a balcony.
#[derive(Debug, Clone)]
pub struct Surface {
    pub name: String,
    pub level: LevelId,
    /// Open to the sky, in plan, m².
    pub area: f64,
    /// Of it, what no inlet takes.
    pub undrained: f64,
    /// Inlets on it.
    pub outlets: usize,
    /// Of the rain network, or the gutters' downpipes of a roof.
    pub needed: usize,
    /// True if it is big enough to be checked (`SMALL_SURFACE`).
    pub checked: bool,
}

/// The gutters of a roof (its covering), each eave by tablas 4.7 and 4.8.
#[derive(Debug, Clone)]
pub struct GutterCheck {
    pub roof: String,
    pub level: LevelId,
    /// The eave's side of the footprint.
    pub side: usize,
    /// Roof area each of its downpipes takes, the largest, m².
    pub area: f64,
    pub downpipes: usize,
    /// Gutter width and downpipe drawn, metres.
    pub gutter: f64,
    pub downpipe: f64,
    /// The sizes asked (gutter at 1 % slope).
    pub gutter_sized: f64,
    pub downpipe_sized: f64,
}

/// The building's drainage.
#[derive(Debug, Clone)]
pub struct Drainage {
    /// Public use (tabla 4.1, uso público): anything but dwellings.
    pub public: bool,
    /// mm/h.
    pub intensity: f64,
    pub networks: Vec<Network>,
    /// Appliances with no drain near.
    pub loose: Vec<ObjectId>,
    pub surfaces: Vec<Surface>,
    pub gutters: Vec<GutterCheck>,
}

impl Drainage {
    /// The intensity factor of the rain tables.
    pub fn factor(&self) -> f64 {
        self.intensity / 100.0
    }

    /// The diameter every run and downpipe should take: the largest of
    /// its stretches.
    pub fn sized_runs(&self) -> (BTreeMap<RunId, f64>, BTreeMap<DeviceId, f64>) {
        let mut runs: BTreeMap<RunId, f64> = BTreeMap::new();
        let mut downpipes: BTreeMap<DeviceId, f64> = BTreeMap::new();
        for n in &self.networks {
            for s in &n.stretches {
                if let Some(r) = s.run {
                    let e = runs.entry(r).or_insert(0.0);
                    *e = e.max(s.sized);
                }
                if let Some(d) = s.downpipe {
                    let e = downpipes.entry(d).or_insert(0.0);
                    *e = e.max(s.sized);
                }
            }
        }
        (runs, downpipes)
    }
}

pub(crate) fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

/// A point of a surface open to the sky.
struct RainPoint {
    p: Point2,
    z: f64,
    surface: usize,
}

/// The surfaces open to the sky (slabs above the lowest level, and roofs)
/// sampled on a grid: every point keeps the highest surface over it.
fn open_surfaces(model: &Model) -> (Vec<Surface>, Vec<RainPoint>, BTreeMap<usize, usize>) {
    struct Top {
        name: String,
        level: LevelId,
        outline: Vec<Point2>,
        holes: Vec<Vec<Point2>>,
        z: Box<dyn Fn(Point2) -> f64>,
        roof: Option<usize>,
    }
    let lowest = model.levels().first().map(|l| l.id);
    let mut tops: Vec<Top> = Vec::new();
    for s in model.slabs() {
        if Some(s.level) == lowest || s.outline.len() < 3 {
            continue;
        }
        let base = level_z(model, s.level);
        let slab = s.clone();
        tops.push(Top {
            name: format!(
                "Forjado de {}",
                model.level(s.level).map_or("", |l| l.name.as_str())
            ),
            level: s.level,
            outline: s.outline.clone(),
            holes: s.holes.clone(),
            z: Box::new(move |p| base + slab.top_at(p)),
            roof: None,
        });
    }
    for (k, r) in model.roofs().enumerate() {
        if r.outline.len() < 3 {
            continue;
        }
        let base = level_z(model, r.level);
        let roof = r.clone();
        tops.push(Top {
            name: format!(
                "Cubierta de {}",
                model.level(r.level).map_or("", |l| l.name.as_str())
            ),
            level: r.level,
            outline: r.outline.clone(),
            holes: Vec::new(),
            // A roof is over the slab of its own level.
            z: Box::new(move |p| base + roof.top_at(p).max(roof.base_offset) + 0.01),
            roof: Some(k),
        });
    }
    let mut surfaces: Vec<Surface> = tops
        .iter()
        .map(|t| Surface {
            name: t.name.clone(),
            level: t.level,
            area: 0.0,
            undrained: 0.0,
            outlets: 0,
            needed: 0,
            checked: false,
        })
        .collect();
    // Each roof's surface, by the roof's place among the model's roofs.
    let roof_of: BTreeMap<usize, usize> = tops
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.roof.map(|k| (k, i)))
        .collect();
    let mut points = Vec::new();
    if tops.is_empty() {
        return (surfaces, points, roof_of);
    }
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for t in &tops {
        for p in &t.outline {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    let cell = RAIN_STEP * RAIN_STEP;
    let (nx, ny) = (
        ((hi.x - lo.x) / RAIN_STEP).ceil() as usize,
        ((hi.y - lo.y) / RAIN_STEP).ceil() as usize,
    );
    // Too big a site is sampled coarser, for the same number of points.
    if nx * ny > 400_000 {
        return (surfaces, points, roof_of);
    }
    for i in 0..nx {
        for j in 0..ny {
            let p = Point2::new(
                lo.x + (i as f64 + 0.5) * RAIN_STEP,
                lo.y + (j as f64 + 0.5) * RAIN_STEP,
            );
            let top = tops
                .iter()
                .enumerate()
                .filter(|(_, t)| inside(p, &t.outline) && !t.holes.iter().any(|h| inside(p, h)))
                .map(|(k, t)| (k, (t.z)(p)))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((k, z)) = top {
                surfaces[k].area += cell;
                points.push(RainPoint { p, z, surface: k });
            }
        }
    }
    (surfaces, points, roof_of)
}

/// The drainage of the model.
pub fn drainage(model: &Model) -> Drainage {
    let public = model.code_settings().building_use != BuildingUse::Dwelling;
    let intensity = model.water_site().rain_intensity;
    let f = intensity / 100.0;
    let mut networks = Vec::new();
    let mut loose = Vec::new();
    // Waste water.
    let g = graph(model, ServiceSystem::Drainage);
    let mut at: BTreeMap<usize, Vec<(ObjectId, Appliance, LevelId)>> = BTreeMap::new();
    for l in model.levels() {
        for a in model.appliances_on(l.id) {
            match g.nearest(a.level, a.point, APPLIANCE_REACH) {
                Some(n) => at.entry(n).or_default().push((a.object, a.kind, a.level)),
                None => loose.push(a.object),
            }
        }
    }
    let waste = components(&g);
    for comp in &waste {
        let drains: BTreeMap<usize, Vec<(ObjectId, Appliance, LevelId)>> = comp
            .iter()
            .filter_map(|n| at.get(n).map(|v| (*n, v.clone())))
            .collect();
        if drains.is_empty() {
            continue;
        }
        let outlet = lowest(&g, comp, &drains);
        networks.push(build(
            model,
            &g,
            ServiceSystem::Drainage,
            outlet,
            &drains,
            &BTreeMap::new(),
            public,
        ));
    }
    // Rain water: the inlets of every network, and the rain on them.
    let rg = graph(model, ServiceSystem::Rainwater);
    let rain = components(&rg);
    let (mut surfaces, points, roof_of) = open_surfaces(model);
    let mut outlets: Vec<usize> = Vec::new();
    let mut inlets: Vec<(usize, usize)> = Vec::new();
    for (k, comp) in rain.iter().enumerate() {
        let none: BTreeMap<usize, ()> = BTreeMap::new();
        let o = lowest(&rg, comp, &none);
        outlets.push(o);
        for n in comp {
            if *n != o && rg.adj[*n].len() == 1 {
                inlets.push((k, *n));
            }
        }
    }
    let mut take: BTreeMap<usize, f64> = BTreeMap::new();
    let mut per_surface: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for rp in &points {
        let best = inlets
            .iter()
            .filter(|(_, n)| {
                let z = rg.nodes[*n].z;
                z <= rp.z + INLET_ABOVE && z >= rp.z - INLET_BELOW
            })
            .min_by(|a, b| {
                rg.nodes[a.1]
                    .p
                    .distance(rp.p)
                    .total_cmp(&rg.nodes[b.1].p.distance(rp.p))
            });
        match best {
            Some((_, n)) => {
                *take.entry(*n).or_default() += RAIN_STEP * RAIN_STEP;
                // An outlet of this surface if it is at its height (not
                // one on the roof below that takes its water).
                if (rg.nodes[*n].z - rp.z).abs() <= INLET_ABOVE {
                    per_surface.entry(rp.surface).or_default().insert(*n);
                }
            }
            None => surfaces[rp.surface].undrained += RAIN_STEP * RAIN_STEP,
        }
    }
    let waste_count = networks.len();
    for (k, comp) in rain.iter().enumerate() {
        let areas: BTreeMap<usize, f64> = comp
            .iter()
            .filter_map(|n| take.get(n).map(|a| (*n, *a)))
            .collect();
        if areas.is_empty() {
            continue;
        }
        let mut net = build(
            model,
            &rg,
            ServiceSystem::Rainwater,
            outlets[k],
            &BTreeMap::new(),
            &areas,
            public,
        );
        // Mixed: it ends on a waste drain.
        let o = rg.nodes[outlets[k]];
        net.joins = networks[..waste_count].iter().position(|w: &Network| {
            w.stretches.iter().any(|s| {
                s.run.is_some_and(|r| {
                    model
                        .runs()
                        .find(|x| x.id == r)
                        .is_some_and(|x| x.level == o.level && x.locate(o.p).1 < 0.3)
                })
            })
        });
        networks.push(net);
    }
    // Mixed collectors: the waste stretches after a rain network's end.
    let joined: Vec<(usize, Point2, LevelId, f64)> = networks[waste_count..]
        .iter()
        .filter_map(|n| n.joins.map(|w| (w, n.outlet.1, n.outlet.0, n.area())))
        .collect();
    for (w, p, level, area) in joined {
        mix(model, &mut networks[w], p, level, area, f);
    }
    for (k, set) in per_surface {
        surfaces[k].outlets = set.len();
    }
    for s in &mut surfaces {
        s.checked = s.area >= SMALL_SURFACE;
        s.needed = outlets_needed(s.area);
    }
    // Gutters of the roofs.
    let mut gutters = Vec::new();
    for (k, r) in model.roofs().enumerate() {
        let Some(gt) = &r.covering.gutter else {
            continue;
        };
        let runs = r.gutter_runs();
        let pipes: Vec<(usize, Point2)> = runs
            .iter()
            .enumerate()
            .flat_map(|(i, g)| g.downpipes.iter().map(move |p| (i, *p)))
            .collect();
        if pipes.is_empty() {
            continue;
        }
        let surface_index = roof_of.get(&k).copied();
        let mut area = vec![0.0; pipes.len()];
        for rp in points.iter().filter(|rp| Some(rp.surface) == surface_index) {
            if let Some(i) = (0..pipes.len()).min_by(|a, b| {
                pipes[*a]
                    .1
                    .distance(rp.p)
                    .total_cmp(&pipes[*b].1.distance(rp.p))
            }) {
                area[i] += RAIN_STEP * RAIN_STEP;
            }
        }
        if let Some(si) = surface_index {
            surfaces[si].outlets = surfaces[si].outlets.max(pipes.len());
            surfaces[si].undrained = 0.0;
        }
        let name = format!(
            "Cubierta de {}",
            model.level(r.level).map_or("", |l| l.name.as_str())
        );
        for (side, run) in runs.iter().enumerate() {
            let mine: Vec<f64> = pipes
                .iter()
                .zip(&area)
                .filter(|((i, _), _)| *i == side)
                .map(|(_, a)| *a)
                .collect();
            let worst = mine.iter().copied().fold(0.0, f64::max);
            let a = worst * f;
            let gutter_sized = pick(&GUTTERS, |c: [f64; 4]| c[1] >= a).0;
            let downpipe_sized = pick(&RAIN_DOWNPIPES, |c: f64| c >= a).0;
            gutters.push(GutterCheck {
                roof: name.clone(),
                level: r.level,
                side: run.side,
                area: worst,
                downpipes: mine.len(),
                gutter: gt.size,
                downpipe: gt.downpipe,
                gutter_sized,
                downpipe_sized,
            });
        }
    }
    loose.sort();
    loose.dedup();
    Drainage {
        public,
        intensity,
        networks,
        loose,
        surfaces,
        gutters,
    }
}

/// The connected parts of a graph, as their nodes.
fn components(g: &Graph) -> Vec<Vec<usize>> {
    let mut seen = vec![false; g.nodes.len()];
    let mut out = Vec::new();
    for s in 0..g.nodes.len() {
        if seen[s] || g.adj[s].is_empty() {
            continue;
        }
        let mut comp = Vec::new();
        let mut q = VecDeque::from([s]);
        seen[s] = true;
        while let Some(n) = q.pop_front() {
            comp.push(n);
            for e in &g.adj[n] {
                let ed = g.edges[*e];
                let m = if ed.a == n { ed.b } else { ed.a };
                if !seen[m] {
                    seen[m] = true;
                    q.push_back(m);
                }
            }
        }
        out.push(comp);
    }
    out
}

/// The outlet of a network: its lowest free end that is not an
/// appliance's (its lowest point if none).
fn lowest<T>(g: &Graph, comp: &[usize], appliances: &BTreeMap<usize, T>) -> usize {
    comp.iter()
        .copied()
        .filter(|n| g.adj[*n].len() == 1 && !appliances.contains_key(n))
        .min_by(|a, b| g.nodes[*a].z.total_cmp(&g.nodes[*b].z))
        .or_else(|| {
            comp.iter()
                .copied()
                .min_by(|a, b| g.nodes[*a].z.total_cmp(&g.nodes[*b].z))
        })
        .unwrap_or(0)
}

/// The network from its outlet: the tree, its stretches and their sizes.
fn build(
    model: &Model,
    g: &Graph,
    system: ServiceSystem,
    root: usize,
    drains_at: &BTreeMap<usize, Vec<(ObjectId, Appliance, LevelId)>>,
    rain_at: &BTreeMap<usize, f64>,
    public: bool,
) -> Network {
    let rain = system == ServiceSystem::Rainwater;
    let f = model.water_site().rain_intensity / 100.0;
    let lowest_level = model.levels().first().map(|l| l.id);
    let n_nodes = g.nodes.len();
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; n_nodes];
    let mut seen = vec![false; n_nodes];
    let mut order = Vec::new();
    let mut q = VecDeque::from([root]);
    seen[root] = true;
    while let Some(n) = q.pop_front() {
        order.push(n);
        for e in &g.adj[n] {
            let ed = g.edges[*e];
            let m = if ed.a == n { ed.b } else { ed.a };
            if !seen[m] {
                seen[m] = true;
                parent[m] = Some((n, *e));
                q.push_back(m);
            }
        }
    }
    // What gathers at every node, from above.
    let mut ud = vec![0u32; n_nodes];
    let mut count = vec![0usize; n_nodes];
    let mut area = vec![0.0; n_nodes];
    let mut largest = vec![0.0f64; n_nodes];
    let mut downpipe_above = vec![false; n_nodes];
    for n in order.iter().rev() {
        if let Some(list) = drains_at.get(n) {
            for (_, a, _) in list {
                let (u, d) = units(*a, public);
                ud[*n] += u;
                count[*n] += 1;
                largest[*n] = largest[*n].max(d / 1000.0);
            }
        }
        if let Some(a) = rain_at.get(n) {
            area[*n] += a;
        }
        if let Some((p, e)) = parent[*n] {
            ud[p] += ud[*n];
            count[p] += count[*n];
            area[p] += area[*n];
            largest[p] = largest[p].max(largest[*n]);
            if g.edges[e].riser.is_some() || downpipe_above[*n] {
                downpipe_above[p] = true;
            }
        }
    }
    let carries = |n: usize| count[n] > 0 || area[n] > 0.0;
    // Stretches.
    let mut stretch_of_edge: BTreeMap<usize, usize> = BTreeMap::new();
    let mut stretches: Vec<Stretch> = Vec::new();
    let mut end: Vec<usize> = Vec::new();
    for n in &order {
        let Some((p, e)) = parent[*n] else {
            continue;
        };
        if !carries(*n) {
            continue;
        }
        let ed = g.edges[e];
        let cont = parent[p]
            .and_then(|(_, pe)| stretch_of_edge.get(&pe).copied())
            .filter(|s| {
                let st = &stretches[*s];
                st.run == ed.run
                    && st.downpipe == ed.riser
                    && st.units == ud[*n]
                    && (st.area - area[*n]).abs() < 1e-6
                    && end[*s] == p
            });
        if let Some(s) = cont {
            stretches[s].length += ed.length;
            end[s] = *n;
            stretch_of_edge.insert(e, s);
            continue;
        }
        let run = ed.run.and_then(|r| model.runs().find(|x| x.id == r));
        let drawn = match (run, ed.riser) {
            (Some(r), _) => r.width,
            (None, Some(d)) => model
                .devices()
                .find(|x| x.id == d)
                .map_or(0.11, |x| x.width),
            _ => 0.11,
        };
        // Under the floor of the lowest level: in the ground.
        let buried = run.is_some_and(|r| {
            Some(r.level) == lowest_level && r.elevation.min(r.end_elevation()) < 0.0
        });
        let kind = if ed.riser.is_some() {
            Kind::Downpipe
        } else if rain {
            Kind::Collector
        } else if count[*n] == 1 && !downpipe_above[*n] {
            Kind::Branch
        } else if downpipe_above[*n] || buried || parent[p].is_none() {
            Kind::Collector
        } else {
            Kind::Gathering
        };
        let up = parent[p].and_then(|(_, pe)| stretch_of_edge.get(&pe).copied());
        stretches.push(Stretch {
            name: String::new(),
            run: ed.run,
            downpipe: ed.riser,
            level: g.nodes[*n].level,
            parent: up,
            kind,
            length: ed.length,
            slope: run.map_or(0.0, |r| r.slope),
            units: ud[*n],
            appliances: count[*n],
            area: area[*n],
            drawn,
            sized: 0.0,
            rule: "",
            fits: true,
        });
        end.push(*n);
        stretch_of_edge.insert(e, stretches.len() - 1);
    }
    // The largest appliance drain above every stretch.
    let mut largest_of = vec![0.0f64; stretches.len()];
    for (k, n) in end.iter().enumerate() {
        largest_of[k] = largest[*n];
    }
    // The table of each stretch.
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); stretches.len()];
    for (i, s) in stretches.iter().enumerate() {
        if let Some(p) = s.parent {
            children[p].push(i);
        }
    }
    for (i, s) in stretches.iter_mut().enumerate() {
        let col = slope_column(s.slope);
        let (d, fits, rule) = match (rain, s.kind) {
            (true, Kind::Downpipe) => {
                let (d, ok) = pick(&RAIN_DOWNPIPES, |c: f64| c >= s.area * f);
                (d, ok, "HS 5, tabla 4.8")
            }
            (true, _) => {
                let (d, ok) = pick(&RAIN_COLLECTORS, |c: [f64; 3]| c[col] >= s.area * f);
                (d, ok, "HS 5, tabla 4.9")
            }
            (false, Kind::Branch) => (pvc_at_least(largest_of[i]), true, "HS 5, tabla 4.1"),
            (false, Kind::Gathering) => {
                let (d, ok) = pick(&BRANCHES, |c: [u32; 3]| c[col] > 0 && c[col] >= s.units);
                (d, ok, "HS 5, tabla 4.3")
            }
            (false, Kind::Downpipe) => (0.0, true, "HS 5, tabla 4.4"),
            (false, Kind::Collector) => {
                let (d, ok) = pick(&COLLECTORS, |c: [u32; 3]| c[col] > 0 && c[col] >= s.units);
                (d, ok, "HS 5, tabla 4.5")
            }
        };
        s.sized = d.max(if rain {
            0.0
        } else {
            pvc_at_least(largest_of[i])
        });
        s.fits = fits;
        s.rule = rule;
    }
    // The downpipes of waste water: by all they take, the most from one
    // floor and how many floors.
    if !rain {
        let mut by_pipe: BTreeMap<DeviceId, Vec<usize>> = BTreeMap::new();
        for (i, s) in stretches.iter().enumerate() {
            if let Some(d) = s.downpipe {
                by_pipe.entry(d).or_default().push(i);
            }
        }
        for list in by_pipe.values() {
            let total = list.iter().map(|i| stretches[*i].units).max().unwrap_or(0);
            let mut floors: BTreeSet<LevelId> = BTreeSet::new();
            let mut most = 0;
            for i in list {
                for c in &children[*i] {
                    if stretches[*c].downpipe.is_none() {
                        floors.insert(stretches[*c].level);
                        most = most.max(stretches[*c].units);
                    }
                }
            }
            let many = floors.len() > 3;
            let (d, ok) = pick(&DOWNPIPES, |c: [u32; 4]| {
                let (all, one) = if many { (c[1], c[3]) } else { (c[0], c[2]) };
                all >= total && one >= most
            });
            for i in list {
                stretches[*i].sized = stretches[*i].sized.max(d);
                stretches[*i].fits = ok;
            }
        }
    }
    // Nothing smaller downstream; a downpipe the same all along.
    for _ in 0..3 {
        for i in (0..stretches.len()).rev() {
            let up = children[i]
                .iter()
                .map(|c| stretches[*c].sized)
                .fold(0.0, f64::max);
            if up > stretches[i].sized {
                stretches[i].sized = up;
            }
        }
        let mut most: BTreeMap<DeviceId, f64> = BTreeMap::new();
        for s in &stretches {
            if let Some(d) = s.downpipe {
                let e = most.entry(d).or_insert(0.0);
                *e = e.max(s.sized);
            }
        }
        for s in &mut stretches {
            if let Some(d) = s.downpipe {
                s.sized = most[&d];
            }
        }
    }
    let prefix = if rain { "P" } else { "S" };
    for (i, s) in stretches.iter_mut().enumerate() {
        s.name = format!("{prefix}{}", i + 1);
    }
    let path_of = |mut n: usize| {
        let mut v = Vec::new();
        while let Some((p, e)) = parent[n] {
            if let Some(s) = stretch_of_edge.get(&e) {
                if v.last() != Some(s) {
                    v.push(*s);
                }
            }
            n = p;
        }
        v.reverse();
        v
    };
    let route_of = |mut n: usize| {
        let mut v = Vec::new();
        let mut along = 0.0;
        loop {
            let node = g.nodes[n];
            let step = parent[n];
            v.push(RoutePoint {
                level: node.level,
                p: node.p,
                z: node.z,
                along,
                downpipe: g.adj[n].iter().any(|e| g.edges[*e].riser.is_some()),
                stretch: step.and_then(|(_, e)| stretch_of_edge.get(&e).copied()),
            });
            match step {
                Some((p, e)) => {
                    along += g.edges[e].length;
                    n = p;
                }
                None => break,
            }
        }
        v
    };
    let mut drains = Vec::new();
    for (n, list) in drains_at {
        if !seen[*n] {
            continue;
        }
        for (object, kind, level) in list {
            drains.push(Drain {
                object: *object,
                kind: *kind,
                level: *level,
                units: units(*kind, public).0,
                path: path_of(*n),
                point: g.nodes[*n].p,
                route: route_of(*n),
            });
        }
    }
    drains.sort_by_key(|d| d.object);
    let inlets = rain_at
        .iter()
        .filter(|(n, _)| seen[**n])
        .map(|(n, a)| Inlet {
            level: g.nodes[*n].level,
            point: g.nodes[*n].p,
            area: *a,
            path: path_of(*n),
        })
        .collect();
    Network {
        system,
        outlet: (g.nodes[root].level, g.nodes[root].p),
        stretches,
        drains,
        inlets,
        joins: None,
    }
}

/// The waste stretches downstream of where a rain network ends carry the
/// rain too: they take tabla 4.9 with the units as an equivalent area.
fn mix(model: &Model, net: &mut Network, p: Point2, level: LevelId, rain: f64, f: f64) {
    let at = net
        .stretches
        .iter()
        .enumerate()
        .filter(|(_, s)| s.level == level)
        .filter_map(|(i, s)| {
            let r = model.runs().find(|x| Some(x.id) == s.run)?;
            Some((i, r.locate(p).1))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0);
    let mut cur = at;
    while let Some(i) = cur {
        let s = &mut net.stretches[i];
        s.area += rain;
        let want = s.area * f + equivalent_area(s.units) * f;
        let col = slope_column(s.slope);
        let (d, ok) = pick(&RAIN_COLLECTORS, |c: [f64; 3]| c[col] >= want);
        s.sized = s.sized.max(d);
        s.fits = s.fits && ok;
        s.rule = "HS 5, 4.3 y tabla 4.9";
        cur = s.parent;
    }
}

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

fn mm(v: f64) -> String {
    format!("Ø{:.0}", v * 1000.0)
}

/// Where a network ends, in words.
pub fn outlet_name(model: &Model, n: &Network) -> String {
    let level = model.level(n.outlet.0).map_or("", |l| l.name.as_str());
    format!(
        "Acometida ({level}, x {} y {})",
        dec(n.outlet.1.x, 1),
        dec(n.outlet.1.y, 1)
    )
}

/// The calculation in text, as the report's annex.
pub fn report(model: &Model, d: &Drainage) -> String {
    let mut s = String::new();
    s.push_str("Evacuación de aguas (CTE DB-HS 5)\n");
    s.push_str(&format!(
        "  Uso {} (tabla 4.1) · intensidad de lluvia {} mm/h (factor {}) · tubería de PVC\n",
        if d.public { "público" } else { "privado" },
        dec(d.intensity, 0),
        dec(d.factor(), 2)
    ));
    if d.networks.is_empty() {
        s.push_str("\nNo hay desagües dibujados que lleguen a un aparato ni bajantes de pluviales con cubierta.\n");
    }
    for n in &d.networks {
        let rain = n.system == ServiceSystem::Rainwater;
        s.push('\n');
        if rain {
            s.push_str(&format!(
                "Pluviales — {}: {} m² de cubierta{}\n",
                outlet_name(model, n),
                dec(n.area(), 1),
                match n.joins {
                    Some(w) => format!(" · acaba en la red de residuales {} (mixta)", w + 1),
                    None => String::new(),
                }
            ));
        } else {
            s.push_str(&format!(
                "Residuales — {}: {} aparatos, {} UD\n",
                outlet_name(model, n),
                n.drains.len(),
                n.units()
            ));
        }
        s.push_str("  Tramo Tipo        Tubería        L (m)  Pend.   UD   Sup. m²  Dibujado Exigido  Regla\n");
        for st in &n.stretches {
            let what = match (st.run, st.downpipe) {
                (Some(r), _) => format!("Tubería {}", r.0),
                (None, Some(dv)) => format!("Bajante {}", dv.0),
                _ => "—".into(),
            };
            s.push_str(&format!(
                "  {:<5} {:<11} {:<14} {:>5} {:>6} {:>4} {:>8}  {:>7} {:>7}{}  {}\n",
                st.name,
                st.kind.name(),
                what,
                dec(st.length, 2),
                if st.downpipe.is_some() {
                    "—".into()
                } else {
                    format!("{} %", dec(st.slope * 100.0, 1))
                },
                st.units,
                if st.area > 0.0 {
                    dec(st.area, 1)
                } else {
                    "—".into()
                },
                mm(st.drawn),
                mm(st.sized),
                if st.short() { " ←" } else { "  " },
                st.rule
            ));
        }
        let short = n.short().count();
        if short > 0 {
            s.push_str(&format!(
                "  {short} tramo{} dibujado{} más pequeño{} de lo exigido (←).\n",
                if short == 1 { "" } else { "s" },
                if short == 1 { "" } else { "s" },
                if short == 1 { "" } else { "s" }
            ));
        }
        if n.stretches.iter().any(|x| !x.fits) {
            s.push_str("  Algún tramo pide más de lo que dan las tablas: hay que partir la red.\n");
        }
    }
    let shown: Vec<&Surface> = d.surfaces.iter().filter(|x| x.checked).collect();
    if !shown.is_empty() {
        s.push_str("\nCubiertas y terrazas (tabla 4.6)\n");
        for x in shown {
            s.push_str(&format!(
                "  {}: {} m², {} sumidero{} (exige {}){}\n",
                x.name,
                dec(x.area, 1),
                x.outlets,
                if x.outlets == 1 { "" } else { "s" },
                x.needed,
                if x.undrained > 1.0 {
                    format!(" · {} m² sin desagüe", dec(x.undrained, 1))
                } else {
                    String::new()
                }
            ));
        }
    }
    if !d.gutters.is_empty() {
        s.push_str("\nCanalones (tabla 4.7, al 1 %) y sus bajantes (tabla 4.8)\n");
        for gc in &d.gutters {
            s.push_str(&format!(
                "  {} lado {}: {} bajante{}, hasta {} m² por bajante · canalón {} mm (exige {}) · bajante {} (exige {})\n",
                gc.roof,
                gc.side + 1,
                gc.downpipes,
                if gc.downpipes == 1 { "" } else { "s" },
                dec(gc.area, 1),
                dec(gc.gutter * 1000.0, 0),
                dec(gc.gutter_sized * 1000.0, 0),
                mm(gc.downpipe),
                mm(gc.downpipe_sized)
            ));
        }
    }
    s.push_str(&crate::drain_vent::report(
        model,
        d,
        &crate::drain_vent::venting(model, d),
    ));
    if !d.loose.is_empty() {
        s.push_str(&format!(
            "\n{} aparato{} sin desagüe a menos de {} cm.\n",
            d.loose.len(),
            if d.loose.len() == 1 { "" } else { "s" },
            dec(APPLIANCE_REACH * 100.0, 0)
        ));
    }
    s
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use archisim_core::{DeviceKind, NewDevice, NewObject, NewRun};

    #[test]
    fn tables() {
        assert_eq!(units(Appliance::Wc, false), (4, 100.0));
        assert_eq!(units(Appliance::Wc, true), (5, 100.0));
        assert_eq!(outlets_needed(80.0), 2);
        assert_eq!(outlets_needed(370.0), 4);
        assert_eq!(outlets_needed(900.0), 6);
        assert!((equivalent_area(100) - 90.0).abs() < 1e-9);
        assert!((equivalent_area(300) - 108.0).abs() < 1e-9);
        // A bathroom (7 UD) on a 2 % branch: Ø50 holds 6, Ø63 holds 11.
        assert_eq!(pick(&BRANCHES, |c: [u32; 3]| c[1] >= 7).0, 0.063);
        // 400 m² of roof at 2 %, at 100 mm/h: Ø125.
        assert_eq!(pick(&RAIN_COLLECTORS, |c: [f64; 3]| c[1] >= 400.0).0, 0.125);
    }

    /// Two bathrooms one over another on a downpipe, a buried collector to
    /// the outlet, and a flat roof with two outlets on a rain downpipe.
    pub(crate) fn house() -> Model {
        let mut m = archisim_library::new_project("Casa");
        let l0 = m.levels()[0].id;
        let l1 = m.add_level("Planta 1", 3.0);
        let wc = archisim_library::add_object_entry(&mut m, "Inodoro").unwrap();
        let basin = archisim_library::add_object_entry(&mut m, "Lavabo 60").unwrap();
        let shower = archisim_library::add_object_entry(&mut m, "Plato de ducha 90×90").unwrap();
        for l in [l0, l1] {
            for (t, x) in [(wc, 2.0), (basin, 3.5), (shower, 5.0)] {
                m.add_object(NewObject {
                    level: l,
                    object_type: t,
                    position: Point2::new(x, 0.8),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            }
        }
        let dev = |m: &mut Model, x: f64, y: f64, h: f64| {
            m.add_device(NewDevice {
                level: l0,
                kind: DeviceKind::Downpipe,
                name: "Bajante".into(),
                position: Point2::new(x, y),
                rotation: 0.0,
                elevation: 0.0,
                width: 0.11,
                depth: 0.11,
                height: h,
            })
            .unwrap()
        };
        dev(&mut m, 1.0, 0.0, 3.0);
        dev(&mut m, 8.0, 0.0, 6.0);
        let run = |m: &mut Model, level, system, path: Vec<Point2>, z: f64, w: f64, s: f64| {
            m.add_run(NewRun {
                level,
                system,
                path,
                elevation: z,
                width: w,
                height: w,
                slope: s,
            })
            .unwrap();
        };
        let d = ServiceSystem::Drainage;
        // Each floor: a branch along the bathroom to the downpipe.
        for l in [l0, l1] {
            let ps: Vec<Point2> = m.appliances_on(l).iter().map(|a| a.point).collect();
            run(
                &mut m,
                l,
                d,
                vec![Point2::new(6.0, 0.0), Point2::new(1.0, 0.0)],
                0.4,
                0.05,
                0.02,
            );
            for p in ps {
                run(
                    &mut m,
                    l,
                    d,
                    vec![p, Point2::new(p.x, 0.0)],
                    0.45,
                    0.04,
                    0.02,
                );
            }
        }
        // The buried collector from the downpipe's foot to the outlet.
        run(
            &mut m,
            l0,
            d,
            vec![Point2::new(1.0, 0.0), Point2::new(1.0, -3.0)],
            -0.5,
            0.09,
            0.02,
        );
        // Rain: the roof is the slab of floor 2 (none here), so a flat roof
        // at floor 1's level 2: use a slab on a new top level.
        let l2 = m.add_level("Cubierta", 6.0);
        let st = m.slab_types().next().unwrap().id;
        m.add_slab(archisim_core::NewSlab {
            slab_type: st,
            level: l2,
            outline: vec![
                Point2::new(0.0, -1.0),
                Point2::new(10.0, -1.0),
                Point2::new(10.0, 4.0),
                Point2::new(0.0, 4.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let r = ServiceSystem::Rainwater;
        run(
            &mut m,
            l2,
            r,
            vec![Point2::new(3.0, 2.0), Point2::new(8.0, 0.0)],
            -0.2,
            0.09,
            0.01,
        );
        run(
            &mut m,
            l2,
            r,
            vec![Point2::new(9.0, 3.0), Point2::new(8.0, 0.0)],
            -0.2,
            0.09,
            0.01,
        );
        run(
            &mut m,
            l0,
            r,
            vec![Point2::new(8.0, 0.0), Point2::new(8.0, -3.0)],
            -0.5,
            0.11,
            0.02,
        );
        m
    }

    #[test]
    fn a_house_drains() {
        let m = house();
        let d = drainage(&m);
        assert!(d.loose.is_empty(), "every appliance drains: {:?}", d.loose);
        let waste: Vec<&Network> = d
            .networks
            .iter()
            .filter(|n| n.system == ServiceSystem::Drainage)
            .collect();
        assert_eq!(waste.len(), 1);
        let w = waste[0];
        assert_eq!(w.drains.len(), 6);
        assert_eq!(w.units(), 2 * (4 + 1 + 2));
        // The WC's own drain asks Ø110; the downpipe, which takes it,
        // too; the collector after it, at least as much.
        let pipe = w
            .stretches
            .iter()
            .find(|s| s.kind == Kind::Downpipe)
            .expect("a downpipe");
        assert!((pipe.sized - 0.110).abs() < 1e-9, "{}", pipe.sized);
        let collector = w
            .stretches
            .iter()
            .find(|s| s.parent.is_none())
            .expect("the outlet's stretch");
        assert_eq!(collector.kind, Kind::Collector);
        assert!(collector.sized >= 0.110 - 1e-9);
        assert!(collector.short(), "drawn Ø90 under a WC");
        // The roof: 50 m² on two outlets, a rain network down to the
        // ground.
        let rain = d
            .networks
            .iter()
            .find(|n| n.system == ServiceSystem::Rainwater)
            .expect("rain network");
        assert!((rain.area() - 50.0).abs() < 1.0, "{}", rain.area());
        let roof = d
            .surfaces
            .iter()
            .find(|s| s.checked)
            .expect("the roof is checked");
        assert_eq!(roof.outlets, 2);
        assert_eq!(roof.needed, 2);
        let text = report(&m, &d);
        if std::env::var("ARCHISIM_SANEAMIENTO_TEXTO").is_ok() {
            println!("{text}");
        }
        assert!(text.contains("tabla 4.4"), "{text}");
        let (runs, pipes) = d.sized_runs();
        assert!(!runs.is_empty() && !pipes.is_empty());
    }
}
