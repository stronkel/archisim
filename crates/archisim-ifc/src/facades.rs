//! The pieces of the façades (E9): on each wall whose type has a façade,
//! one `IfcBuildingElementPart` per material (and another for the band
//! over the slab edges), a triangulated body with every piece, part of
//! the wall (`IfcRelAggregates`), with `ArchiSim_FacadePart` (how many
//! whole and cut pieces) so a file read back does not take it for an
//! element of its own. The façade itself goes as text in the type's
//! `ArchiSim_WallType`.

use std::collections::BTreeMap;

use archisim_core::{
    Bond, Coursing, Facade, FacadeSystem, LevelId, MaterialId, Model, Pattern, SlabFront,
};

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_facade_parts(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
    ) {
        let model = self.model;
        let walls: Vec<_> = model
            .walls()
            .filter(|w| model.wall_facade(w.id).is_some())
            .collect();
        if walls.is_empty() {
            return;
        }
        let idx = archisim_geom::WallIndex::new(model);
        for w in walls {
            let Some(&host) = self.hosts.get(&w.id.0) else {
                continue;
            };
            let Some(&(_, storey_pl)) = storeys.get(&w.level) else {
                continue;
            };
            let solids = archisim_geom::solid::wall_solids_in(&idx, w.id);
            let Some(layout) = archisim_geom::facade::facade_layout(&idx, w.id, &solids) else {
                continue;
            };
            let type_name = model
                .wall_type(w.wall_type)
                .map_or(String::new(), |t| t.name.clone());
            let storey_z = model.level(w.level).map_or(0.0, |l| l.elevation);
            let counts = archisim_geom::facade::piece_counts(&layout);
            let mut parts = Vec::new();
            for (material, front, whole, cut) in counts {
                let pieces = archisim_geom::facade::FacadeLayout {
                    pieces: layout
                        .pieces
                        .iter()
                        .filter(|p| p.material == material && p.slab_front == front)
                        .cloned()
                        .collect(),
                    ..layout.clone()
                };
                let solids = archisim_geom::facade::facade_solids(&pieces);
                let mut pts: Vec<String> = Vec::new();
                let mut index: BTreeMap<[i64; 3], usize> = BTreeMap::new();
                let mut faces = Vec::new();
                for s in &solids {
                    for t in &s.triangles {
                        let mut f = [0usize; 3];
                        for (k, p) in t.iter().enumerate() {
                            let key = [
                                (p[0] * 1e5).round() as i64,
                                (p[1] * 1e5).round() as i64,
                                (p[2] * 1e5).round() as i64,
                            ];
                            f[k] = *index.entry(key).or_insert_with(|| {
                                pts.push(format!(
                                    "({},{},{})",
                                    real(p[0]),
                                    real(p[1]),
                                    real(p[2] - storey_z)
                                ));
                                pts.len()
                            });
                        }
                        if f[0] != f[1] && f[1] != f[2] && f[0] != f[2] {
                            faces.push(format!("({},{},{})", f[0], f[1], f[2]));
                        }
                    }
                }
                if faces.is_empty() {
                    continue;
                }
                let guid = derived_guid(
                    &w.guid,
                    &format!("facade {} {}", u8::from(front), material.0),
                );
                let list3 = self
                    .w
                    .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
                let body = self.w.add(format!(
                    "IFCTRIANGULATEDFACESET({list3},$,$,({}),$)",
                    faces.join(",")
                ));
                if let Some(&style) = self.styles.get(&material) {
                    self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
                }
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({body}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let name = model.material(material).map_or("", |m| m.name.as_str());
                let what = if front {
                    "Frente de forjado"
                } else {
                    "Aparejo"
                };
                let el = self.w.add(format!(
                    "IFCBUILDINGELEMENTPART({},$,{},$,{},{pl},{shape},$,.USERDEFINED.)",
                    string(guid.as_str()),
                    string(&format!("{what} · {name}")),
                    string(what),
                ));
                if let Some(m) = materials.get(&material) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
                let props = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('FromType',$,IFCLABEL({}),$)",
                        string(&type_name)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('WholePieces',$,IFCCOUNTMEASURE({whole}),$)"
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('CutPieces',$,IFCCOUNTMEASURE({cut}),$)"
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('SlabFront',$,IFCBOOLEAN({}),$)",
                        if front { ".T." } else { ".F." }
                    )),
                ];
                self.pset(el, &guid, "ArchiSim_FacadePart", &props);
                parts.push(el);
            }
            if !parts.is_empty() {
                let list: Vec<String> = parts.iter().map(|r| r.to_string()).collect();
                self.w.add(format!(
                    "IFCRELAGGREGATES({},$,'Aparejo de fachada',$,{host},({}))",
                    string(derived_guid(&w.guid, "facade parts").as_str()),
                    list.join(",")
                ));
            }
        }
    }
}

