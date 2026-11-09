//! Weight, lifting and assembly of the modular boxes (M3, `docs/MODULAR.md`
//! §5 and §5 bis). A box's weight is its chassis (the tubes of
//! `archisim_geom::boxes`) and what it carries out of the factory: its
//! walls (their type's weight per m², with the frame if they have one,
//! less their doors and windows, which weigh on their own), floors, roof
//! and false ceilings; each where it is, which gives the centre of
//! gravity. Lifting: the box hangs from its points through a spreader
//! beam, so the slings pull straight up; each long edge of the floor frame
//! carries its share of the weight (by where the centre of gravity is
//! across the box) as a uniform load, between the corners (four points) or
//! in two spans (six). A predimensioning, not a calculation of the frame.

use archisim_core::{signed_area, BoxId, LevelId, LiftPoints, Model, ModularBox, Point2};
use archisim_geom::boxes::{chassis_members, lift_points};

use crate::weights::{slab_type_weight, wall_type_weight};

/// kN to kg.
const KG_PER_KN: f64 = 1000.0 / 9.81;
/// Doors and windows, kN/m² of opening (frame and double glazing).
pub const OPENING_WEIGHT: f64 = 0.35;
/// Partial factor of the weight when it is lifted.
pub const GAMMA_LIFT: f64 = 1.35;
/// Dynamic factor of a crane lift.
pub const DYNAMIC: f64 = 1.20;
/// Yield strength of the chassis' tubes (S275), MPa.
pub const STEEL_FY: f64 = 275.0;
pub const GAMMA_M0: f64 = 1.05;
/// Steel's modulus, MPa.
pub const STEEL_E: f64 = 210_000.0;

/// One part of a box's weight.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightPart {
    pub what: String,
    pub kg: f64,
    /// Its centre, in the box's axes (along, across, over the level).
    pub at: [f64; 3],
}

/// A box's weight and its centre of gravity.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxWeight {
    pub parts: Vec<WeightPart>,
    pub kg: f64,
    /// In the box's axes.
    pub centre: [f64; 3],
}

impl BoxWeight {
    pub fn tonnes(&self) -> f64 {
        self.kg / 1000.0
    }

    pub fn kn(&self) -> f64 {
        self.kg / KG_PER_KN
    }
}

fn local3(b: &ModularBox, p: Point2, z: f64) -> [f64; 3] {
    let (s, t) = b.local(p);
    [s, t, z]
}

/// What a box weighs, and where.
pub fn box_weight(model: &Model, id: BoxId) -> Option<BoxWeight> {
    let b = model.modular_box(id)?;
    let members = model.box_members(id);
    let mut parts = Vec::new();
    // The chassis, as one part.
    let (mut kg, mut m) = (0.0, [0.0; 3]);
    for x in chassis_members(b) {
        let w = x.weight();
        let c = x.centre();
        kg += w;
        for i in 0..3 {
            m[i] += w * c[i];
        }
    }
    if kg > 0.0 {
        parts.push(WeightPart {
            what: "Chasis".into(),
            kg,
            at: m.map(|v| v / kg),
        });
    }
    for wid in &members.walls {
        let Some(w) = model.wall(*wid) else { continue };
        let openings: Vec<_> = archisim_geom::openings_in_wall(model, w.id);
        let holes: f64 = openings
            .iter()
            .map(|o| (o.t1 - o.t0).max(0.0) * (o.head - o.sill).max(0.0))
            .sum();
        let face = (w.length() * w.height - holes).max(0.0);
        let kn = wall_type_weight(model, w.wall_type) * face;
        let mid = (w.start + w.end) * 0.5;
        let name = model
            .wall_type(w.wall_type)
            .map_or_else(|| "Muro".to_string(), |t| format!("Muro {}", t.name));
        parts.push(WeightPart {
            what: name,
            kg: kn * KG_PER_KN,
            at: local3(b, mid, w.base_offset + w.height / 2.0),
        });
        for o in &openings {
            let area = (o.t1 - o.t0).max(0.0) * (o.head - o.sill).max(0.0);
            let p = o.origin + o.u * ((o.t0 + o.t1) / 2.0);
            parts.push(WeightPart {
                what: "Carpintería".into(),
                kg: OPENING_WEIGHT * area * KG_PER_KN,
                at: local3(b, p, (o.sill + o.head) / 2.0),
            });
        }
    }
    for sid in &members.slabs {
        let Some(s) = model.slab(*sid) else { continue };
        let Some(t) = model.slab_type(s.slab_type) else {
            continue;
        };
        let area = signed_area(&s.outline).abs();
        let kn = slab_type_weight(model, s.slab_type) * area;
        parts.push(WeightPart {
            what: format!("Forjado {}", t.name),
            kg: kn * KG_PER_KN,
            at: local3(
                b,
                archisim_core::centroid(&s.outline),
                s.top_offset - t.thickness() / 2.0,
            ),
        });
    }
    for cid in &members.ceilings {
        let Some(c) = model.ceiling(*cid) else {
            continue;
        };
        let Some(t) = model.slab_type(c.ceiling_type) else {
            continue;
        };
        let area = signed_area(&c.outline).abs();
        let kn = slab_type_weight(model, c.ceiling_type) * area;
        parts.push(WeightPart {
            what: format!("Techo {}", t.name),
            kg: kn * KG_PER_KN,
            at: local3(
                b,
                archisim_core::centroid(&c.outline),
                c.height + t.thickness() / 2.0,
            ),
        });
    }
    let kg: f64 = parts.iter().map(|p| p.kg).sum();
    if kg <= 0.0 {
        return None;
    }
    let mut centre = [0.0; 3];
    for p in &parts {
        for i in 0..3 {
            centre[i] += p.kg * p.at[i] / kg;
        }
    }
    Some(BoxWeight { parts, kg, centre })
}

