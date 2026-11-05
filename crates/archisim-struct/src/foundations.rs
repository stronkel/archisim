//! The foundations (DB-SE-C, 4): the footing each column and each
//! load-bearing wall needs for what it brings to the ground, against the
//! admissible pressure of the site, and the check of the footings drawn.
//! Centred loads (the moments at the column bases are not taken), the
//! footing and the earth over it as 10 % more; rigid footings. Their
//! bottom steel by bending at a section 0,15 of the column or wall
//! inside its face (Código Estructural, art. 58), with the ground's
//! ultimate reaction, against the mesh drawn or as a proposal.

use std::collections::BTreeMap;

use archisim_core::{FootingKind, Model};

use crate::cases::Case;
use crate::concrete::Rc;
use crate::gravity::{Carrier, Takedown};
use crate::lightframe::{Check, Status, Subject};
use crate::text::dec;

/// The footing's own weight and the earth over it, as a share of the load.
const OWN: f64 = 0.10;

/// What each carrier brings to the ground in service (G + Q + S, all of
/// it: the characteristic combination with the live load leading and
/// the others whole is on the safe side), kN; with its length for walls.
pub fn service_loads(model: &Model, td: &Takedown) -> BTreeMap<Carrier, (f64, f64)> {
    let mut out: BTreeMap<Carrier, (f64, f64)> = BTreeMap::new();
    for g in &td.ground {
        let Some(c) = g.from else { continue };
        if matches!(g.case, Case::Wind(_) | Case::Quake(_)) {
            continue;
        }
        let len = match c {
            Carrier::Wall(id) => model.wall(id).map_or(1.0, |w| w.start.distance(w.end)),
            _ => 0.0,
        };
        let e = out.entry(c).or_insert((0.0, len));
        e.0 += g.kn;
    }
    out
}

/// The same in the ultimate combination (1,35 G + 1,5 the rest), kN.
pub fn ultimate_loads(td: &Takedown) -> BTreeMap<Carrier, f64> {
    let mut out: BTreeMap<Carrier, f64> = BTreeMap::new();
    for g in &td.ground {
        let Some(c) = g.from else { continue };
        let f = match g.case {
            Case::Dead => 1.35,
            Case::Wind(_) | Case::Quake(_) => 0.0,
            _ => 1.5,
        };
        *out.entry(c).or_default() += f * g.kn;
    }
    out
}

/// The bottom steel of a footing, m²/m: net ultimate pressure `q`
/// (kN/m²) over the overhang `v` from the face of what it carries, of
/// width `post`, and its depth `h`.
pub fn footing_steel(rc: &Rc, q: f64, v: f64, post: f64, h: f64) -> Option<f64> {
    let lever = v.max(0.0) + 0.15 * post;
    let m = q * lever * lever / 2.0;
    let d = (h - 0.06).max(0.1);
    rc.bending_steel(m, 1.0, d).map(|a| a.max(0.0009 * h))
}

fn per_m(sp: &archisim_core::Spaced) -> f64 {
    archisim_core::bar_area(sp.diameter) / sp.spacing.max(0.01)
}

