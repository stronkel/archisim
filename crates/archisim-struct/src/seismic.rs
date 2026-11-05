//! The earthquake by NCSE-02's simplified method (3.7): the design
//! acceleration (2.2), the fundamental period (3.7.2.2), the elastic
//! spectrum (2.3) and the equivalent static force of each storey for the
//! first mode, in each direction.
//!
//! Where the frame takes the earthquake, its modal spectral analysis
//! (NCSE-02, 3.6.2): the masses of each storey at its nodes, the first
//! modes by subspace iteration on the frame's stiffness, the spectrum's
//! acceleration for each period, and the storey shears of the modes
//! combined by the square root of the sum of squares; the forces that
//! give those shears replace the simplified ones.

use archisim_core::{Importance, Model};

use crate::gravity::Takedown;

/// The earthquake on the building.
#[derive(Debug, Clone, PartialEq)]
pub struct Quake {
    /// Whether NCSE-02 asks for it here (and why not, if it does not).
    pub applies: bool,
    pub why: String,
    /// Design acceleration ac/g, the soil amplification S, the spectrum's
    /// value α(T), the fundamental period T (s) and β = υ/μ.
    pub ac: f64,
    pub s: f64,
    pub alpha: f64,
    pub period: f64,
    pub beta: f64,
    /// Each storey: (elevation, weight in kN, force in kN).
    pub storeys: Vec<(f64, f64, f64)>,
}

/// The ductility the structure is designed for (NCSE-02, 3.7.3.1): low
/// (μ 2) for frames and light frames, none (μ 1) for masonry.
fn ductility(model: &Model) -> f64 {
    if model.framings().next().is_some() || model.columns().next().is_some() {
        2.0
    } else {
        1.0
    }
}

/// The normalised spectrum α(T) (NCSE-02, 2.3).
pub fn alpha(k: f64, c: f64, t: f64) -> f64 {
    let ta = k * c / 10.0;
    let tb = k * c / 2.5;
    if t < ta {
        1.0 + 1.5 * t / ta
    } else if t <= tb {
        2.5
    } else {
        k * c / t
    }
}

pub fn quake(model: &Model, td: &Takedown) -> Quake {
    let site = model.structural_site();
    let rho = match site.importance {
        Importance::Special => 1.3,
        _ => 1.0,
    };
    let rab = rho * site.seismic_ab;
    let c = site.soil;
    let s = if rab <= 0.1 {
        c / 1.25
    } else if rab < 0.4 {
        c / 1.25 + 3.33 * (rab - 0.1) * (1.0 - c / 1.25)
    } else {
        1.0
    };
    let ac = s * rab;
    let mut levels: Vec<f64> = model.levels().iter().map(|l| l.elevation).collect();
    levels.sort_by(f64::total_cmp);
    let ground = levels.first().copied().unwrap_or(0.0);
    // Weights by level: the floors' G + ψ Q (NCSE-02, 3.2: 0,5 in
    // dwellings and offices, 0,6 in public buildings) and half the walls
    // above and below.
    let psi = match model.code_settings().building_use {
        archisim_core::BuildingUse::Dwelling
        | archisim_core::BuildingUse::PublicResidential
        | archisim_core::BuildingUse::Administrative => 0.5,
        archisim_core::BuildingUse::Parking => 1.0,
        _ => 0.6,
    };
    let mut weight: Vec<f64> = vec![0.0; levels.len()];
    let nearest = |z: f64| -> usize {
        levels
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - z).abs().total_cmp(&(b.1 - z).abs()))
            .map_or(0, |(i, _)| i)
    };
    for sf in &td.surfaces {
        if sf.on_ground {
            continue;
        }
        let z = model.level(sf.level).map_or(0.0, |l| l.elevation);
        let i = nearest(z);
        let q: f64 = sf.uses.values().map(|(_, kn)| kn).sum();
        weight[i] += sf.area * sf.dead + psi * q;
    }
    for c in td.carriers.values() {
        let own: f64 = c.own.iter().sum();
        weight[nearest(c.top)] += own / 2.0;
        weight[nearest(c.base)] += own / 2.0;
    }
    // Storeys above the ground with weight.
    let storeys: Vec<(f64, f64)> = levels
        .iter()
        .zip(&weight)
        .filter(|(z, w)| **z > ground + 0.5 && **w > 0.0)
        .map(|(z, w)| (*z, *w))
        .collect();
    let n = storeys.len().max(1) as f64;
    let height = storeys.last().map_or(3.0, |s| s.0 - ground).max(0.1);
    let period = if model.columns().next().is_some() {
        0.09 * n
    } else {
        // Walls (NCSE-02, 3.7.2.2): 0,06 n √(H/(2L+H)) √(H/(L+H)) with L
        // the building's smaller side.
        let l = crate::wind::building_wind(model)
            .map_or(10.0, |w| (w.hi.x - w.lo.x).min(w.hi.y - w.lo.y))
            .max(1.0);
        0.06 * n * (height / (2.0 * l + height)).sqrt() * (height / (l + height)).sqrt()
    };
    let k = site.seismic_k;
    let alpha = alpha(k, c, period);
    let beta = 1.0 / ductility(model);
    // First mode shape: sin(π h / 2H).
    let phi = |z: f64| (std::f64::consts::PI * (z - ground) / (2.0 * height)).sin();
    let sm: f64 = storeys.iter().map(|(z, w)| w * phi(*z)).sum();
    let sm2: f64 = storeys.iter().map(|(z, w)| w * phi(*z).powi(2)).sum();
    let forces = storeys
        .iter()
        .map(|(z, w)| {
            let eta = if sm2 > 0.0 { phi(*z) * sm / sm2 } else { 1.0 };
            (*z, *w, ac * alpha * beta * eta * w)
        })
        .collect();
    let (applies, why) = if site.importance == Importance::Moderate {
        (
            false,
            "construcción de importancia moderada (NCSE-02, 1.2.3)".to_string(),
        )
    } else if site.seismic_ab < 0.04 {
        (
            false,
            "aceleración básica menor de 0,04·g (NCSE-02, 1.2.3)".to_string(),
        )
    } else {
        (true, String::new())
    };
    Quake {
        applies,
        why,
        ac,
        s,
        alpha,
        period,
        beta,
        storeys: forces,
    }
}

