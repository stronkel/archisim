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
                    holes: Vec::new(),
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
                let area = signed_area(&plane.outline).abs();
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
            // Its dormers: each an `IfcRoof` of its own, its body, roof and
            // window as triangles.
            let storey_z = model.level(roof.level).map_or(0.0, |l| l.elevation);
            let covering = rtype
                .layers
                .first()
                .map_or(0.05, |l| l.thickness.clamp(0.02, 0.12));
            let glass = model
                .materials()
                .find(|m| m.category == "glass")
                .map(|m| m.id);
            for (i, d) in roof.dormers.iter().enumerate() {
                let Ok(shape) = roof.dormer_shape(d) else {
                    continue;
                };
                let level_z = model.level(roof.level).map_or(0.0, |l| l.elevation);
                let m = archisim_geom::dormers::dormer_meshes(d, &shape, level_z, covering);
                let roof_material = rtype.layers.first().map_or(d.wall_material, |l| l.material);
                let mut items = Vec::new();
                for (tris, material) in [
                    (&m.body, Some(d.wall_material)),
                    (&m.roof, Some(roof_material)),
                    (&m.glass, glass),
                ] {
                    if tris.is_empty() {
                        continue;
                    }
                    let pts: Vec<String> = tris
                        .iter()
                        .flatten()
                        .map(|p| {
                            format!("({},{},{})", real(p[0]), real(p[1]), real(p[2] - storey_z))
                        })
                        .collect();
                    let faces: Vec<String> = (0..tris.len())
                        .map(|k| format!("({},{},{})", 3 * k + 1, 3 * k + 2, 3 * k + 3))
                        .collect();
                    let list3 = self
                        .w
                        .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
                    let body = self.w.add(format!(
                        "IFCTRIANGULATEDFACESET({list3},$,.T.,({}),$)",
                        faces.join(",")
                    ));
                    if let Some(style) = material.and_then(|mat| self.styles.get(&mat).copied()) {
                        self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
                    }
                    items.push(body.to_string());
                }
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({}))",
                    self.body_ctx,
                    items.join(",")
                ));
                let pshape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let dguid = derived_guid(&roof.guid, &format!("dormer {i}"));
                let dpl = self.placement(Some(storey_pl), self.origin, None);
                let dr = self.w.add(format!(
                    "IFCROOF({},$,{},$,{},{dpl},{pshape},$,.{}.)",
                    string(dguid.as_str()),
                    string(&format!("{name} · buhardilla {}", i + 1)),
                    string("Buhardilla"),
                    match d.kind {
                        archisim_core::DormerKind::Shed => "SHED_ROOF",
                        archisim_core::DormerKind::Gable => "GABLE_ROOF",
                    }
                ));
                self.common_pset(
                    dr,
                    roof.id.0,
                    &dguid,
                    "Pset_RoofCommon",
                    "Buhardilla",
                    true,
                    false,
                );
                // Its roof's area (sloping) and its plan.
                let w = d.width + 0.3;
                let (surface, projected) = match d.kind {
                    archisim_core::DormerKind::Shed => {
                        let run = shape.depth + 0.15;
                        (w * run * (1.0 + d.pitch * d.pitch).sqrt(), w * run)
                    }
                    archisim_core::DormerKind::Gable => {
                        let run = (shape.depth + shape.eave_depth) / 2.0 + 0.15;
                        (w * run * (1.0 + d.pitch * d.pitch).sqrt(), w * run)
                    }
                };
                self.quantities(
                    dr,
                    &dguid,
                    "Qto_RoofBaseQuantities",
                    &[
                        ("GrossArea", "IFCQUANTITYAREA", surface),
                        ("NetArea", "IFCQUANTITYAREA", surface),
                        ("ProjectedArea", "IFCQUANTITYAREA", projected),
                    ],
                );
                parts.push(dr);
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
            let mut props = props.to_vec();
            if !roof.dormers.is_empty() {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Dormers',$,IFCTEXT({}),$)",
                    string(&encode_dormers(model, &roof.dormers))
                )));
            }
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
            contained.entry(roof.level).or_default().push(r);
        }
    }
}

/// A roof's dormers as text, `;` between them:
/// `kind|x y|width|height|pitch|wall material|window`.
pub(crate) fn encode_dormers(
    model: &archisim_core::Model,
    dormers: &[archisim_core::Dormer],
) -> String {
    dormers
        .iter()
        .map(|d| {
            format!(
                "{}|{} {}|{}|{}|{}|{}|{}",
                d.kind.code(),
                real(d.position.x),
                real(d.position.y),
                real(d.width),
                real(d.height),
                real(d.pitch),
                model
                    .material(d.wall_material)
                    .map_or("", |m| m.name.as_str())
                    .replace(['|', ';'], "/"),
                u8::from(d.window)
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Reads [`encode_dormers`]; lengths multiplied by `len`, materials found
/// by name (the first one when missing).
pub(crate) fn decode_dormers(
    model: &archisim_core::Model,
    text: &str,
    len: f64,
) -> Vec<archisim_core::Dormer> {
    let first = model.materials().next().map(|m| m.id);
    text.split(';')
        .filter_map(|line| {
            let f: Vec<&str> = line.split('|').collect();
            if f.len() < 7 {
                return None;
            }
            let num = |s: &str| s.trim().parse::<f64>().ok();
            let mut xy = f[1].split_whitespace().map(num);
            let (x, y) = (xy.next()??, xy.next()??);
            Some(archisim_core::Dormer {
                position: archisim_core::Point2::new(x * len, y * len),
                width: num(f[2])? * len,
                height: num(f[3])? * len,
                kind: archisim_core::DormerKind::from_code(f[0].trim())?,
                pitch: num(f[4])?,
                wall_material: model
                    .materials()
                    .find(|m| m.name == f[5])
                    .map(|m| m.id)
                    .or(first)?,
                window: f[6].trim() == "1",
            })
        })
        .collect()
}
