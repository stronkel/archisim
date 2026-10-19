//! Objects: furniture (`IfcFurniture`) and sanitary ware
//! (`IfcSanitaryTerminal`) with their types, as the family's `ifc_class`
//! and `ifc_type` outputs say. The body is the family's boxes; the type
//! carries the family (`ArchiSim_Family`, `ArchiSim_FamilyValues`) so that
//! ArchiSim reads them back as the same objects.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId, ObjectTypeId};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

/// IFC class and predefined type of an object family's outputs.
fn ifc_kind(outputs: &archisim_core::Values) -> (bool, String) {
    let class = outputs
        .get("ifc_class")
        .and_then(|v| v.as_text())
        .unwrap_or("IfcFurniture");
    let sanitary = class.eq_ignore_ascii_case("IfcSanitaryTerminal");
    let allowed: &[&str] = if sanitary {
        &[
            "BATH",
            "BIDET",
            "CISTERN",
            "SHOWER",
            "SINK",
            "SANITARYFOUNTAIN",
            "TOILETPAN",
            "URINAL",
            "WASHHANDBASIN",
            "WCSEAT",
        ]
    } else {
        &[
            "CHAIR",
            "TABLE",
            "DESK",
            "BED",
            "FILECABINET",
            "SHELF",
            "SOFA",
        ]
    };
    let t = outputs
        .get("ifc_type")
        .and_then(|v| v.as_text())
        .map(|t| t.to_uppercase())
        .filter(|t| allowed.contains(&t.as_str()))
        .unwrap_or_else(|| "USERDEFINED".into());
    (sanitary, t)
}

impl Exporter<'_> {
    pub(crate) fn write_objects(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        let mut typed: BTreeMap<ObjectTypeId, (Ref, Vec<Ref>)> = BTreeMap::new();
        for o in model.objects() {
            let Some(&(_, storey_pl)) = storeys.get(&o.level) else {
                continue;
            };
            let Some(t) = model.object_type(o.object_type) else {
                continue;
            };
            let Ok(ev) = model.evaluate_object(o.id) else {
                continue;
            };
            let (sanitary, predefined) = ifc_kind(&ev.outputs);
            // The type, once.
            if let std::collections::btree_map::Entry::Vacant(slot) = typed.entry(t.id) {
                let mut props = Vec::new();
                if let Some(family) = model.family(t.family) {
                    let (a, b) = self.family_psets(&t.guid, family, &t.values);
                    props.push(a);
                    props.extend(b);
                }
                let element_type = model
                    .family(t.family)
                    .map_or("$".to_string(), |f| string(f.name()));
                let ty = if sanitary {
                    self.w.add(format!(
                        "IFCSANITARYTERMINALTYPE({},$,{},$,$,{},$,$,{element_type},.{predefined}.)",
                        string(t.guid.as_str()),
                        string(&t.name),
                        list(&props)
                    ))
                } else {
                    self.w.add(format!(
                        "IFCFURNITURETYPE({},$,{},$,$,{},$,$,{element_type},.NOTDEFINED.,.{predefined}.)",
                        string(t.guid.as_str()),
                        string(&t.name),
                        list(&props)
                    ))
                };
                slot.insert((ty, Vec::new()));
            }
            // Placed at its point, turned, at its height on the storey.
            let at = self.point3(o.position.x, o.position.y, o.base_offset);
            let x = self.w.add(format!(
                "IFCDIRECTION(({},{},0.))",
                real(o.rotation.cos()),
                real(o.rotation.sin())
            ));
            let pl = self.placement(Some(storey_pl), at, Some(x));
            let items: Vec<Ref> = ev
                .parts
                .iter()
                .filter_map(|p| self.box_solid(p.min, p.max, MaterialId(p.material)))
                .collect();
            if items.is_empty() {
                continue;
            }
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',{})",
                self.body_ctx,
                list(&items)
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let name = string(&t.name);
            // ObjectType: the family (required when the type is
            // USERDEFINED).
            let object_type = model
                .family(t.family)
                .map_or("$".to_string(), |f| string(f.name()));
            let el = if sanitary {
                self.w.add(format!(
                    "IFCSANITARYTERMINAL({},$,{name},$,{object_type},{pl},{shape},$,.{predefined}.)",
                    string(o.guid.as_str()),
                ))
            } else {
                self.w.add(format!(
                    "IFCFURNITURE({},$,{name},$,{object_type},{pl},{shape},$,.{predefined}.)",
                    string(o.guid.as_str()),
                ))
            };
            if let Some(m) = ev
                .parts
                .first()
                .and_then(|p| materials.get(&MaterialId(p.material)))
            {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&o.guid, "material").as_str())
                ));
            }
            let status: Vec<Ref> = self.status_prop(o.id.0).into_iter().collect();
            if !status.is_empty() {
                let pset = if sanitary {
                    "Pset_SanitaryTerminalTypeCommon"
                } else {
                    "Pset_FurnitureTypeCommon"
                };
                self.pset(el, &o.guid, pset, &status);
            }
            if let Some((_, users)) = typed.get_mut(&t.id) {
                users.push(el);
            }
            contained.entry(o.level).or_default().push(el);
        }
        for (tid, (ty, users)) in typed {
            if users.is_empty() {
                continue;
            }
            let guid = model
                .object_type(tid)
                .map(|t| derived_guid(&t.guid, "defines"))
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{ty})",
                string(guid.as_str()),
                list(&users)
            ));
        }
    }
}
