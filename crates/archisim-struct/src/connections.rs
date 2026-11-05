//! The joints of the steel frame (EN 1993-1-8, DB-SE-A 8):
//!
//! - the base plates of the steel columns that stand on a footing: plate
//!   100 mm round the profile, bearing on the concrete fjd = βj·kj·fcd
//!   (2/3 · 1,5), its thickness for that pressure over the overhang, the
//!   anchor bolts (4.6) for the uplift that the bending leaves and the
//!   shear by friction (0,2) and by the bolts;
//! - the shear joints of the ends of the steel beams (hinged): a fin
//!   plate with a row of 8.8 bolts in single shear, as many as the end
//!   shear asks for, and whether they fit in the web.

use archisim_core::{Model, Profile};

use crate::analysis::Analysis;
use crate::analytical::Owner;
use crate::cases::Kind as ComboKind;
use crate::concrete::Rc;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

pub const GAMMA_M2: f64 = 1.25;

/// Stressed area of a bolt (ISO 898), m².
pub fn bolt_area(d: f64) -> f64 {
    match (d * 1000.0).round() as i64 {
        12 => 84.3e-6,
        16 => 157e-6,
        20 => 245e-6,
        24 => 353e-6,
        27 => 459e-6,
        30 => 561e-6,
        _ => std::f64::consts::PI / 4.0 * (0.85 * d).powi(2),
    }
}

/// Ultimate tensile strength of a structural steel, N/mm².
fn steel_fu(fy: f64) -> f64 {
    if fy <= 240.0 {
        360.0
    } else if fy <= 280.0 {
        430.0
    } else {
        490.0
    }
}

/// The base plate of a column `b` × `h` (m) for an axial force `n`
/// (compression positive), moments and shear, kN and kN·m.
#[derive(Debug, Clone, PartialEq)]
pub struct BasePlate {
    pub width: f64,
    pub length: f64,
    pub thickness: f64,
    pub bolts: u32,
    pub bolt: f64,
    /// Utilisations: bearing, anchors in tension, shear.
    pub bearing: f64,
    pub tension: f64,
    pub shear: f64,
}

pub fn base_plate(rc: &Rc, fy: f64, b: f64, h: f64, n: f64, m: f64, v: f64) -> BasePlate {
    let c = 0.10;
    let (bp, lp) = (b + 2.0 * c, h + 2.0 * c);
    let fjd = 2.0 / 3.0 * 1.5 * rc.fcd();
    // Bearing: the compressed block under the plate (rectangular).
    let e = if n > 0.0 { m.abs() / n } else { f64::INFINITY };
    let (bearing, tension) = if n > 0.0 && e <= lp / 6.0 {
        ((n / (bp * lp)) * (1.0 + 6.0 * e / lp) / fjd, 0.0)
    } else {
        // Uplift side: anchors a little inside the edge, compression at
        // the far edge over a block of 0,2·lp.
        let z = lp - 0.05 - 0.1 * lp;
        let t = ((m.abs() - n.max(0.0) * (lp / 2.0 - 0.1 * lp)) / z).max(-n.min(0.0));
        let compression = (n.max(0.0) + t.max(0.0)) / (bp * 0.2 * lp);
        (compression / fjd, t.max(0.0))
    };
    // Thickness for fjd over the overhang c, to standard sizes.
    let need = c * (3.0 * fjd * 1.05 / (fy * 1000.0)).sqrt();
    let thickness = [0.012, 0.015, 0.020, 0.025, 0.030, 0.035, 0.040, 0.050]
        .into_iter()
        .find(|t| *t >= need)
        .unwrap_or(0.050);
    // Anchors 4.6 (fub 400): two on the side in tension (four if more is
    // needed).
    let ft = |d: f64| 0.9 * 400e3 * bolt_area(d) / GAMMA_M2;
    let mut pick = (4u32, 0.030);
    'out: for count in [4u32, 8] {
        for d in [0.016, 0.020, 0.024, 0.027, 0.030] {
            if f64::from(count) / 2.0 * ft(d) >= tension {
                pick = (count, d);
                break 'out;
            }
        }
    }
    let (bolts, bolt) = pick;
    let t_use = tension / (f64::from(bolts) / 2.0 * ft(bolt));
    // Shear: friction 0,2·N and the anchors (αbc = 0,44 − 0,0003·240).
    let fv = (0.44 - 0.0003 * 240.0) * 400e3 * bolt_area(bolt) / GAMMA_M2;
    let shear = v.abs() / (0.2 * n.max(0.0) + f64::from(bolts) * fv);
    BasePlate {
        width: bp,
        length: lp,
        thickness,
        bolts,
        bolt,
        bearing,
        tension: t_use,
        shear,
    }
}

