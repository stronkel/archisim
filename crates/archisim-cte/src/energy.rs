//! DB-HE 1, the envelope, from the calculation of
//! `archisim_mep::thermal`: the transmittance of every type of the
//! envelope against its limit, the global coefficient K, and the loads
//! and equipment as figures (the radiators drawn, against their room);
//! and DB-HE 4 and HE 5, the renewables, from `archisim_mep::renewables`.

use archisim_core::Model;
use archisim_mep::thermal as th;

use crate::{Item, Report, Section, Status, Subject};

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

pub fn check(model: &Model, report: &mut Report) {
    renewables(model, report);
    let t = th::thermal(model);
    if t.rooms.is_empty() {
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
            section: Section::Energy,
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
    let w = |v: f64| format!("{} W/m²K", dec(v, 2));
    for e in &t.elements {
        push(
            if e.fails() {
                Status::Fails
            } else {
                Status::Passes
            },
            Subject::Building,
            None,
            "HE 1, tabla 3.1.1.a",
            format!(
                "{} «{}» ({} m², zona {})",
                e.kind.name(),
                e.type_name,
                dec(e.area, 1),
                t.site.zone()
            ),
            w(e.u),
            format!("≤ {}", w(e.limit)),
        );
    }
    push(
        if t.k_fails() {
            Status::Fails
        } else {
            Status::Passes
        },
        Subject::Building,
        None,
        "HE 1, tabla 3.1.1.b",
        format!(
            "Coeficiente global K de la envolvente (V/A = {} m)",
            dec(t.compactness, 2)
        ),
        w(t.k),
        format!("≤ {}", w(t.k_limit)),
    );
    for r in &t.rooms {
        if r.radiators_short() {
            push(
                Status::Fails,
                Subject::Room(r.room),
                Some(r.level),
                "Cargas térmicas",
                format!(
                    "{}: los radiadores no cubren la carga de calefacción",
                    r.name
                ),
                format!("{} W", dec(r.radiator_power, 0)),
                format!("≥ {} W", dec(r.heating, 0)),
            );
        }
    }
    for g in &t.groups {
        push(
            Status::Info,
            Subject::Building,
            None,
            "Cargas térmicas",
            format!("{}: bomba de calor", g.name),
            format!("{} kW", dec(g.heat_pump, 1)),
            format!(
                "calefacción {} kW, refrigeración {} kW",
                dec(g.heating / 1000.0, 2),
                dec(g.cooling / 1000.0, 2)
            ),
        );
    }
}

/// HE 4 and HE 5, where they apply.
fn renewables(model: &Model, report: &mut Report) {
    let r = archisim_mep::renewables::renewables(model);
    let pct = |v: f64| format!("{} %", dec(v * 100.0, 0));
    let item = |status, rule, text, value, required| Item {
        section: Section::Energy,
        status,
        subject: Subject::Building,
        level: None,
        rule,
        text,
        value,
        required,
        route: Vec::new(),
    };
    let h = &r.hot_water;
    if h.applies {
        report.items.push(item(
            if h.passes() {
                Status::Passes
            } else {
                Status::Fails
            },
            "HE 4, 3.1",
            format!(
                "Agua caliente: {} l/día a 60 °C, {} kWh al año; renovable con {}{}",
                dec(h.daily, 0),
                dec(h.annual, 0),
                h.source.name().to_lowercase(),
                if h.weak_pump {
                    " (su SCOP no llega a 2,5: no cuenta)"
                } else {
                    ""
                }
            ),
            pct(h.provided),
            format!("≥ {}", pct(h.required)),
        ));
    }
    let e = &r.electricity;
    if e.applies {
        report.items.push(item(
            if e.passes() {
                Status::Passes
            } else {
                Status::Fails
            },
            "HE 5, 3.1",
            format!(
                "Fotovoltaica: {} m² construidos, {} m² de cubierta no transitable; {} m² de placas",
                dec(e.built, 0),
                dec(e.roof, 0),
                dec(e.panel_area, 1)
            ),
            format!("{} kWp", dec(e.installed, 2)),
            format!("≥ {} kWp", dec(e.minimum, 2)),
        ));
    }
    if h.applies || e.applies {
        report.items.push(item(
            Status::Info,
            "HE 0",
            "El consumo de energía primaria (HE 0) se justifica con la herramienta oficial".into(),
            String::new(),
            String::new(),
        ));
    }
}
