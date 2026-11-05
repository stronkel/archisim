//! The calculation report of the structure («Anejo de cálculo»): its
//! content as blocks (titles, paragraphs, tables and checks), for paper
//! (`archisim-plan::calc_report`) or text.

use archisim_core::Model;

use crate::actions::{self, ROUGHNESS_NAMES};
use crate::analysis::{self, Analysis};
use crate::analytical::Owner;
use crate::cases::Kind;
use crate::gravity::Carrier;
use crate::lightframe::{self, Status, Subject};
use crate::text::dec;

/// A piece of the report.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// The report's title page: title, project, lines under it.
    Cover {
        title: String,
        project: String,
        lines: Vec<String>,
    },
    /// A numbered heading of a chapter (level 1) or section (level 2).
    Heading(u8, String),
    Para(String),
    /// A table: its columns' titles and relative widths, and its rows.
    Table {
        columns: Vec<(String, f64)>,
        rows: Vec<Vec<String>>,
    },
    /// The result of a check: passes, to review or fails.
    Check {
        status: Status,
        text: String,
    },
    /// Start the next block on a new page.
    PageBreak,
}

fn kind_name(k: Kind) -> &'static str {
    k.name()
}

/// The report.
pub fn report(model: &Model) -> Vec<Block> {
    let a = analysis::analyse(model);
    let checks = lightframe::check(model, &a.takedown, a.wind.as_ref());
    blocks(model, &a, &checks)
}

