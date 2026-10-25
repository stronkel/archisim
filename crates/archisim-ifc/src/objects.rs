//! Objects: furniture (`IfcFurniture`), sanitary ware
//! (`IfcSanitaryTerminal`), vegetation (`IfcGeographicElement`, `VEGETATION`
//! in IFC 4.3), vehicles (`IfcVehicle` in IFC 4.3) and people
//! (`IfcBuildingElementProxy`) with their types, as the
//! family's `ifc_class` and `ifc_type` outputs say. The body is the family's boxes; the type
//! carries the family (`ArchiSim_Family`, `ArchiSim_FamilyValues`) so that
//! ArchiSim reads them back as the same objects.
//!
//! A type that shows a 3D model brought from another program carries it
//! once, as its representation map (`IfcTriangulatedFaceSet` per material,
//! with the colours of its faces), and every object of it maps it.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId, ObjectTypeId};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

/// The IFC class an object is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Furniture,
    Sanitary,
    /// Vegetation and the like (`IfcGeographicElement`).
    Geographic,
    /// People… (`IfcBuildingElementProxy`).
    Proxy,
    /// Cars, bicycles… (`IfcVehicle`, IFC 4.3; a proxy in IFC4).
    Vehicle,
}

impl Class {
    /// Entity names of the element and of its type.
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Class::Furniture => ("IFCFURNITURE", "IFCFURNITURETYPE"),
            Class::Sanitary => ("IFCSANITARYTERMINAL", "IFCSANITARYTERMINALTYPE"),
            Class::Geographic => ("IFCGEOGRAPHICELEMENT", "IFCGEOGRAPHICELEMENTTYPE"),
            Class::Proxy => ("IFCBUILDINGELEMENTPROXY", "IFCBUILDINGELEMENTPROXYTYPE"),
            Class::Vehicle => ("IFCVEHICLE", "IFCVEHICLETYPE"),
        }
    }

    /// The property set its status (phase) goes in.
    fn status_pset(self) -> &'static str {
        match self {
            Class::Furniture => "Pset_FurnitureTypeCommon",
            Class::Sanitary => "Pset_SanitaryTerminalTypeCommon",
            Class::Geographic => "ArchiSim_Phase",
            Class::Proxy => "Pset_BuildingElementProxyCommon",
            Class::Vehicle => "ArchiSim_Phase",
        }
    }
}

