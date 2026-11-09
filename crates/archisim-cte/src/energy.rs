//! DB-HE 1, the envelope, from the calculation of
//! `archisim_mep::thermal`: the transmittance of every type of the
//! envelope against its limit, the global coefficient K with the thermal
//! bridges, the solar control of July, the air permeability of the
//! windows and n50, the surface condensation of the opaque types and the
//! bridges, and the loads and equipment as figures (the radiators drawn,
//! against their room);
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
    for (_, g) in &t.ground {
        push(
            Status::Info,
            Subject::Building,
            None,
            "HE 1, UNE-EN ISO 13370",
            format!(
                "Suelo sobre el terreno, U equivalente: B' = {} m (A {} m², P {} m){}; con sus capas solas, {}",
                dec(g.b, 2),
                dec(g.area, 1),
                dec(g.perimeter, 1),
                if g.depth > 0.0 {
                    format!(", a {} m bajo el terreno", dec(g.depth, 2))
                } else {
                    String::new()
                },
                w(g.u_layers)
            ),
            w(g.u),
            String::new(),
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
            "Coeficiente global K de la envolvente, con {} W/K de puentes térmicos (V/A = {} m)",
            dec(t.psi_l, 1),
            dec(t.compactness, 2)
        ),
        w(t.k),
        format!("≤ {}", w(t.k_limit)),
    );
    let sol = &t.solar;
    if sol.useful > 0.0 && sol.windows + sol.by_facing.len() > 0 {
        push(
            if sol.fails() {
                Status::Fails
            } else {
                Status::Passes
            },
            Subject::Building,
            None,
            "HE 1, 3.1.2",
            format!(
                "Control solar de julio q sol;jul ({} m² útiles; {} de {} ventanas con un forjado, balcón o alero encima)",
                dec(sol.useful, 0),
                sol.overhung,
                sol.windows
            ),
            format!("{} kWh/m²·mes", dec(sol.q, 2)),
            format!("≤ {} kWh/m²·mes", dec(sol.limit, 2)),
        );
    }
    for a in &t.air.windows {
        push(
            if a.fails() {
                Status::Fails
            } else {
                Status::Passes
            },
            Subject::Building,
            None,
            "HE 1, tabla 3.1.3.a",
            format!(
                "Permeabilidad de «{}» (clase {}, zona {})",
                a.type_name,
                a.class,
                t.site.winter.letter()
            ),
            format!("{} m³/h·m²", dec(a.q100, 0)),
            format!("≤ {} m³/h·m²", dec(a.limit, 0)),
        );
    }
    if t.air.applies {
        push(
            if t.air.n50_fails() {
                Status::Fails
            } else {
                Status::Passes
            },
            Subject::Building,
            None,
            "HE 1, tabla 3.1.3.b",
            format!(
                "Relación de cambio de aire a 50 Pa ({})",
                if t.air.measured {
                    "medida"
                } else {
                    "anejo H, método simplificado"
                }
            ),
            format!("{} h⁻¹", dec(t.air.n50, 2)),
            format!("≤ {} h⁻¹", dec(t.air.limit, 2)),
        );
    }
    for c in &t.condensation {
        push(
            if c.fails() {
                Status::Fails
            } else {
                Status::Passes
            },
            Subject::Building,
            None,
            "HE 1, condensaciones superficiales",
            format!(
                "{} «{}» (higrometría {})",
                c.kind.name(),
                c.type_name,
                t.site.hygrometry
            ),
            format!("fRsi {}", dec(c.f_rsi, 2)),
            format!("≥ {}", dec(c.f_min, 2)),
        );
    }
    for b in &t.bridges {
        push(
            if b.condenses() {
                Status::Fails
            } else {
                Status::Info
            },
            Subject::Building,
            None,
            "HE 1, puentes térmicos",
            format!(
                "{} ({}): {} m × ψ {} W/mK; fRsi {}",
                b.junction.name(),
                b.insulation.name(),
                dec(b.length, 1),
                dec(b.psi, 2),
                dec(b.f_rsi, 2)
            ),
            format!("{} W/K", dec(b.h, 1)),
            if b.condenses() {
                format!("fRsi ≥ {}", dec(b.f_min, 2))
            } else {
                String::new()
            },
        );
    }
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
                    " (su SCOP no llega a 2,5: no cuenta)".to_string()
                } else if let Some(f) = &h.solar {
                    format!(" (f-Chart, {} m² de captadores dibujados)", dec(f.area, 1))
                } else if h.source == archisim_core::AcsSource::Solar {
                    " (fracción dada: no hay captadores dibujados)".to_string()
                } else {
                    String::new()
                }
            ),
            pct(h.provided),
            format!("≥ {}", pct(h.required)),
        ));
        if let Some(f) = h
            .solar
            .as_ref()
            .filter(|f| f.estimated_sun || f.estimated_air)
        {
            report.items.push(item(
                Status::Warning,
                "HE 4, 3.1",
                "Fracción solar con el clima estimado por la latitud: dar el del lugar (PVGIS, AEMET) en Energías renovables".into(),
                format!("{} %", dec(f.annual * 100.0, 1)),
                String::new(),
            ));
        }
        if let Some(f) = h.solar.as_ref().filter(|f| !f.over.is_empty()) {
            report.items.push(item(
                Status::Warning,
                "HE 4",
                "Los captadores darían más de lo que se gasta algún mes: prever la protección contra sobrecalentamientos".into(),
                format!("{} meses", f.over.len()),
                String::new(),
            ));
        }
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
