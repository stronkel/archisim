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
        if site.altitude.is_none() {
            " (de la georreferencia)"
        } else {
            ""
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
