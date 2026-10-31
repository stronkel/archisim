//! The circuits of a dwelling (REBT, ITC-BT-25, tabla 1), on the
//! mechanisms drawn (E13): how many points each circuit feeds on each
//! level against the most it may, with its breaker and its wires. The
//! switches are not points; a light, a socket or a base is one, and so
//! is a pendant, a ceiling light or a wall light placed as an object.

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
}
