//! The envelope by the DB-HE 1 (2019) and the heating and cooling loads
//! of every room, with the equipment that covers them.
//!
//! - **The envelope** of each room comes from its outline. Each edge
//!   whose other side, 60 cm out, is in no room of the level is outside:
//!   its wall (the nearest one along it), the windows and doors in that
//!   wall within the edge, or a curtain wall. Above it, the floor of the
//!   level above or a roof, unless a room of that level is over it; below
//!   it, its slab on the ground floor (against the ground). Edges and
//!   floors against another room (heated or not), interior walls without
//!   windows (to a stairwell or a shaft; one with a window is on a light
//!   well) and party walls are left out.
//! - **Transmittance** U = 1/(Rsi + Σ e/λ + Rse) with the layers of the
//!   wall, slab or roof type and the conductivity of each material
//!   ([`Model::conductivity`]); windows and doors by the figures of the
//!   project (`ThermalSite::window_u`, `door_u`).
//! - **DB-HE 1** (table 3.1.1.a-HE1): every element against its limit by
//!   the winter zone, and the global coefficient K of the heated rooms
//!   (table 3.1.1.b-HE1, for new dwellings) by the compactness V/A.
//! - **Heating load** of each room: U·A·ΔT of what is outside (half the
//!   difference against the ground) and the ventilation, 1,2·q·ΔT with the
//!   inflow of DB-HS 3 (or half a renewal an hour).
//! - **Cooling load**: the same transmission with the summer difference,
//!   the sun through the glass (by orientation), people (75 W sensible
//!   and 55 W latent each), light and appliances, and the ventilation.
//! - **Equipment**: the indoor unit of each dry room and the heat pump of
//!   each dwelling (or room) from the usual sizes; the radiators drawn,
//!   against the heating load of their room.
//!
//! **To be checked by Antonio**: the limits of DB-HE 1, the default
//! conductivities, the solar gains and the internal gains are written as
//! remembered (they are together below).

use std::collections::BTreeMap;

use archisim_core::{
    point_in_polygon, DeviceId, DeviceKind, LevelId, Model, OpeningKind, Point2, RoomId,
    ThermalSite, Vec2,
};

/// Surface resistances, m²K/W (DB-HE, documento de apoyo 1): inside a
/// wall, outside; inside a roof (heat going up), inside a floor (going
/// down).
pub const RSI_WALL: f64 = 0.13;
pub const RSE: f64 = 0.04;
pub const RSI_UP: f64 = 0.10;
pub const RSI_DOWN: f64 = 0.17;

/// Table 3.1.1.a-HE1: limits of transmittance, W/m²K, by winter zone
/// (α, A, B, C, D, E).
pub const WALL_LIMIT: [f64; 6] = [0.80, 0.70, 0.56, 0.49, 0.41, 0.37];
pub const ROOF_LIMIT: [f64; 6] = [0.55, 0.50, 0.44, 0.40, 0.35, 0.33];
pub const GROUND_LIMIT: [f64; 6] = [0.90, 0.80, 0.75, 0.70, 0.65, 0.59];
pub const OPENING_LIMIT: [f64; 6] = [3.2, 2.7, 2.3, 2.1, 1.8, 1.8];
pub const DOOR_LIMIT: f64 = 5.7;
/// Table 3.1.1.b-HE1, new dwellings: K at V/A ≤ 1 and at V/A ≥ 4, W/m²K.
pub const K_COMPACT_LOW: [f64; 6] = [0.67, 0.60, 0.58, 0.53, 0.48, 0.43];
pub const K_COMPACT_HIGH: [f64; 6] = [0.86, 0.80, 0.77, 0.72, 0.67, 0.62];

/// Peak sun through glass by orientation, W/m² (summer): north, east,
/// south, west, and a roof light.
pub const SUN_N: f64 = 100.0;
pub const SUN_E: f64 = 450.0;
pub const SUN_S: f64 = 350.0;
pub const SUN_W: f64 = 500.0;
pub const SUN_UP: f64 = 750.0;
/// People, W each: sensible and latent.
pub const PERSON_SENSIBLE: f64 = 75.0;
pub const PERSON_LATENT: f64 = 55.0;
/// Light and appliances, W/m²; a kitchen's appliances.
pub const LIGHT_W_M2: f64 = 5.0;
pub const APPLIANCES_W_M2: f64 = 3.0;
pub const KITCHEN_W_M2: f64 = 20.0;
/// What a radiator gives, W per m² of its front (aluminium, ΔT 50).
pub const RADIATOR_W_M2: f64 = 2000.0;
/// Usual sizes, kW: indoor units and heat pumps.
pub const INDOOR_UNITS: [f64; 7] = [2.0, 2.5, 3.5, 4.2, 5.0, 6.0, 7.1];
pub const HEAT_PUMPS: [f64; 10] = [3.5, 5.0, 6.0, 7.1, 8.0, 10.0, 12.0, 14.0, 16.0, 20.0];

