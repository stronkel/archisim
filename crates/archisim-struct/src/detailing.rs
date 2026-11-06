//! The construction rules of NCSE-02, chapter 4, for concrete buildings
//! of low ductility (μ = 2, the one the calculation takes), set against
//! the reinforcement drawn:
//!
//! - beams, with ac ≥ 0,16g (4.5.1.1): two Ø16 at least on each face
//!   along the whole length, a ratio of 0,4 % between both, and stirrups
//!   at 10 cm at most by the supports;
//! - columns, with ac ≥ 0,12g (4.5.1.2): the smaller side 25 cm (30 with
//!   ac ≥ 0,16g), three bars a face (six in a round one) at 20 cm at most
//!   (15), a ratio between 1 % and 6 % with ac ≥ 0,16g, and in the
//!   critical zones at both ends, two sides long, ties of Ø6 at c/3 and
//!   10 cm with bars of 12–14 or 15 cm with 16 and more (Ø8, c/4, 7 and
//!   10 cm with ac ≥ 0,16g);
//! - the ties of the foundation (4.3.2): with ac ≥ 0,16g every pad tied
//!   by tie beams in two directions; between 0,08g and 0,16g the slab on
//!   the ground may tie them if it is continuous and 15 cm thick.

use archisim_core::{FootingKind, Model, Point2, Profile, Reinforcement};

use crate::analysis::Analysis;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

fn cm(x: f64) -> String {
    dec(x * 100.0, 0)
}

