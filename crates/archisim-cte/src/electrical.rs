//! The circuits of a dwelling (REBT, ITC-BT-25, tabla 1), on the
//! mechanisms drawn (E13): how many points each circuit feeds on each
//! level against the most it may, with its breaker and its wires. The
//! switches are not points; a light, a socket or a base is one, and so
//! is a pendant, a ceiling light or a wall light placed as an object.
//!
//! Then the calculation of each installation (`archisim-mep`): the
//! degree of electrification, the voltage drop and the protection of each
//! circuit and of the feeder.

use std::collections::BTreeMap;

use archisim_core::{circuit_spec, DeviceKind, Model};

use crate::{Item, Report, Section, Status, Subject};

pub fn check(model: &Model, report: &mut Report) {
    // Points of each circuit, by level.
    let mut points: BTreeMap<(archisim_core::LevelId, u8), u32> = BTreeMap::new();
    for d in model.devices() {
        if matches!(d.kind, DeviceKind::Switch | DeviceKind::TwoWaySwitch) {
            continue;
        }
        if let Some(n) = model.device_circuit(d.id) {
            *points.entry((d.level, n)).or_default() += 1;
        }
    }
    // The lamps placed as objects (from the Maqueta) are points of C1.
    for (_, level) in model.lamp_points() {
        *points.entry((level, 1)).or_default() += 1;
    }
    for ((level, n), count) in points {
        let Some(c) = circuit_spec(n) else {
            report.items.push(Item {
                section: Section::Electrical,
                status: Status::Info,
                subject: Subject::Level(level),
                level: Some(level),
                rule: "ITC-BT-25",
                text: format!("Circuito C{n}: {count} puntos (no es uno de los de la tabla 1)"),
                value: count.to_string(),
                required: String::new(),
                route: Vec::new(),
            });
            continue;
        };
        let ok = c.max_points == 0 || count <= c.max_points;
        report.items.push(Item {
            section: Section::Electrical,
            status: if ok { Status::Passes } else { Status::Fails },
            subject: Subject::Level(level),
            level: Some(level),
            rule: "ITC-BT-25, tabla 1",
            text: format!(
                "C{} {}: {count} puntos, magnetotérmico {} A, {} mm²",
                c.number,
                c.name,
                c.breaker,
                format!("{}", c.section).replace('.', ",")
            ),
            value: count.to_string(),
            required: format!("≤ {}", c.max_points),
            route: Vec::new(),
        });
    }
    calculation(model, report);
}

/// The calculation of each board's installation (`archisim-mep`).
fn calculation(model: &Model, report: &mut Report) {
    use archisim_mep::electrical::{self as el, fmt, section_text};
    for inst in el::installations(model) {
        let subject = inst
            .board
            .map_or(Subject::Level(inst.level), Subject::Device);
        let mut push =
            |status: Status, rule: &'static str, text: String, value: String, required: String| {
                report.items.push(Item {
                    section: Section::Electrical,
                    status,
                    subject,
                    level: Some(inst.level),
                    rule,
                    text,
                    value,
                    required,
                    route: Vec::new(),
                });
            };
        if inst.board.is_none() {
            push(
                Status::Warning,
                "ITC-BT-17",
                "No hay cuadro dibujado: el cálculo lo supone junto al primer mecanismo".into(),
                String::new(),
                String::new(),
            );
        }
        push(
            Status::Info,
            "ITC-BT-10",
            format!(
                "Electrificación {}{}",
                match inst.grade {
                    archisim_core::Electrification::High => "elevada",
                    _ => "básica",
                },
                if inst.reasons.is_empty() {
                    String::new()
                } else {
                    format!(": {}", inst.reasons.join("; "))
                }
            ),
            format!("{} W", fmt(inst.power, 0)),
            format!("IGA {} A", inst.main_switch),
        );
        let f = inst.feeder;
        push(
            if f.drop <= f.limit + 1e-9 {
                Status::Passes
            } else {
                Status::Fails
            },
            "ITC-BT-15",
            format!(
                "Derivación individual de {} m, 2×{}+TT×{} mm² Cu, tubo Ø{}",
                fmt(f.length, 1),
                section_text(f.section),
                section_text(f.section),
                f.tube
            ),
            format!("ΔU {} %", fmt(f.drop * 100.0, 2)),
            format!("≤ {} %", fmt(f.limit * 100.0, 1)),
        );
        for c in &inst.circuits {
            push(
                if c.protected() {
                    Status::Passes
                } else {
                    Status::Fails
                },
                "ITC-BT-19 y 25",
                format!(
                    "{} {}: {} W, Ib {} A ≤ PIA {} A ≤ Iz {} A",
                    c.code(),
                    c.spec.name,
                    fmt(c.power, 0),
                    fmt(c.current, 1),
                    c.breaker,
                    fmt(c.ampacity, 1)
                ),
                format!("{} A", fmt(c.current, 1)),
                format!("≤ {} A", c.breaker),
            );
            push(
                if c.drop <= el::CIRCUIT_DROP + 1e-9 {
                    if c.bigger_section() {
                        Status::Warning
                    } else {
                        Status::Passes
                    }
                } else {
                    Status::Fails
                },
                "ITC-BT-19, 2.2.2",
                if c.bigger_section() {
                    format!(
                        "{} caída de tensión en {} m: con {} mm² sería un {} %; pide {} mm²",
                        c.code(),
                        fmt(c.length, 1),
                        section_text(c.section),
                        fmt(c.table_drop * 100.0, 2),
                        section_text(c.chosen)
                    )
                } else {
                    format!(
                        "{} caída de tensión en {} m con {} mm²",
                        c.code(),
                        fmt(c.length, 1),
                        section_text(c.chosen)
                    )
                },
                format!("{} %", fmt(c.drop * 100.0, 2)),
                "≤ 3 %".into(),
            );
        }
    }
}
