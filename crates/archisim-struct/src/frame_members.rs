//! The steel and timber beams and columns of the frame, from the forces of
//! each combination along them:
//!
//! - hot-rolled and hollow steel (DB-SE-A and EN 1993-1-1): axial force,
//!   bending about both axes, shear, flexural buckling of the columns
//!   (curves a, b and c), lateral-torsional buckling of the beams that
//!   no floor holds (C1 1,13), and the linear interaction;
//! - timber, solid or glulam, rectangular (DB-SE-M and EN 1995-1-1):
//!   bending, shear, compression with buckling and tension, with the kmod
//!   of each combination.
//!
//! The columns are taken as braced (buckling length their height): the
//! sway of the frame is checked apart (`stability`).

use archisim_core::{Model, Profile, TimberClass};

use crate::analysis::{Analysis, Envelope};
use crate::analytical::Owner;
use crate::cases::Kind as ComboKind;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::{profile_props, Kind};
use crate::text::dec;

/// DB-SE-A, 2.3.3.
pub const GAMMA_M0: f64 = 1.05;
pub const GAMMA_M1: f64 = 1.05;
const E_STEEL: f64 = 210e6;
const G_STEEL: f64 = 81e6;

fn status(u: f64) -> Status {
    if u <= 1.0 {
        Status::Pass
    } else {
        Status::Fail
    }
}

/// χ of a buckling curve with imperfection α at relative slenderness λ̄.
pub fn chi(alpha: f64, lambda: f64) -> f64 {
    if lambda <= 0.2 {
        return 1.0;
    }
    let phi = 0.5 * (1.0 + alpha * (lambda - 0.2) + lambda * lambda);
    (1.0 / (phi + (phi * phi - lambda * lambda).max(0.0).sqrt())).min(1.0)
}

/// What the checks need of a steel profile: area, I strong and weak,
/// torsion constant, elastic or plastic moduli, shear area, warping
/// constant and the buckling curves about each axis.
#[derive(Debug, Clone, Copy)]
pub struct SteelSection {
    pub a: f64,
    pub iy: f64,
    pub iz: f64,
    pub it: f64,
    pub wy: f64,
    pub wz: f64,
    pub av: f64,
    pub iw: f64,
    pub curve_y: f64,
    pub curve_z: f64,
    /// Open sections can twist out: lateral-torsional buckling.
    pub open: bool,
}

pub fn steel_section(p: Profile) -> SteelSection {
    let (a, iy, iz, it) = profile_props(p);
    let (b, h) = p.size();
    let wel_y = iy / (h / 2.0).max(1e-6);
    let wel_z = iz / (b / 2.0).max(1e-6);
    match p {
        Profile::IShape {
            width,
            depth,
            web,
            flange,
        } => {
            let hw = depth - 2.0 * flange;
            let wpl_y = width * flange * (depth - flange) + web * hw * hw / 4.0;
            let wpl_z = flange * width * width / 2.0 + hw * web * web / 4.0;
            let slender = depth / width > 1.2;
            SteelSection {
                a,
                iy,
                iz,
                it,
                wy: wpl_y,
                wz: wpl_z,
                av: depth * web,
                iw: iz * (depth - flange).powi(2) / 4.0,
                curve_y: if slender { 0.21 } else { 0.34 },
                curve_z: if slender { 0.34 } else { 0.49 },
                open: true,
            }
        }
        Profile::RectHollow { width, depth, wall } => {
            let wpl_y = width * depth * depth / 4.0
                - (width - 2.0 * wall) * (depth - 2.0 * wall).powi(2) / 4.0;
            let wpl_z = depth * width * width / 4.0
                - (depth - 2.0 * wall) * (width - 2.0 * wall).powi(2) / 4.0;
            SteelSection {
                a,
                iy,
                iz,
                it,
                wy: wpl_y,
                wz: wpl_z,
                av: a * depth / (width + depth),
                iw: 0.0,
                curve_y: 0.21,
                curve_z: 0.21,
                open: false,
            }
        }
        Profile::CircleHollow { diameter, wall } => {
            let wpl = (diameter.powi(3) - (diameter - 2.0 * wall).powi(3)) / 6.0;
            SteelSection {
                a,
                iy,
                iz,
                it,
                wy: wpl,
                wz: wpl,
                av: 2.0 * a / std::f64::consts::PI,
                iw: 0.0,
                curve_y: 0.21,
                curve_z: 0.21,
                open: false,
            }
        }
        Profile::Rectangle { width, depth } => SteelSection {
            a,
            iy,
            iz,
            it,
            wy: width * depth * depth / 4.0,
            wz: depth * width * width / 4.0,
            av: a,
            iw: 0.0,
            curve_y: 0.49,
            curve_z: 0.49,
            open: false,
        },
        Profile::Circle { diameter } => SteelSection {
            a,
            iy,
            iz,
            it,
            wy: diameter.powi(3) / 6.0,
            wz: diameter.powi(3) / 6.0,
            av: a,
            iw: 0.0,
            curve_y: 0.49,
            curve_z: 0.49,
            open: false,
        },
        // Channels, tees and angles: elastic, curve c.
        _ => SteelSection {
            a,
            iy,
            iz,
            it,
            wy: wel_y,
            wz: wel_z,
            av: a / 2.0,
            iw: 0.0,
            curve_y: 0.49,
            curve_z: 0.49,
            open: true,
        },
    }
}

