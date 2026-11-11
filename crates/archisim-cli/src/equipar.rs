//! `archisim equipar`: a house with its rooms fitted out for the project's
//! detail (LOD 400): the furniture and sanitary ware of a list, and the
//! installations every room needs by its use, joined up so that the
//! calculations follow them, then sized by those calculations.
//!
//! - **Electricity** (REBT, ITC-BT-25, electrificación elevada): the panel
//!   inside the entrance; light points by area, a switch at each door
//!   (two-way where a room has two), sockets round the walls by area and
//!   use, TV and data in the living rooms, bedrooms and offices, the
//!   kitchen's own (C3, C4, C5 at the worktop) and a socket in each
//!   bathroom. The circuit of each, by its class and room.
//! - **Water** (DB-HS 4): the meter in the façade by the entrance, the hot
//!   water heat pump with its tank in a store room near the wet rooms, a
//!   washing machine beside it, and the cold and hot mains in the false
//!   ceiling as a tree from the meter (and the tank) to each wet room;
//!   each appliance joined to the nearest main.
//! - **Drainage** (DB-HS 5): collectors hung in the ventilated crawl space
//!   under the boxes, falling to the inspection chamber outside, and each
//!   appliance's branch falling into them.
//! - **Ventilation** (DB-HS 3): an air inlet over a window of each dry
//!   room, an extract vent in each wet room, the extract ducts in the
//!   false ceiling to a shaft and the fan on the roof, and the kitchen
//!   hood's own duct.
//!
//! The rules are the usual ones, to be checked by the calculations (and by
//! whoever designs the installations).

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{
    point_in_polygon, DeviceKind as K, LevelId, Model, NewDevice, NewObject, NewRun, Point2, Room,
    RoomId, ServiceSystem as S,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Height of the water mains, over the level: in the false ceiling.
pub const WATER_Z: f64 = 2.76;
/// Height of the ducts' axis, over the level: in the false ceiling.
pub const DUCT_Z: f64 = 2.84;
/// Height of the drainage collectors at the inspection chamber: hung in the
/// crawl space under the boxes.
pub const DRAIN_EXIT_Z: f64 = -0.75;
/// Fall of the hung collectors (DB-HS 5, 4.1.3.3: 1 % at least).
pub const COLLECTOR_SLOPE: f64 = 0.015;
/// Fall of the branches to them.
pub const BRANCH_SLOPE: f64 = 0.03;
/// Height of the head of the short downpipes, with their valve: in the
/// WC's frame or the basin's cabinet.
pub const VALVE_Z: f64 = 0.9;
/// Height where the branches end on their downpipe: in the floor.
pub const BRANCH_END_Z: f64 = -0.20;
/// Highest a collector may start: under the chassis.
pub const DRAIN_TOP_Z: f64 = -0.32;

/// What a room is, for its installations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    Living,
    /// A living room with the kitchen in it.
    LivingKitchen,
    Kitchen,
    Bedroom,
    Office,
    Bath,
    Wc,
    Corridor,
    Storage,
    Other,
}

impl Use {
    pub fn of(r: &Room) -> Use {
        let n = format!("{} {}", r.name, r.usage).to_lowercase();
        let has = |k: &str| n.contains(k);
        let living = has("salón") || has("salon") || has("comedor") || has("estar");
        if has("cocina") && living {
            Use::LivingKitchen
        } else if has("cocina") {
            Use::Kitchen
        } else if has("baño") || has("bano") {
            Use::Bath
        } else if has("aseo") {
            Use::Wc
        } else if has("dormitorio") || has("habitación") {
            Use::Bedroom
        } else if has("despacho") || has("estudio") {
            Use::Office
        } else if living || has("estancia") {
            Use::Living
        } else if has("pasillo")
            || has("circulaci")
            || has("distribuidor")
            || has("vestíbulo")
            || has("recibidor")
        {
            Use::Corridor
        } else if has("despensa")
            || has("almac")
            || has("armario")
            || has("vestidor")
            || has("trastero")
            || has("lavader")
            || has("instalaci")
        {
            Use::Storage
        } else {
            Use::Other
        }
    }

    pub fn wet(self) -> bool {
        matches!(
            self,
            Use::LivingKitchen | Use::Kitchen | Use::Bath | Use::Wc
        )
    }

    pub fn dry(self) -> bool {
        matches!(
            self,
            Use::Living | Use::LivingKitchen | Use::Bedroom | Use::Office
        )
    }
}

/// A door: its middle on the wall's axis, along the wall, its half width,
/// and the rooms on either side (`None`: outside).
#[derive(Debug, Clone)]
struct Door {
    at: Point2,
    u: Point2,
    half: f64,
    rooms: [Option<RoomId>; 2],
    /// A point just inside each side.
    sides: [Point2; 2],
    /// The side that is outside the building (no room, box or false
    /// ceiling there), if one is.
    outside: Option<usize>,
}

impl Door {
    fn exterior(&self) -> bool {
        self.outside.is_some()
    }
}

/// The entrance: the widest door to the outside.
fn entrance(doors: &[Door]) -> Option<&Door> {
    doors
        .iter()
        .filter(|d| d.exterior())
        .max_by(|a, b| a.half.total_cmp(&b.half))
}

fn room_at(m: &Model, level: LevelId, p: Point2) -> Option<RoomId> {
    m.rooms()
        .filter(|r| r.level == level)
        .find(|r| point_in_polygon(p, &r.outline))
        .map(|r| r.id)
}

fn doors(m: &Model, level: LevelId) -> Vec<Door> {
    let mut out = Vec::new();
    for o in m.openings() {
        let Some(t) = m.opening_type(o.opening_type) else {
            continue;
        };
        if !t.is_door() || m.wall(o.wall).is_none_or(|w| w.level != level) {
            continue;
        }
        let Some(g) = archisim_geom::opening_geometry(m, o.id) else {
            continue;
        };
        let tm = (g.t0 + g.t1) / 2.0;
        let sgn = (g.exterior_offset - g.interior_offset).signum();
        let sides = [
            g.point(tm, g.exterior_offset + 0.3 * sgn),
            g.point(tm, g.interior_offset - 0.3 * sgn),
        ];
        let rooms = [room_at(m, level, sides[0]), room_at(m, level, sides[1])];
        let far = [
            g.point(tm, g.exterior_offset + 0.8 * sgn),
            g.point(tm, g.interior_offset - 0.8 * sgn),
        ];
        let out_there = |p: Point2| {
            room_at(m, level, p).is_none()
                && m.boxes().all(|b| !point_in_polygon(p, &b.outline()))
                && m.ceilings()
                    .all(|c| c.level != level || !point_in_polygon(p, &c.outline))
        };
        let outside =
            (0..2).find(|&i| rooms[1 - i].is_some() && rooms[i].is_none() && out_there(far[i]));
        out.push(Door {
            at: g.point(tm, (g.interior_offset + g.exterior_offset) / 2.0),
            u: g.u,
            half: (g.t1 - g.t0) / 2.0,
            rooms,
            sides,
            outside,
        });
    }
    out
}

/// A spot against the room's walls: the point (just inside the outline)
/// and the turn of a device on it (its x along the wall, the room on its
/// left).
#[derive(Debug, Clone, Copy)]
struct Spot {
    at: Point2,
    turn: f64,
    /// Distance along the outline from its first point.
    s: f64,
}

fn outline_spots(r: &Room) -> Vec<Spot> {
    let n = r.outline.len();
    let mut out = Vec::new();
    let mut s0 = 0.0;
    for i in 0..n {
        let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
        let len = a.distance(b);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let inward = u.perp_left();
        let mut t = 0.4;
        while t <= len - 0.4 + 1e-9 {
            out.push(Spot {
                at: a + u * t + inward * 0.03,
                turn: u.y.atan2(u.x),
                s: s0 + t,
            });
            t += 0.05;
        }
        s0 += len;
    }
    out
}

/// `count` spots round a room's walls, as evenly spread as its doors and
/// what is already there allow.
fn spread(r: &Room, count: usize, doors: &[Door], taken: &mut Vec<Point2>) -> Vec<Spot> {
    if count == 0 {
        return Vec::new();
    }
    let all = outline_spots(r);
    let free: Vec<Spot> = all
        .into_iter()
        .filter(|sp| {
            doors.iter().all(|d| d.at.distance(sp.at) > d.half + 0.3)
                && taken.iter().all(|t| t.distance(sp.at) > 0.6)
        })
        .collect();
    let Some(last) = free.last() else {
        return Vec::new();
    };
    let perimeter = last.s + 0.4;
    let mut out: Vec<Spot> = Vec::new();
    for k in 0..count {
        let target = (k as f64 + 0.5) * perimeter / count as f64;
        let best = free
            .iter()
            .filter(|sp| out.iter().all(|o| o.at.distance(sp.at) > 0.6))
            .min_by(|a, b| (a.s - target).abs().total_cmp(&(b.s - target).abs()));
        if let Some(sp) = best {
            out.push(*sp);
        }
    }
    taken.extend(out.iter().map(|s| s.at));
    out
}

/// The spot of the room's walls nearest to a point.
fn spot_near(r: &Room, p: Point2, taken: &mut Vec<Point2>) -> Option<Spot> {
    let sp = outline_spots(r)
        .into_iter()
        .filter(|s| taken.iter().all(|t| t.distance(s.at) > 0.25))
        .min_by(|a, b| a.at.distance(p).total_cmp(&b.at.distance(p)))?;
    taken.push(sp.at);
    Some(sp)
}

