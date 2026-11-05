//! The actions and the takedown written out as text (for the command line
//! and as the base of the calculation report).

use std::fmt::Write as _;

use archisim_core::{Model, WallFunction};

use crate::actions::{self, ROUGHNESS_NAMES};
use crate::cases::{combinations, Case, Kind};
use crate::gravity::{Carrier, Takedown};
use crate::wind::BuildingWind;

pub fn dec(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// The site data and what they give.
pub fn site_text(model: &Model) -> String {
    let site = model.structural_site();
    let a = actions::site_actions(model);
    let mut s = String::new();
    let _ = writeln!(s, "Emplazamiento (DB-SE-AE, NCSE-02)");
    let _ = writeln!(
        s,
        "  Altitud: {} m{}",
        dec(a.altitude, 0),
        match (site.altitude, model.georeference()) {
            (Some(_), _) => "",
            (None, Some(_)) => " (de la georreferencia)",
            (None, None) => " (sin georreferencia)",
        }
    );
    let _ = writeln!(
        s,
        "  Nieve: zona de clima invernal {}, sk = {} kN/m² (tabla E.2)",
        site.snow_zone,
        dec(a.ground_snow, 2)
    );
    let _ = writeln!(
        s,
        "  Viento: zona {}, qb = {} kN/m²; entorno {}",
        site.wind_zone.code(),
        dec(a.basic_wind, 2),
        ROUGHNESS_NAMES[(site.roughness.clamp(1, 5) - 1) as usize]
    );
    if site.seismic_ab > 0.0 {
        let _ = writeln!(
            s,
            "  Sismo: ab = {}·g, K = {}, C = {}, importancia {}",
            dec(site.seismic_ab, 2),
            dec(site.seismic_k, 1),
            dec(site.soil, 1),
            site.importance.name().to_lowercase()
        );
    } else {
        let _ = writeln!(s, "  Sismo: no se considera (ab = 0)");
    }
    let _ = writeln!(
        s,
        "  Terreno: tensión admisible {} kN/m²",
        dec(site.bearing, 0)
    );
    s
}

/// The floors and roofs with their loads.
pub fn surfaces_text(model: &Model, td: &Takedown) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "Cargas en forjados y cubiertas");
    for sf in &td.surfaces {
        let level = model.level(sf.level).map_or("—".into(), |l| l.name.clone());
        let uses: Vec<String> = sf
            .uses
            .iter()
            .map(|(c, (a, kn))| {
                format!(
                    "{} {} kN/m² en {} m²",
                    c.code(),
                    dec(if *a > 0.0 { kn / a } else { 0.0 }, 2),
                    dec(*a, 1)
                )
            })
            .collect();
        let _ = writeln!(
            s,
            "  {} · {}: {} m², G {} kN/m²; Q {}{}{}{}",
            level,
            sf.name,
            dec(sf.area, 1),
            dec(sf.dead, 2),
            if uses.is_empty() {
                "—".into()
            } else {
                uses.join(", ")
            },
            if sf.exposed > 0.0 {
                format!("; N {} kN/m² en {} m²", dec(sf.snow, 2), dec(sf.exposed, 1))
            } else {
                String::new()
            },
            if sf.one_way {
                "; unidireccional (entramado)"
            } else {
                ""
            },
            if sf.on_ground {
                "; sobre el terreno"
            } else if sf.unsupported {
                "; SIN APOYOS"
            } else {
                ""
            }
        );
    }
    s
}

