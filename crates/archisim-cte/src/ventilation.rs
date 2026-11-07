//! DB-HS 3, the dwellings' ventilation, from the calculation of
//! `archisim_mep::ventilation`: the flow of every dwelling, the inlets
//! and valves of every room, the passages of the doors, the extract ducts
//! (mechanical or hybrid), where each comes out, the fan, and the
//! garages, storerooms and refuse rooms.

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
    }
    for h in &v.hoods {
        let what = format!("{}: {}", h.dwelling, vt::hood_text(&v, h));
        let (status, subject) = match h.object {
            None => (Status::Warning, Subject::Room(h.kitchen)),
            Some(o) if v.hood_fails(h) => (Status::Fails, Subject::Object(o)),
            Some(o) => (Status::Passes, Subject::Object(o)),
        };
        push(
            status,
            subject,
            Some(h.level),
            "HS 3, 3.1.1",
            what,
            String::new(),
            format!("{} l/s, conducto propio", dec(vt::HOOD_FLOW, 0)),
        );
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
                if n.hybrid.is_some() {
                    "HS 3, tabla 4.2"
                } else {
                    "HS 3, 4.2.2"
                },
                if s.hoods > 0 && s.vents == 0 {
                    format!(
                        "Tramo {} del conducto de campana: {} l/s de {} campana{}",
                        s.name,
                        dec(s.flow, 1),
                        s.hoods,
                        if s.hoods == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "Tramo {} de extracción: {} l/s de {} boca{}",
                        s.name,
                        dec(s.flow, 1),
                        s.vents,
                        if s.vents == 1 { "" } else { "s" }
                    )
                },
                format!("{} ({})", s.size, cm2(s.area)),
                format!("≥ {} ({})", cm2(s.needed), s.sized_text),
            );
        }
        if let Some(o) = &n.outlet {
            push(
                pass(o.fails()),
                o.fan.map_or(Subject::Level(o.level), Subject::Device),
                Some(o.level),
                "HS 3, 3.2.1",
                format!(
                    "Salida de la red de {}: {}",
                    dec(n.flow, 0) + " l/s",
                    vt::outlet_text(o)
                ),
                match &o.intake {
                    Some((_, d)) => format!("{} m", dec(*d, 2)),
                    None => "—".into(),
                },
                if o.hybrid {
                    format!(
                        "≥ {} m de las entradas de aire; ≥ {} m sobre la cubierta",
                        dec(vt::OUTLET_GAP, 0),
                        dec(o.required, 2)
                    )
                } else {
                    format!("≥ {} m de las entradas de aire", dec(vt::OUTLET_GAP, 0))
                },
            );
        }
        if let Some((floors, class)) = n.hybrid {
            push(
                Status::Info,
                Subject::Building,
                None,
                "HS 3, tablas 4.3 y 4.4",
                format!(
                    "Conducto híbrido: {} planta{} vierten a él, zona {}",
                    floors,
                    if floors == 1 { "" } else { "s" },
                    model.ventilation_site().zone.code()
                ),
                format!("tiro T-{class}"),
                String::new(),
            );
        }
        if n.garage_shared() {
            push(
                Status::Fails,
                Subject::Building,
                None,
                "HS 3, 3.1.4",
                "La red de extracción del aparcamiento sirve también a otros locales".into(),
                String::new(),
                "exclusiva del aparcamiento".into(),
            );
        }
        if n.is_hood_duct() {
            continue;
        }
        let head = if n.hybrid.is_some() {
            "Aspirador híbrido"
        } else {
            "Extractor"
        };
        match n.fan {
            Some(f) => push(
                Status::Info,
                Subject::Device(f),
                model.device(f).map(|d| d.level),
                "HS 3, 3.2",
                format!("{head}: caudal de su red"),
                format!("{} m³/h", dec(n.flow * 3.6, 0)),
                String::new(),
            ),
            None => push(
                Status::Fails,
                Subject::Building,
                None,
                "HS 3, 3.2",
                format!(
                    "Red de extracción de {} bocas sin {} dibujado",
                    n.vents.len(),
                    head.to_lowercase()
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
        let level = Some(o.level);
        let subject = Subject::Room(o.room);
        push(
            Status::Info,
            subject,
            level,
            "HS 3, tabla 2.2",
            if o.garage {
                format!(
                    "{} (aparcamiento, {} m², {} plaza{}{}): caudal",
                    o.name,
                    dec(o.area, 1),
                    o.spaces,
                    if o.spaces == 1 { "" } else { "s" },
                    if o.cars_drawn { "" } else { " estimadas" }
                )
            } else {
                format!("{} ({}, {} m²): caudal", o.name, o.what, dec(o.area, 1))
            },
            format!("{} l/s", dec(o.flow, 1)),
            String::new(),
        );
        if !o.drawn() {
            push(
                if o.garage {
                    Status::Warning
                } else {
                    Status::Info
                },
                subject,
                level,
                if o.garage { "HS 3, 3.1.4" } else { "HS 3, 3.1" },
                format!("{}: {}", o.name, vt::other_lines(o).join("; ")),
                String::new(),
                String::new(),
            );
            continue;
        }
        if !o.vents.is_empty() {
            push(
                pass(o.vent_short()),
                subject,
                level,
                "HS 3, tabla 4.1",
                format!(
                    "{}: aberturas de extracción para {} l/s",
                    o.name,
                    dec(o.flow, 0)
                ),
                cm2(o.vent_area),
                format!("≥ {}", cm2(o.vent_needed())),
            );
        }
        if !o.inlets.is_empty() && o.inlet_short() {
            push(
                Status::Warning,
                subject,
                level,
                "HS 3, tabla 4.1",
                format!(
                    "{}: aberturas de admisión dibujadas cortas (a completar con rejillas en la puerta)",
                    o.name
                ),
                cm2(o.inlet_area),
                format!("≥ {}", cm2(o.inlet_needed())),
            );
        }
        if o.garage && !o.vents.is_empty() {
            let need = o.openings_needed();
            push(
                pass(o.vents.len() < need),
                subject,
                level,
                "HS 3, 3.1.4",
                format!("{}: bocas de extracción, una cada 100 m²", o.name),
                format!("{}", o.vents.len()),
                format!("≥ {need}"),
            );
            if o.vents.len() > 1 {
                push(
                    pass(o.vent_gap > vt::GARAGE_VENT_SPACING + 1e-6),
                    subject,
                    level,
                    "HS 3, 3.1.4",
                    format!("{}: distancia entre bocas de extracción", o.name),
                    format!("{} m", dec(o.vent_gap, 1)),
                    format!("< {} m", dec(vt::GARAGE_VENT_SPACING, 0)),
                );
            }
            if o.spaces >= vt::GARAGE_TWO_NETWORKS {
                push(
                    pass(o.networks < 2),
                    subject,
                    level,
                    "HS 3, 3.1.4",
                    format!("{}: redes de extracción en la planta", o.name),
                    format!("{}", o.networks),
                    "≥ 2, cada una con su extractor".into(),
                );
            }
        }
        if o.needs_co() {
            push(
                Status::Warning,
                subject,
                level,
                "HS 3, 3.1.4",
                format!(
                    "{}: detección de monóxido de carbono que ponga en marcha los extractores",
                    o.name
                ),
                String::new(),
                "50 ppm (100 ppm sin personal)".into(),
            );
        }
    }
}
