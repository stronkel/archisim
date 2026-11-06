//! Water supply and drainage (DB-HS 4 and DB-HS 5), on the pipes drawn:
//!
//! - every sanitary appliance (by the model ArchiSim makes for it, or by
//!   its type's name) is fed by the nearest cold and hot water pipes of
//!   its level and drains into the nearest drain; their diameters are
//!   checked against the minimums of HS 4 (tabla 4.2, derivaciones a los
//!   aparatos) and HS 5 (tabla 4.1, uso privado);
//! - the slope of every drain: a branch of a small diameter between 2 and
//!   5 %, a collector at least 1 % hung and 2 % buried (below its level).
//!
//! The diameter of a pipe is what is drawn (the tool asks for the outer
//! one); the code's minimums are nominal ones, so a pipe a millimetre
//! short passes. What is not checked yet: the diameters of collectors and
//! downpipes by the units of discharge they gather (HS 5, tablas 4.3 to
//! 4.5) and the flows of the supply (HS 4, tabla 2.1).

use archisim_core::{Model, ObjectId, Point2, ServiceRun, ServiceSystem};

use crate::{Item, Report, Section, Status, Subject};

pub use archisim_core::Appliance;

/// How far from an appliance its pipes are looked for, metres.
const REACH: f64 = 1.5;
/// A pipe this much under the minimum still passes (rounding of the
/// outer and nominal diameters), metres.
const TOLERANCE: f64 = 0.0015;
/// Drains of less than this are branches to the appliances, metres.
const BRANCH: f64 = 0.075;

fn mm(v: f64) -> String {
    format!("Ø {:.0} mm", v * 1000.0)
}

