//! Solid timber C24 (EN 338, DB-SE-M and EN 1995-1-1): its strengths, the
//! modification factor by load duration, buckling of columns (6.3.2) and
//! the resistances of a rectangular piece.

use crate::cases::{Case, Combination};
use crate::UseCategory;

/// C24, N/mm².
pub const FM: f64 = 24.0;
pub const FT0: f64 = 14.5;
pub const FC0: f64 = 21.0;
pub const FC90: f64 = 2.5;
pub const FV: f64 = 4.0;
pub const E0_MEAN: f64 = 11_000.0;
pub const E005: f64 = 7_400.0;
pub const G_MEAN: f64 = 690.0;
/// Characteristic density, kg/m³.
pub const RHO_K: f64 = 350.0;
/// Partial factor for solid timber (DB-SE-M, table 2.3).
pub const GAMMA_M: f64 = 1.3;
/// Creep factor in service classes 1 and 2 (indoors, sheltered).
pub const K_DEF: f64 = 0.6;
/// Shear crack factor (EN 1995-1-1, 6.1.7).
pub const K_CR: f64 = 0.67;

/// How long an action lasts (DB-SE-M, table 2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Duration {
    Permanent,
    Long,
    Medium,
    Short,
    Instant,
}

impl Duration {
    /// kmod of solid timber in service classes 1 and 2.
    pub fn kmod(self) -> f64 {
        match self {
            Duration::Permanent => 0.6,
            Duration::Long => 0.7,
            Duration::Medium => 0.8,
            Duration::Short => 0.9,
            Duration::Instant => 1.1,
        }
    }

    pub fn of(case: Case, altitude: f64) -> Duration {
        match case {
            Case::Dead => Duration::Permanent,
            Case::Live(UseCategory::A2) => Duration::Long,
            Case::Live(UseCategory::G1 | UseCategory::G2) => Duration::Short,
            Case::Live(_) => Duration::Medium,
            Case::Snow if altitude > 1000.0 => Duration::Medium,
            Case::Snow | Case::Wind(_) => Duration::Short,
            Case::Quake(_) => Duration::Instant,
        }
    }
}

/// kmod of a combination: that of its shortest action (EN 1995-1-1,
/// 3.1.3(2)).
pub fn kmod_of(c: &Combination, altitude: f64) -> f64 {
    c.factors
        .iter()
        .filter(|(_, f)| *f != 0.0)
        .map(|(case, _)| Duration::of(*case, altitude).kmod())
        .fold(Duration::Permanent.kmod(), f64::max)
}

/// The design value of a strength, N/mm².
pub fn design(fk: f64, kmod: f64) -> f64 {
    kmod * fk / GAMMA_M
}

/// The buckling factor kc of a column of relative slenderness λrel
/// (EN 1995-1-1, 6.3.2, βc 0,2 for solid timber).
pub fn kc(lambda_rel: f64) -> f64 {
    if lambda_rel <= 0.3 {
        return 1.0;
    }
    let k = 0.5 * (1.0 + 0.2 * (lambda_rel - 0.3) + lambda_rel * lambda_rel);
    (1.0 / (k + (k * k - lambda_rel * lambda_rel).max(0.0).sqrt())).min(1.0)
}

/// A rectangular piece of timber: `b` across (the narrow side, in the
/// wall's plane for a stud), `h` its depth, mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub b: f64,
    pub h: f64,
}

impl Piece {
    pub fn from_section(s: &archisim_core::FrameSection) -> Piece {
        Piece {
            b: s.width * 1000.0,
            h: s.depth * 1000.0,
        }
    }

    pub fn area(&self) -> f64 {
        self.b * self.h
    }

    pub fn w_strong(&self) -> f64 {
        self.b * self.h * self.h / 6.0
    }

    pub fn i_strong(&self) -> f64 {
        self.b * self.h.powi(3) / 12.0
    }

    /// The relative slenderness about the strong (h) and the weak (b)
    /// axis over buckling lengths in m.
    pub fn lambda_rel(&self, lcr: f64, strong: bool) -> f64 {
        let i = if strong { self.h } else { self.b } / 12f64.sqrt();
        let lambda = lcr * 1000.0 / i;
        lambda / std::f64::consts::PI * (FC0 / E005).sqrt()
    }

