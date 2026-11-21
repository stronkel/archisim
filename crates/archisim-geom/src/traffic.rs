//! Traffic for illustration (N9, `docs/TERRITORIO.md` §2.12): cars that
//! run along the lanes of the streets and through the junctions by their
//! lane connections (N3), stopping at the lights and the stop signs, and
//! people walking along the sidewalks — so that the 3D view and the
//! pictures look lived in. It works out no capacity and no queues: each
//! car keeps its distance to the one in front on its lane and nothing
//! more.
//!
//! Repeatable: the same model, seed and time give the same picture.

use std::collections::HashMap;

use archisim_core::{ArmRef, ArmSign, LevelId, MaterialId, Model, Point2, RoadBand, RoadId, Vec2};

use crate::crossroads::{self, LANE_ON_AXIS};
use crate::solid::{prism, Solid, SolidOwner, P3};

/// How fast the cars go, metres a second (about 30 km/h).
pub const CAR_SPEED: f64 = 8.5;
/// How fast people walk, metres a second.
pub const WALK_SPEED: f64 = 1.3;
/// The distance a car keeps to the one in front, from middle to middle.
pub const CAR_GAP: f64 = 7.5;
/// How long the lights are green for one way, and amber, seconds.
pub const GREEN: f64 = 16.0;
pub const AMBER: f64 = 3.0;
/// How long a car waits at a stop sign, seconds.
pub const STOP_WAIT: f64 = 1.5;
/// A car: its length, width and height.
pub const CAR: (f64, f64, f64) = (4.2, 1.8, 1.45);

/// The cars' colours.
const CAR_COLOURS: [[u8; 3]; 8] = [
    [230, 230, 226],
    [40, 42, 46],
    [150, 154, 160],
    [176, 38, 34],
    [34, 70, 132],
    [210, 182, 92],
    [72, 110, 78],
    [120, 92, 70],
];
/// The people's clothes.
const CLOTHES: [[u8; 3]; 6] = [
    [60, 80, 120],
    [170, 60, 50],
    [230, 210, 150],
    [70, 70, 74],
    [90, 130, 90],
    [200, 120, 150],
];
const SKIN: [u8; 3] = [214, 170, 140];

/// A stretch of a lane of a street, one way, between two junctions (or a
/// junction and a free end).
#[derive(Debug, Clone)]
struct Stretch {
    road: RoadId,
    /// From and to, along the road.
    s0: f64,
    s1: f64,
    /// To the left of the road's axis looking along it.
    offset: f64,
    /// The junction and arm it runs into, and the lane there.
    into: Option<(usize, ArmRef, usize)>,
}

impl Stretch {
    fn length(&self) -> f64 {
        (self.s1 - self.s0).abs()
    }
    fn station(&self, d: f64) -> f64 {
        if self.s1 >= self.s0 {
            self.s0 + d
        } else {
            self.s0 - d
        }
    }
}

/// A way through a junction from a lane to a lane.
#[derive(Debug, Clone)]
struct Way {
    junction: usize,
    /// The arm it comes in by.
    from: ArmRef,
    /// The ways of its junction it crosses or meets at its end: while a
    /// car is on one of them, none goes into this one.
    conflicts: Vec<usize>,
    path: Vec<Point2>,
    /// Along the path, from its start.
    along: Vec<f64>,
    then: Option<usize>,
}

impl Way {
    fn length(&self) -> f64 {
        self.along.last().copied().unwrap_or(0.0)
    }
}

/// Where a car is.
#[derive(Debug, Clone, Copy, PartialEq)]
enum On {
    Stretch(usize),
    Way(usize),
}

#[derive(Debug, Clone)]
struct Car {
    on: On,
    at: f64,
    colour: [u8; 3],
    waited: f64,
}

#[derive(Debug, Clone)]
struct Walker {
    road: RoadId,
    offset: f64,
    s: f64,
    forward: bool,
    speed: f64,
    colour: [u8; 3],
    /// Crossing the street by a zebra crossing: from and to which offset,
    /// and how far across (0 to 1).
    crossing: Option<(f64, f64, f64)>,
    /// Its own die, for whether it crosses.
    dice: u64,
}

/// What a junction's arms say to the cars coming in.
#[derive(Debug, Clone, Default)]
struct Rule {
    /// Each arm's sign and, for the lights, which of the two ways it is.
    arms: Vec<(ArmRef, ArmSign, usize)>,
    top: f64,
    /// Its traffic lights: the head's face towards the cars coming in, the
    /// way it faces, which of the two ways it is and its height over the
    /// junction's top.
    lights: Vec<(Point2, Vec2, usize, f64)>,
}

