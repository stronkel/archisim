//! The members of the light frames (steel frame and timber frame): on
//! each wall, floor and roof whose type is framed, one `IfcMember` per
//! stud, track, header, joist, rafter or tie, with its body as triangles
//! (the C or U of thin plates, or the timber), its material, its kind
//! (`STUD`, `PLATE`, `RAFTER`…) and `ArchiSim_FramePart` (what it is, its
//! section, length and prefabricated panel), part of its element
//! (`IfcRelAggregates`). A file read back drops them: the frame is laid
//! again from the type.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId};
use archisim_geom::framing::{all_members, member_triangles, Role};
use archisim_geom::solid::SolidOwner;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

/// The kind of `IfcMember` of each role (in IFC4 and IFC 4.3 alike).
fn member_kind(role: Role) -> &'static str {
    match role {
        Role::Stud | Role::KingStud | Role::JackStud | Role::Cripple => "STUD",
        Role::BottomTrack
        | Role::TopTrack
        | Role::TopPlate
        | Role::SillTrack
        | Role::Rim
        | Role::RoofRim => "PLATE",
        Role::Noggin | Role::Blocking => "STRUT",
        Role::Rafter => "RAFTER",
        Role::Tie => "CHORD",
        Role::Header | Role::Joist => "MEMBER",
    }
}

impl Exporter<'_> {
    pub(crate) fn write_frame_members(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        let members = all_members(model);
        if members.is_empty() {
            return;
        }
        let mut by_host: BTreeMap<SolidOwner, Vec<usize>> = BTreeMap::new();
        for (i, m) in members.iter().enumerate() {
            by_host.entry(m.owner).or_default().push(i);
        }
        for (owner, list) in by_host {
            let (guid, level) = match owner {
                SolidOwner::Wall(id) => match model.wall(id) {
                    Some(w) => (w.guid.clone(), w.level),
                    None => continue,
                },
                SolidOwner::Slab(id) => match model.slab(id) {
                    Some(s) => (s.guid.clone(), s.level),
                    None => continue,
                },
                SolidOwner::Roof(id) => match model.roof(id) {
                    Some(r) => (r.guid.clone(), r.level),
                    None => continue,
                },
                _ => continue,
            };
            let Some(&host) = self.hosts.get(&owner.id()) else {
                continue;
            };
            let Some(&(_, storey_pl)) = storeys.get(&level) else {
                continue;
            };
            let storey_z = model.level(level).map_or(0.0, |l| l.elevation);
            let mut parts = Vec::new();
            for (n, i) in list.into_iter().enumerate() {
                let m = &members[i];
                let mut pts: Vec<String> = Vec::new();
                let mut index: BTreeMap<[i64; 3], usize> = BTreeMap::new();
                let mut faces = Vec::new();
                for t in member_triangles(m) {
                    let mut f = [0usize; 3];
                    for (k, p) in t.iter().enumerate() {
                        let key = [
                            (p[0] * 1e5).round() as i64,
                            (p[1] * 1e5).round() as i64,
                            (p[2] * 1e5).round() as i64,
                        ];
                        f[k] = *index.entry(key).or_insert_with(|| {
                            pts.push(format!(
                                "({},{},{})",
                                real(p[0]),
                                real(p[1]),
                                real(p[2] - storey_z)
                            ));
                            pts.len()
                        });
                    }
                    if f[0] != f[1] && f[1] != f[2] && f[0] != f[2] {
                        faces.push(format!("({},{},{})", f[0], f[1], f[2]));
                    }
                }
                if faces.is_empty() {
                    continue;
                }
                let guid = derived_guid(&guid, &format!("frame member {n}"));
                let list3 = self
                    .w
                    .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
                let body = self.w.add(format!(
                    "IFCTRIANGULATEDFACESET({list3},$,$,({}),$)",
                    faces.join(",")
                ));
                let material = model.frame_member_material(m.material);
                if let Some(style) = material.and_then(|mt| self.styles.get(&mt)).copied() {
                    self.w.add(format!("IFCSTYLEDITEM({body},({style}),$)"));
                }
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({body}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let name = m.role.name(m.material);
                let el = self.w.add(format!(
                    "IFCMEMBER({},$,{},$,{},{pl},{shape},$,.{}.)",
                    string(guid.as_str()),
                    string(&format!("{name} {}", m.section.name)),
                    string(&m.section.name),
                    member_kind(m.role),
                ));
                if let Some(r) = material.and_then(|mt| materials.get(&mt)) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{r})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
                let mut props = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Role',$,IFCLABEL({}),$)",
                        string(name)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Section',$,IFCLABEL({}),$)",
                        string(&m.section.name)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE({}),$)",
                        real(m.length())
                    )),
                ];
                if let Some(p) = m.panel {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Panel',$,IFCINTEGER({}),$)",
                        p % 1000 + 1
                    )));
                }
                self.pset(el, &guid, "ArchiSim_FramePart", &props);
                parts.push(el);
            }
            // An element is made of one set of parts only (a wall of its
            // façade's, a slab of its bars', a roof of its planes'): with
            // another, the members are in the storey, next to it.
            let taken = match owner {
                SolidOwner::Wall(id) => model.wall_facade(id).is_some(),
                SolidOwner::Slab(id) => model.reinforcement(id.0).is_some(),
                _ => true,
            };
            if taken {
                contained.entry(level).or_default().extend(parts);
            } else if !parts.is_empty() {
                let list: Vec<String> = parts.iter().map(|r| r.to_string()).collect();
                self.w.add(format!(
                    "IFCRELAGGREGATES({},$,'Entramado',$,{host},({}))",
                    string(derived_guid(&guid, "frame members").as_str()),
                    list.join(",")
                ));
            }
        }
    }
}
