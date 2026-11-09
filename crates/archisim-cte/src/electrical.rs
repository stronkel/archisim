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

/// The calculation of each board's installation and of the building
/// (`archisim-mep`).
fn calculation(model: &Model, report: &mut Report) {
    use archisim_mep::electrical::{self as el, fmt, ka, section_text};
    let e = el::analysis(model);
    let site = *model.electrical_site();
    if let Some(b) = e.building.as_ref().filter(|b| b.general.is_some()) {
        let mut push =
            |status: Status, rule: &'static str, text: String, value: String, required: String| {
                report.items.push(Item {
                    section: Section::Electrical,
                    status,
                    subject: Subject::Building,
                    level: None,
                    rule,
                    text,
                    value,
                    required,
                    route: Vec::new(),
                });
            };
        push(
            Status::Info,
            "ITC-BT-10",
            format!(
                "Previsión de cargas del edificio: {} viviendas ({} W × {}), servicios generales {} W, locales {} W, garaje {} W; suministro {}",
                b.dwellings,
                fmt(b.dwelling_mean, 0),
                fmt(b.coefficient, 1),
                fmt(b.services, 0),
                fmt(b.locals, 0),
                fmt(b.garage, 0),
                if b.three_phase { "trifásico" } else { "monofásico" }
            ),
            format!("{} W", fmt(b.total, 0)),
            String::new(),
        );
        if let Some(g) = b.general {
            push(
                if g.passes() {
                    Status::Passes
                } else {
                    Status::Fails
                },
                "ITC-BT-14",
                format!(
                    "Línea general de alimentación de {} m, {} mm² Cu, tubo Ø{}, {} A ≤ Iz {} A",
                    fmt(g.length, 1),
                    g.wires(),
                    g.tube,
                    fmt(g.current, 1),
                    fmt(g.ampacity, 0)
                ),
                format!("ΔU {} %", fmt(g.drop * 100.0, 2)),
                format!("≤ {} %", fmt(g.limit * 100.0, 1)),
            );
        }
        if b.three_phase {
            push(
                if b.unbalance > el::UNBALANCE_WARNING {
                    Status::Warning
                } else {
                    Status::Info
                },
                "ITC-BT-10",
                format!(
                    "Reparto de las instalaciones por fases: L1 {} W, L2 {} W, L3 {} W",
                    fmt(b.phase_loads[0], 0),
                    fmt(b.phase_loads[1], 0),
                    fmt(b.phase_loads[2], 0)
                ),
                format!("desequilibrio {} %", fmt(b.unbalance * 100.0, 0)),
                format!("≤ {} %", fmt(el::UNBALANCE_WARNING * 100.0, 0)),
            );
        }
    }
    for inst in &e.installations {
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
        let what = match inst.use_ {
            el::BoardUse::Dwelling => format!(
                "Vivienda de electrificación {}",
                match inst.grade {
                    archisim_core::Electrification::High => "elevada",
                    _ => "básica",
                }
            ),
            el::BoardUse::Services => "Servicios generales".into(),
            el::BoardUse::Local => format!("Local de {} m²", fmt(inst.area, 0)),
            el::BoardUse::Garage => format!("Garaje de {} m²", fmt(inst.area, 0)),
        };
        push(
            Status::Info,
            "ITC-BT-10",
            format!(
                "{what}, {}{}",
                if inst.three_phase {
                    "trifásica 3×400/230 V"
                } else {
                    "monofásica 230 V"
                },
                if inst.reasons.is_empty() {
                    String::new()
                } else {
                    format!(": {}", inst.reasons.join("; "))
                }
            ),
            format!("{} W", fmt(inst.power, 0)),
            format!("IGA {} A {}", inst.main_switch, inst.poles()),
        );
        if inst.three_phase {
            push(
                if inst.unbalance > el::UNBALANCE_WARNING {
                    Status::Warning
                } else {
                    Status::Info
                },
                "ITC-BT-25",
                format!(
                    "Circuitos por fases: L1 {} W, L2 {} W, L3 {} W",
                    fmt(inst.phase_loads[0], 0),
                    fmt(inst.phase_loads[1], 0),
                    fmt(inst.phase_loads[2], 0)
                ),
                format!("desequilibrio {} %", fmt(inst.unbalance * 100.0, 0)),
                format!("≤ {} %", fmt(el::UNBALANCE_WARNING * 100.0, 0)),
            );
        }
        let f = inst.feeder;
        push(
            if f.passes() {
                Status::Passes
            } else {
                Status::Fails
            },
            "ITC-BT-15",
            format!(
                "Derivación individual de {} m{}, {} mm² Cu, tubo Ø{}",
                fmt(f.length, 1),
                if f.measured {
                    " desde los contadores"
                } else {
                    ""
                },
                f.wires(),
                f.tube
            ),
            format!("ΔU {} %", fmt(f.drop * 100.0, 2)),
            format!("≤ {} %", fmt(f.limit * 100.0, 1)),
        );
        push(
            if inst.breaks() {
                Status::Passes
            } else {
                Status::Fails
            },
            "ITC-BT-22, poder de corte",
            format!(
                "Cortocircuito en el cuadro de {}: IGA y PIA de {} de poder de corte",
                ka(inst.fault),
                ka(inst.breaking_capacity)
            ),
            ka(inst.fault),
            format!("≤ {}", ka(inst.breaking_capacity)),
        );
        for c in &inst.circuits {
            let phase = if c.phase.is_some() {
                format!(" ({})", c.phase_text())
            } else {
                String::new()
            };
            push(
                if c.protected() {
                    Status::Passes
                } else {
                    Status::Fails
                },
                "ITC-BT-19 y 25",
                format!(
                    "{}{phase} {}: {} W, Ib {} A ≤ PIA {} A ≤ Iz {} A",
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
            let stretches = c.worst_path().len();
            let by = if stretches > 1 {
                format!(" en {stretches} tramos")
            } else {
                String::new()
            };
            push(
                if c.drop <= el::CIRCUIT_DROP + 1e-9 {
                    if c.bigger_section() && !c.raised_for_trip {
                        Status::Warning
                    } else {
                        Status::Passes
                    }
                } else {
                    Status::Fails
                },
                "ITC-BT-19, 2.2.2",
                if c.bigger_section() && !c.raised_for_trip {
                    format!(
                        "{} caída de tensión en {} m{by}: con {} mm² sería un {} %; pide {} mm²",
                        c.code(),
                        fmt(c.length, 1),
                        section_text(c.section),
                        fmt(c.table_drop * 100.0, 2),
                        section_text(c.chosen)
                    )
                } else {
                    format!(
                        "{} caída de tensión en {} m{by} con {} mm²",
                        c.code(),
                        fmt(c.length, 1),
                        section_text(c.chosen)
                    )
                },
                format!("{} %", fmt(c.drop * 100.0, 2)),
                "≤ 3 %".into(),
            );
            push(
                if !c.trips() {
                    Status::Fails
                } else if c.raised_for_trip {
                    Status::Warning
                } else {
                    Status::Passes
                },
                "ITC-BT-22, disparo",
                if c.raised_for_trip {
                    format!(
                        "{} cortocircuito al final: con {} mm² no dispararía la curva {}; pide {} mm²",
                        c.code(),
                        section_text(c.section),
                        site.curve.name(),
                        section_text(c.chosen)
                    )
                } else {
                    format!(
                        "{} cortocircuito al final ({} m): el PIA de {} A curva {} dispara",
                        c.code(),
                        fmt(
                            c.stretches
                                .iter()
                                .map(|s| s.distance)
                                .fold(c.length, f64::max),
                            1
                        ),
                        c.breaker,
                        site.curve.name()
                    )
                },
                ka(c.fault_end),
                format!("≥ {}", ka(c.trip / 1000.0)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewDevice, Point2};

    #[test]
    fn breakers_that_do_not_break_are_fixed() {
        let mut m = Model::new("Casa");
        let l = m.add_level("Planta", 0.0);
        for (kind, x) in [(DeviceKind::Panel, 0.0), (DeviceKind::Light, 3.0)] {
            let (w, d, h, z) = kind.defaults();
            m.add_device(NewDevice {
                level: l,
                kind,
                name: kind.name().into(),
                position: Point2::new(x, 0.0),
                rotation: 0.0,
                elevation: z,
                width: w,
                depth: d,
                height: h,
            })
            .unwrap();
        }
        let mut site = *m.electrical_site();
        site.supply_fault = 20.0;
        site.feeder_length = 4.0;
        m.set_electrical_site(site).unwrap();
        let breaks = |m: &Model| {
            crate::check(m)
                .items
                .iter()
                .filter(|i| i.rule == "ITC-BT-22, poder de corte")
                .all(|i| i.status == Status::Passes)
        };
        assert!(!breaks(&m));
        // Every circuit trips at its end with curve C.
        assert!(crate::check(&m)
            .items
            .iter()
            .filter(|i| i.rule == "ITC-BT-22, disparo")
            .all(|i| i.status == Status::Passes));
        let fix = crate::fix::fix(&m);
        assert!(fix
            .changes
            .contains(&crate::fix::Change::BreakingCapacity { ka: 10.0 }));
        assert!(fix.lines.iter().any(|l| l.what == "Poder de corte"));
        crate::fix::apply_all(&mut m, &fix).unwrap();
        assert!(breaks(&m));
        assert!((m.electrical_site().breaking_capacity - 10.0).abs() < 1e-12);
    }
}
