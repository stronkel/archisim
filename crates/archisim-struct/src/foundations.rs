//! The foundations (DB-SE-C, 4): the footing each column and each
//! load-bearing wall needs for what it brings to the ground, against the
//! admissible pressure of the site, and the check of the footings drawn.
//! Centred loads (the moments at the column bases are not taken), the
//! footing and the earth over it as 10 % more; rigid footings. Their
//! bottom steel by bending at a section 0,15 of the column or wall
//! inside its face (Código Estructural, art. 58), with the ground's
//! ultimate reaction, against the mesh drawn or as a proposal.

use std::collections::BTreeMap;

use archisim_core::{FootingKind, Model, Point2, Vec2};

use crate::cases::Case;
use crate::concrete::Rc;
use crate::gravity::{Carrier, Takedown};
use crate::lightframe::{Check, Status, Subject};
use crate::text::dec;

/// The footing's own weight and the earth over it, as a share of the load.
const OWN: f64 = 0.10;

/// What each carrier brings to the ground in service (G + Q + S, all of
/// it: the characteristic combination with the live load leading and
/// the others whole is on the safe side), kN; with its length for walls.
pub fn service_loads(model: &Model, td: &Takedown) -> BTreeMap<Carrier, (f64, f64)> {
    let mut out: BTreeMap<Carrier, (f64, f64)> = BTreeMap::new();
    for g in &td.ground {
        let Some(c) = g.from else { continue };
        if matches!(g.case, Case::Wind(_) | Case::Quake(_)) {
            continue;
        }
        let len = match c {
            Carrier::Wall(id) => model.wall(id).map_or(1.0, |w| w.start.distance(w.end)),
            _ => 0.0,
        };
        let e = out.entry(c).or_insert((0.0, len));
        e.0 += g.kn;
    }
    out
}

/// The same in the ultimate combination (1,35 G + 1,5 the rest), kN.
pub fn ultimate_loads(td: &Takedown) -> BTreeMap<Carrier, f64> {
    let mut out: BTreeMap<Carrier, f64> = BTreeMap::new();
    for g in &td.ground {
        let Some(c) = g.from else { continue };
        let f = match g.case {
            Case::Dead => 1.35,
            Case::Wind(_) | Case::Quake(_) => 0.0,
            _ => 1.5,
        };
        *out.entry(c).or_default() += f * g.kn;
    }
    out
}

/// The bottom steel of a footing, m²/m: net ultimate pressure `q`
/// (kN/m²) over the overhang `v` from the face of what it carries, of
/// width `post`, and its depth `h`.
pub fn footing_steel(rc: &Rc, q: f64, v: f64, post: f64, h: f64) -> Option<f64> {
    // Rigid: the section 0,15 of the post inside its face; flexible (an
    // overhang of more than twice the depth): at the face (art. 58.4.2).
    let lever = if v > 2.0 * h {
        v.max(0.0)
    } else {
        v.max(0.0) + 0.15 * post
    };
    let m = q * lever * lever / 2.0;
    let d = (h - 0.06).max(0.1);
    rc.bending_steel(m, 1.0, d).map(|a| a.max(0.0009 * h))
}

fn per_m(sp: &archisim_core::Spaced) -> f64 {
    archisim_core::bar_area(sp.diameter) / sp.spacing.max(0.01)
}

/// The steel check of a drawn footing.
#[allow(clippy::too_many_arguments)]
fn steel_check(
    model: &Model,
    f: &archisim_core::Footing,
    name: &str,
    nu: f64,
    area: f64,
    v: f64,
    post: f64,
    out: &mut Vec<Check>,
) {
    let rc = Rc::of(model);
    let q = nu / area.max(1e-6);
    let need = footing_steel(&rc, q, v, post, f.depth);
    let has = match model.reinforcement(f.id.0) {
        Some(archisim_core::Reinforcement::Footing { bottom, .. }) => {
            Some((per_m(&bottom[0]).min(per_m(&bottom[1])), bottom))
        }
        _ => None,
    };
    let (what, ratio, status, advice) = match (need, has) {
        (None, _) => (
            format!("{name}: el canto no basta a flexión"),
            2.0,
            Status::Fail,
            Some("más canto".to_string()),
        ),
        (Some(a), Some((h, b))) => {
            let u = a / h.max(1e-9);
            (
                format!(
                    "{name}: pide {} cm²/m abajo; tiene {} y {} ({} cm²/m)",
                    dec(a * 1e4, 2),
                    b[0].label(),
                    b[1].label(),
                    dec(h * 1e4, 2)
                ),
                u,
                if u <= 1.0 { Status::Pass } else { Status::Fail },
                (u > 1.0).then(|| {
                    format!(
                        "{} en las dos direcciones",
                        crate::slab::mesh_for(a).label()
                    )
                }),
            )
        }
        (Some(a), None) => (
            format!(
                "{name}: pide {} cm²/m abajo: {} en las dos direcciones",
                dec(a * 1e4, 2),
                crate::slab::mesh_for(a).label()
            ),
            0.0,
            Status::Info,
            Some("la zapata no tiene armado dibujado: este es el que pide".to_string()),
        ),
    };
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what,
        rule: "Armado de la zapata (Código Estructural, art. 58)",
        ratio,
        status,
        advice,
    });
    // A flexible footing: shear at d from the face, as a slab (art. 58.4.2.1.2).
    if v > 2.0 * f.depth {
        let d = (f.depth - 0.06).max(0.1);
        let vd = q * (v - d).max(0.0);
        let rho = need.map_or(0.002, |a| (a / d).max(0.002));
        let vr = rc.vrd_c(1.0, d, rho);
        let u = vd / vr.max(1e-9);
        out.push(Check {
            subject: Subject::Footing(f.id.0),
            what: format!(
                "{name} (flexible, vuelo {} m): cortante a un canto útil de la cara {} kN/m, resiste {} kN/m",
                dec(v, 2),
                dec(vd, 1),
                dec(vr, 1)
            ),
            rule: "Cortante de la zapata flexible (Código Estructural, art. 58)",
            ratio: u,
            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
            advice: (u > 1.0).then(|| "más canto".into()),
        });
    }
}

