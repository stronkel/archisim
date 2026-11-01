//! The Spanish cadastre's INSPIRE files (Catastro, «servicio ATOM» or the
//! download of a municipality): buildings and their parts (BU), with how
//! many floors each part has over and under ground, and cadastral parcels
//! (CP) with their reference. Coordinates as the file gives them: eastings
//! and northings of its system (`srsName`, ETRS89 / UTM).

use crate::xml::{events, local, Event};

/// A polygon: its outer ring and its holes, each a ring of points
/// (eastings, northings), not repeating the first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Polygon {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// What a feature of the cadastre is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Building,
    BuildingPart,
    Parcel,
    /// Anything else with a geometry (zones, other features).
    Other,
}

/// A building, a part of one or a parcel.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub kind: FeatureKind,
    /// Its identifier (`gml:id`, or the cadastral reference of a parcel).
    pub id: String,
    pub reference: String,
    pub polygons: Vec<Polygon>,
    pub floors_above: Option<u32>,
    pub floors_below: Option<u32>,
    /// A building's use, as the file says it (`1_residential`…).
    pub current_use: String,
}

/// What a cadastral file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cadastre {
    /// EPSG code of its coordinates, from its `srsName` (25830…).
    pub epsg: Option<u32>,
    pub features: Vec<Feature>,
}

/// The EPSG code in an `srsName`: `urn:ogc:def:crs:EPSG::25830`,
/// `http://www.opengis.net/def/crs/EPSG/0/25830`, `EPSG:25830`.
pub fn epsg_of(srs: &str) -> Option<u32> {
    let i = srs.to_ascii_uppercase().rfind("EPSG")?;
    let tail = &srs[i + 4..];
    let digits: String = tail
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    // `EPSG/0/25830`: skip the version.
    let rest = tail.trim_start_matches(|c: char| !c.is_ascii_digit());
    if digits == "0" && rest.len() > 1 {
        return epsg_of(&format!("EPSG:{}", &rest[1..]));
    }
    digits.parse().ok()
}

fn parse_coords(text: &str, dim: usize) -> Vec<[f64; 2]> {
    let v: Vec<f64> = text
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    let mut ring: Vec<[f64; 2]> = v
        .chunks(dim.max(2))
        .filter(|c| c.len() >= 2)
        .map(|c| [c[0], c[1]])
        .collect();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    ring
}

