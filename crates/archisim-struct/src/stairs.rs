//! Concrete stair slabs (losa de escalera) and the lintels over openings
//! in load-bearing walls (cargaderos), the usual sums of a
//! refurbishment.
//!
//! A stair slab spans, simply supported, the longest straight stretch of
//! its walking line in plan between the floors or landings that hold it:
//! its own weight on the slope, the steps, 1 kN/m² of finishes and the
//! live load of the building's use (DB-SE-AE, table 3.1), per metre of
//! plan. The bottom steel for M = q·L²/8, and the slenderness L/d
//! against 20 (simply supported, Código Estructural art. 50.2.2.1).
//!
//! A lintel over an opening in a load-bearing wall carries the wall
//! above it within the arch's triangle of 60° (the rest goes round by
//! the arch), and the floors resting on the wall above the opening if
//! they reach inside the triangle; simply supported with 15 cm of bearing
//! each side. It proposes the steel profiles (two UPN or an IPE) or a
//! concrete lintel; a new opening in an existing wall asks for shoring
//! while it is opened.

use archisim_core::{Model, StairStructure};

use crate::concrete::Rc;
use crate::gravity::{Carrier, Takedown};
use crate::lightframe::{Check, Status, Subject};
use crate::text::dec;

/// The stair slabs.
pub fn stair_checks(model: &Model) -> Vec<Check> {
    let rc = Rc::of(model);
    let building = model.code_settings().building_use;
    let live = crate::actions::category_of_use(building).uniform().max(3.0);
    let mut out = Vec::new();
    for s in model.stairs() {
        if s.design.structure != StairStructure::Slab {
            continue;
        }
        let lay = s.layout();
        let risers = lay.steps.len().max(1);
        let riser = s.height / risers as f64;
        let going = s.going.max(0.2);
        let alpha = (riser / going).atan();
        // The longest straight stretch of the walking line.
        let span = s
            .path
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .fold(0.0, f64::max);
        if span < 0.5 {
            continue;
        }
        let h = s.waist;
        let g = 25.0 * h / alpha.cos() + 25.0 * riser / 2.0 + 1.0;
        let q = 1.35 * g + 1.5 * live;
        let m = q * span * span / 8.0;
        let d = (h - 0.03).max(0.05);
        let subject = Subject::Stair(s.id);
        let steel = rc.bending_steel(m, 1.0, d).map(|a| a.max(0.0018 * h));
        let slender = span / d;
        let what = format!(
            "Escalera {}: losa de {} cm, luz en planta {} m, {} kN/m² (G {} + Q {}): M {} kN·m/m",
            s.id.0,
            dec(h * 100.0, 0),
            dec(span, 2),
            dec(q, 1),
            dec(g, 2),
            dec(live, 1),
            dec(m, 1)
        );
        out.push(match steel {
            Some(a) => Check {
                subject,
                what: format!(
                    "{what}: {} abajo a lo largo y {} de reparto",
                    crate::slab::mesh_for(a).label(),
                    crate::slab::mesh_for((0.2 * a).max(0.0009 * h)).label()
                ),
                rule: "Losa de escalera (Código Estructural, art. 42)",
                ratio: 0.0,
                status: Status::Info,
                advice: Some(
                    "con la misma armadura arriba en los arranques, un cuarto de la luz".into(),
                ),
            },
            None => Check {
                subject,
                what: format!("{what}: el canto no basta"),
                rule: "Losa de escalera (Código Estructural, art. 42)",
                ratio: 1.5,
                status: Status::Fail,
                advice: Some("más espesor de losa o un apoyo intermedio".into()),
            },
        });
        let u = slender / 20.0;
        out.push(Check {
            subject,
            what: format!(
                "Escalera {}: L/d {} con la losa de {} cm",
                s.id.0,
                dec(slender, 1),
                dec(h * 100.0, 0)
            ),
            rule: "Esbeltez de la losa de escalera (L/d ≤ 20, sin calcular la flecha)",
            ratio: u,
            status: if u <= 1.0 { Status::Pass } else { Status::Warn },
            advice: (u > 1.0).then(|| {
                format!(
                    "losa de {} cm, o comprobar la flecha",
                    dec(((span / 20.0 + 0.03) * 100.0).ceil(), 0)
                )
            }),
        });
    }
    out
}

