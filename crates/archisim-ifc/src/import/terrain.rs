//! The site: the true north of any file (from its geometric context), and
//! terrains written by ArchiSim (`IfcGeographicElement` with the
//! `ArchiSim_Terrain` survey points).

use archisim_core::{NewTerrain, Point2, SurveyPoint};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn read_true_north(&mut self) {
        for id in self.file.of_kind("IFCGEOMETRICREPRESENTATIONCONTEXT") {
            let Some(e) = self.ent(id) else { continue };
            let Some(d) = self.deref(e.arg(5)) else {
                continue;
            };
            let v = d.arg(0).numbers();
            if v.len() >= 2 && v[0].hypot(v[1]) > 1e-9 {
                let _ = self.model.set_true_north(v[0].atan2(v[1]));
                return;
            }
        }
    }

    pub(super) fn import_terrains(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCGEOGRAPHICELEMENT");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_terrain(id) {
                self.report.skip("terrenos", why);
            }
        }
    }

    fn import_terrain(&mut self, id: u64) -> Result<(), String> {
        let Some(points) = self
            .property_own(id, "ArchiSim_Terrain", "Points")
            .and_then(|v| v.as_str())
            .map(String::from)
        else {
            return Ok(());
        };
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, "ArchiSim_Terrain", name)
                .and_then(|v| v.as_f64())
                .map(|v| v * len)
                .ok_or_else(|| format!("falta {name}"))
        };
        let depth = number("Depth")?;
        let contour = number("Contour")?;
        let points: Option<Vec<SurveyPoint>> = points
            .split(';')
            .map(|p| {
                let v: Vec<f64> = p
                    .split_whitespace()
                    .filter_map(|x| x.parse::<f64>().ok())
                    .collect();
                (v.len() == 3).then(|| SurveyPoint {
                    at: Point2::new(snap(v[0] * len), snap(v[1] * len)),
                    z: snap(v[2] * len),
                })
            })
            .collect();
        let points = points.ok_or("puntos ilegibles")?;
        let material = self.single_material(id);
        let level = self.level_of(id, 0.0);
        let tid = self
            .model
            .add_terrain(NewTerrain {
                level,
                points,
                material,
                depth: snap(depth),
                contour: snap(contour),
            })
            .map_err(|e| explain(&e))?;
        self.terrain_ids.insert(id);
        self.report.terrains += 1;
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .terrains
            .push((tid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
