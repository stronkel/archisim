//! The sway of the frame: the drift of each storey under the wind and the
//! earthquake (DB-SE
//! 4.3.3.2: 1/250 of the storey and 1/500 of the whole building) and the
//! second-order effects (EN 1993-1-1, 5.2.1: αcr from the stability index
//! θ = P·Δ / (V·h) of each storey; under 0,1 they can be left out, up to
//! 0,3 the horizontal actions are amplified by 1/(1 − θ) — `analyse`
//! does it, 5.2.2(5) — and beyond it the frame needs bracing).

use archisim_core::Model;

use crate::analysis::Analysis;
use crate::analytical::Owner;
use crate::cases::Case;
use crate::lightframe::{Check, Status, Subject};
use crate::text::dec;

/// One storey of columns in one wind direction.
#[derive(Debug, Clone, PartialEq)]
pub struct Storey {
    pub base: f64,
    pub top: f64,
    pub case: Case,
    /// Mean horizontal drift (characteristic), m.
    pub drift: f64,
    /// Horizontal shear the storey takes (characteristic), kN.
    pub shear: f64,
    /// Vertical load on it in the ultimate combinations, kN.
    pub load: f64,
}

impl Storey {
    pub fn theta(&self) -> f64 {
        let h = (self.top - self.base).max(0.1);
        if self.shear.abs() < 1e-6 {
            return 0.0;
        }
        self.load * self.drift / (self.shear.abs() * h)
    }

    /// The amplification of the horizontal actions, 1/(1 − θ).
    pub fn amplification(&self) -> f64 {
        let t = self.theta();
        if t < 0.1 {
            1.0
        } else {
            1.0 / (1.0 - t).max(0.05)
        }
    }
}

pub fn storeys(an: &Analysis) -> Vec<Storey> {
    // The vertical load of each column: its largest ultimate compression.
    let load_of = |o: Owner| -> f64 {
        an.envelopes
            .iter()
            .find(|e| e.owner == o)
            .map_or(0.0, |e| (-e.min[0]).max(0.0))
    };
    storeys_with(&an.model, &an.solutions, &load_of)
}

