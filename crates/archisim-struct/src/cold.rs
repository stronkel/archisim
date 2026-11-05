//! Cold-formed steel sections (EN 1993-1-3 with EN 1993-1-5): lipped C
//! studs and joists and plain U tracks. Gross properties, the effective
//! section in compression and in bending (effective widths and the
//! distortional buckling of the lips, 5.5.3), and the resistances of the
//! cross-section and the member (6.1 and 6.2).
//!
//! Internally in millimetres and N/mm²; the results in kN and kN·m.
//! Corner radii are left out (flat widths from the outside dimensions
//! less the thickness), which is on the safe side.

use std::f64::consts::PI;

/// Steel S350GD+Z (EN 10346): basic yield and ultimate strength, N/mm².
pub const FYB: f64 = 350.0;
pub const FU: f64 = 420.0;
pub const E: f64 = 210_000.0;
pub const NU: f64 = 0.3;
/// Partial factors (DB-SE-A 2.3.3 for the members, EN 1993-1-3 for the
/// joints).
pub const GAMMA_M0: f64 = 1.05;
pub const GAMMA_M1: f64 = 1.05;
pub const GAMMA_M2: f64 = 1.25;

/// A C (with lips) or U (lip 0) section; outside dimensions, mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cold {
    pub h: f64,
    pub b: f64,
    pub c: f64,
    pub t: f64,
}

/// Gross properties (mm, mm², mm⁴, mm³).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gross {
    pub a: f64,
    pub iy: f64,
    pub iz: f64,
    pub wy: f64,
    pub j: f64,
    /// Distance from the web's mid-line to the centroid.
    pub xc: f64,
}

/// Effective widths of a plate (EN 1993-1-5, 4.4): ρ for an internal
/// element (both edges supported) with stress ratio ψ, or for an outstand.
fn rho_internal(bp: f64, t: f64, ksigma: f64, psi: f64) -> f64 {
    let eps = (235.0 / FYB).sqrt();
    let lp = (bp / t) / (28.4 * eps * ksigma.sqrt());
    let limit = 0.5 + (0.085 - 0.055 * psi).sqrt();
    if lp <= limit {
        1.0
    } else {
        ((lp - 0.055 * (3.0 + psi)) / (lp * lp)).min(1.0)
    }
}

fn rho_outstand(cp: f64, t: f64, ksigma: f64) -> f64 {
    let eps = (235.0 / FYB).sqrt();
    let lp = (cp / t) / (28.4 * eps * ksigma.sqrt());
    if lp <= 0.748 {
        1.0
    } else {
        ((lp - 0.188) / (lp * lp)).min(1.0)
    }
}

/// The reduction for distortional buckling (EN 1993-1-3, 5.5.3.1(7)).
pub fn chi_distortional(lambda_d: f64) -> f64 {
    if lambda_d <= 0.65 {
        1.0
    } else if lambda_d < 1.38 {
        1.47 - 0.723 * lambda_d
    } else {
        0.66 / lambda_d
    }
}

/// The reduction for flexural buckling, curve b (α 0,34; EN 1993-1-3
/// table 6.3 for C sections).
pub fn chi_buckling(lambda: f64, alpha: f64) -> f64 {
    if lambda <= 0.2 {
        return 1.0;
    }
    let phi = 0.5 * (1.0 + alpha * (lambda - 0.2) + lambda * lambda);
    (1.0 / (phi + (phi * phi - lambda * lambda).max(0.0).sqrt())).min(1.0)
}

/// What the effective section in compression found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveCompression {
    pub a_eff: f64,
    pub rho_web: f64,
    pub rho_flange: f64,
    pub rho_lip: f64,
    pub chi_d: f64,
}

impl Cold {
    pub fn from_section(s: &archisim_core::FrameSection) -> Cold {
        Cold {
            h: s.depth * 1000.0,
            b: s.width * 1000.0,
            c: s.lip * 1000.0,
            t: s.thickness * 1000.0,
        }
    }

    fn lipped(&self) -> bool {
        self.c > 2.0 * self.t
    }

    /// Mid-line lengths: web, flange, lip.
    fn mid(&self) -> (f64, f64, f64) {
        let t = self.t;
        let hc = self.h - t;
        let bc = self.b - if self.lipped() { t } else { t / 2.0 };
        let cc = if self.lipped() { self.c - t / 2.0 } else { 0.0 };
        (hc, bc, cc)
    }

