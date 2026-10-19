//! Rooms: `IfcSpace`, aggregated into their storey (`IfcRelAggregates`),
//! extruded from their outline (less their holes) by their clear height,
//! with `Pset_SpaceCommon`, `Pset_SpaceCoveringRequirements` (finishes),
//! `Qto_SpaceBaseQuantities` (net area and perimeter of the outline; gross
//! area to the axes of the walls around it) and `ArchiSim_Space` (where the
//! name is drawn, whether it follows the walls and whether its height is
//! its own, for the round trip). The number is the space's `Name`, its
//! name the `LongName` and its use the `ObjectType`, as Revit and ArchiCAD
//! write them.

use std::collections::BTreeMap;

use archisim_core::{perimeter, LevelId, Point2};
use archisim_geom::{room_outline, signed_area, Contour};

use crate::step::{boolean, list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_spaces(&mut self, storeys: &BTreeMap<LevelId, (Ref, Ref)>) {
        let model = self.model;
        let mut by_level: BTreeMap<LevelId, Vec<Ref>> = BTreeMap::new();
        for room in model.rooms() {
            let Some(&(_, storey_pl)) = storeys.get(&room.level) else {
                continue;
            };
            let height = model.room_height(room);
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let outline = self.polyline(&room.outline, true);
            let profile = if room.holes.is_empty() {
                self.w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"))
            } else {
                let inner: Vec<Ref> = room.holes.iter().map(|h| self.polyline(h, true)).collect();
                self.w.add(format!(
                    "IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,{outline},{})",
                    list(&inner)
                ))
            };
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(height)
            ));
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let usage = if room.usage.trim().is_empty() {
                "$".to_string()
            } else {
                string(&room.usage)
            };
            let el = self.w.add(format!(
                "IFCSPACE({},$,{},$,{usage},{pl},{shape},{},.ELEMENT.,.SPACE.,$)",
                string(room.guid.as_str()),
                string(&room.number),
                string(&room.name)
            ));
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                    string(&room.number)
                )),
                self.w
                    .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$)"),
            ];
            self.pset(el, &room.guid, "Pset_SpaceCommon", &props);
            let f = &room.finishes;
            if !(f.floor.is_empty() && f.walls.is_empty() && f.ceiling.is_empty()) {
                let props: Vec<Ref> = [
                    ("FloorCovering", &f.floor),
                    ("WallCovering", &f.walls),
                    ("CeilingCovering", &f.ceiling),
                ]
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(n, v)| {
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('{n}',$,IFCLABEL({}),$)",
                        string(v)
                    ))
                })
                .collect();
                self.pset(el, &room.guid, "Pset_SpaceCoveringRequirements", &props);
            }
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('LabelX',$,IFCLENGTHMEASURE({}),$)",
                    real(room.label.x)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('LabelY',$,IFCLENGTHMEASURE({}),$)",
                    real(room.label.y)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Bounded',$,IFCBOOLEAN({}),$)",
                    boolean(room.bounded)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('OwnHeight',$,IFCBOOLEAN({}),$)",
                    boolean(room.height.is_some())
                )),
            ];
            self.pset(el, &room.guid, "ArchiSim_Space", &props);
            let net = room.area();
            let gross_outline: Vec<Point2> =
                room_outline(model, room.level, room.label, Contour::WallAxis)
                    .unwrap_or_else(|_| room.outline.clone());
            let gross = signed_area(&gross_outline).abs().max(net);
            self.quantities(
                el,
                &room.guid,
                "Qto_SpaceBaseQuantities",
                &[
                    ("Height", "IFCQUANTITYLENGTH", height),
                    ("NetPerimeter", "IFCQUANTITYLENGTH", room.perimeter()),
                    (
                        "GrossPerimeter",
                        "IFCQUANTITYLENGTH",
                        perimeter(&gross_outline),
                    ),
                    ("NetFloorArea", "IFCQUANTITYAREA", net),
                    ("GrossFloorArea", "IFCQUANTITYAREA", gross),
                    ("NetVolume", "IFCQUANTITYVOLUME", net * height),
                    ("GrossVolume", "IFCQUANTITYVOLUME", gross * height),
                ],
            );
            by_level.entry(room.level).or_default().push(el);
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