/// `count` light points inside a room: along its longer side.
fn light_points(r: &Room, count: usize) -> Vec<Point2> {
    let (mut lo, mut hi) = (
        Point2::new(f64::MAX, f64::MAX),
        Point2::new(f64::MIN, f64::MIN),
    );
    for p in &r.outline {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let along_x = hi.x - lo.x >= hi.y - lo.y;
    let mut out = Vec::new();
    let c = archisim_core::rooms::centroid(&r.outline);
    for k in 0..count.max(1) {
        let f = (k as f64 + 0.5) / count.max(1) as f64;
        let p = if count <= 1 {
            c
        } else if along_x {
            Point2::new(lo.x + f * (hi.x - lo.x), c.y)
        } else {
            Point2::new(c.x, lo.y + f * (hi.y - lo.y))
        };
        // Inside, or the nearest point of the room's middle line.
        let p = if point_in_polygon(p, &r.outline) {
            p
        } else if point_in_polygon(r.label, &r.outline) {
            r.label
        } else {
            continue;
        };
        out.push(p);
    }
    out
}

fn device(
    m: &mut Model,
    level: LevelId,
    kind: K,
    name: &str,
    at: Point2,
    turn: f64,
    z: Option<f64>,
) -> Res<archisim_core::DeviceId> {
    let (w, d, h, z0) = kind.defaults();
    Ok(m.add_device(NewDevice {
        level,
        kind,
        name: if name.is_empty() {
            kind.name().to_string()
        } else {
            name.to_string()
        },
        position: at,
        rotation: turn,
        elevation: z.unwrap_or(z0),
        width: w,
        depth: d,
        height: h,
    })?)
}

/// The clear height of a room (its own, or to the level over it).
fn clear_height(m: &Model, r: &Room) -> f64 {
    if let Some(h) = r.height {
        return h;
    }
    // The false ceiling over it, if there is one.
    if let Some(c) = m
        .ceilings()
        .filter(|c| c.level == r.level && point_in_polygon(r.label, &c.outline))
        .map(|c| c.height)
        .reduce(f64::min)
    {
        return c;
    }
    let z = m.level(r.level).map_or(0.0, |l| l.elevation);
    m.levels()
        .iter()
        .map(|l| l.elevation - z)
        .filter(|h| *h > 2.0)
        .reduce(f64::min)
        .unwrap_or(2.6)
}

// --- Objects -------------------------------------------------------------

/// The objects of a list (name; x; y; turn in degrees: the middle of its
/// back, as it is placed), of the library's catalogue or of the project.
pub fn place_objects(m: &mut Model, level: LevelId, list: &str) -> Res<String> {
    let mut t = String::new();
    let mut n = 0;
    for raw in list.lines() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = raw.split(';').map(str::trim).collect();
        let num = |s: &str| s.replace(',', ".").parse::<f64>().ok();
        let (Some(name), Some(x), Some(y)) = (
            f.first(),
            f.get(1).and_then(|s| num(s)),
            f.get(2).and_then(|s| num(s)),
        ) else {
            let _ = writeln!(t, "  no se entiende «{raw}» (nombre; x; y; giro)");
            continue;
        };
        let turn = f.get(3).and_then(|s| num(s)).unwrap_or(0.0).to_radians();
        let own = m.object_types().find(|o| o.name == *name).map(|o| o.id);
        let ty = match own {
            Some(id) => id,
            None => match archisim_library::add_object_entry(m, name) {
                Ok(id) => id,
                Err(e) => {
                    let _ = writeln!(t, "  «{name}»: no está en la biblioteca ({e})");
                    continue;
                }
            },
        };
        m.add_object(NewObject {
            level,
            object_type: ty,
            position: Point2::new(x, y),
            rotation: turn,
            base_offset: 0.0,
        })?;
        n += 1;
    }
    let _ = writeln!(t, "Objetos colocados: {n}");
    Ok(t)
}

// --- Electricity ---------------------------------------------------------

/// How many of each point a room gets (ITC-BT-25, tabla 2, electrificación
/// elevada): lights, sockets (C2), TV, data, bathroom and worktop sockets
/// (C5), cooker (C3) and appliances (C4).
fn points(u: Use, area: f64) -> [usize; 7] {
    let per = |a: f64, k: f64| (a / k).ceil() as usize;
    match u {
        Use::Living => [per(area, 10.0).max(1), per(area, 6.0).max(3), 1, 1, 0, 0, 0],
        Use::LivingKitchen => [
            per(area, 10.0).max(2),
            per(area * 0.7, 6.0).max(3),
            1,
            1,
            3,
            1,
            0,
        ],
        Use::Kitchen => [1, 2, 0, 0, 3, 1, 0],
        Use::Bedroom => [1, 3, 1, 1, 0, 0, 0],
        Use::Office => [1, 3, 0, 1, 0, 0, 0],
        Use::Bath => [1, 0, 0, 0, 1, 0, 0],
        // A toilet with its basin: no socket (C5 is for the bathrooms and
        // the kitchen).
        Use::Wc => [1, 0, 0, 0, 0, 0, 0],
        Use::Corridor => [per(area, 6.0).max(1), 1, 0, 0, 0, 0, 0],
        Use::Storage => [1, usize::from(area >= 3.0), 0, 0, 0, 0, 0],
        Use::Other => [1, 1, 0, 0, 0, 0, 0],
    }
}

/// The kitchen's worktop: where its base units are.
fn kitchen_spots(m: &Model, r: &Room) -> Vec<(Point2, f64)> {
    m.objects()
        .filter(|o| o.level == r.level && point_in_polygon(o.position, &r.outline))
        .filter(|o| {
            m.object_type(o.object_type).is_some_and(|t| {
                let n = t.name.to_lowercase();
                n.starts_with("bajo") || n.contains("encimera") || n.contains("isla")
            })
        })
        .map(|o| (o.position, o.rotation))
        .collect()
}

pub fn electrify(m: &mut Model, level: LevelId) -> Res<String> {
    let mut t = String::new();
    let doors = doors(m, level);
    let rooms: Vec<Room> = m.rooms().filter(|r| r.level == level).cloned().collect();
    let mut taken: Vec<Point2> = m
        .devices()
        .filter(|d| d.level == level)
        .map(|d| d.position)
        .collect();
    let mut counts = [0usize; 7];
    // Over 160 m² or 20 sockets: a second circuit of sockets (C7).
    let sockets: usize = rooms.iter().map(|r| points(Use::of(r), r.area())[1]).sum();
    let floor: f64 = rooms.iter().map(|r| r.area()).sum();
    let big = sockets > 20 || floor > 160.0;
    // The panel: inside the entrance, beside the door.
    let entrance = entrance(&doors);
    if let Some(d) = entrance {
        let inside = 1 - d.outside.unwrap_or(1);
        let into = (d.sides[inside] - d.at).normalized().unwrap_or_default();
        let at = d.at + d.u * (d.half + 0.45) + into * 0.13;
        device(
            m,
            level,
            K::Panel,
            "Cuadro de la vivienda (CGMP)",
            at,
            d.u.y.atan2(d.u.x),
            None,
        )?;
        taken.push(at);
    }
    for r in &rooms {
        let u = Use::of(r);
        let h = clear_height(m, r);
        let p = points(u, r.area());
        // Lights.
        for at in light_points(r, p[0]) {
            let (_, _, lh, _) = K::Light.defaults();
            device(m, level, K::Light, "", at, 0.0, Some(h - lh))?;
            counts[0] += 1;
        }
        // A switch inside each door of the room, on the side away from the
        // hinge (beside the jamb); two-way where there are two or more.
        let mine: Vec<&Door> = doors
            .iter()
            .filter(|d| d.rooms.contains(&Some(r.id)))
            .collect();
        let kind = if mine.len() >= 2 {
            K::TwoWaySwitch
        } else {
            K::Switch
        };
        for d in &mine {
            let side = if d.rooms[0] == Some(r.id) { 0 } else { 1 };
            let into = (d.sides[side] - d.at).normalized().unwrap_or_default();
            let mut best: Option<Point2> = None;
            for sgn in [1.0, -1.0] {
                let at = d.at + d.u * (sgn * (d.half + 0.15)) + into * 0.12;
                if point_in_polygon(at, &r.outline) && taken.iter().all(|x| x.distance(at) > 0.1) {
                    best = Some(at);
                    break;
                }
            }
            if let Some(at) = best {
                device(m, level, kind, "", at, d.u.y.atan2(d.u.x), None)?;
                taken.push(at);
            }
        }
        // Sockets round the walls.
        for sp in spread(r, p[1], &doors, &mut taken) {
            let id = device(m, level, K::Socket, "", sp.at, sp.turn, None)?;
            if u == Use::LivingKitchen {
                // General use, though the room has the kitchen in it.
                m.set_device_circuit(id, Some(2))?;
            }
            if matches!(u, Use::Bedroom | Use::Office) && big {
                // More than 20 sockets or 160 m²: the bedrooms' and the
                // office's on the additional circuit (C7).
                m.set_device_circuit(id, Some(7))?;
            }
            counts[1] += 1;
        }
        for (k, i) in [(K::TvSocket, 2), (K::DataSocket, 3)] {
            for sp in spread(r, p[i], &doors, &mut taken) {
                device(m, level, k, "", sp.at, sp.turn, None)?;
                counts[i] += 1;
            }
        }
        // The kitchen's: at the worktop (C5 at 1,10), the cooker (C3) and
        // the dishwasher (C4) by its units.
        if p[4] > 0 {
            let near = kitchen_spots(m, r);
            if u.wet() && !near.is_empty() && matches!(u, Use::Kitchen | Use::LivingKitchen) {
                for k in 0..p[4] {
                    let (at, turn) = near[k * near.len() / p[4]];
                    let back = Point2::new(turn.sin(), -turn.cos());
                    let at = at - back * (-0.02) + Point2::new(turn.cos(), turn.sin()) * 0.1;
                    let id = device(m, level, K::Socket, "Toma de encimera", at, turn, Some(1.1))?;
                    m.set_device_circuit(id, Some(5))?;
                    taken.push(at);
                    counts[4] += 1;
                }
            } else {
                for sp in spread(r, p[4], &doors, &mut taken) {
                    device(m, level, K::Socket, "", sp.at, sp.turn, Some(1.1))?;
                    counts[4] += 1;
                }
            }
        }
        if p[5] > 0 {
            let near = kitchen_spots(m, r);
            let at = near
                .first()
                .map(|(p, t)| (*p + Point2::new(t.cos(), t.sin()) * 0.3, *t));
            if let Some((at, turn)) = at {
                device(
                    m,
                    level,
                    K::PowerSocket,
                    "Toma de cocina y horno",
                    at,
                    turn,
                    None,
                )?;
                counts[5] += 1;
            } else if let Some(sp) = spread(r, 1, &doors, &mut taken).first() {
                device(
                    m,
                    level,
                    K::PowerSocket,
                    "Toma de cocina y horno",
                    sp.at,
                    sp.turn,
                    None,
                )?;
                counts[5] += 1;
            }
        }
        if p[6] > 0 {
            let near = kitchen_spots(m, r);
            for k in 0..p[6] {
                let place = near
                    .get(near.len().saturating_sub(1).min(k + 1))
                    .map(|(p, t)| (*p - Point2::new(t.cos(), t.sin()) * 0.2, *t));
                let (at, turn) = match place {
                    Some(x) => x,
                    None => match spread(r, 1, &doors, &mut taken).first() {
                        Some(sp) => (sp.at, sp.turn),
                        None => continue,
                    },
                };
                let id = device(m, level, K::Socket, "Toma de lavavajillas", at, turn, None)?;
                m.set_device_circuit(id, Some(4))?;
                counts[6] += 1;
            }
        }
    }
    let _ = writeln!(
        t,
        "Electricidad: {} puntos de luz, {} enchufes, {} de TV, {} de datos, {} de baño y encimera (C5), {} de cocina (C3) y {} de lavavajillas (C4)",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6]
    );
    Ok(t)
}

// --- Pipes and ducts as trees -------------------------------------------

