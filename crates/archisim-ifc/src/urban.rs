//! Plots and roads, on the site (not in the building).
//!
//! Each plot is an `IfcSite` inside the project's site, with its boundary
//! as a `FootPrint`, its cadastral reference as `LandTitleNumber`,
//! `Qto_SiteBaseQuantities` and `ArchiSim_Plot` (setback and level) to read
//! it back.
//!
//! A road, in IFC 4.3, is an `IfcRoad` in the site holding an
//! `IfcPavement` for its carriageway, parking and sidewalks and an
//! `IfcKerb` for each kerb; in IFC4, which has none of them, those pieces
//! are proxies on the site. The carriageway carries `ArchiSim_Road` (axis
//! and section) to read the road back.

use std::collections::BTreeMap;

use archisim_core::{GlobalId, MaterialId, Point2, RoadBand};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter, Schema};

impl Exporter<'_> {
    pub(crate) fn write_urban(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        site: Ref,
        site_pl: Ref,
    ) {
        let model = self.model;
        let elevation = |l| model.level(l).map_or(0.0, |l| l.elevation);
        let level_guid = |l| {
            model
                .level(l)
                .map(|l| l.guid.to_string())
                .unwrap_or_default()
        };

        // Plots.
        let mut plots = Vec::new();
        for p in model.plots() {
            let loc = self.point3(0.0, 0.0, elevation(p.level));
            let pl = self.placement(Some(site_pl), loc, None);
            let curve = self.polyline(&p.outline, true);
            let set = self.w.add(format!("IFCGEOMETRICCURVESET(({curve}))"));
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'FootPrint','GeometricCurveSet',({set}))",
                self.axis_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let land = if p.reference.is_empty() {
                "$".to_string()
            } else {
                string(&p.reference)
            };
            let el = self.w.add(format!(
                "IFCSITE({},$,{},$,'Parcela',{pl},{shape},$,.PARTIAL.,$,$,$,{land},$)",
                string(p.guid.as_str()),
                string(&p.name)
            ));
            self.quantities(
                el,
                &p.guid,
                "Qto_SiteBaseQuantities",
                &[
                    ("GrossArea", "IFCQUANTITYAREA", p.area()),
                    ("GrossPerimeter", "IFCQUANTITYLENGTH", p.perimeter()),
                ],
            );
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Setback',$,IFCLENGTHMEASURE({}),$)",
                    real(p.setback)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                    string(&level_guid(p.level))
                )),
            ];
            self.pset(el, &p.guid, "ArchiSim_Plot", &props);
            plots.push(el);
        }
        if !plots.is_empty() {
            self.rel_aggregates(derived_guid(&model.site_guid, "plots"), site, &plots);
        }

        // Roads.
        let mut roads = Vec::new();
        let mut on_site = Vec::new();
        for r in model.roads() {
            let top = elevation(r.level) + r.top_offset;
            let loc = self.point3(0.0, 0.0, top);
            let road_pl = self.placement(Some(site_pl), loc, None);
            let road = match self.schema {
                Schema::Ifc4x3 => Some(self.w.add(format!(
                    "IFCROAD({},$,{},$,$,{road_pl},$,$,.ELEMENT.,.NOTDEFINED.)",
                    string(r.guid.as_str()),
                    string(&r.name)
                ))),
                Schema::Ifc4 => None,
            };
            let s = &r.section;
            let kh = if s.single_level { 0.0 } else { s.kerb_height };
            let mut pieces = Vec::new();
            for (i, p) in r.pieces().into_iter().enumerate() {
                let (band, outline, material) = (p.band, p.outline, p.material);
                let (z0, z1) = match (band, p.raised) {
                    (RoadBand::Kerb, true) => (-s.depth, kh),
                    (_, true) => (kh - 0.2, kh),
                    _ => (-s.depth, 0.0),
                };
                let curve = self.polyline(&outline, true);
                let profile = self
                    .w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"));
                let o = self.point3(0.0, 0.0, z0);
                let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({o},$,$)"));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                    self.z,
                    real(z1 - z0)
                ));
                let body = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
                let pl = self.placement(Some(road_pl), self.origin, None);
                let guid: GlobalId = derived_guid(&r.guid, &format!("band {i}"));
                let name = string(&format!("{} · {}", r.name, band.name()));
                let el =
                    match (self.schema, band) {
                        (Schema::Ifc4x3, RoadBand::Kerb) => self.w.add(format!(
                            "IFCKERB({},$,{name},$,$,{pl},{shape},$,.NOTDEFINED.)",
                            string(guid.as_str())
                        )),
                        (Schema::Ifc4x3, _) => self.w.add(format!(
                            "IFCPAVEMENT({},$,{name},$,$,{pl},{shape},$,.{}.)",
                            string(guid.as_str()),
                            if band.raised() { "RIGID" } else { "FLEXIBLE" }
                        )),
                        (Schema::Ifc4, _) => self.w.add(format!(
                        "IFCBUILDINGELEMENTPROXY({},$,{name},$,{},{pl},{shape},$,.USERDEFINED.)",
                        string(guid.as_str()),
                        string(if band == RoadBand::Kerb { "KERB" } else { "PAVEMENT" })
                    )),
                    };
                if let Some(m) = materials.get(&material) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
                if i == 0 {
                    // To read the road back: its axis and section.
                    let axis: Vec<String> = r
                        .axis
                        .iter()
                        .map(|p: &Point2| format!("{},{}", real(p.x), real(p.y)))
                        .collect();
                    let mut props = vec![
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Road',$,IFCIDENTIFIER({}),$)",
                            string(r.guid.as_str())
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('RoadName',$,IFCLABEL({}),$)",
                            string(&r.name)
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Axis',$,IFCTEXT({}),$)",
                            string(&axis.join(";"))
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                            string(&level_guid(r.level))
                        )),
                    ];
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Section',$,IFCTEXT({}),$)",
                        string(&s.encode())
                    )));
                    for (key, v) in [
                        ("KerbHeight", s.kerb_height),
                        ("Depth", s.depth),
                        ("TopOffset", r.top_offset),
                    ] {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                            real(v)
                        )));
                    }
                    let mats = [
                        ("Pavement", Some(r.pavement)),
                        ("KerbMaterial", Some(r.kerb_material)),
                        ("SidewalkMaterial", Some(r.sidewalk_material)),
                        ("PlantingMaterial", s.planting),
                        ("BikeMaterial", s.bike),
                        ("MarkingMaterial", s.markings),
                    ];
                    for (key, m) in mats.into_iter().filter_map(|(k, m)| Some((k, m?))) {
                        if let Some(x) = model.material(m) {
                            props.push(self.w.add(format!(
                                "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLABEL({}),$)",
                                string(&x.name)
                            )));
                        }
                    }
                    self.pset(el, &guid, "ArchiSim_Road", &props);
                }
                pieces.push(el);
            }
            match road {
                Some(road) => {
                    self.w.add(format!(
                        "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},{road})",
                        string(derived_guid(&r.guid, "contains").as_str()),
                        list(&pieces)
                    ));
                    roads.push(road);
                }
                None => on_site.extend(pieces),
            }
        }
        if !roads.is_empty() {
            self.rel_aggregates(derived_guid(&model.site_guid, "roads"), site, &roads);
        }
        if !on_site.is_empty() {
            self.w.add(format!(
                "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},{site})",
                string(derived_guid(&model.site_guid, "contains").as_str()),
                list(&on_site)
            ));
        }
    }
}
