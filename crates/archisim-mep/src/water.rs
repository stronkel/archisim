//! The water supply of a building by DB-HS 4, on what is drawn: the pipes
//! of cold and hot water, the risers that take them from floor to floor,
//! the meter, the heater and the sanitary appliances.
//!
//! For each of the two networks:
//!
//! - the **network** is read from the drawing as a tree: from its source
//!   (the meter for the cold water, or a tank or a booster set; the heater
//!   for the hot water) along the pipes, through the tees where a pipe ends
//!   on another, and up and down the risers, to the appliances at the ends;
//! - each **stretch** carries the flow of the appliances after it (HS 4,
//!   tabla 2.1) and, with the simultaneity of UNE 149201 for dwellings
//!   (Qc = 0,682·Qt^0,45 − 0,14), the design flow; in other buildings,
//!   Qc = Qt/√(n − 1);
//! - its **diameter**: the smallest usual size of the pipe chosen that
//!   keeps the water under 2 m/s and is no smaller than HS 4 asks (tabla
//!   4.2 for an appliance's branch; tabla 4.3, 20 mm, for what serves two
//!   or more and for the risers); then, while a tap is short of pressure,
//!   the stretch on its way that loses most takes the next size;
//! - its **pressure loss**: Darcy–Weisbach with the friction of
//!   Swamee–Jain, and 30 % more for the bends, tees and valves;
//! - the **pressure at each tap**: the one at the meter, less the losses on
//!   the way and the height climbed; against 100 kPa (HS 4, 2.1.3), 150 kPa
//!   at the heater, and 500 kPa at most.
//!
//! The pipes are worked out twice: as drawn (the check) and with the
//! diameters the calculation gives (what «Poner los diámetros» sets).

use std::collections::{BTreeMap, VecDeque};

use archisim_core::{
    Appliance, BuildingUse, DeviceId, DeviceKind, LevelId, Model, ObjectId, PipeMaterial, Point2,
    RunId, ServiceSystem,
};

/// The water's weight, kN/m³ (a metre of water is 9,81 kPa).
pub const WATER_WEIGHT: f64 = 9.81;
/// The fastest water the sizes are chosen for, m/s.
pub const DESIGN_SPEED: f64 = 2.0;
/// The bends, tees and valves, as a share of the straight length.
pub const FITTINGS: f64 = 0.30;
/// The least pressure at a tap (HS 4, 2.1.3), kPa.
pub const TAP_PRESSURE: f64 = 100.0;
/// The least pressure at a heater or a flush valve, kPa.
pub const HEATER_PRESSURE: f64 = 150.0;
/// The most pressure at any point, kPa: past it, a pressure reducer.
pub const MAX_PRESSURE: f64 = 500.0;
/// How near a pipe's end must be to another pipe, a riser or a device to
/// join it, metres.
const REACH: f64 = 0.06;
/// How near a pipe's end must be to an appliance, metres.
const APPLIANCE_REACH: f64 = 0.45;
/// How near a riser a pipe's end must be, metres.
const RISER_REACH: f64 = 0.25;
/// How near the meter, the heater or a tank a pipe's end must be, metres.
const SOURCE_REACH: f64 = 0.6;

/// The height of an appliance's tap over its floor, metres.
pub fn tap_height(a: Appliance) -> f64 {
    match a {
        Appliance::Shower => 2.0,
        Appliance::Bath => 0.8,
        Appliance::Wc => 0.2,
        Appliance::Washer | Appliance::Dishwasher => 0.6,
        _ => 1.0,
    }
}

/// The design flow of `qt` l/s of `n` appliances, the largest of which
/// takes `largest`, l/s.
pub fn simultaneous(qt: f64, n: usize, largest: f64, dwelling: bool) -> f64 {
    if n <= 1 {
        return qt;
    }
    let qc = if dwelling {
        if qt <= 20.0 {
            0.682 * qt.powf(0.45) - 0.14
        } else {
            1.7 * qt.powf(0.21) - 0.7
        }
    } else {
        qt / ((n - 1) as f64).sqrt()
    };
    qc.clamp(largest, qt)
}

/// The friction factor of Darcy (Swamee–Jain; 64/Re when laminar).
pub fn friction(reynolds: f64, roughness: f64, inside: f64) -> f64 {
    if reynolds < 1.0 {
        return 0.0;
    }
    if reynolds < 2300.0 {
        return 64.0 / reynolds;
    }
    let x = roughness / (3.7 * inside) + 5.74 / reynolds.powf(0.9);
    0.25 / x.log10().powi(2)
}

