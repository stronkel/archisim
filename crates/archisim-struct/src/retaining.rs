//! Basement walls (DB-SE-C, 6 and 7): the concrete exterior walls with
//! earth against them, the ground's height from the terrain where there
//! is one and ±0,00 elsewhere. Pressure at rest K0 = 1 − sin φ with the
//! weight of the ground and 10 kN/m² of surcharge; the wall held at the
//! top by the floor that rests on it (propped cantilever) or free there
//! (cantilever). The bending at the base and in the span and the shear,
//! against the concrete of its structural layer (Código Estructural), with
//! the steel it needs per metre on each face.
//!
//! A wall that nothing holds at the top is a retaining wall on its strip
//! footing (DB-SE-C, 6.3 and table 2.1): active pressure Ka = (1 − sin φ) /
//! (1 + sin φ) down to the underside of the footing, the wall, the footing
//! and the earth over the heel against overturning about the toe (γ 1,8)
//! and sliding with tan(⅔φ) (γ 1,5), and the pressure under the footing
//! with its eccentricity (the edge to 1,25·σadm). The footing is the one
//! drawn under the wall or, without one, the narrowest that passes.

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
        // A wall of variable section: thicker at the foot of its faces,
        // where it bends most; the earth from the top of its footing.
        let section = model.wall_section(w.id);
        let (concrete, earth) = match &section {
            Some(s) if concrete >= 0.1 => {
                let (e, i) = s.leans_at(w.height, 0.0);
                (concrete + e + i, earth - s.footing_depth())
            }
            _ => (concrete, earth),
        };
        if earth < 0.5 {
            continue;
        }
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
        if !propped {
            match &section {
                Some(sec) if sec.footing.is_some() => {
                    section_checks(model, w, &t.name, earth, sec, &mut out)
                }
                _ => retaining_checks(model, w, &t.name, earth, concrete, &mut out),
            }
        }
        let p = k0 * site.soil_weight * earth;
        let q = k0 * SURCHARGE;
        // A free wall leans: active pressure (it can move), else at rest.
        let (p, q) = if propped {
            (p, q)
        } else {
            let sn = site.soil_friction.to_radians().sin();
            let ka = (1.0 - sn) / (1.0 + sn);
            (ka * site.soil_weight * earth, ka * SURCHARGE)
        };
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

/// A retaining wall on its footing, per metre: earth height `h` above the
/// footing, footing width `b` and depth `hf`, wall thickness `t` and
/// weight `w_wall` (kN/m), ground φ (degrees), γ and the surcharge.
/// Returns (overturning safety, sliding safety, edge pressure kN/m²,
/// eccentricity m).
#[allow(clippy::too_many_arguments)]
pub fn retaining(
    h: f64,
    b: f64,
    hf: f64,
    t: f64,
    w_wall: f64,
    phi: f64,
    gamma: f64,
    q: f64,
) -> (f64, f64, f64, f64) {
    let s = phi.to_radians().sin();
    let ka = (1.0 - s) / (1.0 + s);
    let ht = h + hf;
    let e_tri = 0.5 * ka * gamma * ht * ht;
    let e_q = ka * q * ht;
    let m_over = e_tri * ht / 3.0 + e_q * ht / 2.0;
    let toe = ((b - t) / 2.0).max(0.0);
    let heel = toe;
    let w_foot = 25.0 * b * hf;
    let w_soil = gamma * h * heel + q * heel;
    let m_stab = w_wall * (toe + t / 2.0) + w_foot * b / 2.0 + w_soil * (b - heel / 2.0);
    let v = w_wall + w_foot + w_soil;
    let overturn = m_stab / m_over.max(1e-9);
    let slide = v * (2.0 / 3.0 * phi.to_radians()).tan() / (e_tri + e_q).max(1e-9);
    // Resultant from the toe, eccentricity from the centre.
    let x = (m_stab - m_over) / v.max(1e-9);
    let e = b / 2.0 - x;
    let edge = if e.abs() <= b / 6.0 {
        v / b * (1.0 + 6.0 * e.abs() / b)
    } else if x > 0.0 {
        2.0 * v / (3.0 * x)
    } else {
        f64::INFINITY
    };
    (overturn, slide, edge, e)
}

