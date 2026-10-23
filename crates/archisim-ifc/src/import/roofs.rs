//! Pitched roofs written by ArchiSim: an `IfcRoof` with the
//! `ArchiSim_Roof` property set (footprint, pitch of each side, base and
//! overhang) is read back as one roof, and the slabs of its planes, already
//! imported as inclined slabs, are dropped. Roofs from other programs stay
//! as their planes: inclined slabs.

use archisim_core::{NewRoof, Point2, SlabId};

use super::{explain, snap, snap2, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_roofs(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCROOF");
        ids.sort_unstable();
        for id in ids {
            match self.import_roof(id) {
                Ok(true) => {}
                Ok(false) => {}
                Err(why) => self.report.skip("cubiertas", why),
            }
        }
        self.read_wall_trims();
    }

    /// Walls written trimmed under a roof: the trim and their own height.
    fn read_wall_trims(&mut self) {
        let walls: Vec<(u64, u64)> = self
            .sources
            .iter()
            .copied()
            .filter(|(id, _)| self.model.wall(archisim_core::WallId(*id)).is_some())
            .collect();
        for (wid, ifc) in walls {
            let Some(guid) = self
                .property_own(ifc, "ArchiSim_Trim", "Roof")
                .and_then(|v| v.as_str())
                .map(String::from)
            else {
                continue;
            };
            let Some(roof) = self
                .guids
                .roofs
                .iter()
                .find(|(_, g)| *g == guid)
                .map(|(r, _)| *r)
            else {
                continue;
            };
            let wall = archisim_core::WallId(wid);
            let height = self
                .property_own(ifc, "ArchiSim_Trim", "Height")
                .and_then(|v| v.as_f64())
                .map(|h| snap(h * self.geo.length));
            if let (Some(h), Some(w)) = (height, self.model.wall(wall).cloned()) {
                if w.top == archisim_core::WallTop::Unconnected {
                    let _ = self.model.set_wall_height(wall, h, w.base_offset);
                }
            }
            let _ = self.model.set_wall_roof(wall, Some(roof));
        }
    }

    /// Whether the roof was read as an ArchiSim roof.
    fn import_roof(&mut self, id: u64) -> Result<bool, String> {
        let text = |name: &str| {
            self.property_own(id, "ArchiSim_Roof", name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let (Some(footprint), Some(slopes)) = (text("Footprint"), text("Slopes")) else {
            return Ok(false);
        };
        let dormers_text = text("Dormers");
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, "ArchiSim_Roof", name)
                .and_then(|v| v.as_f64())
                .map(|v| v * len)
        };
        let (Some(base), Some(overhang)) = (number("BaseOffset"), number("Overhang")) else {
            return Err("datos de cubierta incompletos".into());
        };
        let outline: Option<Vec<Point2>> = footprint
            .split(';')
            .map(|p| {
                let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(x), Some(y)) => Some(snap2(Point2::new(x * len, y * len))),
                    _ => None,
                }
            })
            .collect();
        let slopes: Option<Vec<Option<f64>>> = slopes
            .split(';')
            .map(|s| match s.trim() {
                "-" => Some(None),
                v => v.parse::<f64>().ok().map(Some),
            })
            .collect();
        let (Some(outline), Some(slopes)) = (outline, slopes) else {
            return Err("contorno o pendientes de cubierta ilegibles".into());
        };
        // Its planes, imported as inclined slabs: they give the type and
        // the level, and then go.
        let parts: Vec<u64> = self
            .ix
            .parent
            .iter()
            .filter(|(_, &p)| p == id)
            .map(|(&c, _)| c)
            .collect();
        let made: Vec<u64> = self
            .sources
            .iter()
            .filter(|(_, ifc)| parts.contains(ifc))
            .map(|(sid, _)| *sid)
            .collect();
        let Some(first) = made
            .first()
            .and_then(|s| self.model.slab(SlabId(*s)))
            .cloned()
        else {
            return Err("cubierta sin faldones legibles".into());
        };
        for sid in &made {
            let _ = self.model.remove_slab(SlabId(*sid));
        }
        self.sources.retain(|(sid, _)| !made.contains(sid));
        self.guids.slabs.retain(|(sid, _)| !made.contains(&sid.0));
        self.report.slabs -= made.len();
        let rid = self
            .model
            .add_roof(NewRoof {
                roof_type: first.slab_type,
                level: first.level,
                outline,
                slopes,
                base_offset: snap(base),
                overhang: snap(overhang),
            })
            .map_err(|e| explain(&e))?;
        if let Some(dormers) = dormers_text {
            let dormers = crate::roofs::decode_dormers(&self.model, &dormers, len);
            let _ = self.model.set_roof_dormers(rid, dormers);
        }
        self.roof_ids.insert(id);
        self.report.roofs += 1;
        self.sources.push((rid.0, id));
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .roofs
            .push((rid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(true)
    }
}
