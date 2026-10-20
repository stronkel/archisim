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

use archisim_core::{DeviceKind, GlobalId, LevelId, ServiceSystem};

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
            for (i, w) in r.path.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                let Some(u) = (b - a).normalized() else {
                    continue;
                };
                let len = a.distance(b);
                // Along the stretch (local z), across it to the left (x),
                // up (y).
                let loc = self.point3(a.x, a.y, r.elevation);
                let axis = self
                    .w
                    .add(format!("IFCDIRECTION(({},{},0.))", real(u.x), real(u.y)));
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
                let class = if r.is_duct() {
                    "IFCDUCTSEGMENT"
                } else {
                    "IFCPIPESEGMENT"
                };
                let el = self.w.add(format!(
                    "{class}({},$,{name},$,$,{pl},{shape},$,.RIGIDSEGMENT.)",
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
                }
                self.pset(el, &guid, "ArchiSim_Run", &props);
                contained.entry(r.level).or_default().push(el);
                by_system.entry(r.system).or_default().push(el);
            }
        }
        let mut lighting = Vec::new();
        let mut electrical = Vec::new();
        for d in model.devices() {
            let Some(&(_, storey_pl)) = storeys.get(&d.level) else {
                continue;
            };
            let outline = d.outline();
            let curve = self.polyline(&outline, true);
            let profile = self
                .w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"));
            let o = self.point3(0.0, 0.0, d.elevation);
            let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({o},$,$)"));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(d.height)
            ));
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let (class, predefined) = match d.kind {
                DeviceKind::Equipment => ("IFCUNITARYEQUIPMENT", "USERDEFINED"),
                DeviceKind::Light => ("IFCLIGHTFIXTURE", "POINTSOURCE"),
                DeviceKind::Socket => ("IFCOUTLET", "POWEROUTLET"),
                DeviceKind::Switch => ("IFCSWITCHINGDEVICE", "TOGGLESWITCH"),
            };
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
            self.pset(el, &d.guid, "ArchiSim_Device", &props);
            contained.entry(d.level).or_default().push(el);
            match d.kind {
                DeviceKind::Light => lighting.push(el),
                DeviceKind::Socket | DeviceKind::Switch => electrical.push(el),
                DeviceKind::Equipment => {}
            }
        }
        // One distribution system per system, serving the building.
        let mut systems: Vec<(String, &str, Vec<Ref>)> = by_system
            .into_iter()
            .map(|(s, els)| (s.name().to_string(), s.ifc_system(), els))
            .collect();
        if !lighting.is_empty() {
            systems.push(("Alumbrado".into(), "LIGHTING", lighting));
        }
        if !electrical.is_empty() {
            systems.push(("Fuerza".into(), "ELECTRICAL", electrical));
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
}
