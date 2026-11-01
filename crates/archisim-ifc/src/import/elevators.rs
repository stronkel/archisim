//! Lifts written by ArchiSim: an `IfcTransportElement` with
//! `ArchiSim_Elevator` comes back as a lift; its shaft
//! (`ArchiSim_ElevatorPart`) is left out.

use archisim_core::{NewElevator, Point2};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_elevators(&mut self) {
        if !self.own_file {
            return;
        }
        let mut walls = self.file.of_kind("IFCBUILDINGELEMENTPROXY");
        walls.sort_unstable();
        for id in walls {
            if self
                .property_own(id, "ArchiSim_ElevatorPart", "Elevator")
                .is_some()
            {
                self.pool_parts.insert(id);
            }
        }
        let mut ids = self.file.of_kind("IFCTRANSPORTELEMENT");
        ids.sort_unstable();
        for id in ids {
            if self
                .property_own(id, "ArchiSim_Elevator", "ShaftWidth")
                .is_none()
            {
                continue;
            }
            self.pool_parts.insert(id);
            if let Err(why) = self.import_elevator(id) {
                self.report.skip("ascensores", why);
            }
        }
    }

    fn import_elevator(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let len = self.geo.length;
        let text = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Elevator", name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let number = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Elevator", name)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("falta {name}"))
        };
        let position = text(self, "Position")
            .and_then(|t| {
                let (x, y) = t.split_once(',')?;
                Some(Point2::new(
                    snap(x.trim().parse::<f64>().ok()? * len),
                    snap(y.trim().parse::<f64>().ok()? * len),
                ))
            })
            .ok_or("sin posición")?;
        let level = self.level_of(id, 0.0);
        let named = |me: &Self, n: Option<String>| {
            n.and_then(|n| {
                me.model
                    .levels()
                    .into_iter()
                    .find(|l| l.name == n)
                    .map(|l| l.id)
            })
        };
        let top = named(self, text(self, "Top")).unwrap_or(level);
        let mut spec = NewElevator::new(level, top, position);
        spec.name = e.arg(2).as_str().unwrap_or("").to_string();
        spec.car_at = named(self, text(self, "CarAt"));
        spec.rotation = number(self, "Rotation").unwrap_or(0.0);
        let l = |name: &str, slot: &mut f64| -> Result<(), String> {
            *slot = snap(number(self, name)? * len);
            Ok(())
        };
        l("ShaftWidth", &mut spec.shaft_w)?;
        l("ShaftDepth", &mut spec.shaft_d)?;
        l("Wall", &mut spec.wall)?;
        l("Pit", &mut spec.pit)?;
        l("Overrun", &mut spec.overrun)?;
        l("CarWidth", &mut spec.car_w)?;
        l("CarDepth", &mut spec.car_d)?;
        l("CarHeight", &mut spec.car_h)?;
        l("DoorWidth", &mut spec.door_w)?;
        l("DoorHeight", &mut spec.door_h)?;
        spec.load = number(self, "Load").map_or(spec.load, |v| v.round().max(0.0) as u32);
        spec.cut_slabs = self
            .property_own(id, "ArchiSim_Elevator", "CutSlabs")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        spec.material = text(self, "Material")
            .and_then(|n| self.model.materials().find(|m| m.name == n).map(|m| m.id));
        let guid = e.arg(0).as_str().map(String::from);
        let eid = self.model.add_elevator(spec).map_err(|e| explain(&e))?;
        if let Some(g) = guid {
            self.guids.elevators.push((eid, g));
        }
        self.report.elevators += 1;
        Ok(())
    }
}
