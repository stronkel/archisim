//! Load-bearing masonry walls in compression (DB-SE-F, 5.2): the
//! heaviest metre at the base against Φ·t·fd, with the eccentricity of
//! the floors and the accidental one and the slenderness limit.
//! fk of the project's materials (4 N/mm² by default: a perforated brick
//! with M-7,5 mortar) and γM 2,5 (category II of execution).

use archisim_core::{LayerFunction, Model};

use crate::cases::Case;
use crate::gravity::{Carrier, Takedown, BIN};
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

pub const GAMMA_M: f64 = 2.5;

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let mut out = Vec::new();
    for w in model.walls() {
        let Some(t) = model.wall_type(w.wall_type) else {
            continue;
        };
        let Some(l) = t.layers.iter().find(|l| {
            l.function == LayerFunction::Structure
                && Kind::of(model.material(l.material)) == Kind::Masonry
        }) else {
            continue;
        };
        let Some(c) = td.carriers.get(&Carrier::Wall(w.id)) else {
            continue;
        };
        let th = l.thickness;
        let n = c.own.len();
        let worst = (0..n)
            .map(|i| {
                c.delivered
                    .iter()
                    .map(|(case, v)| {
                        let f = match case {
                            Case::Dead => 1.35,
                            Case::Wind(_) | Case::Quake(_) => 0.0,
                            _ => 1.5,
                        };
                        f * v.get(i).copied().unwrap_or(0.0)
                    })
                    .sum::<f64>()
                    / BIN
            })
            .fold(0.0, f64::max);
        if worst <= 0.0 {
            continue;
        }
        let h = (c.top - c.base).max(0.5);
        // Effective height: 0,75 h with floors on both ends (ρ2).
        let hef = 0.75 * h;
        let slender = hef / th;
        let e = 0.05 * th + hef / 450.0;
        let phi = (1.0 - 2.0 * e / th).max(0.0);
        let fk = model.structural_site().materials.masonry_fk;
        let fd = fk / GAMMA_M * 1000.0;
        let nrd = phi * th * fd;
        let u = (worst / nrd.max(1e-6)).max(if slender > 27.0 { slender / 27.0 } else { 0.0 });
        out.push(Check {
            subject: Subject::Wall(w.id),
            what: format!(
                "{}: {} kN/m en la base, resiste {} kN/m (fk {} N/mm², Φ {}, t {} m, esbeltez {})",
                t.name,
                dec(worst, 1),
                dec(nrd, 1),
                dec(fk, 1),
                dec(phi, 2),
                dec(th, 3),
                dec(slender, 1)
            ),
            rule: "Fábrica a compresión (DB-SE-F, 5.2; γM 2,5)",
            ratio: u,
            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
            advice: (u > 1.0).then(|| {
                if slender > 27.0 {
                    "más espesor: la esbeltez pasa de 27".into()
                } else {
                    "más espesor o una pieza más resistente".into()
                }
            }),
        });
    }
    out
}
