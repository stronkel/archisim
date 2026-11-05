//! Load cases and their combinations (CTE DB-SE 4.2): ultimate limit
//! states in the persistent and transient situation, and the
//! characteristic, frequent and quasi-permanent ones for serviceability.

use crate::actions::{snow_psi, UseCategory, WIND_PSI};

/// Where the wind blows towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindDir {
    PosX,
    NegX,
    PosY,
    NegY,
}

impl WindDir {
    pub const ALL: [WindDir; 4] = [WindDir::PosX, WindDir::NegX, WindDir::PosY, WindDir::NegY];

    /// The unit vector it blows along, in plan.
    pub fn vector(self) -> [f64; 2] {
        match self {
            WindDir::PosX => [1.0, 0.0],
            WindDir::NegX => [-1.0, 0.0],
            WindDir::PosY => [0.0, 1.0],
            WindDir::NegY => [0.0, -1.0],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            WindDir::PosX => "+X",
            WindDir::NegX => "−X",
            WindDir::PosY => "+Y",
            WindDir::NegY => "−Y",
        }
    }
}

/// A load case: one action, by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Case {
    /// Permanent: weights of the elements and loads placed as such.
    Dead,
    /// Live load of one category of use.
    Live(UseCategory),
    Snow,
    Wind(WindDir),
    /// The earthquake along X or Y (NCSE-02), accidental.
    Quake(bool),
}

impl Case {
    pub fn name(self) -> String {
        match self {
            Case::Dead => "Permanente (G)".into(),
            Case::Live(c) => format!("Uso {} (Q)", c.code()),
            Case::Snow => "Nieve (N)".into(),
            Case::Wind(d) => format!("Viento {} (V)", d.name()),
            Case::Quake(x) => format!("Sismo {} (E)", if x { "X" } else { "Y" }),
        }
    }

    pub fn short(self) -> String {
        match self {
            Case::Dead => "G".into(),
            Case::Live(c) => format!("Q{}", c.code()),
            Case::Snow => "N".into(),
            Case::Wind(d) => format!("V{}", d.name()),
            Case::Quake(x) => format!("E{}", if x { "X" } else { "Y" }),
        }
    }

    /// ψ0, ψ1, ψ2 (the permanent and the earthquake have none).
    pub fn psi(self, altitude: f64) -> [f64; 3] {
        match self {
            Case::Dead | Case::Quake(_) => [1.0; 3],
            Case::Live(c) => c.psi(),
            Case::Snow => snow_psi(altitude),
            Case::Wind(_) => WIND_PSI,
        }
    }

    /// The group it acts in: only one case of a group at a time (the
    /// winds); the live loads act together.
    fn group(self) -> u8 {
        match self {
            Case::Dead => 0,
            Case::Live(_) => 1,
            Case::Snow => 2,
            Case::Wind(_) => 3,
            Case::Quake(_) => 4,
        }
    }
}

/// What a combination is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Ultimate limit state, persistent or transient situation.
    Ultimate,
    /// Ultimate limit state, seismic situation.
    Seismic,
    /// Serviceability, characteristic (irreversible effects).
    Characteristic,
    /// Serviceability, frequent (comfort).
    Frequent,
    /// Serviceability, quasi-permanent (appearance, long term).
    QuasiPermanent,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Ultimate => "ELU persistente",
            Kind::Seismic => "ELU sísmica",
            Kind::Characteristic => "ELS característica",
            Kind::Frequent => "ELS frecuente",
            Kind::QuasiPermanent => "ELS casi permanente",
        }
    }
}

/// A combination: the factor of each case.
#[derive(Debug, Clone, PartialEq)]
pub struct Combination {
    pub kind: Kind,
    pub factors: Vec<(Case, f64)>,
}

impl Combination {
    pub fn factor(&self, case: Case) -> f64 {
        self.factors
            .iter()
            .find(|(c, _)| *c == case)
            .map_or(0.0, |(_, f)| *f)
    }

