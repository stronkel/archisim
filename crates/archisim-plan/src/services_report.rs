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
use archisim_mep::electrical::{self as el, Installation};
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
    electrical: Vec<Installation>,
    water: Water,
    drainage: Drainage,
    ventilation: Ventilation,
    thermal: Thermal,
    renewables: Renewables,
}

/// The report's blocks.
pub fn report(model: &Model) -> Vec<Block> {
    let all = All {
        electrical: el::installations(model),
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
    let has_ventilation = all.ventilation.applies && !all.ventilation.dwellings.is_empty();
    let has_thermal = !all.thermal.rooms.is_empty();
    let mut chapters: Vec<(&str, Vec<Block>)> = Vec::new();
    if !all.electrical.is_empty() {
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

pub fn electricity(model: &Model, all: &[Installation]) -> Vec<Block> {
    let mut out = vec![Block::Para(
        "Instalación interior de baja tensión, monofásica a 230 V, según el Reglamento Electrotécnico para Baja Tensión: grado de electrificación (ITC-BT-10), circuitos (ITC-BT-25, tabla 1), intensidades admisibles de la UNE-HD 60364-5-52 (método B1, tubo empotrado, PVC, dos conductores cargados), caída de tensión con cobre a 70 °C contra el 3 % (ITC-BT-19) y derivación individual (ITC-BT-15).".into(),
    )];
    for (k, inst) in all.iter().enumerate() {
        let board = inst
            .board
            .and_then(|b| model.device(b))
            .map_or("Cuadro (no dibujado)".to_string(), |d| d.name.clone());
        out.push(Block::Heading(
            2,
            format!("{} {board} · {}", k + 1, level_name(model, inst.level)),
        ));
        out.push(Block::Table {
            columns: vec![col("Dato", 2.0), col("Valor", 3.0)],
            rows: vec![
                vec![
                    "Grado de electrificación".into(),
                    match inst.grade {
                        Electrification::High => "Elevada".into(),
                        _ => "Básica".into(),
                    },
                ],
                vec![
                    "Superficie servida".into(),
                    format!("{} m²", dec(inst.area, 1)),
                ],
                vec![
                    "Potencia prevista".into(),
                    format!("{} W", dec(inst.power, 0)),
                ],
                vec![
                    "Interruptor general (IGA)".into(),
                    format!("{} A", inst.main_switch),
                ],
                vec![
                    "Derivación individual".into(),
                    format!(
                        "{} m, 2 × {} + T mm² Cu, tubo Ø{} mm, ΔU {} % (≤ {} %)",
                        dec(inst.feeder.length, 1),
                        dec(inst.feeder.section, 0),
                        inst.feeder.tube,
                        dec(inst.feeder.drop, 2),
                        dec(inst.feeder.limit, 1)
                    ),
                ],
                vec![
                    "Diferenciales".into(),
                    inst.differentials
                        .iter()
                        .map(|d| {
                            format!(
                                "{} A/{} mA (C{})",
                                d.rating,
                                d.sensitivity,
                                d.circuits
                                    .iter()
                                    .map(|c| c.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", C")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · "),
                ],
            ],
        });
        for r in &inst.reasons {
            out.push(Block::Para(r.clone()));
        }
        out.push(Block::Table {
            columns: vec![
                col("Circuito", 2.6),
                col("Puntos", 0.9),
                col("P (W)", 1.0),
                col("I (A)", 0.9),
                col("PIA", 0.8),
                col("Sección", 1.0),
                col("L (m)", 0.9),
                col("ΔU", 0.9),
            ],
            rows: inst
                .circuits
                .iter()
                .map(|c| {
                    vec![
                        format!("C{} {}", c.spec.number, c.spec.name),
                        c.count.to_string(),
                        dec(c.power, 0),
                        dec(c.current, 1),
                        format!("{} A", c.breaker),
                        format!("{} mm²", dec(c.chosen, 1)),
                        dec(c.length, 1),
                        format!("{} %", dec(c.drop, 2)),
                    ]
                })
                .collect(),
        });
        for c in &inst.circuits {
            let fails = c.current > f64::from(c.breaker) + 1e-9
                || f64::from(c.breaker) > c.ampacity + 1e-9
                || c.drop > 3.0 + 1e-9;
            if fails {
                out.push(check(
                    true,
                    format!(
                        "C{}: Ib {} A, In {} A, Iz {} A, ΔU {} %",
                        c.spec.number,
                        dec(c.current, 1),
                        c.breaker,
                        dec(c.ampacity, 0),
                        dec(c.drop, 2)
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
    let mut out = vec![Block::Para(format!(
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
    ))];
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
        if n.is_hood_duct() {
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
    if !v.others.is_empty() {
        out.push(Block::Heading(
            2,
            "4 Aparcamientos, trasteros y almacenes de residuos".into(),
        ));
        out.push(Block::Table {
            columns: vec![
                col("Local", 2.2),
                col("Clase", 1.6),
                col("m²", 0.8),
                col("Plazas", 0.8),
                col("l/s", 0.8),
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
                    ]
                })
                .collect(),
        });
        for o in &v.others {
            let lines = vt::other_lines(o);
            let status = if o.vent_short() || o.garage_fails() > 0 {
                Status::Fail
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

// ---------------------------------------------------------------------
// Envelope and loads
// ---------------------------------------------------------------------

pub fn thermal(model: &Model, t: &Thermal) -> Vec<Block> {
    let mut out = vec![
        Block::Para(
            "Transmitancias U = 1/(Rsi + Σ e/λ + Rse) con las capas de cada tipo; límites de la tabla 3.1.1.a-HE1 por la zona de invierno y coeficiente global K de los locales calefactados contra la tabla 3.1.1.b-HE1 por la compacidad. Cargas de calefacción por transmisión y ventilación; de refrigeración, con el sol por los vidrios, las personas, la luz y los aparatos.".into(),
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
                        "U ventanas {} W/m²K, puertas {} W/m²K, factor solar {}",
                        dec(t.site.window_u, 2),
                        dec(t.site.door_u, 2),
                        dec(t.site.glass_g, 2)
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
        Block::Heading(2, "1 Elementos de la envolvente".into()),
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
    out.push(check(
        t.k > t.k_limit + 1e-9,
        format!(
            "Coeficiente global K = {} W/m²K (≤ {}, tabla 3.1.1.b-HE1)",
            dec(t.k, 2),
            dec(t.k_limit, 2)
        ),
    ));
    out.push(Block::Heading(2, "2 Cargas térmicas".into()));
    out.push(Block::Table {
        columns: vec![
            col("Local", 2.2),
            col("Planta", 1.4),
            col("m²", 0.7),
            col("Calefacción (W)", 1.1),
            col("Refrigeración (W)", 1.1),
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
                    dec(r.cooling, 0),
                    r.unit.map_or("—".into(), |u| format!("{} kW", dec(u, 1))),
                ]
            })
            .collect(),
    });
    out.push(Block::Heading(2, "3 Equipos".into()));
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
                    _ => format!(
                        "{}, fracción {} %",
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
