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
            // The frame and, apart, the glass, each a face set with its
            // material's colour.
            let (frame, glass) = archisim_geom::solid::loose_railing_parts(model, r.id);
            let mut items = Vec::new();
            for (bars, material) in [(frame, Some(r.material)), (glass, r.glass)] {
                let Some(material) = material else { continue };
                let mut pts: Vec<String> = Vec::new();
                let mut faces: Vec<String> = Vec::new();
                for b in bars {
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
                if let Some(&style) = self.styles.get(&material) {
                    self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
                }
                items.push(body);
            }
            if items.is_empty() {
                continue;
            }
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','Tessellation',{})",
                self.body_ctx,
                crate::step::list(&items)
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
            if let Some(name) = r.glass.and_then(|g| model.material(g)) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Glass',$,IFCLABEL({}),$)",
                    string(&name.name)
                )));
            }
            self.pset(el, &r.guid, "ArchiSim_Railing", &props);
            contained.entry(r.level).or_default().push(el);
        }
    }
}
