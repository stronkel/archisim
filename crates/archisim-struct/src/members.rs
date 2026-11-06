//! The checks of the frame's members from its envelopes: reinforced
//! concrete beams and columns (Código Estructural), with the steel they
//! need set against the reinforcement drawn in the model, if any.

use archisim_core::{bar_area, Model, Profile, Reinforcement, Spaced};

use crate::analysis::{Analysis, Envelope};
use crate::analytical::Owner;
use crate::concrete::Rc;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

/// Distance from the face to the centre of the main bars, m (cover 3 cm,
/// stirrup 8, half a bar of 16).
const TO_BARS: f64 = 0.046;

fn cm2(a: f64) -> String {
    dec(a * 1e4, 2)
}

fn rect(model: &Model, owner: Owner) -> Option<(String, f64, f64)> {
    let t = match owner {
        Owner::Beam(id) => model.beam(id)?.member_type,
        Owner::Column(id) => model.column(id)?.member_type,
    };
    let mt = model.member_type(t)?;
    if Kind::of(model.material(mt.material)) != Kind::Concrete {
        return None;
    }
    match mt.profile {
        Profile::Rectangle { width, depth } => {
            // The name, without the size if it says it already.
            let size = format!("{}×{}", dec(width * 100.0, 0), dec(depth * 100.0, 0));
            let name = if mt.name.contains('×') {
                mt.name.clone()
            } else {
                format!("{} {size} cm", mt.name)
            };
            Some((name, width, depth))
        }
        _ => None,
    }
}

/// A round concrete column: its name and diameter.
fn round(model: &Model, owner: Owner) -> Option<(String, f64)> {
    let Owner::Column(id) = owner else {
        return None;
    };
    let mt = model.member_type(model.column(id)?.member_type)?;
    if Kind::of(model.material(mt.material)) != Kind::Concrete {
        return None;
    }
    match mt.profile {
        Profile::Circle { diameter } => {
            let name = if mt.name.contains('Ø') {
                mt.name.clone()
            } else {
                format!("{} Ø{} cm", mt.name, dec(diameter * 100.0, 0))
            };
            Some((name, diameter))
        }
        _ => None,
    }
}