fn name_of(model: &Model, m: Option<MaterialId>) -> String {
    m.and_then(|m| model.material(m))
        .map_or(String::new(), |m| m.name.clone())
}

fn encode_coursing(model: &Model, c: &Coursing, prefix: &str, out: &mut Vec<String>) {
    let pattern = match c.pattern {
        Pattern::Plain => "plain".to_string(),
        Pattern::Headers => "headers".to_string(),
        Pattern::Courses(n) => format!("courses {n}"),
        Pattern::Diamonds(n) => format!("diamonds {n}"),
    };
    out.push(format!("{prefix}system={}", c.system.key()));
    out.push(format!(
        "{prefix}piece={} {} {}",
        real(c.length),
        real(c.height),
        real(c.depth)
    ));
    out.push(format!("{prefix}joint={}", real(c.joint)));
    out.push(format!("{prefix}bond={}", c.bond.key()));
    out.push(format!(
        "{prefix}joint_material={}",
        name_of(model, c.joint_material)
    ));
    out.push(format!("{prefix}accent={}", name_of(model, c.accent)));
    out.push(format!("{prefix}pattern={pattern}"));
    out.push(format!("{prefix}soldiers={}", u8::from(c.soldier_heads)));
}

/// A façade as text, a `key=value` a line, materials by name.
pub(crate) fn encode_facade(model: &Model, f: &Facade) -> String {
    let mut out = Vec::new();
    encode_coursing(model, &f.coursing, "", &mut out);
    if let Some(s) = &f.slab_front {
        encode_coursing(model, &s.coursing, "front.", &mut out);
        out.push(format!("front.material={}", name_of(model, s.material)));
        out.push(format!("front.projection={}", real(s.projection)));
        out.push(format!("front.over={}", real(s.over)));
    }
    out.join("\n")
}

fn decode_coursing(model: &Model, map: &BTreeMap<&str, &str>, prefix: &str) -> Option<Coursing> {
    let get = |k: &str| map.get(format!("{prefix}{k}").as_str()).copied();
    let material = |k: &str| {
        let name = get(k)?;
        model.materials().find(|m| m.name == name).map(|m| m.id)
    };
    let mut piece = get("piece")?
        .split_whitespace()
        .map(|v| v.parse::<f64>().ok());
    let (length, height, depth) = (piece.next()??, piece.next()??, piece.next()??);
    let pattern = match get("pattern").map(|p| p.split_once(' ').unwrap_or((p, ""))) {
        Some(("headers", _)) => Pattern::Headers,
        Some(("courses", n)) => Pattern::Courses(n.parse().unwrap_or(4)),
        Some(("diamonds", n)) => Pattern::Diamonds(n.parse().unwrap_or(3)),
        _ => Pattern::Plain,
    };
    Some(Coursing {
        system: get("system")
            .and_then(FacadeSystem::from_key)
            .unwrap_or_default(),
        length,
        height,
        depth,
        joint: get("joint")?.parse().ok()?,
        bond: get("bond").and_then(Bond::from_key).unwrap_or_default(),
        joint_material: material("joint_material"),
        accent: material("accent"),
        pattern,
        soldier_heads: get("soldiers") == Some("1"),
    })
}

/// A façade from its text.
pub(crate) fn decode_facade(model: &Model, text: &str) -> Option<Facade> {
    let map: BTreeMap<&str, &str> = text.lines().filter_map(|l| l.split_once('=')).collect();
    let coursing = decode_coursing(model, &map, "")?;
    let slab_front = decode_coursing(model, &map, "front.").map(|c| SlabFront {
        coursing: c,
        material: map
            .get("front.material")
            .and_then(|n| model.materials().find(|m| m.name == *n))
            .map(|m| m.id),
        projection: map
            .get("front.projection")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        over: map
            .get("front.over")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
    });
    Some(Facade {
        coursing,
        slab_front,
    })
}