/// The modal analysis of the frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Modal {
    /// Each mode: period (s) and the share of the mass it moves in X and
    /// in Y.
    pub modes: Vec<(f64, f64, f64)>,
    /// The storey forces in X and in Y (same order as `Quake::storeys`),
    /// kN.
    pub forces: [Vec<f64>; 2],
}

/// Eigenvalues and vectors of a small symmetric matrix (cyclic Jacobi).
fn jacobi(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut v = vec![vec![0.0; n]; n];
    for i in 0..n {
        v[i][i] = 1.0;
    }
    for _ in 0..100 {
        let mut off = 0.0;
        for p in 0..n {
            for q in p + 1..n {
                off += a[p][q] * a[p][q];
            }
        }
        if off < 1e-24 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q].abs() < 1e-30 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k][p], v[k][q]);
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i][i]).collect(), v)
}

/// The modal spectral analysis of a frame for an earthquake: `None` if
/// there is nothing to move.
pub fn modal(model: &Model, frame: &crate::frame::Frame, q: &Quake) -> Option<Modal> {
    const G: f64 = 9.81;
    let site = model.structural_site();
    let free = |n: usize| !frame.supports.iter().any(|(s, _)| *s == n);
    // Masses (t) at the nodes of each storey, and the storey of each.
    let mut mass: Vec<(usize, f64, usize)> = Vec::new();
    for (si, &(z, w, _)) in q.storeys.iter().enumerate() {
        let at: Vec<usize> = (0..frame.nodes.len())
            .filter(|&n| (frame.nodes[n][2] - z).abs() < 0.6 && free(n))
            .collect();
        if at.is_empty() {
            continue;
        }
        let m = w / G / at.len() as f64;
        mass.extend(at.into_iter().map(|n| (n, m, si)));
    }
    if mass.is_empty() || frame.supports.is_empty() {
        return None;
    }
    let dofs = 2 * mass.len();
    let solver = crate::frame::Solver::new(frame);
    // K⁻¹·f for forces on the mass dofs; the movements at them.
    let solve = |f: &[f64]| -> Vec<f64> {
        let loads: Vec<crate::frame::Load> = mass
            .iter()
            .enumerate()
            .map(|(k, (n, _, _))| crate::frame::Load::Node {
                node: *n,
                f: [f[2 * k], f[2 * k + 1], 0.0, 0.0, 0.0, 0.0],
            })
            .collect();
        let sol = solver.solve(&loads);
        mass.iter()
            .flat_map(|(n, _, _)| [sol.displacements[*n][0], sol.displacements[*n][1]])
            .collect()
    };
    let mdiag: Vec<f64> = mass.iter().flat_map(|(_, m, _)| [*m, *m]).collect();
    let mul_m = |x: &[f64]| -> Vec<f64> { x.iter().zip(&mdiag).map(|(a, m)| a * m).collect() };
    let dot = |a: &[f64], b: &[f64]| -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
    // Start vectors: the two directions and some mixes.
    let p = 6.min(dofs);
    let mut x: Vec<Vec<f64>> = (0..p)
        .map(|j| {
            (0..dofs)
                .map(|i| {
                    let node = i / 2;
                    let dir = i % 2;
                    match j {
                        0 => (dir == 0) as u8 as f64,
                        1 => (dir == 1) as u8 as f64,
                        _ => ((node * (j + 3) + dir * 7) % 11) as f64 - 5.0,
                    }
                })
                .collect()
        })
        .collect();
    let mut lambda = vec![0.0; p];
    for _ in 0..25 {
        let y: Vec<Vec<f64>> = x.iter().map(|xi| solve(&mul_m(xi))).collect();
        let my: Vec<Vec<f64>> = y.iter().map(|yi| mul_m(yi)).collect();
        let mx: Vec<Vec<f64>> = x.iter().map(|xi| mul_m(xi)).collect();
        // K̂ = Yᵀ·K·Y = Yᵀ·M·X, M̂ = Yᵀ·M·Y.
        let kh: Vec<Vec<f64>> = (0..p)
            .map(|a| (0..p).map(|b| dot(&y[a], &mx[b])).collect())
            .collect();
        let mh: Vec<Vec<f64>> = (0..p)
            .map(|a| (0..p).map(|b| dot(&y[a], &my[b])).collect())
            .collect();
        // M̂ = L·Lᵀ; L⁻¹·K̂·L⁻ᵀ.
        let mut l = vec![vec![0.0; p]; p];
        for i in 0..p {
            for j in 0..=i {
                let s: f64 = (0..j).map(|k| l[i][k] * l[j][k]).sum();
                if i == j {
                    let d = mh[i][i] - s;
                    if d <= 1e-30 {
                        return None;
                    }
                    l[i][j] = d.sqrt();
                } else {
                    l[i][j] = (mh[i][j] - s) / l[j][j];
                }
            }
        }
        let inv_l = |b: &[f64]| -> Vec<f64> {
            let mut z = vec![0.0; p];
            for i in 0..p {
                let s: f64 = (0..i).map(|k| l[i][k] * z[k]).sum();
                z[i] = (b[i] - s) / l[i][i];
            }
            z
        };
        // A = L⁻¹ K̂ L⁻ᵀ, column by column.
        let mut tmp = vec![vec![0.0; p]; p];
        for c in 0..p {
            let col: Vec<f64> = (0..p).map(|r| kh[r][c]).collect();
            let z = inv_l(&col);
            for r in 0..p {
                tmp[r][c] = z[r];
            }
        }
        let mut a = vec![vec![0.0; p]; p];
        for r in 0..p {
            let z = inv_l(&tmp[r]);
            a[r][..p].copy_from_slice(&z[..p]);
        }
        // Symmetrise.
        for r in 0..p {
            for c in r + 1..p {
                let m = 0.5 * (a[r][c] + a[c][r]);
                a[r][c] = m;
                a[c][r] = m;
            }
        }
        let (vals, vecs) = jacobi(a);
        // q = L⁻ᵀ·v.
        let mut order: Vec<usize> = (0..p).collect();
        order.sort_by(|&i, &j| vals[i].total_cmp(&vals[j]));
        let mut nx = Vec::new();
        for &k in &order {
            let v: Vec<f64> = (0..p).map(|r| vecs[r][k]).collect();
            let mut qv = vec![0.0; p];
            for i in (0..p).rev() {
                let s: f64 = (i + 1..p).map(|r| l[r][i] * qv[r]).sum();
                qv[i] = (v[i] - s) / l[i][i];
            }
            let mut xi = vec![0.0; dofs];
            for (j, yj) in y.iter().enumerate() {
                for (d, val) in xi.iter_mut().enumerate() {
                    *val += qv[j] * yj[d];
                }
            }
            nx.push(xi);
        }
        x = nx;
        lambda = order.iter().map(|&k| vals[k]).collect();
    }
    let total: f64 = mdiag.iter().sum::<f64>() / 2.0;
    let ns = q.storeys.len();
    let mut shears = [vec![0.0; ns], vec![0.0; ns]];
    let mut modes = Vec::new();
    for (k, phi) in x.iter().enumerate() {
        if lambda[k] <= 0.0 {
            continue;
        }
        let omega = lambda[k].sqrt();
        let t = 2.0 * std::f64::consts::PI / omega;
        let mphi = mul_m(phi);
        let gen = dot(phi, &mphi);
        let mut shares = [0.0; 2];
        for dir in 0..2 {
            let gamma: f64 = (0..mass.len()).map(|i| mphi[2 * i + dir]).sum::<f64>() / gen;
            shares[dir] = gamma * gamma * gen / total;
            // Spectral acceleration (m/s²) and the storey forces of this
            // mode in that direction.
            let sa = q.ac * alpha(site.seismic_k, site.soil, t) * q.beta * G;
            let mut f = vec![0.0; ns];
            for (i, (_, m, si)) in mass.iter().enumerate() {
                f[*si] += m * phi[2 * i + dir] * gamma * sa;
            }
            // Storey shears from the top down, squared and summed.
            let mut v = 0.0;
            for si in (0..ns).rev() {
                v += f[si];
                shears[dir][si] += v * v;
            }
        }
        modes.push((t, shares[0], shares[1]));
    }
    // Back to forces: the differences of the combined shears.
    let mut forces = [vec![0.0; ns], vec![0.0; ns]];
    for dir in 0..2 {
        let v: Vec<f64> = shears[dir].iter().map(|s| s.sqrt()).collect();
        for si in 0..ns {
            let above = if si + 1 < ns { v[si + 1] } else { 0.0 };
            forces[dir][si] = (v[si] - above).max(0.0);
        }
    }
    modes.sort_by(|a, b| b.0.total_cmp(&a.0));
    Some(Modal { modes, forces })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spectrum_and_the_forces() {
        let mut m = Model::new("Sismo");
        m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        m.add_slab(archisim_core::NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                archisim_core::Point2::new(0.0, 0.0),
                archisim_core::Point2::new(8.0, 0.0),
                archisim_core::Point2::new(8.0, 8.0),
                archisim_core::Point2::new(0.0, 8.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let mut site = *m.structural_site();
        site.seismic_ab = 0.12;
        site.soil = 1.3;
        m.set_structural_site(site).unwrap();
        let td = crate::takedown(&m);
        let q = quake(&m, &td);
        assert!(q.applies);
        // ρ·ab = 0,12 > 0,1: S = 1,04 + 3,33·0,02·(1 − 1,04).
        assert!((q.s - (1.04 + 3.33 * 0.02 * (1.0 - 1.04))).abs() < 1e-9);
        assert_eq!(q.storeys.len(), 1);
        let (_, w, f) = q.storeys[0];
        assert!(f > 0.0 && f < w, "{f} {w}");
    }

    #[test]
    fn the_frame_takes_the_earthquake() {
        let mut m = Model::new("Pórtico sísmico");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (0.0, 5.0), (5.0, 5.0)] {
            m.add_column(archisim_core::NewColumn {
                member_type: lib.column,
                level: l0,
                position: archisim_core::Point2::new(x, y),
                rotation: 0.0,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        }
        for (a, b) in [
            ((0.0, 0.0), (5.0, 0.0)),
            ((5.0, 0.0), (5.0, 5.0)),
            ((5.0, 5.0), (0.0, 5.0)),
            ((0.0, 5.0), (0.0, 0.0)),
        ] {
            m.add_beam(archisim_core::NewBeam {
                member_type: lib.beam,
                level: l1,
                start: archisim_core::Point2::new(a.0, a.1),
                end: archisim_core::Point2::new(b.0, b.1),
                top_offset: 0.0,
            })
            .unwrap();
        }
        m.add_slab(archisim_core::NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                archisim_core::Point2::new(0.0, 0.0),
                archisim_core::Point2::new(5.0, 0.0),
                archisim_core::Point2::new(5.0, 5.0),
                archisim_core::Point2::new(0.0, 5.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let mut site = *m.structural_site();
        site.seismic_ab = 0.16;
        m.set_structural_site(site).unwrap();
        let an = crate::analysis::analyse(&m);
        assert!(
            an.solutions.contains_key(&crate::Case::Quake(true)),
            "{:?}",
            an.cases
        );
        // The frame's modes: one storey, so one mode in each direction
        // carries all the mass, with T = 2π·√(m/k).
        let q = quake(&m, &an.takedown);
        let modal = an.model.modal.clone().expect("modal analysis");
        assert!(
            modal.modes.iter().any(|m| m.1 > 0.95) && modal.modes.iter().any(|m| m.2 > 0.95),
            "{modal:?}"
        );
        let total: f64 = modal.forces[0].iter().sum();
        let sol = &an.solutions[&crate::Case::Quake(true)];
        let top = an
            .model
            .frame
            .nodes
            .iter()
            .position(|n| n[2] > 2.9)
            .unwrap();
        let k = total / sol.displacements[top][0];
        let w: f64 = q.storeys.iter().map(|s| s.1).sum();
        let t = 2.0 * std::f64::consts::PI * (w / 9.81 / k).sqrt();
        let tx = modal.modes.iter().find(|m| m.1 > 0.5).unwrap().0;
        assert!((tx - t).abs() < 0.05 * t, "{tx} {t}");
        // Its force is the spectrum's at that period on the whole mass.
        let expect = q.ac * alpha(site.seismic_k, site.soil, tx) * q.beta * w;
        assert!((total - expect).abs() < 0.03 * expect, "{total} {expect}");
        // What the earthquake puts on the frame comes back at its supports.
        let back: f64 = sol.reactions.iter().map(|r| r.1[0]).sum();
        assert!(
            (total + back).abs() < 1e-3 * total.max(1.0),
            "{total} {back}"
        );
    }
}