/// The report from an analysis already made.
pub fn blocks(model: &Model, a: &Analysis, checks: &[lightframe::Check]) -> Vec<Block> {
    let mut out = Vec::new();
    let site = model.structural_site();
    let sa = actions::site_actions(model);
    let fails = checks.iter().filter(|c| c.status == Status::Fail).count()
        + a.deflections
            .iter()
            .filter(|d| {
                let (li, lc, la) = Analysis::limits(d, false);
                d.integrity > li || d.comfort > lc || d.appearance > la
            })
            .count();
    out.push(Block::Cover {
        title: "Anejo de cálculo de la estructura".into(),
        project: model.project_name.clone(),
        lines: vec![
            format!("Programa: ArchiSim {}", env!("CARGO_PKG_VERSION")),
            format!(
                "Resultado: {}",
                if fails == 0 {
                    "todas las comprobaciones cumplen".to_string()
                } else {
                    format!("{fails} comprobaciones no cumplen (capítulo 7)")
                }
            ),
            "Autor del cálculo: ______________________________".into(),
            "Este anejo lo genera el programa a partir del modelo. Sus hipótesis (cargas, apoyos, uniones, terreno) las revisa y asume el técnico que lo firma.".into(),
        ],
    });
    out.push(Block::PageBreak);
    // 1. Normativa y método.
    out.push(Block::Heading(1, "1. Normativa y método".into()));
    out.push(Block::Para(
        "Normativa: CTE DB-SE (bases de cálculo), DB-SE-AE (acciones), DB-SE-A (acero), DB-SE-M (madera), DB-SE-C (cimientos), DB-SI 6 (resistencia al fuego), NCSE-02 (sismo) y Código Estructural; Eurocódigos EN 1993-1-3 (perfiles de chapa conformada en frío), EN 1993-1-5 (anchos eficaces) y EN 1995-1-1 (madera), en lo que el CTE remite a ellos.".into(),
    ));
    out.push(Block::Para(
        "Método: las cargas de cada forjado y cubierta se reparten a lo que las sostiene (en los entramados, en la dirección de las viguetas o cabios; en el resto, por áreas tributarias) y bajan por muros, vigas y pilares hasta el terreno. El pórtico de pilares y vigas se calcula en 3D, en régimen elástico lineal y en primer orden, por el método de la rigidez. Cada vigueta y cabio de los entramados se calcula como viga continua sobre sus apoyos, y cada montante con la carga de su muro y el viento de su fachada. Las combinaciones son las de DB-SE 4.2 con los coeficientes de la tabla 4.1 y los ψ de la tabla 4.2.".into(),
    ));
    // 2. Materiales.
    out.push(Block::Heading(1, "2. Materiales y coeficientes".into()));
    out.push(Block::Table {
        columns: vec![
            ("Material".into(), 2.0),
            ("Resistencia".into(), 3.0),
            ("Rigidez".into(), 2.0),
            ("γM".into(), 1.2),
        ],
        rows: vec![
            vec![
                "Acero S350GD+Z (perfiles conformados)".into(),
                "fyb 350 N/mm², fu 420 N/mm²".into(),
                "E 210 000 N/mm²".into(),
                "1,05 / 1,05 / 1,25".into(),
            ],
            vec![
                "Madera C24".into(),
                "fm 24, fc,0 21, ft,0 14,5, fv 4, fc,90 2,5 N/mm²".into(),
                "E0,mean 11 000, E0,05 7 400 N/mm²".into(),
                "1,30; kmod por duración".into(),
            ],
            vec![
                "Hormigón HA-25".into(),
                "fck 25 N/mm²".into(),
                "Ecm 27 300 N/mm²".into(),
                "1,50".into(),
            ],
            vec![
                "Acero B500S (armaduras)".into(),
                "fyk 500 N/mm²".into(),
                "Es 200 000 N/mm²".into(),
                "1,15".into(),
            ],
        ],
    });
    out.push(Block::Para(
        "Coeficientes de las acciones (DB-SE tabla 4.1): permanentes 1,35 (0,80 si favorecen), variables 1,50 (0 si favorecen).".into(),
    ));
    // 3. Acciones.
    out.push(Block::Heading(1, "3. Acciones".into()));
    out.push(Block::Heading(2, "3.1 Emplazamiento".into()));
    out.push(Block::Table {
        columns: vec![("Dato".into(), 2.0), ("Valor".into(), 3.0)],
        rows: vec![
            vec!["Altitud".into(), format!("{} m", dec(sa.altitude, 0))],
            vec![
                "Nieve".into(),
                format!(
                    "zona de clima invernal {}, sk = {} kN/m² (anejo E)",
                    site.snow_zone,
                    dec(sa.ground_snow, 2)
                ),
            ],
            vec![
                "Viento".into(),
                format!(
                    "zona {}, qb = {} kN/m²; entorno {}",
                    site.wind_zone.code(),
                    dec(sa.basic_wind, 2),
                    ROUGHNESS_NAMES[(site.roughness.clamp(1, 5) - 1) as usize]
                ),
            ],
            vec![
                "Sismo (NCSE-02)".into(),
                if site.seismic_ab > 0.0 {
                    format!(
                        "ab = {}·g, K = {}, C = {}, importancia {}",
                        dec(site.seismic_ab, 2),
                        dec(site.seismic_k, 1),
                        dec(site.soil, 1),
                        site.importance.name().to_lowercase()
                    )
                } else {
                    "no se considera (ab = 0)".into()
                },
            ],
            vec![
                "Terreno".into(),
                format!("tensión admisible {} kN/m²", dec(site.bearing, 0)),
            ],
        ],
    });
    out.push(Block::Heading(
        2,
        "3.2 Cargas de forjados y cubiertas".into(),
    ));
    let mut rows = Vec::new();
    for sf in &a.takedown.surfaces {
        let level = model.level(sf.level).map_or("—".into(), |l| l.name.clone());
        let uses: Vec<String> = sf
            .uses
            .iter()
            .map(|(c, (ar, kn))| {
                format!(
                    "{} {}",
                    c.code(),
                    dec(if *ar > 0.0 { kn / ar } else { 0.0 }, 2)
                )
            })
            .collect();
        rows.push(vec![
            format!("{level} · {}", sf.name),
            dec(sf.area, 1),
            dec(sf.dead, 2),
            if uses.is_empty() {
                "—".into()
            } else {
                uses.join(", ")
            },
            if sf.exposed > 0.0 {
                dec(sf.snow, 2)
            } else {
                "—".into()
            },
        ]);
    }
    out.push(Block::Table {
        columns: vec![
            ("Forjado o cubierta".into(), 4.0),
            ("m²".into(), 1.0),
            ("G kN/m²".into(), 1.2),
            ("Q kN/m² (uso)".into(), 2.0),
            ("N kN/m²".into(), 1.2),
        ],
        rows,
    });
    out.push(Block::Para(
        "G incluye el peso de todas las capas del tipo (con su entramado, tableros y tejas). Q es la sobrecarga de la tabla 3.1 según el uso de cada local (A1 viviendas, G1 cubiertas solo para conservación…). Las cargas de los tabiques dibujados se aplican donde están.".into(),
    ));
    if let Some(w) = &a.wind {
        out.push(Block::Heading(2, "3.3 Viento".into()));
        out.push(Block::Para(format!(
            "Edificio de {} m de altura y {} × {} m en planta. Presión qe = qb · ce · cp, con ce por la altura y el entorno (anejo D.2) y cp de las fachadas por su esbeltez (tabla D.3).",
            dec(w.height, 2),
            dec(w.hi.x - w.lo.x, 2),
            dec(w.hi.y - w.lo.y, 2)
        )));
        out.push(Block::Table {
            columns: vec![
                ("Dirección".into(), 1.0),
                ("Banda".into(), 1.6),
                ("Ancho m".into(), 1.0),
                ("ce".into(), 0.8),
                ("cp".into(), 1.4),
                ("F kN".into(), 1.0),
            ],
            rows: w
                .storeys
                .iter()
                .map(|st| {
                    vec![
                        st.dir.name().into(),
                        format!("{} – {} m", dec(st.from, 2), dec(st.to, 2)),
                        dec(st.width, 2),
                        dec(st.exposure, 2),
                        format!("{} / {}", dec(st.cp.0, 2), dec(st.cp.1, 2)),
                        dec(st.force, 1),
                    ]
                })
                .collect(),
        });
    }
    let loads: Vec<_> = model.structural_loads().collect();
    if !loads.is_empty() {
        out.push(Block::Heading(2, "3.4 Cargas puestas a mano".into()));
        out.push(Block::Table {
            columns: vec![
                ("Carga".into(), 2.0),
                ("Caso".into(), 1.5),
                ("Valor".into(), 1.5),
            ],
            rows: loads
                .iter()
                .map(|l| {
                    vec![
                        l.name.clone(),
                        l.case.name().into(),
                        format!("{} {}", dec(l.value, 2), l.shape.unit()),
                    ]
                })
                .collect(),
        });
    }
    // 4. Combinaciones.
    out.push(Block::Heading(1, "4. Combinaciones".into()));
    out.push(Block::Para(format!(
        "{} combinaciones con los casos {}.",
        a.combinations.len(),
        a.cases
            .iter()
            .map(|c| c.short())
            .collect::<Vec<_>>()
            .join(", ")
    )));
    for kind in [
        Kind::Ultimate,
        Kind::Seismic,
        Kind::Characteristic,
        Kind::Frequent,
        Kind::QuasiPermanent,
    ] {
        let of: Vec<_> = a.combinations.iter().filter(|c| c.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        out.push(Block::Heading(
            2,
            format!("{} ({})", kind_name(kind), of.len()),
        ));
        out.push(Block::Para(
            of.iter().map(|c| c.label()).collect::<Vec<_>>().join("; "),
        ));
    }
    // 5. Descenso de cargas.
    out.push(Block::Heading(1, "5. Descenso de cargas".into()));
    out.push(Block::Para("Lo que cada muro, viga y pilar lleva a su base (valores característicos, kN), con su propio peso.".into()));
    let vertical: Vec<_> = a
        .takedown
        .cases
        .iter()
        .copied()
        .filter(|c| !matches!(c, crate::Case::Wind(_) | crate::Case::Quake(_)))
        .collect();
    let mut cols = vec![("Elemento".into(), 4.0), ("Long. m".into(), 1.0)];
    cols.extend(vertical.iter().map(|c| (c.short(), 1.0)));
    let mut rows = Vec::new();
    for (k, c) in &a.takedown.carriers {
        if c.delivered_total(crate::Case::Dead) < 1e-6 {
            continue;
        }
        let name = carrier_name(model, *k);
        let mut row = vec![
            name,
            if c.length > 0.0 {
                dec(c.length, 2)
            } else {
                "—".into()
            },
        ];
        row.extend(vertical.iter().map(|cs| dec(c.delivered_total(*cs), 1)));
        rows.push(row);
    }
    let mut ground = vec!["Al terreno".to_string(), String::new()];
    ground.extend(
        vertical
            .iter()
            .map(|cs| dec(a.takedown.ground_total(*cs), 1)),
    );
    rows.push(ground);
    let mut applied = vec!["Aplicado (comprobación)".to_string(), String::new()];
    applied.extend(
        vertical
            .iter()
            .map(|cs| dec(a.takedown.applied_total(*cs), 1)),
    );
    rows.push(applied);
    out.push(Block::Table {
        columns: cols,
        rows,
    });
    for n in &a.takedown.notes {
        out.push(Block::Check {
            status: Status::Warn,
            text: n.clone(),
        });
    }
    // 6. Pórtico.
    out.push(Block::Heading(1, "6. Pórtico de pilares y vigas".into()));
    if a.model.frame.bars.is_empty() {
        out.push(Block::Para(
            "No hay pilares ni vigas: la estructura son muros y forjados (capítulos 5 y 7).".into(),
        ));
    } else {
        let f = &a.model.frame;
        out.push(Block::Para(format!(
            "{} nudos, {} barras y {} apoyos. Uniones rígidas en hormigón y articuladas en los extremos de las vigas de acero y madera; pilares empotrados en su cimiento. El viento lo toma el pórtico en {}.",
            f.nodes.len(),
            f.bars.len(),
            f.supports.len(),
            if a.model.wind_on_frame.is_empty() {
                "ninguna dirección (lo toman los muros)".to_string()
            } else {
                a.model
                    .wind_on_frame
                    .iter()
                    .map(|d| d.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )));
        let mut rows = Vec::new();
        for case in &a.cases {
            let (ap, back) = a.frame_balance(*case);
            if ap.abs() > 1e-6 {
                rows.push(vec![case.name(), dec(ap, 1), dec(back, 1)]);
            }
        }
        out.push(Block::Heading(2, "6.1 Equilibrio".into()));
        out.push(Block::Table {
            columns: vec![
                ("Caso".into(), 2.0),
                ("Cargas kN".into(), 1.0),
                ("Reacciones kN".into(), 1.0),
            ],
            rows,
        });
        out.push(Block::Heading(
            2,
            "6.2 Esfuerzos de cálculo (envolvente ELU)".into(),
        ));
        out.push(Block::Table {
            columns: vec![
                ("Elemento".into(), 3.0),
                ("N kN".into(), 1.4),
                ("V kN".into(), 1.0),
                ("My kN·m".into(), 1.4),
                ("Mz kN·m".into(), 1.0),
            ],
            rows: a
                .envelopes
                .iter()
                .map(|e| {
                    vec![
                        owner_name(model, a, e.owner),
                        format!("{} / {}", dec(e.min[0], 1), dec(e.max[0], 1)),
                        dec(e.max[2].max(-e.min[2]).max(e.max[1].max(-e.min[1])), 1),
                        format!("{} / {}", dec(e.min[4], 1), dec(e.max[4], 1)),
                        dec(e.max[5].max(-e.min[5]), 1),
                    ]
                })
                .collect(),
        });
        out.push(Block::Heading(2, "6.3 Flechas (DB-SE 4.3.3)".into()));
        for d in &a.deflections {
            let (li, lc, la) = Analysis::limits(d, false);
            let ok = d.integrity <= li && d.comfort <= lc && d.appearance <= la;
            out.push(Block::Check {
                status: if ok { Status::Pass } else { Status::Fail },
                text: format!(
                    "{}, vano de {} m{}: integridad {} mm (≤ {}), confort {} mm (≤ {}), apariencia {} mm (≤ {})",
                    owner_name(model, a, d.owner),
                    dec(d.length, 2),
                    if d.cantilever { " en voladizo" } else { "" },
                    dec(d.integrity * 1000.0, 1),
                    dec(li * 1000.0, 1),
                    dec(d.comfort * 1000.0, 1),
                    dec(lc * 1000.0, 1),
                    dec(d.appearance * 1000.0, 1),
                    dec(la * 1000.0, 1),
                ),
            });
        }
        out.push(Block::Heading(2, "6.4 Reacciones en los apoyos".into()));
        out.push(Block::Table {
            columns: vec![
                ("Apoyo (x, y)".into(), 2.0),
                ("ELU kN".into(), 1.0),
                ("Característica kN".into(), 1.0),
            ],
            rows: a
                .reactions
                .iter()
                .map(|(n, u, c)| {
                    let p = f.nodes[*n];
                    vec![
                        format!("({}, {})", dec(p[0], 2), dec(p[1], 2)),
                        dec(*u, 1),
                        dec(*c, 1),
                    ]
                })
                .collect(),
        });
        for n in &a.notes {
            out.push(Block::Check {
                status: Status::Warn,
                text: n.clone(),
            });
        }
    }
    // 7. Entramados.
    if !checks.is_empty() {
        out.push(Block::Heading(1, "7. Entramados ligeros".into()));
        out.push(Block::Para(format!(
            "Comprobaciones de cada elemento con entramado: se da la pieza más desfavorable de cada comprobación. Fijaciones de los tableros a {} mm en los bordes de los paneles.",
            dec(lightframe::EDGE_SPACING * 1000.0, 0)
        )));
        let mut subjects: Vec<Subject> = checks.iter().map(|c| c.subject).collect();
        subjects.sort();
        subjects.dedup();
        for sub in subjects {
            out.push(Block::Heading(2, subject_name(model, sub)));
            for c in checks.iter().filter(|c| c.subject == sub) {
                let ratio = if c.status == Status::Info || c.ratio == 0.0 {
                    String::new()
                } else {
                    format!(" — aprovechamiento {}", dec(c.ratio, 2))
                };
                out.push(Block::Check {
                    status: c.status,
                    text: format!(
                        "{}: {}{ratio}{}",
                        c.rule,
                        c.what,
                        c.advice
                            .as_ref()
                            .map_or(String::new(), |a| format!(". Propuesta: {a}"))
                    ),
                });
            }
        }
    }
    // 8. Limitaciones.
    out.push(Block::Heading(1, "8. Alcance y limitaciones".into()));
    for p in [
        "El cálculo es elástico, lineal y de primer orden; no incluye el efecto P-Δ ni la plastificación.",
        "Las losas y forjados de hormigón reparten su carga por áreas tributarias; no se calculan sus armados.",
        "Las uniones se suponen como se indica (rígidas en hormigón, articuladas en acero y madera); su diseño y el de los anclajes al cimiento se completan en el proyecto.",
        "El arriostramiento de los entramados sigue el método A de EN 1995-1-1, 9.2.4.2; en steel frame, con la resistencia de las fijaciones al tablero y a la chapa.",
        "La resistencia al fuego de los entramados es orientativa: se justifica con el ensayo o el certificado del sistema.",
    ] {
        out.push(Block::Para(p.into()));
    }
    out
}

fn carrier_name(model: &Model, k: Carrier) -> String {
    match k {
        Carrier::Wall(id) => model
            .wall(id)
            .and_then(|w| model.wall_type(w.wall_type))
            .map_or(format!("Muro {}", id.0), |t| {
                format!("Muro {} · {}", id.0, t.name)
            }),
        Carrier::Beam(id) => format!("Viga {}", id.0),
        Carrier::Column(id) => format!("Pilar {}", id.0),
    }
}

fn owner_name(model: &Model, a: &Analysis, o: Owner) -> String {
    let kind = a.model.kinds.get(&o).map_or("", |k| k.name());
    match o {
        Owner::Column(id) => {
            let t = model
                .column(id)
                .and_then(|c| model.member_type(c.member_type))
                .map_or(String::new(), |t| t.name.clone());
            format!("Pilar {} ({t}, {kind})", id.0)
        }
        Owner::Beam(id) => {
            let t = model
                .beam(id)
                .and_then(|b| model.member_type(b.member_type))
                .map_or(String::new(), |t| t.name.clone());
            format!("Viga {} ({t}, {kind})", id.0)
        }
    }
}

pub fn subject_name(model: &Model, s: Subject) -> String {
    match s {
        Subject::Wall(id) => model
            .wall(id)
            .map(|w| {
                let t = model
                    .wall_type(w.wall_type)
                    .map_or(String::new(), |t| t.name.clone());
                let l = model
                    .level(w.level)
                    .map_or(String::new(), |l| l.name.clone());
                format!("Muro {} · {t} ({l})", id.0)
            })
            .unwrap_or_else(|| format!("Muro {}", id.0)),
        Subject::Floor(id) => model
            .slab(id)
            .map(|s| {
                let t = model
                    .slab_type(s.slab_type)
                    .map_or(String::new(), |t| t.name.clone());
                format!("Forjado {} · {t}", id.0)
            })
            .unwrap_or_else(|| format!("Forjado {}", id.0)),
        Subject::Roof(id) => model
            .roof(id)
            .map(|r| {
                let t = model
                    .slab_type(r.roof_type)
                    .map_or(String::new(), |t| t.name.clone());
                format!("Cubierta {} · {t}", id.0)
            })
            .unwrap_or_else(|| format!("Cubierta {}", id.0)),
        Subject::Building => "Edificio".into(),
    }
}

/// The report as plain text.
pub fn to_text(blocks: &[Block]) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    for b in blocks {
        match b {
            Block::Cover {
                title,
                project,
                lines,
            } => {
                let _ = writeln!(s, "{title}\n{project}");
                for l in lines {
                    let _ = writeln!(s, "  {l}");
                }
            }
            Block::Heading(_, t) => {
                let _ = writeln!(s, "\n{t}");
            }
            Block::Para(p) => {
                let _ = writeln!(s, "{p}");
            }
            Block::Table { columns, rows } => {
                let _ = writeln!(
                    s,
                    "  {}",
                    columns
                        .iter()
                        .map(|c| c.0.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                for r in rows {
                    let _ = writeln!(s, "  {}", r.join(" | "));
                }
            }
            Block::Check { status, text } => {
                let mark = match status {
                    Status::Pass => "cumple",
                    Status::Warn => "revisar",
                    Status::Fail => "NO CUMPLE",
                    Status::Info => "·",
                };
                let _ = writeln!(s, "  [{mark}] {text}");
            }
            Block::PageBreak => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_has_its_chapters() {
        let mut m = Model::new("Anejo");
        let l0 = m.add_level("Planta baja", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        m.add_column(archisim_core::NewColumn {
            member_type: lib.column,
            level: l0,
            position: archisim_core::Point2::new(0.0, 0.0),
            rotation: 0.0,
            base_offset: 0.0,
            top: archisim_core::WallTop::Unconnected,
            height: 3.0,
        })
        .unwrap();
        let r = report(&m);
        let heads: Vec<_> = r
            .iter()
            .filter_map(|b| match b {
                Block::Heading(1, t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert!(
            heads.iter().any(|h| h.starts_with("3. Acciones")),
            "{heads:?}"
        );
        assert!(heads.iter().any(|h| h.starts_with("6. Pórtico")));
        assert!(to_text(&r).contains("Anejo de cálculo"));
    }
}