/// The checks of a round concrete column: as the square ones, with the
/// moments added as a vector and the bars round it (six at least).
fn round_column_checks(
    model: &Model,
    e: &Envelope,
    id: archisim_core::ColumnId,
    out: &mut Vec<Check>,
) {
    let rc = Rc::of(model);
    let Some((name, dia)) = round(model, e.owner) else {
        return;
    };
    let n = (-e.min[0]).max(0.0);
    let my = e.max[4].abs().max(e.min[4].abs());
    let mz = e.max[5].abs().max(e.min[5].abs());
    let mut m = my.hypot(mz);
    let subject = Subject::Column(id);
    // Slenderness with i = D/4.
    let lo = model.column(id).map_or(3.0, |c| c.height);
    let area = std::f64::consts::PI * dia * dia / 4.0;
    let n_rel = n / (area * rc.fcd()).max(1e-9);
    let lim = 10.78 / n_rel.max(0.01).sqrt();
    let lambda = lo / (dia / 4.0);
    let mut slender = String::new();
    if lambda > lim {
        let e2 = rc.fyd() / 200e6 / (0.45 * (dia - 0.046)) * lo * lo / 10.0;
        m += n * e2;
        slender = format!(
            " (segundo orden: λ {} > {}: e2 {} cm)",
            dec(lambda, 0),
            dec(lim, 0),
            dec(e2 * 100.0, 1)
        );
    }
    if lambda > 100.0 {
        out.push(Check {
            subject,
            what: format!(
                "{name}: esbeltez {} con {} m de altura",
                dec(lambda, 0),
                dec(lo, 2)
            ),
            rule: "Esbeltez de pilares de hormigón (λ ≤ 100, EN 1992-1-1 5.8)",
            ratio: lambda / 100.0,
            status: Status::Fail,
            advice: Some("más diámetro".into()),
        });
    }
    let (bars, u_need) = rc.circle_bars(n, m, dia);
    let what = format!(
        "{name}: N {} kN, M {} kN·m (My {} y Mz {} juntos){slender}",
        dec(n, 1),
        dec(m, 1),
        dec(my, 1),
        dec(mz, 1)
    );
    let m_min = m.max(n * (dia / 20.0).max(0.02));
    match model.reinforcement(id.0) {
        Some(Reinforcement::Column {
            cover, bars: has, ..
        }) => {
            let c = cover + 0.008 + has.diameter / 2.0;
            let mut u = rc.circle_check(n, m_min, dia, has, c);
            let mut notes = String::new();
            let amin = 0.004 * area;
            if has.area() < amin {
                u = u.max(amin / has.area().max(1e-9));
                notes.push_str(&format!(", menos de la cuantía mínima ({} cm²)", cm2(amin)));
            }
            if has.count < 6 {
                u = u.max(1.01);
                notes.push_str(", menos de seis barras (art. 54)");
            }
            out.push(Check {
                subject,
                what: format!(
                    "{what}; tiene {} ({} cm²){notes}",
                    has.label(),
                    cm2(has.area())
                ),
                rule: "Compresión y flexión de pilar circular (Código Estructural, art. 42)",
                ratio: u,
                status: status(u),
                advice: (u > 1.0).then(|| format!("{} o más diámetro", bars.label())),
            });
        }
        _ => out.push(Check {
            subject,
            what: format!(
                "{what}: {} en corona ({} cm², aprovechamiento {})",
                bars.label(),
                cm2(bars.area()),
                dec(u_need, 2)
            ),
            rule: "Compresión y flexión de pilar circular (Código Estructural, art. 42)",
            ratio: if u_need > 1.0 { u_need } else { 0.0 },
            status: if u_need > 1.0 {
                Status::Fail
            } else {
                Status::Info
            },
            advice: Some(if u_need > 1.0 {
                "no llega ni con la cuantía máxima (4 %): más diámetro".into()
            } else {
                "el pilar no tiene armado dibujado: este es el que pide".into()
            }),
        }),
    }
}

/// The design moments of a rectangular column with its slenderness
/// (Código Estructural, art. 43; EN 1992-1-1, 5.8): braced, buckling
/// length its height; past λlim = 20·A·B·C/√n (A 0,7, B 1,1, C 0,7), the
/// second-order eccentricity of the nominal curvature. Gives My, Mz, what
/// was added and the slenderness about each axis.
pub fn column_moments(
    rc: &Rc,
    e: &Envelope,
    b: f64,
    h: f64,
    lo: f64,
) -> (f64, f64, Vec<String>, Vec<f64>) {
    let n = (-e.min[0]).max(0.0);
    let mut my = e.max[4].abs().max(e.min[4].abs());
    let mut mz = e.max[5].abs().max(e.min[5].abs());
    let n_rel = n / (b * h * rc.fcd()).max(1e-9);
    let lim = 10.78 / n_rel.max(0.01).sqrt();
    let eyd = rc.fyd() / 200e6;
    let mut slender = Vec::new();
    let mut lambdas = Vec::new();
    for (dim, m) in [(h, &mut my), (b, &mut mz)] {
        let lambda = lo / (dim / 12f64.sqrt());
        lambdas.push(lambda);
        if lambda > lim {
            let e2 = eyd / (0.45 * (dim - 0.045)) * lo * lo / 10.0;
            *m += n * e2;
            slender.push(format!(
                "λ {} > {}: e2 {} cm",
                dec(lambda, 0),
                dec(lim, 0),
                dec(e2 * 100.0, 1)
            ));
        }
    }
    (my, mz, slender, lambdas)
}

