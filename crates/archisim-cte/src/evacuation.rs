//! Evacuation (CTE DB-SI 3): exits of every level, the length of the
//! routes to them, and the doors and corridors the occupants go through.
//!
//! Each level is a graph: the doors (and passages) join the rooms on either
//! side; on the entrance level a door to the outside is an exit of the
//! building, and on the other levels the stairs that lead towards it are
//! the exits of the level. Distances inside a room are walked round its
//! corners ([`Region::paths`]).

use std::collections::BTreeMap;

use archisim_core::{
    BuildingUse, DoorOperation, LevelId, Model, OpeningId, OpeningKind, Point2, RoomId, StairId,
    Vec2,
};

use crate::occupancy::{is_common, is_corridor, room_text};
use crate::region::Region;
use crate::{m, Item, Report, Section, Status, Subject};

/// Leaves are taken as 4 cm thick when measuring the clear width with the
/// leaf open (SUA, Anejo A: «descontando el grosor de la hoja»).
pub const LEAF_THICKNESS: f64 = 0.04;

/// A door or passage people can walk through.
#[derive(Debug, Clone)]
pub struct Door {
    pub id: OpeningId,
    pub level: LevelId,
    /// Points just inside each face: `[exterior, interior]` of its wall.
    pub faces: [Point2; 2],
    /// The room on each side, `None` outside every room.
    pub rooms: [Option<RoomId>; 2],
    /// Wall thickness, walked through.
    pub thickness: f64,
    /// Rough opening less the frames: «anchura libre de paso».
    pub clear: f64,
    /// Width of one leaf (the clear width for one leaf).
    pub leaf: f64,
    /// Hinged leaves: the direction they open to (out of the wall).
    pub swing: Option<Vec2>,
    /// Sliding or folding (not hinged on a vertical axis).
    pub sliding: bool,
    /// Normal of the wall towards side 0.
    pub n: Vec2,
}

impl Door {
    /// Whether going from side `from` to the other one goes the way the
    /// leaves open.
    pub fn opens_towards(&self, from: usize) -> Option<bool> {
        let dir = if from == 1 { self.n } else { -self.n };
        self.swing.map(|s| s.dot(dir) > 0.0)
    }
}

/// The doors and passages at floor level of the model.
pub fn doors(model: &Model) -> Vec<Door> {
    let mut out = Vec::new();
    for o in model.openings() {
        let Some(t) = model.opening_type(o.opening_type) else {
            continue;
        };
        if o.sill > 0.05 {
            continue;
        }
        let Some(g) = archisim_geom::opening_geometry(model, o.id) else {
            continue;
        };
        let Some(wall) = model.wall(o.wall) else {
            continue;
        };
        if matches!(t.kind, OpeningKind::Window(_)) && t.height < 1.9 {
            continue;
        }
        let tm = (g.t0 + g.t1) / 2.0;
        let sign = if g.exterior_offset >= g.interior_offset {
            1.0
        } else {
            -1.0
        };
        let faces = [
            g.point(tm, g.exterior_offset + sign * 0.02),
            g.point(tm, g.interior_offset - sign * 0.02),
        ];
        let probe = [
            g.point(tm, g.exterior_offset + sign * 0.25),
            g.point(tm, g.interior_offset - sign * 0.25),
        ];
        let room_at = |p: Point2| {
            model
                .rooms_on_level(wall.level)
                .find(|r| r.contains(p))
                .map(|r| r.id)
        };
        let rooms = [room_at(probe[0]), room_at(probe[1])];
        // The clear width the family gives (without sidelights, or the
        // short side of a pivot door), or the opening less its jambs.
        let clear_of = |t: &archisim_core::OpeningType| {
            model
                .opening_clear_width(t.id)
                .unwrap_or(t.width - 2.0 * t.frame_width)
        };
        let (clear, leaf, hinged, sliding) = match t.kind {
            OpeningKind::Void => (t.width, t.width, false, false),
            OpeningKind::Door(DoorOperation::DoubleSwing) => {
                let c = clear_of(t);
                (c, c / 2.0, true, false)
            }
            OpeningKind::Door(DoorOperation::SingleSwing | DoorOperation::Pivot) => {
                let c = clear_of(t);
                (c, c, true, false)
            }
            OpeningKind::Door(_) => {
                let c = clear_of(t);
                (c, c, false, true)
            }
            OpeningKind::Window(_) => {
                let c = t.width - 2.0 * t.frame_width;
                let leaves = t.panels.len().max(1) as f64;
                (c, c / leaves, true, false)
            }
        };
        out.push(Door {
            id: o.id,
            level: wall.level,
            faces,
            rooms,
            thickness: (g.exterior_offset - g.interior_offset).abs(),
            clear,
            leaf,
            swing: hinged.then(|| g.swing_dir()),
            sliding,
            n: g.n * sign,
        });
    }
    out
}

