//! Pipes, ducts and devices.
//!
//! Each stretch of a pipe or duct is an `IfcPipeSegment` or
//! `IfcDuctSegment` (`RIGIDSEGMENT`): its round or rectangular profile
//! swept along it. Devices are `IfcUnitaryEquipment`, `IfcLightFixture`,
//! `IfcOutlet` (`POWEROUTLET`) and `IfcSwitchingDevice` (`TOGGLESWITCH`).
//! Everything of a system goes into its `IfcDistributionSystem`, which
//! serves the building. `ArchiSim_Run` (on each stretch: the run, its
//! order, and on the first one the path) and `ArchiSim_Device` read them
//! back.

use std::collections::BTreeMap;

use archisim_core::{DeviceKind, FittingKind, GlobalId, LevelId, Point2, ServiceSystem};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_services(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
        building: Ref,
    ) {
        let model = self.model;
        let mut by_system: BTreeMap<ServiceSystem, Vec<Ref>> = BTreeMap::new();
        for r in model.runs() {
            let Some(&(_, storey_pl)) = storeys.get(&r.level) else {
                continue;
            };
            let profile = if r.is_duct() {
                self.w.add(format!(
                    "IFCRECTANGLEPROFILEDEF(.AREA.,$,$,{},{})",
                    real(r.width),
                    real(r.height)
                ))
            } else {
                self.w.add(format!(
                    "IFCCIRCLEPROFILEDEF(.AREA.,$,$,{})",
                    real(r.width / 2.0)
                ))
            };
            let zs = r.point_elevations();
            let k = (1.0 + r.slope * r.slope).sqrt();
            for (i, w) in r.path.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                let Some(u) = (b - a).normalized() else {
                    continue;
                };
                // Along its slope.
                let len = a.distance(b) * k;
                // Along the stretch (local z), across it to the left (x),
                // up (y).
                let loc = self.point3(a.x, a.y, zs[i]);
                let axis = self.w.add(format!(
                    "IFCDIRECTION(({},{},{}))",
                    real(u.x / k),
                    real(u.y / k),
                    real(-r.slope / k)
                ));
                let across = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(-u.y), real(u.x)));
                let ap = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({loc},{axis},{across})"));
                let pl = self.w.add(format!("IFCLOCALPLACEMENT({storey_pl},{ap})"));
                let o = self.point3(0.0, 0.0, 0.0);
                let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({o},$,$)"));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                    self.z,
                    real(len)
                ));
                let body = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
                let guid: GlobalId = if i == 0 {
                    r.guid.clone()
                } else {
                    derived_guid(&r.guid, &format!("stretch {i}"))
                };
                let name = string(&format!("{} {}", r.system.name(), r.size_text()));
                // Cables under the street go in their conduits (U5).
                let (class, predefined) = if r.is_duct() {
                    ("IFCDUCTSEGMENT", "RIGIDSEGMENT")
                } else if r.system.is_cable() {
                    ("IFCCABLECARRIERSEGMENT", "CONDUITSEGMENT")
                } else {
                    ("IFCPIPESEGMENT", "RIGIDSEGMENT")
                };
                let el = self.w.add(format!(
                    "{class}({},$,{name},$,$,{pl},{shape},$,.{predefined}.)",
                    string(guid.as_str())
                ));
                let mut props = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Run',$,IFCIDENTIFIER({}),$)",
                        string(r.guid.as_str())
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Index',$,IFCINTEGER({i}),$)"
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('System',$,IFCLABEL({}),$)",
                        string(r.system.code())
                    )),
                ];
                if i == 0 {
                    let path: Vec<String> = r
                        .path
                        .iter()
                        .map(|p| format!("{},{}", real(p.x), real(p.y)))
                        .collect();
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Path',$,IFCTEXT({}),$)",
                        string(&path.join(";"))
                    )));
                    for (key, v) in [
                        ("Elevation", r.elevation),
                        ("Width", r.width),
                        ("Height", r.height),
                    ] {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                            real(v)
                        )));
                    }
                    if r.slope > 0.0 {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Slope',$,IFCREAL({}),$)",
                            real(r.slope)
                        )));
                    }
                }
                self.pset(el, &guid, "ArchiSim_Run", &props);
                contained.entry(r.level).or_default().push(el);
                by_system.entry(r.system).or_default().push(el);
            }
        }
        let mut lighting = Vec::new();
        let mut electrical = Vec::new();
        let mut alarm = Vec::new();
        let mut emergency = Vec::new();
        let mut solar = Vec::new();
        for d in model.devices() {
            let Some(&(_, storey_pl)) = storeys.get(&d.level) else {
                continue;
            };
            // Its body: the model the 3D view shows (G4), as triangles.
            let storey_z = model.level(d.level).map_or(0.0, |l| l.elevation);
            let tris: Vec<[[f64; 3]; 3]> = archisim_geom::solid::device_solids(model, d.id)
                .into_iter()
                .flat_map(|s| s.triangles)
                .collect();
            let shape = self.triangulated_shape(&tris, storey_z);
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let (class, predefined) = d.kind.ifc_class();
            let object_type = if predefined == "USERDEFINED" {
                string(d.kind.name())
            } else {
                "$".into()
            };
            let el = self.w.add(format!(
                "{class}({},$,{},$,{object_type},{pl},{shape},$,.{predefined}.)",
                string(d.guid.as_str()),
                string(&d.name)
            ));
            let props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Kind',$,IFCLABEL({}),$)",
                    string(d.kind.code())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Position',$,IFCTEXT({}),$)",
                    string(&format!("{},{}", real(d.position.x), real(d.position.y)))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Rotation',$,IFCPLANEANGLEMEASURE({}),$)",
                    real(d.rotation)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Elevation',$,IFCLENGTHMEASURE({}),$)",
                    real(d.elevation)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Width',$,IFCLENGTHMEASURE({}),$)",
                    real(d.width)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Depth',$,IFCLENGTHMEASURE({}),$)",
                    real(d.depth)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Height',$,IFCLENGTHMEASURE({}),$)",
                    real(d.height)
                )),
            ];
            let mut props = props;
            // Its circuit, when given by hand (E13).
            if let Some((_, n)) = model.device_circuits().find(|(x, _)| *x == d.id) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Circuit',$,IFCINTEGER({n}),$)"
                )));
            }
            // A light point's lamp, by its type's name (G4).
            if let Some(t) = d.fixture.and_then(|t| model.object_type(t)) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Fixture',$,IFCLABEL({}),$)",
                    string(&t.name)
                )));
            }
            self.pset(el, &d.guid, "ArchiSim_Device", &props);
            contained.entry(d.level).or_default().push(el);
            match d.kind {
                DeviceKind::Light => lighting.push(el),
                k if k.is_electric() => electrical.push(el),
                DeviceKind::SmokeDetector | DeviceKind::CallPoint | DeviceKind::Siren => {
                    alarm.push(el)
                }
                DeviceKind::EmergencyLight | DeviceKind::ExitSign => emergency.push(el),
                DeviceKind::SolarPanel => solar.push(el),
                DeviceKind::Extinguisher => by_system
                    .entry(ServiceSystem::FireProtection)
                    .or_default()
                    .push(el),
                DeviceKind::Equipment => {}
                k => {
                    if let Some(s) = k.system() {
                        by_system.entry(s).or_default().push(el);
                    }
                }
            }
        }
        self.write_fittings(storeys, contained, &mut by_system);
        // One distribution system per system, serving the building.
        let mut systems: Vec<(String, &str, Vec<Ref>)> = by_system
            .into_iter()
            .map(|(s, els)| (s.name().to_string(), s.ifc_system(), els))
            .collect();
        // The lamps placed as objects (already written as light
        // fixtures) belong to the lighting too.
        let lamps = model.lamp_points();
        if !lamps.is_empty() {
            let written = self.w.by_guid();
            for (o, _) in lamps {
                if let Some(r) = model.object(o).and_then(|ob| written.get(ob.guid.as_str())) {
                    lighting.push(*r);
                }
            }
        }
        if !lighting.is_empty() {
            systems.push(("Alumbrado".into(), "LIGHTING", lighting));
        }
        if !electrical.is_empty() {
            systems.push(("Fuerza".into(), "ELECTRICAL", electrical));
        }
        if !alarm.is_empty() {
            systems.push(("Detección y alarma".into(), "FIREPROTECTION", alarm));
        }
        if !emergency.is_empty() {
            systems.push(("Alumbrado de emergencia".into(), "LIGHTING", emergency));
        }
        if !solar.is_empty() {
            systems.push(("Fotovoltaica".into(), "ELECTRICAL", solar));
        }
        for (name, predefined, els) in systems {
            let guid = derived_guid(&model.project_guid, &format!("system {name}"));
            let sys = self.w.add(format!(
                "IFCDISTRIBUTIONSYSTEM({},$,{},$,$,$,.{predefined}.)",
                string(guid.as_str()),
                string(&name)
            ));
            self.w.add(format!(
                "IFCRELASSIGNSTOGROUP({},$,$,$,{},$,{sys})",
                string(derived_guid(&guid, "group").as_str()),
                list(&els)
            ));
            self.w.add(format!(
                "IFCRELSERVICESBUILDINGS({},$,$,$,{sys},({building}))",
                string(derived_guid(&guid, "serves").as_str())
            ));
        }
    }

    /// Elbows and tees (`IfcPipeFitting`, `IfcDuctFitting`: `BEND`,
    /// `JUNCTION`): a short sleeve along each way out of them. They follow
    /// from the runs, so `ArchiSim_Fitting` only says so (the import skips
    /// them).
    fn write_fittings(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
        by_system: &mut BTreeMap<ServiceSystem, Vec<Ref>>,
    ) {
        let model = self.model;
        for f in model.run_fittings() {
            let Some(r) = model.run(f.run) else {
                continue;
            };
            let Some(&(_, storey_pl)) = storeys.get(&r.level) else {
                continue;
            };
            let grow = 0.01_f64.max(r.width * 0.15);
            let profile = if r.is_duct() {
                self.w.add(format!(
                    "IFCRECTANGLEPROFILEDEF(.AREA.,$,$,{},{})",
                    real(r.width + grow),
                    real(r.height + grow)
                ))
            } else {
                self.w.add(format!(
                    "IFCCIRCLEPROFILEDEF(.AREA.,$,$,{})",
                    real((r.width + grow) / 2.0)
                ))
            };
            let leg = (r.width.max(r.height) * 0.75).max(0.03);
            let mut ways: Vec<Point2> = vec![-f.from, f.to];
            if f.kind == FittingKind::Tee {
                ways.push(f.from);
            }
            let mut items = Vec::new();
            for d in ways {
                let o = self.point3(f.position.x, f.position.y, f.elevation);
                let axis = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(d.x), real(d.y)));
                let across = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(-d.y), real(d.x)));
                let ap = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({o},{axis},{across})"));
                items.push(self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{ap},{},{})",
                    self.z,
                    real(leg)
                )));
            }
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',{})",
                self.body_ctx,
                list(&items)
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let class = if r.is_duct() {
                "IFCDUCTFITTING"
            } else if r.system.is_cable() {
                "IFCCABLECARRIERFITTING"
            } else {
                "IFCPIPEFITTING"
            };
            let predefined = match f.kind {
                FittingKind::Elbow => "BEND",
                FittingKind::Tee => "JUNCTION",
            };
            let el = self.w.add(format!(
                "{class}({},$,{},$,$,{pl},{shape},$,.{predefined}.)",
                string(f.guid.as_str()),
                string(&format!(
                    "{} {} {}",
                    f.kind.name(),
                    r.system.name(),
                    r.size_text()
                ))
            ));
            let props = vec![self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Run',$,IFCIDENTIFIER({}),$)",
                string(r.guid.as_str())
            ))];
            self.pset(el, &f.guid, "ArchiSim_Fitting", &props);
            contained.entry(r.level).or_default().push(el);
            by_system.entry(r.system).or_default().push(el);
        }
    }
}