/// The elastic critical moment of an open section over an unbraced length
/// (C1 1,13, load at the shear centre), kN·m.
pub fn mcr(s: &SteelSection, l: f64) -> f64 {
    let pi2 = std::f64::consts::PI.powi(2);
    let ez = E_STEEL * s.iz;
    1.13 * pi2 * ez / (l * l)
        * ((s.iw / s.iz.max(1e-12)) + l * l * G_STEEL * s.it / (pi2 * ez)).sqrt()
}

/// Whether a floor rests on a beam (and so holds its top flange).
fn braced_by_floor(model: &Model, id: archisim_core::BeamId) -> bool {
    let Some(b) = model.beam(id) else {
        return false;
    };
    let mid = archisim_core::Point2::new((b.start.x + b.end.x) / 2.0, (b.start.y + b.end.y) / 2.0);
    let d = b.end - b.start;
    let Some(n) = d.normalized() else {
        return false;
    };
    let side = archisim_core::Vec2::new(-n.y, n.x) * 0.3;
    model.slabs().filter(|s| s.level == b.level).any(|s| {
        crate::geo::inside(mid + side, &s.outline) || crate::geo::inside(mid - side, &s.outline)
    })
}

fn owner_info(model: &Model, o: Owner) -> Option<(String, Profile, Kind, f64, Subject)> {
    let (t, len, subject) = match o {
        Owner::Beam(id) => {
            let b = model.beam(id)?;
            (b.member_type, b.length(), Subject::Beam(id))
        }
        Owner::Column(id) => {
            let c = model.column(id)?;
            (c.member_type, c.height, Subject::Column(id))
        }
    };
    let mt = model.member_type(t)?;
    let kind = Kind::of(model.material(mt.material));
    Some((mt.name.clone(), mt.profile, kind, len, subject))
}