pub fn check(model: &Model, td: &Takedown) -> Vec<Check> {
    let sigma = model.structural_site().bearing.max(1.0);
    let ultimate = ultimate_loads(td);
    let loads = service_loads(model, td);
    let mut out = Vec::new();
    let mut drawn: BTreeMap<Carrier, &archisim_core::Footing> = BTreeMap::new();
    for f in model.footings() {
        match f.kind {
            FootingKind::Pad {
                column: Some(c), ..
            } => {
                drawn.insert(Carrier::Column(c), f);
            }
            FootingKind::Strip { wall: Some(w), .. } => {
                drawn.insert(Carrier::Wall(w), f);
            }
            _ => {}
        }
    }
    // Pads under no column of their own that hold several: combined.
    let mut combined = std::collections::BTreeSet::new();
    for f in model.footings() {
        let FootingKind::Pad { column: None, .. } = f.kind else {
            continue;
        };
        let outline = f.outline();
        let under: Vec<_> = model
            .columns()
            .filter(|c| crate::geo::inside(c.position, &outline))
            .filter_map(|c| {
                let kn = loads.get(&Carrier::Column(c.id))?.0;
                let nu = ultimate
                    .get(&Carrier::Column(c.id))
                    .copied()
                    .unwrap_or(1.45 * kn);
                let post = model.member_type(c.member_type).map_or(0.3, |t| {
                    let (a, b) = t.profile.size();
                    a.max(b)
                });
                Some((c.id, c.position, kn, nu, post))
            })
            .collect();
        if under.len() >= 2 {
            combined.extend(under.iter().map(|x| x.0));
            combined_checks(model, f, &under, sigma, &mut out);
        }
    }
    let rule = "Presión sobre el terreno (DB-SE-C, 4.3)";
    for (c, (kn, len)) in &loads {
        if *kn <= 0.0 {
            continue;
        }
        let n = kn * (1.0 + OWN);
        match c {
            Carrier::Column(id) => {
                let need = (n / sigma).sqrt();
                let side = (need * 20.0).ceil() / 20.0;
                let name = format!("Pilar {}", id.0);
                if combined.contains(id) {
                    continue;
                }
                let col_pos = model.column(*id).map(|c| c.position);
                let post_of = || {
                    model
                        .column(*id)
                        .and_then(|c| model.member_type(c.member_type))
                        .map_or(0.3, |t| {
                            let (a, b) = t.profile.size();
                            a.max(b)
                        })
                };
                if let (Some(f), Some(cp)) = (drawn.get(c), col_pos) {
                    let nu = ultimate.get(c).copied().unwrap_or(1.45 * kn);
                    if let Some(p) = model.footing_piles(f.id) {
                        pile_checks(model, f, p, &name, post_of(), *kn, nu, &mut out);
                        continue;
                    }
                    let (ex, ey) = off_centre(model, f, cp);
                    if ex.hypot(ey) > 0.01 {
                        off_centre_checks(model, f, &name, cp, post_of(), *kn, nu, sigma, &mut out);
                        continue;
                    }
                }
                match drawn.get(c) {
                    Some(f) => {
                        let FootingKind::Pad { length, .. } = f.kind else {
                            continue;
                        };
                        let area = f.width * length;
                        let u = n / area.max(1e-6) / sigma;
                        out.push(Check {
                            subject: Subject::Footing(f.id.0),
                            what: format!(
                                "Zapata de {name}, {}×{}×{} m: {} kN, {} kN/m² sobre {} kN/m² admisibles",
                                dec(f.width, 2),
                                dec(length, 2),
                                dec(f.depth, 2),
                                dec(*kn, 1),
                                dec(n / area, 0),
                                dec(sigma, 0)
                            ),
                            rule,
                            ratio: u,
                            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                            advice: (u > 1.0).then(|| {
                                format!("zapata de {}×{} m", dec(side, 2), dec(side, 2))
                            }),
                        });
                        let post = model
                            .column(*id)
                            .and_then(|c| model.member_type(c.member_type))
                            .map_or(0.3, |t| {
                                let (a, b) = t.profile.size();
                                a.max(b)
                            });
                        rigid(f.width.max(length), f.depth, post, f.id.0, &name, &mut out);
                        let nu = ultimate.get(c).copied().unwrap_or(1.45 * kn);
                        let v = (f.width.max(length) - post) / 2.0;
                        steel_check(
                            model,
                            f,
                            &format!("Zapata de {name}"),
                            nu,
                            area,
                            v,
                            post,
                            &mut out,
                        );
                    }
                    None => out.push(Check {
                        subject: Subject::Footing(id.0),
                        what: format!(
                            "{name}: {} kN, pide una zapata de {}×{} m y {} m de canto",
                            dec(*kn, 1),
                            dec(side, 2),
                            dec(side, 2),
                            dec(((side - 0.3) / 4.0).max(0.4).ceil_to(0.05), 2)
                        ),
                        rule,
                        ratio: 0.0,
                        status: Status::Info,
                        advice: Some("el pilar no tiene zapata dibujada".into()),
                    }),
                }
            }
            Carrier::Wall(id) => {
                let q = n / len.max(0.1);
                let need = (q / sigma).max(0.4);
                let width = (need * 20.0).ceil() / 20.0;
                let name = format!("Muro {}", id.0);
                let thick = model.wall(*id).map_or(0.25, |w| {
                    model
                        .wall_type(w.wall_type)
                        .map_or(0.25, |t| t.layers.iter().map(|l| l.thickness).sum())
                });
                match drawn.get(c) {
                    Some(f) => {
                        let u = q / f.width.max(1e-6) / sigma;
                        out.push(Check {
                            subject: Subject::Footing(f.id.0),
                            what: format!(
                                "Zapata corrida de {name}, {} m de ancho y {} de canto: {} kN/m, {} kN/m² sobre {} admisibles",
                                dec(f.width, 2),
                                dec(f.depth, 2),
                                dec(kn / len.max(0.1), 1),
                                dec(q / f.width, 0),
                                dec(sigma, 0)
                            ),
                            rule,
                            ratio: u,
                            status: if u <= 1.0 { Status::Pass } else { Status::Fail },
                            advice: (u > 1.0).then(|| format!("{} m de ancho", dec(width, 2))),
                        });
                        rigid(f.width, f.depth, thick, f.id.0, &name, &mut out);
                        let nu = ultimate.get(c).copied().unwrap_or(1.45 * kn) / len.max(0.1);
                        let v = (f.width - thick) / 2.0;
                        steel_check(
                            model,
                            f,
                            &format!("Zapata corrida de {name}"),
                            nu,
                            f.width,
                            v,
                            thick,
                            &mut out,
                        );
                    }
                    None => {
                        // Only the walls that bear something on the ground:
                        // partitions on a slab on the ground have no footing.
                        if model.wall(*id).is_some_and(|w| {
                            model.wall_type(w.wall_type).is_some_and(|t| {
                                t.layers
                                    .iter()
                                    .any(|l| l.function == archisim_core::LayerFunction::Structure)
                            })
                        }) {
                            out.push(Check {
                                subject: Subject::Footing(id.0),
                                what: format!(
                                    "{name}: {} kN/m, pide una zapata corrida de {} m de ancho",
                                    dec(kn / len.max(0.1), 1),
                                    dec(width, 2)
                                ),
                                rule,
                                ratio: 0.0,
                                status: Status::Info,
                                advice: Some("el muro no tiene zapata dibujada".into()),
                            });
                        }
                    }
                }
            }
            Carrier::Beam(_) => {}
        }
    }
    out
}

