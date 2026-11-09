//! False ceilings (M7): an `IfcCovering` `.CEILING.` in the storey of its
//! level, its type an `IfcCoveringType` `.CEILING.` with the layers of its
//! slab type (written from the bottom up, as slabs). Its body is its
//! outline (with its holes as voids of the profile) placed at its face and
//! extruded upwards by the thickness of its type, so ArchiSim reads it back
//! from its placement and its profile alone.

use std::collections::BTreeMap;

use archisim_core::{LevelId, Point2, SlabTypeId};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_ceilings(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        slab_sets: &BTreeMap<SlabTypeId, Ref>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
        by_type: &mut BTreeMap<SlabTypeId, Vec<Ref>>,
    ) {
        let model = self.model;
        for c in model.ceilings() {
            let (Some(ctype), Some(&(_, storey_pl)), Some(&set)) = (
                model.slab_type(c.ceiling_type),
                storeys.get(&c.level),
                slab_sets.get(&c.ceiling_type),
            ) else {
                continue;
            };
            let thickness = ctype.thickness();
            let loc = self.point3(0.0, 0.0, c.height);
            let pl = self.placement(Some(storey_pl), loc, None);
            let profile = self.ceiling_profile(&c.outline, &c.holes);
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(thickness)
            ));
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let name = format!("{} {}", ctype.name, c.id.0);
            let el = self.w.add(format!(
                "IFCCOVERING({},$,{},$,{},{pl},{shape},{},.CEILING.)",
                string(c.guid.as_str()),
                string(&name),
                string(&ctype.name),
                string(&c.id.0.to_string()),
            ));
            let usage = self.w.add(format!(
                "IFCMATERIALLAYERSETUSAGE({set},.AXIS3.,.POSITIVE.,0.,$)"
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{usage})",
                string(derived_guid(&c.guid, "material").as_str())
            ));
            let status = self.status_prop(c.id.0);
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                    string(&ctype.name)
                )),
                self.w
                    .add("IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$)"),
            ];
            props.extend(status);
            self.pset(el, &c.guid, "Pset_CoveringCommon", &props);
            let area = c.area();
            let gross = archisim_core::signed_area(&c.outline).abs();
            self.quantities(
                el,
                &c.guid,
                "Qto_CoveringBaseQuantities",
                &[
                    ("Width", "IFCQUANTITYLENGTH", thickness),
                    ("GrossArea", "IFCQUANTITYAREA", gross),
                    ("NetArea", "IFCQUANTITYAREA", area),
                ],
            );
            self.hosts.insert(c.id.0, el);
            contained.entry(c.level).or_default().push(el);
            by_type.entry(c.ceiling_type).or_default().push(el);
        }
    }

    fn ceiling_profile(&mut self, outline: &[Point2], holes: &[Vec<Point2>]) -> Ref {
        let outer = self.polyline(outline, true);
        if holes.is_empty() {
            return self
                .w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outer})"));
        }
        let inner: Vec<Ref> = holes.iter().map(|h| self.polyline(h, true)).collect();
        self.w.add(format!(
            "IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,{outer},{})",
            list(&inner)
        ))
    }
}
