//! The accessories of wall and slab types (bloque 5 de la revisión del
//! 22/10/2026): each run of a skirting, band, cornice or coping, and each
//! slab edge, is an `IfcCovering` (`SKIRTINGBOARD`, `MOLDING`, `COPING`;
//! in IFC4, where there is no coping, `USERDEFINED`) in the storey of its
//! element, a triangulated body with its material's colour, and covers
//! its wall or slab (`IfcRelCoversBldgElements`). `ArchiSim_Accessory`
//! says it comes from the type, so a file read back does not take it for
//! an element of its own.

use std::collections::BTreeMap;

use archisim_core::{GlobalId, LevelId, MaterialId, SweepKind};
use archisim_geom::sweeps::SweepPiece;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter, Schema};

impl Exporter<'_> {
    pub(crate) fn write_accessories(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        // What the types say, on each wall and slab, to read them back.
        for w in model.walls() {
            let Some(t) = model.wall_type(w.wall_type) else {
                continue;
            };
            if t.sweeps.is_empty() {
                continue;
            }
            if let Some(&h) = self.hosts.get(&w.id.0) {
                let text = encode_sweeps(model, &t.sweeps);
                let p = self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Sweeps',$,IFCTEXT({}),$)",
                    string(&text)
                ));
                self.pset(h, &w.guid, "ArchiSim_Accessories", &[p]);
            }
        }
        for sl in model.slabs() {
            let Some(e) = model.slab_type(sl.slab_type).and_then(|t| t.edge.as_ref()) else {
                continue;
            };
            if let Some(&h) = self.hosts.get(&sl.id.0) {
                let text = encode_profile(model, e.material, &e.profile);
                let p = self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Edge',$,IFCTEXT({}),$)",
                    string(&text)
                ));
                self.pset(h, &sl.guid, "ArchiSim_Accessories", &[p]);
            }
        }
        let mut runs: Vec<(u64, GlobalId, LevelId, String, SweepPiece)> = Vec::new();
        for w in model.walls() {
            let type_name = model
                .wall_type(w.wall_type)
                .map_or(String::new(), |t| t.name.clone());
            for p in archisim_geom::sweeps::wall_sweep_pieces(model, w.id) {
                runs.push((w.id.0, w.guid.clone(), w.level, type_name.clone(), p));
            }
        }
        for s in model.slabs() {
            let type_name = model
                .slab_type(s.slab_type)
                .map_or(String::new(), |t| t.name.clone());
            if let Some(p) = archisim_geom::sweeps::slab_edge_piece(model, s.id) {
                runs.push((s.id.0, s.guid.clone(), s.level, type_name, p));
            }
        }
        // Several runs of one accessory on one element: numbered.
        let mut seen: BTreeMap<(u64, usize), usize> = BTreeMap::new();
        for (host, host_guid, level, type_name, piece) in runs {
            let Some(&(_, storey_pl)) = storeys.get(&level) else {
                continue;
            };
            let storey_z = model.level(level).map_or(0.0, |l| l.elevation);
            let n = seen.entry((host, piece.index)).or_insert(0);
            *n += 1;
            let guid = derived_guid(&host_guid, &format!("accessory {} {}", piece.index, n));
            let mut pts = Vec::new();
            let mut faces = Vec::new();
            for p in &piece.mesh.positions {
                pts.push(format!(
                    "({},{},{})",
                    real(p[0]),
                    real(p[1]),
                    real(p[2] - storey_z)
                ));
            }
            for t in &piece.mesh.triangles {
                faces.push(format!("({},{},{})", t[0] + 1, t[1] + 1, t[2] + 1));
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
            if let Some(&style) = self.styles.get(&piece.material) {
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
            let (name, predefined, object_type) = match piece.kind {
                Some(SweepKind::Skirting) => ("Rodapié", "SKIRTINGBOARD", None),
                Some(SweepKind::Moulding) => ("Moldura", "MOLDING", None),
                Some(SweepKind::Cornice) => ("Cornisa", "MOLDING", None),
                Some(SweepKind::Coping) => match self.schema {
                    Schema::Ifc4x3 => ("Albardilla", "COPING", None),
                    Schema::Ifc4 => ("Albardilla", "USERDEFINED", Some("Albardilla")),
                },
                None => ("Canto de forjado", "MOLDING", None),
            };
            let el = self.w.add(format!(
                "IFCCOVERING({},$,{},$,{},{pl},{shape},$,.{predefined}.)",
                string(guid.as_str()),
                string(&format!("{name} · {type_name}")),
                object_type.map_or("$".to_string(), string),
            ));
            if let Some(m) = materials.get(&piece.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&guid, "material").as_str())
                ));
            }
            if let Some(&h) = self.hosts.get(&host) {
                self.w.add(format!(
                    "IFCRELCOVERSBLDGELEMENTS({},$,$,$,{h},({el}))",
                    string(derived_guid(&guid, "covers").as_str())
                ));
            }
            let props = vec![self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('FromType',$,IFCLABEL({}),$)",
                string(&type_name)
            ))];
            self.pset(el, &guid, "ArchiSim_Accessory", &props);
            contained.entry(level).or_default().push(el);
        }
    }
}

/// A material and a section as text: `material|x y;x y;…`.
fn encode_profile(
    model: &archisim_core::Model,
    material: MaterialId,
    profile: &[archisim_core::Point2],
) -> String {
    let name = model.material(material).map_or("", |m| m.name.as_str());
    let pts: Vec<String> = profile
        .iter()
        .map(|p| format!("{} {}", real(p.x), real(p.y)))
        .collect();
    format!("{name}|{}", pts.join(";"))
}

/// A wall type's accessories as text, one a line:
/// `kind|side|offset|from_top|material|x y;x y;…`.
pub(crate) fn encode_sweeps(
    model: &archisim_core::Model,
    sweeps: &[archisim_core::WallSweep],
) -> String {
    sweeps
        .iter()
        .map(|s| {
            format!(
                "{}|{}|{}|{}|{}",
                s.kind.code(),
                s.side.code(),
                real(s.offset),
                u8::from(s.from_top),
                encode_profile(model, s.material, &s.profile)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A section from its text, its material found by name.
pub(crate) fn decode_profile(
    model: &archisim_core::Model,
    text: &str,
) -> Option<(MaterialId, Vec<archisim_core::Point2>)> {
    let (name, pts) = text.split_once('|')?;
    let material = model.materials().find(|m| m.name == name)?.id;
    let profile = pts
        .split(';')
        .map(|p| {
            let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
            Some(archisim_core::Point2::new(it.next()??, it.next()??))
        })
        .collect::<Option<Vec<_>>>()?;
    Some((material, profile))
}

/// A wall type's accessories from their text.
pub(crate) fn decode_sweeps(
    model: &archisim_core::Model,
    text: &str,
) -> Vec<archisim_core::WallSweep> {
    text.lines()
        .filter_map(|line| {
            let mut it = line.splitn(5, '|');
            let kind = SweepKind::from_code(it.next()?)?;
            let side = archisim_core::SweepSide::from_code(it.next()?)?;
            let offset = it.next()?.parse().ok()?;
            let from_top = it.next()? == "1";
            let (material, profile) = decode_profile(model, it.next()?)?;
            Some(archisim_core::WallSweep {
                kind,
                side,
                offset,
                from_top,
                profile,
                material,
            })
        })
        .collect()
}