/// What a concrete beam or column asks for, as reinforcement to draw:
/// the same bars the checks propose.
pub fn proposal(model: &Model, e: &Envelope) -> Option<(u64, Reinforcement)> {
    let rc = Rc::of(model);
    match e.owner {
        Owner::Beam(id) => {
            let (_, b, h) = rect(model, e.owner)?;
            let d = h - TO_BARS;
            let sag = e.max[4].max(0.0);
            let hog = (-e.min[4]).max(0.0);
            let v = e.max[2].abs().max(e.min[2].abs());
            let amin = rc.min_beam_steel(b, h);
            let bot = sagging_steel(model, &rc, id, sag, b, d)?.max(amin);
            let top = rc.bending_steel(hog, b, d)?.max(0.3 * amin);
            let per_m = rc.stirrups(v, b, d, bot / (b * d));
            Some((
                id.0,
                Reinforcement::Beam {
                    cover: 0.03,
                    bottom: rc.bars_for(bot, 2),
                    top: rc.bars_for(top, 2),
                    stirrups: rc.stirrups_for(per_m, (0.75 * d).min(0.30)),
                },
            ))
        }
        Owner::Column(id) => {
            let n = (-e.min[0]).max(0.0);
            let bars = if let Some((_, dia)) = round(model, e.owner) {
                let m = e.max[4]
                    .abs()
                    .max(e.min[4].abs())
                    .hypot(e.max[5].abs().max(e.min[5].abs()));
                rc.circle_bars(n, m, dia).0
            } else {
                let (_, b, h) = rect(model, e.owner)?;
                let lo = model.column(id).map_or(3.0, |c| c.height);
                let (my, mz, _, _) = column_moments(&rc, e, b, h, lo);
                let a = rc.column_bars(n, my, mz, b, h);
                let c = rc.column_bars(n, my, mz, h, b);
                if c.0.area() > a.0.area() {
                    c.0
                } else {
                    a.0
                }
            };
            // Ties: a quarter of the bar, 6 at least (art. 54), at 15 bars
            // and 30 cm.
            let tie = if bars.diameter / 4.0 > 0.006 {
                0.008
            } else {
                0.006
            };
            let s = (15.0 * bars.diameter).min(0.30);
            let s = (s * 20.0).floor() / 20.0;
            Some((
                id.0,
                Reinforcement::Column {
                    cover: 0.03,
                    bars,
                    ties: Spaced::new(tie, s.max(0.10)),
                    dense: None,
                },
            ))
        }
    }
}

/// The cracked section of a rectangle `b` × `h` with `as_` of tension
/// steel at `d`: the neutral axis depth and its second moment, with the
/// steel as concrete by the modular ratio `ae`.
fn cracked(b: f64, d: f64, as_: f64, ae: f64) -> (f64, f64) {
    let r = ae * as_ / (b * d);
    let x = d * (-r + (r * r + 2.0 * r).sqrt());
    (x, b * x.powi(3) / 3.0 + ae * as_ * (d - x).powi(2))
}