/// The pressure lost along `length` metres of pipe of `inside` diameter
/// carrying `flow` l/s, kPa (fittings included).
pub fn pressure_loss(
    flow: f64,
    inside: f64,
    length: f64,
    material: PipeMaterial,
    hot: bool,
) -> f64 {
    if flow <= 0.0 || inside <= 0.0 {
        return 0.0;
    }
    let area = std::f64::consts::PI * inside * inside / 4.0;
    let v = flow / 1000.0 / area;
    let nu = if hot { 0.47e-6 } else { 1.0e-6 };
    let f = friction(v * inside / nu, material.roughness(), inside);
    let head = f * length * (1.0 + FITTINGS) / inside * v * v / (2.0 * 9.81);
    head * WATER_WEIGHT
}

/// The speed of `flow` l/s in a pipe of `inside` diameter, m/s.
pub fn speed(flow: f64, inside: f64) -> f64 {
    if inside <= 0.0 {
        return 0.0;
    }
    flow / 1000.0 / (std::f64::consts::PI * inside * inside / 4.0)
}

/// Where a network starts.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A device: the meter, a tank, a booster set or the heater.
    Device(DeviceId, DeviceKind),
    /// No such device is drawn: the lowest free end of the pipes.
    End(LevelId, Point2),
}

/// A stretch of pipe between two branchings: part of a run, or a riser.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    /// «T3», in the order they are met from the source.
    pub name: String,
    pub run: Option<RunId>,
    pub riser: Option<DeviceId>,
    pub level: LevelId,
    /// The stretch it hangs from (`None`: it leaves the source).
    pub parent: Option<usize>,
    /// Its length, metres (along the pipe; a riser, its height).
    pub length: f64,
    /// The appliances after it, their flow and the design flow, l/s.
    pub appliances: usize,
    pub total_flow: f64,
    pub flow: f64,
    /// The pipe drawn: outside and inside diameter, metres (a riser, the
    /// pipe that reaches it).
    pub drawn: (f64, f64),
    /// The size the calculation gives: outside and inside diameter.
    pub sized: (f64, f64),
    /// The least outside diameter the code asks (HS 4, tablas 4.2 y 4.3).
    pub minimum: f64,
    /// Speed and pressure loss, as drawn and as sized.
    pub speed: f64,
    pub loss: f64,
    pub sized_speed: f64,
    pub sized_loss: f64,
}

/// A tap at the end of the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Tap {
    pub object: ObjectId,
    pub kind: Appliance,
    pub level: LevelId,
    /// Its flow, l/s.
    pub flow: f64,
    /// The pressure left at it, kPa, as drawn and as sized.
    pub pressure: f64,
    pub sized_pressure: f64,
    /// The stretches from the source to it.
    pub path: Vec<usize>,
}

/// One network: cold or hot water.
#[derive(Debug, Clone, PartialEq)]
pub struct Network {
    pub system: ServiceSystem,
    pub source: Source,
    /// The pressure at the source, kPa.
    pub source_pressure: f64,
    pub stretches: Vec<Stretch>,
    pub taps: Vec<Tap>,
    /// Appliances that should be on this network and are not joined to it.
    pub loose: Vec<ObjectId>,
    /// Pipes of the system not reached from the source.
    pub unreached: Vec<RunId>,
    /// The heater, with the pressure the cold water brings it (on the cold
    /// network only).
    pub heater: Option<(DeviceId, f64, f64)>,
}

impl Network {
    /// The tap with the least pressure, as drawn.
    pub fn worst(&self) -> Option<&Tap> {
        self.taps
            .iter()
            .min_by(|a, b| a.pressure.total_cmp(&b.pressure))
    }

    /// The outside diameter the calculation gives to each run: the largest
    /// of its stretches.
    pub fn sized_runs(&self) -> BTreeMap<RunId, f64> {
        let mut out: BTreeMap<RunId, f64> = BTreeMap::new();
        for s in &self.stretches {
            if let Some(r) = s.run {
                let e = out.entry(r).or_insert(0.0);
                *e = e.max(s.sized.0);
            }
        }
        out
    }
}

/// The water supply worked out.
#[derive(Debug, Clone, PartialEq)]
pub struct Water {
    pub material: PipeMaterial,
    pub pressure: f64,
    pub dwelling: bool,
    pub networks: Vec<Network>,
}

impl Water {
    /// The outside diameter the calculation gives to each run of both
    /// networks.
    pub fn sized_runs(&self) -> BTreeMap<RunId, f64> {
        let mut out = BTreeMap::new();
        for n in &self.networks {
            out.extend(n.sized_runs());
        }
        out
    }
}

