//! Terrains: an `IfcGeographicElement` (`TERRAIN`) in the storey of its
//! level, its ground as a triangulated body, with `ArchiSim_Terrain` (the
//! survey points) to read it back.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId};

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_terrains(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for t in model.terrains() {
            let Some(&(_, storey_pl)) = storeys.get(&t.level) else {
                continue;
            };
            let z0 = model.level(t.level).map_or(0.0, |l| l.elevation);
            let solids = archisim_geom::solid::terrain_solids(model, t.id);
            let mut pts: Vec<String> = Vec::new();
            let mut faces: Vec<String> = Vec::new();
            for tri in solids.iter().flat_map(|s| s.triangles.iter()) {
                let base = pts.len();
                for p in tri {
                    pts.push(format!(
                        "({},{},{})",
                        real(p[0]),
                        real(p[1]),
                        real(p[2] - z0)
                    ));
                }
                faces.push(format!("({},{},{})", base + 1, base + 2, base + 3));
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
                "IFCGEOGRAPHICELEMENT({},$,{},$,$,{pl},{shape},{},.TERRAIN.)",
                string(t.guid.as_str()),
                string(&format!("Terreno {}", t.id.0)),
                string(&t.id.0.to_string()),
            ));
            if let Some(m) = materials.get(&t.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&t.guid, "material").as_str())
                ));
            }
            let points = t
                .points
                .iter()
                .map(|p| format!("{} {} {}", real(p.at.x), real(p.at.y), real(p.z)))
                .collect::<Vec<_>>()
                .join(";");
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Points',$,IFCTEXT({}),$)",
                    string(&points)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Depth',$,IFCLENGTHMEASURE({}),$)",
                    real(t.depth)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Contour',$,IFCLENGTHMEASURE({}),$)",
                    real(t.contour)
                )),
            ];
            // Contour lines drawn by hand: point indices, line by line.
            if !t.lines.is_empty() {
                let lines = t
                    .lines
                    .iter()
                    .map(|l| {
                        l.iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Lines',$,IFCTEXT({}),$)",
                    string(&lines)
                )));
            }
            self.pset(el, &t.guid, "ArchiSim_Terrain", &props);
            contained.entry(t.level).or_default().push(el);
        }
    }
}
