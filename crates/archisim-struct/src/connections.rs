//! The joints of the steel frame (EN 1993-1-8, DB-SE-A 8):
//!
//! - the base plates of the steel columns that stand on a footing: plate
//!   100 mm round the profile, bearing on the concrete fjd = βj·kj·fcd
//!   (2/3 · 1,5), its thickness for that pressure over the overhang, the
//!   anchor bolts (4.6) for the uplift that the bending leaves and the
//!   shear by friction (0,2) and by the bolts;
//! - the shear joints of the ends of the steel beams (hinged): a fin
//!   plate with a row of 8.8 bolts in single shear, as many as the end
//!   shear asks for, and whether they fit in the web;
//! - with rigid steel joints, a bolted end plate for the end moment and
//!   the shear of the column web panel;
//! - timber beams hung on a slotted-in steel plate with dowels (EN
//!   1995-1-1, 8.2.3, with the splitting of 8.1.4) and the steel shoes of
//!   timber columns on their footing.

use std::collections::BTreeMap;

use archisim_core::{Model, Profile, TimberClass};

use crate::analysis::{Analysis, Envelope};
use crate::analytical::{Analytical, Owner};
use crate::cases::Kind as ComboKind;
use crate::concrete::Rc;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

pub const GAMMA_M2: f64 = 1.25;

/// Stressed area of a bolt (ISO 898), m².
pub fn bolt_area(d: f64) -> f64 {
    match (d * 1000.0).round() as i64 {
        12 => 84.3e-6,
        16 => 157e-6,
        20 => 245e-6,
        24 => 353e-6,
        27 => 459e-6,
        30 => 561e-6,
        _ => std::f64::consts::PI / 4.0 * (0.85 * d).powi(2),
    }
}

/// Ultimate tensile strength of a structural steel, N/mm².
fn steel_fu(fy: f64) -> f64 {
    if fy <= 240.0 {
        360.0
    } else if fy <= 280.0 {
        430.0
    } else {
        490.0
    }
}

/// The base plate of a column `b` × `h` (m) for an axial force `n`
/// (compression positive), moments and shear, kN and kN·m.
#[derive(Debug, Clone, PartialEq)]
pub struct BasePlate {
    pub width: f64,
    pub length: f64,
    pub thickness: f64,
    pub bolts: u32,
    pub bolt: f64,
    /// Utilisations: bearing, anchors in tension, shear.
    pub bearing: f64,
    pub tension: f64,
    pub shear: f64,
}

pub fn base_plate(rc: &Rc, fy: f64, b: f64, h: f64, n: f64, m: f64, v: f64) -> BasePlate {
    let c = 0.10;
    let (bp, lp) = (b + 2.0 * c, h + 2.0 * c);
    let fjd = 2.0 / 3.0 * 1.5 * rc.fcd();
    // Bearing: the compressed block under the plate (rectangular).
    let e = if n > 0.0 { m.abs() / n } else { f64::INFINITY };
    let (bearing, tension) = if n > 0.0 && e <= lp / 6.0 {
        ((n / (bp * lp)) * (1.0 + 6.0 * e / lp) / fjd, 0.0)
    } else {
        // Uplift side: anchors a little inside the edge, compression at
        // the far edge over a block of 0,2·lp.
        let z = lp - 0.05 - 0.1 * lp;
        let t = ((m.abs() - n.max(0.0) * (lp / 2.0 - 0.1 * lp)) / z).max(-n.min(0.0));
        let compression = (n.max(0.0) + t.max(0.0)) / (bp * 0.2 * lp);
        (compression / fjd, t.max(0.0))
    };
    // Thickness for fjd over the overhang c, to standard sizes.
    let need = c * (3.0 * fjd * 1.05 / (fy * 1000.0)).sqrt();
    let thickness = [0.012, 0.015, 0.020, 0.025, 0.030, 0.035, 0.040, 0.050]
        .into_iter()
        .find(|t| *t >= need)
        .unwrap_or(0.050);
    // Anchors 4.6 (fub 400): two on the side in tension (four if more is
    // needed).
    let ft = |d: f64| 0.9 * 400e3 * bolt_area(d) / GAMMA_M2;
    let mut pick = (4u32, 0.030);
    'out: for count in [4u32, 8] {
        for d in [0.016, 0.020, 0.024, 0.027, 0.030] {
            if f64::from(count) / 2.0 * ft(d) >= tension {
                pick = (count, d);
                break 'out;
            }
        }
    }
    let (bolts, bolt) = pick;
    let t_use = tension / (f64::from(bolts) / 2.0 * ft(bolt));
    // Shear: friction 0,2·N and the anchors (αbc = 0,44 − 0,0003·240).
    let fv = (0.44 - 0.0003 * 240.0) * 400e3 * bolt_area(bolt) / GAMMA_M2;
    let shear = v.abs() / (0.2 * n.max(0.0) + f64::from(bolts) * fv);
    BasePlate {
        width: bp,
        length: lp,
        thickness,
        bolts,
        bolt,
        bearing,
        tension: t_use,
        shear,
    }
}

