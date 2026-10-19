//! Stairs: an `IfcStair` in its storey, made of one `IfcStairFlight` per
//! flight and one `IfcSlab` (`LANDING`) per landing (`IfcRelAggregates`).
//! Each flight is its side profile (the steps on their slab) swept across
//! its width; each landing a slab of its outline. `Pset_StairCommon` and
//! `Pset_StairFlightCommon` give risers, treads and their sizes;
//! `ArchiSim_Stair` lets ArchiSim read the stair back as one.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId, Point2, StairUse, WallTop};
use archisim_geom::signed_area;
use archisim_geom::solid::flight_profile;

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_stairs(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for stair in model.stairs() {
            let Some(&(_, storey_pl)) = storeys.get(&stair.level) else {
                continue;
            };
            let l = stair.layout();
            let storey_z = model.level(stair.level).map_or(0.0, |l| l.elevation);
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let name = format!("Escalera {}", stair.id.0);
            let shape = match l.flights.len() {
                1 => "STRAIGHT_RUN_STAIR",
                2 => {
                    let (a, b) = (l.flights[0].direction(), l.flights[1].direction());
                    if a.dot(b) < -0.9 {
                        "HALF_TURN_STAIR"
                    } else if a.dot(b).abs() < 0.1 {
                        "QUARTER_TURN_STAIR"
                    } else {
                        "TWO_STRAIGHT_RUN_STAIR"
                    }
                }
                _ => "USERDEFINED",
            };
            let st = self.w.add(format!(
                "IFCSTAIR({},$,{},$,$,{pl},$,{},.{shape}.)",
                string(stair.guid.as_str()),
                string(&name),
                string(&stair.id.0.to_string()),
            ));
            let mut parts = Vec::new();
            for (i, f) in l.flights.iter().enumerate() {
                let guid = derived_guid(&stair.guid, &format!("flight {i}"));
                // Profile: x along the flight, y up from the storey.
                let profile = flight_profile(f, l.riser, stair.waist, 0.0);
                let poly = self.polyline(&profile, true);
                let def = self
                    .w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{poly})"));
                // Local axes: X along the flight, Z across it to its right
                // (so Y is up); swept from its left side.
                let u = f.direction();
                let n = u.perp_left();
                let corner = f.start + n * (stair.width / 2.0);
                let at = self.point3(corner.x, corner.y, 0.0);
                let axis = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(-n.x), real(-n.y)));
                let xdir = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(u.x), real(u.y)));
                let spl = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({at},{axis},{xdir})"));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({def},{spl},{},{})",
                    self.z,
                    real(stair.width)
                ));
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let fpl = self.placement(Some(pl), self.origin, None);
                let fl = self.w.add(format!(
                    "IFCSTAIRFLIGHT({},$,{},$,$,{fpl},{shape},$,{},{},{},{},.STRAIGHT.)",
                    string(guid.as_str()),
                    string(&format!("{name} · tramo {}", i + 1)),
                    f.risers,
                    f.treads,
                    real(l.riser),
                    real(f.going)
                ));
                let props = self.stair_props(f.risers, f.treads, l.riser, f.going);
                self.pset(fl, &guid, "Pset_StairFlightCommon", &props);
                let volume = signed_area(&profile).abs() * stair.width;
                self.quantities(
                    fl,
                    &guid,
                    "Qto_StairFlightBaseQuantities",
                    &[
                        ("Length", "IFCQUANTITYLENGTH", f.length()),
                        ("GrossVolume", "IFCQUANTITYVOLUME", volume),
                        ("NetVolume", "IFCQUANTITYVOLUME", volume),
                    ],
                );
                parts.push(fl);
            }
            let thickness = stair.waist + l.riser;
            for (i, landing) in l.landings.iter().enumerate() {
                let guid = derived_guid(&stair.guid, &format!("landing {i}"));
                let poly = self.polyline(&landing.outline, true);
                let def = self
                    .w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{poly})"));
                let at = self.point3(0.0, 0.0, landing.top - thickness);
                let lpl = self.placement(Some(pl), at, None);
                let spl = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({def},{spl},{},{})",
                    self.z,
                    real(thickness)
                ));
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let sl = self.w.add(format!(
                    "IFCSLAB({},$,{},$,$,{lpl},{shape},$,.LANDING.)",
                    string(guid.as_str()),
                    string(&format!("{name} · meseta {}", i + 1)),
                ));
                let area = signed_area(&landing.outline).abs();
                self.quantities(
                    sl,
                    &guid,
                    "Qto_SlabBaseQuantities",
                    &[
                        ("Width", "IFCQUANTITYLENGTH", thickness),
                        ("NetArea", "IFCQUANTITYAREA", area),
                        ("NetVolume", "IFCQUANTITYVOLUME", area * thickness),
                    ],
                );
                parts.push(sl);
            }
            // Railings: one IfcRailing, its bars as a triangulated body
            // (with its own material, so not in the flights' list).
            let mut railing = None;
            let bars = archisim_geom::solid::railing_bars(model, stair.id);
            if !bars.is_empty() {
                let guid = derived_guid(&stair.guid, "railings");
                let mut pts: Vec<String> = Vec::new();
                let mut faces: Vec<String> = Vec::new();
                for b in &bars {
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
                let rpl = self.placement(Some(pl), self.origin, None);
                let rail = self.w.add(format!(
                    "IFCRAILING({},$,{},$,$,{rpl},{shape},$,.HANDRAIL.)",
                    string(guid.as_str()),
                    string(&format!("{name} · barandilla")),
                ));
                let mat = archisim_geom::solid::railing_material(model, stair);
                if let Some(m) = materials.get(&mat) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({rail}),{m})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
                let height = self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Height',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                    real(archisim_core::RAILING_HEIGHT)
                ));
                self.pset(rail, &guid, "Pset_RailingCommon", &[height]);
                railing = Some(rail);
            }
            if let Some(m) = materials.get(&stair.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,{},{m})",
                    string(derived_guid(&stair.guid, "material").as_str()),
                    list(&parts)
                ));
            }
            parts.extend(railing);
            self.rel_aggregates(derived_guid(&stair.guid, "parts"), st, &parts);
            let mut props = self.stair_props(l.risers(), l.treads(), l.riser, l.flights[0].going);
            props.extend(self.status_prop(stair.id.0));
            self.pset(st, &stair.guid, "Pset_StairCommon", &props);
            // What ArchiSim needs to read it back as one stair.
            let path = stair
                .path
                .iter()
                .map(|p: &Point2| format!("{} {}", real(p.x), real(p.y)))
                .collect::<Vec<_>>()
                .join(";");
            let top = match stair.top {
                WallTop::Level { level, offset } => model
                    .level(level)
                    .map(|l| format!("{} {}", l.guid, real(offset)))
                    .unwrap_or_default(),
                WallTop::Unconnected => String::new(),
            };
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Path',$,IFCTEXT({}),$)",
                    string(&path)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Usage',$,IFCLABEL({}),$)",
                    string(match stair.usage {
                        StairUse::General => "general",
                        StairUse::Restricted => "restricted",
                    })
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Top',$,IFCTEXT({}),$)",
                    string(&top)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Railings',$,IFCLABEL({}),$)",
                    string(match stair.railings {
                        archisim_core::Railings::None => "none",
                        archisim_core::Railings::Left => "left",
                        archisim_core::Railings::Right => "right",
                        archisim_core::Railings::Both => "both",
                    })
                )),
            ];
            for (key, v) in [
                ("Width", stair.width),
                ("Going", stair.going),
                ("BaseOffset", stair.base_offset),
                ("Height", stair.height),
                ("Waist", stair.waist),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            self.pset(st, &stair.guid, "ArchiSim_Stair", &props);
            contained.entry(stair.level).or_default().push(st);
        }
    }

    fn stair_props(&mut self, risers: usize, treads: usize, riser: f64, going: f64) -> Vec<Ref> {
        vec![
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfRiser',$,IFCCOUNTMEASURE({risers}),$)"
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfTreads',$,IFCCOUNTMEASURE({treads}),$)"
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('RiserHeight',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                real(riser)
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('TreadLength',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                real(going)
            )),
        ]
    }
}