fn crosses(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let o = |p: Point2, q: Point2, r: Point2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// A tree of straight runs from `root` to each target: each target, the
/// nearest to the tree first, joined to the nearest point of the tree by
/// one or two straight stretches (the way that crosses fewer walls). Each
/// path goes from its target to the tree (the first, to the root).
fn tree(
    walls: &[(Point2, Point2)],
    inside: &[Vec<Point2>],
    root: Point2,
    targets: &[Point2],
) -> Vec<Vec<Point2>> {
    let mut segs: Vec<(Point2, Point2)> = Vec::new();
    let mut out = Vec::new();
    let mut left: Vec<Point2> = targets.to_vec();
    let nearest = |segs: &[(Point2, Point2)], p: Point2| -> Point2 {
        if segs.is_empty() {
            return root;
        }
        let mut best = root;
        let mut bd = f64::MAX;
        for (a, b) in segs {
            let ab = *b - *a;
            let l2 = ab.dot(ab);
            let t = if l2 < 1e-12 {
                0.0
            } else {
                ((p - *a).dot(ab) / l2).clamp(0.0, 1.0)
            };
            let q = *a + ab * t;
            if q.distance(p) < bd {
                bd = q.distance(p);
                best = q;
            }
        }
        best
    };
    while !left.is_empty() {
        let (i, q) = left
            .iter()
            .enumerate()
            .map(|(i, p)| (i, nearest(&segs, *p)))
            .min_by(|a, b| left[a.0].distance(a.1).total_cmp(&left[b.0].distance(b.1)))
            .unwrap();
        let p = left.remove(i);
        if p.distance(q) < 0.05 {
            continue;
        }
        let crossings = |path: &[Point2]| -> usize {
            path.windows(2)
                .map(|s| {
                    walls
                        .iter()
                        .filter(|w| crosses(s[0], s[1], w.0, w.1))
                        .count()
                })
                .sum()
        };
        // How much of a way runs outside the building (every 25 cm).
        let outside = |path: &[Point2]| -> usize {
            if inside.is_empty() {
                return 0;
            }
            path.windows(2)
                .map(|s| {
                    let n = (s[0].distance(s[1]) / 0.25).ceil().max(1.0) as usize;
                    (0..=n)
                        .filter(|k| {
                            let x = s[0] + (s[1] - s[0]) * (*k as f64 / n as f64);
                            !inside.iter().any(|poly| point_in_polygon(x, poly))
                        })
                        .count()
                })
                .sum()
        };
        let mut options = vec![
            vec![p, Point2::new(q.x, p.y), q],
            vec![p, Point2::new(p.x, q.y), q],
        ];
        // Round by the middle of a room or a box (an L-shaped house: not
        // across its courtyard).
        for poly in inside {
            let c = archisim_core::rooms::centroid(poly);
            options.push(vec![p, Point2::new(p.x, c.y), Point2::new(q.x, c.y), q]);
            options.push(vec![p, Point2::new(c.x, p.y), Point2::new(c.x, q.y), q]);
        }
        let score = |o: &Vec<Point2>| {
            let len: f64 = o.windows(2).map(|w| w[0].distance(w[1])).sum();
            outside(o) as f64 * 100.0 + crossings(o) as f64 * 2.0 + len
        };
        let mut path = options
            .into_iter()
            .min_by(|a, b| score(a).total_cmp(&score(b)))
            .unwrap();
        path.dedup_by(|a, b| a.distance(*b) < 0.02);
        for w in path.windows(2) {
            segs.push((w[0], w[1]));
        }
        out.push(path);
    }
    out
}

/// Where the pipes may go on a level: its rooms and its boxes.
fn indoors(m: &Model, level: LevelId) -> Vec<Vec<Point2>> {
    m.rooms()
        .filter(|r| r.level == level)
        .map(|r| r.outline.clone())
        .chain(m.boxes().filter(|b| b.level == level).map(|b| b.outline()))
        .collect()
}

fn walls_on(m: &Model, level: LevelId) -> Vec<(Point2, Point2)> {
    m.walls_on_level(level).map(|w| (w.start, w.end)).collect()
}

/// Where each sanitary appliance's pipes go to (hot: only those that take
/// hot water).
fn appliance_points(m: &Model, level: LevelId, hot: bool) -> Vec<Point2> {
    m.appliances_on(level)
        .iter()
        .filter(|a| !hot || a.kind.hot())
        .map(|a| a.point)
        .collect()
}

/// The foot of each downpipe: at every WC (its back, 20 cm in: a WC
/// within 1 m of it, 3.3.1.4), and in each room with appliances farther
/// than 3,5 m from those, in the middle of them (off the appliances).
fn drain_points(m: &Model, level: LevelId) -> Vec<Point2> {
    let appliances = m.appliances_on(level);
    let mut out: Vec<Point2> = Vec::new();
    for a in &appliances {
        if !matches!(a.kind, archisim_core::Appliance::Wc) {
            continue;
        }
        if let Some(o) = m.objects().find(|o| o.id == a.object) {
            let back = Point2::new(o.rotation.sin(), -o.rotation.cos());
            out.push(a.point - back * 0.25);
        }
    }
    for r in m.rooms().filter(|r| r.level == level) {
        let here: Vec<_> = appliances
            .iter()
            .filter(|a| point_in_polygon(a.point, &r.outline))
            .filter(|a| out.iter().all(|f| f.distance(a.point) > 3.5))
            .collect();
        if here.is_empty() {
            continue;
        }
        let c =
            here.iter().fold(Point2::new(0.0, 0.0), |s, a| s + a.point) * (1.0 / here.len() as f64);
        let mut p = if point_in_polygon(c, &r.outline) {
            c
        } else {
            r.label
        };
        // Not on an appliance's own point: 30 cm towards the room.
        if here.iter().any(|a| a.point.distance(p) < 0.1) {
            p = p + (r.label - p).normalized().unwrap_or_default() * 0.3;
        }
        out.push(p);
    }
    out
}

/// The middle of each wet room's sanitary appliances (or of the room).
fn wet_points(m: &Model, level: LevelId, hot: bool) -> Vec<Point2> {
    let appliances = m.appliances_on(level);
    let mut out = Vec::new();
    for r in m.rooms().filter(|r| r.level == level) {
        let here: Vec<Point2> = appliances
            .iter()
            .filter(|a| (!hot || a.kind.hot()) && point_in_polygon(a.point, &r.outline))
            .map(|a| a.point)
            .collect();
        if here.is_empty() {
            continue;
        }
        let c = here.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / here.len() as f64);
        // Into the room, a little off the walls.
        let c = if point_in_polygon(c, &r.outline) {
            c
        } else {
            r.label
        };
        out.push(c);
    }
    out
}

fn run(
    m: &mut Model,
    level: LevelId,
    system: S,
    mut path: Vec<Point2>,
    z: f64,
    size: f64,
    slope: f64,
) -> Res<Option<archisim_core::RunId>> {
    path.dedup_by(|a, b| a.distance(*b) < 0.02);
    if path.len() < 2 {
        return Ok(None);
    }
    let (w, h) = if system.is_duct() {
        (size, 0.10)
    } else {
        (size, size)
    };
    Ok(Some(m.add_run(NewRun {
        level,
        system,
        path,
        elevation: z,
        width: w,
        height: h,
        slope,
    })?))
}

/// The store room nearest to the wet rooms (for the tank and the washing
/// machine), or else the nearest wet room.
fn utility_room(m: &Model, level: LevelId) -> Option<Room> {
    let wet = wet_points(m, level, false);
    let middle = if wet.is_empty() {
        return None;
    } else {
        wet.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / wet.len() as f64)
    };
    let rooms: Vec<&Room> = m.rooms().filter(|r| r.level == level).collect();
    let by = |u: &dyn Fn(Use) -> bool| {
        rooms
            .iter()
            .filter(|r| u(Use::of(r)) && r.area() >= 1.0)
            .min_by(|a, b| {
                a.label
                    .distance(middle)
                    .total_cmp(&b.label.distance(middle))
            })
            .map(|r| (*r).clone())
    };
    // A utility room by its name first; never a wardrobe.
    let named = |keys: &[&str]| {
        rooms
            .iter()
            .filter(|r| {
                let n = r.name.to_lowercase();
                keys.iter().any(|k| n.contains(k)) && r.area() >= 1.0
            })
            .min_by(|a, b| {
                a.label
                    .distance(middle)
                    .total_cmp(&b.label.distance(middle))
            })
            .map(|r| (*r).clone())
    };
    named(&["lavadero", "instalaci", "cuarto técnico"])
        .or_else(|| named(&["despensa", "trastero", "almac"]))
        .or_else(|| by(&|u| u.wet() && u != Use::Bath && u != Use::Wc))
        .or_else(|| by(&|u| u.wet()))
}