/// The column of a pad footing and how far the footing's centre is from
/// it, in the footing's axes (x across its width, y along its length).
fn off_centre(model: &Model, f: &archisim_core::Footing, col: Point2) -> (f64, f64) {
    let FootingKind::Pad {
        position, rotation, ..
    } = f.kind
    else {
        return (0.0, 0.0);
    };
    let d = position - col;
    let (c, s) = (rotation.cos(), rotation.sin());
    let _ = model;
    (d.x * c + d.y * s, -d.x * s + d.y * c)
}

/// The tie beams that centre a footing: one end on the footing (or its
/// column) and the other on another column, which it reaches; each with
/// that column's position (two at a corner).
fn centring_beams<'a>(
    model: &'a Model,
    f: &archisim_core::Footing,
    col: Point2,
) -> Vec<(&'a archisim_core::Footing, Point2)> {
    let reach = f.width.max(match f.kind {
        FootingKind::Pad { length, .. } => length,
        _ => 0.0,
    }) / 2.0
        + 0.3;
    let columns: Vec<Point2> = model.columns().map(|c| c.position).collect();
    model
        .footings()
        .filter_map(|t| {
            let FootingKind::TieBeam { start, end } = t.kind else {
                return None;
            };
            for (near, far) in [(start, end), (end, start)] {
                if near.distance(col) <= reach {
                    if let Some(o) = columns
                        .iter()
                        .copied()
                        .filter(|p| p.distance(col) > 0.5)
                        .find(|p| p.distance(far) < 0.75)
                    {
                        return Some((t, o));
                    }
                }
            }
            None
        })
        .collect()
}

