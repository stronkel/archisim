//! Plans of a refurbishment by phase, as drawn in Spain: the current state
//! ("estado actual"), the demolitions ("demoliciones", in yellow over the
//! current state) and the proposed state ("estado reformado", what is new
//! in red over what is kept).
//!
//! The plan of a phase is the plan of a copy of the model with what that
//! phase does not show taken out, and what it marks drawn in a solid
//! colour instead of its materials.

use archisim_core::{CutPattern, Layer, MemberUsage, Model, Phase, WallFunction};

/// Which phases a plan shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PhaseView {
    /// Everything, as modelled (no refurbishment colours).
    #[default]
    All,
    /// What there is now: existing and to be demolished.
    Current,
    /// What is kept, and in yellow what is demolished.
    Demolition,
    /// What is kept, and in red what is new.
    Proposed,
}

impl PhaseView {
    pub const ALL: [PhaseView; 4] = [
        PhaseView::All,
        PhaseView::Current,
        PhaseView::Demolition,
        PhaseView::Proposed,
    ];

    /// Its name in the interface.
    pub fn name(self) -> &'static str {
        match self {
            PhaseView::All => "Todo",
            PhaseView::Current => "Estado actual",
            PhaseView::Demolition => "Demoliciones",
            PhaseView::Proposed => "Estado reformado",
        }
    }

    /// Whether an element of `phase` shows, and whether it is marked.
    fn shows(self, phase: Phase) -> (bool, bool) {
        match (self, phase) {
            (PhaseView::All, _) => (true, false),
            (PhaseView::Current, Phase::New) => (false, false),
            (PhaseView::Current, _) => (true, false),
            (PhaseView::Demolition, Phase::New) => (false, false),
            (PhaseView::Demolition, Phase::Demolished) => (true, true),
            (PhaseView::Demolition, Phase::Existing) => (true, false),
            (PhaseView::Proposed, Phase::Demolished) => (false, false),
            (PhaseView::Proposed, Phase::New) => (true, true),
            (PhaseView::Proposed, Phase::Existing) => (true, false),
        }
    }

    /// The colour that marks elements in this view.
    pub fn mark_colour(self) -> [u8; 3] {
        match self {
            PhaseView::Demolition => DEMOLITION,
            _ => NEW,
        }
    }
}

/// Demolitions: yellow.
pub const DEMOLITION: [u8; 3] = [246, 214, 40];
/// New work: red.
pub const NEW: [u8; 3] = [222, 74, 60];

