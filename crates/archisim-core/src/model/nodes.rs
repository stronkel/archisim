//! The nodes of the streets (N1, `docs/TERRITORIO.md` §2): where streets
//! meet, as Node Controller does it in Cities: Skylines. The junctions are
//! worked out from the streets each time (`archisim_geom::crossroads`);
//! here is only what Antonio changes of one of them, kept by a key that
//! does not depend on where the streets are exactly — the same node while
//! the same streets meet there in the same way.

use super::{LevelId, Model, ModelError, RoadId};

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
}

impl NodeKind {
    pub const ALL: [NodeKind; 4] = [
        NodeKind::Crossing,
        NodeKind::Raised,
        NodeKind::Shared,
        NodeKind::Open,
    ];

    /// Its name for Antonio.
    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Crossing => "Cruce",
            NodeKind::Raised => "Cruce elevado",
            NodeKind::Shared => "Plataforma única",
            NodeKind::Open => "Sin nudo",
        }
    }

    fn code(self) -> &'static str {
        match self {
            NodeKind::Crossing => "crossing",
            NodeKind::Raised => "raised",
            NodeKind::Shared => "shared",
            NodeKind::Open => "open",
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
}

impl NodeSettings {
    /// A node with nothing changed.
    pub fn new(key: NodeKey) -> Self {
        NodeSettings {
            key,
            kind: None,
            arms: Vec::new(),
            corners: Vec::new(),
        }
    }

    /// Whether nothing is changed.
    pub fn is_plain(&self) -> bool {
        self.kind.is_none_or(|k| k == NodeKind::Crossing)
            && self.arms.iter().all(|(_, a)| a.setback.is_none())
            && self.corners.iter().all(|(_, c)| {
                c.radius.is_none()
                    && c.shape.is_none_or(|s| s == CornerShape::Arc)
                    && c.sidewalk.is_none_or(|s| s)
            })
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
        let arms: Vec<String> = self
            .arms
            .iter()
            .filter_map(|(a, s)| Some(format!("{}={}", arm(a), s.setback?)))
            .collect();
        let corners: Vec<String> = self
            .corners
            .iter()
            .map(|(a, s)| {
                format!(
                    "{}={}/{}/{}",
                    arm(a),
                    s.radius.map(|r| r.to_string()).unwrap_or_default(),
                    s.shape.map(|s| s.code()).unwrap_or_default(),
                    match s.sidewalk {
                        Some(true) => "1",
                        Some(false) => "0",
                        None => "",
                    }
                )
            })
            .collect();
        format!(
            "{};{};{};{};{}",
            self.key.nth,
            self.kind.map(|k| k.code()).unwrap_or_default(),
            roads.join(","),
            arms.join("|"),
            corners.join("|")
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
        let mut arms = Vec::new();
        for a in f.next().unwrap_or("").split('|').filter(|a| !a.is_empty()) {
            let (r, v) = a.split_once('=')?;
            arms.push((
                arm(r)?,
                ArmSettings {
                    setback: Some(v.parse().ok()?),
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
            corners.push((
                arm(r)?,
                CornerSettings {
                    radius,
                    shape,
                    sidewalk,
                },
            ));
        }
        Some(NodeSettings {
            key: NodeKey { level, roads, nth },
            kind,
            arms,
            corners,
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
        for (_, c) in &s.corners {
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
            },
        );
        s.set_corner(
            a,
            CornerSettings {
                radius: Some(8.0),
                shape: Some(CornerShape::Chamfer),
                sidewalk: Some(false),
            },
        );
        assert!(!s.is_plain());
        let text = s.encode(|r| format!("g{}", r.0));
        let back = NodeSettings::decode(&text, LevelId(1), |g| {
            g.strip_prefix('g')?.parse().ok().map(RoadId)
        })
        .unwrap();
        assert_eq!(back, s);
    }
}
