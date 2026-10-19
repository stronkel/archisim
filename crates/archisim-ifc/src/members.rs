//! Columns and beams: `IfcColumnType` / `IfcBeamType` with an
//! `IfcMaterialProfileSet`, and `IfcColumn` / `IfcBeam` extruded from the
//! type's profile, with `Pset_ColumnCommon` / `Pset_BeamCommon` and base
//! quantities.
//!
//! Columns are placed at the centre of their base, with local X along the
//! section's width (the column's rotation), and extruded up. Beams are
//! placed at the start of their axis, at mid-depth, with local X along the
//! beam; the solid's position turns the extrusion to that axis, so the
//! profile's X lies across the beam (to its left) and its Y points up.
//! The profile is the type's own, shared by every member of the type.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId, MemberTypeId, MemberUsage, Profile};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    fn profile_def(&mut self, profile: Profile, name: &str) -> Ref {
        match profile {
            Profile::Rectangle { width, depth } => self.w.add(format!(
                "IFCRECTANGLEPROFILEDEF(.AREA.,{},$,{},{})",
                string(name),
                real(width),
                real(depth)
            )),
            Profile::Circle { diameter } => self.w.add(format!(
                "IFCCIRCLEPROFILEDEF(.AREA.,{},$,{})",
                string(name),
                real(diameter / 2.0)
            )),
            Profile::IShape {
                width,
                depth,
                web,
                flange,
            } => self.w.add(format!(
                "IFCISHAPEPROFILEDEF(.AREA.,{},$,{},{},{},{},$,$,$)",
                string(name),
                real(width),
                real(depth),
                real(web),
                real(flange)
            )),
            Profile::UShape {
                width,
                depth,
                web,
                flange,
            } => self.w.add(format!(
                "IFCUSHAPEPROFILEDEF(.AREA.,{},$,{},{},{},{},$,$,$)",
                string(name),
                real(depth),
                real(width),
                real(web),
                real(flange)
            )),
            Profile::TShape {
                width,
                depth,
                web,
                flange,
            } => self.w.add(format!(
                "IFCTSHAPEPROFILEDEF(.AREA.,{},$,{},{},{},{},$,$,$,$,$)",
                string(name),
                real(depth),
                real(width),
                real(web),
                real(flange)
            )),
            Profile::LShape {
                width,
                depth,
                thickness,
            } => self.w.add(format!(
                "IFCLSHAPEPROFILEDEF(.AREA.,{},$,{},{},{},$,$,$)",
                string(name),
                real(depth),
                real(width),
                real(thickness)
            )),
        }
    }

    /// Writes member types, columns and beams; adds them to the storeys.
    pub(crate) fn write_members(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        // Types, each with its profile set (the profile is shared).
        let mut types: BTreeMap<MemberTypeId, (Ref, Ref, Ref)> = BTreeMap::new();
        for t in model.member_types() {
            let Some(&mat) = materials.get(&t.material) else {
                continue;
            };
            let profile = self.profile_def(t.profile, &t.name);
            let mp = self.w.add(format!(
                "IFCMATERIALPROFILE({},$,{mat},{profile},$,$)",
                string(&t.name)
            ));
            let set = self.w.add(format!(
                "IFCMATERIALPROFILESET({},$,({mp}),$)",
                string(&t.name)
            ));
            let entity = match t.usage {
                MemberUsage::Column => "IFCCOLUMNTYPE",
                MemberUsage::Beam => "IFCBEAMTYPE",
            };
            let predefined = match t.usage {
                MemberUsage::Column => "COLUMN",
                MemberUsage::Beam => "BEAM",
            };
            let ty = self.w.add(format!(
                "{entity}({},$,{},$,$,$,$,$,$,.{predefined}.)",
                string(t.guid.as_str()),
                string(&t.name)
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({ty}),{set})",
                string(derived_guid(&t.guid, "material").as_str())
            ));
            types.insert(t.id, (ty, set, profile));
        }

        let mut by_type: BTreeMap<MemberTypeId, Vec<Ref>> = BTreeMap::new();
        for c in model.columns() {
            let (Some(t), Some(&(_, set, profile)), Some(&(_, storey_pl))) = (
                model.member_type(c.member_type),
                types.get(&c.member_type),
                storeys.get(&c.level),
            ) else {
                continue;
            };
            let loc = self.point3(c.position.x, c.position.y, c.base_offset);
            let xdir = self.w.add(format!(
                "IFCDIRECTION(({},{},0.))",
                real(c.rotation.cos()),
                real(c.rotation.sin())
            ));
            let pl = self.placement(Some(storey_pl), loc, Some(xdir));
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(c.height)
            ));
            let el = self.member_element(
                "IFCCOLUMN",
                "COLUMN",
                &c.guid,
                &format!("{} {}", t.name, c.id.0),
                &t.name,
                c.id.0,
                pl,
                solid,
                set,
            );
            let area = t.profile.area();
            self.member_psets(
                el,
                c.id.0,
                &c.guid,
                "Pset_ColumnCommon",
                "Qto_ColumnBaseQuantities",
                &t.name,
                c.height,
                area,
            );
            contained.entry(c.level).or_default().push(el);
            by_type.entry(c.member_type).or_default().push(el);
        }

        for b in model.beams() {
            let (Some(t), Some(&(_, set, profile)), Some(&(_, storey_pl))) = (
                model.member_type(b.member_type),
                types.get(&b.member_type),
                storeys.get(&b.level),
            ) else {
                continue;
            };
            let length = b.length();
            let Some(u) = (b.end - b.start).normalized() else {
                continue;
            };
            let (_, depth) = t.profile.size();
            let loc = self.point3(b.start.x, b.start.y, b.top_offset - depth / 2.0);
            let xdir = self
                .w
                .add(format!("IFCDIRECTION(({},{},0.))", real(u.x), real(u.y)));
            let pl = self.placement(Some(storey_pl), loc, Some(xdir));
            // Extrude along local X: the solid's Z is the beam's X, its X
            // (the profile's X) the beam's left (local Y).
            let ax = self.w.add("IFCDIRECTION((1.,0.,0.))");
            let rd = self.w.add("IFCDIRECTION((0.,1.,0.))");
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},{ax},{rd})", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(length)
            ));
            let el = self.member_element(
                "IFCBEAM",
                "BEAM",
                &b.guid,
                &format!("{} {}", t.name, b.id.0),
                &t.name,
                b.id.0,
                pl,
                solid,
                set,
            );
            self.member_psets(
                el,
                b.id.0,
                &b.guid,
                "Pset_BeamCommon",
                "Qto_BeamBaseQuantities",
                &t.name,
                length,
                t.profile.area(),
            );
            contained.entry(b.level).or_default().push(el);
            by_type.entry(b.member_type).or_default().push(el);
        }

        for (t, elements) in &by_type {
            let guid = model
                .member_type(*t)
                .map(|t| t.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{})",
                string(derived_guid(&guid, "type").as_str()),
                list(elements),
                types[t].0
            ));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn member_element(
        &mut self,
        entity: &str,
        predefined: &str,
        guid: &archisim_core::GlobalId,
        name: &str,
        object_type: &str,
        tag: u64,
        placement: Ref,
        solid: Ref,
        set: Ref,
    ) -> Ref {
        let body = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
            self.body_ctx
        ));
        let shape = self
            .w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
        let el = self.w.add(format!(
            "{entity}({},$,{},$,{},{placement},{shape},{},.{predefined}.)",
            string(guid.as_str()),
            string(name),
            string(object_type),
            string(&tag.to_string())
        ));
        // Cardinal point 5: the section is centred on the axis.
        let usage = self.w.add(format!("IFCMATERIALPROFILESETUSAGE({set},5,$)"));
        self.w.add(format!(
            "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{usage})",
            string(derived_guid(guid, "material").as_str())
        ));
        el
    }

    #[allow(clippy::too_many_arguments)]
    fn member_psets(
        &mut self,
        el: Ref,
        id: u64,
        guid: &archisim_core::GlobalId,
        pset: &str,
        qto: &str,
        type_name: &str,
        length: f64,
        area: f64,
    ) {
        self.common_pset(el, id, guid, pset, type_name, false, true);
        self.quantities(
            el,
            guid,
            qto,
            &[
                ("Length", "IFCQUANTITYLENGTH", length),
                ("CrossSectionArea", "IFCQUANTITYAREA", area),
                ("GrossVolume", "IFCQUANTITYVOLUME", area * length),
                ("NetVolume", "IFCQUANTITYVOLUME", area * length),
            ],
        );
    }
}
