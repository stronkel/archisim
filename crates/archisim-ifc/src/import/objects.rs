//! Objects written by ArchiSim: an `IfcFurniture` or
//! `IfcSanitaryTerminal` whose type carries ArchiSim's family data is read
//! back as an object of that family, at its placement. Objects from other
//! programs are not imported yet.

use archisim_core::{NewObject, ObjectTypeId, Point2};

use super::{explain, snap, snap2, Entity, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_objects(&mut self) {
        let mut ids = self.file.of_kind("IFCFURNITURE");
        ids.extend(self.file.of_kind("IFCSANITARYTERMINAL"));
        // Vegetation, people and vehicles (terrains have no family).
        ids.extend(self.file.of_kind("IFCGEOGRAPHICELEMENT"));
        ids.extend(self.file.of_kind("IFCBUILDINGELEMENTPROXY"));
        ids.extend(self.file.of_kind("IFCVEHICLE"));
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_object(id) {
                self.report.skip("objetos", why);
            }
        }
    }

    fn import_object(&mut self, id: u64) -> Result<(), String> {
        let Some(ifc_type) = self.ix.type_of.get(&id).copied() else {
            return Ok(());
        };
        let Some(te) = self.ent(ifc_type) else {
            return Ok(());
        };
        let type_props: Vec<&Entity> = te
            .arg(5)
            .as_list()
            .iter()
            .filter_map(|p| self.deref(p))
            .collect();
        let Some(data) = self.family_data(te, &type_props) else {
            return Ok(());
        };
        let type_name = te
            .arg(2)
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Objeto")
            .to_string();
        let tid = match self.object_types.get(&ifc_type) {
            Some(t) => *t,
            None => {
                let (family, values) = self.family_values(&data)?;
                let t = self
                    .model
                    .add_object_type(type_name, family, values)
                    .map_err(|e| explain(&e))?;
                if let Some(g) = self.type_guid(Some(ifc_type)) {
                    self.guids.object_types.push((t, g));
                }
                self.object_types.insert(ifc_type, t);
                t
            }
        };
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let guid = e.arg(0).as_str().unwrap_or("").to_string();
        let z = xf.origin[2];
        let level = self.level_of(id, z + 0.01);
        let elev = self.elevation(level);
        let oid = self
            .model
            .add_object(NewObject {
                level,
                object_type: tid,
                position: snap2(Point2::new(xf.origin[0], xf.origin[1])),
                rotation: snap(xf.x[1].atan2(xf.x[0])),
                base_offset: snap(z - elev),
            })
            .map_err(|e| explain(&e))?;
        self.object_ids.insert(id);
        self.report.objects += 1;
        self.sources.push((oid.0, id));
        self.guids.objects.push((oid, guid));
        Ok(())
    }
}

/// Imported object types by their IFC type entity.
pub(super) type ObjectTypes = std::collections::HashMap<u64, ObjectTypeId>;
