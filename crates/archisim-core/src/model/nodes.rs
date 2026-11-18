//! The nodes of the streets (N1, `docs/TERRITORIO.md` §2): where streets
//! meet, as Node Controller does it in Cities: Skylines. The junctions are
//! worked out from the streets each time (`archisim_geom::crossroads`);
//! here is only what Antonio changes of one of them, kept by a key that
//! does not depend on where the streets are exactly — the same node while
//! the same streets meet there in the same way.

use super::{LevelId, Model, ModelError, RoadId};
use crate::Point2;

/// How a street reaches a node: by its start, by its end, or passing
/// through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeEnd {
    Start,
    End,
    Through,
}

impl NodeEnd {
    fn code(self) -> char {
        match self {
            NodeEnd::Start => 'S',
            NodeEnd::End => 'E',
            NodeEnd::Through => 'T',
        }
    }

    fn of_code(c: &str) -> Option<NodeEnd> {
        match c {
            "S" => Some(NodeEnd::Start),
            "E" => Some(NodeEnd::End),
            "T" => Some(NodeEnd::Through),
            _ => None,
        }
    }
}

/// Which node: its level, the streets that meet there (sorted by id) and
/// how each reaches it, and which of the nodes with the same streets met
/// the same way it is (`nth`, along the first street).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeKey {
    pub level: LevelId,
    pub roads: Vec<(RoadId, NodeEnd)>,
    pub nth: u32,
}

impl NodeKey {
    /// The streets of the node.
    pub fn road_ids(&self) -> impl Iterator<Item = RoadId> + '_ {
        self.roads.iter().map(|r| r.0)
    }
}

/// What a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeKind {
    /// A crossing with its corners and kerbs (what it is unless changed).
    #[default]
    Crossing,
    /// A raised crossing: the carriageway at the sidewalk's height, with
    /// ramps on each arm.
    Raised,
    /// A shared surface: the carriageway at the sidewalk's height, paved
    /// as it, without kerbs, with bollards at the corners.
    Shared,
    /// No node: the streets pass without meeting (one over the other, or
    /// simply laid one on the other).
    Open,
    /// Two streets that go on one into the other without a crossing: the
    /// kerbs and sidewalks straight from one to the other, over a stretch
    /// where the section changes (a change of width without a seam).
    Continuous,
    /// A bend of two streets that go on one into the other: the inside
    /// corner with its radius, the outside one round the same centre, and
    /// the sidewalks following the curve (no corner sidewalks).
    Bend,
    /// The end of a street with nothing beyond (a cul-de-sac): ends
    /// straight unless it has a turning circle (`end_radius`).
    DeadEnd,
}

impl NodeKind {
    pub const ALL: [NodeKind; 7] = [
        NodeKind::Crossing,
        NodeKind::Raised,
        NodeKind::Shared,
        NodeKind::Continuous,
        NodeKind::Bend,
        NodeKind::DeadEnd,
        NodeKind::Open,
    ];

    /// Whether a node of `arms` arms (and `ends` free ends) can be of this
    /// kind: a continuous one and a bend join two arms, a dead end is one.
    pub fn fits(self, arms: usize) -> bool {
        match self {
            NodeKind::DeadEnd => arms == 1,
            NodeKind::Continuous | NodeKind::Bend => arms == 2,
            _ => arms >= 2,
        }
    }

