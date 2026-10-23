//! Pitched roofs: an `IfcRoof` in its storey, made of one `IfcSlab`
//! (`ROOF`) per plane (`IfcRelAggregates`), as Revit and ArchiCAD write
//! them. Each plane goes out as an inclined slab over the part of the roof
//! it covers, with the roof type's layer set and its quantities;
//! `Qto_RoofBaseQuantities` gives the whole roof's areas.

use std::collections::BTreeMap;

use archisim_core::{LevelId, Slab, SlabId, SlabTypeId};
use archisim_geom::signed_area;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_roofs(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        slab_sets: &BTreeMap<SlabTypeId, Ref>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
        slabs_by_type: &mut BTreeMap<SlabTypeId, Vec<Ref>>,
    ) {
        let model = self.model;
        for roof in model.roofs() {
            let (Some(rtype), Some(&(_, storey_pl))) =
                (model.slab_type(roof.roof_type), storeys.get(&roof.level))
            else {
                continue;
            };
            let thickness = rtype.thickness();
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let name = format!("{} {}", rtype.name, roof.id.0);
            let r = self.w.add(format!(
                "IFCROOF({},$,{},$,{},{pl},$,{},.{}.)",
                string(roof.guid.as_str()),
                string(&name),
                string(&rtype.name),
                string(&roof.id.0.to_string()),
                match roof.slopes.iter().filter(|s| s.is_some()).count() {
                    1 => "SHED_ROOF",
                    2 if roof.slopes.len() == 4 => "GABLE_ROOF",
                    _ => "HIP_ROOF",
                }
            ));
            let mut parts = Vec::new();
            let mut plan = 0.0;
            for plane in roof.planes() {
                // The plane as an inclined slab: its top face is at the
                // roof's base along the side it rises from.
                let part = Slab {
                    id: SlabId(roof.id.0),
                    guid: derived_guid(&roof.guid, &format!("plane {}", plane.side)),
                    slab_type: roof.roof_type,
                    level: roof.level,
                    outline: plane.outline.clone(),
                    top_offset: roof.base_offset,
                    // Its dormers' footprints (on this piece of it).
                    holes: roof
                        .plane_holes(plane.side)
                        .into_iter()
                        .filter(|h| {
                            h.iter()
                                .all(|q| archisim_core::point_in_polygon(*q, &plane.outline))
                        })
                        .collect(),
                    slope: Some(plane.slope),
                };
                let (spl, shape) = self.slab_shape(&part, thickness, pl, &[]);
                let sl = self.w.add(format!(
                    "IFCSLAB({},$,{},$,{},{spl},{shape},$,.ROOF.)",
                    string(part.guid.as_str()),
                    string(&format!("{name} · faldón {}", plane.side + 1)),
                    string(&rtype.name),
                ));
                let usage = self.w.add(format!(
                    "IFCMATERIALLAYERSETUSAGE({},.AXIS3.,.POSITIVE.,0.,$)",
                    slab_sets[&roof.roof_type]
                ));
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({sl}),{usage})",
                    string(derived_guid(&part.guid, "material").as_str())
                ));
                self.common_pset(
                    sl,
                    roof.id.0,
                    &part.guid,
                    "Pset_SlabCommon",
                    &rtype.name,
                    true,
                    rtype.is_load_bearing(),
                );
                let area = signed_area(&plane.outline).abs()
                    - part.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>();
                plan += area;
                let surface = area * plane.slope.vertical(1.0);
                self.quantities(
                    sl,
                    &part.guid,
                    "Qto_SlabBaseQuantities",
                    &[
                        ("Width", "IFCQUANTITYLENGTH", thickness),
                        ("GrossArea", "IFCQUANTITYAREA", surface),
                        ("NetArea", "IFCQUANTITYAREA", surface),
                        ("GrossVolume", "IFCQUANTITYVOLUME", surface * thickness),
                        ("NetVolume", "IFCQUANTITYVOLUME", surface * thickness),
                    ],
                );
                // Typed by the roof's type when it is a roof type (a
                // floor type would contradict the part's ROOF).
                if rtype.function == archisim_core::SlabFunction::Roof {
                    slabs_by_type.entry(roof.roof_type).or_default().push(sl);
                }
                parts.push(sl);
            }
            self.rel_aggregates(derived_guid(&roof.guid, "planes"), r, &parts);
            self.common_pset(
                r,
                roof.id.0,
                &roof.guid,
                "Pset_RoofCommon",
                &rtype.name,
                true,
                false,
            );
            // What ArchiSim needs to read the roof back as one (other
            // programs read its planes).
            let footprint = roof
                .outline
                .iter()
                .map(|p| format!("{} {}", real(p.x), real(p.y)))
                .collect::<Vec<_>>()
                .join(";");
            let slopes = roof
                .slopes
                .iter()
                .map(|s| s.map_or("-".to_string(), real))
                .collect::<Vec<_>>()
                .join(";");
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Footprint',$,IFCTEXT({}),$)",
                    string(&footprint)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Slopes',$,IFCTEXT({}),$)",
                    string(&slopes)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('BaseOffset',$,IFCLENGTHMEASURE({}),$)",
                    real(roof.base_offset)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Overhang',$,IFCLENGTHMEASURE({}),$)",
                    real(roof.overhang)
                )),
            ];
            self.pset(r, &roof.guid, "ArchiSim_Roof", &props);
            let surface = roof.surface_area();
            self.quantities(
                r,
                &roof.guid,
                "Qto_RoofBaseQuantities",
                &[
                    ("GrossArea", "IFCQUANTITYAREA", surface),
                    ("NetArea", "IFCQUANTITYAREA", surface),
                    ("ProjectedArea", "IFCQUANTITYAREA", plan),
                ],
            );
            self.hosts.insert(roof.id.0, r);
            contained.entry(roof.level).or_default().push(r);
        }
    }
}