/// The bolts of a fin plate for an end shear `v` (kN) on a web `tw` (m)
/// of clear height `hw`: (count, diameter, fits, utilisation).
pub fn fin_plate(fu: f64, v: f64, tw: f64, hw: f64) -> (u32, f64, bool, f64) {
    let mut best = (0u32, 0.020, false, f64::INFINITY);
    for d in [0.016, 0.020, 0.024] {
        let d0 = d + 0.002;
        let fv = 0.6 * 800e3 * bolt_area(d) / GAMMA_M2;
        // Bearing on the web: k1 2,5, αb 0,6.
        let fb = 2.5 * 0.6 * fu * 1000.0 * d * tw / GAMMA_M2;
        let each = fv.min(fb);
        let n = ((v.abs() / each).ceil() as u32).max(2);
        let room = ((hw - 3.0 * d0) / (3.0 * d0)).floor() as i64 + 1;
        let fits = i64::from(n) <= room;
        let u = v.abs() / (f64::from(n) * each);
        if fits {
            return (n, d, true, u);
        }
        if best.0 == 0 {
            best = (n, d, false, u);
        }
    }
    best
}

/// A bolted end plate of a rigid beam–column joint (EN 1993-1-8, 6.2.7):
/// the moment as a couple between the flanges, taken by rows of two 8.8
/// bolts by the tension flange (two rows inside an extended plate at
/// most), the shear by the two bolts of the compression side, and the
/// shear of the column web panel (6.2.6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct EndPlate {
    pub rows: u32,
    pub bolt: f64,
    pub thickness: f64,
    /// Utilisations: bolts in tension, bolts in shear, column web panel.
    pub tension: f64,
    pub shear: f64,
    pub panel: f64,
    /// Whether two rows are not enough (a haunch is needed).
    pub haunch: bool,
}

