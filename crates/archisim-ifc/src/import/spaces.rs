//! Rooms (`IfcSpace`): the outline of their vertical extrusion (or the top
//! of their body), their height, their storey (they are aggregated into
//! it), number (`Name`), name (`LongName`) and use (`ObjectType`).

use archisim_core::{NewSpace, Point2};

use super::{area, clean_outline, explain, geometry, snap, snap2, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_spaces(&mut self) {
        let mut ids = self.file.of_kind("IFCSPACE");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_space(id) {
                self.report.skip("locales", why);
            }
        }
    }

    fn import_space(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer".into());
        }
        let (outline, bottom, top) = if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            (x.outline.clone(), x.bottom, x.top)
        } else {
            let (lo, hi) = body.bounds().ok_or("sin geometría")?;
            match geometry::top_outline(&body, hi[2]) {
                Some((outline, _)) => (outline, lo[2], hi[2]),
                None => return Err("sin contorno en planta legible".into()),
            }
        };
        let outline = clean_outline(&outline);
        if outline.len() < 3 {
            return Err("contorno sin superficie".into());
        }
        let text = |i: usize| {
            e.arg(i)
                .as_str()
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        let (number, name, usage) = (text(2), text(7), text(4));
        // Some programs write only a name.
        let (number, name) = if name.is_empty() {
            (String::new(), number)
        } else {
            (number, name)
        };
        let level = self.level_of(id, bottom + 0.01);
        let mut spec = NewSpace {
            level,
            number,
            name: if name.is_empty() {
                "Local".to_string()
            } else {
                name
            },
            usage,
            outline,
            height: snap(top - bottom).max(0.01),
        };
        let sid = match self.model.add_space(spec.clone()) {
            Ok(s) => s,
            Err(archisim_core::ModelError::DuplicateNumber(_)) => {
                self.report
                    .warn("Locales con el número repetido en su planta: se importan sin número.");
                spec.number.clear();
                self.model.add_space(spec).map_err(|e| explain(&e))?
            }
            Err(e) => return Err(explain(&e)),
        };
        if self.own_file {
            let x = self
                .property(id, "ArchiSim_Space", "LabelX")
                .and_then(|v| v.as_f64());
            let y = self
                .property(id, "ArchiSim_Space", "LabelY")
                .and_then(|v| v.as_f64());
            if let (Some(x), Some(y)) = (x, y) {
                let len = self.geo.length;
                let p = snap2(Point2::new(x * len, y * len));
                let _ = self.model.edit_space(sid, |s| s.label = Some(p));
            }
        }
        self.report.spaces += 1;
        self.guids
            .spaces
            .push((sid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