/// The level people leave the building from: the one nearest the ±0,00.
pub fn entrance_level(model: &Model) -> Option<LevelId> {
    let with_rooms: Vec<_> = model
        .levels()
        .into_iter()
        .filter(|l| model.rooms_on_level(l.id).next().is_some())
        .collect();
    with_rooms
        .iter()
        .min_by(|a, b| {
            a.elevation
                .abs()
                .total_cmp(&b.elevation.abs())
                .then(a.elevation.total_cmp(&b.elevation))
        })
        .map(|l| l.id)
}

/// An exit of a level.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Exit {
    /// A door to the outside (its id) on the entrance level.
    Outside(OpeningId),
    /// The head or foot of a stair towards the entrance level.
    Stair(StairId, Point2),
}

/// A node of a level's graph: one side of a door, or an exit point of a
/// stair, in a room.
#[derive(Debug, Clone, Copy)]
struct Node {
    room: RoomId,
    at: Point2,
    /// Door and side, or `None` for a stair's point.
    door: Option<(usize, usize)>,
}

/// The exits of a level that stairs give: their point and room.
fn stair_exits(model: &Model, level: LevelId, entrance: LevelId) -> Vec<(StairId, Point2)> {
    let (Some(here), Some(entry)) = (model.level(level), model.level(entrance)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for s in model.stairs() {
        let Some(base) = model.level(s.level) else {
            continue;
        };
        let foot = base.elevation + s.base_offset;
        let head = foot + s.height;
        let (Some(first), Some(last)) = (s.foot(), s.head()) else {
            continue;
        };
        let (first, last) = (&first, &last);
        if here.elevation > entry.elevation && (head - here.elevation).abs() < 0.3 {
            // Going down from this level.
            out.push((s.id, *last));
        } else if here.elevation < entry.elevation && (foot - here.elevation).abs() < 0.3 {
            out.push((s.id, *first));
        }
    }
    out
}

/// The room a point is in, or the nearest one within 1,5 m (a stair's
/// head is usually in the stairwell, beside the landing), with the point
/// to walk to.
fn room_near(model: &Model, level: LevelId, p: Point2) -> Option<(RoomId, Point2)> {
    let mut best: Option<(RoomId, Point2, f64)> = None;
    for r in model.rooms_on_level(level) {
        if r.contains(p) {
            return Some((r.id, p));
        }
        let reg = Region::new(r.outline.clone(), r.holes.clone());
        for (a, b) in reg.edges() {
            let ab = b - a;
            let len2 = ab.dot(ab);
            if len2 < 1e-12 {
                continue;
            }
            let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
            let q = a + ab * t;
            let d = q.distance(p);
            if d <= 1.5 && best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((r.id, q, d));
            }
        }
    }
    best.map(|(r, q, _)| (r, q))
}

/// The result of a level's graph.
struct LevelGraph {
    nodes: Vec<Node>,
    /// Distance from each node to the nearest exit, and the next node on
    /// the way (`None` at an exit) with the path to it.
    dist: Vec<f64>,
    next: Vec<Option<(usize, Vec<Point2>)>>,
    /// The exit each node leads to.
    exit_of: Vec<Option<usize>>,
    exits: Vec<Exit>,
    /// Nodes of each room.
    by_room: BTreeMap<RoomId, Vec<usize>>,
}

fn region_of(model: &Model, r: RoomId) -> Option<Region> {
    model
        .room(r)
        .map(|r| Region::new(r.outline.clone(), r.holes.clone()))
}