/// `m`, `v` (kN·m, kN) at the end of a beam of depth `depth` and flange
/// `flange`; the column web `col_web` (m) of depth `col_depth` and flange
/// `col_flange`, if there is one.
pub fn end_plate(
    fy: f64,
    m: f64,
    v: f64,
    depth: f64,
    flange: f64,
    column: Option<(f64, f64, f64)>,
) -> EndPlate {
    let z = (depth - flange).max(0.05);
    let pull = m.abs() / z;
    let ft = |d: f64| 0.9 * 800e3 * bolt_area(d) / GAMMA_M2;
    let fv = |d: f64| 0.6 * 800e3 * bolt_area(d) / GAMMA_M2;
    let mut pick = (2u32, 0.027, true);
    'out: for rows in [1u32, 2] {
        for d in [0.016, 0.020, 0.024, 0.027] {
            if f64::from(2 * rows) * ft(d) >= pull {
                pick = (rows, d, false);
                break 'out;
            }
        }
    }
    let (rows, bolt, haunch) = pick;
    let tension = pull / (f64::from(2 * rows) * ft(bolt));
    let shear = v.abs() / (2.0 * fv(bolt));
    // The plate about as thick as the bolt (T-stub in mode 2).
    let thickness = [0.015, 0.020, 0.025, 0.030]
        .into_iter()
        .find(|t| *t >= bolt * 0.9)
        .unwrap_or(0.030);
    let panel = column
        .map(|(cd, cf, cw)| {
            let avc = (cd - 2.0 * cf).max(0.0) * cw;
            let vrd = 0.9 * fy * 1000.0 * avc / 3f64.sqrt();
            if vrd > 0.0 {
                pull / vrd
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);
    EndPlate {
        rows,
        bolt,
        thickness,
        tension,
        shear,
        panel,
        haunch,
    }
}

/// Characteristic density of a strength class, kg/m³ (EN 338, EN 14080).
pub fn timber_density(c: TimberClass) -> f64 {
    match c {
        TimberClass::C18 => 320.0,
        TimberClass::C24 => 350.0,
        TimberClass::C30 => 380.0,
        TimberClass::GL24h => 385.0,
        TimberClass::GL28h => 425.0,
        TimberClass::GL32h => 440.0,
    }
}

/// The resistance of one steel dowel (S235, fu 360) through a slotted-in
/// steel plate of 10 mm in a timber piece `b` wide, in its two shear
/// planes (EN 1995-1-1, 8.2.3, modes f, g and h; 8.5.1 for the
/// embedment at `angle` to the grain), kN, with kmod 0,8 and γM 1,3.
pub fn dowel_resistance(rho: f64, d: f64, b: f64, angle: f64) -> f64 {
    let dm = d * 1000.0;
    let t1 = ((b - 0.010) / 2.0 * 1000.0).max(dm);
    let fh0 = 0.082 * (1.0 - 0.01 * dm) * rho;
    let k90 = 1.35 + 0.015 * dm;
    let (s, c) = angle.sin_cos();
    let fh = fh0 / (k90 * s * s + c * c);
    let my = 0.3 * 360.0 * dm.powf(2.6);
    let f = (fh * t1 * dm)
        .min(fh * t1 * dm * ((2.0 + 4.0 * my / (fh * dm * t1 * t1)).sqrt() - 1.0))
        .min(2.3 * (my * fh * dm).sqrt());
    2.0 * f / 1000.0 * 0.8 / 1.3
}

/// The dowels of a timber beam end hung on a slotted-in steel plate for
/// a shear `v` (kN), in a piece `b` × `h` (m), with the splitting check
/// (8.1.4): (count, diameter, columns, fits, use, splitting use).
pub fn timber_hanger(rho: f64, v: f64, b: f64, h: f64) -> (u32, f64, u32, bool, f64, f64) {
    let mut best = None;
    for d in [0.012, 0.016, 0.020] {
        let each = dowel_resistance(rho, d, b, std::f64::consts::FRAC_PI_2);
        let n = ((v.abs() / each).ceil() as u32).max(2);
        // Rows across the grain at 3d, 4d from the top and 3d from the
        // bottom; up to two columns at 5d.
        let per_column = (((h - 7.0 * d) / (3.0 * d)).floor() as i64 + 1).max(0) as u32;
        let columns = if n <= per_column { 1 } else { 2 };
        let fits = per_column > 0 && n <= per_column * columns;
        let u = v.abs() / (f64::from(n) * each);
        // The farthest dowel from the loaded (lower) edge is the top one.
        let he = h - 4.0 * d;
        let ratio = (he / h).min(0.99);
        let f90 = 14.0 * b * 1000.0 * (he * 1000.0 / (1.0 - ratio)).sqrt() / 1000.0 * 0.8 / 1.3;
        let split = v.abs() / f90;
        let pick = (n, d, columns, fits, u, split);
        if fits {
            return pick;
        }
        best.get_or_insert(pick);
    }
    best.unwrap_or((0, 0.0, 0, false, f64::INFINITY, 0.0))
}

/// Dowels, their diameter, the anchors' diameter and the uses of the
/// bearing, the dowels and the anchors.
pub type Shoe = (u32, f64, f64, f64, f64, f64);

/// The steel shoe of a timber column `b` × `h` (m) on a footing: the
/// column bears on the plate (fc,0), a slotted-in plate with dowels takes
/// the uplift (along the grain) and the shear (across it), and four
/// anchors 4.6 hold the shoe: (dowels, diameter, anchor diameter,
/// bearing use, dowel use, anchor use).
pub fn timber_shoe(rho: f64, fc0: f64, b: f64, h: f64, n: f64, v: f64) -> Shoe {
    let gm = 1.3;
    let fcd = 0.8 * fc0 * 1000.0 / gm;
    let bearing = n.max(0.0) / (b * h * fcd);
    let uplift = (-n).max(0.0);
    let d = 0.016;
    let along = dowel_resistance(rho, d, b, 0.0);
    let across = dowel_resistance(rho, d, b, std::f64::consts::FRAC_PI_2);
    // The interaction of the two directions, each by its own strength.
    let dowels = ((uplift / along + v.abs() / across).ceil() as u32).max(2);
    let dowel_use = (uplift / along + v.abs() / across) / f64::from(dowels);
    let ft = |a: f64| 0.9 * 400e3 * bolt_area(a) / GAMMA_M2;
    let fv = |a: f64| (0.44 - 0.0003 * 240.0) * 400e3 * bolt_area(a) / GAMMA_M2;
    let mut anchor = 0.020;
    let mut anchor_use = f64::INFINITY;
    for a in [0.012, 0.016, 0.020] {
        let u = uplift / (4.0 * ft(a)) + (v.abs() - 0.2 * n.max(0.0)).max(0.0) / (4.0 * fv(a));
        anchor = a;
        anchor_use = u;
        if u <= 1.0 {
            break;
        }
    }
    (dowels, d, anchor, bearing, dowel_use, anchor_use)
}

/// The nodes where the bars of an element end (the ones used once).
fn end_nodes(am: &Analytical, owner: Owner) -> Vec<usize> {
    let mut count: BTreeMap<usize, u32> = BTreeMap::new();
    for (bi, p) in am.pieces.iter().enumerate() {
        if p.owner == owner {
            let bar = &am.frame.bars[bi];
            *count.entry(bar.i).or_default() += 1;
            *count.entry(bar.j).or_default() += 1;
        }
    }
    count
        .into_iter()
        .filter(|(_, c)| *c == 1)
        .map(|(n, _)| n)
        .collect()
}

/// The column that meets a node, if any.
fn column_at(am: &Analytical, node: usize) -> Option<archisim_core::ColumnId> {
    am.pieces
        .iter()
        .enumerate()
        .find_map(|(bi, p)| match p.owner {
            Owner::Column(id) if am.frame.bars[bi].i == node || am.frame.bars[bi].j == node => {
                Some(id)
            }
            _ => None,
        })
}

/// The base node of a column, if it stands on a support.
fn base_of(am: &Analytical, owner: Owner) -> Option<usize> {
    let base = am
        .pieces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.owner == owner)
        .flat_map(|(bi, _)| [am.frame.bars[bi].i, am.frame.bars[bi].j])
        .min_by(|a, b| am.frame.nodes[*a][2].total_cmp(&am.frame.nodes[*b][2]))?;
    am.frame
        .supports
        .iter()
        .any(|(s, _)| *s == base)
        .then_some(base)
}

