//! Rooms (`IfcSpace`): the outline of their vertical extrusion (or the
//! bottom of their body) with its holes, their height, their storey (they
//! are aggregated into it), number (`Name`), name (`LongName`, or the
//! description), use (`ObjectType`) and finishes
//! (`Pset_SpaceCoveringRequirements`). A file of ArchiSim also gives back
//! where the name was drawn, whether the room followed its walls and
//! whether its height was its own (`ArchiSim_Space`).

use archisim_core::{Finishes, NewRoom, Point2};

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
        let (outline, holes, bottom, top) = if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            (x.outline.clone(), x.holes.clone(), x.bottom, x.top)
        } else {
            let (lo, hi) = body.bounds().ok_or("sin geometría")?;
            match geometry::top_outline(&body, lo[2]) {
                Some((outline, holes)) => (outline, holes, lo[2], hi[2]),
                None => return Err("sin contorno en planta legible".into()),
            }
        };
        let outline = clean_outline(&outline);
        if outline.len() < 3 {
            return Err("contorno sin superficie".into());
        }
        let holes: Vec<Vec<Point2>> = holes
            .iter()
            .map(|h| clean_outline(h))
            .filter(|h| h.len() >= 3)
            .collect();
        let text = |i: usize| {
            e.arg(i)
                .as_str()
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        let (mut number, mut name, usage) = (text(2), text(7), text(4));
        if name.is_empty() {
            name = text(3);
        }
        // Some programs write only a name.
        if name.is_empty() {
            name = std::mem::take(&mut number);
        }
        if name.is_empty() {
            name = "Local".into();
        }
        let level = self.level_of(id, bottom + 0.01);

        let own = self.own_file;
        let read = |me: &Self, n: &str| me.property(id, "ArchiSim_Space", n);
        let label = match (
            read(self, "LabelX").and_then(|v| v.as_f64()),
            read(self, "LabelY").and_then(|v| v.as_f64()),
        ) {
            (Some(x), Some(y)) if own => {
                let len = self.geo.length;
                snap2(Point2::new(x * len, y * len))
            }
            _ => archisim_core::label_center(&outline),
        };
        let bounded = own && read(self, "Bounded").and_then(|v| v.as_bool()) == Some(true);
        let own_height = if own {
            read(self, "OwnHeight").and_then(|v| v.as_bool())
        } else {
            None
        };

        let mut spec = NewRoom {
            level,
            number,
            name,
            outline,
            holes,
            label,
            bounded,
        };
        let rid = match self.model.add_room(spec.clone()) {
            Ok(r) => r,
            Err(archisim_core::ModelError::DuplicateNumber(_)) => {
                self.report
                    .warn("Locales con el número repetido en su planta: se importan sin número.");
                spec.number.clear();
                self.model.add_room(spec).map_err(|e| explain(&e))?
            }
            Err(e) => return Err(explain(&e)),
        };
        let height = snap(top - bottom);
        let finish = |me: &Self, n: &str| {
            me.property(id, "Pset_SpaceCoveringRequirements", n)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        let finishes = Finishes {
            floor: finish(self, "FloorCovering"),
            walls: finish(self, "WallCovering"),
            ceiling: finish(self, "CeilingCovering"),
        };
        if let Some(mut r) = self.model.room(rid).cloned() {
            let default = self.model.room_height(&r);
            r.height = match own_height {
                Some(true) => Some(height.max(0.01)),
                Some(false) => None,
                // Another program: its own height when it is not the one
                // to the level above.
                None => (height > 0.01 && (height - default).abs() > 0.005).then_some(height),
            };
            r.finishes = finishes;
            r.usage = usage;
            self.model.set_room(r).map_err(|e| explain(&e))?;
        }
        self.report.rooms += 1;
        self.sources.push((rid.0, id));
        self.guids
            .rooms
            .push((rid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