fn level_graph(model: &Model, level: LevelId, doors: &[Door], entrance: LevelId) -> LevelGraph {
    let mut nodes = Vec::new();
    let mut exits = Vec::new();
    let mut exit_nodes: Vec<(usize, usize)> = Vec::new();
    for (i, d) in doors.iter().enumerate() {
        if d.level != level {
            continue;
        }
        for side in 0..2 {
            if let Some(room) = d.rooms[side] {
                nodes.push(Node {
                    room,
                    at: d.faces[side],
                    door: Some((i, side)),
                });
                if level == entrance && d.rooms[1 - side].is_none() {
                    exits.push(Exit::Outside(d.id));
                    exit_nodes.push((nodes.len() - 1, exits.len() - 1));
                }
            }
        }
    }
    if level != entrance {
        for (s, p) in stair_exits(model, level, entrance) {
            if let Some((room, q)) = room_near(model, level, p) {
                nodes.push(Node {
                    room,
                    at: q,
                    door: None,
                });
                exits.push(Exit::Stair(s, p));
                exit_nodes.push((nodes.len() - 1, exits.len() - 1));
            }
        }
    }
    let mut by_room: BTreeMap<RoomId, Vec<usize>> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        by_room.entry(n.room).or_default().push(i);
    }
    // Edges: inside each room between its nodes, and through each door.
    let mut edges: Vec<Vec<(usize, f64, Vec<Point2>)>> = vec![Vec::new(); nodes.len()];
    for (room, list) in &by_room {
        let Some(reg) = region_of(model, *room) else {
            continue;
        };
        for &a in list {
            let targets: Vec<Point2> = list.iter().map(|&b| nodes[b].at).collect();
            for (k, (d, path)) in reg.paths(nodes[a].at, &targets).into_iter().enumerate() {
                let b = list[k];
                if b != a && d.is_finite() {
                    edges[a].push((b, d, path));
                }
            }
        }
    }
    let mut side_node: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Some(ds) = n.door {
            side_node.insert(ds, i);
        }
    }
    for (&(door, side), &a) in &side_node {
        if let Some(&b) = side_node.get(&(door, 1 - side)) {
            let t = doors[door].thickness + 0.04;
            edges[a].push((b, t, vec![nodes[a].at, nodes[b].at]));
        }
    }
    // Dijkstra from the exits (edges are symmetric).
    let n = nodes.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut next: Vec<Option<(usize, Vec<Point2>)>> = vec![None; n];
    let mut exit_of = vec![None; n];
    let mut done = vec![false; n];
    for &(node, exit) in &exit_nodes {
        dist[node] = 0.0;
        exit_of[node] = Some(exit);
    }
    for _ in 0..n {
        let Some(u) = (0..n)
            .filter(|&i| !done[i] && dist[i].is_finite())
            .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
        else {
            break;
        };
        done[u] = true;
        for (v, w, path) in edges[u].clone() {
            let d = dist[u] + w;
            if d < dist[v] - 1e-9 {
                dist[v] = d;
                let mut back = path;
                back.reverse();
                next[v] = Some((u, back));
                exit_of[v] = exit_of[u];
            }
        }
    }
    LevelGraph {
        nodes,
        dist,
        next,
        exit_of,
        exits,
        by_room,
    }
}

impl LevelGraph {
    /// From a point of a room: distance to an exit, the route, the exit
    /// and the nodes walked through.
    fn route_from(
        &self,
        reg: &Region,
        room: RoomId,
        p: Point2,
    ) -> Option<(f64, Vec<Point2>, usize, Vec<usize>)> {
        let list = self.by_room.get(&room)?;
        let targets: Vec<Point2> = list.iter().map(|&i| self.nodes[i].at).collect();
        let (k, (d0, path)) = reg
            .paths(p, &targets)
            .into_iter()
            .enumerate()
            .filter(|(k, (d, _))| d.is_finite() && self.dist[list[*k]].is_finite())
            .min_by(|(ka, (da, _)), (kb, (db, _))| {
                (da + self.dist[list[*ka]]).total_cmp(&(db + self.dist[list[*kb]]))
            })?;
        let mut node = list[k];
        let total = d0 + self.dist[node];
        let mut route = path;
        let mut visited = vec![node];
        while let Some((nxt, seg)) = &self.next[node] {
            route.extend(seg.iter().skip(1));
            node = *nxt;
            visited.push(node);
            if visited.len() > self.nodes.len() + 1 {
                break;
            }
        }
        Some((total, route, self.exit_of[node]?, visited))
    }
}

