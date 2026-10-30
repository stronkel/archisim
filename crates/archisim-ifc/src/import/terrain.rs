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

    /// Where the file is on the map: its `IfcMapConversion` to a projected
    /// system named by its EPSG code.
    pub(super) fn read_georeference(&mut self) {
        for id in self.file.of_kind("IFCMAPCONVERSION") {
            let Some(e) = self.ent(id) else { continue };
            let Some(crs) = self.deref(e.arg(1)) else {
                continue;
            };
            let name = crs.arg(0).as_str().unwrap_or("").trim().to_string();
            let code = name.to_ascii_uppercase();
            let code = if code.starts_with("EPSG:") {
                code
            } else if code.chars().all(|c| c.is_ascii_digit()) && !code.is_empty() {
                format!("EPSG:{code}")
            } else {
                self.report.warn(
                    "Georreferencia con un sistema de coordenadas sin código EPSG: no se importa.",
                );
                return;
            };
            let len = self.geo.length;
            let num = |i: usize| e.arg(i).as_f64().map(|v| v * len);
            let (Some(x), Some(y)) = (num(2), num(3)) else {
                continue;
            };
            let _ = self
                .model
                .set_georeference(Some(archisim_core::Georeference {
                    crs: code,
                    eastings: x,
                    northings: y,
                    height: num(4).unwrap_or(0.0),
                }));
            return;
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
        let lines: Vec<Vec<usize>> = self
            .property_own(id, "ArchiSim_Terrain", "Lines")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(';')
                    .map(|l| {
                        l.split_whitespace()
                            .filter_map(|i| i.parse().ok())
                            .collect()
                    })
                    .collect()
            })
            .unwrap_or_default();
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
                lines,
            })
            .map_err(|e| explain(&e))?;
        // Its edge drawn by hand and what the plan shows (E5).
        let outline: Option<Vec<archisim_core::Point2>> = self
            .property_own(id, "ArchiSim_Terrain", "Outline")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(';')
                    .filter_map(|q| {
                        let mut it = q.split_whitespace().filter_map(|n| n.parse::<f64>().ok());
                        Some(archisim_core::Point2::new(
                            snap(it.next()? * len),
                            snap(it.next()? * len),
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|o| o.len() >= 3);
        let flag = |name: &str| {
            self.property_own(id, "ArchiSim_Terrain", name)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        };
        let (show_heights, slope_arrows) = (flag("ShowHeights"), flag("SlopeArrows"));
        let _ = self.model.edit_terrain(tid, |t| {
            t.outline = outline;
            t.show_heights = show_heights;
            t.slope_arrows = slope_arrows;
        });
        self.terrain_ids.insert(id);
        self.report.terrains += 1;
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .terrains
            .push((tid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
