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
                for (i, side) in b.sides.iter().enumerate() {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Side{}',$,IFCLABEL({}),$)",
                        SIDE_NAMES[i],
                        string(side.code())
                    )));
                }
                self.pset(el, &b.guid, "ArchiSim_Box", &props);

                // Its parts leave the storey for it.
                let parts: Vec<Ref> = members
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
}