/// Maximum length of the routes (SI 3, table 3.1).
fn route_limit(use_: BuildingUse, exits: usize, sprinklers: bool) -> f64 {
    let base = if exits <= 1 {
        if use_ == BuildingUse::Parking {
            35.0
        } else {
            25.0
        }
    } else if matches!(use_, BuildingUse::PublicResidential | BuildingUse::Hospital) {
        // Zones where people sleep.
        35.0
    } else {
        50.0
    };
    if sprinklers {
        base * 1.25
    } else {
        base
    }
}

/// People through each door, and the most through each door from one
/// room.
#[derive(Default)]
struct Flows {
    through: BTreeMap<usize, f64>,
    /// Door → (from side, people of its own room going through it).
    own_room: BTreeMap<usize, (usize, f64)>,
    /// Door → the side people enter it from (the way of evacuation).
    from_side: BTreeMap<usize, usize>,
}

/// Every check of DB-SI 3 on the model.
pub fn check(model: &Model, report: &mut Report) {
    let settings = model.code_settings();
    let use_ = settings.building_use;
    let Some(entrance) = entrance_level(model) else {
        return;
    };
    let doors = doors(model);
    let dwelling = use_ == BuildingUse::Dwelling;
    let rooms: BTreeMap<RoomId, (f64, bool)> = report
        .rooms
        .iter()
        .map(|r| {
            let common = model.room(r.room).is_some_and(|x| is_common(&room_text(x)));
            (r.room, (r.people, common))
        })
        .collect();
    let any_common = rooms.values().any(|(_, c)| *c);
    if dwelling && !any_common {
        report.items.push(Item {
            section: Section::Evacuation,
            status: Status::Info,
            subject: Subject::Building,
            level: None,
            rule: "SI 3 (Anejo SI A, origen de evacuación)",
            text: "Residencial Vivienda sin zonas comunes: los recorridos de evacuación se cuentan desde la puerta de cada vivienda, así que no hay recorridos que comprobar dentro de ella. Si hay zonas comunes, ponles el uso «Zona común», «Portal» o «Rellano».".into(),
            value: String::new(),
            required: String::new(),
            route: Vec::new(),
        });
    }
    let building_people: f64 = rooms.values().map(|(p, _)| p).sum();
    let mut flows = Flows::default();
    for l in model.levels() {
        let level = l.id;
        let level_rooms: Vec<RoomId> = model.rooms_on_level(level).map(|r| r.id).collect();
        if level_rooms.is_empty() {
            continue;
        }
        let g = level_graph(model, level, &doors, entrance);
        // Origins: every point of every occupied room, or in a dwelling
        // building the dwellings' doors to the common zones.
        let mut worst: Option<(f64, Vec<Point2>, RoomId)> = None;
        let mut used_exits: Vec<usize> = Vec::new();
        let mut unreachable: Vec<RoomId> = Vec::new();
        let mut level_people = 0.0;
        for &rid in &level_rooms {
            let Some(room) = model.room(rid) else {
                continue;
            };
            let (people, common) = rooms.get(&rid).copied().unwrap_or((0.0, false));
            level_people += people;
            let reg = Region::new(room.outline.clone(), room.holes.clone());
            let origins: Vec<Point2> = if dwelling {
                if !common {
                    continue;
                }
                // The doors of the dwellings opening onto this zone.
                doors
                    .iter()
                    .filter(|d| d.level == level)
                    .filter_map(|d| {
                        let side = d.rooms.iter().position(|r| *r == Some(rid))?;
                        let other = d.rooms[1 - side]?;
                        let other_common = rooms.get(&other).is_some_and(|(_, c)| *c);
                        (!other_common).then_some(d.faces[side])
                    })
                    .collect()
            } else if people > 0.0 {
                reg.sample_points(1.0)
            } else {
                Vec::new()
            };
            if origins.is_empty() {
                continue;
            }
            let mut room_worst: Option<(f64, Vec<Point2>)> = None;
            for p in origins {
                match g.route_from(&reg, rid, p) {
                    Some((d, route, exit, _)) => {
                        if !used_exits.contains(&exit) {
                            used_exits.push(exit);
                        }
                        if room_worst.as_ref().is_none_or(|(w, _)| d > *w) {
                            room_worst = Some((d, route));
                        }
                    }
                    None => {
                        if !unreachable.contains(&rid) {
                            unreachable.push(rid);
                        }
                    }
                }
            }
            if let Some((d, route)) = room_worst {
                if worst.as_ref().is_none_or(|(w, _, _)| d > *w) {
                    worst = Some((d, route, rid));
                }
            }
            // Where this room's people go: from its label point.
            if !dwelling && people > 0.0 {
                let from = if room.contains(room.label) {
                    room.label
                } else {
                    archisim_core::label_center(&room.outline)
                };
                if let Some((_, _, _, visited)) = g.route_from(&reg, rid, from) {
                    let mut first = true;
                    for w in visited.windows(2) {
                        let (a, b) = (g.nodes[w[0]], g.nodes[w[1]]);
                        if let (Some((da, sa)), Some((db, _))) = (a.door, b.door) {
                            if da == db {
                                *flows.through.entry(da).or_insert(0.0) += people;
                                flows.from_side.entry(da).or_insert(sa);
                                if first {
                                    let e = flows.own_room.entry(da).or_insert((sa, 0.0));
                                    if people > e.1 {
                                        *e = (sa, people);
                                    }
                                }
                                first = false;
                            }
                        }
                    }
                    // Leaving through an outside door.
                    if let Some(&last) = visited.last() {
                        if let Some((d, s)) = g.nodes[last].door {
                            *flows.through.entry(d).or_insert(0.0) += people;
                            flows.from_side.entry(d).or_insert(s);
                            if visited.len() == 1 {
                                let e = flows.own_room.entry(d).or_insert((s, 0.0));
                                if people > e.1 {
                                    *e = (s, people);
                                }
                            }
                        }
                    }
                }
            }
        }
        let exits = g.exits.len();
        for rid in &unreachable {
            report.items.push(Item {
                section: Section::Evacuation,
                status: Status::Fails,
                subject: Subject::Room(*rid),
                level: Some(level),
                rule: "SI 3, 3",
                text: if exits == 0 {
                    format!(
                        "La planta {} no tiene salida: ni puerta al exterior (en la planta de acceso) ni escalera hacia ella",
                        l.name
                    )
                } else {
                    "No hay recorrido desde este local hasta una salida: falta una puerta o un paso".into()
                },
                value: String::new(),
                required: String::new(),
                route: Vec::new(),
            });
        }
        let Some((longest, route, rid)) = worst else {
            continue;
        };
        let exits_used = used_exits.len().max(1);
        let limit = route_limit(use_, exits_used, settings.sprinklers);
        let exit_names: Vec<String> = used_exits
            .iter()
            .map(|&e| match g.exits[e] {
                Exit::Outside(o) => crate::subject_name(model, Subject::Opening(o)),
                Exit::Stair(s, _) => crate::subject_name(model, Subject::Stair(s)),
            })
            .collect();
        report.items.push(Item {
            section: Section::Evacuation,
            status: if longest <= limit + 1e-6 {
                Status::Passes
            } else {
                Status::Fails
            },
            subject: Subject::Room(rid),
            level: Some(level),
            rule: "SI 3, tabla 3.1",
            text: format!(
                "Recorrido de evacuación más largo de {} ({} salida{}: {}), desde {}",
                l.name,
                exits_used,
                if exits_used == 1 { "" } else { "s" },
                exit_names.join(", "),
                crate::subject_name(model, Subject::Room(rid))
            ),
            value: m(longest),
            required: format!("≤ {}", m(limit)),
            route,
        });
        if exits_used == 1 {
            let (people, max, what) = if dwelling {
                (building_people, 500.0, "el edificio de viviendas")
            } else {
                (level_people, 100.0, "la planta")
            };
            if people > max {
                report.items.push(Item {
                    section: Section::Evacuation,
                    status: Status::Fails,
                    subject: Subject::Level(level),
                    level: Some(level),
                    rule: "SI 3, tabla 3.1",
                    text: format!("Con una sola salida, la ocupación de {what} no puede pasar de {max} personas"),
                    value: format!("{} personas", people.ceil()),
                    required: format!("≤ {max} personas"),
                    route: Vec::new(),
                });
            }
        }
    }
    door_checks(model, &doors, &flows, dwelling, report);
    corridor_checks(model, &doors, &flows, dwelling, &rooms, report);
}