/// The storeys of a frame from its solutions, with the vertical load of
/// each column given.
pub fn storeys_with(
    am: &crate::analytical::Analytical,
    solutions: &std::collections::BTreeMap<Case, crate::frame::Solution>,
    load_of: &dyn Fn(Owner) -> f64,
) -> Vec<Storey> {
    // The columns: base and top node and their pieces.
    let mut cols: std::collections::BTreeMap<Owner, (usize, usize, Vec<usize>)> =
        Default::default();
    for (bi, p) in am.pieces.iter().enumerate() {
        if !matches!(p.owner, Owner::Column(_)) {
            continue;
        }
        let bar = &am.frame.bars[bi];
        let e = cols.entry(p.owner).or_insert((bar.i, bar.j, Vec::new()));
        for n in [bar.i, bar.j] {
            if am.frame.nodes[n][2] < am.frame.nodes[e.0][2] {
                e.0 = n;
            }
            if am.frame.nodes[n][2] > am.frame.nodes[e.1][2] {
                e.1 = n;
            }
        }
        e.2.push(bi);
    }
    let mut out = Vec::new();
    for (case, sol) in solutions {
        let [dx, dy] = match case {
            Case::Wind(dir) => dir.vector(),
            Case::Quake(true) => [1.0, 0.0],
            Case::Quake(false) => [0.0, 1.0],
            _ => continue,
        };
        // Group the columns by their base and top (a storey).
        let mut groups: Vec<(f64, f64, Vec<Owner>)> = Vec::new();
        for (o, (b, t, _)) in &cols {
            let zb = am.frame.nodes[*b][2];
            let zt = am.frame.nodes[*t][2];
            match groups
                .iter_mut()
                .find(|g| (g.0 - zb).abs() < 0.5 && (g.1 - zt).abs() < 0.5)
            {
                Some(g) => g.2.push(*o),
                None => groups.push((zb, zt, vec![*o])),
            }
        }
        for (zb, zt, owners) in groups {
            let mut drift = 0.0;
            let mut shear = 0.0;
            let mut load = 0.0;
            for o in &owners {
                let (b, t, bars) = &cols[o];
                let ub = sol.displacements[*b];
                let ut = sol.displacements[*t];
                drift += (ut[0] - ub[0]) * dx + (ut[1] - ub[1]) * dy;
                // The shear at the base of the column: the support's
                // reaction if it stands on one, else its end forces.
                if let Some((_, r)) = sol.reactions.iter().find(|(n, _)| n == b) {
                    shear -= r[0] * dx + r[1] * dy;
                } else if let Some(&bi) = bars
                    .iter()
                    .find(|&&bi| am.frame.bars[bi].i == *b || am.frame.bars[bi].j == *b)
                {
                    let ax = crate::frame::axes(&am.frame, &am.frame.bars[bi]);
                    let ef = &sol.end_forces[bi];
                    let k = if am.frame.bars[bi].i == *b { 0 } else { 6 };
                    // Local end forces back to global.
                    let mut g = [0.0; 3];
                    for (a, row) in ax.iter().enumerate() {
                        for d in 0..3 {
                            g[d] += row[d] * ef[k + a];
                        }
                    }
                    shear -= g[0] * dx + g[1] * dy;
                }
                load += load_of(*o);
            }
            let n = owners.len().max(1) as f64;
            out.push(Storey {
                base: zb,
                top: zt,
                case: *case,
                drift: drift / n,
                shear,
                load,
            });
        }
    }
    out
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let _ = model;
    let st = storeys(an);
    let mut out = Vec::new();
    if st.is_empty() {
        return out;
    }
    // The drift limits are those of the wind (characteristic); the
    // earthquake counts for θ.
    let windy: Vec<&Storey> = st
        .iter()
        .filter(|s| matches!(s.case, Case::Wind(_)))
        .collect();
    if let Some(worst_drift) = windy.iter().copied().max_by(|a, b| {
        (a.drift.abs() / (a.top - a.base)).total_cmp(&(b.drift.abs() / (b.top - b.base)))
    }) {
        let h = (worst_drift.top - worst_drift.base).max(0.1);
        let u = worst_drift.drift.abs() / (h / 250.0);
        out.push(Check {
            subject: Subject::Building,
            what: format!(
                "Desplome de la planta de {} a {} m con {}: {} mm, límite h/250 = {} mm",
                dec(worst_drift.base, 2),
                dec(worst_drift.top, 2),
                worst_drift.case.name(),
                dec(worst_drift.drift.abs() * 1000.0, 1),
                dec(h / 250.0 * 1000.0, 1)
            ),
            rule: "Desplome local (DB-SE 4.3.3.2)",
            ratio: u,
            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
            advice: (u > 1.0).then(|| "pilares más rígidos o arriostramiento".into()),
        });
        // Total drift per direction: the sum of the storeys.
        let mut by_case: std::collections::BTreeMap<Case, (f64, f64, f64)> = Default::default();
        for s in &windy {
            let e = by_case.entry(s.case).or_insert((0.0, f64::MAX, f64::MIN));
            e.0 += s.drift;
            e.1 = e.1.min(s.base);
            e.2 = e.2.max(s.top);
        }
        if let Some((case, (d, lo, hi))) = by_case.iter().max_by(|a, b| {
            (a.1 .0.abs() / (a.1 .2 - a.1 .1)).total_cmp(&(b.1 .0.abs() / (b.1 .2 - b.1 .1)))
        }) {
            let h = (hi - lo).max(0.1);
            let u = d.abs() / (h / 500.0);
            out.push(Check {
                subject: Subject::Building,
                what: format!(
                    "Desplome total con {}: {} mm en {} m, límite H/500 = {} mm",
                    case.name(),
                    dec(d.abs() * 1000.0, 1),
                    dec(h, 1),
                    dec(h / 500.0 * 1000.0, 1)
                ),
                rule: "Desplome total (DB-SE 4.3.3.2)",
                ratio: u,
                status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                advice: (u > 1.0)
                    .then(|| "más rigidez horizontal: muros, cruces o pilares mayores".into()),
            });
        }
    }
    let worst = st
        .iter()
        .max_by(|a, b| a.theta().total_cmp(&b.theta()))
        .unwrap();
    let t = worst.theta();
    let (status, advice) = if t < 0.1 {
        (Status::Pass, None)
    } else if t <= 0.3 {
        (
            Status::Warn,
            Some(format!(
                "efectos de segundo orden: el cálculo ya amplifica {} × {} (EN 1993-1-1, 5.2.2(5))",
                worst.case.name(),
                dec(
                    an.amplification
                        .get(&worst.case)
                        .copied()
                        .unwrap_or(worst.amplification()),
                    2
                )
            )),
        )
    } else {
        (
            Status::Fail,
            Some("pórtico demasiado flexible: hay que arriostrarlo".into()),
        )
    };
    out.push(Check {
        subject: Subject::Building,
        what: format!(
            "Índice de estabilidad θ = P·Δ/(V·h) de la planta de {} a {} m con {}: P {} kN, Δ {} mm, V {} kN",
            dec(worst.base, 2),
            dec(worst.top, 2),
            worst.case.name(),
            dec(worst.load, 0),
            dec(worst.drift.abs() * 1000.0, 1),
            dec(worst.shear.abs(), 1)
        ),
        rule: "Segundo orden (EN 1993-1-1, 5.2.1; θ ≤ 0,10)",
        ratio: t / 0.1,
        status,
        advice,
    });
    out
}