/// A retaining wall per metre as it is built (a wall of variable section
/// with its own footing), measured across from the toe (the edge of the
/// footing away from the earth) towards the earth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetainingSection {
    /// Width and depth of the footing.
    pub b: f64,
    pub hf: f64,
    /// Weight of the wall and its footing, kN/m, and where it acts.
    pub weight: f64,
    pub x_weight: f64,
    /// The earth's face of the wall at the top of the footing, from the
    /// toe, and how far it leans back towards the toe per metre up.
    pub face: f64,
    pub lean: f64,
}

impl RetainingSection {
    /// The section of a wall `crown` thick and `height` high whose earth is
    /// on its exterior face, its materials `density` kN/m³.
    pub fn of(s: &archisim_core::WallSection, crown: f64, height: f64, density: f64) -> Self {
        let f = s.footing.unwrap_or(archisim_core::WallFooting {
            toe_ext: 0.0,
            toe_int: 0.0,
            depth: 0.0,
        });
        let d = f.depth;
        let (e, i) = s.leans_at(height, 0.0);
        let base = crown + e + i;
        let b = base + f.toe_ext + f.toe_int;
        let stem = (height - d).max(0.0);
        // From the toe: the interior face's foot at toe_int, its top at
        // toe_int + i; the exterior face's foot at toe_int + base.
        let (x0, x1) = (f.toe_int, f.toe_int + base);
        let (t0, t1) = (f.toe_int + i, f.toe_int + i + crown);
        let stem_area = stem * (base + crown) / 2.0;
        // The trapezoid's centroid: its two triangles and its rectangle.
        let stem_x = if stem_area > 1e-12 {
            let rect = (t0, t1, crown * stem);
            let left = ((x0 + 2.0 * t0) / 3.0, 0.5 * (t0 - x0) * stem);
            let right = ((x1 + 2.0 * t1) / 3.0, 0.5 * (x1 - t1) * stem);
            ((rect.0 + rect.1) / 2.0 * rect.2 + left.0 * left.1 + right.0 * right.1) / stem_area
        } else {
            0.0
        };
        let foot_area = b * d;
        let area = stem_area + foot_area;
        RetainingSection {
            b,
            hf: d,
            weight: density * area,
            x_weight: if area > 1e-12 {
                (stem_x * stem_area + b / 2.0 * foot_area) / area
            } else {
                b / 2.0
            },
            face: x1,
            lean: s.batter_ext,
        }
    }
}

/// [`retaining`] for a wall as built: `h` of earth above its footing.
/// The earth over the heel takes in the wedge over a battered face.
/// Returns (overturning safety, sliding safety, edge pressure kN/m²,
/// eccentricity m).
pub fn retaining_section(
    h: f64,
    s: &RetainingSection,
    phi: f64,
    gamma: f64,
    q: f64,
) -> (f64, f64, f64, f64) {
    let sn = phi.to_radians().sin();
    let ka = (1.0 - sn) / (1.0 + sn);
    let ht = h + s.hf;
    let e_tri = 0.5 * ka * gamma * ht * ht;
    let e_q = ka * q * ht;
    let m_over = e_tri * ht / 3.0 + e_q * ht / 2.0;
    let b = s.b;
    let heel = (b - s.face).max(0.0);
    // Earth over the heel and over the leaning face (its wedge).
    let wedge = 0.5 * s.lean * h * h;
    let w_heel = gamma * h * heel;
    let w_wedge = gamma * wedge;
    let w_q = q * (heel + s.lean * h);
    let x_heel = b - heel / 2.0;
    let x_wedge = s.face - s.lean * h / 3.0;
    let x_q = b - (heel + s.lean * h) / 2.0;
    let m_stab = s.weight * s.x_weight + w_heel * x_heel + w_wedge * x_wedge + w_q * x_q;
    let v = s.weight + w_heel + w_wedge + w_q;
    let overturn = m_stab / m_over.max(1e-9);
    let slide = v * (2.0 / 3.0 * phi.to_radians()).tan() / (e_tri + e_q).max(1e-9);
    let x = (m_stab - m_over) / v.max(1e-9);
    let e = b / 2.0 - x;
    let edge = if e.abs() <= b / 6.0 {
        v / b * (1.0 + 6.0 * e.abs() / b)
    } else if x > 0.0 {
        2.0 * v / (3.0 * x)
    } else {
        f64::INFINITY
    };
    (overturn, slide, edge, e)
}

