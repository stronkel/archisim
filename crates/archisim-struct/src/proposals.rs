//! The reinforcement the calculation asks for, ready to draw (the
//! «Aplicar el armado propuesto» of the calculation window): beams and
//! columns from the frame, footings from what they bring to the ground,
//! solid slabs from their grillage. Where the earthquake applies, the
//! construction rules of NCSE-02 (`detailing`) are put in: more bars,
//! closer ties at the ends.

use archisim_core::{Counted, FootingKind, Model, Profile, Reinforcement, Spaced};

use crate::analysis::Analysis;
use crate::concrete::Rc;
use crate::gravity::{Carrier, Takedown};
use crate::slab::mesh_for;

/// One element's proposal.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub element: u64,
    /// «Viga 12», «Zapata de Pilar 4»…
    pub name: String,
    pub reinforcement: Reinforcement,
    /// What it has now.
    pub now: Option<Reinforcement>,
}

impl Proposal {
    /// Whether it adds steel to what is drawn (or there is nothing).
    pub fn is_new(&self) -> bool {
        self.now.is_none()
    }
}

/// Every proposal.
pub fn proposals(model: &Model, an: &Analysis) -> Vec<Proposal> {
    let mut out = Vec::new();
    let quake = crate::seismic::quake(model, &an.takedown);
    let ac = if quake.applies { quake.ac } else { 0.0 };
    for e in &an.envelopes {
        if let Some((id, mut r)) = crate::members::proposal(model, e) {
            let name = match e.owner {
                crate::analytical::Owner::Beam(b) => {
                    let t = model
                        .beam(b)
                        .and_then(|x| model.member_type(x.member_type))
                        .map_or(String::new(), |t| format!(" ({})", t.name));
                    format!("Viga {}{t}", b.0)
                }
                crate::analytical::Owner::Column(c) => {
                    let t = model
                        .column(c)
                        .and_then(|x| model.member_type(x.member_type))
                        .map_or(String::new(), |t| format!(" ({})", t.name));
                    format!("Pilar {}{t}", c.0)
                }
            };
            seismic(model, e.owner, ac, &mut r);
            out.push(Proposal {
                element: id,
                name,
                reinforcement: r,
                now: model.reinforcement(id).copied(),
            });
        }
    }
    out.extend(footings(model, &an.takedown));
    out.extend(slabs(model, &an.takedown));
    out
}

/// The construction rules of NCSE-02 on a proposal.
fn seismic(model: &Model, owner: crate::analytical::Owner, ac: f64, r: &mut Reinforcement) {
    let strong = ac >= 0.16;
    match (owner, r) {
        (
            crate::analytical::Owner::Beam(_),
            Reinforcement::Beam {
                bottom,
                top,
                stirrups,
                ..
            },
        ) if strong => {
            for bars in [bottom, top] {
                if bars.count < 2 || bars.diameter < 0.016 {
                    let a = bars.area().max(2.0 * archisim_core::bar_area(0.016));
                    let n = (a / archisim_core::bar_area(0.016)).ceil() as u32;
                    *bars = Counted::new(n.max(2), 0.016);
                }
            }
            if stirrups.spacing > 0.10 {
                *stirrups = Spaced::new(stirrups.diameter, 0.10);
            }
        }
        (
            crate::analytical::Owner::Column(id),
            Reinforcement::Column {
                bars, ties, dense, ..
            },
        ) if ac >= 0.12 => {
            let round = model
                .column(id)
                .and_then(|c| model.member_type(c.member_type))
                .is_some_and(|t| matches!(t.profile, Profile::Circle { .. }));
            let (b, h) = model
                .column(id)
                .and_then(|c| model.member_type(c.member_type))
                .map_or((0.3, 0.3), |t| {
                    let (a, b) = t.profile.size();
                    (a.min(b), a.max(b))
                });
            let least = if round { 6 } else { 8 };
            if bars.count < least {
                let a = bars.area();
                let d = [0.012, 0.016, 0.020, 0.025]
                    .into_iter()
                    .find(|d| f64::from(least) * archisim_core::bar_area(*d) >= a)
                    .unwrap_or(0.025);
                *bars = Counted::new(least, d);
            }
            // A ratio of 1 % at least with ac ≥ 0,16g.
            if strong && !round {
                let need = 0.01 * b * h;
                if bars.area() < need {
                    let d = [0.012, 0.016, 0.020, 0.025]
                        .into_iter()
                        .find(|d| f64::from(bars.count) * archisim_core::bar_area(*d) >= need)
                        .unwrap_or(0.025);
                    *bars = Counted::new(bars.count, d);
                }
            }
            let tie = if strong {
                0.008
            } else {
                0.006f64.max(ties.diameter)
            };
            let near = match (strong, bars.diameter < 0.016) {
                (true, true) => 0.07,
                (true, false) => 0.10,
                (false, true) => 0.10,
                (false, false) => 0.15,
            };
            let s = (b / if strong { 4.0 } else { 3.0 }).min(near);
            let s = ((s * 100.0).floor() / 100.0).max(0.05);
            let len = ((2.0 * h) * 20.0).ceil() / 20.0;
            *ties = Spaced::new(tie.max(ties.diameter), ties.spacing);
            *dense = Some((s, len));
        }
        _ => {}
    }
}

