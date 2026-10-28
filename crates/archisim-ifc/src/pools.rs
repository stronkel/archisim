//! Pools built in the ground: an `IfcElementAssembly` «Piscina» in the
//! storey of its level, made of its floor (`IfcSlab` FLOOR), its walls
//! (`IfcWall`), the lining on their inside faces (`IfcCovering` CLADDING)
//! and its coping (`IfcSlab` USERDEFINED «Coronación»), each a
//! triangulated body; the water is an `IfcSpace` «Lámina de agua» in the
//! storey with its area and volume. `ArchiSim_Pool` on the assembly reads
//! it back; `ArchiSim_PoolPart` on each part (the water too) names its pool.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId};
use archisim_geom::pools::{pool_parts, PoolPartKind};
use archisim_geom::solid::P3;

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    /// A triangulated body as a product's shape, its heights from `z0`.
    fn mesh_shape(&mut self, triangles: &[[P3; 3]], z0: f64) -> Option<Ref> {
        if triangles.is_empty() {
            return None;
        }
        let mut pts: Vec<String> = Vec::new();
        let mut faces: Vec<String> = Vec::new();
        for tri in triangles {
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
        Some(
            self.w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))")),
        )
    }

    /// `ArchiSim_PoolPart` on a part of a pool: its pool and which part.
    fn part_of(&mut self, el: Ref, guid: &archisim_core::GlobalId, pool: &str, part: &str) {
        let props = [
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Pool',$,IFCIDENTIFIER({}),$)",
                string(pool)
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Part',$,IFCLABEL({}),$)",
                string(part)
            )),
        ];
        self.pset(el, guid, "ArchiSim_PoolPart", &props);
    }

    pub(crate) fn write_pools(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        let mut waters: BTreeMap<LevelId, Vec<Ref>> = BTreeMap::new();
        for p in model.pools() {
            let Some(&(_, storey_pl)) = storeys.get(&p.level) else {
                continue;
            };
            let z0 = model.level(p.level).map_or(0.0, |l| l.elevation);
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let assembly = self.w.add(format!(
                "IFCELEMENTASSEMBLY({},$,{},$,'Piscina',{pl},$,{},.SITE.,.USERDEFINED.)",
                string(p.guid.as_str()),
                string(&p.name),
                string(&p.id.0.to_string()),
            ));
            let mut pieces = Vec::new();
            for part in pool_parts(model, p.id) {
                let Some(shape) = self.mesh_shape(&part.triangles, z0) else {
                    continue;
                };
                let part_pl = self.placement(Some(pl), self.origin, None);
                let (key, entity) = match part.kind {
                    PoolPartKind::Floor => (
                        "floor",
                        format!(
                            "IFCSLAB({},$,'Solera',$,$,{part_pl},{shape},$,.FLOOR.)",
                            string(derived_guid(&p.guid, "floor").as_str())
                        ),
                    ),
                    PoolPartKind::Walls => (
                        "walls",
                        format!(
                            "IFCWALL({},$,'Paredes del vaso',$,$,{part_pl},{shape},$,.SOLIDWALL.)",
                            string(derived_guid(&p.guid, "walls").as_str())
                        ),
                    ),
                    PoolPartKind::Lining => (
                        "lining",
                        format!(
                            "IFCCOVERING({},$,'Revestimiento',$,$,{part_pl},{shape},$,.CLADDING.)",
                            string(derived_guid(&p.guid, "lining").as_str())
                        ),
                    ),
                    PoolPartKind::Coping => (
                        "coping",
                        format!(
                            "IFCSLAB({},$,'Coronación',$,'Coronación',{part_pl},{shape},$,.USERDEFINED.)",
                            string(derived_guid(&p.guid, "coping").as_str())
                        ),
                    ),
                    PoolPartKind::Water => {
                        let space = self.w.add(format!(
                            "IFCSPACE({},$,'Agua',$,'Lámina de agua',{part_pl},{shape},{},.ELEMENT.,.USERDEFINED.,$)",
                            string(derived_guid(&p.guid, "water").as_str()),
                            string(&p.name),
                        ));
                        let guid = derived_guid(&p.guid, "water");
                        let common = [self
                            .w
                            .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$)")];
                        self.pset(space, &guid, "Pset_SpaceCommon", &common);
                        self.part_of(space, &guid, p.guid.as_str(), "water");
                        let (area, volume) = (p.area(), p.water_volume());
                        self.quantities(
                            space,
                            &guid,
                            "Qto_SpaceBaseQuantities",
                            &[
                                ("NetFloorArea", "IFCQUANTITYAREA", area),
                                ("GrossFloorArea", "IFCQUANTITYAREA", area),
                                ("NetPerimeter", "IFCQUANTITYLENGTH", p.perimeter()),
                                ("NetVolume", "IFCQUANTITYVOLUME", volume),
                                ("GrossVolume", "IFCQUANTITYVOLUME", volume),
                            ],
                        );
                        waters.entry(p.level).or_default().push(space);
                        continue;
                    }
                };
                let el = self.w.add(entity);
                self.part_of(el, &derived_guid(&p.guid, key), p.guid.as_str(), key);
                if let Some(m) = materials.get(&part.material) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                        string(derived_guid(&p.guid, &format!("{key} material")).as_str())
                    ));
                }
                pieces.push(el);
            }
            if !pieces.is_empty() {
                self.rel_aggregates(derived_guid(&p.guid, "parts"), assembly, &pieces);
            }
            let outline = p
                .outline
                .iter()
                .map(|q| format!("{} {}", real(q.x), real(q.y)))
                .collect::<Vec<_>>()
                .join(";");
            let material = |m: MaterialId| {
                model
                    .material(m)
                    .map(|x| x.name.clone())
                    .unwrap_or_default()
            };
            let mut props = vec![self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Outline',$,IFCTEXT({}),$)",
                string(&outline)
            ))];
            for (name, v) in [
                ("TopOffset", p.top_offset),
                ("Shallow", p.shallow),
                ("Deep", p.deep),
                ("Freeboard", p.freeboard),
                ("Wall", p.wall),
                ("Coping", p.coping),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{name}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Slope',$,IFCPLANEANGLEMEASURE({}),$)",
                real(p.slope)
            )));
            for (name, m) in [
                ("Lining", p.lining),
                ("Shell", p.shell),
                ("CopingMaterial", p.coping_material),
                ("Water", p.water),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{name}',$,IFCLABEL({}),$)",
                    string(&material(m))
                )));
            }
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('WaterVolume',$,IFCVOLUMEMEASURE({}),$)",
                real(p.water_volume())
            )));
            self.pset(assembly, &p.guid, "ArchiSim_Pool", &props);
            contained.entry(p.level).or_default().push(assembly);
        }
        for (level, spaces) in &waters {
            let (storey, _) = storeys[level];
            let guid = model
                .level(*level)
                .map(|l| l.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELAGGREGATES({},$,$,$,{storey},{})",
                string(derived_guid(&guid, "pools").as_str()),
                list(spaces)
            ));
        }
    }
}
