//! The natural ventilation of a garage (DB-HS 3, 3.1.4.1), from the
//! drawing: the openings of its walls to the outside (windows, openings
//! without a door, doors named «rejilla» or «celosía») and the grilles and
//! air inlets drawn on its edge.
//!
//! - **Mixed openings in two opposite zones of the façade**: the openings
//!   are put in zones by the way they face; each of two zones facing
//!   opposite ways must give the whole effective area of table 4.1,
//!   8·qv cm² (qv: 120 l/s a space).
//! - **No point farther than 25 m from an opening** (on a grid of the
//!   room, in a straight line: the obstacles are not seen).
//! - **Opposite openings more than 30 m apart** need another one halfway
//!   (5 % either way).
//! - **Small garages** (five spaces and 100 m² at most) may instead have
//!   their admission low and their extraction high on one wall, 1,5 m
//!   apart in height (between the top of the low one and the bottom of the
//!   high one), each of 4·qv cm².
//!
//! **Written from memory, to be checked** («para revisar»): the figures
//! below.

use archisim_core::{DeviceId, DeviceKind, Model, OpeningId, OpeningKind, Point2, Room, Vec2};

/// Para revisar. Table 4.1: a mixed opening's effective area, cm² per l/s.
pub const MIXED_PER_LS: f64 = 8.0;
/// Para revisar. 3.1.4.1: the farthest any point may be from an opening, m.
pub const REACH: f64 = 25.0;
/// Para revisar. 3.1.4.1: opposite openings farther apart than this need
/// one more halfway, m, with this tolerance.
pub const OPPOSITE_GAP: f64 = 30.0;
pub const HALFWAY_TOLERANCE: f64 = 0.05;
/// Para revisar. 3.1.4.1: a small garage (spaces, m²) and the height
/// between its low and high openings, m.
pub const SMALL_SPACES: usize = 5;
pub const SMALL_AREA: f64 = 100.0;
pub const SMALL_RISE: f64 = 1.5;
/// Table 4.1: admission and extraction openings, cm² per l/s.
pub const SPLIT_PER_LS: f64 = 4.0;
/// Two openings face the same way above this cosine, opposite below its
/// negative.
const SAME_WAY: f64 = 0.7;
/// The grid the farthest point is looked for on, m.
const GRID: f64 = 0.5;

/// What makes an opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    Opening(OpeningId),
    Device(DeviceId),
}

/// An opening to the outside.
#[derive(Debug, Clone, PartialEq)]
pub struct GarageOpening {
    pub source: Source,
    pub name: String,
    /// Its centre in plan and the way it faces (out of the garage).
    pub at: Point2,
    pub facing: Vec2,
    /// Its effective area, cm², and its bottom and top over the level, m.
    pub area: f64,
    pub bottom: f64,
    pub top: f64,
    /// The zone of the façade it is in (index in `Natural::zones`).
    pub zone: usize,
}

/// A zone of the façade: the openings facing one way.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// «norte», «sureste»… (the plan's axes, +y north).
    pub name: String,
    pub facing: Vec2,
    pub area: f64,
    pub openings: usize,
}

/// The natural ventilation of a garage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Natural {
    pub openings: Vec<GarageOpening>,
    pub zones: Vec<Zone>,
    /// The area each opposite zone must give, cm² (8·qv).
    pub needed: f64,
    /// The two opposite zones that give the most (by index), if any face
    /// opposite ways.
    pub opposite: Option<(usize, usize)>,
    /// The farthest point of the garage from any opening, and how far, m.
    pub farthest: f64,
    pub farthest_at: Option<Point2>,
    /// The nearest opposite openings, how far apart, and whether one more
    /// is halfway when needed.
    pub opposite_gap: f64,
    pub halfway: bool,
    /// A small garage: allowed openings on one wall, and whether they make
    /// it (low admission and high extraction, 1,5 m apart, 4·qv each).
    pub small: bool,
    pub one_wall: bool,
}

impl Natural {
    /// Each of two opposite zones gives the whole area.
    pub fn zones_pass(&self) -> bool {
        self.opposite.is_some_and(|(a, b)| {
            self.zones[a].area + 0.5 >= self.needed && self.zones[b].area + 0.5 >= self.needed
        })
    }

    pub fn reach_passes(&self) -> bool {
        self.farthest <= REACH + 1e-6
    }

    pub fn gap_passes(&self) -> bool {
        self.opposite_gap <= OPPOSITE_GAP * (1.0 + HALFWAY_TOLERANCE) + 1e-6 || self.halfway
    }

    pub fn passes(&self) -> bool {
        !self.openings.is_empty()
            && (self.zones_pass() || self.one_wall)
            && self.reach_passes()
            && (self.one_wall || self.gap_passes())
    }

    /// The whole effective area, cm².
    pub fn total(&self) -> f64 {
        self.openings.iter().map(|o| o.area).sum()
    }
}