/// Where a road meets a junction: the junction, and the cut of its arm
/// on and of its arm back (if it has them).
type Meet = (usize, Option<f64>, Option<f64>);

/// A body to draw: a car or a person, where it is and where it looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub person: bool,
    pub at: P3,
    pub dir: Vec2,
    pub colour: [u8; 3],
}

/// A traffic light's lamp as it is now: red, amber or green, lit or not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    /// The middle of its face.
    pub at: P3,
    /// The way it faces (the way the cars it stops come from).
    pub dir: Vec2,
    pub colour: [u8; 3],
}

/// The colours of the lamps: red, amber and green, lit and off.
const LIT: [[u8; 3]; 3] = [[235, 40, 30], [250, 170, 20], [40, 220, 90]];
const OFF: [[u8; 3]; 3] = [[70, 30, 28], [72, 58, 28], [28, 64, 40]];

/// The traffic of a level.
#[derive(Debug, Clone)]
pub struct Traffic {
    elevation: f64,
    stretches: Vec<Stretch>,
    ways: Vec<Way>,
    /// The ways out of each lane coming into a junction.
    choices: HashMap<(usize, ArmRef, usize), Vec<usize>>,
    rules: Vec<Rule>,
    cars: Vec<Car>,
    people: Vec<Walker>,
    time: f64,
    rng: u64,
}

/// The lanes' middles of a road at a station: those one goes along it
/// (forward, kerb first) and those one goes back (kerb first).
fn lanes(model: &Model, road: RoadId, s: f64) -> (Vec<f64>, Vec<f64>) {
    let Some(r) = model.road(road) else {
        return (Vec::new(), Vec::new());
    };
    let middles: Vec<f64> = r
        .offsets_at(s)
        .iter()
        .filter(|o| matches!(o.0, RoadBand::Carriageway | RoadBand::BusLane))
        .map(|o| (o.1 + o.2) / 2.0)
        .collect();
    let mut forward: Vec<f64> = middles
        .iter()
        .copied()
        .filter(|m| *m < LANE_ON_AXIS)
        .collect();
    forward.sort_by(f64::total_cmp);
    let mut back: Vec<f64> = middles
        .iter()
        .copied()
        .filter(|m| *m > -LANE_ON_AXIS)
        .collect();
    back.sort_by(|a, b| b.total_cmp(a));
    (forward, back)
}