/// Crack widths and deflections of the concrete beams under the
/// quasi-permanent loads (Código Estructural, art. 49 and 50; EN 1992-1-1,
/// 7.3.4 and 7.4.3): the moment the bars take at a stress σs over the
/// cracked section, wk = sr,max·(εsm − εcm) against 0,3 mm; the
/// deflection of the frame (with the whole section) by Branson's
/// effective inertia and the creep λ = 2/(1 + 50ρ′), the total against
/// L/250 and the active one against L/400.
fn serviceability(
    model: &Model,
    an: &Analysis,
    e: &Envelope,
    id: archisim_core::BeamId,
    out: &mut Vec<Check>,
) {
    let rc = Rc::of(model);
    let Some((name, b, h)) = rect(model, e.owner) else {
        return;
    };
    let Some((
        _,
        Reinforcement::Beam {
            cover,
            bottom,
            top,
            stirrups,
        },
    )) = model
        .reinforcement(id.0)
        .map(|r| (id.0, *r))
        .or_else(|| proposal(model, e))
    else {
        return;
    };
    let d = h - cover - stirrups.diameter - bottom.diameter / 2.0;
    // The quasi-permanent sagging moment.
    let mqp = an
        .combinations
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == crate::cases::Kind::QuasiPermanent)
        .filter_map(|(i, _)| e.by_combination.get(i))
        .flat_map(|f| f.iter().map(|x| x[4]))
        .fold(0.0f64, f64::max);
    let es = 200e6;
    let ecm = rc.ecm();
    let ae = es / ecm;
    let as_ = bottom.area();
    let (x, icr) = cracked(b, d, as_, ae);
    let ig = b * h.powi(3) / 12.0;
    // N/mm² to kN/m².
    let fct = rc.fctm() * 1000.0;
    let mcr = fct * b * h * h / 6.0;
    let subject = Subject::Beam(id);
    // Cracks.
    if mqp > mcr {
        let z = d - x / 3.0;
        let sigma = mqp / (as_ * z);
        let hc = (2.5 * (h - d)).min((h - x) / 3.0).min(h / 2.0);
        let rho = as_ / (b * hc);
        let eps = ((sigma - 0.4 * fct / rho * (1.0 + ae * rho)) / es).max(0.6 * sigma / es);
        let c = cover + stirrups.diameter;
        let sr = 3.4 * c + 0.425 * 0.8 * 0.5 * bottom.diameter / rho;
        let wk = sr * eps;
        let u = wk / 0.0003;
        out.push(Check {
            subject,
            what: format!(
                "{name}: Mqp {} kN·m, σs {} N/mm², separación de fisuras {} mm: wk {} mm",
                dec(mqp, 1),
                dec(sigma / 1000.0, 0),
                dec(sr * 1000.0, 0),
                dec(wk * 1000.0, 2)
            ),
            rule: "Fisuración (Código Estructural, art. 49; wmáx 0,3 mm)",
            ratio: u,
            status: status(u),
            advice: (u > 1.0).then(|| "más barras de menor diámetro, o más armadura".into()),
        });
    }
    // Deflection.
    let Some(def) = an.deflections.iter().find(|x| x.owner == e.owner) else {
        return;
    };
    let ma = mqp.max(1e-6);
    let ratio = (mcr / ma).min(1.0).powi(3);
    let ie = (ratio * ig + (1.0 - ratio) * icr).min(ig);
    let k = ig / ie.max(1e-12);
    let rho2 = top.area() / (b * d);
    let lambda = 2.0 / (1.0 + 50.0 * rho2);
    let l = if def.cantilever {
        2.0 * def.length
    } else {
        def.length
    };
    let total = def.appearance * k * (1.0 + lambda);
    // Active: what comes after the partitions: the variable load and the
    // creep of the permanent one.
    let active = (def.integrity - def.appearance).max(0.0) * k + lambda * def.appearance * k;
    let u = (total / (l / 250.0)).max(active / (l / 400.0));
    out.push(Check {
        subject,
        what: format!(
            "{name}: Ie {} de Ig (Mcr {} kN·m), λ {}: flecha total {} mm (L/250 = {}), activa {} mm (L/400 = {})",
            dec(ie / ig, 2),
            dec(mcr, 1),
            dec(lambda, 2),
            dec(total * 1000.0, 1),
            dec(l / 250.0 * 1000.0, 1),
            dec(active * 1000.0, 1),
            dec(l / 400.0 * 1000.0, 1)
        ),
        rule: "Flecha de vigas de hormigón con fisuración y fluencia (Código Estructural, art. 50)",
        ratio: u,
        status: status(u),
        advice: (u > 1.0).then(|| "más canto, o armadura de compresión que reduzca la fluencia".into()),
    });
}

