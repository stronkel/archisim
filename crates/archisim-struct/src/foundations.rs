//! The foundations (DB-SE-C, 4): the footing each column and each
//! load-bearing wall needs for what it brings to the ground, against the
//! admissible pressure of the site, and the check of the footings drawn.
//! Centred loads (the moments at the column bases are not taken), the
//! footing and the earth over it as 10 % more; rigid footings.

use std::collections::BTreeMap;

use archisim_core::{FootingKind, Model};

use crate::cases::Case;
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

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let sigma = model.structural_site().bearing.max(1.0);
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
                        rigid(f.width.max(length), f.depth, 0.3, f.id.0, &name, &mut out);
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
