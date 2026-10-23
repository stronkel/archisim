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

/// A sanitary appliance, as the tables name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Appliance {
    Washbasin,
    Bidet,
    Shower,
    Bath,
    Wc,
    Sink,
    Dishwasher,
    Washer,
}

impl Appliance {
    pub fn name(self) -> &'static str {
        match self {
            Appliance::Washbasin => "Lavabo",
            Appliance::Bidet => "Bidé",
            Appliance::Shower => "Ducha",
            Appliance::Bath => "Bañera",
            Appliance::Wc => "Inodoro con cisterna",
            Appliance::Sink => "Fregadero",
            Appliance::Dishwasher => "Lavavajillas",
            Appliance::Washer => "Lavadora",
        }
    }

    /// HS 5, tabla 4.1: units of discharge and the minimum diameter of its
    /// trap and its branch, private use (mm).
    pub fn drain(self) -> (u32, f64) {
        match self {
            Appliance::Washbasin => (1, 32.0),
            Appliance::Bidet => (2, 32.0),
            Appliance::Shower => (2, 40.0),
            Appliance::Bath => (3, 40.0),
            Appliance::Wc => (4, 100.0),
            Appliance::Sink | Appliance::Dishwasher | Appliance::Washer => (3, 40.0),
        }
    }

    /// HS 4, tabla 4.2: the minimum diameter of its supply, copper or
    /// plastic (mm).
    pub fn supply(self) -> f64 {
        match self {
            Appliance::Bath | Appliance::Washer => 20.0,
            _ => 12.0,
        }
    }

    /// Whether it takes hot water.
    pub fn hot(self) -> bool {
        !matches!(self, Appliance::Wc | Appliance::Washer)
    }

    /// What an object is, by the model ArchiSim makes for it or by the
    /// name of its type.
    pub fn of(model: &Model, id: ObjectId) -> Option<Appliance> {
        let o = model.objects().find(|o| o.id == id)?;
        let t = model.object_type(o.object_type)?;
        if t.mesh.is_none() {
            if let Some(kind) = model.family_prop(t.family) {
                let a = match kind.as_str() {
                    "wc" => Some(Appliance::Wc),
                    "bidet" => Some(Appliance::Bidet),
                    "washbasin" => Some(Appliance::Washbasin),
                    "bath" => Some(Appliance::Bath),
                    "shower" => Some(Appliance::Shower),
                    "counter" => Some(Appliance::Sink),
                    "washer" => Some(Appliance::Washer),
                    _ => None,
                };
                if a.is_some() {
                    return a;
                }
            }
        }
        let name = t.name.to_lowercase();
        [
            ("inodoro", Appliance::Wc),
            ("bidé", Appliance::Bidet),
            ("bide", Appliance::Bidet),
            ("lavabo", Appliance::Washbasin),
            ("lavamanos", Appliance::Washbasin),
            ("bañera", Appliance::Bath),
            ("ducha", Appliance::Shower),
            ("fregadero", Appliance::Sink),
            ("lavavajillas", Appliance::Dishwasher),
            ("lavadora", Appliance::Washer),
        ]
        .into_iter()
        .find(|(k, _)| name.contains(k))
        .map(|(_, a)| a)
    }
}

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
fn nearest<'a>(
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