/// IFC class and predefined type of an object family's outputs, in a
/// schema: IFC4 has neither vehicles nor vegetation, so they go as proxies
/// and user-defined geographic elements.
fn ifc_kind(outputs: &archisim_core::Values, schema: crate::Schema) -> (Class, String) {
    let class = outputs
        .get("ifc_class")
        .and_then(|v| v.as_text())
        .unwrap_or("IfcFurniture")
        .to_ascii_lowercase();
    let class = match class.as_str() {
        "ifcsanitaryterminal" => Class::Sanitary,
        "ifcgeographicelement" => Class::Geographic,
        "ifcbuildingelementproxy" => Class::Proxy,
        "ifcvehicle" if schema == crate::Schema::Ifc4x3 => Class::Vehicle,
        "ifcvehicle" => Class::Proxy,
        _ => Class::Furniture,
    };
    let allowed: &[&str] = match class {
        Class::Sanitary => &[
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
        ],
        Class::Furniture => &[
            "CHAIR",
            "TABLE",
            "DESK",
            "BED",
            "FILECABINET",
            "SHELF",
            "SOFA",
        ],
        Class::Geographic => match schema {
            crate::Schema::Ifc4x3 => &["TERRAIN", "VEGETATION", "SOIL_BORING_POINT"],
            crate::Schema::Ifc4 => &["TERRAIN"],
        },
        Class::Proxy => &["COMPLEX", "ELEMENT", "PARTIAL"],
        Class::Vehicle => &[
            "VEHICLE",
            "VEHICLEWHEELED",
            "VEHICLETRACKED",
            "CARGO",
            "ROLLINGSTOCK",
        ],
    };
    let t = outputs
        .get("ifc_type")
        .and_then(|v| v.as_text())
        .map(|t| t.to_uppercase())
        .filter(|t| allowed.contains(&t.as_str()))
        .unwrap_or_else(|| "USERDEFINED".into());
    (class, t)
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
        // Types with a 3D model: its representation map, whole and, for
        // base modules under a worktop run, without their own worktop.
        let mut mapped: BTreeMap<(ObjectTypeId, bool), Ref> = BTreeMap::new();
        let under_run: std::collections::BTreeSet<archisim_core::ObjectId> = model
            .worktop_runs()
            .into_iter()
            .flat_map(|r| r.members)
            .collect();
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
            let (class, predefined) = ifc_kind(&ev.outputs, self.schema);
            let (element_entity, type_entity) = class.names();
            // The type, once.
            if let std::collections::btree_map::Entry::Vacant(slot) = typed.entry(t.id) {
                let mut props = Vec::new();
                if let Some(family) = model.family(t.family) {
                    let (a, b) = self.family_psets(&t.guid, family, &t.values);
                    props.push(a);
                    props.extend(b);
                    let shared = self.shared_type_psets(&t.guid, family, &t.values);
                    props.extend(shared);
                }
                let element_type = model
                    .family(t.family)
                    .map_or("$".to_string(), |f| string(f.name()));
                // Furniture types also say where they are assembled.
                let assembly = if class == Class::Furniture {
                    ".NOTDEFINED.,"
                } else {
                    ""
                };
                // Which of its two models its objects need.
                let mut needs = [false, false];
                for x in model.objects().filter(|x| x.object_type == t.id) {
                    needs[usize::from(under_run.contains(&x.id))] = true;
                }
                let mut maps_of_type = Vec::new();
                for bare in [false, true] {
                    if needs[usize::from(bare)] {
                        if let Some(m) = self.mesh_map(o.id, bare) {
                            mapped.insert((t.id, bare), m);
                            maps_of_type.push(m);
                        }
                    }
                }
                let maps = if maps_of_type.is_empty() {
                    "$".to_string()
                } else {
                    list(&maps_of_type)
                };
                let ty = self.w.add(format!(
                    "{type_entity}({},$,{},$,$,{},{maps},$,{element_type},{assembly}.{predefined}.)",
                    string(t.guid.as_str()),
                    string(&t.name),
                    list(&props)
                ));
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
            let plant = self.plant_body(o.id);
            let rep = if let Some(rep) = plant {
                rep
            } else if let Some(&map) = mapped.get(&(t.id, under_run.contains(&o.id))) {
                let op = self.w.add(format!(
                    "IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,{},$,$)",
                    self.origin
                ));
                let item = self.w.add(format!("IFCMAPPEDITEM({map},{op})"));
                self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','MappedRepresentation',({item}))",
                    self.body_ctx
                ))
            } else {
                let items: Vec<Ref> = ev
                    .parts
                    .iter()
                    .filter_map(|p| self.box_solid(p.min, p.max, MaterialId(p.material)))
                    .collect();
                if items.is_empty() {
                    continue;
                }
                self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',{})",
                    self.body_ctx,
                    list(&items)
                ))
            };
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let name = string(&t.name);
            // ObjectType: the family (required when the type is
            // USERDEFINED).
            let object_type = model
                .family(t.family)
                .map_or("$".to_string(), |f| string(f.name()));
            let el = self.w.add(format!(
                "{element_entity}({},$,{name},$,{object_type},{pl},{shape},$,.{predefined}.)",
                string(o.guid.as_str()),
            ));
            let first_material = match (model.plant_look(o.id), model.object_mesh(o.id)) {
                (Some(l), _) => Some(l.materials.leaves),
                (None, Some((_, _, level))) => level.groups.first().map(|g| g.material),
                (None, None) => ev.parts.first().map(|p| MaterialId(p.material)),
            };
            if let Some(m) = first_material.and_then(|m| materials.get(&m)) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&o.guid, "material").as_str())
                ));
            }
            // Its own values of the family's instance parameters.
            if !o.values.is_empty() {
                if let Some(family) = model.family(t.family) {
                    let values = self.value_properties(family, &o.values);
                    if !values.is_empty() {
                        self.pset(el, &o.guid, "ArchiSim_FamilyValues", &values);
                    }
                }
            }
            let status: Vec<Ref> = self.status_prop(o.id.0).into_iter().collect();
            if !status.is_empty() {
                self.pset(el, &o.guid, class.status_pset(), &status);
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

/// Triangles of a model that go to the IFC at most: the heaviest version
/// under this many.
const IFC_TRIANGLES: usize = 30_000;

impl Exporter<'_> {
    /// The representation map of the 3D model of `object`'s type, in the
    /// object's own axes and at its type's size: one triangulated face set
    /// per material, styled with it and, if its faces differ in colour (a
    /// texture), with the colour of each face.
    fn mesh_map(&mut self, object: archisim_core::ObjectId, bare: bool) -> Option<Ref> {
        let model = self.model;
        let items = match model.object_mesh(object) {
            Some((mesh, k, _)) => {
                let level = mesh.level_under(IFC_TRIANGLES)?;
                let groups: Vec<(MaterialId, &[[u32; 3]])> = level
                    .groups
                    .iter()
                    .map(|g| (g.material, g.triangles.as_slice()))
                    .collect();
                self.tessellation(&level.positions, &level.colors, &groups, k)
            }
            None => {
                // A model ArchiSim makes from the type's measurements
                // (without its worktop: the run's goes on its own).
                let t = model.object(object)?.object_type;
                let skip: &[&str] = if bare { &["top"] } else { &[] };
                let (mesh, parts) = model.type_prop_without(t, skip)?;
                let groups: Vec<(MaterialId, &[[u32; 3]])> =
                    parts.iter().map(|(m, t)| (*m, t.as_slice())).collect();
                // One colour per material: no colour list per face.
                let colors: Vec<archisim_core::Rgb> = vec![[0, 0, 0]; mesh.positions.len()];
                self.tessellation(&mesh.positions, &colors, &groups, 1.0)
            }
        };
        if items.is_empty() {
            return None;
        }
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',{})",
            self.body_ctx,
            list(&items)
        ));
        let origin = self
            .w
            .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
        Some(self.w.add(format!("IFCREPRESENTATIONMAP({origin},{rep})")))
    }

    /// Triangulated face sets of a mesh: one per material, with the
    /// vertices it uses (scaled by `k`), styled with its material and, if
    /// its faces differ in colour, with the colour of each face.
    pub(crate) fn tessellation(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[u8; 3]],
        groups: &[(MaterialId, &[[u32; 3]])],
        k: f64,
    ) -> Vec<Ref> {
        let mut items = Vec::new();
        for &(material, triangles) in groups {
            if triangles.is_empty() {
                continue;
            }
            // Only the vertices this material uses, renumbered.
            let mut index = std::collections::HashMap::new();
            let mut pts = Vec::new();
            let mut faces = Vec::new();
            for t in triangles {
                let f = t.map(|i| {
                    let n = index.len() + 1;
                    *index.entry(i).or_insert_with(|| {
                        let p = positions[i as usize];
                        pts.push(format!(
                            "({},{},{})",
                            real(p[0] as f64 * k),
                            real(p[1] as f64 * k),
                            real(p[2] as f64 * k)
                        ));
                        n
                    })
                });
                faces.push(format!("({},{},{})", f[0], f[1], f[2]));
            }
            let list3 = self
                .w
                .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
            let body = self.w.add(format!(
                "IFCTRIANGULATEDFACESET({list3},$,$,({}),$)",
                faces.join(",")
            ));
            if let Some(&style) = self.styles.get(&material) {
                self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
            }
            // The colour of each face (the average of its corners), on a
            // short palette.
            let mut palette: Vec<[u8; 3]> = Vec::new();
            let mut which = Vec::new();
            for t in triangles {
                let c = [0, 1, 2].map(|k| {
                    let s: u32 = t.iter().map(|&i| colors[i as usize][k] as u32).sum();
                    ((s / 3) as u8) & 0xF8
                });
                let n = match palette.iter().position(|p| *p == c) {
                    Some(n) => n,
                    None => {
                        palette.push(c);
                        palette.len() - 1
                    }
                };
                which.push((n + 1).to_string());
            }
            if palette.len() > 1 {
                let colours: Vec<String> = palette
                    .iter()
                    .map(|c| {
                        format!(
                            "({},{},{})",
                            real(c[0] as f64 / 255.0),
                            real(c[1] as f64 / 255.0),
                            real(c[2] as f64 / 255.0)
                        )
                    })
                    .collect();
                let list = self
                    .w
                    .add(format!("IFCCOLOURRGBLIST(({}))", colours.join(",")));
                self.w.add(format!(
                    "IFCINDEXEDCOLOURMAP({body},$,{list},({}))",
                    which.join(",")
                ));
            }
            items.push(body);
        }
        items
    }

    /// The body of a plant grown by ArchiSim: its own triangles (every
    /// plant is different), at a level of detail light enough for the
    /// number of plants in the project.
    fn plant_body(&mut self, object: archisim_core::ObjectId) -> Option<Ref> {
        let model = self.model;
        let plants = model
            .objects()
            .filter(|o| model.type_grows(o.object_type))
            .count();
        let lod = if plants <= 20 {
            archisim_core::flora::Lod::Full
        } else if plants <= 100 {
            archisim_core::flora::Lod::Medium
        } else {
            archisim_core::flora::Lod::Low
        };
        let (mesh, parts) = model.plant_mesh(object, lod)?;
        let groups: Vec<(MaterialId, &[[u32; 3]])> =
            parts.iter().map(|(m, t)| (*m, t.as_slice())).collect();
        let items = self.tessellation(&mesh.positions, &mesh.colors, &groups, 1.0);
        if items.is_empty() {
            return None;
        }
        Some(self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',{})",
            self.body_ctx,
            list(&items)
        )))
    }
}