pub fn plumb(m: &mut Model, level: LevelId) -> Res<String> {
    let mut t = String::new();
    let doors = doors(m, level);
    let Some(entrance) = entrance(&doors).cloned() else {
        let _ = writeln!(
            t,
            "Fontanería: sin puerta al exterior, no se coloca la acometida"
        );
        return Ok(t);
    };
    let outside = entrance.outside.unwrap_or(0);
    let out_dir = (entrance.sides[outside] - entrance.at)
        .normalized()
        .unwrap_or_default();
    // The meter in the façade, past the panel's side of the door.
    let meter_at = entrance.at - entrance.u * (entrance.half + 0.6) + out_dir * 0.05;
    let turn = entrance.u.y.atan2(entrance.u.x);
    device(
        m,
        level,
        K::WaterMeter,
        "Contador de agua (en fachada)",
        meter_at,
        turn,
        None,
    )?;
    let into = meter_at - out_dir * 0.35;
    // The tank and the washing machine in the utility room.
    let mut taken: Vec<Point2> = m.devices().map(|d| d.position).collect();
    taken.extend(m.objects().map(|o| o.position));
    let utility = utility_room(m, level);
    let mut heater_at = None;
    if let Some(r) = &utility {
        if let Some(sp) = spot_near(r, r.label, &mut taken) {
            let id = device(
                m,
                level,
                K::WaterHeater,
                "Bomba de calor de ACS con depósito de 200 l",
                sp.at,
                sp.turn,
                Some(0.0),
            )?;
            m.edit_device(id, |d| {
                d.width = 0.6;
                d.depth = 0.6;
                d.height = 1.8;
            })?;
            // The heat pump on a circuit of its own (25 A).
            m.set_device_circuit(id, Some(8))?;
            heater_at = Some(sp.at);
        }
        let washer = m
            .appliances_on(level)
            .iter()
            .any(|a| matches!(a.kind, archisim_core::Appliance::Washer));
        if !washer {
            if let Some(sp) = spot_near(r, r.label, &mut taken) {
                let ty = archisim_library::add_object_entry(m, "Lavadora 60")?;
                m.add_object(NewObject {
                    level,
                    object_type: ty,
                    position: sp.at,
                    rotation: sp.turn,
                    base_offset: 0.0,
                })?;
            }
        }
    }
    let walls = walls_on(m, level);
    // Cold water: from the meter to the tank and every appliance.
    let mut targets = appliance_points(m, level, false);
    if let Some(h) = heater_at {
        targets.insert(0, h);
    }
    let mut runs = 0;
    for path in tree(&walls, &indoors(m, level), into, &targets) {
        run(m, level, S::ColdWater, path, WATER_Z, 0.025, 0.0)?;
        runs += 1;
    }
    // The meter up to the main.
    run(
        m,
        level,
        S::ColdWater,
        vec![meter_at, into],
        WATER_Z,
        0.032,
        0.0,
    )?;
    // Hot water: from the tank.
    if let Some(h) = heater_at {
        let hot = appliance_points(m, level, true);
        for path in tree(&walls, &indoors(m, level), h, &hot) {
            run(m, level, S::HotWater, path, WATER_Z - 0.05, 0.02, 0.0)?;
            runs += 1;
        }
        // The return (HS 4, 2.3) where the farthest tap is 15 m off: from
        // it back to the tank, beside the hot main.
        let far = hot
            .iter()
            .copied()
            .max_by(|a, b| a.distance(h).total_cmp(&b.distance(h)));
        if let Some(f) = far.filter(|f| (f.x - h.x).abs() + (f.y - h.y).abs() > 15.0) {
            for path in tree(&walls, &indoors(m, level), h, &[f]) {
                let path: Vec<Point2> = path.iter().map(|p| *p + Point2::new(0.06, 0.06)).collect();
                run(m, level, S::HotReturn, path, WATER_Z - 0.10, 0.02, 0.0)?;
            }
            let _ = writeln!(
                t,
                "  Retorno de ACS: el grifo más lejano está a más de 15 m"
            );
        }
    }
    let mut branches = 0;
    for (system, z) in [(S::ColdWater, WATER_Z), (S::HotWater, WATER_Z - 0.05)] {
        let (made, alone) = m.route_appliances(level, system, z);
        for r in made {
            m.add_run(r)?;
            branches += 1;
        }
        if !alone.is_empty() {
            let _ = writeln!(
                t,
                "  {} aparatos sin {}",
                alone.len(),
                system.name().to_lowercase()
            );
        }
    }
    let _ = writeln!(
        t,
        "Fontanería: contador en fachada, bomba de calor de ACS en «{}», {runs} tramos por el falso techo hasta cada aparato{}",
        utility.as_ref().map_or("—", |r| r.name.as_str()),
        if branches > 0 {
            format!(" y {branches} derivaciones")
        } else {
            String::new()
        }
    );
    // Drainage: the inspection chamber outside, the collectors hung in the
    // crawl space falling to it, the branches falling to them.
    let chamber = entrance.at + entrance.u * (entrance.half + 1.2) + out_dir * 1.5;
    let id = device(
        m,
        level,
        K::Manhole,
        "Arqueta de acometida 60×60",
        chamber,
        turn,
        Some(-1.1),
    )?;
    m.edit_device(id, |d| {
        d.width = 0.6;
        d.depth = 0.6;
        d.height = 0.8;
    })?;
    // A short downpipe through the floor in each wet room: at its WC (a
    // WC within 1 m of it, 3.3.1.4) or in the middle of its appliances.
    let feet: Vec<Point2> = drain_points(m, level);
    // Walls don't matter under the floor: the shortest way.
    let paths = tree(&[], &indoors(m, level), chamber, &feet);
    // Heights: along the tree from the chamber up.
    let mut known: Vec<(Point2, Point2, f64, f64)> = Vec::new(); // a, b, z at a, z at b
    let z_on = |known: &[(Point2, Point2, f64, f64)], p: Point2| -> f64 {
        for (a, b, za, zb) in known {
            let ab = *b - *a;
            let l = ab.length();
            if l < 1e-9 {
                continue;
            }
            let s = (p - *a).dot(ab) / (l * l);
            let q = *a + ab * s.clamp(0.0, 1.0);
            if q.distance(p) < 0.03 {
                return za + (zb - za) * s.clamp(0.0, 1.0);
            }
        }
        DRAIN_EXIT_Z
    };
    let mut high = f64::MIN;
    let mut collectors = 0;
    for path in paths {
        let end = *path.last().unwrap();
        let z_end = z_on(&known, end);
        let len: f64 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
        let z0 = z_end + COLLECTOR_SLOPE * len;
        high = high.max(z0);
        let mut acc = 0.0;
        for w in path.windows(2) {
            let l = w[0].distance(w[1]);
            known.push((
                w[0],
                w[1],
                z0 - COLLECTOR_SLOPE * acc,
                z0 - COLLECTOR_SLOPE * (acc + l),
            ));
            acc += l;
        }
        let (base, dz) = under(m, level);
        run(m, base, S::Drainage, path, z0 + dz, 0.11, COLLECTOR_SLOPE)?;
        collectors += 1;
    }
    for f in &feet {
        let z = z_on(&known, *f);
        // From the collector up through the floor (across the levels in
        // between, so that the calculation takes it as a downpipe).
        let (base, dz) = under(m, level);
        // Its head up in the WC's frame or the cabinet, with an air
        // admittance valve (3.3.3.1: a single-storey house, no column to
        // take through the box's roof).
        let id = device(
            m,
            base,
            K::Downpipe,
            "Bajante corta al colector",
            *f,
            0.0,
            Some(z + dz),
        )?;
        m.edit_device(id, |d| {
            d.width = 0.11;
            d.depth = 0.11;
            d.height = (VALVE_Z - z).max(0.1);
        })?;
        device(
            m,
            level,
            K::AirValve,
            "Válvula de aireación",
            *f,
            0.0,
            Some(VALVE_Z),
        )?;
    }
    // Each appliance's branch to its room's downpipe: one or two straight
    // stretches in the floor, falling to it.
    let mut drains = 0;
    let mut alone: Vec<archisim_core::ObjectId> = Vec::new();
    for a in m.appliances_on(level) {
        let room = m
            .rooms()
            .find(|r| r.level == level && point_in_polygon(a.point, &r.outline))
            .map(|r| r.outline.clone());
        let nearest = |inside: bool| {
            feet.iter()
                .copied()
                .filter(|f| !inside || room.as_ref().is_none_or(|o| point_in_polygon(*f, o)))
                .min_by(|x, y| x.distance(a.point).total_cmp(&y.distance(a.point)))
        };
        let foot = nearest(true)
            .filter(|f| f.distance(a.point) <= 4.0)
            .or_else(|| nearest(false));
        let Some(f) = foot else {
            alone.push(a.object);
            continue;
        };
        if f.distance(a.point) < 0.05 {
            continue;
        }
        let mut path = vec![a.point, Point2::new(f.x, a.point.y), f];
        path.dedup_by(|x, y| x.distance(*y) < 0.02);
        let len: f64 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
        run(
            m,
            level,
            S::Drainage,
            path,
            BRANCH_END_Z + BRANCH_SLOPE * len,
            if matches!(a.kind, archisim_core::Appliance::Wc) {
                0.11
            } else {
                0.04
            },
            BRANCH_SLOPE,
        )?;
        drains += 1;
    }
    let _ = writeln!(
        t,
        "Saneamiento: arqueta de acometida, {collectors} colectores colgados en el forjado sanitario (pendiente {:.1} %, de {:.2} a {:.2} m) y {drains} derivaciones{}",
        COLLECTOR_SLOPE * 100.0,
        high,
        DRAIN_EXIT_Z,
        if alone.is_empty() {
            String::new()
        } else {
            format!(", {} aparatos sin desagüe", alone.len())
        }
    );
    if high > DRAIN_TOP_Z {
        let _ = writeln!(t, "  Aviso: el colector más alto empieza a {high:.2}, por encima de {DRAIN_TOP_Z:.2} (el chasis)");
    }
    Ok(t)
}

/// Height of a collector at a point on it (`known`: its stretches with the
/// heights at both ends), or `exit` off it.
fn z_along(known: &[(Point2, Point2, f64, f64)], p: Point2, exit: f64) -> f64 {
    for (a, b, za, zb) in known {
        let ab = *b - *a;
        let l = ab.length();
        if l < 1e-9 {
            continue;
        }
        let s = (p - *a).dot(ab) / (l * l);
        if (*a + ab * s.clamp(0.0, 1.0)).distance(p) < 0.03 {
            return za + (zb - za) * s.clamp(0.0, 1.0);
        }
    }
    exit
}

/// The lowest level (the crawl space, or the ground) and how far `level`
/// is over it: the collectors go there.
fn under(m: &Model, level: LevelId) -> (LevelId, f64) {
    let z = m.level(level).map_or(0.0, |l| l.elevation);
    m.levels()
        .iter()
        .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
        .filter(|l| l.elevation < z)
        .map_or((level, 0.0), |l| (l.id, z - l.elevation))
}

// --- The system's rules on a model from another program ------------------