/// What a piece of envelope is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Wall,
    Window,
    Door,
    /// A curtain wall: glass.
    Curtain,
    Roof,
    /// The floor of a room under the open air (over a porch).
    ExposedFloor,
    Ground,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Wall => "muro de fachada",
            Kind::Window => "ventana",
            Kind::Door => "puerta exterior",
            Kind::Curtain => "muro cortina",
            Kind::Roof => "cubierta",
            Kind::ExposedFloor => "suelo sobre el exterior",
            Kind::Ground => "suelo sobre el terreno",
        }
    }

    pub fn glazed(self) -> bool {
        matches!(self, Kind::Window | Kind::Curtain)
    }

    /// Its limit in DB-HE 1 for a winter zone (by its column).
    pub fn limit(self, zone: usize) -> f64 {
        match self {
            Kind::Wall | Kind::ExposedFloor => WALL_LIMIT[zone],
            Kind::Roof => ROOF_LIMIT[zone],
            Kind::Ground => GROUND_LIMIT[zone],
            Kind::Window | Kind::Curtain => OPENING_LIMIT[zone],
            Kind::Door => DOOR_LIMIT,
        }
    }
}

/// The four orientations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Facing {
    North,
    East,
    South,
    West,
    Up,
}

impl Facing {
    pub fn name(self) -> &'static str {
        match self {
            Facing::North => "N",
            Facing::East => "E",
            Facing::South => "S",
            Facing::West => "O",
            Facing::Up => "cenital",
        }
    }

    fn sun(self) -> f64 {
        match self {
            Facing::North => SUN_N,
            Facing::East => SUN_E,
            Facing::South => SUN_S,
            Facing::West => SUN_W,
            Facing::Up => SUN_UP,
        }
    }
}

/// A piece of a room's envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub kind: Kind,
    /// What it is made of: the wall, slab, roof or opening type.
    pub type_name: String,
    pub area: f64,
    pub u: f64,
    pub facing: Facing,
}

/// A room's loads.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomLoad {
    pub room: RoomId,
    pub name: String,
    pub level: LevelId,
    pub area: f64,
    pub volume: f64,
    pub pieces: Vec<Piece>,
    /// Ventilation inflow, l/s.
    pub ventilation: f64,
    /// Heating: transmission, ventilation and total, W.
    pub heat_transmission: f64,
    pub heat_ventilation: f64,
    pub heating: f64,
    /// Cooling: transmission, sun, internal, ventilation, latent, total, W.
    pub cool_transmission: f64,
    pub cool_sun: f64,
    pub cool_internal: f64,
    pub cool_ventilation: f64,
    pub cool_latent: f64,
    pub cooling: f64,
    /// Whether it is lived in (a dry room): it gets an indoor unit.
    pub dry: bool,
    /// The indoor unit that covers it, kW.
    pub unit: Option<f64>,
    /// The radiators drawn in it and what they give, W.
    pub radiators: Vec<DeviceId>,
    pub radiator_power: f64,
}

impl RoomLoad {
    /// Radiators drawn that do not reach its heating load.
    pub fn radiators_short(&self) -> bool {
        !self.radiators.is_empty() && self.radiator_power + 1.0 < self.heating
    }
}

/// A group of rooms on one machine: a dwelling, or a room on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub rooms: Vec<RoomId>,
    pub heating: f64,
    pub cooling: f64,
    /// The heat pump, kW.
    pub heat_pump: f64,
}

/// A type of the envelope against its DB-HE 1 limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub type_name: String,
    pub u: f64,
    pub limit: f64,
    pub area: f64,
}

impl Element {
    pub fn fails(&self) -> bool {
        self.u > self.limit + 1e-6
    }
}

