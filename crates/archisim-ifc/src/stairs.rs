//! Stairs: an `IfcStair` in its storey, made of one `IfcStairFlight` per
//! flight and one `IfcSlab` (`LANDING`) per landing (`IfcRelAggregates`).
//! Each flight is its side profile (the steps on their slab) swept across
//! its width; each landing a slab of its outline. `Pset_StairCommon` and
//! `Pset_StairFlightCommon` give risers, treads and their sizes;
//! `ArchiSim_Stair` lets ArchiSim read the stair back as one.

use std::collections::BTreeMap;

use archisim_core::{
    LevelId, MaterialId, Point2, Stair, StairForm, StairLayout, StairStructure, StairUse, WallTop,
};
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
            let shape = stair_shape(stair, &l);
            let st = self.w.add(format!(
                "IFCSTAIR({},$,{},$,$,{pl},$,{},.{shape}.)",
                string(stair.guid.as_str()),
                string(&name),
                string(&stair.id.0.to_string()),
            ));
            let d = stair.design;
            // A concrete stair of straight flights is written exactly: each
            // flight its profile swept, each landing a slab. Any other, piece
            // by piece as its faces.
            let exact = d.form == StairForm::Landings
                && !d.split
                && d.structure == StairStructure::Slab
                && d.finish() == 0.0;
            let mut parts = Vec::new();
            let mut own_material = Vec::new();
            if !exact {
                let flight_type = match d.form {
                    StairForm::Landings if d.split && l.steps.iter().any(|s| s.turned) => "WINDER",
                    StairForm::Landings => "STRAIGHT",
                    StairForm::Winders => "WINDER",
                    StairForm::Spiral => "SPIRAL",
                };
                let mut stringers = 0;
                for part in archisim_geom::stairs::stair_parts(model, stair.id) {
                    use archisim_geom::stairs::StairPartKind as K;
                    let (key, label) = match part.kind {
                        K::Railing => continue,
                        K::Run(i) => (format!("flight {i}"), format!("tramo {}", i + 1)),
                        K::Landing(i) => (format!("landing {i}"), format!("meseta {}", i + 1)),
                        K::Finish => ("finish".to_string(), "peldañeado".to_string()),
                        K::Stringer(left) => {
                            stringers += 1;
                            (
                                format!("stringer {}", if left { "left" } else { "right" }),
                                format!("zanca {stringers}"),
                            )
                        }
                        K::Spine => ("spine".to_string(), "zanca central".to_string()),
                        K::Newel => ("newel".to_string(), "mástil".to_string()),
                    };
                    let guid = derived_guid(&stair.guid, &key);
                    let shape = self.triangulated_shape(&part.triangles, storey_z);
                    let ppl = self.placement(Some(pl), self.origin, None);
                    let full = string(&format!("{name} · {label}"));
                    let g = string(guid.as_str());
                    let e = match part.kind {
                        K::Run(i) => {
                            let run = &l.runs[i];
                            let (risers, treads) = (run.steps.len() + 1, run.steps.len());
                            let going = l.steps[run.steps.clone()]
                                .iter()
                                .map(|s| s.going)
                                .fold(f64::INFINITY, f64::min);
                            let going = if going.is_finite() {
                                going
                            } else {
                                stair.going
                            };
                            let fl = self.w.add(format!(
                                "IFCSTAIRFLIGHT({g},$,{full},$,$,{ppl},{shape},$,{risers},{treads},{},{},.{flight_type}.)",
                                real(l.riser),
                                real(going)
                            ));
                            let props = self.stair_props(risers, treads, l.riser, going);
                            self.pset(fl, &guid, "Pset_StairFlightCommon", &props);
                            fl
                        }
                        K::Landing(_) => self.w.add(format!(
                            "IFCSLAB({g},$,{full},$,$,{ppl},{shape},$,.LANDING.)"
                        )),
                        K::Finish => self.w.add(format!(
                            "IFCCOVERING({g},$,{full},$,$,{ppl},{shape},$,.FLOORING.)"
                        )),
                        K::Newel => self.w.add(format!(
                            "IFCMEMBER({g},$,{full},$,$,{ppl},{shape},$,.POST.)"
                        )),
                        _ => self.w.add(format!(
                            "IFCMEMBER({g},$,{full},$,$,{ppl},{shape},$,.STRINGER.)"
                        )),
                    };
                    own_material.push((e, part.material, guid));
                    parts.push(e);
                }
            } else {
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
                    let axis =
                        self.w
                            .add(format!("IFCDIRECTION(({},{},0.))", real(-n.x), real(-n.y)));
                    let xdir =
                        self.w
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
            }
            // Railings: one IfcRailing, its bars as a triangulated body
            // (with its own material, so not in the flights' list).
            let mut railing = None;
            let pieces = archisim_geom::solid::railing_pieces(model, stair.id);
            if !pieces.is_empty() {
                let guid = derived_guid(&stair.guid, "railings");
                let shape = self.styled_shape(&pieces, storey_z);
                let rpl = self.placement(Some(pl), self.origin, None);
                let rail = self.w.add(format!(
                    "IFCRAILING({},$,{},$,$,{rpl},{shape},$,.HANDRAIL.)",
                    string(guid.as_str()),
                    string(&format!("{name} · barandilla")),
                ));
                let mat = pieces[0].0;
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
            if exact {
                if let Some(m) = materials.get(&stair.material) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,{},{m})",
                        string(derived_guid(&stair.guid, "material").as_str()),
                        list(&parts)
                    ));
                }
            }
            for (e, mat, guid) in own_material {
                if let Some(m) = materials.get(&mat) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({e}),{m})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
            }
            parts.extend(railing);
            self.rel_aggregates(derived_guid(&stair.guid, "parts"), st, &parts);
            let going = l.going();
            let going = if going.is_finite() {
                going
            } else {
                stair.going
            };
            let mut props = self.stair_props(l.risers(), l.treads(), l.riser, going);
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
            if let Some(t) = stair.railing_type.and_then(|t| model.object_type(t)) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('RailingType',$,IFCLABEL({}),$)",
                    string(&t.name)
                )));
            }
            let material_name = |m: Option<MaterialId>| {
                m.and_then(|m| model.material(m))
                    .map_or(String::new(), |m| m.name.clone())
            };
            for (key, v) in [
                (
                    "Form",
                    match d.form {
                        StairForm::Landings => "landings",
                        StairForm::Winders => "winders",
                        StairForm::Spiral => "spiral",
                    }
                    .to_string(),
                ),
                (
                    "Structure",
                    match d.structure {
                        StairStructure::Slab => "slab",
                        StairStructure::Stringers => "stringers",
                        StairStructure::Spine => "spine",
                        StairStructure::Cantilever => "cantilever",
                        StairStructure::Newel => "newel",
                    }
                    .to_string(),
                ),
                ("Material", material_name(Some(stair.material))),
                ("TreadMaterial", material_name(d.tread_material)),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLABEL({}),$)",
                    string(&v)
                )));
            }
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('ClosedRisers',$,IFCBOOLEAN({}),$)",
                if d.closed_risers { ".T." } else { ".F." }
            )));
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Sweep',$,IFCPLANEANGLEMEASURE({}),$)",
                real(d.sweep)
            )));
            if d.split {
                props.push(
                    self.w
                        .add("IFCPROPERTYSINGLEVALUE('SplitLandings',$,IFCBOOLEAN(.T.),$)"),
                );
            }
            if d.arrival > 0.0 {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('ArrivalLanding',$,IFCPLANEANGLEMEASURE({}),$)",
                    real(d.arrival)
                )));
            }
            for (key, v) in [
                ("Core", d.core),
                ("Tread", d.tread),
                ("StringerWidth", d.stringer[0]),
                ("StringerDepth", d.stringer[1]),
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

    /// A body of triangles (model space; heights from the storey).
    pub(crate) fn triangulated_shape(&mut self, tris: &[[[f64; 3]; 3]], storey_z: f64) -> Ref {
        let mut pts: Vec<String> = Vec::new();
        let mut welded = std::collections::HashMap::new();
        let mut faces: Vec<String> = Vec::new();
        for t in tris {
            let f = t.map(|p| {
                crate::step::weld(
                    &mut pts,
                    &mut welded,
                    format!("({},{},{})", real(p[0]), real(p[1]), real(p[2] - storey_z)),
                )
            });
            faces.push(format!("({},{},{})", f[0], f[1], f[2]));
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
        self.w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"))
    }

    /// A body of triangles, one face set per material with its colour.
    fn styled_shape(&mut self, pieces: &[(MaterialId, Vec<[[f64; 3]; 3]>)], storey_z: f64) -> Ref {
        let mut items = Vec::new();
        for (material, tris) in pieces {
            let mut pts: Vec<String> = Vec::new();
            let mut welded = std::collections::HashMap::new();
            let mut faces: Vec<String> = Vec::new();
            for t in tris {
                let f = t.map(|p| {
                    crate::step::weld(
                        &mut pts,
                        &mut welded,
                        format!("({},{},{})", real(p[0]), real(p[1]), real(p[2] - storey_z)),
                    )
                });
                faces.push(format!("({},{},{})", f[0], f[1], f[2]));
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
            if let Some(&style) = self.styles.get(material) {
                self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
            }
            items.push(body);
        }
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',{})",
            self.body_ctx,
            list(&items)
        ));
        self.w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"))
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

/// The kind of stair IFC names: by its turns, with landings or winders,
/// or a spiral.
fn stair_shape(stair: &Stair, l: &StairLayout) -> &'static str {
    // The quarter turns of its walking line.
    let turns: Vec<f64> = stair
        .path
        .windows(3)
        .filter_map(|w| {
            let a = (w[1] - w[0]).normalized()?;
            let b = (w[2] - w[1]).normalized()?;
            let angle = a.cross(b).atan2(a.dot(b));
            (angle.abs() > 0.1).then_some(angle)
        })
        .collect();
    let quarters = (turns.iter().sum::<f64>().abs() / std::f64::consts::FRAC_PI_2).round() as usize;
    match stair.design.form {
        StairForm::Spiral => "SPIRAL_STAIR",
        StairForm::Winders => match quarters {
            0 => "STRAIGHT_RUN_STAIR",
            1 => "QUARTER_WINDING_STAIR",
            2 if turns.len() == 2 && (stair.path.len() == 4) && is_u(stair) => "HALF_WINDING_STAIR",
            2 => "TWO_QUARTER_WINDING_STAIR",
            3 => "THREE_QUARTER_WINDING_STAIR",
            _ => "USERDEFINED",
        },
        StairForm::Landings if stair.design.split && l.runs.len() == 1 => match quarters {
            0 => "STRAIGHT_RUN_STAIR",
            1 => "QUARTER_WINDING_STAIR",
            2 if is_u(stair) => "HALF_WINDING_STAIR",
            2 => "TWO_QUARTER_WINDING_STAIR",
            3 => "THREE_QUARTER_WINDING_STAIR",
            _ => "USERDEFINED",
        },
        StairForm::Landings => match (l.flights.len(), quarters) {
            (1, _) => "STRAIGHT_RUN_STAIR",
            (2, 0) => "TWO_STRAIGHT_RUN_STAIR",
            (2, 1) => "QUARTER_TURN_STAIR",
            (2, 2) => "HALF_TURN_STAIR",
            (3, 2) => "TWO_QUARTER_TURN_STAIR",
            (_, 3) => "THREE_QUARTER_TURN_STAIR",
            _ => "USERDEFINED",
        },
    }
}

/// Two turns close together, as a U: the piece between them shorter than
/// twice the stair's width.
fn is_u(stair: &Stair) -> bool {
    stair.path[1].distance(stair.path[2]) < 2.0 * stair.width
}
