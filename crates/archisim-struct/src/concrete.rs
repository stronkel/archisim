//! Reinforced concrete (Código Estructural, articles of the old EHE-08 kept
//! in its annex 19 and EN 1992-1-1): the design of rectangular sections
//! in bending, in shear and in compression with bending, with HA-25 and
//! B500S. Sections in m, forces in kN and kN·m.

use archisim_core::{bar_area, Counted, Spaced};

pub const GAMMA_C: f64 = 1.5;
pub const GAMMA_S: f64 = 1.15;

/// N/mm² → kN/m².
const K: f64 = 1000.0;

/// The concrete and the reinforcing steel of the project.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rc {
    /// N/mm².
    pub fck: f64,
    pub fyk: f64,
}

impl Default for Rc {
    /// HA-25 and B500S.
    fn default() -> Self {
        Rc {
            fck: 25.0,
            fyk: 500.0,
        }
    }
}

impl Rc {
    pub fn of(model: &archisim_core::Model) -> Rc {
        let m = model.structural_site().materials;
        Rc {
            fck: m.concrete_fck,
            fyk: m.rebar_fyk,
        }
    }

    /// «HA-25 / B500S».
    pub fn name(&self) -> String {
        format!(
            "HA-{} / B{}S",
            self.fck.round() as i64,
            self.fyk.round() as i64
        )
    }

    pub fn fcd(&self) -> f64 {
        self.fck / GAMMA_C * K
    }

    pub fn fyd(&self) -> f64 {
        self.fyk / GAMMA_S * K
    }

    /// Mean tensile strength fctm, N/mm².
    pub fn fctm(&self) -> f64 {
        0.30 * self.fck.powf(2.0 / 3.0)
    }

    /// Secant modulus Ecm, kN/m² (8500·∛fcm).
    pub fn ecm(&self) -> f64 {
        8500.0 * (self.fck + 8.0).cbrt() * K
    }

    /// The bottom (or top) steel a rectangular section `b` × `d` needs for a
    /// moment `m` (kN·m), m²; `None` if the concrete cannot take it without
    /// compression steel (x/d > 0,45). Rectangular stress block 0,8·x.
    pub fn bending_steel(&self, m: f64, b: f64, d: f64) -> Option<f64> {
        let m = m.abs();
        if m < 1e-9 {
            return Some(0.0);
        }
        let mu = m / (b * d * d * self.fcd());
        let lim = 0.8 * 0.45 * (1.0 - 0.4 * 0.45);
        if mu > lim {
            return None;
        }
        let omega = 1.0 - (1.0 - 2.0 * mu).sqrt();
        Some(omega * b * d * self.fcd() / self.fyd())
    }

    /// The minimum steel of a beam in tension (geometric 2,8 ‰ of B500S
    /// and mechanical 0,04·Ac·fcd/fyd), m².
    pub fn min_beam_steel(&self, b: f64, h: f64) -> f64 {
        (0.0028 * b * h).max(0.04 * b * h * self.fcd() / self.fyd())
    }

    /// The shear the concrete takes without stirrups, kN (art. 44, EN
    /// 1992-1-1 6.2.2), with ρl the ratio of tension steel.
    pub fn vrd_c(&self, b: f64, d: f64, rho_l: f64) -> f64 {
        let dmm = d * 1000.0;
        let xi = (1.0 + (200.0 / dmm).sqrt()).min(2.0);
        let rho = rho_l.min(0.02);
        let v1 = 0.18 / GAMMA_C * xi * (100.0 * rho * self.fck).powf(1.0 / 3.0);
        let vmin = 0.075 / GAMMA_C * xi.powf(1.5) * self.fck.sqrt();
        v1.max(vmin) * K * b * d
    }

    /// The crushing of the struts, kN (θ 45°).
    pub fn vrd_max(&self, b: f64, d: f64) -> f64 {
        0.3 * self.fcd() * b * d
    }

    /// Stirrups for a shear `v`: area per metre (m²/m), the strut at 45°.
    pub fn stirrups(&self, v: f64, b: f64, d: f64, rho_l: f64) -> f64 {
        let vc = self.vrd_c(b, d, rho_l);
        let need = (v.abs() - vc).max(0.0) / (0.9 * d * self.fyd());
        // Minimum: 0,02·b·fcd/fyd per metre (art. 44.2.3.4.1, fct form).
        need.max(0.02 * b * self.fcd() / self.fyd() * 0.5)
    }

    /// The fewest bars of one diameter that give `area`, at least `min`.
    pub fn bars_for(&self, area: f64, min: u32) -> Counted {
        for d in [0.012, 0.016, 0.020, 0.025] {
            let n = (area / bar_area(d)).ceil().max(min as f64) as u32;
            if n <= 6 {
                return Counted::new(n, d);
            }
        }
        let d = 0.025;
        Counted::new((area / bar_area(d)).ceil().max(min as f64) as u32, d)
    }