fn door_checks(model: &Model, doors: &[Door], flows: &Flows, dwelling: bool, report: &mut Report) {
    for (&i, &people) in &flows.through {
        let d = &doors[i];
        let level = Some(d.level);
        let subject = Subject::Opening(d.id);
        let p = people.ceil();
        let need = (p / 200.0).max(0.80);
        report.items.push(Item {
            section: Section::Evacuation,
            status: if d.clear >= need - 1e-6 {
                Status::Passes
            } else {
                Status::Fails
            },
            subject,
            level,
            rule: "SI 3, tabla 4.1 (puertas y pasos)",
            text: format!("Anchura libre de paso para {p} personas (A ≥ P/200 ≥ 0,80 m)"),
            value: m(d.clear),
            required: format!("≥ {}", m(need)),
            route: Vec::new(),
        });
        if !d.sliding && d.swing.is_some() && (d.leaf < 0.60 - 1e-6 || d.leaf > 1.23 + 1e-6) {
            report.items.push(Item {
                section: Section::Evacuation,
                status: Status::Fails,
                subject,
                level,
                rule: "SI 3, 4.2",
                text: "La hoja de una puerta de evacuación debe medir entre 0,60 y 1,23 m".into(),
                value: m(d.leaf),
                required: "0,60–1,23 m".into(),
                route: Vec::new(),
            });
        }
        let own = flows.own_room.get(&i).map_or(0.0, |x| x.1);
        let exit = d.rooms.iter().any(|r| r.is_none());
        let limit_exit = if dwelling { 200.0 } else { 100.0 };
        if people > 50.0 && (d.sliding || d.swing.is_none()) {
            report.items.push(Item {
                section: Section::Evacuation,
                status: Status::Fails,
                subject,
                level,
                rule: "SI 3, 6.1",
                text: format!(
                    "Una puerta prevista para más de 50 personas ({p}) debe ser abatible con eje de giro vertical"
                ),
                value: String::new(),
                required: String::new(),
                route: Vec::new(),
            });
        }
        let must_open_out = own > 50.0 || (exit && people > limit_exit);
        if must_open_out {
            if let Some(&from) = flows.from_side.get(&i) {
                if d.opens_towards(from) == Some(false) {
                    report.items.push(Item {
                        section: Section::Evacuation,
                        status: Status::Fails,
                        subject,
                        level,
                        rule: "SI 3, 6.3",
                        text: format!(
                            "Debe abrir en el sentido de la evacuación ({p} personas{})",
                            if own > 50.0 {
                                ", más de 50 del recinto"
                            } else {
                                ""
                            }
                        ),
                        value: "abre hacia dentro".into(),
                        required: "abre hacia la salida".into(),
                        route: Vec::new(),
                    });
                }
            }
        }
    }
    let _ = model;
}