/// The checks of a pad footing off its column (against a party wall or
/// at a corner): with a centring beam, the beam takes the moment and the
/// footing a little more than its column (R = N·L/(L − e)); without, the
/// pressure over the effective area (Meyerhof, B − 2e).
#[allow(clippy::too_many_arguments)]
fn off_centre_checks(
    model: &Model,
    f: &archisim_core::Footing,
    name: &str,
    col: Point2,
    post: f64,
    kn: f64,
    nu: f64,
    sigma: f64,
    out: &mut Vec<Check>,
) {
    let FootingKind::Pad {
        length, rotation, ..
    } = f.kind
    else {
        return;
    };
    let (ex, ey) = off_centre(model, f, col);
    let e_all = ex.hypot(ey);
    let rule = "Presión sobre el terreno (DB-SE-C, 4.3)";
    let rc = Rc::of(model);
    // Each centring beam takes the eccentricity along it; what is left
    // stays on the footing.
    let (c, s) = (rotation.cos(), rotation.sin());
    let mut rest = Vec2::new(ex * c - ey * s, ex * s + ey * c);
    let mut r = 1.0;
    let mut beams = Vec::new();
    for (t, other) in centring_beams(model, f, col) {
        let l = col.distance(other);
        let u = (other - col).normalized().unwrap_or(Vec2::new(1.0, 0.0));
        let e = (rest.x * u.x + rest.y * u.y).max(0.0);
        if e < 0.01 {
            continue;
        }
        rest = rest - u * e;
        r *= l / (l - e).max(0.1);
        beams.push((t, e, l));
    }
    let eb = (rest.x * c + rest.y * s).abs();
    let el = (-rest.x * s + rest.y * c).abs();
    let beam = beams.first().copied();
    let n = kn * r * (1.0 + OWN);
    let b_eff = f.width - 2.0 * eb;
    let l_eff = length - 2.0 * el;
    if b_eff <= 0.05 || l_eff <= 0.05 {
        out.push(Check {
            subject: Subject::Footing(f.id.0),
            what: format!(
                "Zapata de {name}, {}×{} m, a {} m de su pilar: la carga cae fuera de la zapata",
                dec(f.width, 2),
                dec(length, 2),
                dec(e_all, 2)
            ),
            rule,
            ratio: 3.0,
            status: Status::Fail,
            advice: Some("una viga centradora hasta un pilar interior".into()),
        });
        return;
    }
    let p = n / (b_eff * l_eff);
    let u = p / sigma;
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "Zapata de {name} {}, {}×{}×{} m: {} kN{}; área eficaz {}×{} m, {} kN/m² sobre {} admisibles",
            if beam.is_some() { "de medianera con viga centradora" } else { "excéntrica" },
            dec(f.width, 2),
            dec(length, 2),
            dec(f.depth, 2),
            dec(kn, 1),
            if r > 1.0 { format!(" (× {} por la viga centradora)", dec(r, 2)) } else { String::new() },
            dec(b_eff, 2),
            dec(l_eff, 2),
            dec(p, 0),
            dec(sigma, 0)
        ),
        rule,
        ratio: u,
        status: if u <= 1.0 { Status::Pass } else { Status::Fail },
        advice: (u > 1.0).then(|| {
            if beam.is_some() {
                "zapata mayor".into()
            } else {
                "una viga centradora hasta el pilar interior más cercano (o zapata mayor)".into()
            }
        }),
    });
    if beam.is_none() && e_all > f.width.min(length) / 6.0 {
        out.push(Check {
            subject: Subject::Footing(f.id.0),
            what: format!(
                "Zapata de {name}: la carga está a {} m del centro, fuera del núcleo central ({} m): parte de la zapata se levanta",
                dec(e_all, 2),
                dec(f.width.min(length) / 6.0, 2)
            ),
            rule: "Zapata excéntrica sin viga centradora (DB-SE-C, 4.3)",
            ratio: 1.0,
            status: Status::Warn,
            advice: Some("una viga centradora hasta el pilar interior más cercano".into()),
        });
    }
    // The beams: the moment N·e at the column, its shear N·e/(L − e).
    for (t, e, l) in beams {
        {
            let mu = nu * e;
            let vu = nu * e / (l - e).max(0.1);
            let d = (t.depth - 0.05).max(0.15);
            let need = rc.bending_steel(mu, t.width, d);
            let (what, status, ratio) = match need {
                Some(a) => {
                    let a = a.max(rc.min_beam_steel(t.width, t.depth));
                    let stir = rc.stirrups_for(
                        rc.stirrups(vu, t.width, d, (a / (t.width * d)).min(0.02)),
                        0.3,
                    );
                    (
                        format!(
                            "arriba {}, abajo {}, cercos {}",
                            rc.bars_for(a, 2).label(),
                            rc.bars_for(rc.min_beam_steel(t.width, t.depth), 2).label(),
                            stir.label()
                        ),
                        Status::Info,
                        0.0,
                    )
                }
                None => ("el canto no basta".to_string(), Status::Fail, 1.5),
            };
            out.push(Check {
                subject: Subject::Footing(t.id.0),
                what: format!(
                    "Viga centradora de {name}, {}×{} m, {} m entre pilares, excentricidad {} m: Md {} kN·m, Vd {} kN: {what}",
                    dec(t.width, 2),
                    dec(t.depth, 2),
                    dec(l, 2),
                    dec(e, 2),
                    dec(mu, 1),
                    dec(vu, 1)
                ),
                rule: "Viga centradora (Código Estructural, art. 42 y 44)",
                ratio,
                status,
                advice: (status == Status::Fail).then(|| "más canto en la viga centradora".into()),
            });
        }
    }
    let area = b_eff * l_eff;
    let v = (f.width / 2.0 + ex.abs()).max(length / 2.0 + ey.abs()) - post / 2.0;
    steel_check(
        model,
        f,
        &format!("Zapata de {name}"),
        nu * r,
        area,
        v,
        post,
        out,
    );
}

