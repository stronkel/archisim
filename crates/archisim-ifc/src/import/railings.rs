//! Railings written by ArchiSim: an `IfcRailing` with the
//! `ArchiSim_Railing` property set (line, foot, height, post spacing) is
//! read back as one railing. Railings of stairs come back with their
//! stair; railings from other programs are not imported yet.

use archisim_core::{NewRailing, Point2};

use super::{explain, snap, snap2, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_railings(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCRAILING");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_railing(id) {
                self.report.skip("barandillas", why);
            }
        }
    }

    fn import_railing(&mut self, id: u64) -> Result<(), String> {
        const PSET: &str = "ArchiSim_Railing";
        let Some(path) = self
            .property_own(id, PSET, "Path")
            .and_then(|v| v.as_str())
            .map(String::from)
        else {
            return Ok(());
        };
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, PSET, name)
                .and_then(|v| v.as_f64())
                .map(|v| snap(v * len))
                .ok_or_else(|| format!("falta {name}"))
        };
        let [base_offset, height, post_spacing] = [
            number("BaseOffset")?,
            number("Height")?,
            number("PostSpacing")?,
        ];
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
        let path = path.ok_or("línea ilegible")?;
        let material = self.single_material(id);
        let glass = self
            .property_own(id, PSET, "Glass")
            .and_then(|v| v.as_str())
            .and_then(|n| self.model.materials().find(|m| m.name == n))
            .map(|m| m.id);
        let level = self.level_of(id, 0.0);
        let rid = self
            .model
            .add_railing(NewRailing {
                level,
                path,
                base_offset,
                height,
                post_spacing,
                material,
                glass,
                railing_type: None,
            })
            .map_err(|e| explain(&e))?;
        self.railing_ids.insert(id);
        self.report.railings += 1;
        self.sources.push((rid.0, id));
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .railings
            .push((rid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
