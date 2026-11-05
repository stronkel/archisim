//! Basement walls (DB-SE-C, 6 and 7): the concrete exterior walls with
//! earth against them, the ground's height from the terrain where there
//! is one and ±0,00 elsewhere. Pressure at rest K0 = 1 − sin φ with the
//! weight of the ground and 10 kN/m² of surcharge; the wall held at the
//! top by the floor that rests on it (propped cantilever) or free there
//! (cantilever). The bending at the base and in the span and the shear,
//! against the concrete of its structural layer (Código Estructural), with
//! the steel it needs per metre on each face.

use archisim_core::{LayerFunction, Model, Point2, WallFunction};

use crate::concrete::Rc;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

/// The surcharge on the ground beside the building, kN/m².
pub const SURCHARGE: f64 = 10.0;

/// Height of the ground at a point, above project zero.
fn ground_at(model: &Model, p: Point2) -> f64 {
    for l in model.levels() {
        if let Some(z) = model.terrains_on_level(l.id).find_map(|t| t.height_at(p)) {
            return z;
        }
    }
    0.0
}

/// The moments (base, span) and the base shear per metre of a wall of
/// height `h` with triangular pressure `p` at its base and uniform `w`.
pub fn earth_forces(h: f64, p: f64, w: f64, propped: bool) -> (f64, f64, f64) {
    if propped {
        (
            p * h * h / 15.0 + w * h * h / 8.0,
            0.0298 * p * h * h + 9.0 * w * h * h / 128.0,
            0.4 * p * h + 0.625 * w * h,
        )
    } else {
        (p * h * h / 6.0 + w * h * h / 2.0, 0.0, p * h / 2.0 + w * h)
    }
}

pub fn check(model: &Model) -> Vec<Check> {
    let site = model.structural_site();
    let k0 = 1.0 - site.soil_friction.to_radians().sin();
    let rc = Rc::of(model);
    let mut out = Vec::new();
    for w in model.walls() {
        let Some(t) = model.wall_type(w.wall_type) else {
            continue;
        };
        if t.function != WallFunction::Exterior {
            continue;
        }
        let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
        let mid = Point2::new((w.start.x + w.end.x) / 2.0, (w.start.y + w.end.y) / 2.0);
        let earth = (ground_at(model, mid) - base).min(w.height);
        if earth < 0.5 {
            continue;
        }
        let structure: Vec<_> = t
            .layers
            .iter()
            .filter(|l| l.function == LayerFunction::Structure)
            .collect();
        let concrete: f64 = structure
            .iter()
            .filter(|l| Kind::of(model.material(l.material)) == Kind::Concrete)
            .map(|l| l.thickness)
            .sum();
        if concrete < 0.1 {
            out.push(Check {
                subject: Subject::Wall(w.id),
                what: format!(
                    "{}: {} m de tierras contra un muro que no es de hormigón",
                    t.name,
                    dec(earth, 2)
                ),
                rule: "Muro de sótano (DB-SE-C, 6)",
                ratio: 0.0,
                status: Status::Warn,
                advice: Some(
                    "un muro de hormigón armado, o justificar la fábrica con su empuje".into(),
                ),
            });
            continue;
        }
        // Held at the top by a floor resting on it.
        let top = base + w.height;
        let propped = model.slabs().any(|s| {
            let z = model.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset;
            (z - top).abs() < 0.5 && crate::geo::near_polygon(mid, &s.outline, 0.5)
        });
        let p = k0 * site.soil_weight * earth;
        let q = k0 * SURCHARGE;
        let (mb, ms, v) = earth_forces(earth, 1.35 * p, 1.5 * q, propped);
        let d = concrete - 0.05;
        let amin = 0.0009 * concrete;
        let earth_face = rc.bending_steel(mb, 1.0, d);
        let inner_face = rc.bending_steel(ms, 1.0, d);
        let vrd = rc.vrd_c(1.0, d, 0.002);
        let shear = v / vrd;
        let held = if propped {
            "apeado arriba por el forjado"
        } else {
            "en ménsula"
        };
        match (earth_face, inner_face) {
            (Some(a), Some(b)) => {
                let a = a.max(amin);
                let b = b.max(amin);
                out.push(Check {
                    subject: Subject::Wall(w.id),
                    what: format!(
                        "{} ({} m de hormigón, {held}): {} m de tierras, K0 {}; M base {} kN·m/m, M vano {} kN·m/m; pide {} cm²/m en la cara de las tierras abajo ({}) y {} cm²/m en la otra ({})",
                        t.name,
                        dec(concrete, 2),
                        dec(earth, 2),
                        dec(k0, 2),
                        dec(mb, 1),
                        dec(ms, 1),
                        dec(a * 1e4, 2),
                        crate::slab::mesh_for(a).label(),
                        dec(b * 1e4, 2),
                        crate::slab::mesh_for(b).label()
                    ),
                    rule: "Muro de sótano a flexión (DB-SE-C, 6; Código Estructural, art. 42)",
                    ratio: 0.0,
                    status: Status::Info,
                    advice: None,
                });
            }
            _ => out.push(Check {
                subject: Subject::Wall(w.id),
                what: format!(
                    "{}: {} m de tierras, M {} kN·m/m: el espesor de {} m no basta",
                    t.name,
                    dec(earth, 2),
                    dec(mb.max(ms), 1),
                    dec(concrete, 2)
                ),
                rule: "Muro de sótano a flexión (DB-SE-C, 6; Código Estructural, art. 42)",
                ratio: 2.0,
                status: Status::Fail,
                advice: Some("más espesor de muro".into()),
            }),
        }
        out.push(Check {
            subject: Subject::Wall(w.id),
            what: format!(
                "{}: cortante en la base {} kN/m, resiste {} kN/m sin cercos",
                t.name,
                dec(v, 1),
                dec(vrd, 1)
            ),
            rule: "Muro de sótano a cortante (Código Estructural, art. 44)",
            ratio: shear,
            status: if shear <= 1.0 {
                Status::Pass
            } else {
                Status::Fail
            },
            advice: (shear > 1.0).then(|| "más espesor de muro".into()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_on_a_basement_wall() {
        // 3 m of earth, φ 30° (K0 0,5), 18 kN/m³: 27 kN/m² at the base.
        // Propped: M base = 27·9/15 = 16,2 kN·m/m.
        let (mb, ms, v) = earth_forces(3.0, 27.0, 0.0, true);
        assert!((mb - 16.2).abs() < 1e-9);
        assert!(ms > 0.0 && ms < mb);
        assert!((v - 0.4 * 27.0 * 3.0).abs() < 1e-9);
        // A cantilever takes it all at the base: 27·9/6 = 40,5.
        let (mb, _, _) = earth_forces(3.0, 27.0, 0.0, false);
        assert!((mb - 40.5).abs() < 1e-9);
    }
}