    /// Its name for Antonio.
    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Crossing => "Cruce",
            NodeKind::Raised => "Cruce elevado",
            NodeKind::Shared => "Plataforma única",
            NodeKind::Open => "Sin nudo",
            NodeKind::Continuous => "Continuo",
            NodeKind::Bend => "Doblez",
            NodeKind::DeadEnd => "Fin",
        }
    }

    fn code(self) -> &'static str {
        match self {
            NodeKind::Crossing => "crossing",
            NodeKind::Raised => "raised",
            NodeKind::Shared => "shared",
            NodeKind::Open => "open",
            NodeKind::Continuous => "continuous",
            NodeKind::Bend => "bend",
            NodeKind::DeadEnd => "dead-end",
        }
    }

    fn of_code(c: &str) -> Option<NodeKind> {
        NodeKind::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// The shape of a corner's kerb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CornerShape {
    /// An arc of the corner's radius (what it is unless changed).
    #[default]
    Arc,
    /// A chamfer: a straight kerb across, from where the arc would start
    /// to where it would end (as in Barcelona).
    Chamfer,
    /// No rounding: the kerbs meet in a point.
    Sharp,
}

impl CornerShape {
    pub const ALL: [CornerShape; 3] = [CornerShape::Arc, CornerShape::Chamfer, CornerShape::Sharp];

    pub fn label(self) -> &'static str {
        match self {
            CornerShape::Arc => "Arco",
            CornerShape::Chamfer => "Chaflán",
            CornerShape::Sharp => "En punta",
        }
    }

    fn code(self) -> &'static str {
        match self {
            CornerShape::Arc => "arc",
            CornerShape::Chamfer => "chamfer",
            CornerShape::Sharp => "sharp",
        }
    }

    fn of_code(c: &str) -> Option<CornerShape> {
        CornerShape::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// The sign that stops or gives way the traffic that comes into a node by
/// an arm, with its mark on the road (Norma 8.2-IC: M-4.1 stop line, M-4.2
/// give-way line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArmSign {
    #[default]
    None,
    /// Give way (R-1): a dashed line across the lanes coming in.
    Yield,
    /// Stop (R-2): a continuous line across the lanes coming in.
    Stop,
    /// Traffic lights: a continuous line and the lights on their post.
    Lights,
}

impl ArmSign {
    pub const ALL: [ArmSign; 4] = [
        ArmSign::None,
        ArmSign::Yield,
        ArmSign::Stop,
        ArmSign::Lights,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ArmSign::None => "Sin señal",
            ArmSign::Yield => "Ceda el paso",
            ArmSign::Stop => "Stop",
            ArmSign::Lights => "Semáforo",
        }
    }

    fn code(self) -> &'static str {
        match self {
            ArmSign::None => "",
            ArmSign::Yield => "yield",
            ArmSign::Stop => "stop",
            ArmSign::Lights => "lights",
        }
    }

    fn of_code(c: &str) -> Option<ArmSign> {
        ArmSign::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// An arm of a node: a street going out of it one way (along the street,
/// `forward`, or back).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArmRef {
    pub road: RoadId,
    pub forward: bool,
}

/// What is changed of an arm.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArmSettings {
    /// How far from the node the arm ends at least, metres (its setback).
    pub setback: Option<f64>,
    /// The sign for the traffic coming in by it.
    pub sign: Option<ArmSign>,
    /// Where each lane coming in by it may go (N3), from the one by the
    /// kerb (the driver's right) to the one by the axis: a [`Turns`] each.
    /// None: worked out (straight on every lane, right from the kerb's,
    /// left from the axis's).
    pub turns: Option<Vec<Turns>>,
    /// How far behind the end of the arm its stop line stands, metres (N3).
    pub stop_back: Option<f64>,
}

/// Where a lane may go at a node (N3): a set of left, straight on, right
/// and turning back, as painted by its arrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Turns(pub u8);

impl Turns {
    pub const LEFT: Turns = Turns(1);
    pub const STRAIGHT: Turns = Turns(2);
    pub const RIGHT: Turns = Turns(4);
    pub const BACK: Turns = Turns(8);
    pub const NONE: Turns = Turns(0);
    /// Each turn with its label and arrow, in the order they are offered.
    pub const ALL: [(Turns, &'static str, &'static str); 4] = [
        (Turns::LEFT, "A la izquierda", "←"),
        (Turns::STRAIGHT, "De frente", "↑"),
        (Turns::RIGHT, "A la derecha", "→"),
        (Turns::BACK, "Cambio de sentido", "↶"),
    ];