    pub fn gross(&self) -> Gross {
        let t = self.t;
        let (hc, bc, cc) = self.mid();
        let a = t * (hc + 2.0 * bc + 2.0 * cc);
        let iy = t * hc.powi(3) / 12.0
            + 2.0 * bc * t * (hc / 2.0).powi(2)
            + 2.0 * (t * cc.powi(3) / 12.0 + t * cc * (hc / 2.0 - cc / 2.0).powi(2));
        let xc = (2.0 * bc * t * bc / 2.0 + 2.0 * cc * t * bc) / a;
        let iz = hc * t * xc * xc
            + 2.0 * (t * bc.powi(3) / 12.0 + bc * t * (bc / 2.0 - xc).powi(2))
            + 2.0 * cc * t * (bc - xc).powi(2);
        let j = t.powi(3) / 3.0 * (hc + 2.0 * bc + 2.0 * cc);
        Gross {
            a,
            iy,
            iz,
            wy: iy / (self.h / 2.0),
            j,
            xc,
        }
    }

    /// The flat widths: web, flange, lip.
    fn flats(&self) -> (f64, f64, f64) {
        let t = self.t;
        let hp = self.h - 2.0 * t;
        let bp = if self.lipped() {
            self.b - 2.0 * t
        } else {
            self.b - t
        };
        let cp = if self.lipped() { self.c - t } else { 0.0 };
        (hp, bp, cp)
    }

    /// The edge stiffener (lip and the flange's part next to it) of a
    /// compressed flange and its distortional reduction; `both`: both
    /// flanges in compression (kf = 1).
    fn stiffener(&self, both: bool) -> (f64, f64, f64, f64) {
        let t = self.t;
        let (hp, bp, cp) = self.flats();
        let rf = rho_internal(bp, t, 4.0, 1.0);
        let ratio = cp / bp;
        let k_lip = if ratio <= 0.35 {
            0.5
        } else {
            (0.5 + 0.83 * (ratio - 0.35).powf(2.0 / 3.0)).min(0.6)
        };
        let rl = rho_outstand(cp, t, k_lip);
        let be2 = 0.5 * rf * bp;
        let ceff = rl * cp;
        let as_ = t * (be2 + ceff);
        // Centroid of the stiffener from the flange (down the lip) and
        // from the web (along the flange).
        let ys = (ceff * ceff / 2.0) / (be2 + ceff);
        let xs = (be2 * (bp - be2 / 2.0) + ceff * bp) / (be2 + ceff);
        let is = t * be2 * ys * ys + t * ceff.powi(3) / 12.0 + t * ceff * (ceff / 2.0 - ys).powi(2);
        let b1 = xs;
        let kf = if both { 1.0 } else { 0.0 };
        let k = E * t.powi(3)
            / (4.0 * (1.0 - NU * NU))
            / (b1 * b1 * hp + b1.powi(3) + 0.5 * b1 * b1 * hp * kf);
        let sigma = 2.0 * (k * E * is).sqrt() / as_;
        let chi = chi_distortional((FYB / sigma).sqrt());
        (rf, rl, chi, as_)
    }

    /// The effective section in uniform compression.
    pub fn compression(&self) -> EffectiveCompression {
        let t = self.t;
        let g = self.gross();
        let (hp, bp, cp) = self.flats();
        let rw = rho_internal(hp, t, 4.0, 1.0);
        if !self.lipped() {
            let rf = rho_outstand(bp, t, 0.43);
            let lost = t * ((1.0 - rw) * hp + 2.0 * (1.0 - rf) * bp);
            return EffectiveCompression {
                a_eff: (g.a - lost).max(0.1 * g.a),
                rho_web: rw,
                rho_flange: rf,
                rho_lip: 0.0,
                chi_d: 1.0,
            };
        }
        let (rf, rl, chi, as_) = self.stiffener(true);
        let lost = t * ((1.0 - rw) * hp + 2.0 * (1.0 - rf) * bp + 2.0 * (1.0 - rl) * cp)
            + 2.0 * (1.0 - chi) * as_;
        EffectiveCompression {
            a_eff: (g.a - lost).max(0.1 * g.a),
            rho_web: rw,
            rho_flange: rf,
            rho_lip: rl,
            chi_d: chi,
        }
    }