/// A point of the network graph.
#[derive(Debug, Clone, Copy)]
struct Node {
    level: LevelId,
    p: Point2,
    /// Absolute height, metres.
    z: f64,
}

/// A piece of pipe between two points.
#[derive(Debug, Clone, Copy)]
struct Edge {
    a: usize,
    b: usize,
    length: f64,
    run: Option<RunId>,
    riser: Option<DeviceId>,
}

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    adj: Vec<Vec<usize>>,
}

impl Graph {
    fn node(&mut self, level: LevelId, p: Point2, z: f64) -> usize {
        if let Some(i) = self
            .nodes
            .iter()
            .position(|n| n.level == level && n.p.distance(p) < REACH && (n.z - z).abs() < 0.5)
        {
            return i;
        }
        self.nodes.push(Node { level, p, z });
        self.adj.push(Vec::new());
        self.nodes.len() - 1
    }

    fn edge(
        &mut self,
        a: usize,
        b: usize,
        length: f64,
        run: Option<RunId>,
        riser: Option<DeviceId>,
    ) {
        if a == b {
            return;
        }
        self.edges.push(Edge {
            a,
            b,
            length,
            run,
            riser,
        });
        let i = self.edges.len() - 1;
        self.adj[a].push(i);
        self.adj[b].push(i);
    }

    fn nearest(&self, level: LevelId, p: Point2, reach: f64) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.level == level && n.p.distance(p) < reach)
            .min_by(|a, b| a.1.p.distance(p).total_cmp(&b.1.p.distance(p)))
            .map(|(i, _)| i)
    }
}

fn level_z(model: &Model, level: LevelId) -> f64 {
    model.level(level).map_or(0.0, |l| l.elevation)
}

/// The pipes of a system as a graph: their points, the tees where a pipe
/// ends on another, and the risers.
fn graph(model: &Model, system: ServiceSystem) -> Graph {
    let mut g = Graph {
        nodes: Vec::new(),
        edges: Vec::new(),
        adj: Vec::new(),
    };
    let runs: Vec<_> = model.runs().filter(|r| r.system == system).collect();
    // The ends of every run, to split the others where they land (tees).
    let ends: Vec<(LevelId, Point2)> = runs
        .iter()
        .flat_map(|r| {
            let n = r.path.len();
            [(r.level, r.path[0]), (r.level, r.path[n - 1])]
        })
        .collect();
    for r in &runs {
        let base = level_z(model, r.level);
        let zs = r.point_elevations();
        let mut prev: Option<(usize, Point2, f64)> = None;
        for (i, w) in r.path.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let (za, zb) = (base + zs[i], base + zs[i + 1]);
            let len = a.distance(b);
            // The points along the stretch where another pipe ends.
            let mut cuts: Vec<f64> = ends
                .iter()
                .filter(|(l, _)| *l == r.level)
                .filter_map(|(_, p)| {
                    if len < 1e-9 {
                        return None;
                    }
                    let d = b - a;
                    let t = (*p - a).dot(d) / (len * len);
                    let q = a + d * t;
                    (t > 1e-3 && t < 1.0 - 1e-3 && q.distance(*p) < REACH).then_some(t)
                })
                .collect();
            cuts.push(1.0);
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-4);
            let (mut from, mut t0) = match prev {
                Some((n, _, _)) => (n, 0.0),
                None => (g.node(r.level, a, za), 0.0),
            };
            for t in cuts {
                let p = a + (b - a) * t;
                let z = za + (zb - za) * t;
                let to = g.node(r.level, p, z);
                let l = len * (t - t0);
                let dz = (zb - za) * (t - t0);
                g.edge(from, to, (l * l + dz * dz).sqrt(), Some(r.id), None);
                from = to;
                t0 = t;
            }
            prev = Some((from, b, zb));
        }
    }
    // The risers: from floor to floor at their place.
    let levels = model.levels();
    for d in model
        .devices()
        .filter(|d| d.kind == DeviceKind::Riser || d.kind == DeviceKind::Downpipe)
    {
        if d.kind == DeviceKind::Downpipe && system != ServiceSystem::Drainage {
            continue;
        }
        let bottom = level_z(model, d.level) + d.elevation;
        let top = bottom + d.height;
        let mut on: Vec<usize> = levels
            .iter()
            .filter(|l| l.elevation >= bottom - 0.6 && l.elevation <= top + 0.01)
            .filter_map(|l| g.nearest(l.id, d.position, RISER_REACH))
            .collect();
        on.sort_by(|a, b| g.nodes[*a].z.total_cmp(&g.nodes[*b].z));
        on.dedup();
        for w in on.windows(2) {
            let l = (g.nodes[w[1]].z - g.nodes[w[0]].z).abs();
            g.edge(w[0], w[1], l, None, Some(d.id));
        }
    }
    g
}