fn compass(v: Vec2) -> &'static str {
    const NAMES: [&str; 8] = [
        "este", "noreste", "norte", "noroeste", "oeste", "suroeste", "sur", "sureste",
    ];
    let a = v.y.atan2(v.x).rem_euclid(std::f64::consts::TAU);
    let k = ((a / (std::f64::consts::TAU / 8.0)).round() as usize) % 8;
    NAMES[k]
}

fn to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// The edge of the room nearest a point, its distance and its outward
/// normal.
fn nearest_edge(r: &Room, p: Point2) -> Option<(f64, Vec2)> {
    let n = r.outline.len();
    let mut best: Option<(f64, Vec2)> = None;
    for i in 0..n {
        let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
        let d = to_segment(p, a, b);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let mut nrm = Vec2::new(u.y, -u.x);
        let mid = a + (b - a) * 0.5;
        if r.contains(mid + nrm * 0.05) {
            nrm = nrm * -1.0;
        }
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, nrm));
        }
    }
    best
}

/// The openings of a garage to the outside.
pub fn openings(model: &Model, r: &Room) -> Vec<GarageOpening> {
    let mut out = Vec::new();
    for o in model.openings() {
        let (Some(t), Some(wall)) = (model.opening_type(o.opening_type), model.wall(o.wall)) else {
            continue;
        };
        if wall.level != r.level {
            continue;
        }
        let name = t.name.to_lowercase();
        let louvred = [
            "rejilla",
            "celosía",
            "celosia",
            "ventilación",
            "ventilacion",
        ]
        .iter()
        .any(|w| name.contains(w));
        let counts = match t.kind {
            OpeningKind::Void | OpeningKind::Window(_) => true,
            _ => louvred,
        };
        if !counts {
            continue;
        }
        let Some(g) = archisim_geom::opening_geometry(model, o.id) else {
            continue;
        };
        let tm = (g.t0 + g.t1) / 2.0;
        let sign = if g.exterior_offset >= g.interior_offset {
            1.0
        } else {
            -1.0
        };
        let a = g.point(tm, g.exterior_offset + sign * 0.25);
        let b = g.point(tm, g.interior_offset - sign * 0.25);
        let in_any = |p: Point2| model.rooms_on_level(r.level).any(|x| x.contains(p));
        let (inside, outside) = if r.contains(a) && !in_any(b) {
            (a, b)
        } else if r.contains(b) && !in_any(a) {
            (b, a)
        } else {
            continue;
        };
        let w = (g.t1 - g.t0).max(0.0);
        let h = (g.head - g.sill).max(0.0);
        let area = match t.kind {
            OpeningKind::Window(_) => {
                (w - 2.0 * g.frame_width).max(0.0) * (h - 2.0 * g.frame_width).max(0.0)
            }
            _ => w * h,
        } * 1e4;
        out.push(GarageOpening {
            source: Source::Opening(o.id),
            name: t.name.clone(),
            at: g.point(tm, (g.exterior_offset + g.interior_offset) / 2.0),
            facing: (outside - inside).normalized().unwrap_or_default(),
            area,
            bottom: g.sill,
            top: g.head,
            zone: 0,
        });
    }
    for d in model.devices().filter(|d| d.level == r.level) {
        if !matches!(d.kind, DeviceKind::Grille | DeviceKind::AirInlet) {
            continue;
        }
        let Some((dist, facing)) = nearest_edge(r, d.position) else {
            continue;
        };
        if dist > crate::ventilation::ROOM_REACH {
            continue;
        }
        let area = match d.kind {
            DeviceKind::Grille => d.width * d.height * 1e4,
            _ => crate::ventilation::device_area(d.kind, d.width, d.depth, d.height),
        };
        out.push(GarageOpening {
            source: Source::Device(d.id),
            name: d.name.clone(),
            at: d.position,
            facing,
            area,
            bottom: d.elevation,
            top: d.elevation + d.height,
            zone: 0,
        });
    }
    out
}