/// The whole calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct Thermal {
    pub site: ThermalSite,
    pub rooms: Vec<RoomLoad>,
    pub groups: Vec<Group>,
    pub elements: Vec<Element>,
    /// Σ U·A / Σ A of the envelope, the heated volume, the envelope's area
    /// and the compactness V/A.
    pub k: f64,
    pub volume: f64,
    pub envelope: f64,
    pub compactness: f64,
    pub k_limit: f64,
}

impl Thermal {
    pub fn heating(&self) -> f64 {
        self.rooms.iter().map(|r| r.heating).sum()
    }

    pub fn cooling(&self) -> f64 {
        self.rooms.iter().map(|r| r.cooling).sum()
    }

    pub fn k_fails(&self) -> bool {
        self.envelope > 0.0 && self.k > self.k_limit + 1e-6
    }

    pub fn fails(&self) -> usize {
        self.elements.iter().filter(|e| e.fails()).count()
            + self.k_fails() as usize
            + self.rooms.iter().filter(|r| r.radiators_short()).count()
    }
}

/// The transmittance of some layers between two surface resistances.
pub fn u_value(model: &Model, layers: &[archisim_core::Layer], rsi: f64, rse: f64) -> f64 {
    1.0 / (rsi + model.layers_resistance(layers) + rse)
}

/// K's limit for new dwellings by compactness (straight between V/A 1
/// and 4).
pub fn k_limit(zone: usize, compactness: f64) -> f64 {
    let t = ((compactness - 1.0) / 3.0).clamp(0.0, 1.0);
    K_COMPACT_LOW[zone] + (K_COMPACT_HIGH[zone] - K_COMPACT_LOW[zone]) * t
}

fn pick(sizes: &[f64], kw: f64) -> f64 {
    sizes
        .iter()
        .copied()
        .find(|s| *s * 1000.0 >= kw - 1e-6)
        .unwrap_or(sizes[sizes.len() - 1])
}

/// The orientation of an outward normal in plan.
fn facing(model: &Model, n: Vec2) -> Facing {
    // A bearing in plan (clockwise from up) less true north is from north.
    let plan = n.x.atan2(n.y);
    let az = (plan - model.true_north())
        .rem_euclid(std::f64::consts::TAU)
        .to_degrees();
    match az {
        a if !(45.0..315.0).contains(&a) => Facing::North,
        a if a < 135.0 => Facing::East,
        a if a < 225.0 => Facing::South,
        _ => Facing::West,
    }
}

fn segment_distance(p: Point2, a: Point2, b: Point2) -> (f64, f64) {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    ((a + ab * t).distance(p), t)
}

fn level_after(model: &Model, level: LevelId) -> Option<(LevelId, f64)> {
    let levels = model.levels();
    let i = levels.iter().position(|l| l.id == level)?;
    levels.get(i + 1).map(|l| (l.id, l.elevation))
}

