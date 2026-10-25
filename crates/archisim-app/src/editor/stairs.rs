//! Stairs: drawing them by their walking line and changing them, each
//! change one undoable step.

use archisim_core::{
    preset_stair, LevelId, MaterialId, Model, NewStair, Point2, PresetSpec, Stair, StairDesign,
    StairForm, StairId, StairPreset, StairStructure, StairUse, Vec2, WallTop,
};

use super::{EditResult, Editor, TopChoice};

/// Settings of the stair tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairSettings {
    pub width: f64,
    pub going: f64,
    pub waist: f64,
    pub material: MaterialId,
    pub usage: StairUse,
    pub top: TopChoice,
    /// Height of stairs not tied to a level.
    pub height: f64,
    /// The shape drawn: ready-made (from its foot and direction) or any
    /// walking line.
    pub preset: StairPreset,
    /// Turns with winders instead of landings.
    pub winders: bool,
    /// Landings split at 45° (when not winders).
    pub split: bool,
    /// A spiral's landing at its head, radians (0, none).
    pub arrival: f64,
    /// Ready-made shapes turn left (counter-clockwise) walking up.
    pub left: bool,
    /// The riser height aimed at by ready-made shapes.
    pub riser: f64,
    /// The gap between the flights of a U ("ojo").
    pub gap: f64,
    /// The radius of a spiral's newel or eye.
    pub core: f64,
    pub structure: StairStructure,
    pub closed_risers: bool,
    pub tread: f64,
    pub tread_material: Option<MaterialId>,
    pub stringer: [f64; 2],
}

/// The tool's settings for a model: 1 m wide, 28 cm treads, a 15 cm
/// concrete slab, up to the next level.
pub fn default_stair_settings(model: &Model) -> StairSettings {
    let material = model
        .materials()
        .find(|m| m.category == "concrete")
        .or_else(|| model.materials().next())
        .map_or(MaterialId(0), |m| m.id);
    StairSettings {
        width: 1.0,
        going: 0.28,
        waist: 0.15,
        material,
        usage: StairUse::General,
        top: TopChoice::NextLevel,
        height: 3.0,
        preset: StairPreset::Straight,
        winders: false,
        split: false,
        arrival: std::f64::consts::FRAC_PI_2,
        left: true,
        riser: 0.175,
        gap: 0.2,
        core: 0.1,
        structure: StairStructure::Slab,
        closed_risers: true,
        tread: 0.04,
        tread_material: None,
        stringer: [0.08, 0.3],
    }
}

impl StairSettings {
    pub fn top_on(&self, model: &Model, level: LevelId) -> WallTop {
        let target = match self.top {
            TopChoice::Unconnected => None,
            TopChoice::NextLevel => model.level_above(level),
            TopChoice::Level(l) => Some(l),
        };
        match target {
            Some(level) => WallTop::Level { level, offset: 0.0 },
            None => WallTop::Unconnected,
        }
    }

    /// The form and build of a stair drawn with these settings.
    pub fn design(&self, form: StairForm, sweep: f64) -> StairDesign {
        // A spiral is carried by its newel unless asked otherwise; a
        // newel only carries spirals.
        let structure = match (form, self.structure) {
            (StairForm::Spiral, s) => s,
            (_, StairStructure::Newel) => StairStructure::Stringers,
            (_, s) => s,
        };
        StairDesign {
            form,
            sweep,
            core: self.core,
            structure,
            closed_risers: self.closed_risers,
            tread: self.tread,
            tread_material: self.tread_material,
            stringer: self.stringer,
            split: self.split && !self.winders && form == StairForm::Landings,
            arrival: if form == StairForm::Spiral {
                self.arrival
            } else {
                0.0
            },
        }
    }

    /// What a ready-made stair drawn on `level` needs.
    pub fn preset_spec(&self, model: &Model, level: LevelId) -> PresetSpec {
        PresetSpec {
            preset: self.preset,
            winders: self.winders,
            split: self.split,
            left: self.left,
            width: self.width,
            going: self.going,
            riser: self.riser,
            height: self.height_on(model, level),
            gap: self.gap,
            core: self.core,
            edge: archisim_core::walking_edge(self.usage, self.width),
        }
    }

    /// The walking line and design of a ready-made stair from its foot
    /// (a spiral's centre) and the direction it sets off in.
    pub fn preset_on(
        &self,
        model: &Model,
        level: LevelId,
        foot: Point2,
        dir: Vec2,
    ) -> Option<(Vec<Point2>, StairDesign)> {
        let (path, form, sweep) = preset_stair(&self.preset_spec(model, level), foot, dir)?;
        Some((path, self.design(form, sweep)))
    }