/// What the boxes' good practice asks of a model brought from another
/// program (`docs/MODULAR.md` §4), where the boxes are drawn over it:
/// - a thin cladding sheet is not the structure of its wall: the steel
///   frame fills the wall's cavity (the insulation between boards);
/// - a box's floor drawn as its finish only becomes the system's floor on
///   its chassis (so that the envelope counts it);
/// - each joint has one wall: of two back to back, the one without doors
///   goes, and that side of its box is left open.
pub fn system_rules(m: &mut Model) -> Res<String> {
    use archisim_core::{Layer, LayerFunction as F, SlabFunction};
    if m.boxes().next().is_none() {
        return Ok(String::new());
    }
    let mut t = String::new();
    // Materials with no class: by their name (an insulating core, a
    // membrane, a tile), so that the envelope's calculation knows them.
    let unnamed: Vec<archisim_core::Material> = m
        .materials()
        .filter(|x| x.category.trim().is_empty())
        .cloned()
        .collect();
    let mut classed = 0;
    for x in unnamed {
        let n = x.name.to_lowercase();
        let class = if n.contains("ais")
            || n.contains("lana")
            || n.contains("interior") && n.contains("tabihaus")
        {
            "insulation"
        } else if n.contains("geotextil") || n.contains("membrana") || n.contains("lámina") {
            "membrane"
        } else if n.contains("gres") || n.contains("porcel") || n.contains("cerámic") {
            "ceramic"
        } else {
            continue;
        };
        m.set_material(x.id, x.name.clone(), class, x.cut_color, x.cut_pattern)?;
        classed += 1;
    }
    if classed > 0 {
        let _ = writeln!(
            t,
            "Clase por su nombre en {classed} materiales que no la tenían"
        );
    }
    // The wall types.
    let types: Vec<archisim_core::WallType> = m.wall_types().cloned().collect();
    let mut framed = 0;
    for wt in types {
        let kind = |m: &Model, l: &Layer| {
            m.material(l.material).map_or(String::new(), |x| {
                format!("{} {}", x.category, x.name).to_lowercase()
            })
        };
        let soft = |m: &Model, l: &Layer| {
            matches!(l.function, F::Insulation | F::AirGap) || {
                let k = kind(m, l);
                k.contains("insulation") || k.contains("lana") || k.contains("aislamiento")
            }
        };
        // A wall (not a cladding a few centimetres thick) with insulation
        // between boards: the frame fills it.
        let total: f64 = wt.layers.iter().map(|l| l.thickness).sum();
        let cavity = (1..wt.layers.len().saturating_sub(1))
            .filter(|i| total >= 0.12 && soft(m, &wt.layers[*i]) && wt.layers[*i].thickness >= 0.06)
            .max_by(|a, b| wt.layers[*a].thickness.total_cmp(&wt.layers[*b].thickness));
        let thin_structure = wt
            .layers
            .iter()
            .any(|l| l.function == F::Structure && l.thickness < 0.03);
        if !thin_structure && cavity.is_none() {
            continue;
        }
        let layers: Vec<Layer> = wt
            .layers
            .iter()
            .enumerate()
            .map(|(k, l)| Layer {
                function: if Some(k) == cavity {
                    F::Structure
                } else if l.function == F::Structure {
                    F::Finish
                } else {
                    l.function
                },
                ..l.clone()
            })
            .collect();
        if layers != wt.layers {
            m.edit_wall_type(wt.id, wt.name.clone(), wt.function, layers)?;
        }
        if let Some(c) = cavity {
            if m.type_framing(wt.id.0).is_none() {
                let depth = wt.layers[c].thickness;
                m.set_type_framing(
                    wt.id.0,
                    Some(archisim_core::Framing::for_depth(
                        archisim_core::FrameMaterial::Steel,
                        depth,
                    )),
                )?;
                framed += 1;
            }
        }
    }
    let _ = writeln!(t, "Entramado de acero en {framed} tipos de muro (la capa de aislamiento entre placas); las placas finas ya no son la estructura");
    // The studs drawn by hand as columns inside those walls: the frame of
    // the type makes them now.
    let framed_walls: Vec<(LevelId, Point2, Point2)> = m
        .walls()
        .filter(|w| m.type_framing(w.wall_type.0).is_some())
        .map(|w| (w.level, w.start, w.end))
        .collect();
    let studs: Vec<archisim_core::ColumnId> = m
        .columns()
        .filter(|c| {
            m.member_type(c.member_type).is_some_and(|mt| {
                let (a, b) = mt.profile.size();
                a.max(b) <= 0.12
            }) && framed_walls.iter().any(|(l, a, b)| {
                *l == c.level && {
                    let ab = *b - *a;
                    let s = ((c.position - *a).dot(ab) / ab.dot(ab).max(1e-9)).clamp(0.0, 1.0);
                    (*a + ab * s).distance(c.position) < 0.15
                }
            })
        })
        .map(|c| c.id)
        .collect();
    for c in &studs {
        m.remove_column(*c)?;
    }
    if !studs.is_empty() {
        let _ = writeln!(
            t,
            "Quitados {} montantes dibujados a mano como pilares (los pone el entramado del tipo)",
            studs.len()
        );
    }
    // Floor types brought upside down (the structure on top, the finish
    // under it): turned over, top to bottom.
    let upside: Vec<archisim_core::SlabType> = m
        .slab_types()
        .filter(|x| {
            x.function == SlabFunction::Floor
                && x.layers.len() > 1
                && x.layers.first().is_some_and(|l| l.function == F::Structure)
                && x.layers.last().is_some_and(|l| {
                    m.material(l.material).is_some_and(|mt| {
                        let k = format!("{} {}", mt.category, mt.name).to_lowercase();
                        [
                            "ceramic", "gres", "porcel", "suelo", "tarima", "parquet", "stone",
                            "piedra",
                        ]
                        .iter()
                        .any(|w| k.contains(w))
                    })
                })
        })
        .cloned()
        .collect();
    for x in &upside {
        let mut layers = x.layers.clone();
        layers.reverse();
        // The finish on top, what is between it and the structure under it.
        let last = layers.len() - 1;
        for (k, l) in layers.iter_mut().enumerate() {
            if k == 0 {
                l.function = F::Finish;
            } else if k < last && l.function != F::Insulation && l.function != F::Membrane {
                l.function = F::Substrate;
            }
        }
        m.edit_slab_type(x.id, x.name.clone(), x.function, layers)?;
    }
    if !upside.is_empty() {
        let _ = writeln!(
            t,
            "Capas de arriba abajo en {} tipos de suelo que venían al revés",
            upside.len()
        );
    }
    // The boxes' floors.
    let floor_type = {
        let existing = m
            .slab_types()
            .find(|x| x.name == "Suelo de caja 270 (chasis)")
            .map(|x| x.id);
        match existing {
            Some(id) => id,
            None => {
                let wood = material(m, "Tarima de madera", "wood", [190, 150, 105]);
                let board = insulating_board(m);
                let deck = material(m, "Forjado colaborante H90+30", "concrete", [150, 155, 160]);
                let wool = material(m, "Lana mineral", "insulation", [235, 225, 150]);
                let l = |material, thickness, function| Layer {
                    material,
                    thickness,
                    function,
                };
                m.add_slab_type(
                    "Suelo de caja 270 (chasis)",
                    SlabFunction::Floor,
                    vec![
                        l(wood, 0.020, F::Finish),
                        l(board, 0.059, F::Substrate),
                        l(deck, 0.120, F::Structure),
                        l(wool, 0.071, F::Insulation),
                    ],
                )?
            }
        }
    };
    let boxes: Vec<archisim_core::ModularBox> = m.boxes().cloned().collect();
    let thin: Vec<archisim_core::SlabId> = m
        .slabs()
        .filter(|s| {
            let c = archisim_core::rooms::centroid(&s.outline);
            boxes
                .iter()
                .any(|b| b.level == s.level && point_in_polygon(c, &b.outline()))
                && s.top_at(c).abs() < 0.1
                && m.slab_type(s.slab_type).is_some_and(|x| {
                    x.function == SlabFunction::Floor
                        && x.layers.iter().map(|l| l.thickness).sum::<f64>() < 0.1
                })
        })
        .map(|s| s.id)
        .collect();
    for id in &thin {
        m.set_slab_type(*id, floor_type)?;
    }
    let _ = writeln!(
        t,
        "Suelo del sistema sobre el chasis en {} suelos de caja que solo tenían el acabado",
        thin.len()
    );
    // One wall per joint.
    let mut gone = 0;
    let mut kept = 0;
    let ids: Vec<archisim_core::BoxId> = boxes.iter().map(|b| b.id).collect();
    for id in ids {
        for issue in m.box_issues(id) {
            let archisim_core::BoxIssue::DoubleWall { side, other } = issue else {
                continue;
            };
            if other < id {
                continue; // each pair once
            }
            // The walls of each box on that joint: along it, inside the box.
            let on_side =
                |m: &Model, b: archisim_core::BoxId, side: usize| -> Vec<archisim_core::WallId> {
                    let Some(bx) = m.modular_box(b) else {
                        return Vec::new();
                    };
                    let (a, z) = bx.side(side);
                    let u = (z - a).normalized().unwrap_or_default();
                    let outline = bx.outline();
                    m.walls()
                        .filter(|w| w.level == bx.level)
                        .filter(|w| {
                            let mid = (w.start + w.end) * 0.5;
                            let d = (mid - a) - u * (mid - a).dot(u);
                            let along = (mid - a).dot(u);
                            let dir = (w.end - w.start).normalized().unwrap_or_default();
                            d.length() < 0.3
                                && along > -0.05
                                && along < a.distance(z) + 0.05
                                && dir.dot(u).abs() > 0.95
                                && point_in_polygon(mid, &outline)
                        })
                        .map(|w| w.id)
                        .collect()
                };
            let theirs_side = m
                .box_contacts(id)
                .iter()
                .find(|c| c.side == side && c.other == other)
                .map(|c| c.other_side);
            let Some(other_side) = theirs_side else {
                continue;
            };
            let mine = on_side(m, id, side);
            let theirs = on_side(m, other, other_side);
            let doors = |m: &Model, ws: &[archisim_core::WallId]| {
                ws.iter().any(|w| m.openings_in_wall(*w).next().is_some())
            };
            let (drop, b, s) = if !doors(m, &theirs) && !theirs.is_empty() {
                (theirs, other, other_side)
            } else if !doors(m, &mine) && !mine.is_empty() {
                (mine, id, side)
            } else {
                kept += 1;
                continue;
            };
            for w in drop {
                m.remove_wall(w)?;
                gone += 1;
            }
            m.edit_box(b, |x| x.sides[s] = archisim_core::SideState::Open)?;
        }
    }
    let _ = writeln!(
        t,
        "Juntas con muro en las dos cajas: quitados {gone} muros (el de la caja sin puertas en esa junta){}",
        if kept > 0 {
            format!("; {kept} se dejan (las dos tienen puertas)")
        } else {
            String::new()
        }
    );
    Ok(t)
}

/// The steel studs' thickness of each framed wall type: the thinnest of
/// 1,0, 1,2, 1,5 and 2,0 mm with which none of its walls fails the
/// calculation of the structure (wind and loads, DB-SE-A and EN 1993-1-3).
pub fn size_studs(m: &mut Model) -> Res<String> {
    use archisim_struct::lightframe::{Status, Subject};
    const STEPS: [f64; 4] = [1.0, 1.2, 1.5, 2.0];
    let mut t = String::new();
    for _round in 0..STEPS.len() {
        let a = archisim_struct::analysis::analyse(m);
        let failing: std::collections::BTreeSet<u64> = archisim_struct::all_checks(m, &a)
            .into_iter()
            .filter(|c| c.status == Status::Fail)
            .filter_map(|c| match c.subject {
                Subject::Wall(w) => m.wall(w).map(|w| w.wall_type.0),
                _ => None,
            })
            .filter(|ty| {
                m.type_framing(*ty)
                    .is_some_and(|f| f.material == archisim_core::FrameMaterial::Steel)
            })
            .collect();
        if failing.is_empty() {
            break;
        }
        let mut changed = false;
        for ty in failing {
            let Some(f) = m.type_framing(ty).cloned() else {
                continue;
            };
            let now = f.stud.thickness * 1000.0;
            let Some(next) = STEPS.iter().copied().find(|s| *s > now + 1e-6) else {
                continue;
            };
            let mm = |x: f64| x * 1000.0;
            let mut g = f.clone();
            g.stud = archisim_core::FrameSection::steel(
                mm(f.stud.depth),
                mm(f.stud.width),
                mm(f.stud.lip),
                next,
            );
            g.track = archisim_core::FrameSection::steel(
                mm(f.track.depth),
                mm(f.track.width),
                mm(f.track.lip),
                next,
            );
            m.set_type_framing(ty, Some(g))?;
            let name = m
                .wall_types()
                .find(|w| w.id.0 == ty)
                .map_or(String::new(), |w| w.name.clone());
            let _ = writeln!(
                t,
                "  Montantes de {next:.1} mm en «{name}» (con {now:.1} mm no llegaba)"
            );
            changed = true;
        }
        if !changed {
            break;
        }
    }
    Ok(t)
}

/// The project's insulating board (the core of a Tabihaus-like panel if
/// it has one, which already has its studio's layout), or else the
/// system's.
fn insulating_board(m: &mut Model) -> archisim_core::MaterialId {
    let found = m
        .materials()
        .filter(|x| {
            let n = x.name.to_lowercase();
            n.contains("tabihaus") || n.contains("placa con aislamiento")
        })
        .max_by_key(|x| {
            let n = x.name.to_lowercase();
            (n.contains("interior") || n.contains("aislamiento")) as u8
        })
        .map(|x| x.id);
    found.unwrap_or_else(|| material(m, "Placa con aislamiento", "insulation", [230, 220, 170]))
}

// --- Roofs and rain --------------------------------------------------------

fn material(m: &mut Model, name: &str, class: &str, rgb: [u8; 3]) -> archisim_core::MaterialId {
    let found = m.materials().find(|x| x.name == name).map(|x| x.id);
    found.unwrap_or_else(|| m.add_material(name, class, rgb, archisim_core::CutPattern::Solid))
}

