//! The light frames (steel frame and timber frame) in the code check:
//! the member checks of `archisim-struct::lightframe` (EN 1993-1-3,
//! EN 1995-1-1 and DB-SE-M, with the actions and combinations of DB-SE
//! and DB-SE-AE), the shear walls and their hold-downs, and the fire
//! resistance of DB-SI 6 for the framed types; and the rest of the
//! structure (section Structure): concrete beams, columns and slabs from
//! the analysis, load-bearing masonry and the footings.

use archisim_core::Model;
use archisim_struct::lightframe::{self, Status as S, Subject as L};

use crate::{Item, Report, Section, Status, Subject};

/// The checks of the light frames of the model, added to the report.
pub fn check(model: &Model, report: &mut Report) {
    if model.slabs().next().is_none() && model.columns().next().is_none() {
        return;
    }
    let framed = model.framings().next().is_some();
    let frame = model.columns().next().is_some() || model.beams().next().is_some();
    let mut all: Vec<(Section, lightframe::Check)> = Vec::new();
    if frame {
        let a = archisim_struct::analysis::analyse(model);
        if framed {
            let lf = lightframe::check(model, &a.takedown, a.wind.as_ref());
            all.extend(lf.into_iter().map(|c| (Section::Framing, c)));
        }
        all.extend(
            archisim_struct::members::check(model, &a)
                .into_iter()
                .chain(archisim_struct::frame_members::check(model, &a))
                .chain(archisim_struct::connections::check(model, &a))
                .chain(archisim_struct::stability::check(model, &a))
                .map(|c| (Section::Structure, c)),
        );
        rest(model, &a.takedown, &mut all);
    } else {
        let td = archisim_struct::takedown(model);
        if framed {
            let wind = archisim_struct::wind::building_wind(model);
            let lf = lightframe::check(model, &td, wind.as_ref());
            all.extend(lf.into_iter().map(|c| (Section::Framing, c)));
        }
        rest(model, &td, &mut all);
    }
    for (section, c) in all {
        let (subject, level) = match c.subject {
            L::Wall(id) => (Subject::Wall(id), model.wall(id).map(|w| w.level)),
            L::Floor(id) => (Subject::Floor(id), model.slab(id).map(|s| s.level)),
            L::Roof(id) => (Subject::Roof(id), model.roof(id).map(|r| r.level)),
            L::Beam(id) => (Subject::Reinforced(id.0), model.beam(id).map(|b| b.level)),
            L::Column(id) => (Subject::Reinforced(id.0), model.column(id).map(|c| c.level)),
            L::Footing(id) => (Subject::Reinforced(id), None),
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
            section,
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

fn rest(
    model: &Model,
    td: &archisim_struct::Takedown,
    all: &mut Vec<(Section, lightframe::Check)>,
) {
    for c in archisim_struct::slab::check(model, td)
        .into_iter()
        .chain(archisim_struct::masonry::check(model, td))
        .chain(archisim_struct::foundations::check(model, td))
        .chain(archisim_struct::retaining::check(model))
    {
        all.push((Section::Structure, c));
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
