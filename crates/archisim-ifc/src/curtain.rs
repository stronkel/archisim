//! Curtain walls: an `IfcCurtainWall` in its storey, made of `IfcMember`
//! (`MULLION`, the vertical and horizontal frames) and `IfcPlate`
//! (`CURTAIN_PANEL`) aggregated to it, each a box extruded upwards.
//! `ArchiSim_CurtainWall` lets ArchiSim read it back as one.

use std::collections::BTreeMap;

use archisim_core::{CurtainPieceKind, LevelId, MaterialId, WallTop};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_curtain_walls(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for c in model.curtain_walls() {
            let Some(&(_, storey_pl)) = storeys.get(&c.level) else {
                continue;
            };
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let name = format!("Muro cortina {}", c.id.0);
            let cw = self.w.add(format!(
                "IFCCURTAINWALL({},$,{},$,$,{pl},$,{},.NOTDEFINED.)",
                string(c.guid.as_str()),
                string(&name),
                string(&c.id.0.to_string()),
            ));
            let (mut frames, mut panels) = (Vec::new(), Vec::new());
            let (mut nm, mut np) = (0, 0);
            for p in c.pieces() {
                let foot = c.piece_footprint(&p);
                let poly = self.polyline(&foot, true);
                let def = self
                    .w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{poly})"));
                let at = self.point3(0.0, 0.0, c.base_offset + p.up.0);
                let ppl = self.placement(Some(pl), at, None);
                let spl = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
                let depth = p.up.1 - p.up.0;
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({def},{spl},{},{})",
                    self.z,
                    real(depth)
                ));
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let (along, across) = (p.along.1 - p.along.0, p.across.1 - p.across.0);
                let volume = along * across * depth;
                match p.kind {
                    CurtainPieceKind::Panel => {
                        np += 1;
                        let guid = derived_guid(&c.guid, &format!("panel {np}"));
                        let e = self.w.add(format!(
                            "IFCPLATE({},$,{},$,$,{ppl},{shape},$,.CURTAIN_PANEL.)",
                            string(guid.as_str()),
                            string(&format!("{name} · panel {np}")),
                        ));
                        self.quantities(
                            e,
                            &guid,
                            "Qto_PlateBaseQuantities",
                            &[
                                ("Width", "IFCQUANTITYLENGTH", across),
                                ("NetArea", "IFCQUANTITYAREA", along * depth),
                                ("NetVolume", "IFCQUANTITYVOLUME", volume),
                            ],
                        );
                        panels.push(e);
                    }
                    kind => {
                        nm += 1;
                        let guid = derived_guid(&c.guid, &format!("frame {nm}"));
                        let what = if kind == CurtainPieceKind::Mullion {
                            "montante"
                        } else {
                            "travesaño"
                        };
                        let e = self.w.add(format!(
                            "IFCMEMBER({},$,{},$,$,{ppl},{shape},$,.MULLION.)",
                            string(guid.as_str()),
                            string(&format!("{name} · {what} {nm}")),
                        ));
                        let length = if kind == CurtainPieceKind::Mullion {
                            depth
                        } else {
                            along
                        };
                        self.quantities(
                            e,
                            &guid,
                            "Qto_MemberBaseQuantities",
                            &[
                                ("Length", "IFCQUANTITYLENGTH", length),
                                ("NetVolume", "IFCQUANTITYVOLUME", volume),
                            ],
                        );
                        frames.push(e);
                    }
                }
            }
            for (mat, items, key) in [
                (c.frame_material, &frames, "frame material"),
                (c.panel_material, &panels, "panel material"),
            ] {
                if let (Some(m), false) = (materials.get(&mat), items.is_empty()) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,{},{m})",
                        string(derived_guid(&c.guid, key).as_str()),
                        list(items)
                    ));
                }
            }
            let parts: Vec<Ref> = frames.iter().chain(&panels).copied().collect();
            self.rel_aggregates(derived_guid(&c.guid, "parts"), cw, &parts);
            let mut common = vec![self
                .w
                .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$)")];
            common.extend(self.status_prop(c.id.0));
            self.pset(cw, &c.guid, "Pset_CurtainWallCommon", &common);
            self.quantities(
                cw,
                &c.guid,
                "Qto_CurtainWallQuantities",
                &[
                    ("Length", "IFCQUANTITYLENGTH", c.length()),
                    ("Height", "IFCQUANTITYLENGTH", c.height),
                    ("GrossSideArea", "IFCQUANTITYAREA", c.length() * c.height),
                ],
            );
            // What ArchiSim needs to read it back as one curtain wall.
            let top = match c.top {
                WallTop::Level { level, offset } => model
                    .level(level)
                    .map(|l| format!("{} {}", l.guid, real(offset)))
                    .unwrap_or_default(),
                WallTop::Unconnected => String::new(),
            };
            let line = format!(
                "{} {};{} {}",
                real(c.start.x),
                real(c.start.y),
                real(c.end.x),
                real(c.end.y)
            );
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Line',$,IFCTEXT({}),$)",
                    string(&line)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Top',$,IFCTEXT({}),$)",
                    string(&top)
                )),
            ];
            for (key, v) in [
                ("BaseOffset", c.base_offset),
                ("Height", c.height),
                ("Bay", c.bay),
                ("Row", c.row),
                ("FrameWidth", c.frame_width),
                ("FrameDepth", c.frame_depth),
                ("PanelThickness", c.panel_thickness),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            self.pset(cw, &c.guid, "ArchiSim_CurtainWall", &props);
            contained.entry(c.level).or_default().push(cw);
        }
    }
}
