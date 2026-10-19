//! Loose railings: an `IfcRailing` (`GUARDRAIL`) in its storey, its bars
//! as one triangulated body, with `Pset_RailingCommon` and
//! `ArchiSim_Railing` to read it back as one.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId};

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_railings(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for r in model.railings() {
            let Some(&(_, storey_pl)) = storeys.get(&r.level) else {
                continue;
            };
            let storey_z = model.level(r.level).map_or(0.0, |l| l.elevation);
            let mut pts: Vec<String> = Vec::new();
            let mut faces: Vec<String> = Vec::new();
            for b in archisim_geom::solid::loose_railing_bars(model, r.id) {
                for t in b.triangles() {
                    let base = pts.len();
                    for p in t {
                        pts.push(format!(
                            "({},{},{})",
                            real(p[0]),
                            real(p[1]),
                            real(p[2] - storey_z)
                        ));
                    }
                    faces.push(format!("({},{},{})", base + 1, base + 2, base + 3));
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
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({body}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let el = self.w.add(format!(
                "IFCRAILING({},$,{},$,$,{pl},{shape},{},.GUARDRAIL.)",
                string(r.guid.as_str()),
                string(&format!("Barandilla {}", r.id.0)),
                string(&r.id.0.to_string()),
            ));
            if let Some(m) = materials.get(&r.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&r.guid, "material").as_str())
                ));
            }
            let mut common = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Height',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                    real(r.height)
                )),
                self.w
                    .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$)"),
            ];
            common.extend(self.status_prop(r.id.0));
            self.pset(el, &r.guid, "Pset_RailingCommon", &common);
            self.quantities(
                el,
                &r.guid,
                "Qto_RailingBaseQuantities",
                &[("Length", "IFCQUANTITYLENGTH", r.length())],
            );
            let path = r
                .path
                .iter()
                .map(|p| format!("{} {}", real(p.x), real(p.y)))
                .collect::<Vec<_>>()
                .join(";");
            let mut props = vec![self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Path',$,IFCTEXT({}),$)",
                string(&path)
            ))];
            for (key, v) in [
                ("BaseOffset", r.base_offset),
                ("Height", r.height),
                ("PostSpacing", r.post_spacing),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            self.pset(el, &r.guid, "ArchiSim_Railing", &props);
            contained.entry(r.level).or_default().push(el);
        }
    }
}