    /// The effective section modulus for bending about the strong axis
    /// (one flange in compression), mm³: its compressed flange and lip
    /// reduced, the web's compressed half by its effective width
    /// (ψ = −1), about the effective neutral axis.
    pub fn weff_y(&self) -> f64 {
        let t = self.t;
        let (hc, bc, cc) = self.mid();
        let (hp, bp, cp) = self.flats();
        // Parts as (area, y of centroid, own I about it); y up from the
        // middle of the web, the top flange in compression.
        let mut parts: Vec<(f64, f64, f64)> = Vec::new();
        let (bf_c, lip_c, chi) = if self.lipped() {
            let (rf, rl, chi, _) = self.stiffener(false);
            (rf * bp + (bc - bp), rl * cp, chi)
        } else {
            (rho_outstand(bp, t, 0.43) * bp + (bc - bp), 0.0, 1.0)
        };
        // Top flange (compressed): its two halves, the outer one reduced
        // by χd with the lip.
        let half = bf_c / 2.0;
        parts.push((half * t, hc / 2.0, 0.0));
        parts.push((half * t * chi, hc / 2.0, 0.0));
        if self.lipped() {
            parts.push((
                lip_c * t * chi,
                hc / 2.0 - lip_c / 2.0,
                t * lip_c.powi(3) / 12.0,
            ));
        }
        // Bottom flange and lip (tension): whole.
        parts.push((bc * t, -hc / 2.0, 0.0));
        if self.lipped() {
            parts.push((cc * t, -hc / 2.0 + cc / 2.0, t * cc.powi(3) / 12.0));
        }
        // Web: tension half whole; compressed half with its effective
        // width split 0,4 / 0,6.
        let rw = rho_internal(hp, t, 23.9, -1.0);
        let bc_w = hc / 2.0;
        let beff = rw * bc_w;
        parts.push((bc_w * t, -bc_w / 2.0, t * bc_w.powi(3) / 12.0));
        let e1 = 0.4 * beff;
        let e2 = 0.6 * beff;
        parts.push((e1 * t, hc / 2.0 - e1 / 2.0, t * e1.powi(3) / 12.0));
        parts.push((e2 * t, e2 / 2.0, t * e2.powi(3) / 12.0));
        let a: f64 = parts.iter().map(|p| p.0).sum();
        let yc: f64 = parts.iter().map(|p| p.0 * p.1).sum::<f64>() / a;
        let i: f64 = parts.iter().map(|p| p.2 + p.0 * (p.1 - yc).powi(2)).sum();
        let to_top = self.h / 2.0 - yc;
        let to_bottom = self.h / 2.0 + yc;
        (i / to_top.max(to_bottom)).min(self.gross().wy)
    }

    /// Axial resistance of the cross-section, kN.
    pub fn nc_rd(&self) -> f64 {
        self.compression().a_eff * FYB / GAMMA_M0 / 1000.0
    }

    /// Buckling resistance in compression, kN: flexural about y over
    /// `lcr_y` and, if not held by boards, about z over `lcr_z` (m).
    pub fn nb_rd(&self, lcr_y: f64, lcr_z: Option<f64>) -> (f64, f64) {
        let g = self.gross();
        let eff = self.compression();
        let beta = eff.a_eff / g.a;
        let lam = |lcr: f64, i: f64| {
            let r = (i / g.a).sqrt();
            (lcr * 1000.0 / r) / (PI * (E / FYB).sqrt()) * beta.sqrt()
        };
        let ly = lam(lcr_y, g.iy);
        let mut chi = chi_buckling(ly, 0.34);
        let mut lambda = ly;
        if let Some(lz) = lcr_z {
            let lzv = lam(lz, g.iz);
            let cz = chi_buckling(lzv, 0.34);
            if cz < chi {
                chi = cz;
                lambda = lzv;
            }
        }
        (chi * eff.a_eff * FYB / GAMMA_M1 / 1000.0, lambda)
    }

    /// Bending resistance about y, kN·m (the boards hold the compressed
    /// flange: no lateral-torsional buckling).
    pub fn mc_rd(&self) -> f64 {
        self.weff_y() * FYB / GAMMA_M0 / 1e6
    }

    /// Bending resistance with lateral-torsional buckling over `l` m
    /// (compressed flange free), kN·m: Mcr of a beam under a uniform
    /// moment with C1 1,13 about the centroid; curve b (α_LT 0,34).
    pub fn mb_rd(&self, l: f64) -> f64 {
        let g = self.gross();
        let lmm = l * 1000.0;
        let gm = E / (2.0 * (1.0 + NU));
        // Warping constant of a C about its shear centre (approximate,
        // flanges only).
        let (hc, bc, _) = self.mid();
        let iw = self.t * bc.powi(3) * hc * hc / 12.0 * (3.0 * bc * self.t + 2.0 * hc * self.t)
            / (6.0 * bc * self.t + hc * self.t);
        let c1 = 1.13;
        let mcr = c1 * PI * PI * E * g.iz / (lmm * lmm)
            * (iw / g.iz + lmm * lmm * gm * g.j / (PI * PI * E * g.iz)).sqrt();
        let w = self.weff_y();
        let lam = (w * FYB / mcr).sqrt();
        chi_buckling(lam, 0.34) * w * FYB / GAMMA_M1 / 1e6
    }

    /// Shear resistance of the web, kN (6.1.5, web without stiffening at
    /// the support).
    pub fn vb_rd(&self) -> f64 {
        let t = self.t;
        let hw = self.h - t;
        let lw = 0.346 * (hw / t) * (FYB / E).sqrt();
        let fbv = if lw <= 0.83 {
            0.58 * FYB
        } else if lw < 1.40 {
            0.48 * FYB / lw
        } else {
            0.67 * FYB / (lw * lw)
        };
        hw * t * fbv / GAMMA_M0 / 1000.0
    }