/// The natural ventilation of a garage of `spaces` spaces and `flow` l/s.
pub fn natural(model: &Model, r: &Room, spaces: usize, flow: f64) -> Natural {
    let mut openings = openings(model, r);
    let mut zones: Vec<Zone> = Vec::new();
    for o in &mut openings {
        let k = match zones.iter().position(|z| z.facing.dot(o.facing) > SAME_WAY) {
            Some(k) => k,
            None => {
                zones.push(Zone {
                    name: compass(o.facing).into(),
                    facing: o.facing,
                    area: 0.0,
                    openings: 0,
                });
                zones.len() - 1
            }
        };
        o.zone = k;
        zones[k].area += o.area;
        zones[k].openings += 1;
    }
    // The opposite pair whose smaller area is the largest.
    let mut opposite: Option<(usize, usize)> = None;
    for a in 0..zones.len() {
        for b in a + 1..zones.len() {
            if zones[a].facing.dot(zones[b].facing) < -SAME_WAY {
                let least = zones[a].area.min(zones[b].area);
                if opposite.is_none_or(|(x, y)| zones[x].area.min(zones[y].area) < least) {
                    opposite = Some((a, b));
                }
            }
        }
    }
    // The farthest point, on a grid of the room.
    let (mut farthest, mut farthest_at) = (0.0f64, None);
    if !openings.is_empty() && !r.outline.is_empty() {
        let (mut lo, mut hi) = (r.outline[0], r.outline[0]);
        for p in &r.outline {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        let mut y = lo.y + GRID / 2.0;
        while y < hi.y {
            let mut x = lo.x + GRID / 2.0;
            while x < hi.x {
                let p = Point2::new(x, y);
                if r.contains(p) {
                    let d = openings
                        .iter()
                        .map(|o| o.at.distance(p))
                        .fold(f64::MAX, f64::min);
                    if d > farthest {
                        farthest = d;
                        farthest_at = Some(p);
                    }
                }
                x += GRID;
            }
            y += GRID;
        }
    }
    // The nearest opposite openings and one halfway.
    let (mut opposite_gap, mut halfway) = (0.0, false);
    if let Some((za, zb)) = opposite {
        let mut best: Option<(usize, usize, f64)> = None;
        for (i, a) in openings.iter().enumerate().filter(|(_, o)| o.zone == za) {
            for (j, b) in openings.iter().enumerate().filter(|(_, o)| o.zone == zb) {
                let d = a.at.distance(b.at);
                if best.is_none_or(|(_, _, bd)| d < bd) {
                    best = Some((i, j, d));
                }
            }
        }
        if let Some((i, j, d)) = best {
            opposite_gap = d;
            halfway = openings.iter().enumerate().any(|(k, c)| {
                k != i
                    && k != j
                    && (c.at.distance(openings[i].at) - c.at.distance(openings[j].at)).abs()
                        <= HALFWAY_TOLERANCE * d
            });
        }
    }
    let area = r.area();
    let small = spaces <= SMALL_SPACES && area <= SMALL_AREA + 1e-6;
    let one_wall = small
        && zones.iter().enumerate().any(|(k, _)| {
            let here: Vec<&GarageOpening> = openings.iter().filter(|o| o.zone == k).collect();
            let low_top = here.iter().map(|o| o.top).fold(f64::MAX, f64::min);
            let lows: f64 = here
                .iter()
                .filter(|o| (o.top - low_top).abs() < 1e-6)
                .map(|o| o.area)
                .sum();
            let highs: f64 = here
                .iter()
                .filter(|o| o.bottom >= low_top + SMALL_RISE - 1e-6)
                .map(|o| o.area)
                .sum();
            lows + 0.5 >= SPLIT_PER_LS * flow && highs + 0.5 >= SPLIT_PER_LS * flow
        });
    Natural {
        openings,
        zones,
        needed: MIXED_PER_LS * flow,
        opposite,
        farthest,
        farthest_at,
        opposite_gap,
        halfway,
        small,
        one_wall,
    }
}

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

/// The natural ventilation, line by line.
pub fn lines(n: &Natural) -> Vec<String> {
    let no = |b: bool| if b { "" } else { " — NO CUMPLE" };
    let mut out = Vec::new();
    if n.openings.is_empty() {
        out.push("ventilación natural: no hay aberturas al exterior dibujadas en sus muros (ventanas, huecos o rejillas)".into());
        return out;
    }
    let zones: Vec<String> = n
        .zones
        .iter()
        .map(|z| {
            format!(
                "{} {} cm² ({} abertura{})",
                z.name,
                dec(z.area, 0),
                z.openings,
                if z.openings == 1 { "" } else { "s" }
            )
        })
        .collect();
    out.push(format!(
        "ventilación natural: {} aberturas, {} cm² en total; por fachadas: {}",
        n.openings.len(),
        dec(n.total(), 0),
        zones.join(", ")
    ));
    match n.opposite {
        Some((a, b)) => out.push(format!(
            "aberturas mixtas en dos zonas opuestas ({} y {}): {} y {} cm², cada una ≥ {} cm² (8·qv, tabla 4.1){}",
            n.zones[a].name,
            n.zones[b].name,
            dec(n.zones[a].area, 0),
            dec(n.zones[b].area, 0),
            dec(n.needed, 0),
            no(n.zones_pass() || n.one_wall)
        )),
        None => out.push(format!(
            "no hay aberturas en dos zonas opuestas de la fachada (3.1.4.1){}",
            if n.one_wall {
                ": vale la admisión abajo y la extracción arriba en un muro, garaje pequeño"
            } else {
                " — NO CUMPLE"
            }
        )),
    }
    out.push(format!(
        "el punto más alejado está a {} m de la abertura más cercana (≤ {} m, en línea recta){}",
        dec(n.farthest, 1),
        dec(REACH, 0),
        no(n.reach_passes())
    ));
    if n.opposite.is_some() && n.opposite_gap > OPPOSITE_GAP {
        out.push(format!(
            "aberturas opuestas más cercanas a {} m (> {} m): otra equidistante de ambas{}",
            dec(n.opposite_gap, 1),
            dec(OPPOSITE_GAP, 0),
            no(n.halfway)
        ));
    }
    if !n.passes() {
        out.push("con ventilación natural no cumple: necesita ventilación mecánica (3.1.4.2), con bocas de extracción y su extractor".into());
    }
    out
}