/// Works out the envelope and the loads.
pub fn thermal(model: &Model) -> Thermal {
    let site = model.thermal_site().clone();
    let zone = site.winter.index();
    let dt_heat = (site.heated - site.winter_air).max(0.0);
    let dt_cool = (site.summer_air - site.cooled).max(0.0);
    // The ventilation of DB-HS 3, and its dwellings.
    let vent = crate::ventilation::ventilation(model);
    let mut inflow: BTreeMap<RoomId, f64> = BTreeMap::new();
    let mut dwelling_of: BTreeMap<RoomId, usize> = BTreeMap::new();
    for (i, d) in vent.dwellings.iter().enumerate() {
        for r in &d.rooms {
            inflow.insert(r.room, r.admission);
            dwelling_of.insert(r.room, i);
        }
    }
    let levels = model.levels();
    let first = levels.first().map(|l| l.id);
    let mut rooms = Vec::new();
    let mut by_type: BTreeMap<(Kind, String), (f64, f64)> = BTreeMap::new();
    for level in &levels {
        let above = level_after(model, level.id);
        let storey = above.map_or(3.0, |(_, z)| z - level.elevation);
        for room in model.rooms_on_level(level.id) {
            let class = crate::ventilation::Class::of(&room.usage, &room.name);
            if class.common && !class.dry_room() && !class.wet {
                // Corridors, stairs, garages and plant rooms: not
                // conditioned.
                continue;
            }
            let area = room.area();
            let h = room.height.unwrap_or((storey - 0.3).max(2.4));
            let mut pieces: Vec<Piece> = Vec::new();
            let n = room.outline.len();
            for i in 0..n {
                let (a, b) = (room.outline[i], room.outline[(i + 1) % n]);
                let len = a.distance(b);
                if len < 0.05 {
                    continue;
                }
                let d = (b - a) * (1.0 / len);
                let out = Vec2::new(d.y, -d.x);
                let mid = (a + b) * 0.5;
                // The part of the edge with no room beyond it, by probes
                // every 25 cm (a partition between two rooms beyond would
                // fool a single one).
                let probes = ((len / 0.25).ceil() as usize).max(2);
                let outside = (0..probes)
                    .filter(|k| {
                        let p = a + d * (len * (*k as f64 + 0.5) / probes as f64) + out * 0.6;
                        !model
                            .rooms_on_level(level.id)
                            .any(|r| r.id != room.id && r.contains(p))
                    })
                    .count();
                if outside * 3 < probes {
                    continue;
                }
                let len = len * outside as f64 / probes as f64;
                let face = facing(model, out);
                // A curtain wall along it?
                let curtain = model
                    .curtain_walls()
                    .find(|c| c.level == level.id && segment_distance(mid, c.start, c.end).0 < 0.6);
                if curtain.is_some() {
                    pieces.push(Piece {
                        kind: Kind::Curtain,
                        type_name: "Muro cortina".into(),
                        area: len * h,
                        u: site.window_u,
                        facing: face,
                    });
                    continue;
                }
                let wall = model
                    .walls_on_level(level.id)
                    .filter_map(|w| {
                        let t = model.wall_type(w.wall_type)?;
                        let (dist, _) = segment_distance(mid, w.start, w.end);
                        let along = (w.end - w.start).normalized()?;
                        (dist <= t.thickness() + 0.35 && along.cross(d).abs() < 0.2)
                            .then_some((w, t, dist))
                    })
                    .min_by(|x, y| x.2.total_cmp(&y.2));
                let Some((wall, wtype, _)) = wall else {
                    continue;
                };
                // A partition to a space with no room (a stairwell, a
                // shaft) or a party wall against the neighbour: not the
                // envelope to the outside.
                let has_window = model.openings().any(|o| {
                    o.wall == wall.id
                        && model
                            .opening_type(o.opening_type)
                            .is_some_and(|t| matches!(t.kind, OpeningKind::Window(_)))
                });
                if (wtype.function == archisim_core::WallFunction::Interior && !has_window)
                    || wtype.name.to_lowercase().contains("medianer")
                {
                    continue;
                }
                let mut opaque = len * h;
                let wdir = (wall.end - wall.start).normalized().unwrap_or(d);
                for o in model.openings().filter(|o| o.wall == wall.id) {
                    let Some(t) = model.opening_type(o.opening_type) else {
                        continue;
                    };
                    let at = wall.start + wdir * o.along;
                    let (dist, s) = segment_distance(at, a, b);
                    if dist > 0.8 || s <= 0.0 || s >= 1.0 {
                        continue;
                    }
                    let kind = match t.kind {
                        OpeningKind::Window(_) => Kind::Window,
                        OpeningKind::Door(_) => Kind::Door,
                        OpeningKind::Void => continue,
                    };
                    let oa = (t.width * t.height).min(opaque);
                    opaque -= oa;
                    pieces.push(Piece {
                        kind,
                        type_name: t.name.clone(),
                        area: oa,
                        u: if kind == Kind::Window {
                            site.window_u
                        } else {
                            site.door_u
                        },
                        facing: face,
                    });
                }
                pieces.push(Piece {
                    kind: Kind::Wall,
                    type_name: wtype.name.clone(),
                    area: opaque.max(0.0),
                    u: u_value(model, &wtype.layers, RSI_WALL, RSE),
                    facing: face,
                });
            }
            // Above: a room over it is inside; otherwise the slab of the
            // level above, or a roof.
            let p = room.label;
            if let Some((up, _)) = above {
                let covered = model.rooms_on_level(up).any(|r| r.contains(p));
                if !covered {
                    let slab = model
                        .slabs_on_level(up)
                        .find(|s| point_in_polygon(p, &s.outline))
                        .and_then(|s| model.slab_type(s.slab_type));
                    let roof = model
                        .roofs()
                        .find(|r| point_in_polygon(p, &r.outline))
                        .and_then(|r| model.slab_type(r.roof_type));
                    if let Some(t) = slab.or(roof) {
                        pieces.push(Piece {
                            kind: Kind::Roof,
                            type_name: t.name.clone(),
                            area,
                            u: u_value(model, &t.layers, RSI_UP, RSE),
                            facing: Facing::Up,
                        });
                    }
                }
            } else if let Some(t) = model
                .roofs()
                .find(|r| point_in_polygon(p, &r.outline))
                .and_then(|r| model.slab_type(r.roof_type))
            {
                pieces.push(Piece {
                    kind: Kind::Roof,
                    type_name: t.name.clone(),
                    area,
                    u: u_value(model, &t.layers, RSI_UP, RSE),
                    facing: Facing::Up,
                });
            }
            // Below: the ground floor's slab, or a floor over nothing.
            let own = model
                .slabs_on_level(level.id)
                .find(|s| point_in_polygon(p, &s.outline))
                .and_then(|s| model.slab_type(s.slab_type));
            if let Some(t) = own {
                if Some(level.id) == first {
                    pieces.push(Piece {
                        kind: Kind::Ground,
                        type_name: t.name.clone(),
                        area,
                        u: u_value(model, &t.layers, RSI_DOWN, 0.0),
                        facing: Facing::Up,
                    });
                } else {
                    let below = levels
                        .iter()
                        .rev()
                        .find(|l| l.elevation < level.elevation - 0.1)
                        .map(|l| l.id);
                    let open = below.is_some_and(|b| {
                        !model.rooms_on_level(b).any(|r| r.contains(p))
                            && !model
                                .slabs_on_level(b)
                                .any(|s| point_in_polygon(p, &s.outline))
                    });
                    if open {
                        pieces.push(Piece {
                            kind: Kind::ExposedFloor,
                            type_name: t.name.clone(),
                            area,
                            u: u_value(model, &t.layers, RSI_DOWN, RSE),
                            facing: Facing::Up,
                        });
                    }
                }
            }
            for pc in &pieces {
                let e = by_type
                    .entry((pc.kind, pc.type_name.clone()))
                    .or_insert((pc.u, 0.0));
                e.1 += pc.area;
            }
            // Loads.
            let volume = area * h;
            // Lived in: a dry room of a dwelling, or any other room that is
            // not wet nor a passage (a shop, an office).
            let dry = class.dry_room() || (!class.wet && !class.passage && !class.common);
            let q = inflow
                .get(&room.id)
                .copied()
                .filter(|q| *q > 0.0)
                .unwrap_or(if dry { 0.5 * volume / 3.6 } else { 0.0 });
            let ua = |kinds: &dyn Fn(Kind) -> bool| -> f64 {
                pieces
                    .iter()
                    .filter(|p| kinds(p.kind))
                    .map(|p| p.u * p.area)
                    .sum()
            };
            let heat_transmission =
                ua(&|k| k != Kind::Ground) * dt_heat + ua(&|k| k == Kind::Ground) * dt_heat * 0.5;
            let heat_ventilation = 1.2 * q * dt_heat;
            let cool_transmission = ua(&|k| k != Kind::Ground) * dt_cool;
            let cool_sun: f64 = pieces
                .iter()
                .filter(|p| p.kind.glazed())
                .map(|p| p.area * site.glass_g * p.facing.sun())
                .sum();
            let people = if dry {
                (area / 10.0).max(1.0).round()
            } else {
                0.0
            };
            let cool_internal = people * PERSON_SENSIBLE
                + area
                    * (LIGHT_W_M2
                        + if class.kitchen {
                            KITCHEN_W_M2
                        } else {
                            APPLIANCES_W_M2
                        });
            let cool_latent = people * PERSON_LATENT;
            let cool_ventilation = 1.2 * q * dt_cool;
            let heating = heat_transmission + heat_ventilation;
            let cooling =
                cool_transmission + cool_sun + cool_internal + cool_ventilation + cool_latent;
            let mut radiators = Vec::new();
            let mut radiator_power = 0.0;
            for d in model.devices().filter(|d| {
                d.level == level.id && d.kind == DeviceKind::Radiator && room.contains(d.position)
            }) {
                radiators.push(d.id);
                radiator_power += d.width * d.height * RADIATOR_W_M2;
            }
            rooms.push(RoomLoad {
                room: room.id,
                name: room.name.clone(),
                level: level.id,
                area,
                volume,
                pieces,
                ventilation: q,
                heat_transmission,
                heat_ventilation,
                heating,
                cool_transmission,
                cool_sun,
                cool_internal,
                cool_ventilation,
                cool_latent,
                cooling,
                dry,
                unit: dry.then(|| pick(&INDOOR_UNITS, cooling.max(heating))),
                radiators,
                radiator_power,
            });
        }
    }
    // Groups: a dwelling, or a room on its own.
    let mut groups: Vec<Group> = Vec::new();
    let mut of_dwelling: BTreeMap<usize, usize> = BTreeMap::new();
    for r in &rooms {
        let g = match dwelling_of.get(&r.room) {
            Some(d) => *of_dwelling.entry(*d).or_insert_with(|| {
                groups.push(Group {
                    name: vent.dwellings[*d].name.clone(),
                    rooms: Vec::new(),
                    heating: 0.0,
                    cooling: 0.0,
                    heat_pump: 0.0,
                });
                groups.len() - 1
            }),
            None => {
                groups.push(Group {
                    name: r.name.clone(),
                    rooms: Vec::new(),
                    heating: 0.0,
                    cooling: 0.0,
                    heat_pump: 0.0,
                });
                groups.len() - 1
            }
        };
        groups[g].rooms.push(r.room);
        groups[g].heating += r.heating;
        groups[g].cooling += r.cooling;
    }
    for g in &mut groups {
        g.heat_pump = pick(&HEAT_PUMPS, g.heating.max(g.cooling));
    }
    // DB-HE 1.
    let elements: Vec<Element> = by_type
        .into_iter()
        .map(|((kind, type_name), (u, area))| Element {
            kind,
            type_name,
            u,
            limit: kind.limit(zone),
            area,
        })
        .collect();
    let envelope: f64 = elements.iter().map(|e| e.area).sum();
    let ua: f64 = elements.iter().map(|e| e.u * e.area).sum();
    let volume: f64 = rooms.iter().map(|r| r.volume).sum();
    let k = if envelope > 0.0 { ua / envelope } else { 0.0 };
    let compactness = if envelope > 0.0 {
        volume / envelope
    } else {
        0.0
    };
    Thermal {
        site,
        rooms,
        groups,
        elements,
        k,
        volume,
        envelope,
        compactness,
        k_limit: k_limit(zone, compactness),
    }
}