/// The lifting of a box, checked on its floor frame's long edges.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftCheck {
    pub weight: BoxWeight,
    /// Each lifting point (box axes) and the pull of its sling, kN
    /// (unfactored).
    pub points: Vec<([f64; 3], f64)>,
    /// Bending of the worse edge, kNm (factored: weight × γ × dynamic).
    pub moment: f64,
    /// What the edge's tube takes, kNm.
    pub resistance: f64,
    /// Its deflection while hanging, mm, and the limit (span / 250).
    pub deflection: f64,
    pub deflection_limit: f64,
}

impl LiftCheck {
    pub fn usage(&self) -> f64 {
        self.moment / self.resistance.max(1e-9)
    }

    pub fn passes(&self) -> bool {
        self.usage() <= 1.0 && self.deflection <= self.deflection_limit
    }
}

/// Checks the lifting of a box.
pub fn lift_check(model: &Model, id: BoxId) -> Option<LiftCheck> {
    let b = model.modular_box(id)?;
    let weight = box_weight(model, id)?;
    let w = weight.kn();
    let tube = b.chassis.edge;
    // Each long edge's share, by where the centre is across.
    let across = (weight.centre[1] / b.width).clamp(0.0, 1.0);
    let shares = [w * (1.0 - across), w * across];
    let along = (weight.centre[0] / b.length).clamp(0.0, 1.0);
    let l = b.length;
    let ei = STEEL_E * 1e3 * tube.inertia(); // kN·m²
    let factor = GAMMA_LIFT * DYNAMIC;
    let (moment, deflection, span) = match b.chassis.lift {
        LiftPoints::Four => {
            let q = shares[0].max(shares[1]) / l;
            (
                factor * q * l * l / 8.0,
                5.0 * q * l.powi(4) / (384.0 * ei),
                l,
            )
        }
        LiftPoints::Six => {
            // Two spans: the moment over the middle point.
            let q = shares[0].max(shares[1]) / l;
            let s = l / 2.0;
            (factor * q * s * s / 8.0, q * s.powi(4) / (185.0 * ei), s)
        }
    };
    let resistance = tube.plastic_modulus() * STEEL_FY * 1e3 / GAMMA_M0;
    let pts = lift_points(b);
    let points = match b.chassis.lift {
        LiftPoints::Four => {
            // [A start, A end, C end, C start].
            let pulls = [
                shares[0] * (1.0 - along),
                shares[0] * along,
                shares[1] * along,
                shares[1] * (1.0 - along),
            ];
            pts.into_iter().zip(pulls).collect()
        }
        LiftPoints::Six => {
            // [A start, A middle, A end, C end, C middle, C start]: a
            // continuous beam over three supports, 3/8, 10/8 and 3/8 of
            // each span's load.
            let p = |share: f64| {
                let half = share / 2.0;
                [3.0 / 8.0 * half, 10.0 / 8.0 * half, 3.0 / 8.0 * half]
            };
            let (a, c) = (p(shares[0]), p(shares[1]));
            let pulls = [a[0], a[1], a[2], c[2], c[1], c[0]];
            pts.into_iter().zip(pulls).collect()
        }
    };
    Some(LiftCheck {
        weight,
        points,
        moment,
        resistance,
        deflection: deflection * 1000.0,
        deflection_limit: span / 250.0 * 1000.0,
    })
}