/// The checks of a retaining wall of variable section on its own footing:
/// its real weight and base.
fn section_checks(
    model: &Model,
    w: &archisim_core::Wall,
    name: &str,
    earth: f64,
    sec: &archisim_core::WallSection,
    out: &mut Vec<Check>,
) {
    let site = model.structural_site();
    let Some(t) = model.wall_type(w.wall_type) else {
        return;
    };
    let crown = t.thickness();
    let density = crate::weights::wall_type_weight(model, w.wall_type) / crown.max(1e-9);
    let rs = RetainingSection::of(sec, crown, w.height, density);
    let (o, sl, edge, e) =
        retaining_section(earth, &rs, site.soil_friction, site.soil_weight, SURCHARGE);
    let f = sec.footing.unwrap_or(archisim_core::USUAL_FOOTING);
    let foot = format!(
        "zarpa de {} m (puntera {}, talón {}) y {} de canto, {} m en la base del alzado, {} kN/m",
        dec(rs.b, 2),
        dec(f.toe_int, 2),
        dec(f.toe_ext, 2),
        dec(rs.hf, 2),
        dec(sec.base_thickness(crown, w.height), 2),
        dec(rs.weight, 1)
    );
    let mut push = |rule: &'static str, what: String, ratio: f64| {
        out.push(Check {
            subject: Subject::Wall(w.id),
            what,
            rule,
            ratio,
            status: if ratio <= 1.0 {
                Status::Pass
            } else {
                Status::Fail
            },
            advice: (ratio > 1.0).then(|| {
                "una zarpa más ancha (sobre todo el talón, del lado de las tierras), más talud o un tacón"
                    .into()
            }),
        });
    };
    push(
        "Vuelco del muro de contención (DB-SE-C, 6.3; γ 1,8)",
        format!(
            "{name} con {foot}: {} m de tierras, seguridad al vuelco {}",
            dec(earth, 2),
            dec(o, 2)
        ),
        1.8 / o.max(1e-9),
    );
    push(
        "Deslizamiento del muro de contención (DB-SE-C, 6.3; γ 1,5)",
        format!(
            "{name}: seguridad al deslizamiento {} (rozamiento tan ⅔φ)",
            dec(sl, 2)
        ),
        1.5 / sl.max(1e-9),
    );
    push(
        "Presión bajo la zapata del muro (DB-SE-C, 4.3; borde ≤ 1,25·σadm)",
        format!(
            "{name}: presión en el borde {} kN/m² (excentricidad {} m) sobre {} admisibles",
            dec(edge, 0),
            dec(e, 2),
            dec(1.25 * site.bearing, 0)
        ),
        edge / (1.25 * site.bearing),
    );
}