/// A pad footing under several columns (combined): the pressure with the
/// resultant's eccentricity, and as a beam along its length on the
/// ground's ultimate reaction, the top steel between the columns and the
/// bottom under them, and its shear.
fn combined_checks(
    model: &Model,
    f: &archisim_core::Footing,
    columns: &[(archisim_core::ColumnId, Point2, f64, f64, f64)],
    sigma: f64,
    out: &mut Vec<Check>,
) {
    let FootingKind::Pad {
        position,
        length,
        rotation,
        ..
    } = f.kind
    else {
        return;
    };
    let (c, s) = (rotation.cos(), rotation.sin());
    let local = |p: Point2| {
        let d = p - position;
        (d.x * c + d.y * s, -d.x * s + d.y * c)
    };
    let n: f64 = columns.iter().map(|x| x.2).sum();
    let nu: f64 = columns.iter().map(|x| x.3).sum();
    if n <= 0.0 {
        return;
    }
    let (mut rx, mut ry) = (0.0, 0.0);
    for (_, p, kn, _, _) in columns {
        let (a, b) = local(*p);
        rx += a * kn / n;
        ry += b * kn / n;
    }
    let b_eff = f.width - 2.0 * rx.abs();
    let l_eff = length - 2.0 * ry.abs();
    let names: Vec<String> = columns.iter().map(|x| x.0 .0.to_string()).collect();
    let name = format!("Zapata combinada de los pilares {}", names.join(", "));
    let p = n * (1.0 + OWN) / (b_eff * l_eff).max(1e-6);
    let u = if b_eff > 0.05 && l_eff > 0.05 {
        p / sigma
    } else {
        3.0
    };
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "{name}, {}×{}×{} m: {} kN con la resultante a {} m del centro; {} kN/m² sobre {} admisibles",
            dec(f.width, 2),
            dec(length, 2),
            dec(f.depth, 2),
            dec(n, 1),
            dec(rx.hypot(ry), 2),
            dec(p, 0),
            dec(sigma, 0)
        ),
        rule: "Presión sobre el terreno (DB-SE-C, 4.3)",
        ratio: u,
        status: if u <= 1.0 { Status::Pass } else { Status::Fail },
        advice: (u > 1.0).then(|| "zapata mayor, o con su centro sobre la resultante de los pilares".into()),
    });
    // As a beam along its longer side.
    let along_y = length >= f.width;
    let (span, across) = if along_y {
        (length, f.width)
    } else {
        (f.width, length)
    };
    let w = nu / span;
    let mut pts: Vec<(f64, f64, f64)> = columns
        .iter()
        .map(|(_, p, _, u, post)| {
            let (a, b) = local(*p);
            (if along_y { b } else { a }, *u, *post)
        })
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let x0 = -span / 2.0;
    let d = (f.depth - 0.06).max(0.15);
    let steps = 200;
    let (mut m_bot, mut m_top, mut v_max) = (0.0f64, 0.0f64, 0.0f64);
    for k in 0..=steps {
        let x = x0 + span * k as f64 / steps as f64;
        let mut m = w * (x - x0).powi(2) / 2.0;
        let mut v = w * (x - x0);
        for (s, nu_i, _) in &pts {
            if *s < x {
                m -= nu_i * (x - s);
                v -= nu_i;
            }
        }
        m_bot = m_bot.max(m);
        m_top = m_top.max(-m);
        // The shear at d from a column's face.
        if pts
            .iter()
            .all(|(s, _, post)| (x - s).abs() >= post / 2.0 + d)
        {
            v_max = v_max.max(v.abs());
        }
    }
    let rc = Rc::of(model);
    let bottom = rc.bending_steel(m_bot, across, d);
    let top = rc.bending_steel(m_top, across, d);
    let vr = rc.vrd_c(across, d, 0.003);
    let (status, ratio, what) = match (bottom, top) {
        (Some(b), Some(t)) => {
            let amin = 0.0009 * across * f.depth;
            (
                if v_max <= vr { Status::Info } else { Status::Fail },
                if v_max <= vr { 0.0 } else { v_max / vr },
                format!(
                    "abajo {} cm² ({} en {} m), arriba {} cm² ({}) a lo largo; cortante {} kN sobre {} kN",
                    dec(b.max(amin) * 1e4, 1),
                    crate::slab::mesh_for(b.max(amin) / across).label(),
                    dec(across, 2),
                    dec(t.max(amin) * 1e4, 1),
                    crate::slab::mesh_for(t.max(amin) / across).label(),
                    dec(v_max, 1),
                    dec(vr, 1)
                ),
            )
        }
        _ => (Status::Fail, 1.5, "el canto no basta a flexión".to_string()),
    };
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "{name} como viga sobre el terreno: M {} kN·m abajo y {} kN·m arriba: {what}",
            dec(m_bot, 1),
            dec(m_top, 1)
        ),
        rule: "Zapata combinada (Código Estructural, art. 58)",
        ratio,
        status,
        advice: (status == Status::Fail).then(|| "más canto".into()),
    });
}