/// The flange a concrete beam has in the slab on it (a T): its width
/// (EN 1992-1-1, 5.3.2.1: the web and 0,1·l0 each side, l0 = 0,85·L)
/// and the slab's depth; none if no solid or ribbed floor of concrete
/// rests on it.
pub fn flange(model: &Model, id: archisim_core::BeamId, b: f64) -> Option<(f64, f64)> {
    let beam = model.beam(id)?;
    let mid = beam.start + (beam.end - beam.start) * 0.5;
    let l0 = 0.85 * beam.start.distance(beam.end);
    let slab = model.slabs_on_level(beam.level).find(|s| {
        crate::geo::inside(mid, &s.outline) || crate::geo::to_path(mid, &s.outline).0 < b
    })?;
    let t = model.slab_type(slab.slab_type)?;
    let hf = match model.floor_system(slab.slab_type) {
        Some(f) => f.topping,
        None => t
            .layers
            .iter()
            .filter(|l| {
                l.function == archisim_core::LayerFunction::Structure
                    && Kind::of(model.material(l.material)) == Kind::Concrete
            })
            .map(|l| l.thickness)
            .sum(),
    };
    if hf < 0.03 {
        return None;
    }
    // Both sides inside the slab, or one at its edge.
    let inside =
        crate::geo::inside(mid, &slab.outline) && crate::geo::to_path(mid, &slab.outline).0 > b;
    let side = (0.1 * l0).min(0.2 * l0);
    Some((b + if inside { 2.0 } else { 1.0 } * side, hf))
}

/// The bottom steel for a sagging moment, with the slab's flange if the
/// compressed block stays in it.
fn sagging_steel(
    model: &Model,
    rc: &Rc,
    id: archisim_core::BeamId,
    m: f64,
    b: f64,
    d: f64,
) -> Option<f64> {
    if let Some((bf, hf)) = flange(model, id, b) {
        if let Some(a) = rc.bending_steel(m, bf, d) {
            let x = a * rc.fyd() / (0.8 * bf * rc.fcd());
            if 0.8 * x <= hf {
                return Some(a);
            }
        }
    }
    rc.bending_steel(m, b, d)
}

fn status(u: f64) -> Status {
    if u <= 1.0 {
        Status::Pass
    } else {
        Status::Fail
    }
}

fn beam_checks(model: &Model, e: &Envelope, id: archisim_core::BeamId, out: &mut Vec<Check>) {
    let rc = Rc::of(model);
    let Some((name, b, h)) = rect(model, e.owner) else {
        return;
    };
    let d = h - TO_BARS;
    // Sagging positive in the frame's local axes.
    let sag = e.max[4].max(0.0);
    let hog = (-e.min[4]).max(0.0);
    let v = e.max[2].abs().max(e.min[2].abs());
    let subject = Subject::Beam(id);
    let amin = rc.min_beam_steel(b, h);
    let (Some(bot), Some(top)) = (
        sagging_steel(model, &rc, id, sag, b, d),
        rc.bending_steel(hog, b, d),
    ) else {
        out.push(Check {
            subject,
            what: format!(
                "{name}: M {} / {} kN·m, más de lo que toma el hormigón sin armadura de compresión",
                dec(sag, 1),
                dec(-hog, 1)
            ),
            rule: "Flexión (Código Estructural, art. 42; x/d ≤ 0,45)",
            ratio: 2.0,
            status: Status::Fail,
            advice: Some("más canto o más ancho de viga".into()),
        });
        return;
    };
    let bot = bot.max(amin);
    let top = top.max(0.3 * amin);
    let rho = bot / (b * d);
    let per_m = rc.stirrups(v, b, d, rho);
    let crush = v / rc.vrd_max(b, d);
    let need = format!(
        "{name}: M+ {} kN·m pide {} cm² abajo, M− {} kN·m pide {} cm² arriba, V {} kN pide {} cm²/m de cercos",
        dec(sag, 1),
        cm2(bot),
        dec(hog, 1),
        cm2(top),
        dec(v, 1),
        cm2(per_m)
    );
    if crush > 1.0 {
        out.push(Check {
            subject,
            what: format!("{need}; V supera la compresión oblicua del alma"),
            rule: "Cortante, bielas (Código Estructural, art. 44)",
            ratio: crush,
            status: Status::Fail,
            advice: Some("más ancho o más canto".into()),
        });
    }
    match model.reinforcement(id.0) {
        Some(Reinforcement::Beam {
            bottom,
            top: t,
            stirrups,
            ..
        }) => {
            let has_s = 2.0 * bar_area(stirrups.diameter) / stirrups.spacing.max(0.01);
            let u = (bot / bottom.area().max(1e-9))
                .max(top / t.area().max(1e-9))
                .max(per_m / has_s);
            out.push(Check {
                subject,
                what: format!(
                    "{need}; tiene {} abajo ({} cm²), {} arriba ({} cm²) y cercos {} ({} cm²/m)",
                    bottom.label(),
                    cm2(bottom.area()),
                    t.label(),
                    cm2(t.area()),
                    stirrups.label(),
                    cm2(has_s)
                ),
                rule: "Armado de la viga (Código Estructural, art. 42 y 44)",
                ratio: u,
                status: status(u),
                advice: (u > 1.0).then(|| {
                    format!(
                        "abajo {}, arriba {}, cercos {}",
                        rc.bars_for(bot, 2).label(),
                        rc.bars_for(top, 2).label(),
                        rc.stirrups_for(per_m, (0.75 * d).min(0.30)).label()
                    )
                }),
            });
        }
        _ => out.push(Check {
            subject,
            what: format!(
                "{need}: abajo {}, arriba {}, cercos {}",
                rc.bars_for(bot, 2).label(),
                rc.bars_for(top, 2).label(),
                rc.stirrups_for(per_m, (0.75 * d).min(0.30)).label()
            ),
            rule: "Armado de la viga (Código Estructural, art. 42 y 44)",
            ratio: 0.0,
            status: Status::Info,
            advice: Some("la viga no tiene armado dibujado: este es el que pide".into()),
        }),
    }
}