    /// Stirrups of 8 (or 10) mm for an area per metre, two legs, spacing to
    /// whole centimetres, no more than `max`.
    pub fn stirrups_for(&self, per_m: f64, max: f64) -> Spaced {
        for d in [0.008, 0.010, 0.012] {
            let s = 2.0 * bar_area(d) / per_m.max(1e-9);
            let s = ((s * 100.0).floor() / 100.0).min(max);
            if s >= 0.07 {
                return Spaced::new(d, s);
            }
        }
        Spaced::new(0.012, 0.07)
    }

    /// The resistance of a rectangular column `b` × `h` with `as_total`
    /// steel in two equal layers at its faces (bending about the side `h`):
    /// the moment it takes with an axial compression `n` (kN, compression
    /// positive), kN·m; `None` if `n` is beyond its capacity. Strain
    /// compatibility with the rectangular block and εcu 3,5 ‰.
    pub fn column_moment(&self, n: f64, b: f64, h: f64, as_total: f64, cover: f64) -> Option<f64> {
        let fcd = self.fcd();
        let fyd = self.fyd();
        let es = 200e6;
        let a1 = as_total / 2.0;
        let d = h - cover;
        let d2 = cover;
        let n_max = 0.85 * fcd * b * h + as_total * fyd;
        if n > n_max {
            return None;
        }
        // Find x so that the section's axial force is n (bisection).
        let force = |x: f64| -> (f64, f64) {
            let x = x.max(1e-6);
            let y = (0.8 * x).min(h);
            let c = 0.85 * fcd * b * y;
            let strain = |di: f64| 0.0035 * (x - di) / x;
            let sig = |e: f64| (e * es).clamp(-fyd, fyd);
            let s_top = sig(strain(d2)) * a1;
            let s_bot = sig(strain(d)) * a1;
            let nn = c + s_top + s_bot;
            // Moment about the centre of the section.
            let m = c * (h / 2.0 - y / 2.0) + s_top * (h / 2.0 - d2) - s_bot * (d - h / 2.0);
            (nn, m)
        };
        let (mut lo, mut hi) = (1e-4, 10.0 * h);
        if force(lo).0 > n {
            // Even all in tension it carries more: pure tension side.
            return Some(force(lo).1.max(0.0));
        }
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if force(mid).0 < n {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Some(force(0.5 * (lo + hi)).1.max(0.0))
    }

    /// The bars a column needs for (N, My, Mz) on a section `b` (across, z)
    /// × `h` (y): the lightest of the usual layouts whose capacity covers
    /// My/MRy + Mz/MRz ≤ 1 at that N, with the minimum eccentricity; and its
    /// utilisation.
    pub fn column_bars(&self, n: f64, my: f64, mz: f64, b: f64, h: f64) -> (Counted, f64) {
        let cover = 0.04;
        let ey = (h / 20.0).max(0.02);
        let ez = (b / 20.0).max(0.02);
        let my = my.abs().max(n.max(0.0) * ey);
        let mz = mz.abs().max(n.max(0.0) * ez);
        let amin = (0.004 * b * h).max(0.1 * n.max(0.0) / self.fyd());
        let options = [
            (4, 0.012),
            (4, 0.016),
            (4, 0.020),
            (8, 0.016),
            (8, 0.020),
            (4, 0.025),
            (8, 0.025),
            (12, 0.020),
            (12, 0.025),
            (16, 0.025),
        ];
        let mut best = (Counted::new(16, 0.025), f64::INFINITY);
        for (c, d) in options {
            let bars = Counted::new(c, d);
            let a = bars.area();
            if a < amin || a > 0.04 * b * h {
                continue;
            }
            let u = self.column_check(n, my, mz, b, h, a, cover);
            if u <= 1.0 {
                return (bars, u);
            }
            if u < best.1 {
                best = (bars, u);
            }
        }
        best
    }

    /// The bending strength of a round column of diameter `dia` with
    /// `count` bars of total area `as_total` evenly round it at `cover`
    /// to their centre, under an axial force `n` (kN): strain
    /// compatibility with the rectangular block, the concrete integrated
    /// in strips (Código Estructural, art. 42).
    pub fn circle_moment(
        &self,
        n: f64,
        dia: f64,
        count: u32,
        as_total: f64,
        cover: f64,
    ) -> Option<f64> {
        let fcd = self.fcd();
        let fyd = self.fyd();
        let r = dia / 2.0;
        let rs = r - cover;
        let count = count.max(1);
        let a1 = as_total / f64::from(count);
        let area = std::f64::consts::PI * r * r;
        if n > 0.85 * fcd * area + as_total * fyd {
            return None;
        }
        // Bars at their depth from the top of the section; the worst
        // position of the set is close to one bar on the axis.
        let bars: Vec<f64> = (0..count)
            .map(|i| {
                let a = std::f64::consts::TAU * f64::from(i) / f64::from(count);
                r - rs * a.cos()
            })
            .collect();
        let strips = 60;
        let force = |x: f64| -> (f64, f64) {
            let x = x.max(1e-6);
            let y = (0.8 * x).min(dia);
            let (mut nn, mut m) = (0.0, 0.0);
            for k in 0..strips {
                let t0 = y * f64::from(k) / f64::from(strips);
                let t1 = y * f64::from(k + 1) / f64::from(strips);
                let t = 0.5 * (t0 + t1);
                let half = (r * r - (r - t).powi(2)).max(0.0).sqrt();
                let c = 0.85 * fcd * 2.0 * half * (t1 - t0);
                nn += c;
                m += c * (r - t);
            }
            for di in &bars {
                let e = 0.0035 * (x - di) / x;
                let sig = (e * 200e6).clamp(-fyd, fyd);
                nn += sig * a1;
                m += sig * a1 * (r - di);
            }
            (nn, m)
        };
        let (mut lo, mut hi) = (1e-4, 10.0 * dia);
        if force(lo).0 > n {
            return Some(force(lo).1.max(0.0));
        }
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if force(mid).0 < n {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Some(force(0.5 * (lo + hi)).1.max(0.0))
    }

    /// The utilisation of a round column: a round section has no weak
    /// axis, so the two moments add as a vector.
    pub fn circle_check(&self, n: f64, m: f64, dia: f64, bars: &Counted, cover: f64) -> f64 {
        match self.circle_moment(n, dia, bars.count, bars.area(), cover) {
            Some(mr) if mr > 0.0 => m.abs() / mr,
            _ => 9.99,
        }
    }

    /// The bars a round column asks for (at least six, art. 54): the
    /// first set that works, with its utilisation.
    pub fn circle_bars(&self, n: f64, m: f64, dia: f64) -> (Counted, f64) {
        let area = std::f64::consts::PI * dia * dia / 4.0;
        let m = m.abs().max(n.max(0.0) * (dia / 20.0).max(0.02));
        let amin = (0.004 * area).max(0.1 * n.max(0.0) / self.fyd());
        let mut best = (Counted::new(12, 0.025), f64::INFINITY);
        for (c, d) in [
            (6, 0.012),
            (6, 0.016),
            (8, 0.016),
            (6, 0.020),
            (8, 0.020),
            (10, 0.020),
            (8, 0.025),
            (12, 0.020),
            (10, 0.025),
            (12, 0.025),
        ] {
            let bars = Counted::new(c, d);
            if bars.area() < amin || bars.area() > 0.04 * area {
                continue;
            }
            let u = self.circle_check(n, m, dia, &bars, 0.046);
            if u <= 1.0 {
                return (bars, u);
            }
            if u < best.1 {
                best = (bars, u);
            }
        }
        best
    }

    /// The utilisation of a column with `as_total` steel.
    #[allow(clippy::too_many_arguments)]
    pub fn column_check(
        &self,
        n: f64,
        my: f64,
        mz: f64,
        b: f64,
        h: f64,
        as_total: f64,
        cover: f64,
    ) -> f64 {
        let ry = self.column_moment(n, b, h, as_total, cover);
        let rz = self.column_moment(n, h, b, as_total, cover);
        match (ry, rz) {
            (Some(ry), Some(rz)) if ry > 0.0 && rz > 0.0 => my.abs() / ry + mz.abs() / rz,
            _ => 9.99,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beam_needs_its_steel() {
        // 30 × 50, d 0,45, 150 kN·m: μ = 150/(0,3·0,2025·16 667) = 0,148,
        // ω = 0,161, As = 0,161·0,3·0,45·16 667/434 783 = 8,33 cm².
        let rc = Rc::default();
        let a = rc.bending_steel(150.0, 0.3, 0.45).unwrap();
        assert!((a * 1e4 - 8.33).abs() < 0.05, "{}", a * 1e4);
        assert!(rc.bending_steel(800.0, 0.3, 0.45).is_none());
        let bars = rc.bars_for(a, 2);
        assert!(bars.area() >= a);
        // Shear of a 30 × 50 with ρ 0,6 %: some 60–80 kN.
        let v = rc.vrd_c(0.3, 0.45, 0.006);
        assert!((50.0..90.0).contains(&v), "{v}");
        // A 30 × 30 column with 4Ø16 under 600 kN: ν 0,40, ω 0,23 give
        // μ ≈ 0,19 in the charts, about 85 kN·m.
        let m = rc
            .column_moment(600.0, 0.3, 0.3, 4.0 * bar_area(0.016), 0.04)
            .unwrap();
        assert!((70.0..100.0).contains(&m), "{m}");
        let (b, u) = rc.column_bars(900.0, 40.0, 10.0, 0.3, 0.3);
        assert!(u <= 1.0, "{} {u}", b.label());
        // A round column of Ø35 (the area of a 31 × 31) with 6Ø16: about
        // the strength of the square one with the same steel, a little
        // less (π/4 of the lever).
        let round = rc
            .circle_moment(600.0, 0.35, 6, 6.0 * bar_area(0.016), 0.046)
            .unwrap();
        let square = rc
            .column_moment(600.0, 0.31, 0.31, 6.0 * bar_area(0.016), 0.046)
            .unwrap();
        assert!(
            round > 0.6 * square && round < 1.1 * square,
            "{round} {square}"
        );
        let (b, u) = rc.circle_bars(900.0, 40.0, 0.35);
        assert!(u <= 1.0 && b.count >= 6, "{} {u}", b.label());
    }
}
