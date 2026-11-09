//! The installations' report («Memoria de instalaciones»): the
//! calculations of `archisim-mep` written as a document for the project
//! (electricity by the REBT, water supply by DB-HS 4, drainage by DB-HS 5,
//! ventilation by DB-HS 3, and the envelope and thermal loads by DB-HE 1),
//! as the same blocks as the structure's report, so that
//! [`crate::calc_report::pages`] puts it on A4 pages.
//!
//! A chapter is written only when its installation is drawn.

use archisim_core::{Electrification, LevelId, Model, ServiceSystem};
use archisim_mep::drain::{self as dr, Drainage};
use archisim_mep::electrical::{self as el, Electrical};
use archisim_mep::renewables::{self as rn, Renewables};
use archisim_mep::thermal::{self as th, Thermal};
use archisim_mep::ventilation::{self as vt, Ventilation};
use archisim_mep::water::{self as wa, Water};
use archisim_struct::lightframe::Status;
use archisim_struct::report::Block;

fn dec(v: f64, n: usize) -> String {
    format!("{v:.n$}").replace('.', ",")
}

fn col(name: &str, w: f64) -> (String, f64) {
    (name.to_string(), w)
}

fn level_name(model: &Model, l: LevelId) -> String {
    model.level(l).map_or("—".into(), |x| x.name.clone())
}

fn mm(v: f64) -> String {
    format!("Ø{:.0}", v * 1000.0)
}

fn check(fails: bool, text: String) -> Block {
    Block::Check {
        status: if fails { Status::Fail } else { Status::Pass },
        text,
    }
}

/// What the report carries: the calculations, once.
struct All {
    electrical: Electrical,
    water: Water,
    drainage: Drainage,
    ventilation: Ventilation,
    thermal: Thermal,
    renewables: Renewables,
}

