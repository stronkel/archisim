//! Plots and roads: a site inside the project's site with its boundary as
//! a `FootPrint` is a plot (from any program); a road is read back from
//! the `ArchiSim_Road` set of its carriageway (its other pieces are made
//! again from the section).

use archisim_core::{NewPlot, NewRoad, Point2, RoadSection};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_plots(&mut self) {
        let mut ids = self.file.of_kind("IFCSITE");
        ids.sort_unstable();
        for id in ids {
            let inner = self
                .ix
                .parent
                .get(&id)
                .and_then(|p| self.ent(*p))
                .is_some_and(|p| p.kind == "IFCSITE");
            if !inner {
                continue;
            }
            if let Err(why) = self.import_plot(id) {
                self.report.skip("parcelas", why);
            }
        }
    }

    fn import_plot(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        // The FootPrint's first curve.
        let shape = self.deref(e.arg(6)).ok_or("sin contorno")?;
        let mut outline: Option<Vec<Point2>> = None;
        for rep in shape.arg(2).as_list() {
            let Some(rep) = self.deref(rep) else { continue };
            if rep.arg(1).as_str() != Some("FootPrint") {
                continue;
            }
            for item in rep.arg(3).as_list() {
                let Some(item) = self.deref(item) else {
                    continue;
                };
                let curves: Vec<_> = if item.kind == "IFCGEOMETRICCURVESET" {
                    item.arg(0).as_list().to_vec()
                } else {
                    vec![rep.arg(3).as_list()[0].clone()]
                };
                for c in curves {
                    let mut curved = false;
                    if let Some(pts) = self.geo.curve(&c, &mut curved) {
                        let mut pts: Vec<Point2> = pts
                            .iter()
                            .map(|p| {
                                let q = xf.apply(*p);
                                Point2::new(snap(q[0]), snap(q[1]))
                            })
                            .collect();
                        if pts.len() > 3 && pts.first() == pts.last() {
                            pts.pop();
                        }
                        outline = Some(pts);
                        break;
                    }
                }
                if outline.is_some() {
                    break;
                }
            }
        }
        let outline = outline.ok_or("parcela sin contorno (FootPrint) que ArchiSim sepa leer")?;
        let level = self.level_by_guid(id, "ArchiSim_Plot", xf.apply([0.0, 0.0, 0.0])[2]);
        let setback = self
            .property_own(id, "ArchiSim_Plot", "Setback")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let name = e
            .arg(2)
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(|| self.model.next_plot_name(), str::to_string);
        let reference = e.arg(12).as_str().unwrap_or("").to_string();
        let pid = self
            .model
            .add_plot(NewPlot {
                name,
                level,
                outline,
                setback: snap(setback),
                reference,
            })
            .map_err(|e| explain(&e))?;
        self.report.plots += 1;
        self.sources.push((pid.0, id));
        self.guids
            .plots
            .push((pid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }

    /// The level named by an ArchiSim set (`Level`: its GlobalId), or the
    /// one under `z`.
    fn level_by_guid(&mut self, id: u64, pset: &str, z: f64) -> archisim_core::LevelId {
        let named = self
            .property_own(id, pset, "Level")
            .and_then(|v| v.as_str())
            .and_then(|g| {
                self.guids
                    .levels
                    .iter()
                    .find(|(_, s)| s == g)
                    .map(|(l, _)| *l)
            });
        named.unwrap_or_else(|| self.level_of(id, z))
    }

    pub(super) fn import_roads(&mut self) {
        let mut ids = self.file.of_kind("IFCPAVEMENT");
        ids.extend(self.file.of_kind("IFCBUILDINGELEMENTPROXY"));
        ids.sort_unstable();
        for id in ids {
            if self.property_own(id, "ArchiSim_Road", "Axis").is_none() {
                continue;
            }
            if let Err(why) = self.import_road(id) {
                self.report.skip("viales", why);
            }
        }
    }

    fn import_road(&mut self, id: u64) -> Result<(), String> {
        let num = |s: &Self, key: &str, default: f64| {
            s.property_own(id, "ArchiSim_Road", key)
                .and_then(|v| v.as_f64())
                .map_or(default, snap)
        };
        let text = |s: &Self, key: &str| {
            s.property_own(id, "ArchiSim_Road", key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let axis: Vec<Point2> = text(self, "Axis")
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
        // Its strips (H6); or the four widths of before.
        let mut section = match text(self, "Section").and_then(|t| RoadSection::decode(&t)) {
            Some(s) => s,
            None => {
                let d = (6.0, 2.2, 0.15, 2.5);
                RoadSection::classic(
                    num(self, "Carriageway", d.0),
                    num(self, "Parking", d.1),
                    num(self, "Kerb", d.2),
                    num(self, "Sidewalk", d.3),
                )
            }
        };
        section.kerb_height = num(self, "KerbHeight", section.kerb_height);
        section.depth = num(self, "Depth", section.depth);
        let named = |s: &Self, key: &str| {
            text(s, key).and_then(|n| s.model.materials().find(|m| m.name == n).map(|m| m.id))
        };
        section.planting = named(self, "PlantingMaterial");
        section.bike = named(self, "BikeMaterial");
        section.markings = named(self, "MarkingMaterial");
        let top_offset = num(self, "TopOffset", 0.0);
        let level = self.level_by_guid(id, "ArchiSim_Road", 0.0);
        let fallback = self.single_material(id);
        let mat = |s: &Self, key: &str| {
            text(s, key)
                .and_then(|n| s.model.materials().find(|m| m.name == n).map(|m| m.id))
                .unwrap_or(fallback)
        };
        let (pavement, kerb_material, sidewalk_material) = (
            mat(self, "Pavement"),
            mat(self, "KerbMaterial"),
            mat(self, "SidewalkMaterial"),
        );
        let name = text(self, "RoadName").unwrap_or_else(|| self.model.next_road_name());
        let guid = text(self, "Road").unwrap_or_default();
        let rid = self
            .model
            .add_road(NewRoad {
                name,
                level,
                axis,
                section,
                top_offset,
                profile: text(self, "Profile")
                    .and_then(|t| archisim_core::Road::decode_profile(&t))
                    .unwrap_or_default(),

                changes: text(self, "Changes")
                    .and_then(|t| archisim_core::Road::decode_changes(&t))
                    .unwrap_or_default(),
                pavement,
                kerb_material,
                sidewalk_material,
            })
            .map_err(|e| explain(&e))?;
        self.report.roads += 1;
        self.sources.push((rid.0, id));
        // Its pieces are not loose elements.
        if let Some(g) = archisim_core::GlobalId::parse(&guid) {
            let pieces: Vec<String> = (0..4096)
                .map(|i| crate::derived_guid(&g, &format!("band {i}")).to_string())
                .collect();
            for kind in ["IFCPAVEMENT", "IFCKERB", "IFCBUILDINGELEMENTPROXY"] {
                for x in self.file.of_kind(kind) {
                    if self
                        .ent(x)
                        .and_then(|e| e.arg(0).as_str())
                        .is_some_and(|s| pieces.iter().any(|p| p == s))
                    {
                        self.road_ids.insert(x);
                    }
                }
            }
        }
        self.guids.roads.push((rid, guid));
        Ok(())
    }

    /// Roundabouts and squares (U3), from their `ArchiSim_Roundabout` set.
    pub(super) fn import_roundabouts(&mut self) {
        let ids = self.file.of_kind("IFCBUILDINGELEMENTPROXY");
        for id in ids {
            if self
                .property_own(id, "ArchiSim_Roundabout", "Shape")
                .is_none()
            {
                continue;
            }
            if let Err(why) = self.import_roundabout(id) {
                self.report.skip("glorietas", why);
            }
            self.road_ids.insert(id);
        }
    }

    fn import_roundabout(&mut self, id: u64) -> Result<(), String> {
        let text = |s: &Self, key: &str| {
            s.property_own(id, "ArchiSim_Roundabout", key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let centre = text(self, "Centre")
            .and_then(|t| {
                let (x, y) = t.split_once(',')?;
                Some(Point2::new(
                    snap(x.trim().parse().ok()?),
                    snap(y.trim().parse().ok()?),
                ))
            })
            .ok_or("sin centro")?;
        let shape = text(self, "Shape").unwrap_or_default();
        let v: Vec<&str> = shape.split_whitespace().collect();
        if v.len() != 6 {
            return Err("forma no válida".into());
        }
        let num = |i: usize| v[i].parse::<f64>().map(snap).map_err(|e| e.to_string());
        let level = self.level_by_guid(id, "ArchiSim_Roundabout", 0.0);
        let fallback = self.single_material(id);
        let mat = |s: &Self, key: &str| {
            text(s, key)
                .and_then(|n| s.model.materials().find(|m| m.name == n).map(|m| m.id))
                .unwrap_or(fallback)
        };
        let name = self
            .ent(id)
            .and_then(|e| e.arg(2).as_str().map(str::to_string))
            .unwrap_or_else(|| self.model.next_roundabout_name());
        let rid = self
            .model
            .add_roundabout(archisim_core::NewRoundabout {
                name,
                level,
                centre,
                radius: num(0)?,
                island: num(1)?,
                apron: num(2)?,
                top_offset: num(3)?,
                plaza: v[4] == "1",
                island_finish: archisim_core::IslandFinish::from_code(v[5]).unwrap_or_default(),
                pavement: mat(self, "Pavement"),
                kerb_material: mat(self, "KerbMaterial"),
                island_material: mat(self, "IslandMaterial"),
            })
            .map_err(|e| explain(&e))?;
        self.report.roundabouts += 1;
        self.sources.push((rid.0, id));
        if let Some(g) = self
            .ent(id)
            .and_then(|e| e.arg(0).as_str().map(str::to_string))
        {
            self.guids.roundabouts.push((rid, g));
        }
        Ok(())
    }
}