fn steel_checks(model: &Model, an: &Analysis, e: &Envelope, out: &mut Vec<Check>) {
    let Some((name, p, _, len, subject)) = owner_info(model, e.owner) else {
        return;
    };
    let fy = model.structural_site().materials.steel_fy * 1000.0;
    let s = steel_section(p);
    let column = matches!(e.owner, Owner::Column(_));
    let npl = s.a * fy / GAMMA_M0;
    let pi2 = std::f64::consts::PI.powi(2);
    let lb = len.max(0.1);
    let lam_y = (s.a * fy / (pi2 * E_STEEL * s.iy / (lb * lb))).sqrt();
    let lam_z = (s.a * fy / (pi2 * E_STEEL * s.iz / (lb * lb))).sqrt();
    let chi_n = chi(s.curve_y, lam_y).min(chi(s.curve_z, lam_z));
    let my_rd = s.wy * fy / GAMMA_M0;
    let mz_rd = s.wz * fy / GAMMA_M0;
    let v_rd = s.av * fy / (3f64.sqrt() * GAMMA_M0);
    // Lateral-torsional buckling of open beams no floor holds.
    let chi_lt = match e.owner {
        Owner::Beam(id) if s.open && !braced_by_floor(model, id) => {
            let m = mcr(&s, lb);
            let lam = (s.wy * fy / m).sqrt();
            chi(if s.curve_y < 0.3 { 0.34 } else { 0.49 }, lam)
        }
        _ => 1.0,
    };
    let mut worst = (0.0f64, 0usize, [0.0; 6]);
    let mut shear = 0.0f64;
    for (ci, list) in e.by_combination.iter().enumerate() {
        let k = an.combinations[ci].kind;
        if !matches!(k, ComboKind::Ultimate | ComboKind::Seismic) {
            continue;
        }
        for f in list {
            let n = f[0];
            let nb = if n < 0.0 && column { chi_n * npl } else { npl };
            let u = n.abs() / nb + f[4].abs() / (chi_lt * my_rd) + f[5].abs() / mz_rd;
            if u > worst.0 {
                worst = (u, ci, *f);
            }
            shear = shear.max(f[2].abs().max(f[1].abs()) / v_rd);
        }
    }
    let fy_n = model.structural_site().materials.steel_fy;
    let f = worst.2;
    out.push(Check {
        subject,
        what: format!(
            "{name} (S{fy_n:.0}): N {} kN, My {} kN·m, Mz {} kN·m; resiste N {} kN (χ {}), My {} kN·m{}, Mz {} kN·m",
            dec(f[0], 1),
            dec(f[4], 1),
            dec(f[5], 1),
            dec(if column { chi_n * npl } else { npl }, 0),
            dec(if column { chi_n } else { 1.0 }, 2),
            dec(chi_lt * my_rd, 1),
            if chi_lt < 1.0 {
                format!(" (vuelco lateral, χLT {})", dec(chi_lt, 2))
            } else {
                String::new()
            },
            dec(mz_rd, 1)
        ),
        rule: if column {
            "Pilar de acero: compresión, pandeo y flexión (DB-SE-A, 6.3)"
        } else {
            "Viga de acero: flexión y vuelco lateral (DB-SE-A, 6.3.3)"
        },
        ratio: worst.0,
        status: status(worst.0),
        advice: (worst.0 > 1.0).then(|| "un perfil mayor o arriostrar (pandeo y vuelco)".into()),
    });
    out.push(Check {
        subject,
        what: format!("{name}: cortante, resiste {} kN", dec(v_rd, 1)),
        rule: "Cortante del perfil (DB-SE-A, 6.2.4)",
        ratio: shear,
        status: status(shear),
        advice: (shear > 1.0).then(|| "un perfil con más alma".into()),
    });
}

/// Strength class values, N/mm²: fm, ft0, fc0, fc90, fv, E0mean, E005.
pub fn timber_values(c: TimberClass) -> [f64; 7] {
    match c {
        TimberClass::C18 => [18.0, 11.0, 18.0, 2.2, 3.4, 9000.0, 6000.0],
        TimberClass::C24 => [24.0, 14.5, 21.0, 2.5, 4.0, 11000.0, 7400.0],
        TimberClass::C30 => [30.0, 19.0, 24.0, 2.7, 4.0, 12000.0, 8000.0],
        TimberClass::GL24h => [24.0, 19.2, 24.0, 2.5, 3.5, 11500.0, 9600.0],
        TimberClass::GL28h => [28.0, 22.3, 28.0, 2.5, 3.5, 12600.0, 10500.0],
        TimberClass::GL32h => [32.0, 25.6, 32.0, 2.5, 3.5, 14200.0, 11800.0],
    }
}