/// The nearest run of a system to an appliance's outline, and how far.
pub(crate) fn nearest<'a>(
    model: &'a Model,
    level: archisim_core::LevelId,
    systems: &[ServiceSystem],
    outline: &[Point2],
) -> Option<(&'a ServiceRun, f64)> {
    let centre = archisim_core::rooms::centroid(outline);
    model
        .runs_on_level(level)
        .filter(|r| systems.contains(&r.system))
        .map(|r| {
            let d = std::iter::once(centre)
                .chain(outline.iter().copied())
                .map(|p| r.locate(p).1)
                .fold(f64::INFINITY, f64::min);
            (r, d)
        })
        .filter(|(_, d)| *d <= REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub fn check(model: &Model, report: &mut Report) {
    let any = |s: ServiceSystem| model.runs().any(|r| r.system == s);
    let has_drains = any(ServiceSystem::Drainage);
    let has_water = any(ServiceSystem::ColdWater) || any(ServiceSystem::HotWater);
    let push = |report: &mut Report,
                status: Status,
                subject: Subject,
                level,
                rule: &'static str,
                text: String,
                value: String,
                required: String| {
        report.items.push(Item {
            section: Section::Services,
            status,
            subject,
            level: Some(level),
            rule,
            text,
            value,
            required,
            route: Vec::new(),
        });
    };
    let mut appliances: Vec<(ObjectId, Appliance)> = model
        .objects()
        .filter_map(|o| Some((o.id, Appliance::of(model, o.id)?)))
        .collect();
    appliances.sort();
    for (id, a) in appliances {
        let Some(o) = model.objects().find(|o| o.id == id) else {
            continue;
        };
        let outline = model.object_outline(id);
        if outline.is_empty() {
            continue;
        }
        let subject = Subject::Object(id);
        if has_drains {
            let (units, need) = a.drain();
            match nearest(model, o.level, &[ServiceSystem::Drainage], &outline) {
                Some((r, _)) => {
                    let ok = r.width + TOLERANCE >= need / 1000.0;
                    push(
                        report,
                        if ok { Status::Passes } else { Status::Fails },
                        subject,
                        o.level,
                        "HS 5, tabla 4.1",
                        format!(
                            "{}: desagüe de {} ({units} UD)",
                            a.name(),
                            r.system.name().to_lowercase()
                        ),
                        mm(r.width),
                        format!("≥ {}", mm(need / 1000.0)),
                    );
                }
                None => push(
                    report,
                    Status::Warning,
                    subject,
                    o.level,
                    "HS 5, tabla 4.1",
                    format!(
                        "{}: no hay ningún desagüe dibujado a menos de {} m",
                        a.name(),
                        crate::m(REACH).trim_end_matches(" m")
                    ),
                    String::new(),
                    format!("≥ {}", mm(need / 1000.0)),
                ),
            }
        }
        if has_water {
            let need = a.supply();
            let mut systems = vec![ServiceSystem::ColdWater];
            if a.hot() {
                systems.push(ServiceSystem::HotWater);
            }
            for s in systems {
                match nearest(model, o.level, &[s], &outline) {
                    Some((r, _)) => {
                        let ok = r.width + TOLERANCE >= need / 1000.0;
                        push(
                            report,
                            if ok { Status::Passes } else { Status::Fails },
                            subject,
                            o.level,
                            "HS 4, tabla 4.2",
                            format!("{}: derivación de {}", a.name(), s.name().to_lowercase()),
                            mm(r.width),
                            format!("≥ {}", mm(need / 1000.0)),
                        );
                    }
                    None if any(s) => push(
                        report,
                        Status::Warning,
                        subject,
                        o.level,
                        "HS 4, tabla 4.2",
                        format!(
                            "{}: no hay ninguna tubería de {} a menos de {} m",
                            a.name(),
                            s.name().to_lowercase(),
                            crate::m(REACH).trim_end_matches(" m")
                        ),
                        String::new(),
                        format!("≥ {}", mm(need / 1000.0)),
                    ),
                    None => {}
                }
            }
        }
    }
    // The slope of every drain.
    for r in model.runs().filter(|r| r.system == ServiceSystem::Drainage) {
        let subject = Subject::Run(r.id);
        let value = crate::pct(r.slope);
        if r.width < BRANCH {
            let ok = (0.02 - 1e-9..=0.05 + 1e-9).contains(&r.slope);
            push(
                report,
                if ok { Status::Passes } else { Status::Fails },
                subject,
                r.level,
                "HS 5, 3.3.1.1",
                format!("Pendiente de la derivación {}", mm(r.width)),
                value,
                "2,0 % a 5,0 %".into(),
            );
        } else {
            let buried = r.elevation.min(r.end_elevation()) < 0.0;
            let need = if buried { 0.02 } else { 0.01 };
            let ok = r.slope + 1e-9 >= need;
            push(
                report,
                if ok { Status::Passes } else { Status::Fails },
                subject,
                r.level,
                if buried {
                    "HS 5, 3.3.1.5"
                } else {
                    "HS 5, 3.3.1.4"
                },
                format!(
                    "Pendiente del colector {} {}",
                    if buried { "enterrado" } else { "colgado" },
                    mm(r.width)
                ),
                value,
                format!("≥ {}", crate::pct(need)),
            );
        }
    }
}

/// The calculation of the water supply (`archisim-mep::water`): the
/// pressure at every tap and at the heater, the diameter of every
/// stretch for its flow and the speed of the water.
pub fn calculation(model: &Model, report: &mut Report) {
    use archisim_mep::water::{self as wt, Source};
    let w = wt::water(model);
    let dec = |v: f64, n: usize| format!("{v:.n$}").replace('.', ",");
    let first_level = model.levels().first().map(|l| l.id);
    for n in &w.networks {
        let mut push = |status: Status,
                        subject: Subject,
                        level: Option<archisim_core::LevelId>,
                        rule: &'static str,
                        text: String,
                        value: String,
                        required: String| {
            report.items.push(Item {
                section: Section::Services,
                status,
                subject,
                level,
                rule,
                text,
                value,
                required,
                route: Vec::new(),
            });
        };
        let system = n.system.name().to_lowercase();
        match &n.source {
            Source::End(level, _) if n.system == ServiceSystem::ColdWater => push(
                Status::Warning,
                Subject::Level(*level),
                Some(*level),
                "HS 4, 3.2.1.1",
                "No hay contador dibujado: el agua fría se calcula desde el extremo más bajo de las tuberías".into(),
                String::new(),
                String::new(),
            ),
            Source::End(level, _) => push(
                Status::Warning,
                Subject::Level(*level),
                Some(*level),
                "HS 4, 3.2.2",
                "No hay termo ni calentador dibujado junto al agua caliente: se calcula desde su extremo más bajo".into(),
                String::new(),
                String::new(),
            ),
            Source::Device(..) => {}
        }
        if n.system == ServiceSystem::ColdWater {
            push(
                if n.source_pressure <= wt::MAX_PRESSURE {
                    Status::Passes
                } else {
                    Status::Fails
                },
                Subject::Building,
                first_level,
                "HS 4, 2.1.3",
                "Presión de la red en el contador (con más de 500 kPa, reductor de presión)".into(),
                format!("{} kPa", dec(n.source_pressure, 0)),
                format!("≤ {} kPa", wt::MAX_PRESSURE),
            );
        }
        for t in &n.taps {
            push(
                if t.pressure + 1e-6 >= wt::TAP_PRESSURE {
                    Status::Passes
                } else {
                    Status::Fails
                },
                Subject::Object(t.object),
                Some(t.level),
                "HS 4, 2.1.3",
                format!("{}: presión del {system} en el grifo", t.kind.name()),
                format!("{} kPa", dec(t.pressure, 0)),
                format!("≥ {} kPa", wt::TAP_PRESSURE),
            );
        }
        if let Some((heater, p, _)) = n.heater {
            let level = model.devices().find(|d| d.id == heater).map(|d| d.level);
            push(
                if p + 1e-6 >= wt::HEATER_PRESSURE {
                    Status::Passes
                } else {
                    Status::Fails
                },
                Subject::Device(heater),
                level,
                "HS 4, 2.1.3",
                "Presión del agua fría en la entrada del calentador".into(),
                format!("{} kPa", dec(p, 0)),
                format!("≥ {} kPa", wt::HEATER_PRESSURE),
            );
        }
        let top_speed = if w.material.is_metal() { 2.0 } else { 3.5 };
        for s in &n.stretches {
            let subject = match (s.run, s.riser) {
                (Some(r), _) => Subject::Run(r),
                (None, Some(d)) => Subject::Device(d),
                _ => Subject::Level(s.level),
            };
            push(
                if s.drawn.0 + 1e-4 >= s.sized.0 {
                    Status::Passes
                } else {
                    Status::Fails
                },
                subject,
                Some(s.level),
                "HS 4, 4.2",
                format!(
                    "Tramo {} del {system}: {} aparato{}, Qc = {} l/s",
                    s.name,
                    s.appliances,
                    if s.appliances == 1 { "" } else { "s" },
                    dec(s.flow, 2)
                ),
                format!("Ø{:.0} mm", s.drawn.0 * 1000.0),
                format!("≥ Ø{:.0} mm", s.sized.0 * 1000.0),
            );
            if s.speed > top_speed + 1e-6 {
                push(
                    Status::Fails,
                    subject,
                    Some(s.level),
                    "HS 4, 4.2.1",
                    format!("Tramo {} del {system}: velocidad del agua", s.name),
                    format!("{} m/s", dec(s.speed, 2)),
                    format!("≤ {} m/s", dec(top_speed, 2)),
                );
            }
        }
        for o in &n.loose {
            let level = model.objects().find(|x| x.id == *o).map(|x| x.level);
            push(
                Status::Warning,
                Subject::Object(*o),
                level,
                "HS 4, 3.2.1",
                format!("Aparato sin unir a la red de {system}: no entra en el cálculo"),
                String::new(),
                String::new(),
            );
        }
    }
}
