//! What a roof carries (`docs/MAQUETA.md` §5):
//! - each dormer, an `IfcElementAssembly` (`ObjectType` «Buhardilla») made
//!   of its front and cheeks (`IfcWall`), its roof (`IfcSlab` `ROOF`) and
//!   its window (`IfcWindow`); a roof window, an `IfcWindow` `SKYLIGHT`;
//! - the tiles, an `IfcCovering` (`ROOFING`) covering the roof;
//! - each gutter, an `IfcPipeSegment` `GUTTER`, and each downpipe an
//!   `IfcPipeSegment` `RIGIDSEGMENT`.
//!
//! The roof's planes already have the dormers' holes. Every piece carries
//! `ArchiSim_RoofPart`, and the roof `ArchiSim_RoofParts` (the dormers and
//! the covering as text), so ArchiSim reads it all back with the roof and
//! not as elements of their own.

use std::collections::BTreeMap;

use archisim_core::{DormerKind, GlobalId, LevelId, MaterialId, Model, Roof};
use archisim_geom::roof_parts::{dormer_parts, gutter_pieces, tile_solids, DormerPart};
use archisim_geom::solid::Solid;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    /// A tessellated body of solids, one face set per material, heights
    /// from the storey; `None` if they have no triangles.
    pub(crate) fn solids_shape(&mut self, solids: &[&Solid], storey_z: f64) -> Option<Ref> {
        let mut by_material: BTreeMap<MaterialId, Vec<&Solid>> = BTreeMap::new();
        for s in solids {
            by_material.entry(s.material).or_default().push(s);
        }
        let mut items = Vec::new();
        for (material, list) in by_material {
            let mut pts: Vec<String> = Vec::new();
            let mut index: BTreeMap<[i64; 3], usize> = BTreeMap::new();
            let mut faces: Vec<String> = Vec::new();
            for s in list {
                for t in &s.triangles {
                    let mut ids = [0usize; 3];
                    for (k, p) in t.iter().enumerate() {
                        let key = [
                            (p[0] * 1e6).round() as i64,
                            (p[1] * 1e6).round() as i64,
                            ((p[2] - storey_z) * 1e6).round() as i64,
                        ];
                        let n = index.len();
                        ids[k] = *index.entry(key).or_insert_with(|| {
                            pts.push(format!(
                                "({},{},{})",
                                real(p[0]),
                                real(p[1]),
                                real(p[2] - storey_z)
                            ));
                            n
                        });
                    }
                    if ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] {
                        faces.push(format!("({},{},{})", ids[0] + 1, ids[1] + 1, ids[2] + 1));
                    }
                }
            }
            if faces.is_empty() {
                continue;
            }
            let list3 = self
                .w
                .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
            let body = self.w.add(format!(
                "IFCTRIANGULATEDFACESET({list3},$,.T.,({}),$)",
                faces.join(",")
            ));
            if let Some(&style) = self.styles.get(&material) {
                self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
            }
            items.push(body);
        }
        if items.is_empty() {
            return None;
        }
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',{})",
            self.body_ctx,
            crate::step::list(&items)
        ));
        Some(
            self.w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))")),
        )
    }

    fn roof_part_pset(&mut self, el: Ref, guid: &GlobalId, roof: &Roof, what: &str) {
        let props = [
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Roof',$,IFCLABEL({}),$)",
                string(roof.guid.as_str())
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Part',$,IFCLABEL({}),$)",
                string(what)
            )),
        ];
        self.pset(el, guid, "ArchiSim_RoofPart", &props);
    }

    pub(crate) fn material_of(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        el: Ref,
        guid: &GlobalId,
        m: MaterialId,
    ) {
        if let Some(r) = materials.get(&m) {
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{r})",
                string(derived_guid(guid, "material").as_str())
            ));
        }
    }

    pub(crate) fn write_roof_parts(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for roof in model.roofs() {
            let Some(&(_, storey_pl)) = storeys.get(&roof.level) else {
                continue;
            };
            let storey_z = model.level(roof.level).map_or(0.0, |l| l.elevation);
            let host = self.hosts.get(&roof.id.0).copied();
            // What ArchiSim needs to read them back.
            if let Some(h) = host {
                if !roof.dormers.is_empty() || roof.covering != Default::default() {
                    let p = [
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Dormers',$,IFCTEXT({}),$)",
                            string(&encode_dormers(model, roof))
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Covering',$,IFCTEXT({}),$)",
                            string(&encode_covering(model, roof))
                        )),
                    ];
                    self.pset(h, &roof.guid, "ArchiSim_RoofParts", &p);
                }
            }
            // Dormers and roof windows.
            for (i, d) in roof.dormers.iter().enumerate() {
                let parts = dormer_parts(model, roof.id, i);
                if parts.is_empty() {
                    continue;
                }
                let of = |kinds: &[DormerPart]| -> Vec<&Solid> {
                    parts
                        .iter()
                        .filter(|(p, _)| kinds.contains(p))
                        .map(|(_, s)| s)
                        .collect()
                };
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let window_size = |s: &[&Solid]| -> (f64, f64) {
                    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                    for t in s.iter().flat_map(|s| s.triangles.iter()).flatten() {
                        for k in 0..3 {
                            lo[k] = lo[k].min(t[k]);
                            hi[k] = hi[k].max(t[k]);
                        }
                    }
                    (
                        (hi[2] - lo[2]).max(0.0),
                        (hi[0] - lo[0]).hypot(hi[1] - lo[1]),
                    )
                };
                if d.kind == DormerKind::Skylight {
                    let solids = of(&[DormerPart::Frame, DormerPart::Glass]);
                    let (h, w) = window_size(&solids);
                    let Some(shape) = self.solids_shape(&solids, storey_z) else {
                        continue;
                    };
                    let el = self.w.add(format!(
                        "IFCWINDOW({},$,{},$,$,{pl},{shape},$,{},{},.SKYLIGHT.,.SINGLE_PANEL.,$)",
                        string(d.guid.as_str()),
                        string(d.kind.name()),
                        real(h),
                        real(w.min(d.width)),
                    ));
                    self.material_of(materials, el, &d.guid, d.frame_material);
                    self.roof_part_pset(el, &d.guid, roof, "Dormer");
                    contained.entry(roof.level).or_default().push(el);
                    continue;
                }
                let assembly = self.w.add(format!(
                    "IFCELEMENTASSEMBLY({},$,{},$,'Buhardilla',{pl},$,$,.NOTDEFINED.,.USERDEFINED.)",
                    string(d.guid.as_str()),
                    string(d.kind.name()),
                ));
                self.roof_part_pset(assembly, &d.guid, roof, "Dormer");
                let mut members = Vec::new();
                let pieces: [(&str, DormerPart, &[DormerPart]); 3] = [
                    ("walls", DormerPart::Walls, &[DormerPart::Walls]),
                    ("roof", DormerPart::Roof, &[DormerPart::Roof]),
                    (
                        "window",
                        DormerPart::Frame,
                        &[DormerPart::Frame, DormerPart::Glass],
                    ),
                ];
                for (salt, part, kinds) in pieces {
                    let solids = of(kinds);
                    let Some(shape) = self.solids_shape(&solids, storey_z) else {
                        continue;
                    };
                    let guid = derived_guid(&d.guid, salt);
                    let ppl = self.placement(Some(storey_pl), self.origin, None);
                    let (el, material) = match part {
                        DormerPart::Walls => (
                            self.w.add(format!(
                                "IFCWALL({},$,'Frente y laterales',$,$,{ppl},{shape},$,.NOTDEFINED.)",
                                string(guid.as_str())
                            )),
                            d.wall_material,
                        ),
                        DormerPart::Roof => (
                            self.w.add(format!(
                                "IFCSLAB({},$,'Cubierta de la buhardilla',$,$,{ppl},{shape},$,.ROOF.)",
                                string(guid.as_str())
                            )),
                            solids.first().map_or(d.wall_material, |s| s.material),
                        ),
                        _ => {
                            let (h, w) = window_size(&solids);
                            (
                                self.w.add(format!(
                                    "IFCWINDOW({},$,'Ventana de la buhardilla',$,$,{ppl},{shape},$,{},{},.WINDOW.,.SINGLE_PANEL.,$)",
                                    string(guid.as_str()),
                                    real(h),
                                    real(w)
                                )),
                                d.frame_material,
                            )
                        }
                    };
                    self.material_of(materials, el, &guid, material);
                    self.roof_part_pset(el, &guid, roof, "Dormer");
                    members.push(el);
                }
                self.rel_aggregates(derived_guid(&d.guid, "parts"), assembly, &members);
                contained.entry(roof.level).or_default().push(assembly);
            }
            // The tiles.
            let tiles = tile_solids(model, roof.id);
            if !tiles.is_empty() {
                let refs: Vec<&Solid> = tiles.iter().collect();
                if let Some(shape) = self.solids_shape(&refs, storey_z) {
                    let guid = derived_guid(&roof.guid, "tiles");
                    let pl = self.placement(Some(storey_pl), self.origin, None);
                    let name = roof.covering.tiles.map_or("Cubrición", |t| t.name());
                    let el = self.w.add(format!(
                        "IFCCOVERING({},$,{},$,$,{pl},{shape},$,.ROOFING.)",
                        string(guid.as_str()),
                        string(name)
                    ));
                    if let Some(m) = model.tile_material(roof) {
                        self.material_of(materials, el, &guid, m);
                    }
                    if let Some(h) = host {
                        self.w.add(format!(
                            "IFCRELCOVERSBLDGELEMENTS({},$,$,$,{h},({el}))",
                            string(derived_guid(&guid, "covers").as_str())
                        ));
                    }
                    self.roof_part_pset(el, &guid, roof, "Tiles");
                    contained.entry(roof.level).or_default().push(el);
                }
            }
            // Gutters and downpipes.
            let pieces = gutter_pieces(model, roof.id);
            let mut groups: BTreeMap<(usize, Option<usize>), Vec<&Solid>> = BTreeMap::new();
            for (run, pipe, s) in &pieces {
                groups.entry((*run, *pipe)).or_default().push(s);
            }
            for ((run, pipe), solids) in groups {
                let Some(shape) = self.solids_shape(&solids, storey_z) else {
                    continue;
                };
                let (salt, name, kind) = match pipe {
                    None => (format!("gutter {run}"), "Canalón", "GUTTER"),
                    Some(p) => (format!("downpipe {run} {p}"), "Bajante", "RIGIDSEGMENT"),
                };
                let guid = derived_guid(&roof.guid, &salt);
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let el = self.w.add(format!(
                    "IFCPIPESEGMENT({},$,{},$,$,{pl},{shape},$,.{kind}.)",
                    string(guid.as_str()),
                    string(name)
                ));
                if let Some(g) = &roof.covering.gutter {
                    self.material_of(materials, el, &guid, g.material);
                }
                self.roof_part_pset(el, &guid, roof, "Gutter");
                contained.entry(roof.level).or_default().push(el);
            }
        }
    }
}