/// The ultimate and seismic reactions at a node, one per combination.
fn reactions_at(an: &Analysis, node: usize) -> Vec<[f64; 6]> {
    an.combinations
        .iter()
        .filter(|c| matches!(c.kind, ComboKind::Ultimate | ComboKind::Seismic))
        .map(|combo| {
            let mut r = [0.0; 6];
            for (case, f) in &combo.factors {
                if let Some(s) = an.solutions.get(case) {
                    if let Some((_, rr)) = s.reactions.iter().find(|(n, _)| *n == node) {
                        for d in 0..6 {
                            r[d] += f * rr[d];
                        }
                    }
                }
            }
            r
        })
        .collect()
}

/// The largest end moment and shear of a beam over its combinations.
fn end_forces(e: &Envelope) -> (f64, f64) {
    let mut m: f64 = 0.0;
    for forces in &e.by_combination {
        for f in [forces.first(), forces.last()].into_iter().flatten() {
            m = m.max(f[4].abs());
        }
    }
    (m, e.max[2].abs().max(e.min[2].abs()))
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let mut out = Vec::new();
    let rc = Rc::of(model);
    let materials = &model.structural_site().materials;
    let fy = materials.steel_fy;
    let class = materials.timber;
    let rho = timber_density(class);
    let am = &an.model;
    for (owner, kind) in &am.kinds {
        match (kind, owner) {
            (Kind::Steel, Owner::Column(id)) => {
                let Some(base) = base_of(am, *owner) else {
                    continue;
                };
                let Some(t) = model
                    .column(*id)
                    .and_then(|c| model.member_type(c.member_type))
                else {
                    continue;
                };
                let (b, h) = t.profile.size();
                // The worst ultimate reaction.
                let mut worst: Option<(f64, BasePlate, [f64; 6])> = None;
                for r in reactions_at(an, base) {
                    let m = r[3].hypot(r[4]);
                    let v = r[0].hypot(r[1]);
                    let p = base_plate(&rc, fy, b, h, r[2], m, v);
                    let u = p.bearing.max(p.tension).max(p.shear);
                    if worst.as_ref().is_none_or(|w| u > w.0) {
                        worst = Some((u, p, r));
                    }
                }
                let Some((u, p, r)) = worst else { continue };
                out.push(Check {
                    subject: Subject::Column(*id),
                    what: format!(
                        "{}: N {} kN, M {} kN·m, V {} kN. Placa {}×{}×{} mm con {} pernos M{} (4.6): compresión {}, pernos {}, cortante {}",
                        t.name,
                        dec(r[2], 1),
                        dec(r[3].hypot(r[4]), 1),
                        dec(r[0].hypot(r[1]), 1),
                        dec(p.width * 1000.0, 0),
                        dec(p.length * 1000.0, 0),
                        dec(p.thickness * 1000.0, 0),
                        p.bolts,
                        dec(p.bolt * 1000.0, 0),
                        dec(p.bearing, 2),
                        dec(p.tension, 2),
                        dec(p.shear, 2)
                    ),
                    rule: "Placa de anclaje (EN 1993-1-8, 6.2.5 y 6.2.6.12)",
                    ratio: u,
                    status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                    advice: (u > 1.0).then(|| {
                        "placa mayor, cartelas o más pernos; con mucho cortante, una llave bajo la placa".into()
                    }),
                });
            }
            (Kind::Steel, Owner::Beam(id)) => {
                let Some(t) = model
                    .beam(*id)
                    .and_then(|b| model.member_type(b.member_type))
                else {
                    continue;
                };
                let Profile::IShape {
                    depth, web, flange, ..
                } = t.profile
                else {
                    continue;
                };
                let Some(e) = an.envelopes.iter().find(|e| e.owner == *owner) else {
                    continue;
                };
                let (m_end, v) = end_forces(e);
                if materials.steel_rigid {
                    // The column it meets, for its web panel.
                    let column = end_nodes(am, *owner)
                        .into_iter()
                        .filter_map(|n| column_at(am, n))
                        .filter_map(|c| model.column(c))
                        .filter_map(|c| model.member_type(c.member_type))
                        .find_map(|t| match t.profile {
                            Profile::IShape {
                                depth, web, flange, ..
                            } => Some((depth, flange, web)),
                            _ => None,
                        });
                    let p = end_plate(fy, m_end, v, depth, flange, column);
                    let u = p.tension.max(p.shear).max(p.panel);
                    out.push(Check {
                        subject: Subject::Beam(*id),
                        what: format!(
                            "{}: en el apoyo M {} kN·m y V {} kN; chapa de testa de {} mm con {} fila{} de 2 tornillos M{} (8.8) a tracción{}: tornillos {}, cortante {}{}",
                            t.name,
                            dec(m_end, 1),
                            dec(v, 1),
                            dec(p.thickness * 1000.0, 0),
                            p.rows,
                            if p.rows == 1 { "" } else { "s" },
                            dec(p.bolt * 1000.0, 0),
                            if p.rows == 2 { " (chapa extendida)" } else { "" },
                            dec(p.tension, 2),
                            dec(p.shear, 2),
                            if column.is_some() {
                                format!(", panel del alma del pilar {}", dec(p.panel, 2))
                            } else {
                                String::new()
                            }
                        ),
                        rule: "Unión rígida con chapa de testa (EN 1993-1-8, 6.2.6 y 6.2.7)",
                        ratio: u,
                        status: if !p.haunch && u <= 1.0 {
                            Status::Pass
                        } else {
                            Status::Fail
                        },
                        advice: (p.haunch || u > 1.0).then(|| {
                            if p.panel > 1.0 {
                                "rigidizadores en el pilar a la altura de las alas y una chapa de refuerzo del alma".into()
                            } else {
                                "una cartela bajo la viga para alargar el brazo, o tornillos mayores".into()
                            }
                        }),
                    });
                } else {
                    let (n, d, fits, u) = fin_plate(steel_fu(fy), v, web, depth - 2.0 * flange);
                    out.push(Check {
                        subject: Subject::Beam(*id),
                        what: format!(
                            "{}: cortante en el apoyo {} kN; chapa de unión con {} tornillos M{} (8.8) en una fila{}",
                            t.name,
                            dec(v, 1),
                            n,
                            dec(d * 1000.0, 0),
                            if fits { "" } else { ", que no caben en el alma" }
                        ),
                        rule: "Unión articulada atornillada (EN 1993-1-8, 3.6)",
                        ratio: if fits { u } else { 1.5 },
                        status: if fits && u <= 1.0 {
                            Status::Pass
                        } else {
                            Status::Fail
                        },
                        advice: (!fits).then(|| {
                            "dos filas de tornillos, un angular doble o una unión soldada".into()
                        }),
                    });
                }
            }
            (Kind::Timber, Owner::Beam(id)) => {
                let Some(t) = model
                    .beam(*id)
                    .and_then(|b| model.member_type(b.member_type))
                else {
                    continue;
                };
                let Profile::Rectangle { width, depth } = t.profile else {
                    continue;
                };
                let Some(e) = an.envelopes.iter().find(|e| e.owner == *owner) else {
                    continue;
                };
                let (_, v) = end_forces(e);
                let (n, d, columns, fits, u, split) = timber_hanger(rho, v, width, depth);
                let worst = if fits { u.max(split) } else { 1.5 };
                out.push(Check {
                    subject: Subject::Beam(*id),
                    what: format!(
                        "{}: cortante en el apoyo {} kN; chapa de acero embebida de 10 mm con {} pasadores Ø{} en {} columna{}{}: pasadores {}, hienda {}",
                        t.name,
                        dec(v, 1),
                        n,
                        dec(d * 1000.0, 0),
                        columns,
                        if columns == 1 { "" } else { "s" },
                        if fits { "" } else { ", que no caben en el canto" },
                        dec(u, 2),
                        dec(split, 2)
                    ),
                    rule: "Unión de madera con pasadores (EN 1995-1-1, 8.2.3 y 8.1.4)",
                    ratio: worst,
                    status: if worst <= 1.0 {
                        Status::Pass
                    } else {
                        Status::Fail
                    },
                    advice: (worst > 1.0).then(|| {
                        if split > 1.0 {
                            "subir los pasadores hacia la cara de arriba de la viga o apoyarla en un estribo".into()
                        } else {
                            "un estribo de acero (colgador) o una viga con más canto".into()
                        }
                    }),
                });
            }
            (Kind::Timber, Owner::Column(id)) => {
                let Some(base) = base_of(am, *owner) else {
                    continue;
                };
                let Some(t) = model
                    .column(*id)
                    .and_then(|c| model.member_type(c.member_type))
                else {
                    continue;
                };
                let Profile::Rectangle { width, depth } = t.profile else {
                    continue;
                };
                let fc0 = crate::frame_members::timber_values(class)[2];
                let mut worst: Option<(f64, Shoe, [f64; 6])> = None;
                for r in reactions_at(an, base) {
                    let s = timber_shoe(
                        rho,
                        fc0,
                        width.min(depth),
                        width.max(depth),
                        r[2],
                        r[0].hypot(r[1]),
                    );
                    let u = s.3.max(s.5).max(if s.4 > 1.0 { s.4 } else { 0.0 });
                    if worst.as_ref().is_none_or(|w| u > w.0) {
                        worst = Some((u, s, r));
                    }
                }
                let Some((u, (dowels, d, anchor, bearing, _, anchors), r)) = worst else {
                    continue;
                };
                out.push(Check {
                    subject: Subject::Column(*id),
                    what: format!(
                        "{}: N {} kN, V {} kN. Herraje en U con chapa embebida, {} pasadores Ø{} y 4 pernos M{} (4.6): apoyo {}, pernos {}",
                        t.name,
                        dec(r[2], 1),
                        dec(r[0].hypot(r[1]), 1),
                        dowels,
                        dec(d * 1000.0, 0),
                        dec(anchor * 1000.0, 0),
                        dec(bearing, 2),
                        dec(anchors, 2)
                    ),
                    rule: "Anclaje del pilar de madera (EN 1995-1-1, 8.2.3; EN 1993-1-8, 6.2.6.12)",
                    ratio: u,
                    status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                    advice: (u > 1.0).then(|| {
                        "un pilar más grueso o un herraje con más pernos; separar la madera del suelo 2 cm".into()
                    }),
                });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plate_and_its_bolts() {
        // A HEB 200 with 500 kN of compression and no moment: 400 × 400
        // plate, σ 3,1 N/mm² on fjd 16,7: some 0,19.
        let rc = Rc::default();
        let p = base_plate(&rc, 275.0, 0.2, 0.2, 500.0, 0.0, 10.0);
        assert!(
            (p.bearing - 500.0 / 0.16 / (rc.fcd())).abs() < 0.01,
            "{p:?}"
        );
        assert_eq!(p.tension, 0.0);
        // With 60 kN·m the anchors pull.
        let p = base_plate(&rc, 275.0, 0.2, 0.2, 100.0, 60.0, 10.0);
        assert!(p.tension > 0.0 && p.tension <= 1.0, "{p:?}");
        // An IPE 300 end with 120 kN: two or three M16–M20.
        let (n, _, fits, u) = fin_plate(430.0, 120.0, 0.0071, 0.2786);
        assert!(fits && (2..=4).contains(&n) && u <= 1.0, "{n} {u}");
    }

    #[test]
    fn rigid_end_plate() {
        // An IPE 300 with 80 kN·m: 80 / 0,2893 = 277 kN over the bolts;
        // one row of two M20 (141 kN each) takes it.
        let p = end_plate(275.0, 80.0, 100.0, 0.300, 0.0107, Some((0.2, 0.015, 0.009)));
        assert!(!p.haunch && p.tension <= 1.0 && p.shear <= 1.0, "{p:?}");
        // A HEB 200 web: 0,9·275·(0,17·0,009)/√3 = 218 kN < 277.
        assert!(p.panel > 1.0, "{p:?}");
        // 400 kN·m need more than an extended plate (4 × M27 take 1056 kN).
        assert!(end_plate(275.0, 400.0, 100.0, 0.300, 0.0107, None).haunch);
    }

    #[test]
    fn timber_dowels() {
        // C24, Ø12 through 10 mm: about 2 × 3,5 kN · 0,8/1,3 per dowel,
        // across the grain (k90 1,53).
        let f = dowel_resistance(350.0, 0.012, 0.12, std::f64::consts::FRAC_PI_2);
        assert!((3.0..12.0).contains(&f), "{f}");
        let along = dowel_resistance(350.0, 0.012, 0.12, 0.0);
        assert!(along > f);
        // A 120 × 300 beam with 20 kN.
        let (n, _, _, fits, u, split) = timber_hanger(350.0, 20.0, 0.12, 0.30);
        assert!(
            fits && n >= 2 && u <= 1.0 && split <= 1.0,
            "{n} {u} {split}"
        );
        // A 140 × 140 column with 5 kN of uplift.
        let s = timber_shoe(350.0, 21.0, 0.14, 0.14, -5.0, 3.0);
        assert!(s.0 >= 2 && s.5 <= 1.0, "{s:?}");
    }
}
