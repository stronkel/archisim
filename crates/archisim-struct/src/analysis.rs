//! The analysis of the frame: each load case solved once, the
//! combinations added up, and for each column and beam the envelope of
//! its internal forces, its deflections against the limits of DB-SE
//! (4.3.3) and the reactions at the supports.

use std::collections::BTreeMap;

use archisim_core::Model;

use crate::analytical::{self, Analytical, Owner};
use crate::cases::{combinations, Case, Combination, Kind};
use crate::frame::{internal, Solution, Solver};
use crate::gravity::{takedown, Takedown};
use crate::wind::{building_wind, BuildingWind};

/// The internal forces of a section: N, Vy, Vz, T, My, Mz (kN, kN·m).
pub type Forces = [f64; 6];

/// The envelope of one element (a column or a beam) in the ultimate
/// combinations.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    pub owner: Owner,
    pub length: f64,
    /// Most and least of each force along the element.
    pub max: Forces,
    pub min: Forces,
    /// The combination (index) that gives the largest bending moment.
    pub governing: usize,
    /// Sections along it: (along, forces) of the governing combination.
    pub diagram: Vec<(f64, Forces)>,
    /// For each combination, the forces at the stations (for the checks).
    pub stations: Vec<f64>,
    pub by_combination: Vec<Vec<Forces>>,
}

/// The deflection of a span.
#[derive(Debug, Clone, PartialEq)]
pub struct Deflection {
    pub owner: Owner,
    pub length: f64,
    pub cantilever: bool,
    /// Relative deflection (from the line between its ends), m: under the
    /// characteristic combinations (integrity), the frequent ones
    /// (comfort, variable part only) and the quasi-permanent one
    /// (appearance).
    pub integrity: f64,
    pub comfort: f64,
    pub appearance: f64,
}

/// The analysis of the model.
#[derive(Debug, Clone)]
pub struct Analysis {
    pub takedown: Takedown,
    pub wind: Option<BuildingWind>,
    pub model: Analytical,
    pub cases: Vec<Case>,
    pub combinations: Vec<Combination>,
    /// The solution of each case.
    pub solutions: BTreeMap<Case, Solution>,
    pub envelopes: Vec<Envelope>,
    pub deflections: Vec<Deflection>,
    /// The largest reaction at each support in the ultimate combinations
    /// (and the characteristic ones, for the ground): node, (ultimate,
    /// characteristic) vertical, kN.
    pub reactions: Vec<(usize, f64, f64)>,
    pub notes: Vec<String>,
}

/// The loads over one bar, local: (from, to, per metre or force).
type BarLoads = Vec<(f64, f64, [f64; 3])>;

/// Number of stations along each piece for the envelopes.
const STATIONS: usize = 4;

