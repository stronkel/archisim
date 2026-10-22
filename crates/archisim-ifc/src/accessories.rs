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