/// What a set of failed rules gives: a pass or a fail with the list.
fn verdict(
    subject: Subject,
    what: String,
    rule: &'static str,
    failed: Vec<String>,
    out: &mut Vec<Check>,
) {
    let ok = failed.is_empty();
    out.push(Check {
        subject,
        what: if ok {
            what
        } else {
            format!("{what}: {}", failed.join("; "))
        },
        rule,
        ratio: if ok { 0.0 } else { 1.1 },
        status: if ok { Status::Pass } else { Status::Fail },
        advice: (!ok).then(|| "cambiar el armado dibujado o la sección".into()),
    });
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let mut out = Vec::new();
    let q = crate::seismic::quake(model, &an.takedown);
    if !q.applies || q.ac < 0.08 {
        return out;
    }
    let ac = q.ac;
    let strong = ac >= 0.16;
    for (owner, kind) in &an.model.kinds {
        if *kind != Kind::Concrete {
            continue;
        }
        match owner {
            crate::analytical::Owner::Beam(id) if strong => {
                let Some(t) = model
                    .beam(*id)
                    .and_then(|b| model.member_type(b.member_type))
                else {
                    continue;
                };
                let Profile::Rectangle { width, depth } = t.profile else {
                    continue;
                };
                let Some(Reinforcement::Beam {
                    bottom,
                    top,
                    stirrups,
                    ..
                }) = model.reinforcement(id.0)
                else {
                    continue;
                };
                let mut failed = Vec::new();
                for (face, bars) in [("abajo", bottom), ("arriba", top)] {
                    if bars.count < 2 || bars.diameter < 0.016 - 1e-6 {
                        failed.push(format!("{} {face}, menos de 2Ø16", bars.label()));
                    }
                }
                let ratio = (bottom.area() + top.area()) / (width * depth);
                if ratio < 0.004 {
                    failed.push(format!(
                        "cuantía {} ‰, menos del 4 ‰",
                        dec(ratio * 1000.0, 1)
                    ));
                }
                if stirrups.spacing > 0.10 + 1e-6 {
                    failed.push(format!(
                        "cercos a {} cm, más de 10 cm junto a los apoyos (en {} cm)",
                        cm(stirrups.spacing),
                        cm(2.0 * depth)
                    ));
                }
                verdict(
                    Subject::Beam(*id),
                    format!(
                        "{}: {} abajo, {} arriba, cercos {}",
                        t.name,
                        bottom.label(),
                        top.label(),
                        stirrups.label()
                    ),
                    "Armado sísmico de vigas (NCSE-02, 4.5.1.1)",
                    failed,
                    &mut out,
                );
            }
            crate::analytical::Owner::Column(id) if ac >= 0.12 => {
                let Some(t) = model
                    .column(*id)
                    .and_then(|c| model.member_type(c.member_type))
                else {
                    continue;
                };
                let (round, b, h) = match t.profile {
                    Profile::Rectangle { width, depth } => {
                        (false, width.min(depth), width.max(depth))
                    }
                    Profile::Circle { diameter } => (true, diameter, diameter),
                    _ => continue,
                };
                let mut failed = Vec::new();
                let least = if strong { 0.30 } else { 0.25 };
                if b < least - 1e-6 {
                    failed.push(format!(
                        "lado menor {} cm, menos de {} cm",
                        cm(b),
                        cm(least)
                    ));
                }
                let Some(Reinforcement::Column {
                    cover,
                    bars,
                    ties,
                    dense,
                }) = model.reinforcement(id.0)
                else {
                    // Only the size can be checked.
                    verdict(
                        Subject::Column(*id),
                        format!("{}: sin armado dibujado, solo la sección", t.name),
                        "Armado sísmico de pilares (NCSE-02, 4.5.1.2)",
                        failed,
                        &mut out,
                    );
                    continue;
                };
                // Bars a face and their spacing.
                let inner =
                    |side: f64| (side - 2.0 * (cover + ties.diameter) - bars.diameter).max(0.0);
                let (enough, gap) = if round {
                    let ring = std::f64::consts::PI * inner(b);
                    (bars.count >= 6, ring / f64::from(bars.count.max(1)))
                } else {
                    // Corners first, then the rest shared out by the sides.
                    let per_face = 2 + (bars.count.saturating_sub(4)) / 4;
                    (bars.count >= 8, inner(h) / f64::from(per_face.max(2) - 1))
                };
                if !enough {
                    failed.push(format!(
                        "{}, menos de {}",
                        bars.label(),
                        if round {
                            "seis barras"
                        } else {
                            "tres barras por cara"
                        }
                    ));
                }
                let gap_max = if strong { 0.15 } else { 0.20 };
                if gap > gap_max + 1e-6 {
                    failed.push(format!(
                        "barras a {} cm, más de {} cm",
                        cm(gap),
                        cm(gap_max)
                    ));
                }
                if strong {
                    let area = if round {
                        std::f64::consts::PI * b * b / 4.0
                    } else {
                        b * h
                    };
                    let rho = bars.area() / area;
                    if !(0.01..=0.06).contains(&rho) {
                        failed.push(format!(
                            "cuantía {} %, fuera de 1 % – 6 %",
                            dec(rho * 100.0, 2)
                        ));
                    }
                }
                // The critical zones at both ends.
                let (tie_min, frac, near) = if strong {
                    (0.008, 4.0, if bars.diameter < 0.016 { 0.07 } else { 0.10 })
                } else {
                    (0.006, 3.0, if bars.diameter < 0.016 { 0.10 } else { 0.15 })
                };
                let s_max = (b / frac).min(near);
                let (s, len) = dense.unwrap_or((ties.spacing, f64::INFINITY));
                if ties.diameter < tie_min - 1e-6 {
                    failed.push(format!(
                        "cercos Ø{}, menos de Ø{}",
                        dec(ties.diameter * 1000.0, 0),
                        dec(tie_min * 1000.0, 0)
                    ));
                }
                if s > s_max + 1e-6 {
                    failed.push(format!(
                        "cercos a {} cm en los extremos, más de {} cm",
                        cm(s),
                        cm(s_max)
                    ));
                }
                if len < 2.0 * h - 1e-6 {
                    failed.push(format!(
                        "zona de cercos juntos de {} cm, menos de dos cantos ({} cm)",
                        cm(len),
                        cm(2.0 * h)
                    ));
                }
                verdict(
                    Subject::Column(*id),
                    format!(
                        "{}: {}, cercos {}{}",
                        t.name,
                        bars.label(),
                        ties.label(),
                        dense.map_or(String::new(), |(s, l)| format!(
                            " (a {} cm en {} cm de cada extremo)",
                            cm(s),
                            cm(l)
                        ))
                    ),
                    "Armado sísmico de pilares (NCSE-02, 4.5.1.2)",
                    failed,
                    &mut out,
                );
            }
            _ => {}
        }
    }
    ties(model, ac, &mut out);
    out
}