/// The report's blocks.
pub fn report(model: &Model) -> Vec<Block> {
    let all = All {
        electrical: el::analysis(model),
        water: wa::water(model),
        drainage: dr::drainage(model),
        ventilation: vt::ventilation(model),
        thermal: th::thermal(model),
        renewables: rn::renewables(model),
    };
    let has_water = all.water.networks.iter().any(|n| !n.stretches.is_empty());
    let has_drainage = all
        .drainage
        .networks
        .iter()
        .any(|n| !n.stretches.is_empty());
    let has_ventilation =
        !all.ventilation.dwellings.is_empty() || !all.ventilation.others.is_empty();
    let has_rite = all.ventilation.rite.applies();
    let has_thermal = !all.thermal.rooms.is_empty();
    let mut chapters: Vec<(&str, Vec<Block>)> = Vec::new();
    if !all.electrical.installations.is_empty() {
        chapters.push((
            "Instalación eléctrica (REBT)",
            electricity(model, &all.electrical),
        ));
    }
    if has_water {
        chapters.push(("Suministro de agua (DB-HS 4)", water(model, &all.water)));
    }
    if has_drainage {
        chapters.push((
            "Evacuación de aguas (DB-HS 5)",
            drainage(model, &all.drainage),
        ));
    }
    if has_ventilation {
        chapters.push((
            "Calidad del aire interior (DB-HS 3)",
            ventilation(model, &all.ventilation),
        ));
    }
    if has_rite {
        chapters.push((
            "Calidad del aire interior (RITE IT 1.1.4.2)",
            rite(model, &all.ventilation),
        ));
    }
    if has_thermal {
        chapters.push((
            "Envolvente térmica y cargas (DB-HE 1)",
            thermal(model, &all.thermal),
        ));
    }
    if all.renewables.hot_water.applies || all.renewables.electricity.applies {
        chapters.push((
            "Energías renovables (DB-HE 4 y HE 5)",
            renewables(&all.renewables),
        ));
    }
    let mut out = vec![Block::Cover {
        title: "Memoria de instalaciones".into(),
        project: model.project_name.clone(),
        lines: vec![
            format!("Programa: ArchiSim {}", env!("CARGO_PKG_VERSION")),
            if chapters.is_empty() {
                "No hay instalaciones dibujadas.".into()
            } else {
                format!(
                    "Contenido: {}.",
                    chapters
                        .iter()
                        .map(|(t, _)| *t)
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            },
            "Autor de la memoria: ______________________________".into(),
            "Esta memoria la genera el programa a partir del modelo dibujado. Sus datos de partida (presión de la compañía, zona climática, conductividades, potencias) los revisa y asume el técnico que la firma.".into(),
        ],
    }];
    for (i, (title, blocks)) in chapters.into_iter().enumerate() {
        out.push(Block::PageBreak);
        out.push(Block::Heading(1, format!("{}. {title}", i + 1)));
        // The sections keep the chapter's number.
        out.extend(blocks.into_iter().map(|b| match b {
            Block::Heading(2, t) => Block::Heading(2, format!("{}.{t}", i + 1)),
            other => other,
        }));
    }
    out
}

// ---------------------------------------------------------------------
// Electricity
// ---------------------------------------------------------------------

pub fn electricity(model: &Model, e: &Electrical) -> Vec<Block> {
    let site = *model.electrical_site();
    let pct = |v: f64| format!("{} %", dec(v * 100.0, 2));
    let mut out = vec![Block::Para(
        "Instalación interior de baja tensión según el Reglamento Electrotécnico para Baja Tensión: previsión de cargas y grado de electrificación (ITC-BT-10), suministro monofásico a 230 V o trifásico a 3×400/230 V por encima de 14 490 W, línea general de alimentación (ITC-BT-14), derivaciones individuales (ITC-BT-15), circuitos (ITC-BT-25, tabla 1), intensidades admisibles de la UNE-HD 60364-5-52 (método B1, tubo empotrado, PVC), caída de tensión con cobre a 70 °C sumada tramo a tramo por el recorrido de los cables (ITC-BT-19) y cortocircuito por el método simplificado de la guía técnica, Icc = 0,8·U/R, con la resistencia de los conductores a 20 °C (ITC-BT-22).".into(),
    )];
    if let Some(b) = e
        .building
        .as_ref()
        .filter(|b| b.general.is_some() || e.installations.len() > 1)
    {
        out.push(Block::Heading(2, "Previsión de cargas del edificio".into()));
        let mut rows = Vec::new();
        if b.dwellings > 0 {
            rows.push(vec![
                format!("{} viviendas", b.dwellings),
                format!(
                    "{} W de media × {} = {} W",
                    dec(b.dwelling_mean, 0),
                    dec(b.coefficient, 1),
                    dec(b.dwellings_power, 0)
                ),
            ]);
        }
        for (name, v) in [
            ("Servicios generales", b.services),
            ("Locales y oficinas", b.locals),
            ("Garaje", b.garage),
        ] {
            if v > 0.0 {
                rows.push(vec![name.into(), format!("{} W", dec(v, 0))]);
            }
        }
        rows.push(vec![
            "Total".into(),
            format!(
                "{} W, {}",
                dec(b.total, 0),
                if b.three_phase {
                    "trifásico 3×400/230 V"
                } else {
                    "monofásico 230 V"
                }
            ),
        ]);
        if b.three_phase {
            rows.push(vec![
                "Reparto por fases".into(),
                format!(
                    "L1 {} W · L2 {} W · L3 {} W (desequilibrio {} %)",
                    dec(b.phase_loads[0], 0),
                    dec(b.phase_loads[1], 0),
                    dec(b.phase_loads[2], 0),
                    dec(b.unbalance * 100.0, 0)
                ),
            ]);
        }
        rows.push(vec![
            "Cortocircuito en la CGP".into(),
            if site.supply_fault > 0.0 {
                format!("{} (dado por la compañía)", el::ka(b.fault_supply))
            } else {
                format!(
                    "{} (desde el transformador, {} m de línea)",
                    el::ka(b.fault_supply),
                    dec(site.network_length, 0)
                )
            },
        ]);
        if let Some(g) = &b.general {
            rows.push(vec![
                "Línea general de alimentación".into(),
                format!(
                    "{} m, {} mm² Cu, tubo Ø{} mm, {} A, ΔU {} (≤ {})",
                    dec(g.length, 1),
                    g.wires(),
                    g.tube,
                    dec(g.current, 1),
                    pct(g.drop),
                    pct(g.limit)
                ),
            ]);
            rows.push(vec![
                "Cortocircuito en los contadores".into(),
                el::ka(b.fault_meters),
            ]);
        }
        out.push(Block::Table {
            columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
            rows,
        });
        if let Some(g) = &b.general {
            out.push(check(
                !g.passes(),
                format!(
                    "Línea general de alimentación: ΔU {} ≤ {}, {} A ≤ Iz {} A",
                    pct(g.drop),
                    pct(g.limit),
                    dec(g.current, 1),
                    dec(g.ampacity, 0)
                ),
            ));
        }
    }
    for (k, inst) in e.installations.iter().enumerate() {
        let board = inst
            .board
            .and_then(|b| model.device(b))
            .map_or("Cuadro (no dibujado)".to_string(), |d| d.name.clone());
        out.push(Block::Heading(
            2,
            format!("{} {board} · {}", k + 1, level_name(model, inst.level)),
        ));
        let mut rows = vec![
            vec![
                "Uso".into(),
                match inst.use_ {
                    el::BoardUse::Dwelling => format!(
                        "Vivienda, electrificación {}",
                        match inst.grade {
                            Electrification::High => "elevada",
                            _ => "básica",
                        }
                    ),
                    u => {
                        let n = u.name();
                        let mut c = n.chars();
                        c.next().map_or(String::new(), |f| {
                            f.to_uppercase().collect::<String>() + c.as_str()
                        })
                    }
                },
            ],
            vec![
                "Superficie servida".into(),
                format!("{} m²", dec(inst.area, 1)),
            ],
            vec![
                "Potencia prevista".into(),
                format!(
                    "{} W, {}",
                    dec(inst.power, 0),
                    if inst.three_phase {
                        "trifásica 3×400/230 V"
                    } else {
                        "monofásica 230 V"
                    }
                ),
            ],
        ];
        if inst.three_phase {
            rows.push(vec![
                "Fases".into(),
                format!(
                    "L1 {} W · L2 {} W · L3 {} W (desequilibrio {} %)",
                    dec(inst.phase_loads[0], 0),
                    dec(inst.phase_loads[1], 0),
                    dec(inst.phase_loads[2], 0),
                    dec(inst.unbalance * 100.0, 0)
                ),
            ]);
        }
        rows.push(vec![
            "Interruptor general (IGA)".into(),
            format!(
                "{} A {} · {}",
                inst.main_switch,
                inst.poles(),
                el::ka(inst.breaking_capacity)
            ),
        ]);
        rows.push(vec![
            "Derivación individual".into(),
            format!(
                "{} m{}, {} mm² Cu, tubo Ø{} mm, {} A, ΔU {} (≤ {})",
                dec(inst.feeder.length, 1),
                if inst.feeder.measured {
                    " desde los contadores"
                } else {
                    ""
                },
                inst.feeder.wires(),
                inst.feeder.tube,
                dec(inst.feeder.current, 1),
                pct(inst.feeder.drop),
                pct(inst.feeder.limit)
            ),
        ]);
        rows.push(vec![
            "Cortocircuito en el cuadro".into(),
            format!(
                "{} (poder de corte {})",
                el::ka(inst.fault),
                el::ka(inst.breaking_capacity)
            ),
        ]);
        rows.push(vec![
            "Diferenciales".into(),
            inst.differentials
                .iter()
                .map(|d| {
                    format!(
                        "{} A/{} mA {}P (C{})",
                        d.rating,
                        d.sensitivity,
                        d.poles,
                        d.circuits
                            .iter()
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join(", C")
                    )
                })
                .collect::<Vec<_>>()
                .join(" · "),
        ]);
        rows.push(vec![
            "Caída de tensión total".into(),
            format!(
                "{} desde la CGP hasta el punto más lejano",
                pct(inst.total_drop())
            ),
        ]);
        out.push(Block::Table {
            columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
            rows,
        });
        for r in &inst.reasons {
            out.push(Block::Para(r.clone()));
        }
        out.push(Block::Table {
            columns: vec![
                col("Circuito", 2.4),
                col("Fase", 0.6),
                col("Puntos", 0.8),
                col("P (W)", 0.9),
                col("I (A)", 0.8),
                col("PIA", 0.9),
                col("Sección", 0.9),
                col("L (m)", 0.8),
                col("ΔU", 0.8),
                col("Icc final", 1.0),
            ],
            rows: inst
                .circuits
                .iter()
                .map(|c| {
                    vec![
                        format!("C{} {}", c.spec.number, c.spec.name),
                        if c.phase.is_some() {
                            c.phase_text()
                        } else {
                            "L".into()
                        },
                        c.count.to_string(),
                        dec(c.power, 0),
                        dec(c.current, 1),
                        format!("{} A {}", c.breaker, site.curve.name()),
                        format!("{} mm²", dec(c.chosen, 1)),
                        dec(c.length, 1),
                        pct(c.drop),
                        el::ka(c.fault_end),
                    ]
                })
                .collect(),
        });
        if !inst.feeder.passes() {
            out.push(check(
                true,
                format!(
                    "Derivación individual: ΔU {} (≤ {}), {} A (Iz {} A)",
                    pct(inst.feeder.drop),
                    pct(inst.feeder.limit),
                    dec(inst.feeder.current, 1),
                    dec(inst.feeder.ampacity, 0)
                ),
            ));
        }
        if !inst.breaks() {
            out.push(check(
                true,
                format!(
                    "Poder de corte: {} en el cuadro con magnetotérmicos de {}",
                    el::ka(inst.fault),
                    el::ka(inst.breaking_capacity)
                ),
            ));
        }
        for c in &inst.circuits {
            let fails = !c.protected() || c.drop > el::CIRCUIT_DROP + 1e-9 || !c.trips();
            if fails {
                out.push(check(
                    true,
                    format!(
                        "C{}: Ib {} A, In {} A, Iz {} A, ΔU {}, Icc final {} (dispara con {})",
                        c.spec.number,
                        dec(c.current, 1),
                        c.breaker,
                        dec(c.ampacity, 0),
                        pct(c.drop),
                        el::ka(c.fault_end),
                        el::ka(c.trip / 1000.0)
                    ),
                ));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Water supply
// ---------------------------------------------------------------------

pub fn water(model: &Model, w: &Water) -> Vec<Block> {
    let mut out = vec![
        Block::Para(format!(
            "Red de agua fría desde el contador y de agua caliente sanitaria desde el acumulador, de {}. Caudales instantáneos de la tabla 2.1 del DB-HS 4 con simultaneidad {}; diámetros comerciales que dejan el agua por debajo de {} m/s y no bajan de las tablas 4.2 y 4.3; pérdidas de carga por Darcy–Weisbach más un {} % por accesorios.",
            w.material.name().to_lowercase(),
            if w.dwelling {
                "de la UNE 149201 (viviendas)"
            } else {
                "Qt/√(n − 1)"
            },
            dec(wa::DESIGN_SPEED, 1),
            dec(wa::FITTINGS * 100.0, 0)
        )),
        Block::Table {
            columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
            rows: vec![
                vec![
                    "Presión en el contador".into(),
                    format!("{} kPa", dec(w.pressure, 0)),
                ],
                vec![
                    "Pérdidas en contador y filtro".into(),
                    format!("{} kPa", dec(w.meter_loss, 0)),
                ],
                vec![
                    "Pérdidas en el calentador".into(),
                    format!("{} kPa", dec(w.heater_loss, 0)),
                ],
                vec![
                    "Presión mínima en grifos".into(),
                    format!(
                        "{} kPa (calentadores {} kPa)",
                        dec(wa::TAP_PRESSURE, 0),
                        dec(wa::HEATER_PRESSURE, 0)
                    ),
                ],
            ],
        },
    ];
    for n in w.networks.iter().filter(|n| !n.stretches.is_empty()) {
        let hot = n.system == ServiceSystem::HotWater;
        out.push(Block::Heading(
            2,
            format!(
                "{} {}",
                if hot { 2 } else { 1 },
                if hot {
                    "Agua caliente sanitaria"
                } else {
                    "Agua fría"
                }
            ),
        ));
        out.push(Block::Table {
            columns: vec![
                col("Tramo", 0.9),
                col("Planta", 1.6),
                col("L (m)", 0.8),
                col("Aparatos", 0.9),
                col("Qc (l/s)", 0.9),
                col("Dibujado", 0.9),
                col("Calculado", 0.9),
                col("v (m/s)", 0.8),
                col("Pérdida (kPa)", 1.0),
            ],
            rows: n
                .stretches
                .iter()
                .map(|s| {
                    vec![
                        s.name.clone(),
                        level_name(model, s.level),
                        dec(s.length, 1),
                        s.appliances.to_string(),
                        dec(s.flow, 2),
                        mm(s.drawn.0),
                        mm(s.sized.0),
                        dec(s.sized_speed, 2),
                        dec(s.sized_loss, 1),
                    ]
                })
                .collect(),
        });
        let worst = n
            .taps
            .iter()
            .min_by(|a, b| a.sized_pressure.total_cmp(&b.sized_pressure));
        if let Some(t) = worst {
            out.push(check(
                t.sized_pressure + 1e-9 < wa::TAP_PRESSURE,
                format!(
                    "Grifo más desfavorable ({}, {}): {} kPa con los diámetros calculados; {} kPa con los dibujados",
                    t.kind.name(),
                    level_name(model, t.level),
                    dec(t.sized_pressure, 0),
                    dec(t.pressure, 0)
                ),
            ));
        }
        let short = n
            .stretches
            .iter()
            .filter(|s| s.drawn.0 + 1e-4 < s.sized.0)
            .count();
        if short > 0 {
            out.push(check(
                true,
                format!("{short} tramos dibujados con un diámetro menor que el calculado"),
            ));
        }
    }
    if let Some(g) = &w.group {
        out.push(check(
            g.needed && g.drawn.is_none(),
            format!(
                "Grupo de presión{}: {} l/s, arranque {} kPa, parada {} kPa, {} bombas (HS 4, 4.5.2)",
                if g.drawn.is_some() { "" } else { " (a poner)" },
                dec(g.flow, 2),
                dec(g.min, 0),
                dec(g.max, 0),
                g.pumps
            ),
        ));
    }
    if let Some(t) = &w.tank {
        out.push(Block::Para(format!(
            "Aljibe de {} l (15 minutos del caudal simultáneo, HS 4, 4.5.1){}.",
            dec(t.volume, 0),
            if t.drawn.is_some() {
                ""
            } else {
                ", sin dibujar"
            }
        )));
    }
    if let Some(r) = &w.hot_return {
        out.push(Block::Para(format!(
            "Red de retorno de ACS, {}: {} m de tubería al grifo más lejano; {} l/h por {} columna{}, tubería {} aislada (HS 4, 2.3 y tabla 4.4).",
            r.drawn_text(),
            dec(r.length, 1),
            dec(r.flow, 0),
            r.columns,
            if r.columns == 1 { "" } else { "s" },
            r.pipe.0
        )));
    }
    out
}

// ---------------------------------------------------------------------
// Drainage
// ---------------------------------------------------------------------

pub fn drainage(model: &Model, d: &Drainage) -> Vec<Block> {
    let mut out = vec![Block::Para(format!(
        "Red separativa de aguas residuales y pluviales en PVC. Unidades de desagüe de uso {} (tabla 4.1); ramales por la tabla 4.3, bajantes por las tablas 4.4 y 4.8, colectores por las 4.5 y 4.9; intensidad de lluvia {} mm/h.",
        if d.public { "público" } else { "privado" },
        dec(d.intensity, 0)
    ))];
    let mut k = 0;
    for n in d.networks.iter().filter(|n| !n.stretches.is_empty()) {
        k += 1;
        let rain = n.system == ServiceSystem::Rainwater;
        out.push(Block::Heading(
            2,
            format!(
                "{k} {} · {}",
                if rain {
                    "Aguas pluviales"
                } else {
                    "Aguas residuales"
                },
                dr::outlet_name(model, n)
            ),
        ));
        out.push(Block::Table {
            columns: vec![
                col("Tramo", 0.8),
                col("Tipo", 1.2),
                col("Planta", 1.5),
                col(if rain { "m²" } else { "UD" }, 0.8),
                col("Pendiente", 0.9),
                col("Dibujado", 0.9),
                col("Exigido", 0.9),
                col("Tabla", 1.0),
            ],
            rows: n
                .stretches
                .iter()
                .map(|s| {
                    vec![
                        s.name.clone(),
                        s.kind.name().to_string(),
                        level_name(model, s.level),
                        if rain {
                            dec(s.area, 1)
                        } else {
                            s.units.to_string()
                        },
                        if s.slope > 0.0 {
                            format!("{} %", dec(s.slope * 100.0, 1))
                        } else {
                            "—".into()
                        },
                        mm(s.drawn),
                        mm(s.sized),
                        s.rule.trim_start_matches("HS 5, ").to_string(),
                    ]
                })
                .collect(),
        });
        let short = n.stretches.iter().filter(|s| !s.fits).count();
        out.push(check(
            short > 0,
            if short == 0 {
                "Todos los tramos con el diámetro exigido".into()
            } else {
                format!("{short} tramos dibujados con un diámetro menor que el exigido")
            },
        ));
    }
    for s in d.surfaces.iter().filter(|s| s.checked) {
        out.push(check(
            s.outlets < s.needed,
            format!(
                "{}: {} m², {} sumideros (≥ {}, tabla 4.6)",
                s.name,
                dec(s.area, 1),
                s.outlets,
                s.needed
            ),
        ));
    }
    for g in &d.gutters {
        out.push(check(
            g.gutter + 1e-4 < g.gutter_sized || g.downpipe + 1e-4 < g.downpipe_sized,
            format!(
                "Canalón de {} (lado {}): {} m², canalón {} (≥ {}), bajante {} (≥ {})",
                g.roof,
                g.side + 1,
                dec(g.area, 1),
                mm(g.gutter),
                mm(g.gutter_sized),
                mm(g.downpipe),
                mm(g.downpipe_sized)
            ),
        ));
    }
    if !d.loose.is_empty() {
        out.push(check(
            true,
            format!("{} aparatos sin desagüe a menos de 45 cm", d.loose.len()),
        ));
    }
    out.extend(drain_venting(model, d));
    out
}

/// The ventilation of the waste network and its small evacuation (HS 5
/// 3.3.1 and 3.3.3).
fn drain_venting(model: &Model, d: &Drainage) -> Vec<Block> {
    use archisim_mep::drain_vent as dv;
    let v = dv::venting(model, d);
    let mut out = Vec::new();
    if v.columns.is_empty() && v.ways.is_empty() {
        return out;
    }
    out.push(Block::Heading(2, "Ventilación y pequeña evacuación".into()));
    out.push(Block::Para(format!(
        "Ventilación primaria por la prolongación de cada bajante sobre la cubierta ({} m si no es transitable, {} m si lo es), a más de {} m de las tomas de aire o por encima de ellas y {} m sobre las ventanas cercanas, o con una válvula de aireación en su cabeza. Secundaria desde {} plantas ({} con la bajante sobredimensionada) y terciaria en los ramales de más de {} m o por encima de {} plantas (3.3.3). Cierres hidráulicos de {} a {} mm; botes sifónicos solo con aparatos de su local y sin inodoros, a {} m como mucho de la bajante y con derivaciones de hasta {} m (3.3.1).",
        dec(dv::ABOVE_ROOF, 2),
        dec(dv::ABOVE_WALKED_ROOF, 2),
        dec(dv::INTAKE_GAP, 0),
        dec(dv::OVER_WINDOWS, 2),
        dv::PRIMARY_FLOORS,
        dv::PRIMARY_FLOORS_OVERSIZED,
        dec(dv::LONG_BRANCH, 0),
        dv::TERTIARY_FLOORS,
        dec(dv::SEAL_MIN * 1000.0, 0),
        dec(dv::SEAL_MAX * 1000.0, 0),
        dec(dv::BOX_TO_DOWNPIPE, 2),
        dec(dv::BOX_BRANCH, 2),
    )));
    if !v.columns.is_empty() {
        out.push(Block::Table {
            columns: vec![
                col("Bajante", 2.0),
                col("Plantas", 0.7),
                col("Sobre cubierta", 1.0),
                col("Exigido", 0.8),
                col("Secundaria", 1.4),
                col("Terciaria", 1.2),
            ],
            rows: v
                .columns
                .iter()
                .map(|c| {
                    let tertiary = c.branches.iter().filter(|b| b.needed).count();
                    vec![
                        dv::column_name(model, c),
                        c.floors.to_string(),
                        if c.valve.is_some() {
                            "válvula".into()
                        } else {
                            c.above().map_or("—".into(), |a| format!("{} m", dec(a, 2)))
                        },
                        if c.valve.is_some() {
                            "—".into()
                        } else {
                            match (c.required_top(), &c.cover) {
                                (Some(z), Some(cv)) => format!("{} m", dec(z - cv.z, 2)),
                                _ => "—".into(),
                            }
                        },
                        if !c.secondary_needed() {
                            "no hace falta".into()
                        } else if c.valves_on_branches() {
                            "válvulas".into()
                        } else {
                            match c.vent {
                                Some((_, w)) => format!("{} (≥ {})", mm(w), mm(c.vent_sized)),
                                None => "falta".into(),
                            }
                        },
                        if tertiary == 0 {
                            "no hace falta".into()
                        } else {
                            format!(
                                "{} de {} ramales",
                                tertiary - c.branches.iter().filter(|b| b.fails()).count(),
                                tertiary
                            )
                        },
                    ]
                })
                .collect(),
        });
        for c in &v.columns {
            out.push(check(
                c.primary_fails(),
                format!(
                    "{}: ventilación primaria, {}",
                    dv::column_name(model, c),
                    dv::primary_text(c)
                ),
            ));
            if c.secondary_needed() {
                out.push(check(
                    c.secondary_fails(),
                    format!(
                        "{}: ventilación secundaria para {} plantas",
                        dv::column_name(model, c),
                        c.floors
                    ),
                ));
            }
            let bad = c.branches.iter().filter(|b| b.fails()).count();
            if c.branches.iter().any(|b| b.needed) {
                out.push(check(
                    bad > 0,
                    format!(
                        "{}: ventilación terciaria de los ramales largos{}",
                        dv::column_name(model, c),
                        if bad > 0 {
                            format!(" ({bad} sin ventilar)")
                        } else {
                            String::new()
                        }
                    ),
                ));
            }
        }
    }
    out.push(check(
        v.seal_fails(),
        format!(
            "Altura del cierre hidráulico: {} mm (de {} a {})",
            dec(v.seal * 1000.0, 0),
            dec(dv::SEAL_MIN * 1000.0, 0),
            dec(dv::SEAL_MAX * 1000.0, 0)
        ),
    ));
    for b in &v.trap_boxes {
        out.push(check(
            b.fails(),
            format!(
                "Bote sifónico de {}: {} aparatos{}{}",
                level_name(model, b.level),
                b.appliances.len(),
                b.to_downpipe.map_or(String::new(), |x| format!(
                    ", a {} m de la bajante",
                    dec(x, 2)
                )),
                if b.has_wc() { ", con un inodoro" } else { "" }
            ),
        ));
    }
    let far = v.ways.iter().filter(|w| w.too_far()).count();
    let slope = v.ways.iter().filter(|w| w.bad_slope()).count();
    out.push(check(
        far > 0,
        format!(
            "Distancia de los aparatos a la bajante (3.3.1.4): {} de {} dentro del límite",
            v.ways.len() - far,
            v.ways.len()
        ),
    ));
    out.push(check(
        slope > 0,
        format!(
            "Pendiente de las derivaciones de los aparatos (3.3.1.4): {} de {} dentro del límite",
            v.ways.len() - slope,
            v.ways.len()
        ),
    ));
    out
}

// ---------------------------------------------------------------------
// Ventilation
// ---------------------------------------------------------------------

pub fn ventilation(model: &Model, v: &Ventilation) -> Vec<Block> {
    let site = model.ventilation_site();
    let ducts = match site.system {
        archisim_core::VentSystem::Mechanical => format!(
            "conductos de 4.2.2 ({} cm² por l/s junto a los locales, {} en la cubierta)",
            dec(vt::DUCT_PER_LS, 1),
            dec(vt::ROOF_DUCT_PER_LS, 1)
        ),
        archisim_core::VentSystem::Hybrid => format!(
            "conductos de la tabla 4.2 por el tiro de la tabla 4.3, en la zona {} de la tabla 4.4",
            site.zone.code()
        ),
    };
    let dwellings = !v.dwellings.is_empty();
    let mut out = Vec::new();
    if dwellings {
        out.push(Block::Para(format!(
        "Ventilación {}: el aire entra por los aireadores de los locales secos, pasa bajo las puertas y sale por las bocas de extracción de los húmedos, por los conductos hasta {}. Caudales de la tabla 2.1, aberturas de la tabla 4.1 ({} cm² por l/s) y {}. Cada cocina lleva además la extracción de su campana, {} l/s por un conducto propio (3.1.1). Cada conducto sale a {} m al menos de las ventanas, puertas exteriores, aireadores y terrazas (3.2.1).",
        site.system.name().to_lowercase(),
        if site.system == archisim_core::VentSystem::Hybrid {
            "el aspirador híbrido de cubierta"
        } else {
            "el extractor de cubierta"
        },
        dec(vt::OPENING_PER_LS, 0),
        ducts,
        dec(vt::HOOD_FLOW, 0),
        dec(vt::OUTLET_GAP, 0)
    )));
        out.push(Block::Heading(2, "1 Caudales de las viviendas".into()));
        out.push(Block::Table {
            columns: vec![
                col("Vivienda", 2.6),
                col("Dormitorios", 1.0),
                col("Secos (l/s)", 1.0),
                col("Húmedos (l/s)", 1.0),
                col("Caudal (l/s)", 1.0),
                col("No cumple", 0.9),
            ],
            rows: v
                .dwellings
                .iter()
                .map(|d| {
                    vec![
                        d.name.clone(),
                        d.bedrooms.to_string(),
                        dec(d.dry_flow, 1),
                        dec(d.wet_flow, 1),
                        dec(d.flow, 1),
                        d.fails().to_string(),
                    ]
                })
                .collect(),
        });
        out.push(Block::Heading(2, "2 Aberturas de los locales".into()));
        let mut rows = Vec::new();
        for d in &v.dwellings {
            for r in &d.rooms {
                if let Some(dry) = r.dry {
                    rows.push(vec![
                        d.name.clone(),
                        r.name.clone(),
                        dry.name().to_string(),
                        format!("{} entra", dec(r.admission, 1)),
                        format!("{} cm²", dec(r.inlet_area, 0)),
                        format!("{} cm²", dec(r.inlet_needed(), 0)),
                    ]);
                }
                if r.wet {
                    rows.push(vec![
                        d.name.clone(),
                        r.name.clone(),
                        "húmedo".into(),
                        format!("{} sale", dec(r.extraction, 1)),
                        format!("{} cm²", dec(r.vent_area, 0)),
                        format!("{} cm²", dec(r.vent_needed(), 0)),
                    ]);
                }
            }
        }
        out.push(Block::Table {
            columns: vec![
                col("Vivienda", 2.0),
                col("Local", 1.6),
                col("Clase", 1.4),
                col("l/s", 0.9),
                col("Abertura", 0.9),
                col("Exigida", 0.9),
            ],
            rows,
        });
        let passages: Vec<String> = v
            .dwellings
            .iter()
            .flat_map(|d| d.passages.iter().map(move |p| (d, p)))
            .filter(|(_, p)| p.short())
            .map(|(d, p)| {
                let name = |r| model.room(r).map_or("—".to_string(), |x| x.name.clone());
                format!(
                    "{}: paso de {} a {}, {} l/s{}",
                    d.name,
                    name(p.from),
                    name(p.to),
                    dec(p.flow, 1),
                    p.gap_needed()
                        .map_or(String::new(), |g| format!(" — holgura de {g} mm o rejilla"))
                )
            })
            .collect();
        out.push(check(
            !passages.is_empty(),
            if passages.is_empty() {
                "Pasos de aire bajo las puertas suficientes (tabla 4.1)".into()
            } else {
                format!(
                    "{} pasos de puerta cortos (tabla 4.1): {}",
                    passages.len(),
                    passages.join("; ")
                )
            },
        ));
        out.push(Block::Heading(2, "3 Conductos de extracción".into()));
        out.push(Block::Table {
            columns: vec![
                col("Tramo", 0.8),
                col("Planta", 1.5),
                col("l/s", 0.8),
                col("Dibujado", 1.2),
                col("cm²", 0.8),
                col("Exigido", 0.8),
                col("Basta", 1.2),
            ],
            rows: v
                .networks
                .iter()
                .filter(|n| !n.rite && !n.supply)
                .flat_map(|n| &n.stretches)
                .map(|s| {
                    vec![
                        s.name.clone(),
                        level_name(model, s.level),
                        dec(s.flow, 1),
                        s.size.clone(),
                        dec(s.area, 0),
                        dec(s.needed, 0),
                        s.sized_text.clone(),
                    ]
                })
                .collect(),
        });
        for (i, n) in v.networks.iter().enumerate() {
            if n.is_hood_duct() || n.rite || n.supply {
                continue;
            }
            out.push(check(
                n.fan.is_none() || n.shared(),
                format!(
                    "Red {}: {} bocas, {} m³/h; extractor {}{}",
                    i + 1,
                    n.vents.len(),
                    dec(n.flow * 3.6, 0),
                    if n.fan.is_some() {
                        "dibujado"
                    } else {
                        "sin dibujar"
                    },
                    if n.shared() {
                        "; lleva campanas, que van por conducto propio"
                    } else {
                        ""
                    }
                ),
            ));
        }
        let outlets: Vec<(usize, &vt::Outlet)> = v
            .networks
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.outlet.as_ref().map(|o| (i, o)))
            .collect();
        let bad: Vec<String> = outlets
            .iter()
            .filter(|(_, o)| o.fails())
            .map(|(i, o)| format!("red {}: {}", i + 1, vt::outlet_text(o)))
            .collect();
        if !outlets.is_empty() {
            out.push(check(
            !bad.is_empty(),
            if bad.is_empty() {
                format!(
                    "Salidas de los conductos: las {} a {} m al menos de las entradas de aire (3.2.1)",
                    outlets.len(),
                    dec(vt::OUTLET_GAP, 0)
                )
            } else {
                format!("Salidas de los conductos (3.2.1): {}", bad.join("; "))
            },
        ));
        }
        let hoods_ok = v.hoods.iter().filter(|h| !v.hood_fails(h)).count();
        let bad: Vec<String> = v
            .hoods
            .iter()
            .filter(|h| v.hood_fails(h))
            .map(|h| format!("{}: {}", h.dwelling, vt::hood_text(v, h)))
            .collect();
        out.push(check(
            !bad.is_empty(),
            if bad.is_empty() {
                format!("Campanas de cocina: {hoods_ok} con conducto propio (3.1.1)")
            } else {
                bad.join("; ")
            },
        ));
    }
    if !v.others.is_empty() {
        out.push(Block::Heading(
            2,
            format!(
                "{} Aparcamientos, trasteros y almacenes de residuos",
                if dwellings { 4 } else { 1 }
            ),
        ));
        out.push(Block::Para(format!(
            "Caudales de la tabla 2.2 (aparcamientos {} l/s por plaza). Con ventilación natural (3.1.4.1), aberturas mixtas en dos zonas opuestas de la fachada de {} cm² por l/s cada una, ningún punto a más de {} m de una abertura y otra a media distancia si las opuestas están a más de {} m; con mecánica (3.1.4.2), una boca de extracción cada 100 m², a menos de 10 m unas de otras.",
            dec(vt::GARAGE_PER_SPACE, 0),
            dec(archisim_mep::garage::MIXED_PER_LS, 0),
            dec(archisim_mep::garage::REACH, 0),
            dec(archisim_mep::garage::OPPOSITE_GAP, 0)
        )));
        out.push(Block::Table {
            columns: vec![
                col("Local", 2.2),
                col("Clase", 1.6),
                col("m²", 0.8),
                col("Plazas", 0.8),
                col("l/s", 0.8),
                col("Ventilación", 1.0),
            ],
            rows: v
                .others
                .iter()
                .map(|o| {
                    vec![
                        o.name.clone(),
                        o.what.to_string(),
                        dec(o.area, 1),
                        if o.garage {
                            o.spaces.to_string()
                        } else {
                            "—".into()
                        },
                        dec(o.flow, 1),
                        if !o.garage {
                            "—".into()
                        } else if o.natural.is_some() {
                            "natural".into()
                        } else if o.mechanical {
                            "mecánica".into()
                        } else {
                            "sin dibujar".into()
                        },
                    ]
                })
                .collect(),
        });
        for o in &v.others {
            let lines = vt::other_lines(o);
            let status = if o.vent_short() || o.garage_fails() > 0 {
                Status::Fail
            } else if o.natural.is_some() {
                Status::Pass
            } else if !o.drawn() {
                if o.garage {
                    Status::Warn
                } else {
                    Status::Info
                }
            } else {
                Status::Pass
            };
            out.push(Block::Check {
                status,
                text: format!("{}: {}", o.name, lines.join("; ")),
            });
        }
    }
    out
}

/// The rooms ventilated by the RITE (IT 1.1.4.2).
pub fn rite(model: &Model, v: &Ventilation) -> Vec<Block> {
    use archisim_mep::rite as rt;
    let r = &v.rite;
    let mut out = vec![Block::Para(format!(
        "Los locales que no son de vivienda se ventilan por el RITE (IT 1.1.4.2). Cada uno tiene su categoría de calidad del aire interior (IDA) por su uso. El aire exterior se da por persona (tabla 1.4.2.1: {} l/s en IDA 1 a 4), con la ocupación del DB-SI 3, o por superficie en los locales sin ocupación permanente (tabla 1.4.2.4); los locales de servicio, aseos y vestuarios, extraen {} l/s por m² (IT 1.1.4.2.5). Los conductos se dimensionan por la velocidad del aire: {} m/s en los principales y {} m/s en los ramales a un solo terminal.",
        rt::PER_PERSON.iter().map(|x| dec(*x, 1)).collect::<Vec<_>>().join(" / "),
        dec(rt::SERVICE_PER_M2, 0),
        dec(model.ventilation_site().duct_speed, 1),
        dec(model.ventilation_site().duct_speed * rt::BRANCH_SPEED, 1),
    ))];
    out.push(Block::Heading(
        2,
        "1 Aire exterior y extracción de los locales".into(),
    ));
    out.push(Block::Table {
        columns: vec![
            col("Local", 2.0),
            col("Planta", 1.2),
            col("IDA", 0.5),
            col("Método", 1.6),
            col("Personas", 0.8),
            col("m²", 0.7),
            col("Exterior (l/s)", 0.9),
            col("Extracción (l/s)", 0.9),
        ],
        rows: r
            .rooms
            .iter()
            .map(|x| {
                vec![
                    x.name.clone(),
                    level_name(model, x.level),
                    format!("{}{}", x.ida, if x.by_hand { " (a mano)" } else { "" }),
                    match x.method {
                        rt::Method::Person => "por persona",
                        rt::Method::Area => "por superficie",
                        rt::Method::Service => "local de servicio",
                        rt::Method::Pool => "piscina",
                    }
                    .into(),
                    if x.people > 0 {
                        x.people.to_string()
                    } else {
                        "—".into()
                    },
                    dec(x.area, 1),
                    dec(x.outdoor, 1),
                    dec(x.extraction, 1),
                ]
            })
            .collect(),
    });
    for x in r.rooms.iter().filter(|x| x.no_supply() || x.no_vent()) {
        out.push(Block::Check {
            status: Status::Warn,
            text: rt::room_text(x),
        });
    }
    let stretches: Vec<(&vt::Network, &vt::Stretch)> = v
        .networks
        .iter()
        .filter(|n| n.rite || n.supply)
        .flat_map(|n| n.stretches.iter().map(move |s| (n, s)))
        .collect();
    if !stretches.is_empty() {
        out.push(Block::Heading(
            2,
            "2 Conductos de impulsión y extracción".into(),
        ));
        out.push(Block::Table {
            columns: vec![
                col("Tramo", 0.7),
                col("Red", 1.0),
                col("l/s", 0.8),
                col("Dibujado", 1.1),
                col("m/s", 0.7),
                col("Exigido (cm²)", 1.0),
                col("Basta", 1.1),
            ],
            rows: stretches
                .iter()
                .map(|(n, s)| {
                    vec![
                        s.name.clone(),
                        if n.supply {
                            "impulsión"
                        } else {
                            "extracción"
                        }
                        .into(),
                        dec(s.flow, 1),
                        s.size.clone(),
                        s.drawn_speed().map_or("—".into(), |x| dec(x, 1)),
                        dec(s.needed, 0),
                        s.sized_text.clone(),
                    ]
                })
                .collect(),
        });
        let short: Vec<String> = stretches
            .iter()
            .filter(|(_, s)| s.short())
            .map(|(_, s)| format!("{} ({} → {})", s.name, s.size, s.sized_text))
            .collect();
        out.push(check(
            !short.is_empty(),
            if short.is_empty() {
                "Conductos suficientes para la velocidad de proyecto".into()
            } else {
                format!("Conductos cortos: {}", short.join("; "))
            },
        ));
    }
    out.push(Block::Heading(
        2,
        "3 Filtros y recuperación de calor".into(),
    ));
    let f: Vec<String> = r
        .filters()
        .into_iter()
        .map(|(i, f)| format!("IDA {i}: {f}"))
        .collect();
    out.push(Block::Para(format!(
        "Aire exterior ODA {}; filtros mínimos de la tabla 1.4.2.5: {}.",
        r.oda,
        if f.is_empty() {
            "—".into()
        } else {
            f.join("; ")
        }
    )));
    out.push(Block::Check {
        status: match (r.recovery_needed, r.recovery) {
            (false, _) => Status::Info,
            (true, Some(_)) => Status::Pass,
            (true, None) => Status::Warn,
        },
        text: rt::recovery_text(r),
    });
    out
}

// ---------------------------------------------------------------------
// Envelope and loads
// ---------------------------------------------------------------------

pub fn thermal(model: &Model, t: &Thermal) -> Vec<Block> {
    let zone = t.site.winter.index();
    // Sections numbered in order (the ground's only when there is one).
    let mut n = 0;
    let mut heading = |text: &str| {
        n += 1;
        Block::Heading(2, format!("{n} {text}"))
    };
    let mut out = vec![
        Block::Para(
            "Transmitancias U = 1/(Rsi + Σ e/λ + Rse) con las capas de cada tipo; los suelos y muros contra el terreno con la U equivalente de la UNE-EN ISO 13370 (dimensión característica B' = A/(0,5·P), espesor equivalente y aislamiento perimetral). Puentes térmicos lineales medidos en el dibujo (frentes de forjado, pilares en fachada, contorno de huecos, esquinas y encuentros con la cubierta y la solera), con un ψ por clase de encuentro según dónde está el aislamiento del muro. Límites de la tabla 3.1.1.a-HE1 por la zona de invierno y coeficiente global K = (Σ U·A + Σ ψ·L) / Σ A de los locales calefactados contra la tabla 3.1.1.b-HE1 por la compacidad. Control solar de julio (3.1.2), permeabilidad al aire de los huecos y n50 por el anejo H (3.1.3) y condensaciones superficiales (fRsi). Cargas de calefacción por transmisión, puentes térmicos y ventilación; de refrigeración, con el sol por los vidrios, las personas, la luz y los aparatos.".into(),
        ),
        Block::Table {
            columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
            rows: vec![
                vec!["Zona climática".into(), t.site.zone()],
                vec![
                    "Temperaturas exteriores".into(),
                    format!(
                        "{} °C en invierno, {} °C en verano",
                        dec(t.site.winter_air, 0),
                        dec(t.site.summer_air, 0)
                    ),
                ],
                vec![
                    "Temperaturas interiores".into(),
                    format!(
                        "{} °C calefactado, {} °C refrigerado",
                        dec(t.site.heated, 0),
                        dec(t.site.cooled, 0)
                    ),
                ],
                vec![
                    "Huecos".into(),
                    format!(
                        "U ventanas {} W/m²K, puertas {} W/m²K, factor solar {} ({} con la protección móvil), marco {} %, clase de permeabilidad {}",
                        dec(t.site.window_u, 2),
                        dec(t.site.door_u, 2),
                        dec(t.site.glass_g, 2),
                        dec(t.site.shaded_g, 2),
                        dec(t.site.frame_fraction * 100.0, 0),
                        t.site.air_class
                    ),
                ],
                vec![
                    "Terreno".into(),
                    format!(
                        "λ = {} W/mK{}",
                        dec(t.site.ground_conductivity, 1),
                        t.site.edge.map_or(String::new(), |e| format!(
                            "; aislamiento perimetral {} de {} m, R = {} m²K/W",
                            if e.vertical { "vertical" } else { "horizontal" },
                            dec(e.size, 2),
                            dec(e.resistance, 2)
                        ))
                    ),
                ],
                vec![
                    "Compacidad V/A".into(),
                    format!(
                        "{} m³ / {} m² = {} m",
                        dec(t.volume, 0),
                        dec(t.envelope, 0),
                        dec(t.compactness, 2)
                    ),
                ],
            ],
        },
        heading("Elementos de la envolvente"),
        Block::Table {
            columns: vec![
                col("Elemento", 1.2),
                col("Tipo", 2.8),
                col("Superficie", 1.0),
                col("U", 0.8),
                col("Límite", 0.8),
            ],
            rows: t
                .elements
                .iter()
                .map(|e| {
                    vec![
                        e.kind.name().to_string(),
                        e.type_name.clone(),
                        format!("{} m²", dec(e.area, 1)),
                        dec(e.u, 2),
                        dec(e.limit, 2),
                    ]
                })
                .collect(),
        },
    ];
    for e in t.elements.iter().filter(|e| e.u > e.limit + 1e-9) {
        out.push(check(
            true,
            format!(
                "{} «{}»: U {} W/m²K, límite {}",
                e.kind.name(),
                e.type_name,
                dec(e.u, 2),
                dec(e.limit, 2)
            ),
        ));
    }
    if !t.ground.is_empty() {
        out.push(heading("Suelos sobre el terreno (UNE-EN ISO 13370)"));
        out.push(Block::Table {
            columns: vec![
                col("Planta", 1.6),
                col("A (m²)", 0.8),
                col("P (m)", 0.8),
                col("B' (m)", 0.8),
                col("dt (m)", 0.8),
                col("z (m)", 0.7),
                col("U capas", 0.8),
                col("U equivalente", 1.0),
            ],
            rows: t
                .ground
                .iter()
                .map(|(l, g)| {
                    vec![
                        level_name(model, *l),
                        dec(g.area, 1),
                        dec(g.perimeter, 1),
                        dec(g.b, 2),
                        dec(g.dt, 2),
                        dec(g.depth, 2),
                        dec(g.u_layers, 2),
                        dec(g.u, 2),
                    ]
                })
                .collect(),
        });
    }
    out.push(heading("Puentes térmicos"));
    if t.bridges.is_empty() {
        out.push(Block::Para(
            "No hay encuentros de fachada en la envolvente.".into(),
        ));
    } else {
        out.push(Block::Table {
            columns: vec![
                col("Encuentro", 1.8),
                col("Aislamiento", 1.5),
                col("Longitud (m)", 0.9),
                col("ψ (W/mK)", 0.8),
                col("ψ·L (W/K)", 0.8),
                col("fRsi", 0.6),
            ],
            rows: t
                .bridges
                .iter()
                .map(|b| {
                    vec![
                        b.junction.name().to_string(),
                        b.insulation.name().to_string(),
                        dec(b.length, 1),
                        dec(b.psi, 2),
                        dec(b.h, 1),
                        dec(b.f_rsi, 2),
                    ]
                })
                .collect(),
        });
    }
    out.push(check(
        t.k_fails(),
        format!(
            "Coeficiente global K = ({} + {}) / {} = {} W/m²K (≤ {}, tabla 3.1.1.b-HE1)",
            dec(t.ua, 1),
            dec(t.psi_l, 1),
            dec(t.envelope, 0),
            dec(t.k, 2),
            dec(t.k_limit, 2)
        ),
    ));
    out.push(heading("Control solar (3.1.2)"));
    let sol = &t.solar;
    if !sol.by_facing.is_empty() {
        out.push(Block::Table {
            columns: vec![
                col("Orientación", 1.0),
                col("Vidrio (m²)", 1.0),
                col("H julio (kWh/m²)", 1.2),
                col("Sombra media", 1.0),
                col("Ganancia (kWh)", 1.0),
            ],
            rows: sol
                .by_facing
                .iter()
                .map(|(f, a, sh, g)| {
                    vec![
                        f.name().to_string(),
                        dec(*a, 1),
                        dec(archisim_mep::envelope::july_radiation(t.site.summer, *f), 0),
                        dec(*sh, 2),
                        dec(*g, 0),
                    ]
                })
                .collect(),
        });
    }
    out.push(check(
        sol.fails(),
        format!(
            "q sol;jul = {} kWh/m²·mes sobre {} m² útiles (≤ {}, {})",
            dec(sol.q, 2),
            dec(sol.useful, 1),
            dec(sol.limit, 2),
            if sol.residential {
                "residencial privado"
            } else {
                "otros usos"
            }
        ),
    ));
    out.push(heading("Permeabilidad al aire (3.1.3)"));
    if !t.air.windows.is_empty() {
        out.push(Block::Table {
            columns: vec![
                col("Ventana", 2.6),
                col("m²", 0.6),
                col("Clase", 0.6),
                col("Q100 (m³/h·m²)", 1.1),
                col("Límite", 0.7),
            ],
            rows: t
                .air
                .windows
                .iter()
                .map(|w| {
                    vec![
                        w.type_name.clone(),
                        dec(w.area, 1),
                        w.class.to_string(),
                        dec(w.q100, 0),
                        dec(w.limit, 0),
                    ]
                })
                .collect(),
        });
        for w in t.air.windows.iter().filter(|w| w.fails()) {
            out.push(check(
                true,
                format!(
                    "«{}»: clase {} ({} m³/h·m²), límite {} en la zona {}",
                    w.type_name,
                    w.class,
                    dec(w.q100, 0),
                    dec(w.limit, 0),
                    t.site.winter.letter()
                ),
            ));
        }
    }
    let n50_text = format!(
        "n50 = {} h⁻¹ ({})",
        dec(t.air.n50, 2),
        if t.air.measured {
            "medida".to_string()
        } else {
            format!(
                "anejo H: 0,629·({} m³/h·m² × {} m² opacos + huecos por su clase) / {} m³",
                dec(t.air.opaque_air, 0),
                dec(t.air.opaque_area, 0),
                dec(t.volume, 0)
            )
        }
    );
    if t.air.applies {
        out.push(check(
            t.air.n50_fails(),
            format!("{n50_text}; límite {} (tabla 3.1.3.b)", dec(t.air.limit, 2)),
        ));
    } else {
        out.push(Block::Check {
            status: Status::Info,
            text: format!(
                "{n50_text}. La tabla 3.1.3.b se aplica a las viviendas de más de 120 m² útiles."
            ),
        });
    }
    out.push(heading("Condensaciones superficiales"));
    out.push(Block::Para(format!(
        "fRsi = 1 − 0,25·U de cada tipo opaco y el de cada clase de puente térmico, contra fRsi,min = {} (higrometría {}, zona {}).",
        dec(archisim_mep::envelope::f_rsi_min(zone, t.site.hygrometry), 2),
        t.site.hygrometry,
        t.site.winter.letter()
    )));
    out.push(Block::Table {
        columns: vec![col("Elemento", 1.4), col("Tipo", 2.8), col("fRsi", 0.7)],
        rows: t
            .condensation
            .iter()
            .map(|c| {
                vec![
                    c.kind.name().to_string(),
                    c.type_name.clone(),
                    dec(c.f_rsi, 2),
                ]
            })
            .collect(),
    });
    for c in t.condensation.iter().filter(|c| c.fails()) {
        out.push(check(
            true,
            format!(
                "{} «{}»: fRsi {} < {}",
                c.kind.name(),
                c.type_name,
                dec(c.f_rsi, 2),
                dec(c.f_min, 2)
            ),
        ));
    }
    for b in t.bridges.iter().filter(|b| b.condenses()) {
        out.push(check(
            true,
            format!(
                "{} ({}): fRsi {} < {}",
                b.junction.name(),
                b.insulation.name(),
                dec(b.f_rsi, 2),
                dec(b.f_min, 2)
            ),
        ));
    }
    if !t.condensation.iter().any(|c| c.fails()) && !t.bridges.iter().any(|b| b.condenses()) {
        out.push(check(false, "Sin condensaciones superficiales".into()));
    }
    out.push(heading("Cargas térmicas"));
    out.push(Block::Table {
        columns: vec![
            col("Local", 1.8),
            col("Planta", 1.1),
            col("m²", 0.6),
            col("Calefacción (W)", 1.0),
            col("de puentes (W)", 0.9),
            col("Refrigeración (W)", 1.0),
            col("Unidad", 0.8),
        ],
        rows: t
            .rooms
            .iter()
            .map(|r| {
                vec![
                    r.name.clone(),
                    level_name(model, r.level),
                    dec(r.area, 1),
                    dec(r.heating, 0),
                    dec(r.heat_bridges, 0),
                    dec(r.cooling, 0),
                    r.unit.map_or("—".into(), |u| format!("{} kW", dec(u, 1))),
                ]
            })
            .collect(),
    });
    out.push(heading("Equipos"));
    out.push(Block::Table {
        columns: vec![
            col("Grupo", 2.4),
            col("Calefacción (kW)", 1.1),
            col("Refrigeración (kW)", 1.1),
            col("Bomba de calor", 1.1),
        ],
        rows: t
            .groups
            .iter()
            .map(|g| {
                vec![
                    g.name.clone(),
                    dec(g.heating / 1000.0, 1),
                    dec(g.cooling / 1000.0, 1),
                    format!("{} kW", dec(g.heat_pump, 1)),
                ]
            })
            .collect(),
    });
    out
}

const MONTHS: [&str; 12] = [
    "Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic",
];

fn applies(on: bool, text: &str) -> Block {
    Block::Check {
        status: Status::Info,
        text: if on {
            text.into()
        } else {
            format!("{text} No se aplica.")
        },
    }
}

/// The renewables: HE 4 (hot water) and HE 5 (electricity).
pub fn renewables(r: &Renewables) -> Vec<Block> {
    let h = &r.hot_water;
    let e = &r.electricity;
    let mut out = vec![
        Block::Para(
            "Contribución renovable para el agua caliente sanitaria (HE 4) con la demanda del Anejo F (28 l/día por persona a 60 °C, personas por dormitorios, factor de centralización) y el agua fría del Anejo G, y potencia fotovoltaica mínima (HE 5). La demanda se incrementa con las pérdidas de distribución (el agua que queda en las tuberías sin retorno se enfría tres veces al día), de acumulación (pérdida estática de un acumulador de clase C) y de recirculación (ida y retorno del anillo, 0,25 W/mK, todo el año). El consumo de energía primaria (HE 0) se justifica con la herramienta oficial.".into(),
        ),
        Block::Heading(2, "1 Agua caliente sanitaria (HE 4)".into()),
        applies(
            h.applies,
            "Se aplica a los edificios nuevos o reformados enteros con una demanda de más de 100 l/día.",
        ),
    ];
    if !h.given && !h.dwellings.is_empty() {
        out.push(Block::Table {
            columns: vec![
                col("Vivienda", 2.4),
                col("Dormitorios", 1.0),
                col("Personas", 1.0),
            ],
            rows: h
                .dwellings
                .iter()
                .map(|d| vec![d.name.clone(), d.bedrooms.to_string(), dec(d.persons, 1)])
                .collect(),
        });
    }
    out.push(Block::Table {
        columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
        rows: vec![
            vec![
                "Demanda".into(),
                if h.given {
                    format!("{} l/día a 60 °C (dada)", dec(h.daily, 0))
                } else {
                    format!(
                        "{} personas × 28 l × {} = {} l/día a 60 °C",
                        dec(h.persons, 1),
                        dec(h.factor, 2),
                        dec(h.daily, 0)
                    )
                },
            ],
            vec![
                "A la temperatura de acumulación".into(),
                format!(
                    "{} l/día a {} °C",
                    dec(h.daily_stored, 0),
                    dec(r.site.storage_temp, 0)
                ),
            ],
            vec![
                "Agua fría".into(),
                format!(
                    "{} ({} m sobre la capital)",
                    r.site.province,
                    dec(r.site.above_capital, 0)
                ),
            ],
            vec![
                "Energía del agua".into(),
                format!("{} kWh", dec(h.useful.iter().sum::<f64>(), 0)),
            ],
            vec![
                "Pérdidas de distribución".into(),
                format!(
                    "{} kWh ({} m de tubería, {} l sin retorno)",
                    dec(h.losses.distribution, 0),
                    dec(h.losses.pipe, 0),
                    dec(h.losses.dead_water, 0)
                ),
            ],
            vec![
                "Pérdidas de acumulación".into(),
                format!(
                    "{} kWh (acumulador de {} l{}, {} W)",
                    dec(h.losses.storage, 0),
                    dec(h.losses.tank, 0),
                    if h.losses.tank_given { "" } else { ", un día" },
                    dec(h.losses.standing, 0)
                ),
            ],
            vec![
                "Pérdidas de recirculación".into(),
                if h.losses.looped > 0.0 {
                    format!(
                        "{} kWh (ida {} m, retorno {} m{})",
                        dec(h.losses.recirculation, 0),
                        dec(h.losses.looped, 0),
                        dec(h.losses.back, 0),
                        if h.losses.return_drawn {
                            ", dibujado"
                        } else {
                            ", supuesto"
                        }
                    )
                } else {
                    "sin retorno".into()
                },
            ],
            vec![
                "Energía anual con pérdidas".into(),
                format!("{} kWh", dec(h.annual, 0)),
            ],
            vec![
                "Fuente renovable".into(),
                match h.source {
                    archisim_core::AcsSource::HeatPump => {
                        format!("{}, SCOP {}", h.source.name(), dec(r.site.scop, 1))
                    }
                    archisim_core::AcsSource::None => h.source.name().into(),
                    archisim_core::AcsSource::Solar if h.solar.is_some() => format!(
                        "{}, fracción {} % por f-Chart (abajo)",
                        h.source.name(),
                        dec(h.provided * 100.0, 1)
                    ),
                    _ => format!(
                        "{}, fracción {} % (dada)",
                        h.source.name(),
                        dec(r.site.fraction * 100.0, 0)
                    ),
                },
            ],
        ],
    });
    out.push(Block::Table {
        columns: std::iter::once(col("Mes", 1.2))
            .chain(MONTHS.iter().map(|m| col(m, 0.7)))
            .collect(),
        rows: vec![
            std::iter::once("T agua fría".to_string())
                .chain(h.cold.iter().map(|t| dec(*t, 1)))
                .collect(),
            std::iter::once("Agua, kWh".to_string())
                .chain(h.useful.iter().map(|k| dec(*k, 0)))
                .collect(),
            std::iter::once("Pérdidas, kWh".to_string())
                .chain(h.lost.iter().map(|k| dec(*k, 0)))
                .collect(),
            std::iter::once("Total, kWh".to_string())
                .chain(h.monthly.iter().map(|k| dec(*k, 0)))
                .collect(),
        ],
    });
    if let Some(f) = &h.solar {
        out.push(Block::Heading(3, "Fracción solar (f-Chart)".into()));
        out.push(Block::Para(format!(
            "{} m² de captadores (área a lo largo de su pendiente) a {}°, orientados {}, con η0 = {} y a1 = {} W/m²K; acumulación de {} l ({} l/m²); latitud {}°. Radiación sobre los captadores por el cielo isótropo de Liu y Jordan con la parte difusa de Erbs; D1 = A·F'R(τα)·H·N/Q y D2 = A·F'R·UL·(100 − Ta)·Δt·K1·K2/Q, con (τα)/(τα)n = 0,96, F'R/FR = 0,95 y agua de uso a {} °C; f = 1,029·D1 − 0,065·D2 − 0,245·D1² + 0,0018·D2² + 0,0215·D1³.{}",
            dec(f.area, 1),
            dec(f.tilt_deg, 0),
            rn::orientation(f.azimuth_deg),
            dec(f.eta0, 2),
            dec(f.k1, 2),
            dec(f.volume, 0),
            dec(f.litres_m2, 0),
            dec(f.latitude, 2),
            dec(archisim_mep::solar::TAC, 0),
            if f.estimated_sun || f.estimated_air {
                " El clima está estimado (radiación por la latitud, aire por el agua fría): el técnico debe sustituirlo por los datos del lugar."
            } else {
                " Clima del lugar, dado."
            }
        )));
        let row = |name: &str, v: &dyn Fn(&archisim_mep::solar::Month) -> String| {
            std::iter::once(name.to_string())
                .chain(f.months.iter().map(v))
                .collect::<Vec<_>>()
        };
        out.push(Block::Table {
            columns: std::iter::once(col("Mes", 1.2))
                .chain(MONTHS.iter().map(|m| col(m, 0.7)))
                .collect(),
            rows: vec![
                row("H horiz., kWh/m²", &|m| dec(m.horizontal, 2)),
                row("H captad., kWh/m²", &|m| dec(m.on_collectors, 2)),
                row("T aire, °C", &|m| dec(m.ambient, 1)),
                row("D1", &|m| dec(m.d1, 2)),
                row("D2", &|m| dec(m.d2, 2)),
                row("f", &|m| dec(m.f.clamp(0.0, 1.0), 2)),
                row("Solar, kWh", &|m| dec(m.solar, 0)),
            ],
        });
        if !f.over.is_empty() {
            out.push(check(
                true,
                format!(
                    "En {} los captadores darían más de lo que se gasta: prever la protección contra sobrecalentamientos",
                    f.over
                        .iter()
                        .map(|&i| MONTHS[i])
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    if h.weak_pump {
        out.push(check(
            true,
            format!(
                "La bomba de calor tiene un SCOP de {}: con {} o menos no cuenta como renovable.",
                dec(r.site.scop, 1),
                dec(rn::MIN_SCOP, 1)
            ),
        ));
    }
    if h.applies {
        out.push(check(
            !h.passes(),
            format!(
                "Contribución renovable {} % ({} kWh de {} kWh); exigida {} %",
                dec(h.provided * 100.0, 0),
                dec(h.renewable_kwh(), 0),
                dec(h.annual, 0),
                dec(h.required * 100.0, 0)
            ),
        ));
    }
    out.push(Block::Heading(
        2,
        "2 Generación de electricidad (HE 5)".into(),
    ));
    out.push(applies(
        e.applies,
        "Se aplica a los edificios nuevos, ampliaciones o reformas enteras de más de 1.000 m² construidos.",
    ));
    out.push(Block::Table {
        columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
        rows: vec![
            vec![
                "Superficie construida S".into(),
                format!(
                    "{} m²{}",
                    dec(e.built, 0),
                    if e.built_given { " (dada)" } else { "" }
                ),
            ],
            vec![
                "Cubierta no transitable Sc".into(),
                format!("{} m²", dec(e.roof, 0)),
            ],
            vec![
                "Ocupada por captadores térmicos Soc".into(),
                format!("{} m²", dec(e.collectors, 0)),
            ],
            vec![
                "P1 = Fpr,el · S".into(),
                format!(
                    "{} × {} = {} kWp",
                    dec(e.factor, 3),
                    dec(e.built, 0),
                    dec(e.p1, 2)
                ),
            ],
            vec![
                "P2 = 0,1 · (0,5 · Sc − Soc)".into(),
                format!("{} kWp", dec(e.p2, 2)),
            ],
            vec![
                "Placas dibujadas".into(),
                format!(
                    "{} filas, {} m² × {} kWp/m² = {} kWp",
                    e.panels,
                    dec(e.panel_area, 1),
                    dec(r.site.panel_kwp_m2, 2),
                    dec(e.installed, 2)
                ),
            ],
        ],
    });
    if e.applies {
        out.push(check(
            !e.passes(),
            format!(
                "Potencia instalada {} kWp; mínima {} kWp",
                dec(e.installed, 2),
                dec(e.minimum, 2)
            ),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flat_has_its_report() {
        let mut m = Model::new("Piso");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Cubierta", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        vt::example_flat(&mut m, l0, l1, lib.partition, lib.facade, lib.interior_door);
        let hood = archisim_library::add_object_entry(&mut m, "Campana decorativa 60").unwrap();
        vt::example_hood(&mut m, l0, l1, hood);
        let blocks = report(&m);
        let headings: Vec<&str> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading(1, t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            headings.iter().any(|h| h.contains("DB-HS 3")),
            "{headings:?}"
        );
        assert!(
            headings.iter().any(|h| h.contains("DB-HE 1")),
            "{headings:?}"
        );
        // The chapters numbered in order, with their sections.
        assert!(headings[0].starts_with("1. "));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Heading(2, t) if t.starts_with("1.1 "))));
        let pages = crate::calc_report::pages(&m, &blocks);
        assert!(pages.len() >= 3, "{}", pages.len());
        if let Ok(dir) = std::env::var("ARCHISIM_MEMORIA_OUT") {
            std::fs::create_dir_all(&dir).unwrap();
            for (i, p) in pages.iter().enumerate() {
                std::fs::write(format!("{dir}/memoria-{:02}.svg", i + 1), p).unwrap();
            }
        }
    }
}