fn timber_checks(model: &Model, an: &Analysis, e: &Envelope, out: &mut Vec<Check>) {
    let Some((name, p, _, len, subject)) = owner_info(model, e.owner) else {
        return;
    };
    let Profile::Rectangle { width, depth } = p else {
        return;
    };
    let class = model.structural_site().materials.timber;
    let [fm, ft0, fc0, _, fv, _, e005] = timber_values(class);
    let gm = if class.glulam() { 1.25 } else { 1.3 };
    let column = matches!(e.owner, Owner::Column(_));
    let (b, h) = (width * 1000.0, depth * 1000.0);
    let area = b * h;
    let wy = b * h * h / 6.0;
    let wz = h * b * b / 6.0;
    let lb = len.max(0.1) * 1000.0;
    let lam_rel = |i: f64| lb / i / std::f64::consts::PI * (fc0 / e005).sqrt();
    let kc = |l: f64| {
        let beta = if class.glulam() { 0.1 } else { 0.2 };
        if l <= 0.3 {
            1.0
        } else {
            let k = 0.5 * (1.0 + beta * (l - 0.3) + l * l);
            (1.0 / (k + (k * k - l * l).max(0.0).sqrt())).min(1.0)
        }
    };
    let kcy = kc(lam_rel(h / 12f64.sqrt()));
    let kcz = kc(lam_rel(b / 12f64.sqrt()));
    let altitude = model.site_altitude();
    let mut worst = (0.0f64, [0.0; 6], 0.0);
    for (ci, list) in e.by_combination.iter().enumerate() {
        let combo = &an.combinations[ci];
        if !matches!(combo.kind, ComboKind::Ultimate | ComboKind::Seismic) {
            continue;
        }
        let kmod = crate::timber::kmod_of(combo, altitude);
        let d = |f: f64| kmod * f / gm;
        for f in list {
            // N/mm² from kN and kN·m.
            let sn = f[0] * 1000.0 / area;
            let smy = f[4].abs() * 1e6 / wy;
            let smz = f[5].abs() * 1e6 / wz;
            let tau = 1.5 * f[2].abs().max(f[1].abs()) * 1000.0 / (0.67 * area);
            let km = 0.7;
            let bend = smy / d(fm) + km * smz / d(fm);
            let axial = if sn < 0.0 {
                let c = -sn;
                if column {
                    (c / (kcy * d(fc0)) + smy / d(fm) + km * smz / d(fm))
                        .max(c / (kcz * d(fc0)) + km * smy / d(fm) + smz / d(fm))
                } else {
                    (c / d(fc0)).powi(2) + bend
                }
            } else {
                sn / d(ft0) + bend
            };
            let u = axial.max(smy / d(fm) + km * smz / d(fm)).max(tau / d(fv));
            if u > worst.0 {
                worst = (u, *f, kmod);
            }
        }
    }
    let f = worst.1;
    out.push(Check {
        subject,
        what: format!(
            "{name} ({}): N {} kN, My {} kN·m, V {} kN, kmod {}{}",
            class.code(),
            dec(f[0], 1),
            dec(f[4], 1),
            dec(f[2].abs().max(f[1].abs()), 1),
            dec(worst.2, 2),
            if column {
                format!(", kc {} / {}", dec(kcy, 2), dec(kcz, 2))
            } else {
                String::new()
            }
        ),
        rule: if column {
            "Pilar de madera: compresión, pandeo y flexión (DB-SE-M, 6.3)"
        } else {
            "Viga de madera: flexión y cortante (DB-SE-M, 6.1)"
        },
        ratio: worst.0,
        status: status(worst.0),
        advice: (worst.0 > 1.0).then(|| "más sección o una clase resistente mayor".into()),
    });
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let mut out = Vec::new();
    for e in &an.envelopes {
        match an.model.kinds.get(&e.owner) {
            Some(Kind::Steel) => steel_checks(model, an, e, &mut out),
            Some(Kind::Timber) => timber_checks(model, an, e, &mut out),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckling_curves() {
        // EN 1993-1-1 table 6.1 / figure 6.4: λ̄ 1,0 gives 0,666 (a),
        // 0,597 (b), 0,538 (c).
        assert!((chi(0.21, 1.0) - 0.666).abs() < 0.005);
        assert!((chi(0.34, 1.0) - 0.597).abs() < 0.005);
        assert!((chi(0.49, 1.0) - 0.538).abs() < 0.005);
    }

    #[test]
    fn an_ipe_300() {
        // IPE 300: A 53,8 cm², Wpl,y 628 cm³ (with root radii; without
        // them a little less), Iz 604 cm⁴.
        let s = steel_section(Profile::IShape {
            width: 0.15,
            depth: 0.3,
            web: 0.0071,
            flange: 0.0107,
        });
        assert!((s.a * 1e4 - 53.8).abs() < 4.0, "{}", s.a * 1e4);
        assert!((s.wy * 1e6 - 628.0).abs() < 40.0, "{}", s.wy * 1e6);
        assert!((s.iz * 1e8 - 604.0).abs() < 30.0, "{}", s.iz * 1e8);
        // Mcr of an IPE 300 over 6 m with C1 1,13: some 85–100 kN·m.
        let m = mcr(&s, 6.0);
        assert!((70.0..120.0).contains(&m), "{m}");
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    use archisim_core::*;

    /// A steel table: four HEB 160 and four IPE 300 under a floor.
    fn steel_table(m: &mut Model) {
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.5);
        let lib = archisim_library::add_library(m).unwrap();
        let ty = |m: &Model, n: &str| m.member_types().find(|t| t.name.contains(n)).unwrap().id;
        let (heb, ipe) = (ty(m, "HEB 160"), ty(m, "IPE 300"));
        let pts = [(0.0, 0.0), (6.0, 0.0), (6.0, 5.0), (0.0, 5.0)];
        for (x, y) in pts {
            m.add_column(NewColumn {
                member_type: heb,
                level: l0,
                position: Point2::new(x, y),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.5,
            })
            .unwrap();
        }
        for k in 0..4 {
            let (a, b) = (pts[k], pts[(k + 1) % 4]);
            m.add_beam(NewBeam {
                member_type: ipe,
                level: l1,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                top_offset: 0.0,
            })
            .unwrap();
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: pts.iter().map(|p| Point2::new(p.0, p.1)).collect(),
            top_offset: 0.0,
        })
        .unwrap();
    }

    #[test]
    fn a_steel_table_is_checked() {
        let mut m = Model::new("Mesa de acero");
        steel_table(&mut m);
        let mut site = *m.structural_site();
        site.seismic_ab = 0.12;
        m.set_structural_site(site).unwrap();
        let an = crate::analysis::analyse(&m);
        let c = check(&m, &an);
        assert!(
            c.iter().any(|c| c.rule.starts_with("Pilar de acero")),
            "{c:?}"
        );
        assert!(
            c.iter().any(|c| c.rule.starts_with("Viga de acero")),
            "{c:?}"
        );
        for c in &c {
            assert!(c.ratio.is_finite() && c.ratio > 0.0, "{c:?}");
        }
        // The frame sways in the earthquake: its drift and θ are reported.
        let s = crate::stability::check(&m, &an);
        assert!(
            s.iter().any(|c| c.rule.starts_with("Segundo orden")),
            "{s:?}"
        );
        let st = crate::stability::storeys(&an);
        assert!(
            st.iter()
                .any(|s| s.drift.abs() > 0.0 && s.shear.abs() > 0.0),
            "{st:?}"
        );
    }
}