/// The steel check of a drawn footing.
#[allow(clippy::too_many_arguments)]
fn steel_check(
    model: &Model,
    f: &archisim_core::Footing,
    name: &str,
    nu: f64,
    area: f64,
    v: f64,
    post: f64,
    out: &mut Vec<Check>,
) {
    let rc = Rc::of(model);
    let q = nu / area.max(1e-6);
    let need = footing_steel(&rc, q, v, post, f.depth);
    let has = match model.reinforcement(f.id.0) {
        Some(archisim_core::Reinforcement::Footing { bottom, .. }) => {
            Some((per_m(&bottom[0]).min(per_m(&bottom[1])), bottom))
        }
        _ => None,
    };
    let (what, ratio, status, advice) = match (need, has) {
        (None, _) => (
            format!("{name}: el canto no basta a flexión"),
            2.0,
            Status::Fail,
            Some("más canto".to_string()),
        ),
        (Some(a), Some((h, b))) => {
            let u = a / h.max(1e-9);
            (
                format!(
                    "{name}: pide {} cm²/m abajo; tiene {} y {} ({} cm²/m)",
                    dec(a * 1e4, 2),
                    b[0].label(),
                    b[1].label(),
                    dec(h * 1e4, 2)
                ),
                u,
                if u <= 1.0 { Status::Pass } else { Status::Fail },
                (u > 1.0).then(|| {
                    format!(
                        "{} en las dos direcciones",
                        crate::slab::mesh_for(a).label()
                    )
                }),
            )
        }
        (Some(a), None) => (
            format!(
                "{name}: pide {} cm²/m abajo: {} en las dos direcciones",
                dec(a * 1e4, 2),
                crate::slab::mesh_for(a).label()
            ),
            0.0,
            Status::Info,
            Some("la zapata no tiene armado dibujado: este es el que pide".to_string()),
        ),
    };
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what,
        rule: "Armado de la zapata (Código Estructural, art. 58)",
        ratio,
        status,
        advice,
    });
}

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let sigma = model.structural_site().bearing.max(1.0);
    let ultimate = ultimate_loads(td);
    let loads = service_loads(model, td);
    let mut out = Vec::new();
    let mut drawn: BTreeMap<Carrier, &archisim_core::Footing> = BTreeMap::new();
    for f in model.footings() {
        match f.kind {
            FootingKind::Pad {
                column: Some(c), ..
            } => {
                drawn.insert(Carrier::Column(c), f);
            }
            FootingKind::Strip { wall: Some(w), .. } => {
                drawn.insert(Carrier::Wall(w), f);
            }
            _ => {}
        }
    }
    let rule = "Presión sobre el terreno (DB-SE-C, 4.3)";
    for (c, (kn, len)) in &loads {
        if *kn <= 0.0 {
            continue;
        }
        let n = kn * (1.0 + OWN);
        match c {
            Carrier::Column(id) => {
                let need = (n / sigma).sqrt();
                let side = (need * 20.0).ceil() / 20.0;
                let name = format!("Pilar {}", id.0);
                match drawn.get(c) {
                    Some(f) => {
                        let FootingKind::Pad { length, .. } = f.kind else {
                            continue;
                        };
                        let area = f.width * length;
                        let u = n / area.max(1e-6) / sigma;
                        out.push(Check {
                            subject: Subject::Footing(f.id.0),
                            what: format!(
                                "Zapata de {name}, {}×{}×{} m: {} kN, {} kN/m² sobre {} kN/m² admisibles",
                                dec(f.width, 2),
                                dec(length, 2),
                                dec(f.depth, 2),
                                dec(*kn, 1),
                                dec(n / area, 0),
                                dec(sigma, 0)
                            ),
                            rule,
                            ratio: u,
                            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                            advice: (u > 1.0).then(|| {
                                format!("zapata de {}×{} m", dec(side, 2), dec(side, 2))
                            }),
                        });
                        let post = model
                            .column(*id)
                            .and_then(|c| model.member_type(c.member_type))
                            .map_or(0.3, |t| {
                                let (a, b) = t.profile.size();
                                a.max(b)
                            });
                        rigid(f.width.max(length), f.depth, post, f.id.0, &name, &mut out);
                        let nu = ultimate.get(c).copied().unwrap_or(1.45 * kn);
                        let v = (f.width.max(length) - post) / 2.0;
                        steel_check(
                            model,
                            f,
                            &format!("Zapata de {name}"),
                            nu,
                            area,
                            v,
                            post,
                            &mut out,
                        );
                    }
                    None => out.push(Check {
                        subject: Subject::Footing(id.0),
                        what: format!(
                            "{name}: {} kN, pide una zapata de {}×{} m y {} m de canto",
                            dec(*kn, 1),
                            dec(side, 2),
                            dec(side, 2),
                            dec(((side - 0.3) / 4.0).max(0.4).ceil_to(0.05), 2)
                        ),
                        rule,
                        ratio: 0.0,
                        status: Status::Info,
                        advice: Some("el pilar no tiene zapata dibujada".into()),
                    }),
                }
            }
            Carrier::Wall(id) => {
                let q = n / len.max(0.1);
                let need = (q / sigma).max(0.4);
                let width = (need * 20.0).ceil() / 20.0;
                let name = format!("Muro {}", id.0);
                let thick = model.wall(*id).map_or(0.25, |w| {
                    model
                        .wall_type(w.wall_type)
                        .map_or(0.25, |t| t.layers.iter().map(|l| l.thickness).sum())
                });
                match drawn.get(c) {
                    Some(f) => {
                        let u = q / f.width.max(1e-6) / sigma;
                        out.push(Check {
                            subject: Subject::Footing(f.id.0),
                            what: format!(
                                "Zapata corrida de {name}, {} m de ancho y {} de canto: {} kN/m, {} kN/m² sobre {} admisibles",
                                dec(f.width, 2),
                                dec(f.depth, 2),
                                dec(kn / len.max(0.1), 1),
                                dec(q / f.width, 0),
                                dec(sigma, 0)
                            ),
                            rule,
                            ratio: u,
                            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                            advice: (u > 1.0).then(|| format!("{} m de ancho", dec(width, 2))),
                        });
                        rigid(f.width, f.depth, thick, f.id.0, &name, &mut out);
                        let nu = ultimate.get(c).copied().unwrap_or(1.45 * kn) / len.max(0.1);
                        let v = (f.width - thick) / 2.0;
                        steel_check(
                            model,
                            f,
                            &format!("Zapata corrida de {name}"),
                            nu,
                            f.width,
                            v,
                            thick,
                            &mut out,
                        );
                    }
                    None => {
                        // Only the walls that bear something on the ground:
                        // partitions on a slab on the ground have no footing.
                        if model.wall(*id).is_some_and(|w| {
                            model.wall_type(w.wall_type).is_some_and(|t| {
                                t.layers
                                    .iter()
                                    .any(|l| l.function == archisim_core::LayerFunction::Structure)
                            })
                        }) {
                            out.push(Check {
                                subject: Subject::Footing(id.0),
                                what: format!(
                                    "{name}: {} kN/m, pide una zapata corrida de {} m de ancho",
                                    dec(kn / len.max(0.1), 1),
                                    dec(width, 2)
                                ),
                                rule,
                                ratio: 0.0,
                                status: Status::Info,
                                advice: Some("el muro no tiene zapata dibujada".into()),
                            });
                        }
                    }
                }
            }
            Carrier::Beam(_) => {}
        }
    }
    out
}