/// The bolts of a fin plate for an end shear `v` (kN) on a web `tw` (m)
/// of clear height `hw`: (count, diameter, fits, utilisation).
pub fn fin_plate(fu: f64, v: f64, tw: f64, hw: f64) -> (u32, f64, bool, f64) {
    let mut best = (0u32, 0.020, false, f64::INFINITY);
    for d in [0.016, 0.020, 0.024] {
        let d0 = d + 0.002;
        let fv = 0.6 * 800e3 * bolt_area(d) / GAMMA_M2;
        // Bearing on the web: k1 2,5, αb 0,6.
        let fb = 2.5 * 0.6 * fu * 1000.0 * d * tw / GAMMA_M2;
        let each = fv.min(fb);
        let n = ((v.abs() / each).ceil() as u32).max(2);
        let room = ((hw - 3.0 * d0) / (3.0 * d0)).floor() as i64 + 1;
        let fits = i64::from(n) <= room;
        let u = v.abs() / (f64::from(n) * each);
        if fits {
            return (n, d, true, u);
        }
        if best.0 == 0 {
            best = (n, d, false, u);
        }
    }
    best
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let mut out = Vec::new();
    let rc = Rc::of(model);
    let fy = model.structural_site().materials.steel_fy;
    let am = &an.model;
    for (owner, kind) in &am.kinds {
        if *kind != Kind::Steel {
            continue;
        }
        match owner {
            Owner::Column(id) => {
                // Its base node, if it stands on a support.
                let Some(base) = am
                    .pieces
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.owner == *owner)
                    .flat_map(|(bi, _)| [am.frame.bars[bi].i, am.frame.bars[bi].j])
                    .min_by(|a, b| am.frame.nodes[*a][2].total_cmp(&am.frame.nodes[*b][2]))
                else {
                    continue;
                };
                if !am.frame.supports.iter().any(|(s, _)| *s == base) {
                    continue;
                }
                let Some(t) = model
                    .column(*id)
                    .and_then(|c| model.member_type(c.member_type))
                else {
                    continue;
                };
                let (b, h) = t.profile.size();
                // The worst ultimate reaction.
                let mut worst: Option<(f64, BasePlate, [f64; 6])> = None;
                for combo in &an.combinations {
                    if !matches!(combo.kind, ComboKind::Ultimate | ComboKind::Seismic) {
                        continue;
                    }
                    let mut r = [0.0; 6];
                    for (case, f) in &combo.factors {
                        if let Some(s) = an.solutions.get(case) {
                            if let Some((_, rr)) = s.reactions.iter().find(|(n, _)| *n == base) {
                                for d in 0..6 {
                                    r[d] += f * rr[d];
                                }
                            }
                        }
                    }
                    let m = r[3].hypot(r[4]);
                    let v = r[0].hypot(r[1]);
                    let p = base_plate(&rc, fy, b, h, r[2], m, v);
                    let u = p.bearing.max(p.tension).max(p.shear);
                    if worst.as_ref().is_none_or(|w| u > w.0) {
                        worst = Some((u, p, r));
                    }
                }
                let Some((u, p, r)) = worst else { continue };
                out.push(Check {
                    subject: Subject::Column(*id),
                    what: format!(
                        "{}: N {} kN, M {} kN·m, V {} kN. Placa {}×{}×{} mm con {} pernos M{} (4.6): compresión {}, pernos {}, cortante {}",
                        t.name,
                        dec(r[2], 1),
                        dec(r[3].hypot(r[4]), 1),
                        dec(r[0].hypot(r[1]), 1),
                        dec(p.width * 1000.0, 0),
                        dec(p.length * 1000.0, 0),
                        dec(p.thickness * 1000.0, 0),
                        p.bolts,
                        dec(p.bolt * 1000.0, 0),
                        dec(p.bearing, 2),
                        dec(p.tension, 2),
                        dec(p.shear, 2)
                    ),
                    rule: "Placa de anclaje (EN 1993-1-8, 6.2.5 y 6.2.6.12)",
                    ratio: u,
                    status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                    advice: (u > 1.0).then(|| {
                        "placa mayor, cartelas o más pernos; con mucho cortante, una llave bajo la placa".into()
                    }),
                });
            }
            Owner::Beam(id) => {
                let Some(t) = model
                    .beam(*id)
                    .and_then(|b| model.member_type(b.member_type))
                else {
                    continue;
                };
                let Profile::IShape {
                    depth, web, flange, ..
                } = t.profile
                else {
                    continue;
                };
                let Some(e) = an.envelopes.iter().find(|e| e.owner == *owner) else {
                    continue;
                };
                let v = e.max[2].abs().max(e.min[2].abs());
                let (n, d, fits, u) = fin_plate(steel_fu(fy), v, web, depth - 2.0 * flange);
                out.push(Check {
                    subject: Subject::Beam(*id),
                    what: format!(
                        "{}: cortante en el apoyo {} kN; chapa de unión con {} tornillos M{} (8.8) en una fila{}",
                        t.name,
                        dec(v, 1),
                        n,
                        dec(d * 1000.0, 0),
                        if fits { "" } else { ", que no caben en el alma" }
                    ),
                    rule: "Unión articulada atornillada (EN 1993-1-8, 3.6)",
                    ratio: if fits { u } else { 1.5 },
                    status: if fits && u <= 1.0 {
                        Status::Pass
                    } else {
                        Status::Fail
                    },
                    advice: (!fits).then(|| {
                        "dos filas de tornillos, un angular doble o una unión soldada".into()
                    }),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plate_and_its_bolts() {
        // A HEB 200 with 500 kN of compression and no moment: 400 × 400
        // plate, σ 3,1 N/mm² on fjd 16,7: some 0,19.
        let rc = Rc::default();
        let p = base_plate(&rc, 275.0, 0.2, 0.2, 500.0, 0.0, 10.0);
        assert!(
            (p.bearing - 500.0 / 0.16 / (rc.fcd())).abs() < 0.01,
            "{p:?}"
        );
        assert_eq!(p.tension, 0.0);
        // With 60 kN·m the anchors pull.
        let p = base_plate(&rc, 275.0, 0.2, 0.2, 100.0, 60.0, 10.0);
        assert!(p.tension > 0.0 && p.tension <= 1.0, "{p:?}");
        // An IPE 300 end with 120 kN: two or three M16–M20.
        let (n, _, fits, u) = fin_plate(430.0, 120.0, 0.0071, 0.2786);
        assert!(fits && (2..=4).contains(&n) && u <= 1.0, "{n} {u}");
    }
}