/// Analyses the model.
pub fn analyse(model: &Model) -> Analysis {
    let td = takedown(model);
    let wind = building_wind(model);
    let am = analytical::build(model, &td, wind.as_ref());
    let mut cases: Vec<Case> = td.cases.clone();
    for c in am.loads.keys() {
        if !cases.contains(c) {
            cases.push(*c);
        }
    }
    cases.sort();
    let combos = combinations(&cases, model.site_altitude());
    let mut notes = am.notes.clone();
    let mut solutions = BTreeMap::new();
    let mut envelopes = Vec::new();
    let mut deflections = Vec::new();
    let mut reactions = Vec::new();
    if !am.frame.bars.is_empty() {
        let solver = Solver::new(&am.frame);
        for case in &cases {
            let loads = am.loads.get(case).cloned().unwrap_or_default();
            solutions.insert(*case, solver.solve(&loads));
        }
        if let Some(s) = solutions.values().next() {
            if !s.loose.is_empty() {
                let mut nodes: Vec<usize> = s.loose.iter().map(|(n, _)| *n).collect();
                nodes.sort_unstable();
                nodes.dedup();
                notes.push(format!(
                    "El pórtico tiene {} movimientos sin coaccionar en {} nudos (falta arriostramiento o un apoyo): se han sujetado con un muelle débil y sus resultados no valen para cargas horizontales.",
                    s.loose.len(),
                    nodes.len()
                ));
            }
        }
        // Per element: stations of its pieces.
        let mut by_owner: BTreeMap<Owner, Vec<usize>> = BTreeMap::new();
        for (i, p) in am.pieces.iter().enumerate() {
            by_owner.entry(p.owner).or_default().push(i);
        }
        let ultimate: Vec<usize> = combos
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == Kind::Ultimate || c.kind == Kind::Seismic)
            .map(|(i, _)| i)
            .collect();
        let bar_loads: BTreeMap<Case, Vec<BarLoads>> = cases
            .iter()
            .map(|c| {
                let loads = am.loads.get(c).cloned().unwrap_or_default();
                (
                    *c,
                    (0..am.frame.bars.len())
                        .map(|b| solver.bar_loads(&loads, b))
                        .collect(),
                )
            })
            .collect();
        for (owner, list) in &by_owner {
            let mut stations = Vec::new();
            let mut per_case: BTreeMap<Case, Vec<[f64; 6]>> = BTreeMap::new();
            for &bi in list {
                let p = &am.pieces[bi];
                let l = solver.bar_length(bi);
                for k in 0..=STATIONS {
                    let x = l * k as f64 / STATIONS as f64;
                    let x_in = x.clamp(1e-9, (l - 1e-9).max(1e-9));
                    stations.push(p.from + (p.to - p.from) * k as f64 / STATIONS as f64);
                    for case in &cases {
                        let f =
                            internal(&solutions[case].end_forces[bi], &bar_loads[case][bi], x_in);
                        per_case.entry(*case).or_default().push(f);
                    }
                }
            }
            let mut by_combination = Vec::new();
            let mut max = [f64::NEG_INFINITY; 6];
            let mut min = [f64::INFINITY; 6];
            let mut governing = 0;
            let mut worst = -1.0;
            for (ci, combo) in combos.iter().enumerate() {
                let mut v = vec![[0.0; 6]; stations.len()];
                for (case, factor) in &combo.factors {
                    if let Some(fs) = per_case.get(case) {
                        for (s, f) in v.iter_mut().zip(fs) {
                            for d in 0..6 {
                                s[d] += factor * f[d];
                            }
                        }
                    }
                }
                if ultimate.contains(&ci) {
                    for s in &v {
                        for d in 0..6 {
                            max[d] = max[d].max(s[d]);
                            min[d] = min[d].min(s[d]);
                        }
                        let m = s[4].abs().max(s[5].abs()).max(s[0].abs() * 0.01);
                        if m > worst {
                            worst = m;
                            governing = ci;
                        }
                    }
                }
                by_combination.push(v);
            }
            let diagram = stations
                .iter()
                .copied()
                .zip(by_combination[governing].iter().copied())
                .collect();
            let length = stations.last().copied().unwrap_or(0.0);
            envelopes.push(Envelope {
                owner: *owner,
                length,
                max,
                min,
                governing,
                diagram,
                stations,
                by_combination,
            });
        }
        // Deflections of the beams' spans: the vertical movement of each
        // node from the line between the span's ends.
        let disp = |case: &Case, n: usize| solutions[case].displacements[n][2];
        for sp in &am.spans {
            let rel = |combo: &Combination, only_variable: bool| -> f64 {
                let at = |n: usize| -> f64 {
                    combo
                        .factors
                        .iter()
                        .filter(|(c, _)| !(only_variable && *c == Case::Dead))
                        .map(|(c, f)| f * disp(c, n))
                        .sum()
                };
                let first = sp.nodes[0];
                let last = *sp.nodes.last().unwrap();
                let k = sp.nodes.len() - 1;
                let mut worst: f64 = 0.0;
                for (i, n) in sp.nodes.iter().enumerate() {
                    let base = if sp.cantilever {
                        // From the supported end (the one that moves less
                        // in its turn is unknown here: the larger).
                        0.0
                    } else {
                        at(first) + (at(last) - at(first)) * i as f64 / k as f64
                    };
                    let d = if sp.cantilever {
                        (at(*n) - at(first)).abs().max((at(*n) - at(last)).abs())
                    } else {
                        (at(*n) - base).abs()
                    };
                    worst = worst.max(d);
                }
                worst
            };
            let pick = |kind: Kind, only_variable: bool| {
                combos
                    .iter()
                    .filter(|c| c.kind == kind)
                    .map(|c| rel(c, only_variable))
                    .fold(0.0, f64::max)
            };
            deflections.push(Deflection {
                owner: sp.owner,
                length: sp.length,
                cantilever: sp.cantilever,
                integrity: pick(Kind::Characteristic, false),
                comfort: pick(Kind::Frequent, true),
                appearance: pick(Kind::QuasiPermanent, false),
            });
        }
        // Reactions.
        for (si, (node, _)) in am.frame.supports.iter().enumerate() {
            let mut ult: f64 = 0.0;
            let mut chr: f64 = 0.0;
            for combo in &combos {
                let r: f64 = combo
                    .factors
                    .iter()
                    .map(|(c, f)| f * solutions[c].reactions[si].1[2])
                    .sum();
                match combo.kind {
                    Kind::Ultimate | Kind::Seismic => ult = ult.max(r),
                    Kind::Characteristic => chr = chr.max(r),
                    _ => {}
                }
            }
            reactions.push((*node, ult, chr));
        }
    }
    Analysis {
        takedown: td,
        wind,
        model: am,
        cases,
        combinations: combos,
        solutions,
        envelopes,
        deflections,
        reactions,
        notes,
    }
}