/// The roof of each box that has none (a box put over the walls of a model
/// from another program): the system's roof (M2), 20 cm under the crown.
pub fn box_roofs(m: &mut Model) -> Res<String> {
    use archisim_core::{Layer, LayerFunction as F, SlabFunction};
    let boxes: Vec<archisim_core::ModularBox> = m.boxes().cloned().collect();
    if boxes.is_empty() {
        return Ok(String::new());
    }
    let existing = m
        .slab_types()
        .find(|t| t.name == "Cubierta de caja 180")
        .map(|t| t.id);
    let type_id = match existing {
        Some(id) => id,
        None => {
            let sheet = material(m, "Lámina impermeable", "membrane", [60, 60, 60]);
            let falls = material(m, "Mortero de pendiente", "mortar", [205, 200, 190]);
            let board = insulating_board(m);
            let deck = material(m, "Tablero de cubierta", "wood", [200, 170, 120]);
            let wool = material(m, "Lana mineral", "insulation", [235, 225, 150]);
            let l = |material, thickness, function| Layer {
                material,
                thickness,
                function,
            };
            m.add_slab_type(
                "Cubierta de caja 180",
                SlabFunction::Roof,
                vec![
                    l(sheet, 0.005, F::Membrane),
                    l(falls, 0.030, F::Substrate),
                    l(board, 0.059, F::Insulation),
                    l(deck, 0.006, F::Substrate),
                    l(wool, 0.080, F::Structure),
                ],
            )?
        }
    };
    let mut n = 0;
    for b in &boxes {
        let outline = b.outline();
        let c = outline.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * 0.25;
        let has = m.slabs().any(|s| {
            s.level == b.level
                && s.top_at(c) > 2.5
                && point_in_polygon(c, &s.outline)
                && m.slab_type(s.slab_type)
                    .is_some_and(|t| t.function == SlabFunction::Roof)
        });
        if has {
            continue;
        }
        m.fill_box(
            b.id,
            archisim_core::BoxFill {
                wall_type: None,
                floor_type: None,
                roof_type: Some(type_id),
                ceiling_type: None,
                ceiling_height: 0.0,
                parapet: archisim_core::BOX_PARAPET,
            },
        )?;
        n += 1;
    }
    Ok(format!(
        "Cubiertas: la del sistema en {n} cajas (20 cm bajo la coronación)\n"
    ))
}

/// Rain (DB-HS 5, red separativa): each open surface (a roof, a porch, a
/// terrace not covered by anything) gets the outlets of table 4.6 on its
/// edge to the outside, each to a downpipe outside the façade (or at the
/// outlet on the ground), and the downpipes' feet a buried collector to a
/// rain inspection chamber beside the waste one.
pub fn rain(m: &mut Model, level: LevelId) -> Res<String> {
    let z_level = m.level(level).map_or(0.0, |l| l.elevation);
    let lowest = m.levels().first().map(|l| l.id);
    struct Open {
        outline: Vec<Point2>,
        level: LevelId,
        top: f64,
        area: f64,
    }
    let slabs: Vec<archisim_core::Slab> = m.slabs().cloned().collect();
    let abs_top = |m: &Model, s: &archisim_core::Slab, p: Point2| {
        m.level(s.level).map_or(0.0, |l| l.elevation) + s.top_at(p)
    };
    let mut open: Vec<Open> = Vec::new();
    for s in &slabs {
        if Some(s.level) == lowest || s.outline.len() < 3 {
            continue;
        }
        let area = archisim_core::signed_area(&s.outline).abs();
        if area < 3.0 {
            continue;
        }
        let c = archisim_core::rooms::centroid(&s.outline);
        let top = abs_top(m, s, c);
        // Covered: another slab or a false ceiling over it.
        let covered = slabs.iter().any(|o| {
            o.id != s.id && abs_top(m, o, c) > top + 0.3 && point_in_polygon(c, &o.outline)
        }) || m.ceilings().any(|x| {
            m.level(x.level).map_or(0.0, |l| l.elevation) + x.height > top
                && point_in_polygon(c, &x.outline)
        }) || (top < z_level + 2.0
            && m.rooms()
                .any(|r| r.level == s.level && point_in_polygon(c, &r.outline)));
        // A pool's floor (under the ground) drains with the pool.
        if covered || top < z_level - 0.3 {
            continue;
        }
        open.push(Open {
            outline: s.outline.clone(),
            level: s.level,
            top,
            area,
        });
    }
    let inside = |m: &Model, p: Point2| {
        m.rooms()
            .any(|r| r.level == level && point_in_polygon(p, &r.outline))
            || m.boxes().any(|b| point_in_polygon(p, &b.outline()))
    };
    let mut feet: Vec<Point2> = Vec::new();
    let mut outlets = 0;
    for o in &open {
        let n_out = archisim_mep::drain::outlets_needed(o.area).max(1);
        let k = o.outline.len();
        // The edge to the outside: the longest whose outer side is clear.
        let mut edges: Vec<(Point2, Point2, Point2)> = Vec::new();
        let c = archisim_core::rooms::centroid(&o.outline);
        for i in 0..k {
            let (a, b) = (o.outline[i], o.outline[(i + 1) % k]);
            let Some(u) = (b - a).normalized() else {
                continue;
            };
            let mut out = u.perp_left();
            if ((a + b) * 0.5 + out - c).length() < ((a + b) * 0.5 - c).length() {
                out = out * -1.0;
            }
            if !inside(m, (a + b) * 0.5 + out * 0.6) {
                edges.push((a, b, out));
            }
        }
        // Not over a lower roof (a porch): the downpipe falls clear to
        // the ground; then the longest.
        let over_lower = |p: Point2| {
            slabs.iter().any(|x| {
                let top = abs_top(m, x, p);
                top > z_level + 0.5 && top < o.top - 0.1 && point_in_polygon(p, &x.outline)
            })
        };
        let Some((a, b, out)) = edges.into_iter().max_by(|x, y| {
            let key = |e: &(Point2, Point2, Point2)| {
                let clear = !over_lower((e.0 + e.1) * 0.5 + e.2 * 0.3);
                (clear as u8 as f64) * 1000.0 + e.0.distance(e.1)
            };
            key(x).total_cmp(&key(y))
        }) else {
            continue;
        };
        let ground = o.level == level && (o.top - z_level).abs() < 0.5;
        let len = a.distance(b).max(1e-6);
        for j in 0..n_out {
            // On a roof, by the corners (the downpipes in the joints
            // between boxes); on the ground, spread along.
            let f = if ground {
                (j as f64 + 0.5) / n_out as f64
            } else if n_out == 1 {
                0.5f64.min(0.4 / len)
            } else {
                let lo = (0.4 / len).min(0.5);
                lo + (1.0 - 2.0 * lo) * j as f64 / (n_out - 1) as f64
            };
            let edge = a + (b - a) * f;
            let outlet = edge - out * 0.4;
            let top_rel = o.top - m.level(o.level).map_or(0.0, |l| l.elevation);
            if ground {
                feet.push(outlet);
            } else {
                let pipe = edge + out * 0.2;
                run(
                    m,
                    o.level,
                    S::Rainwater,
                    vec![outlet, pipe],
                    top_rel - 0.04,
                    0.09,
                    0.01,
                )?;
                feet.push(pipe);
            }
            outlets += 1;
        }
    }
    if feet.is_empty() {
        return Ok(String::new());
    }
    // The chamber: beside the waste one (or by the first downpipe).
    let waste = m
        .devices()
        .find(|d| d.kind == K::Manhole)
        .map(|d| d.position);
    let chamber = waste.map_or(feet[0] + Point2::new(1.0, 1.0), |p| {
        p + Point2::new(1.2, 0.0)
    });
    let id = device(
        m,
        level,
        K::Manhole,
        "Arqueta de pluviales 60×60",
        chamber,
        0.0,
        Some(-1.2),
    )?;
    // The collector, laid out first; then deep enough for the farthest
    // downpipe's foot to stay 50 cm under the ground with its 2 % (DB-HS
    // 5, buried).
    let slope = 0.02;
    let paths = tree(&[], &[], chamber, &feet);
    let heights = |exit: f64| -> Vec<(Point2, Point2, f64, f64)> {
        let mut known: Vec<(Point2, Point2, f64, f64)> = Vec::new();
        for path in &paths {
            let end = *path.last().unwrap();
            let len: f64 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
            let z0 = z_along(&known, end, exit) + slope * len;
            let mut acc = 0.0;
            for w in path.windows(2) {
                let l = w[0].distance(w[1]);
                known.push((w[0], w[1], z0 - slope * acc, z0 - slope * (acc + l)));
                acc += l;
            }
        }
        known
    };
    let rise = heights(0.0)
        .iter()
        .map(|k| k.2.max(k.3))
        .fold(0.0, f64::max);
    let exit = (-0.5 - rise).min(-0.9);
    let known = heights(exit);
    m.edit_device(id, |d| {
        d.width = 0.6;
        d.depth = 0.6;
        d.elevation = exit - 0.3;
        d.height = -exit + 0.3;
    })?;
    let z_on = |known: &[(Point2, Point2, f64, f64)], p: Point2| z_along(known, p, exit);
    for path in &paths {
        let start = z_on(&known, path[0]);
        let (base, dz) = under(m, level);
        run(m, base, S::Rainwater, path.clone(), start + dz, 0.11, slope)?;
    }
    // The downpipes, from the collector up to their outlet.
    let mut pipes = 0;
    for (f, o) in feet.iter().zip(open.iter().flat_map(|o| {
        let n = archisim_mep::drain::outlets_needed(o.area).max(1);
        std::iter::repeat_n(o.top, n)
    })) {
        let z = z_on(&known, *f);
        let top = o - z_level;
        let (base, dz) = under(m, level);
        let id = device(
            m,
            base,
            K::Downpipe,
            "Bajante de pluviales",
            *f,
            0.0,
            Some(z + dz),
        )?;
        m.edit_device(id, |d| {
            d.width = 0.09;
            d.depth = 0.09;
            d.height = (top - z).max(0.1);
        })?;
        pipes += 1;
    }
    Ok(format!(
        "Pluviales: {} superficies al aire, {outlets} sumideros, {pipes} bajantes y colector enterrado a la arqueta de pluviales\n",
        open.len()
    ))
}

// --- Ventilation -----------------------------------------------------------

/// The spot nearest to `from` on the roof (over a box or a roof slab of
/// `level`) at least 3,2 m from every window, outside door and air inlet
/// (DB-HS 3, 3.2.1), for an outlet at height `z`.
fn clear_spot(m: &Model, level: LevelId, from: Point2, z: f64) -> Point2 {
    let on_roof = |p: Point2| {
        m.boxes().any(|b| point_in_polygon(p, &b.outline()))
            || m.slabs()
                .any(|s| s.level == level && s.top_at(p) > 2.4 && point_in_polygon(p, &s.outline))
    };
    (0..=40)
        .flat_map(|k| {
            (0..16).map(move |j| {
                let a = std::f64::consts::PI / 8.0 * f64::from(j);
                from + Point2::new(a.cos(), a.sin()) * (0.25 * f64::from(k))
            })
        })
        .find(|p| {
            on_roof(*p)
                && archisim_mep::ventilation::intake_near(m, *p, z)
                    .is_none_or(|(_, d)| d >= archisim_mep::ventilation::OUTLET_GAP + 0.2)
        })
        .unwrap_or(from)
}

