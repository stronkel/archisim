//! Pipes, ducts and devices written by ArchiSim, read back from their
//! `ArchiSim_Run` and `ArchiSim_Device` sets (a run from its first
//! stretch; the others are made again from its path).

use archisim_core::{DeviceKind, NewDevice, NewRun, Point2, ServiceSystem};

use super::{explain, snap, Importer};

const RUN_CLASSES: [&str; 2] = ["IFCPIPESEGMENT", "IFCDUCTSEGMENT"];
const DEVICE_CLASSES: [&str; 4] = [
    "IFCUNITARYEQUIPMENT",
    "IFCLIGHTFIXTURE",
    "IFCOUTLET",
    "IFCSWITCHINGDEVICE",
];

impl<'a> Importer<'a> {
    pub(super) fn import_services(&mut self) {
        let mut ids: Vec<u64> = RUN_CLASSES
            .iter()
            .flat_map(|k| self.file.of_kind(k))
            .collect();
        ids.sort_unstable();
        for id in ids {
            if self.property_own(id, "ArchiSim_Run", "Run").is_none() {
                continue;
            }
            // Every stretch of a run read back is part of it.
            self.service_ids.insert(id);
            if self.property_own(id, "ArchiSim_Run", "Path").is_none() {
                continue;
            }
            if let Err(why) = self.import_run(id) {
                self.report.skip("instalaciones", why);
            }
        }
        let mut ids: Vec<u64> = DEVICE_CLASSES
            .iter()
            .flat_map(|k| self.file.of_kind(k))
            .collect();
        ids.sort_unstable();
        for id in ids {
            if self.property_own(id, "ArchiSim_Device", "Kind").is_none() {
                continue;
            }
            self.service_ids.insert(id);
            if let Err(why) = self.import_device(id) {
                self.report.skip("instalaciones", why);
            }
        }
    }

    fn run_text(&self, id: u64, pset: &str, key: &str) -> Option<String> {
        self.property_own(id, pset, key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    fn run_num(&self, id: u64, pset: &str, key: &str) -> Option<f64> {
        self.property_own(id, pset, key).and_then(|v| v.as_f64())
    }

    fn import_run(&mut self, id: u64) -> Result<(), String> {
        let path: Vec<Point2> = self
            .run_text(id, "ArchiSim_Run", "Path")
            .unwrap_or_default()
            .split(';')
            .filter_map(|p| {
                let (x, y) = p.split_once(',')?;
                Some(Point2::new(
                    snap(x.trim().parse().ok()?),
                    snap(y.trim().parse().ok()?),
                ))
            })
            .collect();
        let system = self
            .run_text(id, "ArchiSim_Run", "System")
            .and_then(|c| ServiceSystem::from_code(&c))
            .ok_or("sistema desconocido")?;
        let elevation = snap(self.run_num(id, "ArchiSim_Run", "Elevation").unwrap_or(0.0));
        let width = snap(
            self.run_num(id, "ArchiSim_Run", "Width")
                .ok_or("sin tamaño")?,
        );
        let height = snap(self.run_num(id, "ArchiSim_Run", "Height").unwrap_or(width));
        let level = self.level_of(id, 0.0);
        let rid = self
            .model
            .add_run(NewRun {
                level,
                system,
                path,
                elevation,
                width,
                height,
            })
            .map_err(|e| explain(&e))?;
        self.report.services += 1;
        self.sources.push((rid.0, id));
        let guid = self.run_text(id, "ArchiSim_Run", "Run").unwrap_or_default();
        self.guids.runs.push((rid, guid));
        Ok(())
    }

    fn import_device(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let kind = self
            .run_text(id, "ArchiSim_Device", "Kind")
            .and_then(|c| DeviceKind::from_code(&c))
            .ok_or("aparato desconocido")?;
        let position = self
            .run_text(id, "ArchiSim_Device", "Position")
            .and_then(|t| {
                let (x, y) = t.split_once(',')?;
                Some(Point2::new(
                    snap(x.trim().parse().ok()?),
                    snap(y.trim().parse().ok()?),
                ))
            })
            .ok_or("sin posición")?;
        let num = |s: &Self, k: &str, d: f64| s.run_num(id, "ArchiSim_Device", k).unwrap_or(d);
        let (w, dpt, h, z) = kind.defaults();
        let spec = NewDevice {
            level: self.level_of(id, 0.0),
            kind,
            name: e
                .arg(2)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(kind.name())
                .to_string(),
            position,
            rotation: num(self, "Rotation", 0.0),
            elevation: snap(num(self, "Elevation", z)),
            width: snap(num(self, "Width", w)),
            depth: snap(num(self, "Depth", dpt)),
            height: snap(num(self, "Height", h)),
        };
        let did = self.model.add_device(spec).map_err(|e| explain(&e))?;
        self.report.services += 1;
        self.sources.push((did.0, id));
        self.guids
            .devices
            .push((did, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