/// Reads a cadastral GML.
pub fn read(text: &str) -> Cadastre {
    let mut out = Cadastre::default();
    let mut current: Option<Feature> = None;
    let mut depth_of_feature = 0usize;
    let mut depth = 0usize;
    // Where we are: in an exterior or an interior ring, in which element.
    let mut ring_kind: Option<bool> = None; // Some(true): exterior
    let mut field: Option<String> = None;
    let mut dim = 2usize;
    for e in events(text) {
        match e {
            Event::Start { name, attrs, .. } => {
                depth += 1;
                for (k, v) in &attrs {
                    if local(k) == "srsName" && out.epsg.is_none() {
                        out.epsg = epsg_of(v);
                    }
                    if local(k) == "srsDimension" {
                        dim = v.parse().unwrap_or(2);
                    }
                }
                let l = local(name);
                let kind = match l {
                    "Building" => Some(FeatureKind::Building),
                    "BuildingPart" => Some(FeatureKind::BuildingPart),
                    "CadastralParcel" => Some(FeatureKind::Parcel),
                    _ => None,
                };
                if let (Some(kind), None) = (kind, &current) {
                    let id = attrs
                        .iter()
                        .find(|(k, _)| local(k) == "id")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    current = Some(Feature {
                        kind,
                        id,
                        reference: String::new(),
                        polygons: Vec::new(),
                        floors_above: None,
                        floors_below: None,
                        current_use: String::new(),
                    });
                    depth_of_feature = depth;
                }
                match l {
                    "exterior" | "outerBoundaryIs" => ring_kind = Some(true),
                    "interior" | "innerBoundaryIs" => ring_kind = Some(false),
                    _ => {}
                }
                field = Some(l.to_string());
            }
            Event::End(name) => {
                let l = local(name);
                if matches!(
                    l,
                    "exterior" | "interior" | "outerBoundaryIs" | "innerBoundaryIs"
                ) {
                    ring_kind = None;
                }
                if depth == depth_of_feature && current.is_some() {
                    if let Some(f) = current.take() {
                        out.features.push(f);
                    }
                }
                depth = depth.saturating_sub(1);
                field = None;
            }
            Event::Text(t) => {
                let (Some(f), Some(name)) = (current.as_mut(), field.as_deref()) else {
                    continue;
                };
                match name {
                    "posList" | "coordinates" => {
                        let ring = if name == "coordinates" {
                            parse_coords(&t.replace(',', " "), 2)
                        } else {
                            parse_coords(&t, dim)
                        };
                        if ring.len() < 3 {
                            continue;
                        }
                        match ring_kind {
                            Some(false) => {
                                if let Some(p) = f.polygons.last_mut() {
                                    p.holes.push(ring);
                                }
                            }
                            _ => f.polygons.push(Polygon {
                                outer: ring,
                                holes: Vec::new(),
                            }),
                        }
                    }
                    "numberOfFloorsAboveGround" => f.floors_above = t.trim().parse().ok(),
                    "numberOfFloorsBelowGround" => f.floors_below = t.trim().parse().ok(),
                    "currentUse" => f.current_use = t.trim().to_string(),
                    "nationalCadastralReference" => f.reference = t.trim().to_string(),
                    "localId" if f.reference.is_empty() => f.reference = t.trim().to_string(),
                    _ => {}
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml/3.2" xmlns:bu-ext2d="x">
<gml:featureMember>
<bu-ext2d:BuildingPart gml:id="ES.SDGC.BU.0000001AA0000A_part1">
<bu-core2d:localId>0000001AA0000A_part1</bu-core2d:localId>
<bu-ext2d:geometry><gml:Surface gml:id="s1" srsName="http://www.opengis.net/def/crs/EPSG/0/25829"><gml:patches><gml:PolygonPatch>
<gml:exterior><gml:LinearRing><gml:posList srsDimension="2" count="5">0 0 10 0 10 8 0 8 0 0</gml:posList></gml:LinearRing></gml:exterior>
<gml:interior><gml:LinearRing><gml:posList srsDimension="2" count="5">4 3 6 3 6 5 4 5 4 3</gml:posList></gml:LinearRing></gml:interior>
</gml:PolygonPatch></gml:patches></gml:Surface></bu-ext2d:geometry>
<bu-ext2d:numberOfFloorsAboveGround>3</bu-ext2d:numberOfFloorsAboveGround>
<bu-ext2d:numberOfFloorsBelowGround>1</bu-ext2d:numberOfFloorsBelowGround>
</bu-ext2d:BuildingPart>
</gml:featureMember>
<gml:featureMember>
<cp:CadastralParcel gml:id="ES.SDGC.CP.0000001AA0000A">
<cp:geometry><gml:MultiSurface><gml:surfaceMember><gml:Surface><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>
<gml:posList srsDimension="2">-2 -2 12 -2 12 10 -2 10 -2 -2</gml:posList></gml:LinearRing></gml:exterior></gml:PolygonPatch></gml:patches></gml:Surface></gml:surfaceMember></gml:MultiSurface></cp:geometry>
<cp:nationalCadastralReference>0000001AA0000A</cp:nationalCadastralReference>
</cp:CadastralParcel>
</gml:featureMember>
</gml:FeatureCollection>"#;

    #[test]
    fn reads_a_building_part_and_a_parcel() {
        let c = read(PART);
        assert_eq!(c.epsg, Some(25829));
        assert_eq!(c.features.len(), 2);
        let b = &c.features[0];
        assert_eq!(b.kind, FeatureKind::BuildingPart);
        assert_eq!(b.floors_above, Some(3));
        assert_eq!(b.floors_below, Some(1));
        assert_eq!(b.polygons.len(), 1);
        assert_eq!(b.polygons[0].outer.len(), 4);
        assert_eq!(b.polygons[0].holes.len(), 1);
        let p = &c.features[1];
        assert_eq!(p.kind, FeatureKind::Parcel);
        assert_eq!(p.reference, "0000001AA0000A");
        assert_eq!(epsg_of("urn:ogc:def:crs:EPSG::25830"), Some(25830));
        assert_eq!(epsg_of("EPSG:25831"), Some(25831));
    }
}
