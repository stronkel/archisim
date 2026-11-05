//! The earthquake by NCSE-02's simplified method (3.7): the design
//! acceleration (2.2), the fundamental period (3.7.2.2), the elastic
//! spectrum (2.3) and the equivalent static force of each storey for the
//! first mode, in each direction.

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
    let ta = k * c / 10.0;
    let tb = k * c / 2.5;
    let alpha = if period < ta {
        1.0 + 1.5 * period / ta
    } else if period <= tb {
        2.5
    } else {
        k * c / period
    };
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
        // What the earthquake puts on the frame comes back at its supports.
        let q = quake(&m, &an.takedown);
        let total: f64 = q.storeys.iter().map(|s| s.2).sum();
        let back: f64 = an.solutions[&crate::Case::Quake(true)]
            .reactions
            .iter()
            .map(|r| r.1[0])
            .sum();
        assert!(
            (total + back).abs() < 1e-3 * total.max(1.0),
            "{total} {back}"
        );
    }
}