fn column_checks(model: &Model, e: &Envelope, id: archisim_core::ColumnId, out: &mut Vec<Check>) {
    let rc = Rc::of(model);
    let Some((name, b, h)) = rect(model, e.owner) else {
        return;
    };
    let n = (-e.min[0]).max(0.0);
    let subject = Subject::Column(id);
    let lo = model.column(id).map_or(3.0, |c| c.height);
    let (my, mz, slender, lambdas) = column_moments(&rc, e, b, h, lo);
    for lambda in lambdas {
        if lambda > 100.0 {
            out.push(Check {
                subject,
                what: format!(
                    "{name}: esbeltez {} con {} m de altura",
                    dec(lambda, 0),
                    dec(lo, 2)
                ),
                rule: "Esbeltez de pilares de hormigón (λ ≤ 100, EN 1992-1-1 5.8)",
                ratio: lambda / 100.0,
                status: Status::Fail,
                advice: Some("más sección".into()),
            });
        }
    }
    // The section's sides go with its local axes either way: the worse.
    let (bars, u_need) = {
        let a = rc.column_bars(n, my, mz, b, h);
        let c = rc.column_bars(n, my, mz, h, b);
        if c.0.area() > a.0.area() {
            c
        } else {
            a
        }
    };
    let what = format!(
        "{name}: N {} kN, My {} kN·m, Mz {} kN·m{}",
        dec(n, 1),
        dec(my, 1),
        dec(mz, 1),
        if slender.is_empty() {
            String::new()
        } else {
            format!(" (segundo orden: {})", slender.join("; "))
        }
    );
    let check = |a: f64, cover: f64| -> f64 {
        let ey = (h / 20.0).max(0.02);
        let ez = (b / 20.0).max(0.02);
        let (my, mz) = (my.max(n * ey), mz.max(n * ez));
        rc.column_check(n, my, mz, b, h, a, cover)
            .max(rc.column_check(n, my, mz, h, b, a, cover))
    };
    match model.reinforcement(id.0) {
        Some(Reinforcement::Column {
            cover, bars: has, ..
        }) => {
            let c = cover + 0.008 + has.diameter / 2.0;
            let mut u = check(has.area(), c);
            let amin = 0.004 * b * h;
            let min_note = if has.area() < amin {
                u = u.max(amin / has.area().max(1e-9));
                format!(", menos de la cuantía mínima ({} cm²)", cm2(amin))
            } else {
                String::new()
            };
            out.push(Check {
                subject,
                what: format!(
                    "{what}; tiene {} ({} cm²){min_note}",
                    has.label(),
                    cm2(has.area())
                ),
                rule: "Compresión y flexión esviada (Código Estructural, art. 42)",
                ratio: u,
                status: status(u),
                advice: (u > 1.0).then(|| format!("{} o más sección", bars.label())),
            });
        }
        _ => out.push(Check {
            subject,
            what: format!(
                "{what}: {} ({} cm², aprovechamiento {})",
                bars.label(),
                cm2(bars.area()),
                dec(u_need, 2)
            ),
            rule: "Compresión y flexión esviada (Código Estructural, art. 42)",
            ratio: if u_need > 1.0 { u_need } else { 0.0 },
            status: if u_need > 1.0 {
                Status::Fail
            } else {
                Status::Info
            },
            advice: Some(if u_need > 1.0 {
                "no llega ni con la cuantía máxima (4 %): más sección".into()
            } else {
                "el pilar no tiene armado dibujado: este es el que pide".into()
            }),
        }),
    }
}