fn corridor_checks(
    model: &Model,
    doors: &[Door],
    flows: &Flows,
    dwelling: bool,
    rooms: &BTreeMap<RoomId, (f64, bool)>,
    report: &mut Report,
) {
    for r in model.rooms() {
        let text = room_text(r);
        let common = rooms.get(&r.id).is_some_and(|(_, c)| *c);
        if !is_corridor(&text) || (dwelling && !common) {
            continue;
        }
        let people = doors
            .iter()
            .enumerate()
            .filter(|(_, d)| d.rooms.contains(&Some(r.id)))
            .filter_map(|(i, _)| flows.through.get(&i))
            .fold(0.0f64, |a, b| a.max(*b))
            .ceil();
        let need = (people / 200.0).max(1.0);
        let width = Region::new(r.outline.clone(), r.holes.clone()).width();
        report.items.push(Item {
            section: Section::Evacuation,
            status: if width >= need - 0.005 {
                Status::Passes
            } else {
                Status::Fails
            },
            subject: Subject::Room(r.id),
            level: Some(r.level),
            rule: "SI 3, tabla 4.1 (pasillos)",
            text: format!("Anchura del pasillo para {people} personas (A ≥ P/200 ≥ 1,00 m)"),
            value: m(width),
            required: format!("≥ {}", m(need)),
            route: Vec::new(),
        });
    }
}