/// A footing is rigid if its overhang is no more than twice its depth
/// (Código Estructural, art. 55); a flexible one needs its own design.
fn rigid(width: f64, depth: f64, post: f64, id: u64, name: &str, out: &mut Vec<Check>) {
    let v = ((width - post) / 2.0).max(0.0);
    let u = v / (2.0 * depth).max(1e-6);
    out.push(Check {
        subject: Subject::Footing(id),
        what: format!(
            "Zapata de {name}: vuelo {} m con {} m de canto",
            dec(v, 2),
            dec(depth, 2)
        ),
        rule: "Zapata rígida, vuelo ≤ 2 cantos (Código Estructural, art. 55)",
        ratio: u,
        status: if u <= 1.0 { Status::Pass } else { Status::Warn },
        advice: (u > 1.0)
            .then(|| format!("{} m de canto o calcularla como flexible", dec(v / 2.0, 2))),
    });
}

trait CeilTo {
    fn ceil_to(self, step: f64) -> f64;
}

impl CeilTo for f64 {
    fn ceil_to(self, step: f64) -> f64 {
        (self / step).ceil() * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn a_column_asks_for_its_footing() {
        let mut m = Model::new("Zapata");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut cols = Vec::new();
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (0.0, 5.0), (5.0, 5.0)] {
            cols.push(
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(x, y),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap(),
            );
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(5.0, 0.0),
                Point2::new(5.0, 5.0),
                Point2::new(0.0, 5.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        assert_eq!(c.len(), 4, "{c:?}");
        assert!(c.iter().all(|c| c.status == Status::Info));
        // A small footing under one: too much pressure.
        let concrete = m.materials().next().unwrap().id;
        m.add_footing(NewFooting {
            level: l0,
            kind: FootingKind::Pad {
                position: Point2::new(0.0, 0.0),
                length: 0.3,
                rotation: 0.0,
                column: Some(cols[0]),
            },
            material: concrete,
            width: 0.3,
            depth: 0.5,
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        let f = c
            .iter()
            .find(|c| c.rule.starts_with("Presión") && c.status != Status::Info)
            .unwrap();
        assert_eq!(f.status, Status::Fail, "{}", f.what);
    }
}
