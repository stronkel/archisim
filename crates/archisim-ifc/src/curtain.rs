//! Curtain walls: an `IfcCurtainWall` in its storey, typed by an
//! `IfcCurtainWallType`, made of `IfcMember` (`MULLION`: the border,
//! mullions, transoms and caps), `IfcPlate` (`CURTAIN_PANEL`) and
//! `IfcShadingDevice` (`LOUVER`) aggregated to it, each a box extruded
//! upwards. `ArchiSim_CurtainWall` (on the wall: line, top, height) and
//! `ArchiSim_CurtainWallType` (on the type: grid, frames, panels, extras;
//! materials by name) let ArchiSim read it back as one.

use std::collections::BTreeMap;

use archisim_core::{
    CurtainDesign, CurtainDivision, CurtainPieceKind, CurtainProfile, CurtainRest, CurtainTypeId,
    LevelId, MaterialId, Model, WallTop,
};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

/// A division as text: its fixed bays, then `max <spacing>` or
/// `count <n>` ("1 0.6 max 1.2").
pub(crate) fn division_text(d: &CurtainDivision) -> String {
    let mut out: Vec<String> = d.fixed.iter().map(|v| real(*v)).collect();
    match d.rest {
        CurtainRest::MaxSpacing(v) => out.push(format!("max {}", real(v))),
        CurtainRest::Count(n) => out.push(format!("count {n}")),
    }
    out.join(" ")
}

fn material_name(model: &Model, m: MaterialId) -> String {
    model
        .material(m)
        .map(|m| m.name.clone())
        .unwrap_or_default()
}

/// The properties of a curtain wall type's set, as (name, text).
pub(crate) fn design_texts(model: &Model, d: &CurtainDesign) -> Vec<(String, String)> {
    let profile = |p: &CurtainProfile| {
        format!(
            "{} {}|{}",
            real(p.width),
            real(p.depth),
            material_name(model, p.material)
        )
    };
    let mut out = vec![
        ("Columns".to_string(), division_text(&d.columns)),
        ("Rows".to_string(), division_text(&d.rows)),
        ("Border".to_string(), profile(&d.border)),
        ("Mullion".to_string(), profile(&d.mullion)),
        ("Transom".to_string(), profile(&d.transom)),
        (
            "Joint".to_string(),
            match d.joint {
                archisim_core::CurtainJoint::Mullions => "mullions",
                archisim_core::CurtainJoint::Transoms => "transoms",
            }
            .to_string(),
        ),
    ];
    for (i, p) in d.panels.iter().enumerate() {
        out.push((
            format!("Panel{}", i + 1),
            format!(
                "{}|{}|{}",
                real(p.thickness),
                material_name(model, p.material),
                p.name
            ),
        ));
    }
    if !d.row_panels.is_empty() {
        let rows: Vec<String> = d.row_panels.iter().map(|r| r.to_string()).collect();
        out.push(("RowPanels".into(), rows.join(" ")));
    }
    if let Some(c) = d.caps {
        out.push((
            "Caps".into(),
            format!(
                "{} {}|{}",
                real(c.width),
                real(c.depth),
                material_name(model, c.material)
            ),
        ));
    }
    if let Some(l) = d.louvres {
        out.push((
            "Louvres".into(),
            format!(
                "{} {} {} {}|{}",
                real(l.depth),
                real(l.thickness),
                real(l.spacing),
                real(l.offset),
                material_name(model, l.material)
            ),
        ));
    }
    out
}