pub fn ventilate(m: &mut Model, level: LevelId, roof: Option<LevelId>) -> Res<String> {
    let mut t = String::new();
    let rooms: Vec<Room> = m.rooms().filter(|r| r.level == level).cloned().collect();
    // The windows to the outside.
    let windows: Vec<(archisim_geom::OpeningGeometry, f64, Point2)> = m
        .openings()
        .filter_map(|o| {
            let ty = m.opening_type(o.opening_type)?;
            if !ty.is_window() && !ty.is_door() {
                return None;
            }
            let g = archisim_geom::opening_geometry(m, o.id)?;
            if m.wall(o.wall)?.level != level {
                return None;
            }
            let tm = (g.t0 + g.t1) / 2.0;
            let sgn = (g.exterior_offset - g.interior_offset).signum();
            let a = g.point(tm, g.exterior_offset + 0.3 * sgn);
            let b = g.point(tm, g.interior_offset - 0.3 * sgn);
            let (ra, rb) = (room_at(m, level, a), room_at(m, level, b));
            let inside = match (ra, rb) {
                (Some(_), None) => a,
                (None, Some(_)) => b,
                _ => return None,
            };
            Some((g, o.sill + ty.height, inside))
        })
        .collect();
    let mut inlets = 0;
    let mut vents: Vec<Point2> = Vec::new();
    for r in &rooms {
        let u = Use::of(r);
        if u.dry() {
            // Over a window of the room (a balcony door if there is none).
            let w = windows
                .iter()
                .filter(|(_, _, inside)| point_in_polygon(*inside, &r.outline))
                .max_by(|a, b| (a.0.t1 - a.0.t0).total_cmp(&(b.0.t1 - b.0.t0)));
            if let Some((g, head, _)) = w {
                let tm = (g.t0 + g.t1) / 2.0;
                let at = g.point(tm, (g.interior_offset + g.exterior_offset) / 2.0);
                let id = device(
                    m,
                    level,
                    K::AirInlet,
                    "Aireador higrorregulable en el marco",
                    at,
                    g.u.y.atan2(g.u.x),
                    Some((head + 0.02).min(clear_height(m, r) - 0.1)),
                )?;
                m.edit_device(id, |d| {
                    d.width = 0.4;
                    d.depth = 0.06;
                    d.height = 0.02;
                })?;
                inlets += 1;
            }
        }
        if u.wet() || (u == Use::Storage && r.area() >= 4.0) {
            // In the ceiling, at the room's wet end (its appliances).
            let here: Vec<Point2> = m
                .appliances_on(level)
                .iter()
                .filter(|a| point_in_polygon(a.point, &r.outline))
                .map(|a| a.point)
                .collect();
            let c = if here.is_empty() {
                r.label
            } else {
                here.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / here.len() as f64)
            };
            let c = if point_in_polygon(c, &r.outline) {
                c
            } else {
                r.label
            };
            let h = clear_height(m, r);
            device(
                m,
                level,
                K::ExhaustVent,
                "Boca de extracción",
                c,
                0.0,
                Some(h - 0.05),
            )?;
            vents.push(c);
        }
    }
    // The shaft and the fan: by the middle of the vents, on the roof.
    let mut ducts = 0;
    if !vents.is_empty() {
        let mid =
            vents.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / vents.len() as f64);
        let shaft_at = vents
            .iter()
            .copied()
            .min_by(|a, b| a.distance(mid).total_cmp(&b.distance(mid)))
            .unwrap_or(mid)
            + Point2::new(0.0, 0.0);
        let shaft_at = shaft_at + Point2::new(0.25, 0.25);
        let roof_z = roof.and_then(|l| m.level(l)).map_or(3.3, |l| l.elevation)
            - m.level(level).map_or(0.0, |l| l.elevation);
        // A shaft in the partition, from the floor up through the roof.
        let id = device(
            m,
            level,
            K::AirShaft,
            "Patinillo de extracción a cubierta",
            shaft_at,
            0.0,
            Some(0.0),
        )?;
        m.edit_device(id, |d| {
            d.width = 0.16;
            d.depth = 0.16;
            d.height = roof_z + 0.3;
        })?;
        for path in tree(&walls_on(m, level), &indoors(m, level), shaft_at, &vents) {
            run(m, level, S::ExhaustAir, path, DUCT_Z, 0.125, 0.0)?;
            ducts += 1;
        }
        if let Some(roof) = roof {
            // The fan on the roof, clear of windows and inlets.
            let roof_abs = m.level(roof).map_or(3.3, |l| l.elevation);
            let fan = clear_spot(m, level, shaft_at, roof_abs + 0.5);
            let id = device(
                m,
                roof,
                K::Fan,
                "Extractor de VMC higrorregulable",
                fan,
                0.0,
                Some(0.0),
            )?;
            m.edit_device(id, |d| {
                d.width = 0.5;
                d.depth = 0.5;
                d.height = 0.45;
            })?;
            run(m, roof, S::ExhaustAir, vec![shaft_at, fan], 0.25, 0.16, 0.0)?;
        }
    }
    // Each kitchen hood: its own duct to the roof.
    let hoods: Vec<(Point2, f64)> = m
        .objects()
        .filter(|o| o.level == level)
        .filter(|o| {
            m.object_type(o.object_type)
                .is_some_and(|t| t.name.to_lowercase().contains("campana"))
        })
        .map(|o| (o.position, o.rotation))
        .collect();
    for (at, turn) in &hoods {
        let fwd = Point2::new(turn.cos(), turn.sin());
        let shaft = *at + fwd * 0.45;
        let roof_z = roof.and_then(|l| m.level(l)).map_or(3.3, |l| l.elevation);
        let base = m.level(level).map_or(0.0, |l| l.elevation);
        let id = device(
            m,
            level,
            K::AirShaft,
            "Conducto de la campana",
            shaft,
            0.0,
            Some(0.0),
        )?;
        m.edit_device(id, |d| {
            d.width = 0.15;
            d.depth = 0.15;
            d.height = roof_z - base + 0.5;
        })?;
        run(m, level, S::ExhaustAir, vec![*at, shaft], DUCT_Z, 0.15, 0.0)?;
        if let Some(roof) = roof {
            let roof_abs = m.level(roof).map_or(3.3, |l| l.elevation);
            let out = clear_spot(m, level, shaft, roof_abs + 0.4);
            let out = if out.distance(shaft) < 0.3 {
                shaft + Point2::new(0.0, 0.6)
            } else {
                out
            };
            run(m, roof, S::ExhaustAir, vec![shaft, out], 0.4, 0.15, 0.0)?;
        }
        ducts += 1;
    }
    let _ = writeln!(
        t,
        "Ventilación: {inlets} aireadores, {} bocas de extracción, {ducts} conductos por el falso techo, extractor en cubierta y {} campana{} con conducto propio",
        vents.len(),
        hoods.len(),
        if hoods.len() == 1 { "" } else { "s" }
    );
    Ok(t)
}

// --- Porch ----------------------------------------------------------------

/// A light steel porch (M17) on a side of a box, by its mark, the
/// cardinal side (norte, sur, este, oeste), its depth and its length
/// (running on over the boxes beside it, `docs/MODULAR.md` §5.3), in
/// place of what stood there: the slabs and columns over 1 m above the
/// box's level inside its outline (the concrete porch of a model from
/// another program).
pub fn porch(m: &mut Model, spec: &str) -> Res<String> {
    let f: Vec<&str> = spec.split(':').map(str::trim).collect();
    let num = |s: &str| s.replace(',', ".").parse::<f64>().ok();
    let (Some(mark), Some(side), Some(depth)) =
        (f.first(), f.get(1), f.get(2).and_then(|s| num(s)))
    else {
        return Err(format!("«{spec}»: marca:lado:fondo[:largo]").into());
    };
    let b = m
        .boxes()
        .find(|b| b.mark == *mark)
        .cloned()
        .ok_or_else(|| format!("no hay ninguna caja {mark}"))?;
    let want = match side.to_lowercase().as_str() {
        "norte" | "n" => Point2::new(0.0, 1.0),
        "sur" | "s" => Point2::new(0.0, -1.0),
        "este" | "e" => Point2::new(1.0, 0.0),
        "oeste" | "o" | "w" => Point2::new(-1.0, 0.0),
        _ => return Err(format!("«{side}»: norte, sur, este u oeste").into()),
    };
    let centre = b
        .outline()
        .iter()
        .fold(Point2::new(0.0, 0.0), |s, p| s + *p)
        * 0.25;
    let side_i = (0..4)
        .max_by(|i, j| {
            let out = |k: usize| {
                let (a, z) = b.side(k);
                ((a + z) * 0.5 - centre)
                    .normalized()
                    .unwrap_or_default()
                    .dot(want)
            };
            out(*i).total_cmp(&out(*j))
        })
        .unwrap_or(0);
    let (a, z) = b.side(side_i);
    let len = a.distance(z);
    let run = f.get(3).and_then(|s| num(s)).unwrap_or(len);
    // Over the boxes beside it: the way towards the middle of all.
    let all: Vec<Point2> = m.boxes().flat_map(|x| x.outline()).collect();
    let middle =
        all.iter().fold(Point2::new(0.0, 0.0), |s, p| s + *p) * (1.0 / all.len().max(1) as f64);
    let u = (z - a).normalized().unwrap_or_default();
    let forwards = run <= len + 1e-6
        || (a + u * (run / 2.0)).distance(middle) <= (a + u * (len - run / 2.0)).distance(middle);
    let (from, to) = if forwards {
        (0.0, run)
    } else {
        (len - run, len)
    };

    // Taken away first: what stood there.
    let n = (z - a).perp_left();
    let n = if ((a + z) * 0.5 + n - centre).length() > ((a + z) * 0.5 - centre).length() {
        n
    } else {
        n * -1.0
    };
    let n = n.normalized().unwrap_or_default();
    let area = [
        a + u * from,
        a + u * to,
        a + u * to + n * (depth + 0.3),
        a + u * from + n * (depth + 0.3),
    ];
    let z0 = m.level(b.level).map_or(0.0, |l| l.elevation);
    let mut gone = 0;
    let slabs: Vec<_> = m
        .slabs()
        .filter(|s| {
            m.level(s.level).is_some_and(|l| l.elevation > z0 + 1.0)
                && point_in_polygon(archisim_core::rooms::centroid(&s.outline), &area)
        })
        .map(|s| s.id)
        .collect();
    for id in slabs {
        m.remove_slab(id)?;
        gone += 1;
    }
    let made = m.build_porch(
        b.id,
        &archisim_core::PorchOptions {
            side: side_i,
            depth,
            from,
            to,
            run_on: true,
            // Over 7 m: deeper beams (the return carries the front).
            beam_depth: 0.20,
            beam_width: 0.10,
            beam_wall: 0.006,
            ..Default::default()
        },
    )?;
    Ok(format!(
        "Porche de {:.2} × {depth:.2} m en {mark} ({side}): {} pilares, {} vigas, {:.0} kg de acero; quitado lo que había: {gone} elementos\n",
        to - from,
        made.posts.len(),
        made.beams.len(),
        made.steel
    ))
}