/// The model a plan of `view` draws; `None` for [`PhaseView::All`] (the
/// model itself).
pub fn phase_model(model: &Model, view: PhaseView) -> Option<Model> {
    if view == PhaseView::All {
        return None;
    }
    let mut m = model.clone();
    let colour = view.mark_colour();
    let mark = m.add_material("(fase)", "phase", colour, CutPattern::Solid);
    let solid = |layers: &[Layer]| -> Vec<Layer> {
        layers
            .iter()
            .map(|l| Layer {
                material: mark,
                ..l.clone()
            })
            .collect()
    };

    // Doors and windows first, then what hosts them.
    let openings: Vec<_> = m.openings().map(|o| o.id).collect();
    for o in openings {
        if !view.shows(m.phase(o.0)).0 {
            let _ = m.remove_opening(o);
        }
    }
    let walls: Vec<_> = m.walls().map(|w| (w.id, w.wall_type)).collect();
    for (w, t) in walls {
        let (show, marked) = view.shows(m.phase(w.0));
        if !show {
            let _ = m.remove_wall(w);
        } else if marked {
            let layers = m.wall_type(t).map(|t| solid(&t.layers)).unwrap_or_default();
            if let Ok(nt) =
                m.add_wall_type(format!("(fase) {}", w.0), WallFunction::Interior, layers)
            {
                let _ = m.set_wall_type(w, nt);
            }
        }
    }
    let slabs: Vec<_> = m.slabs().map(|s| (s.id, s.slab_type)).collect();
    for (s, t) in slabs {
        let (show, marked) = view.shows(m.phase(s.0));
        if !show {
            let _ = m.remove_slab(s);
        } else if marked {
            let Some(st) = m.slab_type(t).cloned() else {
                continue;
            };
            if let Ok(nt) =
                m.add_slab_type(format!("(fase) {}", s.0), st.function, solid(&st.layers))
            {
                let _ = m.set_slab_type(s, nt);
            }
        }
    }
    let stairs: Vec<_> = m.stairs().map(|s| s.id).collect();
    for s in stairs {
        let (show, marked) = view.shows(m.phase(s.0));
        if !show {
            let _ = m.remove_stair(s);
        } else if marked {
            let _ = m.edit_stair(s, |s| s.material = mark);
        }
    }
    let curtains: Vec<_> = m.curtain_walls().map(|c| c.id).collect();
    for c in curtains {
        let (show, marked) = view.shows(m.phase(c.0));
        if !show {
            let _ = m.remove_curtain_wall(c);
        } else if marked {
            // A copy of its type, all of the phase's colour.
            let Some(t) = m.curtain_wall(c).map(|c| c.curtain_type) else {
                continue;
            };
            let Some(mut design) = m.curtain_type(t).map(|t| t.design.clone()) else {
                continue;
            };
            for p in [&mut design.border, &mut design.mullion, &mut design.transom] {
                p.material = mark;
            }
            for p in &mut design.panels {
                p.material = mark;
            }
            if let Some(x) = &mut design.caps {
                x.material = mark;
            }
            if let Some(x) = &mut design.louvres {
                x.material = mark;
            }
            if let Ok(copy) = m.add_curtain_type("", design) {
                let _ = m.edit_curtain_wall(c, |c| c.curtain_type = copy);
            }
        }
    }
    let railings: Vec<_> = m.railings().map(|r| r.id).collect();
    for r in railings {
        let (show, marked) = view.shows(m.phase(r.0));
        if !show {
            let _ = m.remove_railing(r);
        } else if marked {
            let _ = m.edit_railing(r, |r| r.material = mark);
        }
    }
    let roofs: Vec<_> = m.roofs().map(|r| (r.id, r.roof_type)).collect();
    for (r, t) in roofs {
        let (show, marked) = view.shows(m.phase(r.0));
        if !show {
            let _ = m.remove_roof(r);
        } else if marked {
            let Some(st) = m.slab_type(t).cloned() else {
                continue;
            };
            if let Ok(nt) =
                m.add_slab_type(format!("(fase) {}", r.0), st.function, solid(&st.layers))
            {
                let _ = m.edit_roof(r, |r| r.roof_type = nt);
            }
        }
    }
    let columns: Vec<_> = m.columns().map(|c| (c.id, c.member_type)).collect();
    for (c, t) in columns {
        let (show, marked) = view.shows(m.phase(c.0));
        if !show {
            let _ = m.remove_column(c);
        } else if marked {
            if let Some(profile) = m.member_type(t).map(|t| t.profile) {
                if let Ok(nt) = m.add_member_type(
                    format!("(fase) {}", c.0),
                    MemberUsage::Column,
                    mark,
                    profile,
                ) {
                    let _ = m.edit_column(c, |c| c.member_type = nt);
                }
            }
        }
    }
    let beams: Vec<_> = m.beams().map(|b| (b.id, b.member_type)).collect();
    for (b, t) in beams {
        let (show, marked) = view.shows(m.phase(b.0));
        if !show {
            let _ = m.remove_beam(b);
        } else if marked {
            if let Some(profile) = m.member_type(t).map(|t| t.profile) {
                if let Ok(nt) =
                    m.add_member_type(format!("(fase) {}", b.0), MemberUsage::Beam, mark, profile)
                {
                    let _ = m.edit_beam(b, |b| b.member_type = nt);
                }
            }
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{LayerFunction, NewWall, Point2, ReferenceLine, WallTop};

    #[test]
    fn each_view_shows_its_phases() {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.24,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = |m: &mut Model, y: f64| {
            m.add_wall(NewWall {
                wall_type: wt,
                level: l,
                start: Point2::new(0.0, y),
                end: Point2::new(5.0, y),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap()
        };
        let new = wall(&mut m, 0.0);
        let existing = wall(&mut m, 3.0);
        let gone = wall(&mut m, 6.0);
        m.set_phase(existing.0, Phase::Existing).unwrap();
        m.set_phase(gone.0, Phase::Demolished).unwrap();
        assert!(phase_model(&m, PhaseView::All).is_none());
        let current = phase_model(&m, PhaseView::Current).unwrap();
        assert!(current.wall(new).is_none() && current.wall(gone).is_some());
        let demo = phase_model(&m, PhaseView::Demolition).unwrap();
        let t = demo.wall_type(demo.wall(gone).unwrap().wall_type).unwrap();
        assert_eq!(
            demo.material(t.layers[0].material).unwrap().cut_color,
            DEMOLITION
        );
        // Kept walls keep their materials.
        assert_eq!(demo.wall(existing).unwrap().wall_type, wt);
        let proposed = phase_model(&m, PhaseView::Proposed).unwrap();
        assert!(proposed.wall(gone).is_none());
        let t = proposed
            .wall_type(proposed.wall(new).unwrap().wall_type)
            .unwrap();
        assert_eq!(
            proposed.material(t.layers[0].material).unwrap().cut_color,
            NEW
        );
    }
}