/// The order the boxes are craned onto their supports: floor by floor
/// from the bottom; on each floor, from the box nearest a corner of the
/// whole (the lowest `x + y` of its centre), then each time the box that
/// touches most of those already placed (the nearest, if several), so
/// every box comes against others already there.
pub fn assembly_order(model: &Model) -> Vec<BoxId> {
    let mut levels: Vec<(f64, LevelId)> = model
        .boxes()
        .map(|b| b.level)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|l| (model.level(l).map_or(0.0, |x| x.elevation), l))
        .collect();
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    for (_, level) in levels {
        let mut left: Vec<&ModularBox> = model.boxes_on_level(level).collect();
        let mut placed: Vec<BoxId> = Vec::new();
        while !left.is_empty() {
            let pick = if placed.is_empty() {
                (0..left.len())
                    .min_by(|&i, &j| {
                        let (a, b) = (left[i].centre(), left[j].centre());
                        (a.x + a.y).total_cmp(&(b.x + b.y))
                    })
                    .expect("not empty")
            } else {
                let last = model
                    .modular_box(*placed.last().expect("one"))
                    .expect("placed")
                    .centre();
                let score = |b: &ModularBox| {
                    let touching = model
                        .box_contacts(b.id)
                        .iter()
                        .filter(|c| placed.contains(&c.other))
                        .count();
                    (touching, -(b.centre() - last).length())
                };
                (0..left.len())
                    .max_by(|&i, &j| {
                        let (a, b) = (score(left[i]), score(left[j]));
                        a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
                    })
                    .expect("not empty")
            };
            placed.push(left.remove(pick).id);
        }
        out.extend(placed);
    }
    out
}

