//! Stairs written by ArchiSim: an `IfcStair` with the `ArchiSim_Stair`
//! property set (walking line, width, going, foot, top, waist, use) is
//! read back as one stair; the slabs of its landings, already imported,
//! are dropped. Stairs from other programs are not imported yet.

use archisim_core::{NewStair, Point2, SlabId, StairUse, WallTop};

use super::{explain, snap, snap2, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_stairs(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCSTAIR");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_stair(id) {
                self.report.skip("escaleras", why);
            }
        }
    }

    fn import_stair(&mut self, id: u64) -> Result<(), String> {
        let text = |name: &str| {
            self.property_own(id, "ArchiSim_Stair", name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let Some(path) = text("Path") else {
            return Ok(());
        };
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, "ArchiSim_Stair", name)
                .and_then(|v| v.as_f64())
                .map(|v| v * len)
                .ok_or_else(|| format!("falta {name}"))
        };
        let width = number("Width")?;
        let going = number("Going")?;
        let base_offset = number("BaseOffset")?;
        let height = number("Height")?;
        let waist = number("Waist")?;
        let railings = match text("Railings").as_deref() {
            Some("none") => archisim_core::Railings::None,
            Some("left") => archisim_core::Railings::Left,
            Some("right") => archisim_core::Railings::Right,
            _ => archisim_core::Railings::Both,
        };
        let usage = match text("Usage").as_deref() {
            Some("restricted") => StairUse::Restricted,
            _ => StairUse::General,
        };
        let path: Option<Vec<Point2>> = path
            .split(';')
            .map(|p| {
                let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(x), Some(y)) => Some(snap2(Point2::new(x * len, y * len))),
                    _ => None,
                }
            })
            .collect();
        let path = path.ok_or("línea de huella ilegible")?;
        // Up to a level, by its GlobalId.
        let top = match text("Top") {
            Some(t) if !t.trim().is_empty() => {
                let mut it = t.split_whitespace();
                let guid = it.next().unwrap_or_default().to_string();
                let offset = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * len;
                let storey = self
                    .file
                    .of_kind("IFCBUILDINGSTOREY")
                    .into_iter()
                    .find(|s| self.ent(*s).and_then(|e| e.arg(0).as_str()) == Some(guid.as_str()));
                match storey.and_then(|s| self.storeys.get(&s)).copied() {
                    Some(level) => WallTop::Level {
                        level,
                        offset: snap(offset),
                    },
                    None => WallTop::Unconnected,
                }
            }
            _ => WallTop::Unconnected,
        };
        let parts: Vec<u64> = self
            .ix
            .parent
            .iter()
            .filter(|(_, &p)| p == id)
            .map(|(&c, _)| c)
            .collect();
        let flight = parts
            .iter()
            .copied()
            .filter(|c| self.ent(*c).is_some_and(|e| e.kind == "IFCSTAIRFLIGHT"))
            .min();
        let material = match flight {
            Some(f) => self.single_material(f),
            None => self.single_material(id),
        };
        // Landings were imported as slabs: they go.
        let made: Vec<u64> = self
            .sources
            .iter()
            .filter(|(_, ifc)| parts.contains(ifc))
            .map(|(sid, _)| *sid)
            .collect();
        for sid in &made {
            if self.model.remove_slab(SlabId(*sid)).is_ok() {
                self.report.slabs -= 1;
            }
        }
        self.sources.retain(|(sid, _)| !made.contains(sid));
        self.guids.slabs.retain(|(sid, _)| !made.contains(&sid.0));
        let level = self.level_of(id, 0.0);
        let sid = self
            .model
            .add_stair(NewStair {
                level,
                path,
                width: snap(width),
                going: snap(going),
                base_offset: snap(base_offset),
                top,
                height: snap(height),
                waist: snap(waist),
                material,
                usage,
            })
            .map_err(|e| explain(&e))?;
        let _ = self.model.edit_stair(sid, |s| s.railings = railings);
        self.stair_ids.insert(id);
        self.report.stairs += 1;
        self.sources.push((sid.0, id));
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .stairs
            .push((sid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