/// A pile cap: what each pile takes in service against what it may
/// take, whether they fit, and the cap's ties by struts and ties (art.
/// 48 and 58.4.1.2.2).
#[allow(clippy::too_many_arguments)]
fn pile_checks(
    model: &Model,
    f: &archisim_core::Footing,
    piles: &archisim_core::Piles,
    name: &str,
    post: f64,
    kn: f64,
    nu: f64,
    out: &mut Vec<Check>,
) {
    let FootingKind::Pad { length, .. } = f.kind else {
        return;
    };
    let cap_weight = 25.0 * f.width * length * f.depth;
    let each = (kn + cap_weight) / f64::from(piles.count);
    let allowed = piles.allowed();
    let u = each / allowed.max(1e-6);
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "Encepado de {name} con {} pilote{} Ø{} de {} m: {} kN por pilote en servicio, admite {} kN{}",
            piles.count,
            if piles.count == 1 { "" } else { "s" },
            dec(piles.diameter * 100.0, 0),
            dec(piles.length, 1),
            dec(each, 0),
            dec(allowed, 0),
            if piles.capacity > 0.0 { " (del estudio geotécnico)" } else { " (tope estructural, 4 N/mm²)" }
        ),
        rule: "Carga de los pilotes (DB-SE-C, 5.3)",
        ratio: u,
        status: if u <= 1.0 { Status::Pass } else { Status::Fail },
        advice: (u > 1.0).then(|| {
            let need = ((kn + cap_weight) / allowed).ceil() as u32;
            format!("{need} pilotes, o de más diámetro")
        }),
    });
    if piles.capacity <= 0.0 {
        out.push(Check {
            subject: Subject::Footing(f.id.0),
            what: format!("Encepado de {name}: sin carga admisible del estudio geotécnico"),
            rule: "Carga de los pilotes (DB-SE-C, 5.3)",
            ratio: 0.0,
            status: Status::Warn,
            advice: Some("pon la carga admisible de cada pilote que da el estudio geotécnico (por fuste y punta)".into()),
        });
    }
    let (w, l) = piles.cap_size();
    let (w, l) = if length >= f.width { (w, l) } else { (l, w) };
    let fits = f.width + 1e-6 >= w && length + 1e-6 >= l;
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "Encepado de {name}, {}×{} m: los pilotes a {} m entre ejes piden {}×{} m",
            dec(f.width, 2),
            dec(length, 2),
            dec(piles.spacing(), 2),
            dec(w, 2),
            dec(l, 2)
        ),
        rule: "Tamaño del encepado (3 diámetros entre pilotes)",
        ratio: if fits { 0.0 } else { 1.2 },
        status: if fits { Status::Pass } else { Status::Fail },
        advice: (!fits).then(|| format!("encepado de {}×{} m", dec(w, 2), dec(l, 2))),
    });
    if piles.count < 2 {
        return;
    }
    let rc = Rc::of(model);
    let d = (f.depth - 0.10).max(0.3);
    let s = piles.spacing();
    // The tie between two piles: half the load over the strut's lever.
    let lever = (s / 2.0 - post / 4.0).max(0.0);
    let ties = if piles.count == 2 { 1.0 } else { 2.0 };
    let t = nu / 2.0 * lever / (0.85 * d) / ties;
    let fyd = rc.fyd().min(400e3);
    let a = t / fyd;
    // A rigid cap: the farthest pile no more than twice the depth from the face.
    let v = s / 2.0 - post / 2.0;
    let rigid = v <= 2.0 * f.depth;
    out.push(Check {
        subject: Subject::Footing(f.id.0),
        what: format!(
            "Encepado de {name}: tirante entre pilotes {} kN, {} sobre las cabezas de los pilotes{}",
            dec(t, 0),
            rc.bars_for(a, 3).label(),
            if piles.count > 2 { " en cada dirección" } else { "" }
        ),
        rule: "Armado del encepado (bielas y tirantes, Código Estructural, art. 48 y 58)",
        ratio: if rigid { 0.0 } else { 1.0 },
        status: if rigid { Status::Info } else { Status::Warn },
        advice: (!rigid).then(|| format!("encepado flexible: canto de {} m al menos para que sea rígido", dec(v / 2.0, 2))),
    });
}