/// The lintels over openings in load-bearing walls.
pub fn lintel_checks(model: &Model, td: &Takedown) -> Vec<Check> {
    let rc = Rc::of(model);
    let fy = model.structural_site().materials.steel_fy * 1000.0;
    let mut out = Vec::new();
    for o in model.openings() {
        let Some(w) = model.wall(o.wall) else {
            continue;
        };
        let Some(c) = td.carriers.get(&Carrier::Wall(w.id)) else {
            continue;
        };
        let Some(ot) = model.opening_type(o.opening_type) else {
            continue;
        };
        let width = ot.width;
        let head = o.sill + ot.height;
        if width < 0.3 || head >= w.height - 0.05 {
            continue;
        }
        let Some(wt) = model.wall_type(w.wall_type) else {
            continue;
        };
        let thick: f64 = wt.layers.iter().map(|l| l.thickness).sum();
        let unit = crate::weights::wall_type_weight(model, w.wall_type);
        // The triangle of the wall over it (60°) and what the wall
        // carries along the opening, if the floors rest within it.
        let tri_h = width / 2.0 * 3f64.sqrt();
        let above = w.height - head;
        let wall_load = unit
            * tri_h.min(above)
            * 0.5
            * if tri_h <= above {
                1.0
            } else {
                2.0 - above / tri_h
            };
        let length = w.start.distance(w.end).max(0.1);
        let received: f64 = c
            .received
            .values()
            .map(|v| v.iter().sum::<f64>())
            .sum::<f64>()
            / length;
        let floors = if tri_h > above { received } else { 0.0 };
        let q = 1.4 * (wall_load + floors) + 1.35 * unit * 0.0;
        let l = width + 0.15;
        let m = q * l * l / 8.0;
        let v = q * l / 2.0;
        // Steel: two UPN or one IPE; W needed at fy/1,05.
        let w_need = m / (fy / 1.05);
        let upn = [
            ("2 UPN 80", 2.0 * 26.5e-6),
            ("2 UPN 100", 2.0 * 41.2e-6),
            ("2 UPN 120", 2.0 * 60.7e-6),
            ("2 UPN 140", 2.0 * 86.4e-6),
            ("2 UPN 160", 2.0 * 116e-6),
            ("2 UPN 180", 2.0 * 150e-6),
            ("2 UPN 200", 2.0 * 191e-6),
        ];
        let pick = upn
            .iter()
            .find(|(_, wel)| *wel >= w_need)
            .map_or("2 UPN 220 o mayor", |x| x.0);
        let d = 0.25 - 0.04;
        let concrete = rc.bending_steel(m, thick.max(0.12), d).map(|a| {
            rc.bars_for(a.max(rc.min_beam_steel(thick.max(0.12), 0.25)), 2)
                .label()
        });
        let existing = model.phase(o.id.0) == archisim_core::Phase::New
            && model.phase(w.id.0) == archisim_core::Phase::Existing;
        out.push(Check {
            subject: Subject::Wall(w.id),
            what: format!(
                "Hueco de {} m en {} (muro {}): carga {} kN/m ({} de muro{}), M {} kN·m, V {} kN: cargadero de {} o de hormigón 25 cm con {}",
                dec(width, 2),
                wt.name,
                w.id.0,
                dec(q, 1),
                dec(wall_load, 1),
                if floors > 0.0 { format!(" y {} de forjados", dec(floors, 1)) } else { String::new() },
                dec(m, 1),
                dec(v, 1),
                pick,
                concrete.unwrap_or_else(|| "más canto".into())
            ),
            rule: "Dintel o cargadero sobre el hueco (DB-SE-F, DB-SE-A)",
            ratio: 0.0,
            status: if existing { Status::Warn } else { Status::Info },
            advice: Some(if existing {
                "hueco nuevo en un muro existente: apear el muro (puntales y sopandas) mientras se abre y se coloca el cargadero, apoyado 15 cm por lado".into()
            } else {
                "apoyado 15 cm por lado".into()
            }),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stair_slab() {
        let mut m = Model::new("Escalera");
        let l0 = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Hormigón",
            "concrete",
            [180, 180, 180],
            archisim_core::CutPattern::Solid,
        );
        m.add_stair(archisim_core::NewStair {
            level: l0,
            path: vec![
                archisim_core::Point2::new(0.0, 0.0),
                archisim_core::Point2::new(4.2, 0.0),
            ],
            width: 1.0,
            going: 0.28,
            base_offset: 0.0,
            top: archisim_core::WallTop::Unconnected,
            height: 2.8,
            waist: 0.15,
            material: mat,
            usage: archisim_core::StairUse::General,
            design: archisim_core::StairDesign::default(),
        })
        .unwrap();
        let c = stair_checks(&m);
        let slab = c
            .iter()
            .find(|c| c.rule.starts_with("Losa de escalera"))
            .unwrap();
        assert!(slab.what.contains("luz en planta 4,20"), "{}", slab.what);
        // 4,2 m with a waist of 15: L/d 35, too slender.
        let s = c.iter().find(|c| c.rule.starts_with("Esbeltez")).unwrap();
        assert_eq!(s.status, Status::Warn, "{}", s.what);
    }

    #[test]
    fn a_new_opening_in_an_old_wall() {
        use archisim_core::*;
        let mut m = Model::new("Reforma");
        let l0 = m.add_level("Planta baja", 0.0);
        let l1 = m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let mut walls = Vec::new();
        for x in [0.0, 5.0] {
            walls.push(
                m.add_wall(NewWall {
                    wall_type: lib.facade,
                    level: l0,
                    start: Point2::new(x, 0.0),
                    end: Point2::new(x, 6.0),
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height: 3.0,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                })
                .unwrap(),
            );
        }
        m.add_slab(NewSlab {
            slab_type: lib.floor_slab,
            level: l1,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(5.0, 0.0),
                Point2::new(5.0, 6.0),
                Point2::new(0.0, 6.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let o = m
            .add_opening(NewOpening {
                opening_type: lib.balcony_door,
                wall: walls[0],
                along: 3.0,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
        // An existing wall; the door, new in it.
        m.set_phase(walls[0].0, Phase::Existing).unwrap();
        m.set_phase(o.0, Phase::New).unwrap();
        let td = crate::takedown(&m);
        let c = lintel_checks(&m, &td);
        assert_eq!(c.len(), 1, "{c:?}");
        assert!(c[0].what.contains("UPN"), "{}", c[0].what);
        assert_eq!(c[0].status, Status::Warn, "{}", c[0].what);
        assert!(c[0].advice.as_deref().unwrap_or("").contains("apear"));
    }
}
