//! OpenStreetMap's XML (`.osm`, as «Exportar» of openstreetmap.org or
//! Overpass give it): its nodes with their latitude and longitude, and its
//! ways as lists of nodes, each with its tags (`highway`, `building`,
//! `building:levels`, `natural`…).

use std::collections::HashMap;

use crate::xml::{events, Event};

/// A node: degrees, north and east positive.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub tags: Vec<(String, String)>,
}

/// A way: its points (latitude, longitude) in order; closed if its first
/// node is its last.
#[derive(Debug, Clone, PartialEq)]
pub struct Way {
    pub id: i64,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
    pub tags: Vec<(String, String)>,
}

impl Way {
    pub fn tag(&self, k: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(x, _)| x == k)
            .map(|(_, v)| v.as_str())
    }
}

impl Node {
    pub fn tag(&self, k: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(x, _)| x == k)
            .map(|(_, v)| v.as_str())
    }
}

/// What an OSM file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Osm {
    /// The nodes that have tags (trees, benches…); the others only give
    /// their place to the ways.
    pub nodes: Vec<Node>,
    pub ways: Vec<Way>,
}

/// A way being read: its id, its nodes' ids and its tags.
type OpenWay = (i64, Vec<i64>, Vec<(String, String)>);

/// Reads an OSM XML file.
pub fn read(text: &str) -> Osm {
    let mut out = Osm::default();
    let mut coords: HashMap<i64, [f64; 2]> = HashMap::new();
    let mut node: Option<Node> = None;
    let mut way: Option<OpenWay> = None;
    let attr = |attrs: &[(&str, String)], k: &str| -> Option<String> {
        attrs.iter().find(|(x, _)| *x == k).map(|(_, v)| v.clone())
    };
    for e in events(text) {
        match e {
            Event::Start { name, attrs, .. } => match name {
                "node" => {
                    let id = attr(&attrs, "id").and_then(|v| v.parse().ok()).unwrap_or(0);
                    let lat = attr(&attrs, "lat").and_then(|v| v.parse().ok());
                    let lon = attr(&attrs, "lon").and_then(|v| v.parse().ok());
                    if let (Some(lat), Some(lon)) = (lat, lon) {
                        coords.insert(id, [lat, lon]);
                        node = Some(Node {
                            id,
                            lat,
                            lon,
                            tags: Vec::new(),
                        });
                    }
                }
                "way" => {
                    let id = attr(&attrs, "id").and_then(|v| v.parse().ok()).unwrap_or(0);
                    way = Some((id, Vec::new(), Vec::new()));
                }
                "nd" => {
                    if let (Some(w), Some(r)) = (
                        way.as_mut(),
                        attr(&attrs, "ref").and_then(|v| v.parse().ok()),
                    ) {
                        w.1.push(r);
                    }
                }
                "tag" => {
                    if let (Some(k), Some(v)) = (attr(&attrs, "k"), attr(&attrs, "v")) {
                        if let Some(w) = way.as_mut() {
                            w.2.push((k, v));
                        } else if let Some(n) = node.as_mut() {
                            n.tags.push((k, v));
                        }
                    }
                }
                _ => {}
            },
            Event::End(name) => match name {
                "node" => {
                    if let Some(n) = node.take() {
                        if !n.tags.is_empty() {
                            out.nodes.push(n);
                        }
                    }
                }
                "way" => {
                    if let Some((id, refs, tags)) = way.take() {
                        let points: Vec<[f64; 2]> =
                            refs.iter().filter_map(|r| coords.get(r).copied()).collect();
                        let closed = refs.len() > 2 && refs.first() == refs.last();
                        let mut points = points;
                        if closed {
                            points.pop();
                        }
                        if points.len() >= 2 {
                            out.ways.push(Way {
                                id,
                                points,
                                closed,
                                tags,
                            });
                        }
                    }
                }
                _ => {}
            },
            Event::Text(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nodes_ways_and_tags() {
        let o = read(
            r#"<osm version="0.6">
<node id="1" lat="37.2" lon="-6.95"/>
<node id="2" lat="37.2" lon="-6.949"/>
<node id="3" lat="37.201" lon="-6.949"/>
<node id="4" lat="37.2005" lon="-6.9495"><tag k="natural" v="tree"/></node>
<way id="10"><nd ref="1"/><nd ref="2"/><tag k="highway" v="residential"/><tag k="name" v="Calle Ancha"/></way>
<way id="11"><nd ref="1"/><nd ref="2"/><nd ref="3"/><nd ref="1"/><tag k="building" v="yes"/><tag k="building:levels" v="3"/></way>
</osm>"#,
        );
        assert_eq!(o.nodes.len(), 1);
        assert_eq!(o.nodes[0].tag("natural"), Some("tree"));
        assert_eq!(o.ways.len(), 2);
        assert_eq!(o.ways[0].tag("name"), Some("Calle Ancha"));
        assert!(!o.ways[0].closed);
        assert!(o.ways[1].closed);
        assert_eq!(o.ways[1].points.len(), 3);
    }
}