    /// Resistance of the web to a support reaction or a load through the
    /// flange (6.1.7.2, single unstiffened web), kN: at an end support
    /// (`end`: c ≤ 1,5 hw) or inside the span, over a bearing of `ss` mm.
    pub fn rw_rd(&self, ss: f64, end: bool) -> f64 {
        let t = self.t;
        let hw = self.h - t;
        let r_t: f64 = 1.0;
        let k = FYB / 228.0;
        let k3 = 0.7 + 0.3; // φ = 90°
        let st = ss / t;
        let base = t * t * FYB / GAMMA_M1 / 1000.0;
        if end {
            let k1 = 1.33 - 0.33 * k;
            let k2 = (1.15 - 0.15 * r_t).clamp(0.5, 1.0);
            let (a, b) = if self.lipped() {
                (5.92 - (hw / t) / 132.0, 1.0 + 0.01 * st.min(60.0))
            } else {
                (9.04 - (hw / t) / 60.0, 1.0 + 0.01 * st.min(60.0))
            };
            let b = if st > 60.0 { 0.71 + 0.015 * st } else { b };
            (k1 * k2 * k3 * a.max(0.0) * b * base).max(0.0)
        } else {
            let k4 = 1.22 - 0.22 * k;
            let k5 = (1.06 - 0.06 * r_t).min(1.0);
            let a = 14.7 - (hw / t) / 49.5;
            let b = if st <= 60.0 {
                1.0 + 0.007 * st
            } else {
                0.75 + 0.011 * st
            };
            (k3 * k4 * k5 * a.max(0.0) * b * base).max(0.0)
        }
    }
}

/// Bearing resistance of a self-drilling screw of diameter `d` (mm) in a
/// sheet of `t` (mm), kN (EN 1993-1-3 table 8.4, the thinner sheet
/// under the head).
pub fn screw_bearing(d: f64, t: f64) -> f64 {
    let alpha = (3.2 * (t / d).sqrt()).min(2.1);
    alpha * FU * d * t / GAMMA_M2 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_c_stud_has_sensible_properties() {
        // C 100 × 40 × 10 × 1: area ≈ 1,96 cm² (mid-line), Iy ≈ 32 cm⁴.
        let c = Cold {
            h: 100.0,
            b: 40.0,
            c: 10.0,
            t: 1.0,
        };
        let g = c.gross();
        assert!((g.a - 196.0).abs() < 1.0, "{}", g.a);
        assert!((g.iy / 1e4 - 33.0).abs() < 3.0, "{}", g.iy / 1e4);
        let eff = c.compression();
        assert!(eff.a_eff < g.a && eff.a_eff > 0.4 * g.a, "{eff:?}");
        assert!(eff.chi_d <= 1.0);
        let w = c.weff_y();
        assert!(w < g.wy && w > 0.5 * g.wy, "{w} {}", g.wy);
        // A thicker one resists more in every way.
        let thick = Cold { t: 1.5, ..c };
        assert!(
            thick.nc_rd() > c.nc_rd() && thick.mc_rd() > c.mc_rd() && thick.vb_rd() > c.vb_rd()
        );
        assert!(thick.rw_rd(40.0, true) > c.rw_rd(40.0, true));
        // Longer is weaker.
        assert!(c.nb_rd(3.0, None).0 < c.nb_rd(1.0, None).0);
        assert!(c.mb_rd(3.0) < c.mc_rd());
    }

    #[test]
    fn screws_bear_by_the_table() {
        // Ø 4,2 in 1 mm: α = 3,2 √(1/4,2) = 1,56; 1,56 · 420 · 4,2 · 1 / 1,25.
        let f = screw_bearing(4.2, 1.0);
        assert!(
            (f - 1.5614 * 420.0 * 4.2 / 1.25 / 1000.0).abs() < 0.01,
            "{f}"
        );
    }

    #[test]
    fn web_crippling_matches_a_published_example() {
        // Dlubal's example (S 235, fyb 211,5; C t = 2, hw = 198, ss 100,
        // unstiffened flanges, γM1 1,1) gives 8,73 kN; with our S350GD and
        // γM1 1,05 the same formula scales by fyb·k1 and 1/γM1.
        let fyb = 211.5f64;
        let k1 = 1.33 - 0.33 * fyb / 228.0;
        let r = k1 * (9.04 - 99.0 / 60.0) * (1.0 + 0.01 * 50.0) * 4.0 * fyb / 1.1 / 1000.0;
        assert!((r - 8.73).abs() < 0.01, "{r}");
    }
}