/// The checks of a retaining wall.
#[allow(clippy::too_many_arguments)]
fn retaining_checks(
    model: &Model,
    w: &archisim_core::Wall,
    name: &str,
    earth: f64,
    t: f64,
    out: &mut Vec<Check>,
) {
    let site = model.structural_site();
    let w_wall = crate::weights::wall_type_weight(model, w.wall_type) * (earth + 0.5);
    let drawn = model.footings().find(|f| {
        matches!(f.kind, archisim_core::FootingKind::Strip { wall: Some(id), .. } if id == w.id)
    });
    let passes = |b: f64, hf: f64| {
        let (o, sl, edge, _) = retaining(
            earth,
            b,
            hf,
            t,
            w_wall,
            site.soil_friction,
            site.soil_weight,
            SURCHARGE,
        );
        o >= 1.8 && sl >= 1.5 && edge <= 1.25 * site.bearing
    };
    let hf = drawn.map_or((earth / 8.0).max(0.4), |f| f.depth);
    let (b, from_drawing) = match drawn {
        Some(f) => (f.width, true),
        None => {
            let mut b = (t + 0.4).max(0.6);
            while !passes(b, hf) && b < 4.0 * earth.max(1.0) {
                b += 0.05;
            }
            (b, false)
        }
    };
    let (o, sl, edge, e) = retaining(
        earth,
        b,
        hf,
        t,
        w_wall,
        site.soil_friction,
        site.soil_weight,
        SURCHARGE,
    );
    let rule_o = "Vuelco del muro de contención (DB-SE-C, 6.3; γ 1,8)";
    let rule_s = "Deslizamiento del muro de contención (DB-SE-C, 6.3; γ 1,5)";
    let rule_p = "Presión bajo la zapata del muro (DB-SE-C, 4.3; borde ≤ 1,25·σadm)";
    let foot = format!(
        "zapata de {} m de ancho y {} de canto",
        dec(b, 2),
        dec(hf, 2)
    );
    if !from_drawing {
        out.push(Check {
            subject: Subject::Wall(w.id),
            what: format!(
                "{name}: {} m de tierras sin forjado arriba: muro de contención en ménsula; pide una {foot} centrada",
                dec(earth, 2)
            ),
            rule: rule_o,
            ratio: 0.0,
            status: Status::Info,
            advice: Some("el muro no tiene zapata corrida dibujada".into()),
        });
        return;
    }
    let mut push = |rule: &'static str, what: String, ratio: f64| {
        out.push(Check {
            subject: Subject::Wall(w.id),
            what,
            rule,
            ratio,
            status: if ratio <= 1.0 {
                Status::Pass
            } else {
                Status::Fail
            },
            advice: (ratio > 1.0).then(|| {
                "una zapata más ancha (sobre todo el talón, del lado de las tierras) o un tacón"
                    .into()
            }),
        });
    };
    push(
        rule_o,
        format!(
            "{name} sobre {foot}: {} m de tierras, seguridad al vuelco {}",
            dec(earth, 2),
            dec(o, 2)
        ),
        1.8 / o.max(1e-9),
    );
    push(
        rule_s,
        format!(
            "{name}: seguridad al deslizamiento {} (rozamiento tan ⅔φ)",
            dec(sl, 2)
        ),
        1.5 / sl.max(1e-9),
    );
    push(
        rule_p,
        format!(
            "{name}: presión en el borde {} kN/m² (excentricidad {} m) sobre {} admisibles",
            dec(edge, 0),
            dec(e, 2),
            dec(1.25 * site.bearing, 0)
        ),
        edge / (1.25 * site.bearing),
    );
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

    #[test]
    fn a_retaining_wall_stands() {
        // 2 m of earth, φ 30° (Ka 1/3), 18 kN/m³, 25 cm wall of 12,5 kN/m,
        // footing 0,40 m deep: total height 2,4 m, E = ½·⅓·18·2,4² = 17,3
        // kN/m plus ⅓·10·2,4 = 8 kN/m.
        let narrow = retaining(2.0, 0.8, 0.4, 0.25, 12.5, 30.0, 18.0, 10.0);
        let wide = retaining(2.0, 2.0, 0.4, 0.25, 12.5, 30.0, 18.0, 10.0);
        assert!(
            narrow.0 < wide.0 && narrow.1 < wide.1,
            "{narrow:?} {wide:?}"
        );
        assert!(wide.0 > 1.8, "{wide:?}");
        // A narrow one tips over.
        assert!(narrow.0 < 1.8, "{narrow:?}");
    }

    #[test]
    fn the_section_as_built_matches_the_plain_wall() {
        // An upright 25 cm wall on a centred 2 m footing, 0,40 deep, as a
        // section: the same as the plain formula (12,5 kN/m of wall over
        // 2,4 m of height is 20,8 kN/m³ × 0,25 × 2,4; footing at 25).
        let s = archisim_core::WallSection {
            footing: Some(archisim_core::WallFooting {
                toe_ext: 0.875,
                toe_int: 0.875,
                depth: 0.4,
            }),
            ..Default::default()
        };
        let rs = RetainingSection::of(&s, 0.25, 2.4, 25.0);
        assert!((rs.b - 2.0).abs() < 1e-12 && (rs.x_weight - 1.0).abs() < 1e-12);
        let a = retaining_section(2.0, &rs, 30.0, 18.0, 10.0);
        let w_wall = 25.0 * 0.25 * 2.0;
        let b = retaining(2.0, 2.0, 0.4, 0.25, w_wall, 30.0, 18.0, 10.0);
        assert!(
            (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
            "{a:?} {b:?}"
        );
        // A batter on the earth side adds weight and the earth over it.
        let battered = archisim_core::WallSection {
            batter_ext: 0.1,
            ..s
        };
        let rb = RetainingSection::of(&battered, 0.25, 2.4, 25.0);
        let c = retaining_section(2.0, &rb, 30.0, 18.0, 10.0);
        assert!(
            rb.weight > rs.weight && c.0 > a.0 && c.1 > a.1,
            "{a:?} {c:?}"
        );
    }
}