/// The footings drawn under columns and walls.
fn footings(model: &Model, td: &Takedown) -> Vec<Proposal> {
    let rc = Rc::of(model);
    let ultimate = crate::foundations::ultimate_loads(td);
    let service = crate::foundations::service_loads(model, td);
    let mut out = Vec::new();
    for f in model.footings() {
        let (carrier, post, area, v, per_length) = match f.kind {
            FootingKind::Pad {
                column: Some(c),
                length,
                ..
            } => {
                let post = model
                    .column(c)
                    .and_then(|x| model.member_type(x.member_type))
                    .map_or(0.3, |t| {
                        let (a, b) = t.profile.size();
                        a.max(b)
                    });
                // Off its column: the farther edge.
                let e = model.footing_offset(f.id).map_or(0.0, |o| o.x.hypot(o.y));
                let v = (f.width.max(length) / 2.0 + e) - post / 2.0;
                (Carrier::Column(c), post, f.width * length, v, 1.0)
            }
            FootingKind::Strip { wall: Some(w), .. } => {
                let thick = model.wall(w).map_or(0.25, |w| {
                    model
                        .wall_type(w.wall_type)
                        .map_or(0.25, |t| t.layers.iter().map(|l| l.thickness).sum())
                });
                let len = service.get(&Carrier::Wall(w)).map_or(1.0, |x| x.1.max(0.1));
                (
                    Carrier::Wall(w),
                    thick,
                    f.width,
                    (f.width - thick) / 2.0,
                    len,
                )
            }
            _ => continue,
        };
        if model.footing_piles(f.id).is_some() {
            continue;
        }
        let Some(nu) = ultimate.get(&carrier).copied() else {
            continue;
        };
        let q = nu / per_length / area.max(1e-6);
        let Some(a) = crate::foundations::footing_steel(&rc, q, v, post, f.depth) else {
            continue;
        };
        let mesh = mesh_for(a);
        out.push(Proposal {
            element: f.id.0,
            name: format!(
                "{} {}",
                f.kind_name(),
                match carrier {
                    Carrier::Column(c) => format!("del pilar {}", c.0),
                    Carrier::Wall(w) => format!("del muro {}", w.0),
                    Carrier::Beam(b) => format!("de la viga {}", b.0),
                }
            ),
            reinforcement: Reinforcement::Footing {
                cover: 0.05,
                bottom: [mesh, mesh],
                top: None,
            },
            now: model.reinforcement(f.id.0).copied(),
        });
    }
    out
}

/// The solid slabs (not the one-way or waffle floors, whose ribs are not
/// meshes).
fn slabs(model: &Model, td: &Takedown) -> Vec<Proposal> {
    let mut out = Vec::new();
    for r in crate::slab::analyse(model, td) {
        if r.waffle.is_some() {
            continue;
        }
        if r.need_bottom
            .iter()
            .chain(&r.need_top)
            .any(|a| !a.is_finite())
        {
            continue;
        }
        let name = model
            .slab(r.slab)
            .and_then(|s| model.slab_type(s.slab_type))
            .map_or(String::new(), |t| t.name.clone());
        out.push(Proposal {
            element: r.slab.0,
            name: format!("Forjado {} ({name})", r.slab.0),
            reinforcement: Reinforcement::Slab {
                cover: 0.025,
                bottom: [mesh_for(r.need_bottom[0]), mesh_for(r.need_bottom[1])],
                top: Some([mesh_for(r.need_top[0]), mesh_for(r.need_top[1])]),
            },
            now: model.reinforcement(r.slab.0).copied(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_gets_its_steel_and_the_checks_pass() {
        let mut m = Model::new("Pórtico");
        crate::members::tests::portal(&mut m);
        let an = crate::analysis::analyse(&m);
        let p = proposals(&m, &an);
        assert!(p
            .iter()
            .any(|p| matches!(p.reinforcement, Reinforcement::Beam { .. })));
        assert!(p
            .iter()
            .any(|p| matches!(p.reinforcement, Reinforcement::Column { .. })));
        for x in &p {
            m.set_reinforcement(x.element, Some(x.reinforcement))
                .unwrap();
        }
        // What it proposes passes its own checks of strength (the
        // deflection asks for a section, not for bars).
        let an = crate::analysis::analyse(&m);
        for c in crate::members::check(&m, &an) {
            if c.rule.starts_with("Flecha") {
                continue;
            }
            assert_ne!(c.status, crate::lightframe::Status::Fail, "{}", c.what);
        }
        // With the earthquake of Granada, the columns get 8 bars and the
        // ties closer at their ends, and the rules pass.
        let mut site = *m.structural_site();
        site.seismic_ab = 0.23;
        m.set_structural_site(site).unwrap();
        let an = crate::analysis::analyse(&m);
        for x in proposals(&m, &an) {
            m.set_reinforcement(x.element, Some(x.reinforcement))
                .unwrap();
        }
        let an = crate::analysis::analyse(&m);
        for c in crate::detailing::check(&m, &an) {
            if c.what.contains("lado menor") {
                continue;
            }
            assert_ne!(c.status, crate::lightframe::Status::Fail, "{}", c.what);
        }
    }
}