    /// The design of a stair drawn by its walking line.
    pub fn free_design(&self) -> StairDesign {
        self.design(
            if self.winders {
                StairForm::Winders
            } else {
                StairForm::Landings
            },
            0.0,
        )
    }

    /// The height a stair drawn on `level` climbs.
    pub fn height_on(&self, model: &Model, level: LevelId) -> f64 {
        match self.top_on(model, level) {
            WallTop::Level { level: to, offset } => {
                let z0 = model.level(level).map_or(0.0, |l| l.elevation);
                model
                    .level(to)
                    .map_or(self.height, |l| l.elevation + offset - z0)
            }
            WallTop::Unconnected => self.height,
        }
    }
}

impl Editor {
    /// Draws a stair along a walking line on the active level.
    pub fn add_stair(&mut self, s: &StairSettings, path: Vec<Point2>) -> EditResult<StairId> {
        self.add_designed_stair(s, path, s.free_design())
    }

    /// Draws a ready-made stair on the active level from its foot (a
    /// spiral's centre) and the direction it sets off in.
    pub fn add_preset_stair(
        &mut self,
        s: &StairSettings,
        foot: Point2,
        dir: Vec2,
    ) -> EditResult<StairId> {
        let (path, design) = s
            .preset_on(self.model(), self.level, foot, dir)
            .ok_or_else(|| {
                "no cabe: para esa altura hacen falta más peldaños (o menos ancho)".to_string()
            })?;
        self.add_designed_stair(s, path, design)
    }

    fn add_designed_stair(
        &mut self,
        s: &StairSettings,
        path: Vec<Point2>,
        design: StairDesign,
    ) -> EditResult<StairId> {
        let level = self.level;
        let top = s.top_on(self.model(), level);
        self.transact("Crear escalera", |m| {
            m.add_stair(NewStair {
                level,
                path,
                width: s.width,
                going: s.going,
                base_offset: 0.0,
                top,
                height: s.height,
                waist: s.waist,
                material: s.material,
                usage: s.usage,
                design,
            })
        })
    }

    pub fn edit_stair(
        &mut self,
        label: &str,
        id: StairId,
        edit: impl FnOnce(&mut Stair),
    ) -> EditResult {
        self.transact(label, |m| m.edit_stair(id, edit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stairs_with_the_editor() {
        let mut ed = Editor::new_project();
        let pb = ed.level;
        ed.add_level("Planta 1", 2.97).unwrap();
        ed.level = pb;
        let s = default_stair_settings(ed.model());
        assert!((s.height_on(ed.model(), pb) - 2.97).abs() < 1e-12);
        let id = ed
            .add_stair(&s, vec![Point2::new(0.0, 0.0), Point2::new(4.48, 0.0)])
            .unwrap();
        assert_eq!(ed.model().stair(id).unwrap().layout().risers(), 17);
        ed.edit_stair("Ancho", id, |s| s.width = 1.2).unwrap();
        assert!(ed.edit_stair("Ancho", id, |s| s.width = 0.0).is_err());
        ed.undo();
        assert_eq!(ed.model().stair(id).unwrap().width, 1.0);
        // Ready-made: a U with winders and a spiral round its newel, both
        // up to the floor above in 17 risers.
        let mut s = default_stair_settings(ed.model());
        s.preset = StairPreset::UShape;
        s.winders = true;
        let u = ed
            .add_preset_stair(&s, Point2::new(5.0, 0.0), Vec2::new(0.0, 1.0))
            .unwrap();
        let st = ed.model().stair(u).unwrap();
        assert_eq!(st.design.form, StairForm::Winders);
        assert_eq!(st.layout().risers(), 17);
        s.preset = StairPreset::Spiral;
        s.structure = StairStructure::Newel;
        s.width = 0.8;
        s.going = 0.25;
        let c = ed
            .add_preset_stair(&s, Point2::new(10.0, 0.0), Vec2::new(1.0, 0.0))
            .unwrap();
        let st = ed.model().stair(c).unwrap();
        assert_eq!(st.design.form, StairForm::Spiral);
        assert_eq!(st.design.structure, StairStructure::Newel);
        assert_eq!(st.layout().risers(), 17);
        // A U too short to turn is refused.
        s.preset = StairPreset::UShape;
        s.width = 2.0;
        s.top = TopChoice::Unconnected;
        s.height = 0.6;
        assert!(ed
            .add_preset_stair(&s, Point2::new(0.0, 8.0), Vec2::new(1.0, 0.0))
            .is_err());
    }
}