impl Traffic {
    /// The traffic of a level with about `cars` cars and `people` people,
    /// placed by `seed`.
    pub fn new(model: &Model, level: LevelId, cars: usize, people: usize, seed: u64) -> Traffic {
        let junctions = crossroads::junctions(model, level);
        // Where each road meets a junction: its station, the arm on and the
        // arm back.
        struct Stop {
            station: f64,
            junction: usize,
            on: Option<f64>,
            back: Option<f64>,
        }
        let mut stops: HashMap<RoadId, Vec<Stop>> = HashMap::new();
        let mut rules = Vec::new();
        for (ji, j) in junctions.iter().enumerate() {
            let settings = j.key.as_ref().and_then(|k| model.node(k));
            let mut rule = Rule {
                top: j.top,
                ..Default::default()
            };
            let mut first_light: Option<Vec2> = None;
            for a in &j.arms {
                let arm = crossroads::Junction::arm_ref(a);
                let sign = settings
                    .and_then(|n| n.arm(arm).sign)
                    .unwrap_or(ArmSign::None);
                let way = if sign == ArmSign::Lights {
                    match first_light {
                        None => {
                            first_light = Some(a.dir);
                            0
                        }
                        Some(d) => usize::from(d.dot(a.dir).abs() < 0.7),
                    }
                } else {
                    0
                };
                rule.arms.push((arm, sign, way));
                if sign == ArmSign::Lights {
                    let kerb = model.road(a.road).map_or(0.15, |r| r.section.kerb_height);
                    if let Some((post, dir, _)) = j
                        .signs
                        .iter()
                        .filter(|(_, d, s)| *s == ArmSign::Lights && d.dot(a.dir) > 0.999)
                        .min_by(|x, y| x.0.distance(a.origin).total_cmp(&y.0.distance(a.origin)))
                    {
                        // The head stands on the post's side towards the
                        // junction's outside, as `crossroads` builds it.
                        let face = *post + *dir * (crossroads::POST + 0.31);
                        rule.lights.push((face, *dir, way, kerb));
                    }
                }
                let list = stops.entry(a.road).or_default();
                let at = match list
                    .iter_mut()
                    .find(|x| x.junction == ji && (x.station - a.station).abs() < 1.0)
                {
                    Some(x) => x,
                    None => {
                        list.push(Stop {
                            station: a.station,
                            junction: ji,
                            on: None,
                            back: None,
                        });
                        list.last_mut().unwrap()
                    }
                };
                if a.forward {
                    at.on = Some(a.cut);
                } else {
                    at.back = Some(a.cut);
                }
            }
            rules.push(rule);
        }
        let mut stretches = Vec::new();
        // Where a lane starts out of a junction: its stretch.
        let mut starts: HashMap<(usize, ArmRef, usize), usize> = HashMap::new();
        for r in model.roads_on_level(level) {
            let len = r.length();
            if len < 1.0 {
                continue;
            }
            let mut list = stops.remove(&r.id).unwrap_or_default();
            list.sort_by(|a, b| a.station.total_cmp(&b.station));
            // The road's free ends.
            let mut points: Vec<(f64, Option<Meet>)> = Vec::new();
            if list.first().is_none_or(|x| x.station > 1.0) {
                points.push((0.0, None));
            }
            for x in &list {
                points.push((x.station, Some((x.junction, x.on, x.back))));
            }
            if list.last().is_none_or(|x| x.station < len - 1.0) {
                points.push((len, None));
            }
            for w in points.windows(2) {
                let ((sa, a), (sb, b)) = (w[0], w[1]);
                let mid = (sa + sb) / 2.0;
                let (on, back) = lanes(model, r.id, mid);
                // Along the road: out of `a` (its arm on, or the free end),
                // into `b` (its arm back, or the free end).
                let out_a = match a {
                    None => Some(0.0),
                    Some((_, on_cut, _)) => on_cut,
                };
                let into_b = match b {
                    None => Some(0.0),
                    Some((_, _, back_cut)) => back_cut,
                };
                if let (Some(ca), Some(cb)) = (out_a, into_b) {
                    for (k, &offset) in on.iter().enumerate() {
                        let st = Stretch {
                            road: r.id,
                            s0: sa + ca,
                            s1: sb - cb,
                            offset,
                            into: b.map(|(ji, _, _)| {
                                (
                                    ji,
                                    ArmRef {
                                        road: r.id,
                                        forward: false,
                                    },
                                    k,
                                )
                            }),
                        };
                        if st.s1 - st.s0 < 0.5 {
                            continue;
                        }
                        if let Some((ji, _, _)) = a {
                            starts.insert(
                                (
                                    ji,
                                    ArmRef {
                                        road: r.id,
                                        forward: true,
                                    },
                                    k,
                                ),
                                stretches.len(),
                            );
                        }
                        stretches.push(st);
                    }
                }
                // Back along it: out of `b` (its arm back), into `a`.
                let out_b = match b {
                    None => Some(0.0),
                    Some((_, _, back_cut)) => back_cut,
                };
                let into_a = match a {
                    None => Some(0.0),
                    Some((_, on_cut, _)) => on_cut,
                };
                if let (Some(cb), Some(ca)) = (out_b, into_a) {
                    for (k, &offset) in back.iter().enumerate() {
                        let st = Stretch {
                            road: r.id,
                            s0: sb - cb,
                            s1: sa + ca,
                            offset,
                            into: a.map(|(ji, _, _)| {
                                (
                                    ji,
                                    ArmRef {
                                        road: r.id,
                                        forward: true,
                                    },
                                    k,
                                )
                            }),
                        };
                        if st.s0 - st.s1 < 0.5 {
                            continue;
                        }
                        if let Some((ji, _, _)) = b {
                            starts.insert(
                                (
                                    ji,
                                    ArmRef {
                                        road: r.id,
                                        forward: false,
                                    },
                                    k,
                                ),
                                stretches.len(),
                            );
                        }
                        stretches.push(st);
                    }
                }
            }
        }
        // The ways through the junctions.
        let mut ways = Vec::new();
        let mut choices: HashMap<(usize, ArmRef, usize), Vec<usize>> = HashMap::new();
        for (ji, j) in junctions.iter().enumerate() {
            for c in &j.lanes {
                if c.path.len() < 2 {
                    continue;
                }
                let mut along = vec![0.0];
                for w in c.path.windows(2) {
                    along.push(along.last().unwrap() + w[0].distance(w[1]));
                }
                choices
                    .entry((ji, c.from, c.lane))
                    .or_default()
                    .push(ways.len());
                ways.push(Way {
                    junction: ji,
                    from: c.from,
                    conflicts: Vec::new(),
                    path: c.path.clone(),
                    along,
                    then: starts.get(&(ji, c.to, c.to_lane)).copied(),
                });
            }
        }
        // Which ways of a junction get in each other's way: they cross, or
        // they end in the same lane; not those out of the same lane (the
        // car in front is kept to by its gap).
        let mut by_junction: HashMap<usize, Vec<usize>> = HashMap::new();
        for (k, w) in ways.iter().enumerate() {
            by_junction.entry(w.junction).or_default().push(k);
        }
        for list in by_junction.values() {
            for (n, &i) in list.iter().enumerate() {
                for &k in &list[n + 1..] {
                    if ways_meet(&ways[i].path, &ways[k].path) {
                        ways[i].conflicts.push(k);
                        ways[k].conflicts.push(i);
                    }
                }
            }
        }
        let elevation = model.level(level).map_or(0.0, |l| l.elevation);
        let mut t = Traffic {
            elevation,
            stretches,
            ways,
            choices,
            rules,
            cars: Vec::new(),
            people: Vec::new(),
            time: 0.0,
            rng: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        };
        for _ in 0..cars {
            if let Some(c) = t.spawn() {
                t.cars.push(c);
            }
        }
        // People on the sidewalks.
        let walks: Vec<(RoadId, f64, f64)> = model
            .roads_on_level(level)
            .flat_map(|r| {
                let len = r.length();
                r.offsets_at(len / 2.0)
                    .into_iter()
                    .filter(|o| o.0 == RoadBand::Sidewalk)
                    .map(move |o| (r.id, (o.1 + o.2) / 2.0, len))
            })
            .filter(|w| w.2 > 4.0)
            .collect();
        let total: f64 = walks.iter().map(|w| w.2).sum();
        if total > 0.0 {
            for _ in 0..people {
                let mut pick = t.unit() * total;
                let w = walks
                    .iter()
                    .find(|w| {
                        pick -= w.2;
                        pick <= 0.0
                    })
                    .unwrap_or(&walks[0]);
                let walker = Walker {
                    road: w.0,
                    offset: w.1 + (t.unit() - 0.5) * 0.8,
                    s: 1.0 + t.unit() * (w.2 - 2.0),
                    forward: t.unit() < 0.5,
                    speed: WALK_SPEED * (0.8 + 0.4 * t.unit()),
                    colour: CLOTHES[(t.unit() * CLOTHES.len() as f64) as usize % CLOTHES.len()],
                    crossing: None,
                    dice: (t.unit() * u64::MAX as f64) as u64 | 1,
                };
                t.people.push(walker);
            }
        }
        t
    }