    /// The utilisation of a stud or post in compression and bending about
    /// its strong axis (EN 1995-1-1, 6.3.2(3), expression 6.23 with the
    /// bending of the strong axis): N kN, M kN·m.
    pub fn compression_bending(
        &self,
        n: f64,
        m: f64,
        lcr_strong: f64,
        lcr_weak: Option<f64>,
        kmod: f64,
    ) -> f64 {
        let sc = n * 1000.0 / self.area();
        let sm = m.abs() * 1e6 / self.w_strong();
        let fc = design(FC0, kmod);
        let fm = design(FM, kmod);
        let ky = kc(self.lambda_rel(lcr_strong, true));
        let mut u = sc / (ky * fc) + sm / fm;
        if let Some(lw) = lcr_weak {
            let kz = kc(self.lambda_rel(lw, false));
            // About the weak axis, the strong axis bending with km 0,7.
            u = u.max(sc / (kz * fc) + 0.7 * sm / fm);
        }
        u
    }

    /// The utilisation in bending (braced against lateral buckling) and
    /// in shear: M kN·m, V kN.
    pub fn bending_shear(&self, m: f64, v: f64, kmod: f64) -> (f64, f64) {
        let sm = m.abs() * 1e6 / self.w_strong();
        let tau = 1.5 * v.abs() * 1000.0 / (K_CR * self.area());
        (sm / design(FM, kmod), tau / design(FV, kmod))
    }

    /// The utilisation in tension: N kN.
    pub fn tension(&self, n: f64, kmod: f64) -> f64 {
        n.abs() * 1000.0 / self.area() / design(FT0, kmod)
    }

    /// Bearing across the grain of a sole plate under a stud: N kN on its
    /// width `b` × the plate's width `w` (mm), kc,90 1,25.
    pub fn bearing(&self, n: f64, plate_width: f64, kmod: f64) -> f64 {
        let a = self.b * plate_width;
        n.abs() * 1000.0 / a / (1.25 * design(FC90, kmod))
    }
}

/// The lateral resistance of a nail or screw of diameter `d` (mm) fixing a
/// board of `t1` mm (OSB or plywood) to timber with a penetration of `t2`
/// mm, single shear, kN (EN 1995-1-1, 8.2.2, expression 8.6, without the
/// rope effect).
pub fn fastener_board(d: f64, t1: f64, t2: f64, kmod: f64) -> f64 {
    // Embedment strengths, N/mm²: OSB (8.22) and timber without
    // predrilling (8.15).
    let fh1 = 65.0 * d.powf(-0.7) * t1.powf(0.1);
    let fh2 = 0.082 * RHO_K * d.powf(-0.3);
    let beta = fh2 / fh1;
    // Yield moment of a round nail (8.14), fu 600 N/mm².
    let my = 0.3 * 600.0 * d.powf(2.6);
    let a = fh1 * t1 * d;
    let b = fh2 * t2 * d;
    let c = fh1 * t1 * d / (1.0 + beta)
        * ((beta
            + 2.0 * beta * beta * (1.0 + t2 / t1 + (t2 / t1).powi(2))
            + beta.powi(3) * (t2 / t1).powi(2))
        .sqrt()
            - beta * (1.0 + t2 / t1));
    let d_ = 1.05 * fh1 * t1 * d / (2.0 + beta)
        * ((2.0 * beta * (1.0 + beta) + 4.0 * beta * (2.0 + beta) * my / (fh1 * d * t1 * t1))
            .sqrt()
            - beta);
    let e = 1.05 * fh1 * t2 * d / (1.0 + 2.0 * beta)
        * ((2.0 * beta * beta * (1.0 + beta)
            + 4.0 * beta * (1.0 + 2.0 * beta) * my / (fh1 * d * t2 * t2))
            .sqrt()
            - beta);
    let f = 1.15 * (2.0 * beta / (1.0 + beta)).sqrt() * (2.0 * my * fh1 * d).sqrt();
    let rk = [a, b, c, d_, e, f]
        .into_iter()
        .filter(|v| v.is_finite() && *v > 0.0)
        .fold(f64::INFINITY, f64::min);
    // Connections: γM 1,3.
    kmod * rk / GAMMA_M / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stud_buckles_and_bends() {
        let s = Piece { b: 45.0, h: 145.0 };
        // λ about the weak axis over 2,7 m is large; kc small.
        let weak = kc(s.lambda_rel(2.7, false));
        let strong = kc(s.lambda_rel(2.7, true));
        assert!(weak < 0.3 && strong > 0.5, "{weak} {strong}");
        let u = s.compression_bending(20.0, 0.5, 2.7, None, 0.9);
        assert!(u > 0.0 && u < 1.0, "{u}");
        // EN 1995: 2,8 mm nails in 12 mm OSB, about 0,5 kN.
        let f = fastener_board(2.8, 12.0, 38.0, 0.9);
        assert!((0.3..1.0).contains(&f), "{f}");
        assert_eq!(
            Duration::of(Case::Wind(crate::WindDir::PosX), 0.0),
            Duration::Short
        );
    }
}