/// The level with most rooms (or walls).
pub fn main_level(m: &Model) -> Option<LevelId> {
    m.levels()
        .iter()
        .max_by_key(|l| {
            (
                m.rooms().filter(|r| r.level == l.id).count(),
                m.walls_on_level(l.id).count(),
            )
        })
        .map(|l| l.id)
}

/// The level of the roof over `level`: the first one at or over the top of
/// the roofs on it (the boxes' roofs), or else the first 2,4 m over it.
pub fn level_over(m: &Model, level: LevelId) -> Option<LevelId> {
    let z = m.level(level)?.elevation;
    let roof_top = m
        .slabs()
        .filter(|s| s.level == level)
        .filter(|s| {
            m.slab_type(s.slab_type)
                .is_some_and(|t| t.function == archisim_core::SlabFunction::Roof)
        })
        .map(|s| z + s.top_at(archisim_core::rooms::centroid(&s.outline)))
        .reduce(f64::max);
    if let Some(top) = roof_top {
        if let Some(l) = m
            .levels()
            .iter()
            .filter(|l| l.elevation >= top - 0.05)
            .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
        {
            return Some(l.id);
        }
    }
    m.levels()
        .iter()
        .filter(|l| l.elevation > z + 2.4)
        .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
        .map(|l| l.id)
}

/// Everything: objects of the list, electricity, water and drainage,
/// ventilation; then the sizes of the calculations.
pub fn equip(
    m: &mut Model,
    objects: Option<&str>,
    collection: Option<&str>,
    porches: &[String],
    n50: Option<f64>,
) -> Res<String> {
    let level = main_level(m).ok_or("el proyecto no tiene plantas")?;
    if m.rooms().all(|r| r.level != level) {
        return Err("la planta no tiene locales: ponlos primero (archisim locales)".into());
    }
    let mut t = String::new();
    if let Some(list) = objects {
        t += &place_objects(m, level, list)?;
    }
    if let Some(name) = collection {
        let c = archisim_library::collections::COLLECTIONS
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("no hay ninguna colección «{name}»"))?;
        let ids: Vec<archisim_core::ObjectId> = m.objects().map(|o| o.id).collect();
        let n = archisim_library::collections::apply_collection(m, &ids, Some(c))?;
        let _ = writeln!(t, "Colección «{}» en {n} objetos", c.name);
    }
    t += &system_rules(m)?;
    for p in porches {
        t += &porch(m, p)?;
    }
    t += &box_roofs(m)?;
    let roof = level_over(m, level);
    t += &plumb(m, level)?;
    t += &electrify(m, level)?;
    t += &ventilate(m, level, roof)?;
    t += &rain(m, level)?;
    t += &size_studs(m)?;
    // The studio's layout on the boards added (M18).
    archisim_library::modular_board_layouts(m);
    // Sizes of the calculations.
    let w = archisim_mep::water::water(m);
    let mut n = 0;
    for (r, d) in w.sized_runs() {
        m.edit_run(r, |x| x.width = d)?;
        n += 1;
    }
    let d = archisim_mep::drain::drainage(m);
    let (runs, pipes) = d.sized_runs();
    for (r, w) in runs {
        m.edit_run(r, |x| x.width = x.width.max(w))?;
        n += 1;
    }
    for (p, w) in pipes {
        m.edit_device(p, |x| {
            x.width = x.width.max(w);
            x.depth = x.depth.max(w);
        })?;
    }
    let v = archisim_mep::ventilation::ventilation(m);
    for (r, w) in v.sized_runs(m) {
        m.edit_run(r, |x| x.width = x.width.max(w))?;
        n += 1;
    }
    for (dv, w, dp) in v.sized_devices(m) {
        m.edit_device(dv, |x| {
            x.width = w;
            x.depth = dp;
        })?;
    }
    let _ = writeln!(
        t,
        "Tamaños de los cálculos de fontanería, saneamiento y ventilación puestos en {n} tramos"
    );
    // One dwelling on its own: a single-family house (DB-SI, the
    // structure's fire resistance).
    if archisim_mep::ventilation::ventilation(m).dwellings.len() == 1
        && !m.structural_site().single_house
    {
        let mut site = *m.structural_site();
        site.single_house = true;
        m.set_structural_site(site)?;
        let _ = writeln!(t, "Vivienda unifamiliar: resistencia al fuego de la estructura de vivienda unifamiliar (DB-SI)");
    }
    if let Some(v) = n50 {
        let mut site = m.thermal_site().clone();
        site.n50 = Some(v);
        m.set_thermal_site(site)?;
        let _ = writeln!(t, "Estanqueidad de proyecto: n50 = {v:.1} h⁻¹ (a comprobar con el ensayo de presurización)");
    }
    Ok(t)
}

pub fn main(args: &[String]) -> Res {
    let mut objects = None;
    let mut collection = None;
    let mut porches = Vec::new();
    let mut n50 = None;
    let mut plain = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--objetos" => {
                objects = args.get(i + 1).cloned();
                i += 2;
            }
            "--porche" => {
                if let Some(p) = args.get(i + 1) {
                    porches.push(p.clone());
                }
                i += 2;
            }
            "--n50" => {
                n50 = args
                    .get(i + 1)
                    .and_then(|v| v.replace(',', ".").parse::<f64>().ok());
                i += 2;
            }
            "--coleccion" => {
                collection = args.get(i + 1).cloned();
                i += 2;
            }
            _ => {
                plain.push(args[i].clone());
                i += 1;
            }
        }
    }
    let (Some(src), Some(out)) = (plain.first(), plain.get(1)) else {
        return Err("Uso: archisim equipar <proyecto> <salida.archisim> [--objetos lista.csv] [--coleccion «Nórdico cálido»] [--porche MOD-07:sur:3,0:7,2] [--n50 3]".into());
    };
    let mut m = archisim_file::open(Path::new(src))?;
    let list = objects.map(std::fs::read_to_string).transpose()?;
    let t = equip(
        &mut m,
        list.as_deref(),
        collection.as_deref(),
        &porches,
        n50,
    )?;
    print!("{t}");
    archisim_file::save(&m, Path::new(out))?;
    println!("Guardado: {out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{Hand, NewOpening, NewRoom, NewWall, ReferenceLine, WallSide, WallTop};

    /// A one-storey house of 8 × 6 with a bathroom: its walls, a front
    /// door, a window, two rooms and the bathroom's appliances.
    fn house() -> Model {
        let mut m = Model::new("Casa");
        let pb = m.add_level("Planta baja", 0.0);
        m.add_level("Cubierta", 2.8);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let p = |x: f64, y: f64| Point2::new(x, y);
        let c = [p(0.0, 0.0), p(8.0, 0.0), p(8.0, 6.0), p(0.0, 6.0)];
        let mut walls = Vec::new();
        for k in 0..4 {
            walls.push(
                m.add_wall(NewWall {
                    wall_type: lib.facade,
                    level: pb,
                    start: c[k],
                    end: c[(k + 1) % 4],
                    reference: ReferenceLine::Center,
                    height: 2.8,
                    base_offset: 0.0,
                    bulge: 0.0,
                    top: WallTop::Unconnected,
                })
                .unwrap(),
            );
        }
        // The bathroom: 2,5 × 2,5 in the north-east corner.
        for (a, b) in [(p(5.5, 6.0), p(5.5, 3.5)), (p(5.5, 3.5), p(8.0, 3.5))] {
            m.add_wall(NewWall {
                wall_type: lib.partition,
                level: pb,
                start: a,
                end: b,
                reference: ReferenceLine::Center,
                height: 2.6,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        let door = |m: &mut Model, w, t, along, sill| {
            m.add_opening(NewOpening {
                opening_type: t,
                wall: w,
                along,
                sill,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
        };
        door(&mut m, walls[0], lib.entrance_door, 2.0, 0.0);
        door(&mut m, walls[3], lib.window_two_leaves, 3.0, 0.9);
        let inner = m.walls().last().map(|w| w.id).unwrap();
        door(&mut m, inner, lib.interior_door, 1.0, 0.0);
        for (name, usage, pts) in [
            (
                "Salón",
                "Estancia",
                vec![
                    p(0.1, 0.1),
                    p(7.9, 0.1),
                    p(7.9, 3.4),
                    p(5.4, 3.4),
                    p(5.4, 5.9),
                    p(0.1, 5.9),
                ],
            ),
            (
                "Baño",
                "Baño",
                vec![p(5.6, 3.6), p(7.9, 3.6), p(7.9, 5.9), p(5.6, 5.9)],
            ),
        ] {
            m.add_room(NewRoom {
                level: pb,
                number: String::new(),
                name: name.into(),
                outline: pts,
                holes: Vec::new(),
                label: if name == "Baño" {
                    p(6.7, 4.7)
                } else {
                    p(2.5, 2.5)
                },
                bounded: false,
            })
            .unwrap();
            let mut r = m.rooms().last().cloned().unwrap();
            r.usage = usage.into();
            m.set_room(r).unwrap();
        }
        m
    }

    #[test]
    fn a_house_gets_its_installations_joined_up() {
        let mut m = house();
        let list = "Inodoro; 6.2; 5.9; 180\nLavabo 60; 7.2; 5.9; 180\nPlato de ducha 90×90; 7.9; 4.2; 90\n";
        let report = equip(&mut m, Some(list), None, &[], None).unwrap();
        assert!(report.contains("Objetos colocados: 3"), "{report}");
        let has = |k: K| m.devices().any(|d| d.kind == k);
        for k in [
            K::Panel,
            K::WaterMeter,
            K::WaterHeater,
            K::Manhole,
            K::Downpipe,
            K::AirValve,
            K::ExhaustVent,
            K::AirInlet,
            K::Light,
            K::Socket,
        ] {
            assert!(has(k), "{k:?}: {report}");
        }
        // Every appliance: cold water reaching it, and a drain from it.
        let level = m.levels()[0].id;
        for a in m.appliances_on(level) {
            for system in [S::ColdWater, S::Drainage] {
                assert!(
                    m.runs()
                        .any(|r| r.system == system
                            && r.path.iter().any(|q| q.distance(a.point) < 0.05)),
                    "{:?} without {system:?}",
                    a.kind
                );
            }
        }
        // The drains: each branch within the rule's reach of its downpipe.
        let v = archisim_mep::drain_vent::venting(&m, &archisim_mep::drain::drainage(&m));
        assert!(
            !v.ways.is_empty() && v.ways.iter().all(|w| !w.too_far() && !w.bad_slope()),
            "{:?}",
            v.ways
        );
    }

    #[test]
    fn rooms_are_known_by_their_name_and_use() {
        let mut r = house().rooms().next().cloned().unwrap();
        r.name = "Salón-comedor-cocina".into();
        assert_eq!(Use::of(&r), Use::LivingKitchen);
        r.name = "Armario D1".into();
        r.usage = "Almacén".into();
        assert_eq!(Use::of(&r), Use::Storage);
        assert!(Use::Bath.wet() && !Use::Bath.dry() && Use::Bedroom.dry());
    }
}
