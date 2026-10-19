//! Curtain walls written by ArchiSim: an `IfcCurtainWall` with the
//! `ArchiSim_CurtainWall` property set (line, top, grid, frame, panels) is
//! read back as one curtain wall; its frame and panel materials are those
//! of its members and plates. Curtain walls from other programs are not
//! imported yet.

use archisim_core::{NewCurtainWall, Point2, WallTop};

use super::{explain, snap, snap2, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_curtain_walls(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCCURTAINWALL");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_curtain_wall(id) {
                self.report.skip("muros cortina", why);
            }
        }
    }

    fn import_curtain_wall(&mut self, id: u64) -> Result<(), String> {
        const PSET: &str = "ArchiSim_CurtainWall";
        let text = |name: &str| {
            self.property_own(id, PSET, name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let Some(line) = text("Line") else {
            return Ok(());
        };
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, PSET, name)
                .and_then(|v| v.as_f64())
                .map(|v| snap(v * len))
                .ok_or_else(|| format!("falta {name}"))
        };
        let [base_offset, height, bay, row, frame_width, frame_depth, panel_thickness] = [
            number("BaseOffset")?,
            number("Height")?,
            number("Bay")?,
            number("Row")?,
            number("FrameWidth")?,
            number("FrameDepth")?,
            number("PanelThickness")?,
        ];
        let ends: Option<Vec<Point2>> = line
            .split(';')
            .map(|p| {
                let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(x), Some(y)) => Some(snap2(Point2::new(x * len, y * len))),
                    _ => None,
                }
            })
            .collect();
        let ends = ends.ok_or("línea ilegible")?;
        let [start, end] = ends[..] else {
            return Err("línea ilegible".into());
        };
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
        let first = |kind: &str| {
            parts
                .iter()
                .copied()
                .filter(|c| self.ent(*c).is_some_and(|e| e.kind == kind))
                .min()
        };
        let (frame, panel) = (first("IFCMEMBER"), first("IFCPLATE"));
        let frame_material = self.single_material(frame.unwrap_or(id));
        let panel_material = self.single_material(panel.unwrap_or(id));
        let level = self.level_of(id, 0.0);
        let cid = self
            .model
            .add_curtain_wall(NewCurtainWall {
                level,
                start,
                end,
                base_offset,
                top,
                height,
                bay,
                row,
                frame_width,
                frame_depth,
                panel_thickness,
                frame_material,
                panel_material,
            })
            .map_err(|e| explain(&e))?;
        self.curtain_ids.insert(id);
        self.report.curtain_walls += 1;
        self.sources.push((cid.0, id));
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .curtain_walls
            .push((cid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