    pub fn has(self, t: Turns) -> bool {
        self.0 & t.0 != 0
    }

    pub fn with(self, t: Turns, on: bool) -> Turns {
        if on {
            Turns(self.0 | t.0)
        } else {
            Turns(self.0 & !t.0)
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 & 15 == 0
    }

    /// As arrows: `←↑`.
    pub fn arrows(self) -> String {
        Turns::ALL
            .iter()
            .filter(|(t, _, _)| self.has(*t))
            .map(|(_, _, a)| *a)
            .collect()
    }
}

/// A connection made by hand from a lane coming in by an arm to a lane
/// going out by another (N3), the lanes counted from the kerb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaneLink {
    pub from: ArmRef,
    pub lane: u8,
    pub to: ArmRef,
    pub to_lane: u8,
}

/// How an island painted on a node is painted (N3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IslandPaint {
    /// Hatched with its border (M-7.1, «cebreado»).
    #[default]
    Chevrons,
    /// Painted all over.
    Fill,
    /// Only its border.
    Outline,
}

impl IslandPaint {
    pub const ALL: [IslandPaint; 3] = [
        IslandPaint::Chevrons,
        IslandPaint::Fill,
        IslandPaint::Outline,
    ];

    pub fn label(self) -> &'static str {
        match self {
            IslandPaint::Chevrons => "Cebreado",
            IslandPaint::Fill => "Relleno",
            IslandPaint::Outline => "Solo el borde",
        }
    }

    fn code(self) -> &'static str {
        match self {
            IslandPaint::Chevrons => "c",
            IslandPaint::Fill => "f",
            IslandPaint::Outline => "o",
        }
    }

    fn of_code(c: &str) -> Option<IslandPaint> {
        IslandPaint::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// An island painted on a node (N3): its outline in the node's own axes
/// (from its centre, `x` along its first arm and `y` to its left), so it
/// follows the node when its streets move.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintedIsland {
    pub points: Vec<Point2>,
    pub paint: IslandPaint,
}

/// What is changed of a corner: the one from an arm to the next one round
/// the node, counter-clockwise.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CornerSettings {
    /// Its radius, metres (the smaller of its streets' unless changed).
    pub radius: Option<f64>,
    pub shape: Option<CornerShape>,
    /// Whether it has a sidewalk (it has unless changed).
    pub sidewalk: Option<bool>,
    /// How far its sidewalk is brought out over the carriageway (over the
    /// parking, a bump-out or «oreja»), metres.
    pub bump: Option<f64>,
}

/// What Antonio changed of a node: only that is kept, the rest is worked
/// out from the streets.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSettings {
    pub key: NodeKey,
    pub kind: Option<NodeKind>,
    pub arms: Vec<(ArmRef, ArmSettings)>,
    /// By the arm the corner starts from.
    pub corners: Vec<(ArmRef, CornerSettings)>,
    /// Flat at the height of its centre (unless changed, it follows the
    /// streets that meet there).
    pub flat: Option<bool>,
    /// The street that rules: the node takes its height and its paving, and
    /// the others come to it.
    pub main: Option<RoadId>,
    /// Without its road markings (zebras, lines and stop lines) near it.
    pub hide_marks: Option<bool>,
    /// A dead end's turning circle: the radius of its kerb, metres.
    pub end_radius: Option<f64>,
    /// The connections between lanes made by hand (N3): an arm with any
    /// goes only by them.
    pub links: Vec<LaneLink>,
    /// The islands painted on it (N3).
    pub islands: Vec<PaintedIsland>,
}

impl NodeSettings {
    /// A node with nothing changed.
    pub fn new(key: NodeKey) -> Self {
        NodeSettings {
            key,
            kind: None,
            arms: Vec::new(),
            corners: Vec::new(),
            flat: None,
            main: None,
            hide_marks: None,
            end_radius: None,
            links: Vec::new(),
            islands: Vec::new(),
        }
    }