/// What each wall, beam and column carries, and what reaches the ground.
pub fn takedown_text(model: &Model, td: &Takedown) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "Descenso de cargas (valores característicos)");
    let vertical: Vec<Case> = td
        .cases
        .iter()
        .copied()
        .filter(|c| !matches!(c, Case::Wind(_) | Case::Quake(_)))
        .collect();
    for (k, c) in &td.carriers {
        let total = |case: Case| c.delivered_total(case);
        let g = total(Case::Dead);
        if g < 1e-6 {
            continue;
        }
        let name = match k {
            Carrier::Wall(id) => model.wall(*id).and_then(|w| {
                model.wall_type(w.wall_type).map(|t| {
                    format!(
                        "Muro {} · {}{}",
                        id.0,
                        t.name,
                        if t.function == WallFunction::Exterior {
                            " (fachada)"
                        } else {
                            ""
                        }
                    )
                })
            }),
            Carrier::Beam(id) => Some(format!("Viga {}", id.0)),
            Carrier::Column(id) => Some(format!("Pilar {}", id.0)),
        }
        .unwrap_or_default();
        let others: Vec<String> = vertical
            .iter()
            .filter(|cs| **cs != Case::Dead)
            .filter_map(|cs| {
                let v = total(*cs);
                (v > 1e-6).then(|| format!("{} {}", cs.short(), dec(v, 1)))
            })
            .collect();
        let per_m = if c.length > 0.0 {
            format!(
                " ({} m, {} kN/m en la base)",
                dec(c.length, 2),
                dec(g / c.length, 1)
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            s,
            "  {name}: G {} kN{}{}",
            dec(g, 1),
            if others.is_empty() {
                String::new()
            } else {
                format!(", {}", others.join(", "))
            },
            per_m
        );
    }
    let _ = writeln!(s, "  Al terreno (y lo aplicado):");
    for case in &vertical {
        let v = td.ground_total(*case);
        let a = td.applied_total(*case);
        if v.abs() > 1e-6 || a.abs() > 1e-6 {
            let _ = writeln!(
                s,
                "    {}: {} kN (aplicado {} kN){}",
                case.name(),
                dec(v, 1),
                dec(a, 1),
                if (v - a).abs() > 0.005 * a.abs().max(1.0) {
                    " — NO CUADRA"
                } else {
                    ""
                }
            );
        }
    }
    for n in &td.notes {
        let _ = writeln!(s, "  Aviso: {n}");
    }
    s
}

pub fn wind_text(w: &BuildingWind) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Viento sobre el edificio (altura {} m, planta {} × {} m)",
        dec(w.height, 2),
        dec(w.hi.x - w.lo.x, 2),
        dec(w.hi.y - w.lo.y, 2)
    );
    for st in &w.storeys {
        let _ = writeln!(
            s,
            "  {} · de {} a {} m: ancho {} m, ce {}, cp {} / {}, F = {} kN",
            st.dir.name(),
            dec(st.from, 2),
            dec(st.to, 2),
            dec(st.width, 2),
            dec(st.exposure, 2),
            dec(st.cp.0, 2),
            dec(st.cp.1, 2),
            dec(st.force, 1)
        );
    }
    s
}