/// The checks of the concrete beams and columns of an analysis.
pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let mut out = Vec::new();
    for e in &an.envelopes {
        match e.owner {
            Owner::Beam(id) => {
                beam_checks(model, e, id, &mut out);
                serviceability(model, an, e, id, &mut out);
            }
            Owner::Column(id) => {
                column_checks(model, e, id, &mut out);
                round_column_checks(model, e, id, &mut out);
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use archisim_core::*;

    /// Two concrete columns 5 m apart with a beam and a floor over them.
    pub(crate) fn portal(m: &mut Model) -> (archisim_library::Library, BeamId) {
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(m).unwrap();
        for x in [0.0, 5.0] {
            m.add_column(NewColumn {
                member_type: lib.column,
                level: l0,
                position: Point2::new(x, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        }
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
                Point2::new(0.0, -2.0),
                Point2::new(5.0, -2.0),
                Point2::new(5.0, 2.0),
                Point2::new(0.0, 2.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        (lib, beam)
    }

    #[test]
    fn a_concrete_portal_gets_its_steel() {
        let mut m = Model::new("Pórtico");
        let (lib, beam) = portal(&mut m);
        let an = crate::analysis::analyse(&m);
        // Sagging is positive: the middle of the beam's governing diagram.
        let e = an
            .envelopes
            .iter()
            .find(|e| e.owner == crate::analytical::Owner::Beam(beam))
            .unwrap();
        let mid = e
            .diagram
            .iter()
            .min_by(|a, b| (a.0 - 2.5).abs().total_cmp(&(b.0 - 2.5).abs()))
            .unwrap();
        assert!(mid.1[4] > 0.0, "{:?}", e.diagram);
        let checks = check(&m, &an);
        assert!(
            rect(&m, e.owner).is_some(),
            "the template's beam is concrete"
        );
        assert!(
            checks.iter().any(|c| c.subject == Subject::Beam(beam)),
            "{checks:?}"
        );
        // With the reinforcement drawn, it is set against it.
        let r = Reinforcement::default_for(RebarKind::Beam);
        m.set_reinforcement(beam.0, Some(r)).unwrap();
        let an = crate::analysis::analyse(&m);
        let c = check(&m, &an);
        let c = c.iter().find(|c| c.subject == Subject::Beam(beam)).unwrap();
        assert!(c.what.contains("tiene"), "{}", c.what);
        assert!(c.ratio > 0.0);
        // Round columns get their own check, with bars round them.
        let t = m.member_type(lib.column).unwrap().clone();
        m.edit_member_type(
            lib.column,
            "Pilar circular",
            t.material,
            Profile::Circle { diameter: 0.35 },
        )
        .unwrap();
        let an = crate::analysis::analyse(&m);
        let c = check(&m, &an);
        let round = c
            .iter()
            .find(|c| c.rule.contains("circular"))
            .unwrap_or_else(|| panic!("{c:?}"));
        assert!(round.what.contains("Ø"), "{}", round.what);
        assert!(round.ratio <= 1.0, "{}", round.what);
    }
}