fn dec(v: f64, n: usize) -> String {
    let s = format!("{v:.n$}").replace('.', ",");
    // No «-0».
    if s.trim_start_matches('-')
        .chars()
        .all(|c| c == '0' || c == ',')
    {
        s.trim_start_matches('-').to_string()
    } else {
        s
    }
}

/// The calculation in words.
pub fn report(model: &Model, t: &Thermal) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let site = &t.site;
    let _ = writeln!(
        s,
        "ENVOLVENTE (DB-HE 1) Y CARGAS TÉRMICAS — zona climática {}; exterior {} °C en invierno y {} °C en verano; interior {} y {} °C",
        site.zone(),
        dec(site.winter_air, 1),
        dec(site.summer_air, 1),
        dec(site.heated, 0),
        dec(site.cooled, 0)
    );
    let _ = writeln!(s, "\nTransmitancias (tabla 3.1.1.a-HE1)");
    for e in &t.elements {
        let _ = writeln!(
            s,
            "  {} «{}»: U = {} W/m²K (≤ {}), {} m²{}",
            e.kind.name(),
            e.type_name,
            dec(e.u, 2),
            dec(e.limit, 2),
            dec(e.area, 1),
            if e.fails() { " — NO CUMPLE" } else { "" }
        );
    }
    let _ = writeln!(
        s,
        "  K global: {} W/m²K (≤ {} con V/A = {} m: {} m³ / {} m²; tabla 3.1.1.b-HE1){}",
        dec(t.k, 2),
        dec(t.k_limit, 2),
        dec(t.compactness, 2),
        dec(t.volume, 0),
        dec(t.envelope, 0),
        if t.k_fails() { " — NO CUMPLE" } else { "" }
    );
    let level_name = |l: LevelId| model.level(l).map_or("—".to_string(), |x| x.name.clone());
    let _ = writeln!(s, "\nCargas por local (W)");
    for r in &t.rooms {
        let _ = writeln!(
            s,
            "  {} ({}, {} m²): calefacción {} (transmisión {} + ventilación {}); refrigeración {} (transmisión {}, sol {}, interior {}, ventilación {}, latente {}){}{}",
            r.name,
            level_name(r.level),
            dec(r.area, 1),
            dec(r.heating, 0),
            dec(r.heat_transmission, 0),
            dec(r.heat_ventilation, 0),
            dec(r.cooling, 0),
            dec(r.cool_transmission, 0),
            dec(r.cool_sun, 0),
            dec(r.cool_internal, 0),
            dec(r.cool_ventilation, 0),
            dec(r.cool_latent, 0),
            r.unit
                .map_or(String::new(), |u| format!("; unidad interior {} kW", dec(u, 1))),
            if r.radiators.is_empty() {
                String::new()
            } else {
                format!(
                    "; radiadores {} W{}",
                    dec(r.radiator_power, 0),
                    if r.radiators_short() { " — NO BASTAN" } else { "" }
                )
            }
        );
    }
    let _ = writeln!(s, "\nEquipos");
    for g in &t.groups {
        let _ = writeln!(
            s,
            "  {}: calefacción {} kW, refrigeración {} kW → bomba de calor de {} kW",
            g.name,
            dec(g.heating / 1000.0, 2),
            dec(g.cooling / 1000.0, 2),
            dec(g.heat_pump, 1)
        );
    }
    let _ = writeln!(
        s,
        "\nTotal: calefacción {} kW, refrigeración {} kW",
        dec(t.heating() / 1000.0, 1),
        dec(t.cooling() / 1000.0, 1)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewSlab;

    #[test]
    fn the_limits() {
        assert!((k_limit(1, 0.5) - 0.60).abs() < 1e-9);
        assert!((k_limit(1, 4.5) - 0.80).abs() < 1e-9);
        assert!((k_limit(1, 2.5) - 0.70).abs() < 1e-9);
        assert_eq!(pick(&HEAT_PUMPS, 4200.0), 5.0);
    }

    /// The flat of the ventilation, with a roof slab over it and the
    /// ground slab under it.
    #[test]
    fn a_flat_has_its_loads() {
        let mut m = Model::new("Piso");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        crate::ventilation::example_flat(
            &mut m,
            l0,
            l1,
            lib.partition,
            lib.facade,
            lib.interior_door,
        );
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        m.add_slab(NewSlab {
            slab_type: lib.ground_slab,
            level: l0,
            outline: square.clone(),
            top_offset: 0.0,
        })
        .unwrap();
        m.add_slab(NewSlab {
            slab_type: lib.flat_roof,
            level: l1,
            outline: square,
            top_offset: 0.0,
        })
        .unwrap();
        let t = thermal(&m);
        assert_eq!(
            t.rooms.len(),
            6,
            "{:?}",
            t.rooms.iter().map(|r| &r.name).collect::<Vec<_>>()
        );
        let living = t.rooms.iter().find(|r| r.name == "Estar").unwrap();
        // Its south wall, its west wall, the roof and the ground.
        assert!(living
            .pieces
            .iter()
            .any(|p| p.kind == Kind::Wall && p.facing == Facing::South));
        assert!(living.pieces.iter().any(|p| p.kind == Kind::Roof));
        assert!(living.pieces.iter().any(|p| p.kind == Kind::Ground));
        assert!(living.heating > 0.0 && living.cooling > 0.0);
        // The corridor has no outside wall but its ends.
        let corridor = t.rooms.iter().find(|r| r.name == "Pasillo").unwrap();
        assert!(
            corridor
                .pieces
                .iter()
                .filter(|p| p.kind == Kind::Wall)
                .count()
                <= 2
        );
        // The façade type: its U is under 1 W/m²K and the elements are
        // checked against zone A.
        let wall = t.elements.iter().find(|e| e.kind == Kind::Wall).unwrap();
        assert!(wall.u > 0.1 && wall.u < 1.0, "{}", wall.u);
        assert!((wall.limit - 0.70).abs() < 1e-9);
        assert!(t.k > 0.0 && t.compactness > 0.5);
        assert_eq!(t.groups.len(), 1, "one dwelling");
        assert!(t.groups[0].heat_pump >= 3.5);
        let text = report(&m, &t);
        if std::env::var("ARCHISIM_CLIMA_TEXTO").is_ok() {
            println!("{text}");
        }
        assert!(text.contains("tabla 3.1.1.a-HE1"));
    }
}