pub fn combinations_text(cases: &[Case], altitude: f64) -> String {
    let all = combinations(cases, altitude);
    let mut s = String::new();
    let _ = writeln!(s, "Combinaciones (DB-SE 4.2): {}", all.len());
    for kind in [
        Kind::Ultimate,
        Kind::Seismic,
        Kind::Characteristic,
        Kind::Frequent,
        Kind::QuasiPermanent,
    ] {
        let of: Vec<_> = all.iter().filter(|c| c.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        let _ = writeln!(s, "  {}: {}", kind.name(), of.len());
        for c in of.iter().take(4) {
            let _ = writeln!(s, "    {}", c.label());
        }
        if of.len() > 4 {
            let _ = writeln!(s, "    … y {} más", of.len() - 4);
        }
    }
    s
}

/// The frame analysis written out.
pub fn analysis_text(model: &Model, a: &crate::analysis::Analysis) -> String {
    use crate::analytical::Owner;
    let mut s = String::new();
    let f = &a.model.frame;
    if f.bars.is_empty() {
        let _ = writeln!(
            s,
            "Pórtico de pilares y vigas: no hay (la estructura son muros y forjados)."
        );
        return s;
    }
    let _ = writeln!(
        s,
        "Pórtico de pilares y vigas: {} nudos, {} barras, {} apoyos; {} casos y {} combinaciones",
        f.nodes.len(),
        f.bars.len(),
        f.supports.len(),
        a.cases.len(),
        a.combinations.len()
    );
    for case in &a.cases {
        let (ap, back) = a.frame_balance(*case);
        if ap.abs() > 1e-6 {
            let _ = writeln!(
                s,
                "  Equilibrio {}: cargas {} kN, reacciones {} kN{}",
                case.short(),
                dec(ap, 1),
                dec(back, 1),
                if (ap - back).abs() > 1e-3 * ap.abs().max(1.0) {
                    " — NO CUADRA"
                } else {
                    ""
                }
            );
        }
    }
    let _ = writeln!(s, "  Esfuerzos de cálculo (envolvente ELU):");
    for e in &a.envelopes {
        let kind = a.model.kinds.get(&e.owner).map_or("", |k| k.name());
        let name = match e.owner {
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
        };
        let _ = writeln!(
            s,
            "    {name}: N {} / {} kN, Vz {} kN, My {} / {} kN·m, Mz {} kN·m; manda {}",
            dec(e.min[0], 1),
            dec(e.max[0], 1),
            dec(e.max[2].max(-e.min[2]), 1),
            dec(e.min[4], 1),
            dec(e.max[4], 1),
            dec(e.max[5].max(-e.min[5]), 1),
            a.combinations[e.governing].label()
        );
    }
    let _ = writeln!(s, "  Flechas (DB-SE 4.3.3):");
    for d in &a.deflections {
        let (li, lc, la) = crate::analysis::Analysis::limits(d, false);
        let ok = d.integrity <= li && d.comfort <= lc && d.appearance <= la;
        let name = match d.owner {
            Owner::Beam(id) => format!("Viga {}", id.0),
            Owner::Column(id) => format!("Pilar {}", id.0),
        };
        let _ = writeln!(
            s,
            "    {name}, vano de {} m{}: integridad {} mm (≤ {}), confort {} mm (≤ {}), apariencia {} mm (≤ {}) {}",
            dec(d.length, 2),
            if d.cantilever { " en voladizo" } else { "" },
            dec(d.integrity * 1000.0, 1),
            dec(li * 1000.0, 1),
            dec(d.comfort * 1000.0, 1),
            dec(lc * 1000.0, 1),
            dec(d.appearance * 1000.0, 1),
            dec(la * 1000.0, 1),
            if ok { "cumple" } else { "NO CUMPLE" }
        );
    }
    let _ = writeln!(s, "  Reacciones verticales máximas en los apoyos:");
    for (node, ult, chr) in &a.reactions {
        let p = f.nodes[*node];
        let _ = writeln!(
            s,
            "    ({}, {}): ELU {} kN, característica {} kN",
            dec(p[0], 2),
            dec(p[1], 2),
            dec(*ult, 1),
            dec(*chr, 1)
        );
    }
    for n in &a.notes {
        let _ = writeln!(s, "  Aviso: {n}");
    }
    s
}

/// The checks of the light frames written out.
pub fn lightframe_text(model: &Model, checks: &[crate::lightframe::Check]) -> String {
    use crate::lightframe::{Status, Subject};
    let mut s = String::new();
    if checks.is_empty() {
        return s;
    }
    let _ = writeln!(
        s,
        "Entramados ligeros (EN 1993-1-3, EN 1995-1-1, DB-SE-M, DB-SI)"
    );
    let mut subjects: Vec<Subject> = checks.iter().map(|c| c.subject).collect();
    subjects.sort();
    subjects.dedup();
    for sub in subjects {
        let title = match sub {
            Subject::Wall(id) => format!("Muro {}", id.0),
            Subject::Floor(id) => format!("Forjado {}", id.0),
            Subject::Roof(id) => format!("Cubierta {}", id.0),
            Subject::Building => "Edificio".into(),
        };
        let _ = model;
        let _ = writeln!(s, "  {title}");
        for c in checks.iter().filter(|c| c.subject == sub) {
            let mark = match c.status {
                Status::Pass => "cumple   ",
                Status::Warn => "revisar  ",
                Status::Fail => "NO CUMPLE",
                Status::Info => "         ",
            };
            let ratio = if matches!(c.status, Status::Info) || c.ratio == 0.0 {
                String::new()
            } else {
                format!(" [{}]", dec(c.ratio, 2))
            };
            let _ = writeln!(
                s,
                "    {mark} {}{ratio} — {}{}",
                c.what,
                c.rule,
                c.advice
                    .as_ref()
                    .map_or(String::new(), |a| format!(". Propuesta: {a}"))
            );
        }
    }
    s
}