    /// As written in the report: «1,35·G + 1,50·QA1 + 0,75·N».
    pub fn label(&self) -> String {
        self.factors
            .iter()
            .filter(|(_, f)| *f != 0.0)
            .map(|(c, f)| format!("{}·{}", format!("{f:.2}").replace('.', ","), c.short()))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

/// Partial factors of DB-SE table 4.1 (resistance).
pub const G_UNFAV: f64 = 1.35;
pub const G_FAV: f64 = 0.8;
pub const Q_UNFAV: f64 = 1.5;

/// Every combination of `cases` (the permanent always; at most one wind
/// direction and one earthquake direction at a time).
pub fn combinations(cases: &[Case], altitude: f64) -> Vec<Combination> {
    let variable: Vec<Case> = cases
        .iter()
        .copied()
        .filter(|c| matches!(c, Case::Live(_) | Case::Snow | Case::Wind(_)))
        .collect();
    let lives: Vec<Case> = variable
        .iter()
        .copied()
        .filter(|c| matches!(c, Case::Live(_)))
        .collect();
    let snow = variable.contains(&Case::Snow);
    let winds: Vec<Case> = variable
        .iter()
        .copied()
        .filter(|c| matches!(c, Case::Wind(_)))
        .collect();
    let has_dead = cases.contains(&Case::Dead);
    let quakes: Vec<Case> = cases
        .iter()
        .copied()
        .filter(|c| matches!(c, Case::Quake(_)))
        .collect();
    // The leading actions: the live loads together, the snow, or one wind.
    let mut leads: Vec<Vec<Case>> = Vec::new();
    if !lives.is_empty() {
        leads.push(lives.clone());
    }
    if snow {
        leads.push(vec![Case::Snow]);
    }
    for w in &winds {
        leads.push(vec![*w]);
    }
    let mut out: Vec<Combination> = Vec::new();
    let mut push = |c: Combination| {
        if !out.contains(&c) {
            out.push(c);
        }
    };
    let psi = |c: Case, i: usize| c.psi(altitude)[i];
    // The other groups, each there or not (one wind direction or none).
    let others = |lead: &[Case]| -> Vec<Vec<Case>> {
        let g = lead[0].group();
        let mut sets: Vec<Vec<Case>> = vec![Vec::new()];
        let mut options: Vec<Vec<Vec<Case>>> = Vec::new();
        if g != 1 && !lives.is_empty() {
            options.push(vec![Vec::new(), lives.clone()]);
        }
        if g != 2 && snow {
            options.push(vec![Vec::new(), vec![Case::Snow]]);
        }
        if g != 3 && !winds.is_empty() {
            let mut o = vec![Vec::new()];
            o.extend(winds.iter().map(|w| vec![*w]));
            options.push(o);
        }
        for opt in options {
            let mut next = Vec::new();
            for s in &sets {
                for o in &opt {
                    let mut t = s.clone();
                    t.extend(o.iter().copied());
                    next.push(t);
                }
            }
            sets = next;
        }
        sets
    };
    let dead_factors: &[f64] = if winds.is_empty() {
        &[G_UNFAV]
    } else {
        &[G_UNFAV, G_FAV]
    };
    // Ultimate.
    for &gf in dead_factors {
        let base: Vec<(Case, f64)> = if has_dead {
            vec![(Case::Dead, gf)]
        } else {
            Vec::new()
        };
        if leads.is_empty() {
            push(Combination {
                kind: Kind::Ultimate,
                factors: base.clone(),
            });
        }
        for lead in &leads {
            for acc in others(lead) {
                let mut f = base.clone();
                f.extend(lead.iter().map(|c| (*c, Q_UNFAV)));
                f.extend(acc.iter().map(|c| (*c, Q_UNFAV * psi(*c, 0))));
                push(Combination {
                    kind: Kind::Ultimate,
                    factors: f,
                });
            }
        }
    }
    let dead1: Vec<(Case, f64)> = if has_dead {
        vec![(Case::Dead, 1.0)]
    } else {
        Vec::new()
    };
    // Characteristic and frequent.
    for lead in &leads {
        for acc in others(lead) {
            let mut f = dead1.clone();
            f.extend(lead.iter().map(|c| (*c, 1.0)));
            f.extend(acc.iter().map(|c| (*c, psi(*c, 0))));
            push(Combination {
                kind: Kind::Characteristic,
                factors: f,
            });
            let mut f = dead1.clone();
            f.extend(lead.iter().map(|c| (*c, psi(*c, 1))));
            f.extend(acc.iter().map(|c| (*c, psi(*c, 2))));
            f.retain(|(_, v)| *v != 0.0);
            push(Combination {
                kind: Kind::Frequent,
                factors: f,
            });
        }
    }
    // Quasi-permanent.
    let mut f = dead1.clone();
    f.extend(
        variable
            .iter()
            .map(|c| (*c, psi(*c, 2)))
            .filter(|(c, v)| *v != 0.0 && !matches!(c, Case::Wind(_))),
    );
    push(Combination {
        kind: Kind::QuasiPermanent,
        factors: f,
    });
    // Seismic: G + E + Σ ψ2 Q, each direction both ways.
    for q in &quakes {
        for sign in [1.0, -1.0] {
            let mut f = dead1.clone();
            f.push((*q, sign));
            f.extend(
                variable
                    .iter()
                    .map(|c| (*c, psi(*c, 2)))
                    .filter(|(c, v)| *v != 0.0 && !matches!(c, Case::Wind(_))),
            );
            push(Combination {
                kind: Kind::Seismic,
                factors: f,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_house_has_its_combinations() {
        let cases = [
            Case::Dead,
            Case::Live(UseCategory::A1),
            Case::Live(UseCategory::G1),
            Case::Snow,
            Case::Wind(WindDir::PosX),
            Case::Wind(WindDir::NegX),
        ];
        let all = combinations(&cases, 50.0);
        let ult: Vec<_> = all.iter().filter(|c| c.kind == Kind::Ultimate).collect();
        // 1,35 G + 1,5 Q (lead) + 0,75 N + 0,9 V+X.
        assert!(ult
            .iter()
            .any(|c| (c.factor(Case::Dead) - 1.35).abs() < 1e-9
                && (c.factor(Case::Live(UseCategory::A1)) - 1.5).abs() < 1e-9
                && (c.factor(Case::Snow) - 0.75).abs() < 1e-9
                && (c.factor(Case::Wind(WindDir::PosX)) - 0.9).abs() < 1e-9));
        // Never two winds at once.
        assert!(all.iter().all(|c| c
            .factors
            .iter()
            .filter(|(k, f)| matches!(k, Case::Wind(_)) && *f != 0.0)
            .count()
            <= 1));
        // The upkeep load of the roof never accompanies (ψ0 = 0).
        assert!(ult
            .iter()
            .all(|c| c.factor(Case::Live(UseCategory::G1)) == 0.0
                || c.factor(Case::Live(UseCategory::A1)) != 0.0));
        // Uplift: 0,8 G with the wind leading.
        assert!(ult.iter().any(|c| (c.factor(Case::Dead) - 0.8).abs() < 1e-9
            && (c.factor(Case::Wind(WindDir::NegX)) - 1.5).abs() < 1e-9));
        let qp = all.iter().find(|c| c.kind == Kind::QuasiPermanent).unwrap();
        assert!((qp.factor(Case::Live(UseCategory::A1)) - 0.3).abs() < 1e-9);
        assert_eq!(qp.factor(Case::Snow), 0.0);
        assert!(ult[0].label().starts_with("1,35·G"));
    }
}