fn num(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// The boxes' weights, lifting and assembly order as text, for the
/// report and the command line.
pub fn boxes_report(model: &Model) -> String {
    let order = assembly_order(model);
    if order.is_empty() {
        return "No hay cajas modulares.\n".into();
    }
    let mut t = String::new();
    t.push_str("PESO E IZADO DE LAS CAJAS\n");
    t.push_str(&format!(
        "Izado con balancín (eslingas verticales); peso × {} × {} (dinámico). Tubo S275.\n\
         Cada borde largo del chasis de suelo lleva su parte del peso solo, sin contar el muro que tenga encima \
         como viga: es el lado de la seguridad.\n\n",
        num(GAMMA_LIFT, 2),
        num(DYNAMIC, 2)
    ));
    for (n, id) in order.iter().enumerate() {
        let b = model.modular_box(*id).expect("box");
        t.push_str(&format!(
            "{}º {} {}\n",
            n + 1,
            b.mark,
            if b.kind.is_empty() { "" } else { &b.kind }
        ));
        let Some(c) = lift_check(model, *id) else {
            t.push_str("   sin peso (vacía)\n");
            continue;
        };
        let w = &c.weight;
        t.push_str(&format!(
            "   Peso {} t (chasis {} t); centro de gravedad a {} m a lo largo, {} m de través y {} m de altura\n",
            num(w.tonnes(), 2),
            num(
                w.parts
                    .iter()
                    .filter(|p| p.what == "Chasis")
                    .map(|p| p.kg)
                    .sum::<f64>()
                    / 1000.0,
                2
            ),
            num(w.centre[0], 2),
            num(w.centre[1], 2),
            num(w.centre[2], 2)
        ));
        // By kind: chassis, walls, floors, ceilings, doors and windows.
        let mut kinds: Vec<(String, f64)> = Vec::new();
        for p in &w.parts {
            let kind = p.what.split(' ').next().unwrap_or("").to_string();
            match kinds.iter_mut().find(|k| k.0 == kind) {
                Some(k) => k.1 += p.kg,
                None => kinds.push((kind, p.kg)),
            }
        }
        let kinds: Vec<String> = kinds
            .iter()
            .map(|(k, kg)| format!("{} {} t", k.to_lowercase(), num(kg / 1000.0, 2)))
            .collect();
        t.push_str(&format!("   ({})\n", kinds.join(" · ")));
        let pulls: Vec<String> = c.points.iter().map(|(_, f)| num(*f, 1)).collect();
        t.push_str(&format!(
            "   {} ({}): {} kN por punto\n",
            b.chassis.lift.label(),
            b.chassis.edge.label(),
            pulls.join(" · ")
        ));
        t.push_str(&format!(
            "   Borde del chasis: {} kNm de {} kNm ({} %), flecha {} mm de {} mm → {}\n",
            num(c.moment, 1),
            num(c.resistance, 1),
            (c.usage() * 100.0).round(),
            num(c.deflection, 1),
            num(c.deflection_limit, 1),
            if c.passes() { "cumple" } else { "NO CUMPLE" }
        ));
    }
    t.push_str("\nDESPIECE POR CAJA\n");
    let frames = archisim_geom::framing::members_by_box(model);
    for id in &order {
        let b = model.modular_box(*id).expect("box");
        t.push_str(&format!("{}\n", b.mark));
        for l in chassis_cut_list(b) {
            t.push_str(&format!(
                "   {} {} · {} × {} mm · {} kg\n",
                l.role.label(),
                l.tube.label(),
                l.count,
                l.length_mm,
                num(l.kg, 1)
            ));
        }
        if let Some(members) = frames.get(id) {
            let list = archisim_geom::framing::cut_list(model, members);
            for s in &list.totals {
                t.push_str(&format!(
                    "   Entramado {} · {} piezas · {} m · {} kg\n",
                    s.section,
                    s.pieces,
                    num(s.metres, 1),
                    num(s.kg, 1)
                ));
            }
        }
    }
    t
}

/// A line of a chassis' cut list: pieces of the same role, tube and length.
#[derive(Debug, Clone, PartialEq)]
pub struct ChassisCut {
    pub role: archisim_geom::boxes::ChassisRole,
    pub tube: archisim_core::BoxTube,
    pub length_mm: i64,
    pub count: usize,
    pub kg: f64,
}

/// The cut list of a box's chassis, by role, tube and length.
pub fn chassis_cut_list(b: &ModularBox) -> Vec<ChassisCut> {
    let mut out: Vec<ChassisCut> = Vec::new();
    for m in chassis_members(b) {
        let mm = (m.length() * 1000.0).round() as i64;
        match out
            .iter_mut()
            .find(|c| c.role == m.role && c.tube == m.tube && c.length_mm == mm)
        {
            Some(c) => {
                c.count += 1;
                c.kg += m.weight();
            }
            None => out.push(ChassisCut {
                role: m.role,
                tube: m.tube,
                length_mm: mm,
                count: 1,
                kg: m.weight(),
            }),
        }
    }
    out.sort_by(|a, b| a.role.cmp(&b.role).then(b.length_mm.cmp(&a.length_mm)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewBox, SideState, SlabFunction, WallFunction,
        BOX_PARAPET, BOX_WIDTH,
    };

    fn houses() -> (Model, BoxId, BoxId) {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let board = m.add_material("Placa de yeso", "board", [230; 3], CutPattern::Solid);
        let layer = |t| Layer {
            material: board,
            thickness: t,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("Muro", WallFunction::Exterior, vec![layer(0.10)])
            .unwrap();
        let ft = m
            .add_slab_type("Suelo", SlabFunction::Floor, vec![layer(0.05)])
            .unwrap();
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, BOX_WIDTH + 0.02), 0.0))
            .unwrap();
        m.set_box_sides_from_neighbours(a).unwrap();
        m.set_box_sides_from_neighbours(b).unwrap();
        for id in [a, b] {
            m.fill_box(
                id,
                archisim_core::BoxFill {
                    wall_type: Some(wt),
                    floor_type: Some(ft),
                    roof_type: None,
                    ceiling_type: None,
                    ceiling_height: 2.6,
                    parapet: BOX_PARAPET,
                },
            )
            .unwrap();
        }
        (m, a, b)
    }

    #[test]
    fn weight_and_centre() {
        let (m, a, b) = houses();
        let wa = box_weight(&m, a).unwrap();
        let wb = box_weight(&m, b).unwrap();
        // A carries the joint's wall: heavier and balanced across; B,
        // open on A, leans towards its wall on C.
        assert!(wa.kg > wb.kg, "{} {}", wa.kg, wb.kg);
        assert!(
            (wa.centre[1] - BOX_WIDTH / 2.0).abs() < 0.05,
            "{:?}",
            wa.centre
        );
        assert!(wb.centre[1] > BOX_WIDTH / 2.0 + 0.1, "{:?}", wb.centre);
        assert!((wa.centre[0] - 3.1).abs() < 0.05, "{:?}", wa.centre);
        assert!(wa.parts.iter().any(|p| p.what == "Chasis"));
    }

    #[test]
    fn lifting_from_four_and_six_points() {
        let (mut m, a, b) = houses();
        let four = lift_check(&m, a).unwrap();
        assert_eq!(four.points.len(), 4);
        let total: f64 = four.points.iter().map(|p| p.1).sum();
        assert!((total - four.weight.kn()).abs() < 1e-6);
        m.edit_box(a, |x| x.chassis.lift = LiftPoints::Six).unwrap();
        let six = lift_check(&m, a).unwrap();
        assert_eq!(six.points.len(), 6);
        let total: f64 = six.points.iter().map(|p| p.1).sum();
        assert!((total - six.weight.kn()).abs() < 1e-6);
        // Half the span, a quarter of the moment.
        assert!((six.moment / four.moment - 0.25).abs() < 0.01);
        assert!(six.usage() < four.usage());
        // The order: the first one, then the one against it.
        assert_eq!(assembly_order(&m), vec![a, b]);
        let text = boxes_report(&m);
        assert!(text.contains("1º MOD-01"), "{text}");
        assert!(text.contains("2º MOD-02"));
        assert!(
            text.contains("Pilar de esquina 120×100×4 · 4 × 3580 mm"),
            "{text}"
        );
        let _ = SideState::Free;
        let _ = b;
    }
}