impl Analysis {
    /// The applied vertical load of a case on the frame and what its
    /// supports give back, kN (they must be equal).
    pub fn frame_balance(&self, case: Case) -> (f64, f64) {
        let Some(sol) = self.solutions.get(&case) else {
            return (0.0, 0.0);
        };
        let applied: f64 = self
            .model
            .loads
            .get(&case)
            .map(|ls| {
                ls.iter()
                    .map(|l| match l {
                        crate::frame::Load::Node { f, .. } => -f[2],
                        crate::frame::Load::Spread { from, to, w, .. } => -w[2] * (to - from),
                        crate::frame::Load::Point { f, .. } => -f[2],
                    })
                    .sum()
            })
            .unwrap_or(0.0);
        let back: f64 = sol.reactions.iter().map(|r| r.1[2]).sum();
        (applied, back)
    }

    /// The deflection limit of a span (DB-SE 4.3.3.1): 1/400 with
    /// ordinary partitions on it (floors), 1/300 on roofs; a cantilever
    /// counts twice its length.
    pub fn limits(d: &Deflection, roof: bool) -> (f64, f64, f64) {
        let l = if d.cantilever {
            2.0 * d.length
        } else {
            d.length
        };
        (l / if roof { 300.0 } else { 400.0 }, l / 350.0, l / 300.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    /// Two concrete columns and a beam with a slab strip over it.
    fn portal() -> (Model, ColumnId, ColumnId, BeamId) {
        let mut m = Model::new("Pórtico");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let col = |m: &mut Model, x: f64| {
            m.add_column(NewColumn {
                member_type: lib.column,
                level: l0,
                position: Point2::new(x, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap()
        };
        let a = col(&mut m, 0.0);
        let b = col(&mut m, 5.0);
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
                Point2::new(0.0, -1.0),
                Point2::new(5.0, -1.0),
                Point2::new(5.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        (m, a, b, beam)
    }

    #[test]
    fn a_portal_carries_its_floor() {
        let (m, a, b, beam) = portal();
        let an = analyse(&m);
        assert!(an.notes.is_empty(), "{:?}", an.notes);
        // What goes on the frame comes back at its supports.
        for case in &an.cases {
            let (applied, back) = an.frame_balance(*case);
            assert!(
                (applied - back).abs() < 1e-6 * applied.abs().max(1.0),
                "{case:?}: {applied} {back}"
            );
        }
        // The beam: a span of 5 m, a sagging moment in the middle and
        // hogging at its rigid ends.
        let env = an
            .envelopes
            .iter()
            .find(|e| e.owner == Owner::Beam(beam))
            .unwrap();
        assert!(
            env.max[4] > 0.0 && env.min[4] < 0.0,
            "{:?} {:?}",
            env.max,
            env.min
        );
        // Both columns in compression, about equally.
        let n = |c: ColumnId| {
            an.envelopes
                .iter()
                .find(|e| e.owner == Owner::Column(c))
                .unwrap()
                .min[0]
        };
        let (na, nb) = (n(a), n(b));
        assert!(na < 0.0 && (na - nb).abs() < 0.05 * na.abs(), "{na} {nb}");
        let d = an
            .deflections
            .iter()
            .find(|d| d.owner == Owner::Beam(beam))
            .unwrap();
        assert!(d.integrity > 0.0 && d.integrity < 0.02, "{d:?}");
        assert!(an.reactions.iter().all(|r| r.1 > 0.0));
        // The wind sways the frame: the columns bend.
        assert!(an.solutions.keys().any(|c| matches!(c, Case::Wind(_))) || an.wind.is_none());
    }
}