/// The pipe that reaches a riser at a node: the largest drawn diameter of
/// the runs there (a riser has no diameter of its own; the pipes on its
/// two floors give it).
fn riser_pipe(g: &Graph, model: &Model, node: usize) -> Option<f64> {
    g.adj[node]
        .iter()
        .filter_map(|e| g.edges[*e].run)
        .filter_map(|r| model.runs().find(|x| x.id == r).map(|x| x.width))
        .max_by(f64::total_cmp)
}

/// The least outside diameter HS 4 asks of a stretch: an appliance's
/// branch (tabla 4.2) or, serving two or more or going up a riser, 20 mm
/// (tabla 4.3: «alimentación a cuarto húmedo privado», «columna»).
fn minimum(appliances: &[Appliance], riser: bool) -> f64 {
    if riser || appliances.len() > 1 {
        return 0.020;
    }
    appliances.first().map_or(0.012, |a| a.supply() / 1000.0)
}

/// The smallest usual size of `material` of at least `minimum` outside
/// that carries `flow` l/s under the design speed (the largest if none).
pub fn size_for(material: PipeMaterial, flow: f64, minimum: f64) -> (f64, f64) {
    let sizes = material.sizes();
    sizes
        .iter()
        .copied()
        .filter(|s| s.0 + 1e-6 >= minimum)
        .find(|s| speed(flow, s.1) <= DESIGN_SPEED)
        .unwrap_or(sizes[sizes.len() - 1])
}