fn material_name(model: &Model, m: MaterialId) -> String {
    model.material(m).map_or(String::new(), |m| m.name.clone())
}

/// A roof's dormers as text, one a line:
/// `kind|side|along|up|width|height|pitch|window width|window height|wall|frame|glass|guid`
/// (materials by name).
pub(crate) fn encode_dormers(model: &Model, roof: &Roof) -> String {
    roof.dormers
        .iter()
        .map(|d| {
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                d.kind.code(),
                d.side,
                real(d.along),
                real(d.up),
                real(d.width),
                real(d.height),
                real(d.pitch),
                real(d.window_width),
                real(d.window_height),
                material_name(model, d.wall_material),
                material_name(model, d.frame_material),
                material_name(model, d.glass_material),
                d.guid.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A roof's covering as text: `tiles|tile material|gutter shape|size|gutter
/// material|downpipe|spacing`, `-` for what it has not.
pub(crate) fn encode_covering(model: &Model, roof: &Roof) -> String {
    let c = &roof.covering;
    let tiles = c.tiles.map_or("-".to_string(), |t| t.code().into());
    let tm = c
        .tile_material
        .map_or("-".to_string(), |m| material_name(model, m));
    let gutter = c.gutter.as_ref().map_or("-|-|-|-|-".to_string(), |g| {
        format!(
            "{}|{}|{}|{}|{}",
            g.shape.code(),
            real(g.size),
            material_name(model, g.material),
            real(g.downpipe),
            real(g.spacing)
        )
    });
    format!("{tiles}|{tm}|{gutter}")
}

/// Reads [`encode_dormers`] and [`encode_covering`] back onto a roof of
/// `model` (materials found by name); what cannot be read is left out.
pub(crate) fn decode_parts(
    model: &Model,
    dormers: &str,
    covering: &str,
    length: f64,
) -> (Vec<archisim_core::Dormer>, archisim_core::RoofCovering) {
    let mat = |name: &str| model.materials().find(|m| m.name == name).map(|m| m.id);
    let num = |v: &str| v.parse::<f64>().ok();
    let mut out = Vec::new();
    for line in dormers.lines().filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split('|').collect();
        if f.len() < 13 {
            continue;
        }
        let read = || -> Option<archisim_core::Dormer> {
            Some(archisim_core::Dormer {
                guid: GlobalId::parse(f[12])?,
                kind: DormerKind::from_code(f[0])?,
                side: f[1].parse().ok()?,
                along: num(f[2])? * length,
                up: num(f[3])? * length,
                width: num(f[4])? * length,
                height: num(f[5])? * length,
                pitch: num(f[6])?,
                window_width: num(f[7])? * length,
                window_height: num(f[8])? * length,
                wall_material: mat(f[9])?,
                frame_material: mat(f[10])?,
                glass_material: mat(f[11])?,
            })
        };
        if let Some(d) = read() {
            out.push(d);
        }
    }
    let f: Vec<&str> = covering.split('|').collect();
    let mut c = archisim_core::RoofCovering::default();
    if f.len() >= 7 {
        c.tiles = archisim_core::TileKind::from_code(f[0]);
        c.tile_material = if f[1] == "-" { None } else { mat(f[1]) };
        if let (Some(shape), Some(size), Some(material), Some(downpipe), Some(spacing)) = (
            archisim_core::GutterShape::from_code(f[2]),
            num(f[3]),
            mat(f[4]),
            num(f[5]),
            num(f[6]),
        ) {
            c.gutter = Some(archisim_core::Gutter {
                shape,
                size: size * length,
                material,
                downpipe: downpipe * length,
                spacing: spacing * length,
            });
        }
    }
    (out, c)
}