/// The ties of the pad footings (NCSE-02, 4.3.2).
fn ties(model: &Model, ac: f64, out: &mut Vec<Check>) {
    let beams: Vec<(Point2, Point2)> = model
        .footings()
        .filter_map(|f| match f.kind {
            FootingKind::TieBeam { start, end } => Some((start, end)),
            _ => None,
        })
        .collect();
    let pads: Vec<_> = model
        .footings()
        .filter_map(|f| match f.kind {
            FootingKind::Pad {
                position, length, ..
            } => Some((f, position, 0.5 * length.max(f.width) + 0.15)),
            _ => None,
        })
        .collect();
    if pads.len() < 2 {
        return;
    }
    for (f, at, reach) in pads {
        let dirs: Vec<Point2> = beams
            .iter()
            .filter(|(a, b)| a.distance(at) <= reach || b.distance(at) <= reach)
            .filter_map(|(a, b)| (*b - *a).normalized())
            .collect();
        let both = dirs
            .iter()
            .any(|u| dirs.iter().any(|v| u.cross(*v).abs() > 0.5));
        let (status, advice) = if both {
            (Status::Pass, None)
        } else if ac >= 0.16 {
            (
                Status::Fail,
                Some("vigas de atado en las dos direcciones hasta las zapatas vecinas".into()),
            )
        } else {
            (
                Status::Warn,
                Some(
                    "vigas de atado, o una solera continua de 15 cm al menos (y 1/50 de la luz) que haga de atado"
                        .into(),
                ),
            )
        };
        out.push(Check {
            subject: Subject::Footing(f.id.0),
            what: format!(
                "zapata aislada con ac {}g: {}",
                dec(ac, 3),
                match (dirs.len(), both) {
                    (_, true) => "atada en dos direcciones".to_string(),
                    (0, _) => "sin vigas de atado".to_string(),
                    _ => "atada en una sola dirección".to_string(),
                }
            ),
            rule: "Atado de la cimentación (NCSE-02, 4.3.2)",
            ratio: if both { 0.0 } else { 1.1 },
            status,
            advice,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn seismic_rules() {
        let mut m = Model::new("Sismo");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut cols = Vec::new();
        for x in [0.0, 5.0] {
            cols.push(
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(x, 0.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap(),
            );
        }
        let beam = m
            .add_beam(NewBeam {
                member_type: lib.beam,
                level: l1,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                top_offset: 0.0,
            })
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                Point2::new(0.0, -2.0),
                Point2::new(5.0, -2.0),
                Point2::new(5.0, 2.0),
                Point2::new(0.0, 2.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        m.set_reinforcement(beam.0, Some(Reinforcement::default_for(RebarKind::Beam)))
            .unwrap();
        for c in &cols {
            m.set_reinforcement(c.0, Some(Reinforcement::default_for(RebarKind::Column)))
                .unwrap();
        }
        // No earthquake: nothing.
        let an = crate::analysis::analyse(&m);
        if !crate::seismic::quake(&m, &an.takedown).applies {
            assert!(check(&m, &an).is_empty());
        }
        // Granada: ab 0,23g.
        let mut site = *m.structural_site();
        site.seismic_ab = 0.23;
        m.set_structural_site(site).unwrap();
        let an = crate::analysis::analyse(&m);
        let q = crate::seismic::quake(&m, &an.takedown);
        assert!(q.applies && q.ac >= 0.16, "{q:?}");
        let c = check(&m, &an);
        let col = c
            .iter()
            .find(|c| c.subject == Subject::Column(cols[0]))
            .unwrap_or_else(|| panic!("{c:?}"));
        // The template's 4 bars are not three a face.
        assert_eq!(col.status, Status::Fail, "{}", col.what);
        assert!(col.what.contains("tres barras"), "{}", col.what);
        assert!(c.iter().any(|c| c.subject == Subject::Beam(beam)), "{c:?}");
        // A column as the rules ask: 8Ø16 with Ø8 ties at 7 cm along
        // 60 cm.
        let t = m.member_type(lib.column).unwrap().clone();
        m.edit_member_type(
            lib.column,
            "Pilar 30×30",
            t.material,
            Profile::Rectangle {
                width: 0.30,
                depth: 0.30,
            },
        )
        .unwrap();
        m.set_reinforcement(
            cols[0].0,
            Some(Reinforcement::Column {
                cover: 0.03,
                bars: Counted::new(8, 0.016),
                ties: Spaced::new(0.008, 0.15),
                dense: Some((0.07, 0.60)),
            }),
        )
        .unwrap();
        let an = crate::analysis::analyse(&m);
        let c = check(&m, &an);
        let col = c
            .iter()
            .find(|c| c.subject == Subject::Column(cols[0]))
            .unwrap();
        assert_eq!(col.status, Status::Pass, "{}", col.what);
    }
}