fn network(model: &Model, system: ServiceSystem, source_pressure: f64) -> Option<Network> {
    let site = model.water_site();
    let material = site.material;
    let hot = system == ServiceSystem::HotWater;
    let dwelling = model.code_settings().building_use == BuildingUse::Dwelling;
    let g = graph(model, system);
    if g.nodes.is_empty() {
        return None;
    }
    // The appliances, at the free end nearest each.
    let mut taps_at: BTreeMap<usize, Vec<(ObjectId, Appliance, LevelId)>> = BTreeMap::new();
    let mut loose = Vec::new();
    for l in model.levels() {
        for a in model.appliances_on(l.id) {
            if hot && !a.kind.hot() {
                continue;
            }
            match g.nearest(a.level, a.point, APPLIANCE_REACH) {
                Some(n) => taps_at
                    .entry(n)
                    .or_default()
                    .push((a.object, a.kind, a.level)),
                None => loose.push(a.object),
            }
        }
    }
    // The source.
    let wanted: &[DeviceKind] = if hot {
        &[DeviceKind::WaterHeater]
    } else {
        &[DeviceKind::WaterMeter, DeviceKind::Tank, DeviceKind::Pump]
    };
    let mut source: Option<(usize, Source)> = None;
    for kind in wanted {
        for d in model.devices().filter(|d| d.kind == *kind) {
            if let Some(n) = g.nearest(d.level, d.position, SOURCE_REACH) {
                source = Some((n, Source::Device(d.id, d.kind)));
                break;
            }
        }
        if source.is_some() {
            break;
        }
    }
    let (root, source) = match source {
        Some(s) => s,
        None => {
            // The lowest free end that is not an appliance's.
            let n = (0..g.nodes.len())
                .filter(|i| g.adj[*i].len() == 1 && !taps_at.contains_key(i))
                .min_by(|a, b| g.nodes[*a].z.total_cmp(&g.nodes[*b].z))
                .or_else(|| {
                    (0..g.nodes.len()).min_by(|a, b| g.nodes[*a].z.total_cmp(&g.nodes[*b].z))
                })?;
            (n, Source::End(g.nodes[n].level, g.nodes[n].p))
        }
    };
    // The tree from the source.
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; g.nodes.len()];
    let mut seen = vec![false; g.nodes.len()];
    let mut order = Vec::new();
    let mut queue = VecDeque::from([root]);
    seen[root] = true;
    while let Some(n) = queue.pop_front() {
        order.push(n);
        for e in &g.adj[n] {
            let ed = g.edges[*e];
            let m = if ed.a == n { ed.b } else { ed.a };
            if !seen[m] {
                seen[m] = true;
                parent[m] = Some((n, *e));
                queue.push_back(m);
            }
        }
    }
    // What each node feeds.
    let mut below: Vec<Vec<(ObjectId, Appliance, LevelId)>> = vec![Vec::new(); g.nodes.len()];
    for n in order.iter().rev() {
        if let Some(t) = taps_at.get(n) {
            below[*n].extend(t.iter().copied());
        }
        if let Some((p, _)) = parent[*n] {
            let mine = below[*n].clone();
            below[p].extend(mine);
        }
    }
    for (n, t) in &taps_at {
        if !seen[*n] {
            loose.extend(t.iter().map(|x| x.0));
        }
    }
    let flow_of = |a: Appliance| if hot { a.flows().1 } else { a.flows().0 };
    // Stretches: the edges joined while they carry the same appliances,
    // on the same run, one after another.
    let mut stretch_of_edge: BTreeMap<usize, usize> = BTreeMap::new();
    let mut stretches: Vec<Stretch> = Vec::new();
    let mut stretch_end: Vec<usize> = Vec::new();
    for n in &order {
        let Some((p, e)) = parent[*n] else {
            continue;
        };
        let ed = g.edges[e];
        let fed = &below[*n];
        if fed.is_empty() {
            continue;
        }
        // Continue the parent's stretch if it is the same pipe and feeds
        // the same appliances.
        let cont = parent[p]
            .and_then(|(_, pe)| stretch_of_edge.get(&pe).copied())
            .filter(|s| {
                let st = &stretches[*s];
                st.run == ed.run
                    && st.riser == ed.riser
                    && st.appliances == fed.len()
                    && stretch_end[*s] == p
            });
        let kinds: Vec<Appliance> = fed.iter().map(|x| x.1).collect();
        let qt: f64 = kinds.iter().map(|a| flow_of(*a)).sum();
        let largest = kinds.iter().map(|a| flow_of(*a)).fold(0.0, f64::max);
        let qc = simultaneous(qt, kinds.len(), largest, dwelling);
        match cont {
            Some(s) => {
                stretches[s].length += ed.length;
                stretch_end[s] = *n;
                stretch_of_edge.insert(e, s);
            }
            None => {
                let drawn_outside = match ed.run {
                    Some(r) => model.runs().find(|x| x.id == r).map_or(0.016, |x| x.width),
                    None => match (riser_pipe(&g, model, *n), riser_pipe(&g, model, p)) {
                        (Some(a), Some(b)) => a.max(b),
                        (a, b) => a.or(b).unwrap_or(0.020),
                    },
                };
                let min = minimum(&kinds, ed.riser.is_some());
                let sized = size_for(material, qc, min);
                let up = parent[p].and_then(|(_, pe)| stretch_of_edge.get(&pe).copied());
                stretches.push(Stretch {
                    name: String::new(),
                    run: ed.run,
                    riser: ed.riser,
                    level: g.nodes[*n].level,
                    parent: up,
                    length: ed.length,
                    appliances: kinds.len(),
                    total_flow: qt,
                    flow: qc,
                    drawn: (drawn_outside, material.inside_of(drawn_outside)),
                    sized,
                    minimum: min,
                    speed: 0.0,
                    loss: 0.0,
                    sized_speed: 0.0,
                    sized_loss: 0.0,
                });
                stretch_end.push(*n);
                stretch_of_edge.insert(e, stretches.len() - 1);
            }
        }
    }
    for (i, s) in stretches.iter_mut().enumerate() {
        s.name = format!("{}{}", if hot { "C" } else { "F" }, i + 1);
        s.speed = speed(s.flow, s.drawn.1);
        s.sized_speed = speed(s.flow, s.sized.1);
        s.loss = pressure_loss(s.flow, s.drawn.1, s.length, material, hot);
        s.sized_loss = pressure_loss(s.flow, s.sized.1, s.length, material, hot);
    }
    // The pressure at every node, as drawn and as sized.
    let mut pressure = vec![(f64::NAN, f64::NAN); g.nodes.len()];
    pressure[root] = (source_pressure, source_pressure);
    for n in &order {
        let Some((p, e)) = parent[*n] else {
            continue;
        };
        let rise = (g.nodes[*n].z - g.nodes[p].z) * WATER_WEIGHT;
        let (loss, sloss) = match stretch_of_edge.get(&e) {
            Some(s) => {
                let st = &stretches[*s];
                let k = if st.length > 1e-9 {
                    g.edges[e].length / st.length
                } else {
                    0.0
                };
                (st.loss * k, st.sized_loss * k)
            }
            None => (0.0, 0.0),
        };
        pressure[*n] = (pressure[p].0 - loss - rise, pressure[p].1 - sloss - rise);
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
    let mut taps = Vec::new();
    for (n, list) in &taps_at {
        if !seen[*n] {
            continue;
        }
        for (object, kind, level) in list {
            let up = (level_z(model, *level) + tap_height(*kind) - g.nodes[*n].z) * WATER_WEIGHT;
            taps.push(Tap {
                object: *object,
                kind: *kind,
                level: *level,
                flow: flow_of(*kind),
                pressure: pressure[*n].0 - up,
                sized_pressure: pressure[*n].1 - up,
                path: path_of(*n),
            });
        }
    }
    taps.sort_by_key(|t| t.object);
    // Sized for speed, a far tap may still be short of pressure: the
    // stretch on its way that loses most takes the next size, until it
    // has enough (or nothing can grow).
    let heights: Vec<f64> = taps
        .iter()
        .map(|t| {
            source_pressure - t.pressure - t.path.iter().map(|i| stretches[*i].loss).sum::<f64>()
        })
        .collect();
    let sizes = material.sizes();
    for _ in 0..400 {
        let Some(worst) = (0..taps.len())
            .filter(|i| taps[*i].sized_pressure < TAP_PRESSURE + 5.0)
            .min_by(|a, b| taps[*a].sized_pressure.total_cmp(&taps[*b].sized_pressure))
        else {
            break;
        };
        let grow = taps[worst]
            .path
            .iter()
            .copied()
            .filter(|i| sizes.iter().any(|z| z.0 > stretches[*i].sized.0 + 1e-6))
            .max_by(|a, b| {
                stretches[*a]
                    .sized_loss
                    .total_cmp(&stretches[*b].sized_loss)
            });
        let Some(i) = grow else {
            break;
        };
        let st = &mut stretches[i];
        if let Some(next) = sizes.iter().find(|z| z.0 > st.sized.0 + 1e-6) {
            st.sized = *next;
            st.sized_speed = speed(st.flow, st.sized.1);
            st.sized_loss = pressure_loss(st.flow, st.sized.1, st.length, material, hot);
        }
        for (t, h) in taps.iter_mut().zip(&heights) {
            t.sized_pressure =
                source_pressure - h - t.path.iter().map(|i| stretches[*i].sized_loss).sum::<f64>();
        }
    }
    // The heater on the cold network: the pressure it gets.
    let heater = if hot {
        None
    } else {
        model
            .devices()
            .filter(|d| d.kind == DeviceKind::WaterHeater)
            .find_map(|d| {
                let n = g.nearest(d.level, d.position, SOURCE_REACH)?;
                seen[n].then_some((d.id, pressure[n].0, pressure[n].1))
            })
    };
    let mut unreached: Vec<RunId> = g
        .edges
        .iter()
        .filter(|e| !seen[e.a] && !seen[e.b])
        .filter_map(|e| e.run)
        .collect();
    unreached.sort();
    unreached.dedup();
    loose.sort();
    loose.dedup();
    Some(Network {
        system,
        source,
        source_pressure,
        stretches,
        taps,
        loose,
        unreached,
        heater,
    })
}

