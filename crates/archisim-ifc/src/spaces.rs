//! Rooms: `IfcSpace`, aggregated into their storey (`IfcRelAggregates`),
//! extruded from their outline by their height, with `Pset_SpaceCommon`,
//! `Qto_SpaceBaseQuantities` (net area and perimeter of the outline; gross
//! area to the axes of the walls around it) and `ArchiSim_Space` (where the
//! label is drawn, for the round trip). The number is the space's `Name`,
//! its name the `LongName` and its use the `ObjectType`, as Revit and
//! ArchiCAD write them.

use std::collections::BTreeMap;

use archisim_core::{perimeter, LevelId, Point2};
use archisim_geom::{room_outline, signed_area, Contour};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_spaces(&mut self, storeys: &BTreeMap<LevelId, (Ref, Ref)>) {
        let model = self.model;
        let mut by_level: BTreeMap<LevelId, Vec<Ref>> = BTreeMap::new();
        for sp in model.spaces() {
            let Some(&(_, storey_pl)) = storeys.get(&sp.level) else {
                continue;
            };
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let outline = self.polyline(&sp.outline, true);
            let profile = self
                .w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"));
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(sp.height)
            ));
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let usage = if sp.usage.trim().is_empty() {
                "$".to_string()
            } else {
                string(&sp.usage)
            };
            let el = self.w.add(format!(
                "IFCSPACE({},$,{},$,{usage},{pl},{shape},{},.ELEMENT.,.SPACE.,$)",
                string(sp.guid.as_str()),
                string(&sp.number),
                string(&sp.name)
            ));
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                    string(&sp.number)
                )),
                self.w
                    .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$)"),
            ];
            self.pset(el, &sp.guid, "Pset_SpaceCommon", &props);
            if let Some(p) = sp.label {
                let props = [
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('LabelX',$,IFCLENGTHMEASURE({}),$)",
                        real(p.x)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('LabelY',$,IFCLENGTHMEASURE({}),$)",
                        real(p.y)
                    )),
                ];
                self.pset(el, &sp.guid, "ArchiSim_Space", &props);
            }
            let net = sp.area();
            let gross_outline: Vec<Point2> =
                room_outline(model, sp.level, sp.label_point(), Contour::WallAxis)
                    .unwrap_or_else(|_| sp.outline.clone());
            let gross = signed_area(&gross_outline).abs().max(net);
            let perim = sp.perimeter();
            self.quantities(
                el,
                &sp.guid,
                "Qto_SpaceBaseQuantities",
                &[
                    ("Height", "IFCQUANTITYLENGTH", sp.height),
                    ("NetPerimeter", "IFCQUANTITYLENGTH", perim),
                    (
                        "GrossPerimeter",
                        "IFCQUANTITYLENGTH",
                        perimeter(&gross_outline),
                    ),
                    ("NetFloorArea", "IFCQUANTITYAREA", net),
                    ("GrossFloorArea", "IFCQUANTITYAREA", gross),
                    ("NetVolume", "IFCQUANTITYVOLUME", net * sp.height),
                    ("GrossVolume", "IFCQUANTITYVOLUME", gross * sp.height),
                ],
            );
            by_level.entry(sp.level).or_default().push(el);
        }
        for (level, spaces) in &by_level {
            let (storey, _) = storeys[level];
            let guid = model
                .level(*level)
                .map(|l| l.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELAGGREGATES({},$,$,$,{storey},{})",
                string(derived_guid(&guid, "spaces").as_str()),
                list(spaces)
            ));
        }
    }
}