/// A footing is rigid if its overhang is no more than twice its depth
/// (Código Estructural, art. 55); a flexible one is designed as such (its
/// bending at the face and its shear, in `steel_check`).
fn rigid(width: f64, depth: f64, post: f64, id: u64, name: &str, out: &mut Vec<Check>) {
    let v = ((width - post) / 2.0).max(0.0);
    let u = v / (2.0 * depth).max(1e-6);
    out.push(Check {
        subject: Subject::Footing(id),
        what: format!(
            "Zapata de {name}: vuelo {} m con {} m de canto",
            dec(v, 2),
            dec(depth, 2)
        ),
        rule: "Zapata rígida o flexible (Código Estructural, art. 55; vuelo ≤ 2 cantos)",
        ratio: if u <= 1.0 { u } else { 0.0 },
        status: if u <= 1.0 { Status::Pass } else { Status::Info },
        advice: (u > 1.0).then(|| {
            format!(
                "flexible: se calcula a flexión en la cara y a cortante; con {} m de canto sería rígida",
                dec(v / 2.0, 2)
            )
        }),
    });
}

trait CeilTo {
    fn ceil_to(self, step: f64) -> f64;
}

impl CeilTo for f64 {
    fn ceil_to(self, step: f64) -> f64 {
        (self / step).ceil() * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    /// Two columns 5 m apart under a floor, the first against a party
    /// wall (x = 0).
    fn party_wall(m: &mut Model) -> (Vec<ColumnId>, LevelId, MaterialId) {
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(m).unwrap();
        let mut cols = Vec::new();
        for (x, y) in [(0.15, 0.0), (5.0, 0.0), (0.15, 4.0), (5.0, 4.0)] {
            cols.push(
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(x, y),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap(),
            );
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                Point2::new(0.0, -0.5),
                Point2::new(5.5, -0.5),
                Point2::new(5.5, 4.5),
                Point2::new(0.0, 4.5),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let concrete = m
            .materials()
            .find(|x| x.name.to_lowercase().contains("hormigón"))
            .unwrap()
            .id;
        (cols, l0, concrete)
    }

    fn pad(
        m: &mut Model,
        l: LevelId,
        mat: MaterialId,
        c: ColumnId,
        at: Point2,
        side: f64,
    ) -> FootingId {
        m.add_footing(NewFooting {
            level: l,
            kind: FootingKind::Pad {
                position: at,
                length: side,
                rotation: 0.0,
                column: Some(c),
            },
            material: mat,
            width: side,
            depth: 0.6,
            top_offset: 0.0,
        })
        .unwrap()
    }

    #[test]
    fn a_party_wall_footing_and_its_centring_beam() {
        let mut m = Model::new("Medianera");
        let (cols, l0, mat) = party_wall(&mut m);
        let f = pad(&mut m, l0, mat, cols[0], Point2::new(0.15, 0.0), 1.4);
        // Against the party wall: its centre 0,55 m inside.
        m.set_footing_offset(f, Some(Vec2::new(0.55, 0.0))).unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        let p = c
            .iter()
            .find(|c| c.subject == Subject::Footing(f.0) && c.rule.starts_with("Presión"))
            .unwrap();
        assert!(p.what.contains("excéntrica"), "{}", p.what);
        // Without a beam the effective width is 1,4 − 1,1 = 0,3 m.
        assert!(p.what.contains("0,30×1,40"), "{}", p.what);
        // With a centring beam to the inner column.
        m.add_footing(NewFooting {
            level: l0,
            kind: FootingKind::TieBeam {
                start: Point2::new(0.15, 0.0),
                end: Point2::new(5.0, 0.0),
            },
            material: mat,
            width: 0.4,
            depth: 0.6,
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        let p = c
            .iter()
            .find(|c| c.subject == Subject::Footing(f.0) && c.rule.starts_with("Presión"))
            .unwrap();
        assert!(p.what.contains("viga centradora"), "{}", p.what);
        // R = N·4,85/4,30 = 1,13·N over the whole 1,4 × 1,4.
        assert!(p.what.contains("1,13"), "{}", p.what);
        assert!(p.what.contains("1,40×1,40"), "{}", p.what);
        let beam = c
            .iter()
            .find(|c| c.rule.starts_with("Viga centradora"))
            .unwrap();
        assert!(beam.what.contains("Md"), "{}", beam.what);
    }

    #[test]
    fn a_combined_footing_and_a_pile_cap() {
        let mut m = Model::new("Combinada");
        let (cols, l0, mat) = party_wall(&mut m);
        // One footing under the two columns on x = 0,15.
        let f = m
            .add_footing(NewFooting {
                level: l0,
                kind: FootingKind::Pad {
                    position: Point2::new(0.6, 2.0),
                    length: 5.6,
                    rotation: 0.0,
                    column: None,
                },
                material: mat,
                width: 1.2,
                depth: 0.6,
                top_offset: 0.0,
            })
            .unwrap();
        let p = pad(&mut m, l0, mat, cols[1], Point2::new(5.0, 0.0), 1.5);
        m.set_footing_piles(p, Some(Piles::new(2, 0.45))).unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        assert!(
            c.iter()
                .any(|c| c.subject == Subject::Footing(f.0)
                    && c.rule.starts_with("Zapata combinada")),
            "{c:?}"
        );
        // The columns on it ask for no footings of their own.
        assert!(!c.iter().any(|c| c.subject == Subject::Footing(cols[0].0)));
        let piles = c
            .iter()
            .find(|c| c.rule.starts_with("Carga de los pilotes") && c.status != Status::Warn)
            .unwrap();
        assert!(piles.what.contains("2 pilotes Ø45"), "{}", piles.what);
        // Two piles of 45 at 1,35 m: a cap of 0,75 × 2,10.
        let size = c
            .iter()
            .find(|c| c.rule.starts_with("Tamaño del encepado"))
            .unwrap();
        assert_eq!(size.status, Status::Fail, "{}", size.what);
        assert!(c.iter().any(|c| c.rule.starts_with("Armado del encepado")));
    }

    #[test]
    fn a_column_asks_for_its_footing() {
        let mut m = Model::new("Zapata");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut cols = Vec::new();
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (0.0, 5.0), (5.0, 5.0)] {
            cols.push(
                m.add_column(NewColumn {
                    member_type: lib.column,
                    level: l0,
                    position: Point2::new(x, y),
                    rotation: 0.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                    height: 3.0,
                })
                .unwrap(),
            );
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(5.0, 0.0),
                Point2::new(5.0, 5.0),
                Point2::new(0.0, 5.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        assert_eq!(c.len(), 4, "{c:?}");
        assert!(c.iter().all(|c| c.status == Status::Info));
        // A small footing under one: too much pressure.
        let concrete = m.materials().next().unwrap().id;
        m.add_footing(NewFooting {
            level: l0,
            kind: FootingKind::Pad {
                position: Point2::new(0.0, 0.0),
                length: 0.3,
                rotation: 0.0,
                column: Some(cols[0]),
            },
            material: concrete,
            width: 0.3,
            depth: 0.5,
            top_offset: 0.0,
        })
        .unwrap();
        let td = crate::takedown(&m);
        let c = check(&m, &td);
        let f = c
            .iter()
            .find(|c| c.rule.starts_with("Presión") && c.status != Status::Info)
            .unwrap();
        assert_eq!(f.status, Status::Fail, "{}", f.what);
    }
}