    /// The traffic of a level as it is `seconds` after it starts.
    pub fn at(
        model: &Model,
        level: LevelId,
        cars: usize,
        people: usize,
        seed: u64,
        seconds: f64,
    ) -> Traffic {
        let mut t = Traffic::new(model, level, cars, people, seed);
        let steps = (seconds / 0.1).ceil() as usize;
        for _ in 0..steps {
            t.step(model, seconds / steps.max(1) as f64);
        }
        t
    }

    fn unit(&mut self) -> f64 {
        // xorshift64*.
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A car somewhere free on a stretch, or none if there is no room.
    fn spawn(&mut self) -> Option<Car> {
        let total: f64 = self.stretches.iter().map(Stretch::length).sum();
        if total <= 0.0 {
            return None;
        }
        for _ in 0..8 {
            let mut pick = self.unit() * total;
            let k = self
                .stretches
                .iter()
                .position(|s| {
                    pick -= s.length();
                    pick <= 0.0
                })
                .unwrap_or(0);
            let at = self.unit() * self.stretches[k].length();
            let free = self
                .cars
                .iter()
                .all(|c| c.on != On::Stretch(k) || (c.at - at).abs() >= CAR_GAP);
            if free {
                let colour = CAR_COLOURS
                    [(self.unit() * CAR_COLOURS.len() as f64) as usize % CAR_COLOURS.len()];
                return Some(Car {
                    on: On::Stretch(k),
                    at,
                    colour,
                    waited: 0.0,
                });
            }
        }
        None
    }

    /// Seconds since it started.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// How many cars there are.
    pub fn cars(&self) -> usize {
        self.cars.len()
    }

    /// How many cars are stopped at a red light or a stop sign.
    pub fn waiting(&self) -> usize {
        self.cars
            .iter()
            .filter(|c| match c.on {
                On::Stretch(k) => c.at >= self.stretches[k].length() - 0.01,
                On::Way(_) => false,
            })
            .count()
    }

    /// Whether the lights of a junction's arm let the cars in now.
    fn green(&self, junction: usize, arm: ArmRef) -> bool {
        let Some(rule) = self.rules.get(junction) else {
            return true;
        };
        let Some(&(_, sign, way)) = rule.arms.iter().find(|(a, _, _)| *a == arm) else {
            return true;
        };
        if sign != ArmSign::Lights {
            return true;
        }
        let period = 2.0 * (GREEN + AMBER);
        let t = (self.time % period) - way as f64 * (GREEN + AMBER);
        (0.0..GREEN).contains(&t)
    }

    /// Whether a car on stretch `s` waits at the line of another arm of
    /// junction `ji` than `arm`, without a sign, to go by one of `crossing`.
    fn waits_to_cross(
        &self,
        c: &Car,
        s: usize,
        ji: usize,
        arm: ArmRef,
        crossing: &[usize],
    ) -> bool {
        let st = &self.stretches[s];
        let Some((j, other, _)) = st.into else {
            return false;
        };
        if j != ji || other == arm || c.at < st.length() - 1.0 {
            return false;
        }
        let sign = self.sign(ji, other);
        if matches!(sign, ArmSign::Yield | ArmSign::Stop) || !self.green(ji, other) {
            return false;
        }
        crossing.iter().any(|w| self.ways[*w].from == other)
    }

    /// The lamps of the traffic lights as they are now: each head's red,
    /// amber and green.
    pub fn lamps(&self) -> Vec<Lamp> {
        let period = 2.0 * (GREEN + AMBER);
        let mut out = Vec::new();
        for rule in &self.rules {
            for &(face, dir, way, kerb) in &rule.lights {
                let t = (self.time % period) - way as f64 * (GREEN + AMBER);
                let lit = if (0.0..GREEN).contains(&t) {
                    2
                } else if (GREEN..GREEN + AMBER).contains(&t) {
                    1
                } else {
                    0
                };
                for (k, h) in [3.0, 2.75, 2.5].into_iter().enumerate() {
                    out.push(Lamp {
                        at: [face.x, face.y, self.elevation + rule.top + kerb + h],
                        dir,
                        colour: if k == lit { LIT[k] } else { OFF[k] },
                    });
                }
            }
        }
        out
    }

    fn sign(&self, junction: usize, arm: ArmRef) -> ArmSign {
        self.rules
            .get(junction)
            .and_then(|r| r.arms.iter().find(|(a, _, _)| *a == arm))
            .map_or(ArmSign::None, |x| x.1)
    }

    fn length_of(&self, on: On) -> f64 {
        match on {
            On::Stretch(k) => self.stretches[k].length(),
            On::Way(k) => self.ways[k].length(),
        }
    }

    /// Moves everything on by `dt` seconds.
    pub fn step(&mut self, model: &Model, dt: f64) {
        self.time += dt;
        // The cars, front first on each path so that the one behind sees
        // where the one in front got to.
        let mut order: Vec<usize> = (0..self.cars.len()).collect();
        order.sort_by(|a, b| self.cars[*b].at.total_cmp(&self.cars[*a].at));
        let mut gone = Vec::new();
        for i in order {
            let car = self.cars[i].clone();
            let len = self.length_of(car.on);
            // The one in front on the same path.
            let front = self
                .cars
                .iter()
                .enumerate()
                .filter(|(j, c)| *j != i && c.on == car.on && c.at > car.at)
                .map(|(_, c)| c.at)
                .fold(f64::INFINITY, f64::min);
            let mut at = (car.at + CAR_SPEED * dt).min(front - CAR_GAP).max(car.at);
            let mut on = car.on;
            let mut waited = car.waited;
            if at >= len {
                match on {
                    On::Stretch(k) => match self.stretches[k].into {
                        None => {
                            gone.push(i);
                            continue;
                        }
                        Some((ji, arm, lane)) => {
                            let sign = self.sign(ji, arm);
                            let stop_ok = sign != ArmSign::Stop || waited >= STOP_WAIT;
                            if !self.green(ji, arm) || !stop_ok {
                                at = len;
                                waited += dt;
                            } else {
                                let ways = self
                                    .choices
                                    .get(&(ji, arm, lane))
                                    .or_else(|| {
                                        (0..4).find_map(|l| self.choices.get(&(ji, arm, l)))
                                    })
                                    .cloned()
                                    .unwrap_or_default();
                                if ways.is_empty() {
                                    gone.push(i);
                                    continue;
                                }
                                let pick =
                                    ways[(self.unit() * ways.len() as f64) as usize % ways.len()];
                                // Only in if the way is clear at its start,
                                // no car is on a way it crosses, and (with a
                                // give way or a stop) no car waits to come in
                                // on a way it crosses from an arm without one.
                                let crossing = &self.ways[pick].conflicts;
                                let yields = matches!(sign, ArmSign::Yield | ArmSign::Stop);
                                let clear = self.cars.iter().all(|c| match c.on {
                                    On::Way(w) => {
                                        !(w == pick && c.at < CAR_GAP) && !crossing.contains(&w)
                                    }
                                    On::Stretch(s) => {
                                        !yields || !self.waits_to_cross(c, s, ji, arm, crossing)
                                    }
                                });
                                if clear {
                                    on = On::Way(pick);
                                    at -= len;
                                    waited = 0.0;
                                } else {
                                    at = len;
                                }
                            }
                        }
                    },
                    On::Way(k) => match self.ways[k].then {
                        None => {
                            gone.push(i);
                            continue;
                        }
                        Some(next) => {
                            let clear = self
                                .cars
                                .iter()
                                .all(|c| c.on != On::Stretch(next) || c.at >= CAR_GAP);
                            if clear {
                                on = On::Stretch(next);
                                at -= len;
                            } else {
                                at = len;
                            }
                        }
                    },
                }
            }
            let c = &mut self.cars[i];
            c.on = on;
            c.at = at;
            c.waited = waited;
        }
        // The cars gone out at a free end come back somewhere else.
        gone.sort_unstable();
        for i in gone.into_iter().rev() {
            self.cars.swap_remove(i);
            if let Some(c) = self.spawn() {
                self.cars.push(c);
            }
        }
        // People: along their sidewalk, and back at its ends.
        for p in &mut self.people {
            let Some(r) = model.road(p.road) else {
                continue;
            };
            let len = r.length();
            // Crossing: across at walking speed, then on along the other
            // sidewalk.
            if let Some((from, to, done)) = p.crossing {
                let width = (to - from).abs().max(1.0);
                let done = done + p.speed * dt / width;
                if done >= 1.0 {
                    p.offset = to;
                    p.crossing = None;
                } else {
                    p.offset = from + (to - from) * done;
                    p.crossing = Some((from, to, done));
                }
                continue;
            }
            let d = if p.forward {
                p.speed * dt
            } else {
                -p.speed * dt
            };
            let before = p.s;
            p.s += d;
            // At a zebra crossing, every other one crosses (N9).
            let (lo, hi) = if before < p.s {
                (before, p.s)
            } else {
                (p.s, before)
            };
            if let Some(c) = r.section.crossings.iter().find(|c| c.at > lo && c.at <= hi) {
                p.dice ^= p.dice << 13;
                p.dice ^= p.dice >> 7;
                p.dice ^= p.dice << 17;
                if p.dice % 2 == 0 {
                    let other = r
                        .offsets_at(c.at)
                        .into_iter()
                        .filter(|o| o.0 == RoadBand::Sidewalk)
                        .map(|o| (o.1 + o.2) / 2.0)
                        .find(|o| o.signum() != p.offset.signum());
                    if let Some(to) = other {
                        p.s = c.at;
                        p.crossing = Some((p.offset, to, 0.0));
                        continue;
                    }
                }
            }
            if p.s > len - 0.5 {
                p.s = len - 0.5;
                p.forward = false;
            } else if p.s < 0.5 {
                p.s = 0.5;
                p.forward = true;
            }
        }
    }

    /// The cars and people as they are now.
    pub fn bodies(&self, model: &Model) -> Vec<Body> {
        let mut out = Vec::new();
        for c in &self.cars {
            let (p, dir, z) = match c.on {
                On::Stretch(k) => {
                    let s = &self.stretches[k];
                    let Some(r) = model.road(s.road) else {
                        continue;
                    };
                    let st = s.station(c.at);
                    let (q, u) = r.axis_at(st);
                    let dir = if s.s1 >= s.s0 { u } else { u * -1.0 };
                    (q + u.perp_left() * s.offset, dir, r.top_at(st))
                }
                On::Way(k) => {
                    let w = &self.ways[k];
                    let i = w
                        .along
                        .windows(2)
                        .position(|x| c.at <= x[1])
                        .unwrap_or(w.along.len().saturating_sub(2));
                    let (a, b) = (w.path[i], w.path[i + 1]);
                    let seg = (w.along[i + 1] - w.along[i]).max(1e-9);
                    let t = ((c.at - w.along[i]) / seg).clamp(0.0, 1.0);
                    let dir = (b - a).normalized().unwrap_or(Vec2::new(1.0, 0.0));
                    (a + (b - a) * t, dir, self.rules[w.junction].top)
                }
            };
            out.push(Body {
                person: false,
                at: [p.x, p.y, self.elevation + z],
                dir,
                colour: c.colour,
            });
        }
        for p in &self.people {
            let Some(r) = model.road(p.road) else {
                continue;
            };
            let (q, u) = r.axis_at(p.s);
            let dir = if p.forward { u } else { u * -1.0 };
            let at = q + u.perp_left() * p.offset;
            out.push(Body {
                person: true,
                at: [
                    at.x,
                    at.y,
                    self.elevation
                        + r.top_at(p.s)
                        + if p.crossing.is_some() {
                            0.0
                        } else {
                            r.section.kerb_height
                        },
                ],
                dir,
                colour: p.colour,
            });
        }
        out
    }
}

/// How many cars and people for the streets of a level: one car every
/// 40 m of street and a person every 25 m, within bounds.
pub fn usual_counts(model: &Model, level: LevelId) -> (usize, usize) {
    let len: f64 = model.roads_on_level(level).map(|r| r.length()).sum();
    (
        ((len / 40.0) as usize).clamp(4, 400),
        ((len / 25.0) as usize).clamp(4, 600),
    )
}

/// A rectangle `l` × `w` about `c` turned to `dir`.
fn rect(c: Point2, dir: Vec2, l: f64, w: f64) -> Vec<Point2> {
    let (u, n) = (dir * (l / 2.0), dir.perp_left() * (w / 2.0));
    vec![c - u - n, c + u - n, c + u + n, c - u + n]
}

/// The lamps as solids: a small square on the face of each head, in the
/// colour it has now (`SolidOwner::Traffic`).
pub fn lamp_solids(lamps: &[Lamp], material: MaterialId) -> Vec<Solid> {
    lamps
        .iter()
        .map(|l| {
            let c = Point2::new(l.at[0], l.at[1]) + l.dir * 0.02;
            Solid {
                owner: SolidOwner::Traffic(l.colour),
                material,
                triangles: prism(&rect(c, l.dir, 0.04, 0.18), l.at[2] - 0.09, l.at[2] + 0.09),
            }
        })
        .collect()
}

/// Whether two ways through a junction get in each other's way: they end
/// in the same place, or they cross away from their starts.
fn ways_meet(a: &[Point2], b: &[Point2]) -> bool {
    let (Some(a0), Some(b0)) = (a.first(), b.first()) else {
        return false;
    };
    if a0.distance(*b0) < 0.5 {
        return false;
    }
    if let (Some(a1), Some(b1)) = (a.last(), b.last()) {
        if a1.distance(*b1) < 0.5 {
            return true;
        }
    }
    a.windows(2)
        .any(|p| b.windows(2).any(|q| segments_cross(p[0], p[1], q[0], q[1])))
}

fn segments_cross(p: Point2, q: Point2, r: Point2, s: Point2) -> bool {
    let d = |a: Point2, b: Point2, c: Point2| (b - a).cross(c - a);
    let (d1, d2) = (d(r, s, p), d(r, s, q));
    let (d3, d4) = (d(p, q, r), d(p, q, s));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The bodies as solids, simple boxes in their colours: a car's body and
/// cabin, a person's clothes and head. Their material is `material`; their
/// colour comes with them (`SolidOwner::Traffic`).
pub fn traffic_solids(bodies: &[Body], material: MaterialId) -> Vec<Solid> {
    let mut out = Vec::new();
    for b in bodies {
        let c = Point2::new(b.at[0], b.at[1]);
        let z = b.at[2];
        let mut add = |poly: Vec<Point2>, z0: f64, z1: f64, colour: [u8; 3]| {
            out.push(Solid {
                owner: SolidOwner::Traffic(colour),
                material,
                triangles: prism(&poly, z0, z1),
            });
        };
        if b.person {
            add(rect(c, b.dir, 0.3, 0.45), z, z + 1.45, b.colour);
            add(rect(c, b.dir, 0.22, 0.2), z + 1.45, z + 1.72, SKIN);
        } else {
            let (l, w, h) = CAR;
            add(rect(c, b.dir, l, w), z + 0.25, z + 0.85, b.colour);
            let shade = b.colour.map(|x| (x as f64 * 0.55) as u8);
            add(
                rect(c - b.dir * 0.3, b.dir, l * 0.5, w * 0.88),
                z + 0.85,
                z + h,
                shade,
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewRoad, NodeSettings, RoadSection};

    fn town() -> (Model, LevelId) {
        let mut m = Model::new("Tráfico");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Asfalto",
            "asphalt",
            [60, 60, 60],
            archisim_core::CutPattern::Solid,
        );
        for (a, b) in [
            (Point2::new(-100.0, 0.0), Point2::new(100.0, 0.0)),
            (Point2::new(0.0, -100.0), Point2::new(0.0, 100.0)),
        ] {
            m.add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![a, b],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        }
        (m, level)
    }

    #[test]
    fn people_cross_by_the_zebra_crossings() {
        let (mut m, level) = town();
        let ids: Vec<RoadId> = m.roads().map(|r| r.id).collect();
        for id in ids {
            m.edit_road(id, |r| {
                r.section.crossings = (1..20)
                    .map(|k| archisim_core::Crossing {
                        at: k as f64 * 10.0,
                        width: 4.0,
                    })
                    .collect()
            })
            .unwrap();
        }
        let mut t = Traffic::new(&m, level, 0, 30, 3);
        let kerb = RoadSection::default().kerb_height;
        let mut crossed = 0;
        let mut sides = std::collections::HashSet::new();
        for _ in 0..600 {
            t.step(&m, 0.2);
            for b in t.bodies(&m).iter().filter(|b| b.person) {
                if b.at[2] < kerb - 1e-6 {
                    crossed += 1;
                }
            }
            for p in &t.people {
                sides.insert((p.road, p.offset > 0.0));
            }
        }
        assert!(crossed > 0, "someone crosses the street");
        assert!(sides.len() >= 3, "and walks on the other side: {sides:?}");
    }

    #[test]
    fn cars_go_round_and_stop_at_the_lights() {
        let (mut m, level) = town();
        let t = Traffic::new(&m, level, 12, 10, 7);
        // Four arms, two ways each: eight stretches; twelve ways through.
        assert_eq!(t.stretches.len(), 8);
        assert_eq!(t.ways.len(), 12);
        assert_eq!(t.cars(), 12);
        // Repeatable.
        let a = Traffic::at(&m, level, 12, 10, 7, 20.0).bodies(&m);
        let b = Traffic::at(&m, level, 12, 10, 7, 20.0).bodies(&m);
        assert_eq!(a, b);
        assert_eq!(a.iter().filter(|x| x.person).count(), 10);
        // The cars stay on the carriageway: within its width of an axis.
        let w = RoadSection::default().width() / 2.0;
        for x in a.iter().filter(|x| !x.person) {
            assert!(x.at[0].abs() < w || x.at[1].abs() < w, "{x:?}");
        }
        // Lights on every arm: some wait at red.
        let key = crossroads::junctions(&m, level)[0].key.clone().unwrap();
        let arms = crossroads::node_arms(&m, &key);
        let mut n = NodeSettings::new(key);
        for a in &arms {
            let mut s = n.arm(crossroads::Junction::arm_ref(a));
            s.sign = Some(ArmSign::Lights);
            n.arms.push((crossroads::Junction::arm_ref(a), s));
        }
        m.set_node(n).unwrap();
        let mut t = Traffic::new(&m, level, 16, 0, 3);
        let mut most = 0;
        for _ in 0..600 {
            t.step(&m, 0.1);
            most = most.max(t.waiting());
        }
        assert!(most > 0, "nobody stopped at the lights");
        assert!(t.cars() >= 12);
        let solids = traffic_solids(&t.bodies(&m), MaterialId(0));
        assert_eq!(solids.len(), 2 * t.cars());
        // Four heads of three lamps; two green and two red, then amber.
        let lamps = t.lamps();
        assert_eq!(lamps.len(), 12);
        let lit = |t: &Traffic, c: [u8; 3]| t.lamps().iter().filter(|l| l.colour == c).count();
        let mut t = Traffic::new(&m, level, 0, 0, 3);
        t.step(&m, 1.0);
        assert_eq!((lit(&t, LIT[2]), lit(&t, LIT[0])), (2, 2));
        t.step(&m, GREEN);
        assert_eq!((lit(&t, LIT[1]), lit(&t, LIT[0])), (2, 2));
        assert_eq!(lamp_solids(&lamps, MaterialId(0)).len(), 12);
    }

    #[test]
    fn cars_do_not_meet_inside_a_crossing() {
        let (m, level) = town();
        let mut t = Traffic::new(&m, level, 24, 0, 11);
        // The left turn across the way straight on from the other side.
        assert!(t.ways.iter().any(|w| !w.conflicts.is_empty()));
        let mut entered = 0;
        let mut before: Vec<On> = t.cars.iter().map(|c| c.on).collect();
        for _ in 0..1500 {
            t.step(&m, 0.1);
            let on: Vec<usize> = t
                .cars
                .iter()
                .filter_map(|c| match c.on {
                    On::Way(w) => Some(w),
                    _ => None,
                })
                .collect();
            for (n, a) in on.iter().enumerate() {
                for b in &on[n + 1..] {
                    assert!(
                        !t.ways[*a].conflicts.contains(b),
                        "two cars on crossing ways {a} and {b} at {}",
                        t.time()
                    );
                }
            }
            let now: Vec<On> = t.cars.iter().map(|c| c.on).collect();
            entered += now
                .iter()
                .zip(&before)
                .filter(|(a, b)| matches!(a, On::Way(_)) && !matches!(b, On::Way(_)))
                .count();
            before = now;
        }
        assert!(entered > 20, "the junction let few cars by: {entered}");
    }
}