impl Exporter<'_> {
    /// The worktop of each run of base modules (`docs/MAQUETA.md` §6
    /// quinquies): an `IfcFurniture` («Encimera») over its modules, with
    /// the openings of its sinks, its length and area. It carries
    /// `ArchiSim_Worktop`, so ArchiSim does not read it back as an object
    /// of its own: it comes back from its modules.
    pub(crate) fn write_worktops(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        // One type per material of worktop, with the worktops of it.
        let mut types: BTreeMap<MaterialId, Vec<Ref>> = BTreeMap::new();
        for run in model.worktop_runs() {
            let Some(&(_, storey_pl)) = storeys.get(&run.level) else {
                continue;
            };
            let Some(first) = model.object(run.members[0]) else {
                continue;
            };
            let guid = derived_guid(&first.guid, "worktop");
            let storey_z = model.level(run.level).map_or(0.0, |l| l.elevation);
            let solids = archisim_geom::solid::worktop_solids(
                model,
                &run,
                archisim_geom::solid::SolidOwner::Object(first.id),
            );
            let refs: Vec<&archisim_geom::solid::Solid> = solids.iter().collect();
            let Some(shape) = self.solids_shape(&refs, storey_z) else {
                continue;
            };
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let el = self.w.add(format!(
                "IFCFURNITURE({},$,'Encimera',$,'Encimera',{pl},{shape},$,.USERDEFINED.)",
                string(guid.as_str()),
            ));
            self.material_of(materials, el, &guid, run.material);
            let members: Vec<String> = run
                .members
                .iter()
                .filter_map(|m| model.object(*m))
                .map(|o| o.guid.as_str().to_string())
                .collect();
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE({}),$)",
                    real(run.length)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Area',$,IFCAREAMEASURE({}),$)",
                    real(run.area)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Modules',$,IFCTEXT({}),$)",
                    string(&members.join(" "))
                )),
            ];
            self.pset(el, &guid, "ArchiSim_Worktop", &props);
            contained.entry(run.level).or_default().push(el);
            types.entry(run.material).or_default().push(el);
        }
        for (m, users) in types {
            let name = model
                .material(m)
                .map_or("Encimera".to_string(), |x| format!("Encimera · {}", x.name));
            let guid = derived_guid(&model.project_guid, &format!("worktop-type-{}", m.0));
            let ty = self.w.add(format!(
                "IFCFURNITURETYPE({},$,{},$,$,$,$,$,'Encimera',.NOTDEFINED.,.USERDEFINED.)",
                string(guid.as_str()),
                string(&name)
            ));
            self.material_of(materials, ty, &guid, m);
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{ty})",
                string(derived_guid(&guid, "defines").as_str()),
                list(&users)
            ));
        }
    }
}
