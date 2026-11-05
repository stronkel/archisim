//! The light frames (steel frame and timber frame) in the code check:
//! the member checks of `archisim-struct::lightframe` (EN 1993-1-3,
//! EN 1995-1-1 and DB-SE-M, with the actions and combinations of DB-SE
//! and DB-SE-AE), the shear walls and their hold-downs, and the fire
//! resistance of DB-SI 6 for the framed types.

use archisim_core::Model;
use archisim_struct::lightframe::{self, Status as S, Subject as L};

use crate::{Item, Report, Section, Status, Subject};

/// The checks of the light frames of the model, added to the report.
pub fn check(model: &Model, report: &mut Report) {
    if model.framings().next().is_none() {
        return;
    }
    let td = archisim_struct::takedown(model);
    let wind = archisim_struct::wind::building_wind(model);
    for c in lightframe::check(model, &td, wind.as_ref()) {
        let (subject, level) = match c.subject {
            L::Wall(id) => (Subject::Wall(id), model.wall(id).map(|w| w.level)),
            L::Floor(id) => (Subject::Floor(id), model.slab(id).map(|s| s.level)),
            L::Roof(id) => (Subject::Roof(id), model.roof(id).map(|r| r.level)),
            L::Building => (Subject::Building, None),
        };
        let status = match c.status {
            S::Pass => Status::Passes,
            S::Warn => Status::Warning,
            S::Fail => Status::Fails,
            S::Info => Status::Info,
        };
        let mut text = c.what.clone();
        if let Some(a) = &c.advice {
            text.push_str(&format!(". Propuesta: {a}"));
        }
        let (value, required) = if matches!(c.status, S::Info) || c.ratio == 0.0 {
            (String::new(), String::new())
        } else {
            (
                format!(
                    "aprovechamiento {}",
                    format!("{:.2}", c.ratio).replace('.', ",")
                ),
                "≤ 1,00".into(),
            )
        };
        report.items.push(Item {
            section: Section::Framing,
            status,
            subject,
            level,
            rule: c.rule,
            text,
            value,
            required,
            route: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_floor_without_a_support_fails_and_says_why() {
        use archisim_core::*;
        let mut m = Model::new("Forjado");
        let level = m.add_level("Planta", 3.0);
        let wool = m.add_material("Lana", "insulation", [230, 220, 120], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Forjado de madera",
                SlabFunction::Floor,
                vec![Layer {
                    material: wool,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_type_framing(st.0, Some(Framing::for_depth(FrameMaterial::Timber, 0.2)))
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(9.0, 0.0),
                Point2::new(9.0, 7.0),
                Point2::new(0.0, 7.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let mut report = Report::default();
        check(&m, &mut report);
        let item = report
            .items
            .iter()
            .find(|i| i.section == Section::Framing && i.status == Status::Fails)
            .unwrap();
        assert!(item.text.contains("apoyo intermedio"), "{}", item.text);
    }
}