/// The water supply of the model.
pub fn water(model: &Model) -> Water {
    let site = *model.water_site();
    let mut networks = Vec::new();
    let cold = network(model, ServiceSystem::ColdWater, site.pressure);
    // The hot water starts at the heater with what the cold water brings
    // it.
    let (heater_in, heater_sized) = cold
        .as_ref()
        .and_then(|c| c.heater.map(|h| (h.1, h.2)))
        .unwrap_or((site.pressure, site.pressure));
    if let Some(c) = cold {
        networks.push(c);
    }
    if let Some(mut h) = network(model, ServiceSystem::HotWater, heater_in) {
        // As sized, from the pressure the sized cold network brings.
        let d = heater_sized - heater_in;
        for t in &mut h.taps {
            t.sized_pressure += d;
        }
        networks.push(h);
    }
    Water {
        material: site.material,
        pressure: site.pressure,
        dwelling: model.code_settings().building_use == BuildingUse::Dwelling,
        networks,
    }
}

/// The calculation in text, as the report's annex.
pub fn report(model: &Model, w: &Water) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let dec = |v: f64, n: usize| format!("{v:.n$}").replace('.', ",");
    let _ = writeln!(s, "Suministro de agua (CTE DB-HS 4)");
    let _ = writeln!(
        s,
        "  Presión en el contador: {} kPa · tubería: {}",
        dec(w.pressure, 0),
        w.material.name()
    );
    let _ = writeln!(
        s,
        "  Simultaneidad: {} · velocidad de cálculo ≤ {} m/s · pérdidas locales +{} %",
        if w.dwelling {
            "UNE 149201 viviendas, Qc = 0,682·Qt^0,45 − 0,14"
        } else {
            "Qc = Qt/√(n − 1)"
        },
        dec(DESIGN_SPEED, 1),
        dec(FITTINGS * 100.0, 0)
    );
    if w.networks.is_empty() {
        let _ = writeln!(s, "\n  No hay tuberías de agua fría ni caliente dibujadas.");
        return s;
    }
    for n in &w.networks {
        let _ = writeln!(
            s,
            "\n{} — desde {} ({} kPa)",
            n.system.name(),
            source_name(model, n),
            dec(n.source_pressure, 0)
        );
        if n.stretches.is_empty() {
            let _ = writeln!(
                s,
                "  Ninguna tubería lleva agua a un aparato: dibuja las derivaciones hasta ellos (Tubería → «Conectar los aparatos de esta planta»)."
            );
        }
        let _ = writeln!(
            s,
            "  {:<5} {:<14} {:>6} {:>4} {:>6} {:>6} {:>9} {:>6} {:>7} {:>9} {:>6} {:>7}",
            "Tramo",
            "Tubería",
            "L (m)",
            "n",
            "Qt",
            "Qc",
            "Dibujado",
            "v",
            "ΔP kPa",
            "Calculado",
            "v",
            "ΔP kPa"
        );
        for st in &n.stretches {
            let what = match (st.run, st.riser) {
                (Some(r), _) => format!("Tubería {}", r.0),
                (None, Some(d)) => format!("Montante {}", d.0),
                _ => String::new(),
            };
            let _ = writeln!(
                s,
                "  {:<5} {:<14} {:>6} {:>4} {:>6} {:>6} {:>9} {:>6} {:>7} {:>9} {:>6} {:>7}",
                st.name,
                what,
                dec(st.length, 2),
                st.appliances,
                dec(st.total_flow, 2),
                dec(st.flow, 2),
                format!("Ø{:.0}", st.drawn.0 * 1000.0),
                dec(st.speed, 2),
                dec(st.loss, 1),
                format!("Ø{:.0}", st.sized.0 * 1000.0),
                dec(st.sized_speed, 2),
                dec(st.sized_loss, 1)
            );
        }
        let _ = writeln!(s, "  Presión en cada grifo (kPa; mínimo {}):", TAP_PRESSURE);
        for t in &n.taps {
            let _ = writeln!(
                s,
                "    {:<22} {:<16} {:>6} dibujado · {:>6} calculado{}",
                t.kind.name(),
                model
                    .level(t.level)
                    .map_or(String::new(), |l| l.name.clone()),
                dec(t.pressure, 0),
                dec(t.sized_pressure, 0),
                if t.pressure < TAP_PRESSURE {
                    "  NO CUMPLE"
                } else {
                    ""
                }
            );
        }
        if let Some((_, p, _)) = n.heater {
            let _ = writeln!(
                s,
                "  Al termo llega: {} kPa (mínimo {})",
                dec(p, 0),
                HEATER_PRESSURE
            );
        }
        if !n.loose.is_empty() {
            let _ = writeln!(s, "  Aparatos sin unir a esta red: {}", n.loose.len());
        }
        if !n.unreached.is_empty() {
            let _ = writeln!(
                s,
                "  Tuberías que no llegan desde el origen: {}",
                n.unreached
                    .iter()
                    .map(|r| r.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    s
}

/// The name of where a network starts.
pub fn source_name(model: &Model, n: &Network) -> String {
    match &n.source {
        Source::Device(id, kind) => model
            .devices()
            .find(|d| d.id == *id)
            .map_or_else(|| kind.name().to_string(), |d| d.name.clone()),
        Source::End(level, _) => format!(
            "el extremo más bajo de las tuberías ({}; no hay contador dibujado)",
            model
                .level(*level)
                .map_or(String::new(), |l| l.name.clone())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewDevice, NewObject, NewRun};

    #[test]
    fn simultaneity_and_sizes() {
        // One appliance: its own flow.
        assert_eq!(simultaneous(0.2, 1, 0.2, true), 0.2);
        // A bathroom (lavabo, inodoro, ducha, bidé: 0,5 l/s): about 0,36.
        let q = simultaneous(0.5, 4, 0.2, true);
        assert!((q - 0.359).abs() < 0.005, "{q}");
        // Never below the largest appliance, never above the total.
        let two = simultaneous(0.25, 2, 0.2, true);
        assert!((0.2..0.25).contains(&two), "{two}");
        assert_eq!(simultaneous(0.21, 2, 0.2, true), 0.2);
        assert!(simultaneous(0.5, 4, 0.2, false) <= 0.5);
        // 0,33 l/s in multilayer: 20×2 gives 1,6 m/s.
        let s = size_for(PipeMaterial::Multilayer, 0.33, 0.020);
        assert_eq!(s.0, 0.020);
        // 1 l/s needs 32×3 (26 mm inside, 1,9 m/s).
        assert_eq!(size_for(PipeMaterial::Multilayer, 1.0, 0.020).0, 0.032);
        // Darcy: 0,3 l/s in 16 mm inside (1,5 m/s), 10 m: f ≈ 0,025, some
        // 1,7 m of water and 2,3 with the fittings, 23 kPa.
        let p = pressure_loss(0.3, 0.016, 10.0, PipeMaterial::Multilayer, false);
        assert!(p > 20.0 && p < 26.0, "{p}");
    }

    /// A bathroom on the ground floor and another upstairs, fed by a riser
    /// from a meter: the flows add up, the upper taps get less pressure.
    #[test]
    fn two_bathrooms_on_a_riser() {
        let mut m = archisim_library::new_project("Casa");
        let l0 = m.levels()[0].id;
        let l1 = m.add_level("Planta 1", 3.0);
        let wc = archisim_library::add_object_entry(&mut m, "Inodoro").unwrap();
        let basin = archisim_library::add_object_entry(&mut m, "Lavabo 60").unwrap();
        let mut objects = Vec::new();
        for l in [l0, l1] {
            for (t, x) in [(wc, 2.0), (basin, 4.0)] {
                objects.push(
                    m.add_object(NewObject {
                        level: l,
                        object_type: t,
                        position: Point2::new(x, 0.5),
                        rotation: 0.0,
                        base_offset: 0.0,
                    })
                    .unwrap(),
                );
            }
        }
        let points: Vec<_> = m.appliances_on(l0).iter().map(|a| a.point).collect();
        assert_eq!(points.len(), 2);
        let meter = m
            .add_device(NewDevice {
                level: l0,
                kind: DeviceKind::WaterMeter,
                name: "Contador".into(),
                position: Point2::new(0.0, 0.0),
                rotation: 0.0,
                elevation: 0.3,
                width: 0.3,
                depth: 0.2,
                height: 0.4,
            })
            .unwrap();
        // A riser at (1, 0) from the ground floor up.
        m.add_device(NewDevice {
            level: l0,
            kind: DeviceKind::Riser,
            name: "Montante".into(),
            position: Point2::new(1.0, 0.0),
            rotation: 0.0,
            elevation: 0.0,
            width: 0.05,
            depth: 0.05,
            height: 3.0,
        })
        .unwrap();
        let run = |m: &mut Model, level, path: Vec<Point2>, w: f64| {
            m.add_run(NewRun {
                level,
                system: ServiceSystem::ColdWater,
                path,
                elevation: 0.3,
                width: w,
                height: w,
                slope: 0.0,
            })
            .unwrap()
        };
        // Meter → riser → along to the appliances, on each floor.
        run(
            &mut m,
            l0,
            vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
            0.016,
        );
        for l in [l0, l1] {
            let ps: Vec<Point2> = m.appliances_on(l).iter().map(|a| a.point).collect();
            let main = run(
                &mut m,
                l,
                vec![Point2::new(1.0, 0.0), Point2::new(5.0, 0.0)],
                0.016,
            );
            let _ = main;
            for p in ps {
                run(&mut m, l, vec![Point2::new(p.x, 0.0), p], 0.016);
            }
        }
        let w = water(&m);
        assert_eq!(w.networks.len(), 1);
        let n = &w.networks[0];
        assert_eq!(n.source, Source::Device(meter, DeviceKind::WaterMeter));
        assert!(n.loose.is_empty(), "{:?}", n.loose);
        assert_eq!(n.taps.len(), 4, "{:#?}", n.stretches);
        // The first stretch carries all four.
        assert_eq!(n.stretches[0].appliances, 4);
        assert!((n.stretches[0].total_flow - 0.4).abs() < 1e-9);
        // A riser carries the two upstairs.
        let riser = n.stretches.iter().find(|s| s.riser.is_some()).unwrap();
        assert_eq!(riser.appliances, 2);
        // Upstairs gets less pressure than downstairs (3 m of height).
        let p = |l| {
            n.taps
                .iter()
                .filter(|t| t.level == l)
                .map(|t| t.pressure)
                .fold(f64::INFINITY, f64::min)
        };
        assert!(p(l0) - p(l1) > 25.0, "{} {}", p(l0), p(l1));
        // The common pipes are made 20 mm at least.
        assert!(n.stretches[0].sized.0 >= 0.020);
        let text = report(&m, &w);
        assert!(text.contains("F1"), "{text}");
        let _ = objects;
    }
}