impl Exporter<'_> {
    /// The curtain wall types, each with its own property set.
    fn write_curtain_types(&mut self) -> BTreeMap<CurtainTypeId, Ref> {
        let model = self.model;
        let mut out = BTreeMap::new();
        for t in model.curtain_types() {
            let props: Vec<Ref> = design_texts(model, &t.design)
                .into_iter()
                .map(|(k, v)| {
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE({},$,IFCTEXT({}),$)",
                        string(&k),
                        string(&v)
                    ))
                })
                .collect();
            let set = self.w.add(format!(
                "IFCPROPERTYSET({},$,'ArchiSim_CurtainWallType',$,{})",
                string(derived_guid(&t.guid, "ArchiSim_CurtainWallType").as_str()),
                list(&props)
            ));
            let ty = self.w.add(format!(
                "IFCCURTAINWALLTYPE({},$,{},$,$,({set}),$,$,$,.NOTDEFINED.)",
                string(t.guid.as_str()),
                string(&t.name),
            ));
            out.insert(t.id, ty);
        }
        out
    }

    pub(crate) fn write_curtain_walls(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        if model.curtain_walls().next().is_none() && model.curtain_types().next().is_none() {
            return;
        }
        let types = self.write_curtain_types();
        let mut typed: BTreeMap<CurtainTypeId, Vec<Ref>> = BTreeMap::new();
        for c in model.curtain_walls() {
            let Some(&(_, storey_pl)) = storeys.get(&c.level) else {
                continue;
            };
            let Some(t) = model.curtain_type(c.curtain_type) else {
                continue;
            };
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let name = format!("{} {}", t.name, c.id.0);
            let cw = self.w.add(format!(
                "IFCCURTAINWALL({},$,{},$,{},{pl},$,{},.NOTDEFINED.)",
                string(c.guid.as_str()),
                string(&name),
                string(&t.name),
                string(&c.id.0.to_string()),
            ));
            let mut parts = Vec::new();
            let mut by_material: BTreeMap<MaterialId, Vec<Ref>> = BTreeMap::new();
            let (mut nm, mut np, mut nc, mut nl) = (0, 0, 0, 0);
            for p in c.pieces(&t.design) {
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
                let length = if p.is_vertical() { depth } else { along };
                let e = match p.kind {
                    CurtainPieceKind::Panel(k) => {
                        np += 1;
                        let guid = derived_guid(&c.guid, &format!("panel {np}"));
                        let kind = t.design.panels.get(k).map_or("panel", |p| p.name.as_str());
                        let e = self.w.add(format!(
                            "IFCPLATE({},$,{},$,$,{ppl},{shape},$,.CURTAIN_PANEL.)",
                            string(guid.as_str()),
                            string(&format!("{name} · {kind} {np}")),
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
                        e
                    }
                    CurtainPieceKind::Louvre => {
                        nl += 1;
                        let guid = derived_guid(&c.guid, &format!("louvre {nl}"));
                        self.w.add(format!(
                            "IFCSHADINGDEVICE({},$,{},$,$,{ppl},{shape},$,.LOUVER.)",
                            string(guid.as_str()),
                            string(&format!("{name} · lama {nl}")),
                        ))
                    }
                    kind => {
                        let (what, guid) = if kind == CurtainPieceKind::Cap {
                            nc += 1;
                            ("tapajuntas", derived_guid(&c.guid, &format!("cap {nc}")))
                        } else {
                            nm += 1;
                            let what = match kind {
                                CurtainPieceKind::Border => "marco",
                                CurtainPieceKind::Mullion => "montante",
                                _ => "travesaño",
                            };
                            (what, derived_guid(&c.guid, &format!("frame {nm}")))
                        };
                        let number = if kind == CurtainPieceKind::Cap {
                            nc
                        } else {
                            nm
                        };
                        let e = self.w.add(format!(
                            "IFCMEMBER({},$,{},$,$,{ppl},{shape},$,.MULLION.)",
                            string(guid.as_str()),
                            string(&format!("{name} · {what} {number}")),
                        ));
                        self.quantities(
                            e,
                            &guid,
                            "Qto_MemberBaseQuantities",
                            &[
                                ("Length", "IFCQUANTITYLENGTH", length),
                                ("NetVolume", "IFCQUANTITYVOLUME", volume),
                            ],
                        );
                        e
                    }
                };
                by_material.entry(p.material).or_default().push(e);
                parts.push(e);
            }
            for (mat, items) in &by_material {
                if let Some(m) = materials.get(mat) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,{},{m})",
                        string(derived_guid(&c.guid, &format!("material {}", mat.0)).as_str()),
                        list(items)
                    ));
                }
            }
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
            for (key, v) in [("BaseOffset", c.base_offset), ("Height", c.height)] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            self.pset(cw, &c.guid, "ArchiSim_CurtainWall", &props);
            contained.entry(c.level).or_default().push(cw);
            typed.entry(c.curtain_type).or_default().push(cw);
        }
        for (t, walls) in typed {
            let Some(ty) = types.get(&t) else { continue };
            let guid = model
                .curtain_type(t)
                .map(|t| derived_guid(&t.guid, "defines"))
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{ty})",
                string(guid.as_str()),
                list(&walls)
            ));
        }
    }
}
