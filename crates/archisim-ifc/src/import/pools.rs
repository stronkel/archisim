//! Pools written by ArchiSim: an `IfcElementAssembly` with `ArchiSim_Pool`
//! comes back as a pool; its parts and its water are left out of the walls,
//! slabs, coverings and spaces.

use archisim_core::{NewPool, Point2};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_pools(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCELEMENTASSEMBLY");
        ids.sort_unstable();
        for id in ids {
            if self.property_own(id, "ArchiSim_Pool", "Outline").is_none() {
                continue;
            }
            // Its parts, and the water by its GlobalId.
            let parts: Vec<u64> = self
                .ix
                .parent
                .iter()
                .filter(|(_, p)| **p == id)
                .map(|(c, _)| *c)
                .collect();
            self.pool_parts.extend(parts);
            if let Some(guid) = self
                .ent(id)
                .and_then(|e| e.arg(0).as_str())
                .and_then(archisim_core::GlobalId::parse)
            {
                let water = crate::derived_guid(&guid, "water").to_string();
                self.pool_waters.insert(water);
            }
            if let Err(why) = self.import_pool(id) {
                self.report.skip("piscinas", why);
            }
        }
    }

    fn import_pool(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let len = self.geo.length;
        let text = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Pool", name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let number = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Pool", name)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("falta {name}"))
        };
        let outline: Option<Vec<Point2>> = text(self, "Outline")
            .ok_or("sin contorno")?
            .split(';')
            .map(|p| {
                let v: Vec<f64> = p
                    .split_whitespace()
                    .filter_map(|x| x.parse::<f64>().ok())
                    .collect();
                (v.len() == 2).then(|| Point2::new(snap(v[0] * len), snap(v[1] * len)))
            })
            .collect();
        let outline = outline.ok_or("contorno ilegible")?;
        let lengths: Vec<f64> = [
            "TopOffset",
            "Shallow",
            "Deep",
            "Freeboard",
            "Wall",
            "Coping",
        ]
        .iter()
        .map(|n| number(self, n).map(|v| snap(v * len)))
        .collect::<Result<_, _>>()?;
        let slope = number(self, "Slope")?;
        let names: Vec<String> = ["Lining", "Shell", "CopingMaterial", "Water"]
            .iter()
            .map(|n| text(self, n).unwrap_or_default())
            .collect();
        let name = e.arg(2).as_str().unwrap_or("").to_string();
        let guid = e.arg(0).as_str().map(String::from);
        let mut mats = Vec::new();
        for wanted in &names {
            let m = self
                .model
                .materials()
                .find(|m| m.name == *wanted)
                .map(|m| m.id);
            mats.push(match m {
                Some(m) => m,
                None => self.fallback_material(""),
            });
        }
        let level = self.level_of(id, 0.0);
        let mut spec = NewPool::new(level, outline, mats[0]);
        spec.name = name;
        spec.top_offset = lengths[0];
        spec.shallow = lengths[1];
        spec.deep = lengths[2];
        spec.freeboard = lengths[3];
        spec.wall = lengths[4];
        spec.coping = lengths[5];
        spec.slope = slope;
        spec.lining = mats[0];
        spec.shell = mats[1];
        spec.coping_material = mats[2];
        spec.water = mats[3];
        let pid = self.model.add_pool(spec).map_err(|e| explain(&e))?;
        if let Some(g) = guid {
            self.guids.pools.push((pid, g));
        }
        self.report.pools += 1;
        Ok(())
    }
}