    /// Whether nothing is changed.
    pub fn is_plain(&self) -> bool {
        self.kind.is_none_or(|k| k == NodeKind::Crossing)
            && self.arms.iter().all(|(_, a)| {
                a.setback.is_none()
                    && a.sign.is_none_or(|s| s == ArmSign::None)
                    && a.turns.is_none()
                    && a.stop_back.is_none_or(|b| b <= 0.0)
            })
            && self.corners.iter().all(|(_, c)| {
                c.radius.is_none()
                    && c.shape.is_none_or(|s| s == CornerShape::Arc)
                    && c.sidewalk.is_none_or(|s| s)
                    && c.bump.is_none_or(|b| b <= 0.0)
            })
            && self.flat.is_none_or(|f| !f)
            && self.main.is_none()
            && self.hide_marks.is_none_or(|h| !h)
            && self.end_radius.is_none()
            && self.links.is_empty()
            && self.islands.is_empty()
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.unwrap_or_default()
    }

    pub fn arm(&self, arm: ArmRef) -> ArmSettings {
        self.arms
            .iter()
            .find(|(a, _)| *a == arm)
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    pub fn corner(&self, from: ArmRef) -> CornerSettings {
        self.corners
            .iter()
            .find(|(a, _)| *a == from)
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    /// Changes an arm's settings.
    pub fn set_arm(&mut self, arm: ArmRef, s: ArmSettings) {
        self.arms.retain(|(a, _)| *a != arm);
        if s != ArmSettings::default() {
            self.arms.push((arm, s));
            self.arms.sort_by_key(|(a, _)| *a);
        }
    }

    /// Changes a corner's settings.
    pub fn set_corner(&mut self, from: ArmRef, s: CornerSettings) {
        self.corners.retain(|(a, _)| *a != from);
        if s != CornerSettings::default() {
            self.corners.push((from, s));
            self.corners.sort_by_key(|(a, _)| *a);
        }
    }

    /// As a line of text, the streets named by `name` (their GlobalId in
    /// the IFC): `nth;kind;road:end,…;arm road:f setback|…;corner …`.
    pub fn encode(&self, name: impl Fn(RoadId) -> String) -> String {
        let arm = |a: &ArmRef| format!("{}:{}", name(a.road), if a.forward { 'F' } else { 'B' });
        let roads: Vec<String> = self
            .key
            .roads
            .iter()
            .map(|(r, e)| format!("{}:{}", name(*r), e.code()))
            .collect();
        let opt = |v: Option<f64>| v.map(|r| r.to_string()).unwrap_or_default();
        let arms: Vec<String> = self
            .arms
            .iter()
            .map(|(a, s)| {
                let turns = s
                    .turns
                    .as_ref()
                    .map(|t| {
                        t.iter()
                            .map(|x| x.0.to_string())
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .unwrap_or_default();
                format!(
                    "{}={}/{}/{}/{}",
                    arm(a),
                    opt(s.setback),
                    s.sign.map(|k| k.code()).unwrap_or_default(),
                    // «-» for none: an empty list is a lane-less «.».
                    if s.turns.is_some() {
                        format!("t{turns}")
                    } else {
                        String::new()
                    },
                    opt(s.stop_back)
                )
            })
            .collect();
        let corners: Vec<String> = self
            .corners
            .iter()
            .map(|(a, s)| {
                format!(
                    "{}={}/{}/{}/{}",
                    arm(a),
                    opt(s.radius),
                    s.shape.map(|s| s.code()).unwrap_or_default(),
                    match s.sidewalk {
                        Some(true) => "1",
                        Some(false) => "0",
                        None => "",
                    },
                    opt(s.bump)
                )
            })
            .collect();
        let flag = |v: Option<bool>| match v {
            Some(true) => "1",
            Some(false) => "0",
            None => "",
        };
        let links: Vec<String> = self
            .links
            .iter()
            .map(|l| format!("{}.{}>{}.{}", arm(&l.from), l.lane, arm(&l.to), l.to_lane))
            .collect();
        let islands: Vec<String> = self
            .islands
            .iter()
            .map(|i| {
                let pts: Vec<String> = i
                    .points
                    .iter()
                    .map(|q| format!("{},{}", q.x, q.y))
                    .collect();
                format!("{}:{}", i.paint.code(), pts.join(" "))
            })
            .collect();
        format!(
            "{};{};{};{};{};{};{};{};{};{};{}",
            self.key.nth,
            self.kind.map(|k| k.code()).unwrap_or_default(),
            roads.join(","),
            arms.join("|"),
            corners.join("|"),
            flag(self.flat),
            self.main.map(&name).unwrap_or_default(),
            flag(self.hide_marks),
            opt(self.end_radius),
            links.join("|"),
            islands.join("|")
        )
    }

    /// Reads [`NodeSettings::encode`], the streets found by `road`.
    pub fn decode(
        text: &str,
        level: LevelId,
        road: impl Fn(&str) -> Option<RoadId>,
    ) -> Option<NodeSettings> {
        let mut f = text.split(';');
        let nth = f.next()?.parse().ok()?;
        let kind = match f.next()? {
            "" => None,
            k => Some(NodeKind::of_code(k)?),
        };
        let arm = |t: &str| {
            let (r, d) = t.rsplit_once(':')?;
            Some(ArmRef {
                road: road(r)?,
                forward: d == "F",
            })
        };
        let mut roads = Vec::new();
        for r in f.next()?.split(',').filter(|r| !r.is_empty()) {
            let (g, e) = r.rsplit_once(':')?;
            roads.push((road(g)?, NodeEnd::of_code(e)?));
        }
        roads.sort();
        let num = |t: Option<&str>| -> Option<Option<f64>> {
            match t.unwrap_or("") {
                "" => Some(None),
                x => Some(Some(x.parse().ok()?)),
            }
        };
        let mut arms = Vec::new();
        for a in f.next().unwrap_or("").split('|').filter(|a| !a.is_empty()) {
            let (r, v) = a.split_once('=')?;
            let mut p = v.split('/');
            let setback = num(p.next())?;
            let sign = match p.next().unwrap_or("") {
                "" => None,
                x => Some(ArmSign::of_code(x)?),
            };
            let turns = match p.next().unwrap_or("") {
                "" => None,
                x => {
                    let x = x.strip_prefix('t')?;
                    let mut v = Vec::new();
                    for n in x.split('.').filter(|n| !n.is_empty()) {
                        v.push(Turns(n.parse().ok()?));
                    }
                    Some(v)
                }
            };
            let stop_back = num(p.next())?;
            arms.push((
                arm(r)?,
                ArmSettings {
                    setback,
                    sign,
                    turns,
                    stop_back,
                },
            ));
        }
        let mut corners = Vec::new();
        for c in f.next().unwrap_or("").split('|').filter(|c| !c.is_empty()) {
            let (r, v) = c.split_once('=')?;
            let mut p = v.split('/');
            let radius = match p.next()? {
                "" => None,
                x => Some(x.parse().ok()?),
            };
            let shape = match p.next()? {
                "" => None,
                x => Some(CornerShape::of_code(x)?),
            };
            let sidewalk = match p.next()? {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            };
            let bump = num(p.next())?;
            corners.push((
                arm(r)?,
                CornerSettings {
                    radius,
                    shape,
                    sidewalk,
                    bump,
                },
            ));
        }
        let flag = |t: Option<&str>| match t.unwrap_or("") {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        };
        let flat = flag(f.next());
        let main = match f.next().unwrap_or("") {
            "" => None,
            g => Some(road(g)?),
        };
        let hide_marks = flag(f.next());
        let end_radius = num(f.next())?;
        let mut links = Vec::new();
        for l in f.next().unwrap_or("").split('|').filter(|l| !l.is_empty()) {
            let (a, b) = l.split_once('>')?;
            let lane = |t: &str| -> Option<(ArmRef, u8)> {
                let (r, n) = t.rsplit_once('.')?;
                Some((arm(r)?, n.parse().ok()?))
            };
            let ((from, lane_in), (to, to_lane)) = (lane(a)?, lane(b)?);
            links.push(LaneLink {
                from,
                lane: lane_in,
                to,
                to_lane,
            });
        }
        let mut islands = Vec::new();
        for i in f.next().unwrap_or("").split('|').filter(|i| !i.is_empty()) {
            let (k, pts) = i.split_once(':')?;
            let mut points = Vec::new();
            for q in pts.split(' ').filter(|q| !q.is_empty()) {
                let (x, y) = q.split_once(',')?;
                points.push(Point2::new(x.parse().ok()?, y.parse().ok()?));
            }
            islands.push(PaintedIsland {
                points,
                paint: IslandPaint::of_code(k)?,
            });
        }
        Some(NodeSettings {
            key: NodeKey { level, roads, nth },
            kind,
            arms,
            corners,
            flat,
            main,
            hide_marks,
            end_radius,
            links,
            islands,
        })
    }
}

impl Model {
    /// The nodes changed by hand.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeSettings> {
        self.nodes.iter()
    }

    /// What is changed of a node (nothing if it is as worked out).
    pub fn node(&self, key: &NodeKey) -> Option<&NodeSettings> {
        self.nodes.iter().find(|n| n.key == *key)
    }

    /// Keeps what is changed of a node (and forgets it if nothing is).
    pub fn set_node(&mut self, s: NodeSettings) -> Result<(), ModelError> {
        if self.level(s.key.level).is_none() {
            return Err(ModelError::UnknownLevel(s.key.level));
        }
        for r in s.key.road_ids() {
            if self.road(r).is_none() {
                return Err(ModelError::UnknownElement(r.0));
            }
        }
        for (_, a) in &s.arms {
            if let Some(v) = a.setback {
                if !v.is_finite() {
                    return Err(ModelError::NotFinite("retranqueo"));
                }
                if v < 0.0 {
                    return Err(ModelError::NonPositive("retranqueo"));
                }
            }
        }
        for (_, a) in &s.arms {
            if let Some(v) = a.stop_back {
                if !v.is_finite() {
                    return Err(ModelError::NotFinite("línea de detención"));
                }
                if v < 0.0 {
                    return Err(ModelError::NonPositive("línea de detención"));
                }
            }
        }
        for l in &s.links {
            for a in [l.from, l.to] {
                if !s.key.road_ids().any(|r| r == a.road) {
                    return Err(ModelError::UnknownElement(a.road.0));
                }
            }
        }
        for i in &s.islands {
            if i.points.len() < 3 {
                return Err(ModelError::InvalidOutline);
            }
            if i.points
                .iter()
                .any(|q| !q.x.is_finite() || !q.y.is_finite())
            {
                return Err(ModelError::NotFinite("isleta"));
            }
        }
        if let Some(m) = s.main {
            if !s.key.road_ids().any(|r| r == m) {
                return Err(ModelError::UnknownElement(m.0));
            }
        }
        if let Some(v) = s.end_radius {
            if !v.is_finite() {
                return Err(ModelError::NotFinite("radio de giro"));
            }
            if v <= 0.0 {
                return Err(ModelError::NonPositive("radio de giro"));
            }
        }
        for (_, c) in &s.corners {
            if let Some(v) = c.bump {
                if !v.is_finite() {
                    return Err(ModelError::NotFinite("oreja"));
                }
                if v < 0.0 {
                    return Err(ModelError::NonPositive("oreja"));
                }
            }
            if let Some(v) = c.radius {
                if !v.is_finite() {
                    return Err(ModelError::NotFinite("radio de esquina"));
                }
                if v < 0.0 {
                    return Err(ModelError::NonPositive("radio de esquina"));
                }
            }
        }
        self.nodes.retain(|n| n.key != s.key);
        if !s.is_plain() {
            self.nodes.push(s);
            self.nodes.sort_by(|a, b| a.key.cmp(&b.key));
        }
        Ok(())
    }

    /// Forgets the nodes that are no longer there (`there`: the nodes the
    /// streets make now): a street taken away takes its nodes' settings
    /// with it (decision 3 of `docs/TERRITORIO.md` §5). How many it forgot.
    pub fn keep_nodes(&mut self, there: &[NodeKey]) -> usize {
        let before = self.nodes.len();
        self.nodes.retain(|n| there.contains(&n.key));
        before - self.nodes.len()
    }

    /// The nodes of streets that no longer exist taken away (cheap: without
    /// working out where the streets meet).
    pub(super) fn drop_nodes_of_gone_roads(&mut self) {
        let gone: Vec<NodeKey> = self
            .nodes
            .iter()
            .filter(|n| n.key.road_ids().any(|r| self.road(r).is_none()))
            .map(|n| n.key.clone())
            .collect();
        self.nodes.retain(|n| !gone.contains(&n.key));
    }

    pub(super) fn restore_nodes(&mut self, nodes: Vec<NodeSettings>) -> Result<(), ModelError> {
        for n in nodes {
            self.set_node(n)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_read_back() {
        let key = NodeKey {
            level: LevelId(1),
            roads: vec![(RoadId(5), NodeEnd::Through), (RoadId(7), NodeEnd::Start)],
            nth: 0,
        };
        let mut s = NodeSettings::new(key);
        assert!(s.is_plain());
        s.kind = Some(NodeKind::Raised);
        let a = ArmRef {
            road: RoadId(7),
            forward: true,
        };
        s.set_arm(
            a,
            ArmSettings {
                setback: Some(12.5),
                sign: Some(ArmSign::Stop),
                turns: Some(vec![Turns(6), Turns(3)]),
                stop_back: Some(2.0),
            },
        );
        s.set_corner(
            a,
            CornerSettings {
                radius: Some(8.0),
                shape: Some(CornerShape::Chamfer),
                sidewalk: Some(false),
                bump: Some(1.5),
            },
        );
        s.flat = Some(true);
        s.main = Some(RoadId(5));
        s.hide_marks = Some(true);
        s.end_radius = Some(9.0);
        s.links.push(LaneLink {
            from: a,
            lane: 1,
            to: ArmRef {
                road: RoadId(5),
                forward: false,
            },
            to_lane: 0,
        });
        s.islands.push(PaintedIsland {
            points: vec![
                Point2::new(1.5, -2.0),
                Point2::new(6.25, -2.0),
                Point2::new(4.0, 3.0),
            ],
            paint: IslandPaint::Fill,
        });
        assert!(!s.is_plain());
        let text = s.encode(|r| format!("g{}", r.0));
        let back = NodeSettings::decode(&text, LevelId(1), |g| {
            g.strip_prefix('g')?.parse().ok().map(RoadId)
        })
        .unwrap();
        assert_eq!(back, s);
        // As the first version wrote it (0.94: setbacks and three corner
        // fields, nothing after them).
        let old = "0;raised;g5:T,g7:S;g7:F=12.5;g7:F=8/chamfer/0".to_string();
        let read = NodeSettings::decode(&old, LevelId(1), |g| {
            g.strip_prefix('g')?.parse().ok().map(RoadId)
        })
        .unwrap();
        assert_eq!(read.arm(a).setback, Some(12.5));
        assert_eq!(read.corner(a).shape, Some(CornerShape::Chamfer));
        assert!(read.flat.is_none() && read.end_radius.is_none());
    }
}
