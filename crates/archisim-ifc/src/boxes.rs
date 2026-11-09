//! Modular boxes (M1): an `IfcElementAssembly` «Caja modular» assembled in
//! the factory (`.FACTORY.`) that aggregates what the box holds (its
//! walls, doors and windows, floors, ceilings, columns, beams and
//! furniture), which leave the storey's containment for it, as IFC asks
//! (an element is either contained or a part). Its placement is the box's
//! origin at the underside of its chassis, `x` along its length; its shape,
//! only its bounding box. `ArchiSim_Box` keeps what ArchiSim needs to read
//! it back: mark, kind, size and the state of its four sides.

use std::collections::BTreeMap;

use archisim_core::{LevelId, SIDE_NAMES};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

/// The `ObjectType` of a box's assembly, by which the importer knows it.
pub const BOX_OBJECT_TYPE: &str = "Caja modular";

impl Exporter<'_> {
    pub(crate) fn write_boxes(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        if model.boxes().next().is_none() {
            return;
        }
        let written = self.w.by_guid();
        let levels: std::collections::BTreeSet<LevelId> = model.boxes().map(|b| b.level).collect();
        for level in levels {
            let Some(&(_, storey_pl)) = storeys.get(&level) else {
                continue;
            };
            let members = model.box_members_on(level);
            for b in model.boxes_on_level(level) {
                let loc = self.point3(b.origin.x, b.origin.y, b.bottom);
                let dir = self.w.add(format!(
                    "IFCDIRECTION(({},{},0.))",
                    real(b.angle.cos()),
                    real(b.angle.sin())
                ));
                let pl = self.placement(Some(storey_pl), loc, Some(dir));
                let corner = self.point3(0.0, 0.0, 0.0);
                let bb = self.w.add(format!(
                    "IFCBOUNDINGBOX({corner},{},{},{})",
                    real(b.length),
                    real(b.width),
                    real(b.height)
                ));
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Box','BoundingBox',({bb}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let description = if b.kind.is_empty() {
                    "$".to_string()
                } else {
                    string(&b.kind)
                };
                let el = self.w.add(format!(
                    "IFCELEMENTASSEMBLY({},$,{},{description},{},{pl},{shape},{},.FACTORY.,.USERDEFINED.)",
                    string(b.guid.as_str()),
                    string(&b.mark),
                    string(BOX_OBJECT_TYPE),
                    string(&b.mark),
                ));
                let mut props = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Mark',$,IFCLABEL({}),$)",
                        string(&b.mark)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Kind',$,IFCLABEL({}),$)",
                        string(&b.kind)
                    )),
                ];
                for (name, v) in [
                    ("OriginX", b.origin.x),
                    ("OriginY", b.origin.y),
                    ("Angle", b.angle),
                    ("Length", b.length),
                    ("Width", b.width),
                    ("Bottom", b.bottom),
                    ("Height", b.height),
                ] {
                    let value = if name == "Angle" {
                        format!("IFCPLANEANGLEMEASURE({})", real(v))
                    } else {
                        format!("IFCLENGTHMEASURE({})", real(v))
                    };
                    props.push(
                        self.w
                            .add(format!("IFCPROPERTYSINGLEVALUE('{name}',$,{value},$)")),
                    );
                }
                // Its chassis (M3), to read it back.
                let c = &b.chassis;
                for (name, tube) in [
                    ("EdgeTube", c.edge),
                    ("PostTube", c.post),
                    ("JoistTube", c.joist),
                    ("RoofJoistTube", c.roof_joist),
                ] {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('{name}',$,IFCLABEL({}),$)",
                        string(&tube.label())
                    )));
                }
                for (name, v) in [("JoistSpacing", c.spacing), ("RoofFrameAt", c.roof_at)] {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('{name}',$,IFCLENGTHMEASURE({}),$)",
                        real(v)
                    )));
                }
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('MiddleBeam',$,IFCBOOLEAN({}),$)",
                    if c.middle_beam { ".T." } else { ".F." }
                )));
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('LiftPoints',$,IFCLABEL({}),$)",
                    string(c.lift.code())
                )));
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('ChassisWeight',$,IFCMASSMEASURE({}),$)",
                    real(archisim_geom::boxes::chassis_weight(b))
                )));
                for (i, side) in b.sides.iter().enumerate() {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Side{}',$,IFCLABEL({}),$)",
                        SIDE_NAMES[i],
                        string(side.code())
                    )));
                }
                self.pset(el, &b.guid, "ArchiSim_Box", &props);

                // Its parts leave the storey for it.
                let mut parts: Vec<Ref> = members
                    .get(&b.id)
                    .map(|m| {
                        m.ids()
                            .into_iter()
                            .filter_map(|id| model.element_guid(id))
                            .filter_map(|g| written.get(g.as_str()).copied())
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(list_of) = contained.get_mut(&level) {
                    list_of.retain(|r| !parts.contains(r));
                }
                parts.extend(self.write_chassis(b, pl));
                if !parts.is_empty() {
                    self.w.add(format!(
                        "IFCRELAGGREGATES({},$,$,$,{el},{})",
                        string(derived_guid(&b.guid, "parts").as_str()),
                        list(&parts)
                    ));
                }
                contained.entry(level).or_default().push(el);
            }
        }
    }

    /// The tubes of a box's chassis: an `IfcMember` each (posts `.POST.`,
    /// the rest `.MEMBER.`), the tube's hollow profile swept along it in
    /// the box's placement, with `ArchiSim_BoxPart` (its role and tube),
    /// which the importer leaves out: the chassis comes back from the
    /// box's own data.
    fn write_chassis(&mut self, b: &archisim_core::ModularBox, box_pl: Ref) -> Vec<Ref> {
        use archisim_geom::boxes::chassis_members;
        let material = self
            .model
            .frame_member_material(archisim_core::FrameMaterial::Steel);
        let mut out = Vec::new();
        for (n, m) in chassis_members(b).iter().enumerate() {
            let ext = [m.hi[0] - m.lo[0], m.hi[1] - m.lo[1], m.hi[2] - m.lo[2]];
            let c = m.centre();
            // Local z along the tube, local x across it; the profile's
            // x and y dimensions are the extents on those.
            let (start, z, x, xdim, ydim) = match m.axis {
                0 => (
                    [m.lo[0], c[1], c[2]],
                    (1.0, 0.0, 0.0),
                    (0.0, 1.0, 0.0),
                    ext[1],
                    ext[2],
                ),
                1 => (
                    [c[0], m.lo[1], c[2]],
                    (0.0, 1.0, 0.0),
                    (1.0, 0.0, 0.0),
                    ext[0],
                    ext[2],
                ),
                _ => (
                    [c[0], c[1], m.lo[2]],
                    (0.0, 0.0, 1.0),
                    (1.0, 0.0, 0.0),
                    ext[0],
                    ext[1],
                ),
            };
            let wall = m.tube.wall.min(xdim.min(ydim) / 2.0 - 1e-4);
            let profile = self.w.add(format!(
                "IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,{},$,{},{},{},$,$)",
                string(&m.tube.label()),
                real(xdim),
                real(ydim),
                real(wall)
            ));
            let loc = self.point3(start[0], start[1], start[2] - b.bottom);
            let dz = self.w.add(format!(
                "IFCDIRECTION(({},{},{}))",
                real(z.0),
                real(z.1),
                real(z.2)
            ));
            let dx = self.w.add(format!(
                "IFCDIRECTION(({},{},{}))",
                real(x.0),
                real(x.1),
                real(x.2)
            ));
            let axes = self.w.add(format!("IFCAXIS2PLACEMENT3D({loc},{dz},{dx})"));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{axes},{},{})",
                self.z,
                real(m.length())
            ));
            if let Some(style) = material.and_then(|mt| self.styles.get(&mt)).copied() {
                self.w.add(format!("IFCSTYLEDITEM({solid},({style}),$)"));
            }
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let origin = self.point3(0.0, 0.0, 0.0);
            let pl = self.placement(Some(box_pl), origin, None);
            let guid = derived_guid(&b.guid, &format!("chassis {n}"));
            let el = self.w.add(format!(
                "IFCMEMBER({},$,{},$,{},{pl},{shape},$,.{}.)",
                string(guid.as_str()),
                string(&format!("{} {} {}", b.mark, m.role.label(), m.tube.label())),
                string(m.role.label()),
                if m.role.is_post() { "POST" } else { "MEMBER" },
            ));
            let props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Role',$,IFCLABEL({}),$)",
                    string(m.role.code())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Tube',$,IFCLABEL({}),$)",
                    string(&m.tube.label())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE({}),$)",
                    real(m.length())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Weight',$,IFCMASSMEASURE({}),$)",
                    real(m.weight())
                )),
            ];
            self.pset(el, &guid, "ArchiSim_BoxPart", &props);
            out.push(el);
        }
        out
    }
}
