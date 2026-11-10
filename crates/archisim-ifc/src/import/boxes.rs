//! Modular boxes written by ArchiSim: an `IfcElementAssembly` with
//! `ArchiSim_Box` comes back as a box. What it holds comes back as the
//! walls, slabs and the rest they are (their storey is the box's); the box
//! finds them again by where they are.

use archisim_core::{NewBox, Point2, SideState, SIDE_NAMES};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_boxes(&mut self) {
        if !self.own_file {
            return;
        }
        let mut ids = self.file.of_kind("IFCELEMENTASSEMBLY");
        ids.sort_unstable();
        for id in ids {
            if self.property_own(id, "ArchiSim_Box", "Mark").is_none() {
                continue;
            }
            if let Err(why) = self.import_box(id) {
                self.report.skip("cajas modulares", why);
            }
        }
    }

    fn import_box(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let guid = e.arg(0).as_str().map(String::from);
        let len = self.geo.length;
        let text = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Box", name)
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default()
        };
        let number = |me: &Self, name: &str| {
            me.property_own(id, "ArchiSim_Box", name)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("falta {name}"))
        };
        let length = |me: &Self, name: &str| number(me, name).map(|v| snap(v * len));
        let mut sides = [SideState::Free; 4];
        for (i, s) in sides.iter_mut().enumerate() {
            let code = text(self, &format!("Side{}", SIDE_NAMES[i]));
            *s = SideState::from_code(&code).ok_or_else(|| format!("lado «{code}»"))?;
        }
        let spec = NewBox {
            mark: Some(text(self, "Mark")),
            kind: text(self, "Kind"),
            level: self.level_of(id, 0.0),
            origin: Point2::new(length(self, "OriginX")?, length(self, "OriginY")?),
            angle: number(self, "Angle")?,
            length: length(self, "Length")?,
            width: length(self, "Width")?,
            bottom: length(self, "Bottom")?,
            height: length(self, "Height")?,
            sides,
            chassis: self.chassis_of(id),
            progress: self.progress_of(id),
        };
        let bid = self.model.add_box(spec).map_err(|e| explain(&e))?;
        if let Some(g) = guid {
            self.guids.boxes.push((bid, g));
        }
        self.report.boxes += 1;
        Ok(())
    }

    /// The chassis kept in `ArchiSim_Box`; the usual one for what is
    /// missing (a box written before M3).
    fn chassis_of(&self, id: u64) -> archisim_core::Chassis {
        let mut c = archisim_core::Chassis::default();
        let text = |name: &str| {
            self.property_own(id, "ArchiSim_Box", name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let number = |name: &str| {
            self.property_own(id, "ArchiSim_Box", name)
                .and_then(|v| v.as_f64())
                .map(|v| snap(v * self.geo.length))
        };
        let tube = |name: &str| text(name).and_then(|t| archisim_core::BoxTube::parse(&t));
        if let Some(t) = tube("EdgeTube") {
            c.edge = t;
        }
        if let Some(t) = tube("PostTube") {
            c.post = t;
        }
        if let Some(t) = tube("JoistTube") {
            c.joist = t;
        }
        if let Some(t) = tube("RoofJoistTube") {
            c.roof_joist = t;
        }
        if let Some(t) = tube("RoofEdgeTube") {
            c.roof_edge = t;
        }
        if let Some(t) = tube("CrownTube") {
            c.crown = t;
        }
        if let Some(t) = tube("ParapetPostTube") {
            c.parapet_post = t;
        }
        if let Some(v) = number("ParapetSpacing") {
            c.parapet_spacing = v;
        }
        if let Some(v) = number("JoistSpacing") {
            c.spacing = v;
        }
        if let Some(v) = number("RoofFrameAt") {
            c.roof_at = v;
        }
        if let Some(l) = text("LiftPoints").and_then(|t| archisim_core::LiftPoints::from_code(&t)) {
            c.lift = l;
        }
        if let Some(v) = self.property_own(id, "ArchiSim_Box", "MiddleBeam") {
            if let Some(b) = v.as_bool() {
                c.middle_beam = b;
            }
        }
        if c.is_valid() {
            c
        } else {
            archisim_core::Chassis::default()
        }
    }

    /// Where the box is, from `ArchiSim_Box` (in design when missing).
    fn progress_of(&self, id: u64) -> archisim_core::BoxProgress {
        let text = |name: &str| {
            self.property_own(id, "ArchiSim_Box", name)
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default()
        };
        archisim_core::BoxProgress {
            stage: archisim_core::BoxStage::from_code(&text("Stage")).unwrap_or_default(),
            station: text("Station"),
            date: text("StageDate"),
        }
    }
}
