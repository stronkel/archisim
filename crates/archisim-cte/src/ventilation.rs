//! DB-HS 3, the dwellings' ventilation, from the calculation of
//! `archisim_mep::ventilation`: the flow of every dwelling, the inlets
//! and valves of every room, the passages of the doors, the extract ducts
//! and the fan.

use archisim_core::Model;
use archisim_mep::ventilation as vt;

use crate::{Item, Report, Section, Status, Subject};

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

pub fn check(model: &Model, report: &mut Report) {
    let v = vt::ventilation(model);
    if !v.applies {
        return;
    }
    let mut push = |status: Status,
                    subject: Subject,
                    level: Option<archisim_core::LevelId>,
                    rule: &'static str,
                    text: String,
                    value: String,
                    required: String| {
        report.items.push(Item {
            section: Section::Ventilation,
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
    let pass = |short: bool| if short { Status::Fails } else { Status::Passes };
    let cm2 = |v: f64| format!("{} cm²", dec(v, 0));
    for d in &v.dwellings {
        push(
            Status::Info,
            Subject::Level(d.level),
            Some(d.level),
            "HS 3, tabla 2.1",
            format!(
                "{}: {} dormitorio{}, caudal de ventilación",
                d.name,
                d.bedrooms,
                if d.bedrooms == 1 { "" } else { "s" }
            ),
            format!("{} l/s", dec(d.flow, 1)),
            format!(
                "secos {} l/s, húmedos {} l/s",
                dec(d.dry_flow, 1),
                dec(d.wet_flow, 1)
            ),
        );
        for r in &d.rooms {
            if r.admission > 0.0 {
                push(
                    pass(r.inlet_short()),
                    Subject::Room(r.room),
                    Some(d.level),
                    "HS 3, tabla 4.1",
                    format!(
                        "{}: aberturas de admisión para {} l/s",
                        r.name,
                        dec(r.admission, 1)
                    ),
                    cm2(r.inlet_area),
                    format!("≥ {}", cm2(r.inlet_needed())),
                );
            }
            if r.extraction > 0.0 {
                push(
                    pass(r.vent_short()),
                    Subject::Room(r.room),
                    Some(d.level),
                    "HS 3, tabla 4.1",
                    format!(
                        "{}: aberturas de extracción para {} l/s",
                        r.name,
                        dec(r.extraction, 1)
                    ),
                    cm2(r.vent_area),
                    format!("≥ {}", cm2(r.vent_needed())),
                );
            }
        }
        let name = |id| model.room(id).map_or("—".to_string(), |r| r.name.clone());
        for p in &d.passages {
            push(
                pass(p.short()),
                Subject::Opening(p.opening),
                Some(d.level),
                "HS 3, tabla 4.1",
                format!(
                    "Paso de {} a {}: {} l/s{}",
                    name(p.from),
                    name(p.to),
                    dec(p.flow, 1),
                    match (p.short(), p.gap_needed()) {
                        (true, Some(g)) => format!(" (holgura de {g} mm o rejilla de paso)"),
                        _ => String::new(),
                    }
                ),
                cm2(p.area),
                format!("≥ {}", cm2(p.needed())),
            );
        }
        if d.kitchens > 0 {
            push(
                Status::Info,
                Subject::Level(d.level),
                Some(d.level),
                "HS 3, 3.1.1",
                format!(
                    "{}: la cocina lleva extracción propia para la campana, con su conducto",
                    d.name
                ),
                format!("{} l/s", dec(vt::HOOD_FLOW, 0)),
                String::new(),
            );
        }
    }
    for n in &v.networks {
        for s in &n.stretches {
            let subject = match (s.run, s.shaft) {
                (Some(r), _) => Subject::Run(r),
                (None, Some(dv)) => Subject::Device(dv),
                _ => Subject::Level(s.level),
            };
            push(
                pass(s.short()),
                subject,
                Some(s.level),
                "HS 3, tabla 4.2",
                format!(
                    "Tramo {} de extracción: {} l/s de {} boca{}",
                    s.name,
                    dec(s.flow, 1),
                    s.vents,
                    if s.vents == 1 { "" } else { "s" }
                ),
                format!("{} ({})", s.size, cm2(s.area)),
                format!("≥ {} ({})", cm2(s.needed), s.sized_text),
            );
        }
        match n.fan {
            Some(f) => push(
                Status::Info,
                Subject::Device(f),
                model.device(f).map(|d| d.level),
                "HS 3, 3.2",
                "Extractor: caudal de su red".into(),
                format!("{} m³/h", dec(n.flow * 3.6, 0)),
                String::new(),
            ),
            None => push(
                Status::Fails,
                Subject::Building,
                None,
                "HS 3, 3.2",
                format!(
                    "Red de extracción de {} bocas sin extractor dibujado",
                    n.vents.len()
                ),
                "—".into(),
                format!("{} m³/h", dec(n.flow * 3.6, 0)),
            ),
        }
    }
    for id in &v.loose {
        push(
            Status::Warning,
            Subject::Device(*id),
            model.device(*id).map(|d| d.level),
            "HS 3, 3.2",
            "Boca de extracción sin conducto de extracción a menos de 45 cm".into(),
            String::new(),
            String::new(),
        );
    }
    for r in &v.lone_wet {
        push(
            Status::Warning,
            Subject::Room(*r),
            model.room(*r).map(|x| x.level),
            "HS 3, 2.1",
            format!(
                "{}: local húmedo sin puerta a ninguna vivienda, no se calcula",
                model.room(*r).map_or("—", |x| x.name.as_str())
            ),
            String::new(),
            String::new(),
        );
    }
    for o in &v.others {
        push(
            Status::Info,
            Subject::Room(o.room),
            model.room(o.room).map(|x| x.level),
            "HS 3, tabla 2.2",
            format!("{} ({}, {} m²): caudal", o.name, o.what, dec(o.area, 1)),
            format!("{} l/s", dec(o.flow, 1)),
            String::new(),
        );
    }
}
