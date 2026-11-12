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

/// A box's floor frame standing on its supports (`docs/MODULAR.md` §5):
/// each long edge of the chassis carries half the box finished, with its
/// use, as a continuous beam on its corner and middle supports (two spans
/// of half the length). The floor itself (deck and joists) spans across
/// between the edges and is the floor type's own (its maker's tables).
#[derive(Debug, Clone, PartialEq)]
pub struct InPlaceCheck {
    /// Factored line load on the worse edge, kN/m.
    pub load: f64,
    /// Span between supports, m.
    pub span: f64,
    /// Bending over the middle support, kNm (factored), and what the
    /// tube takes.
    pub moment: f64,
    pub resistance: f64,
    /// Deflection in service, mm, and the limit (span / 300).
    pub deflection: f64,
    pub deflection_limit: f64,
}

impl InPlaceCheck {
    pub fn usage(&self) -> f64 {
        self.moment / self.resistance.max(1e-9)
    }

    pub fn passes(&self) -> bool {
        self.usage() <= 1.0 && self.deflection <= self.deflection_limit
    }
}

/// Checks a box's floor frame on its supports (only the boxes on the
/// ground: one on another rests along its edges).
pub fn in_place_check(model: &Model, id: BoxId) -> Option<InPlaceCheck> {
    let b = model.modular_box(id)?;
    if model.box_below(id).is_some() {
        return None;
    }
    let weight = box_weight(model, id)?;
    let area = b.length * b.width;
    let (above_dead, above_use) = stack_above(model, id);
    let dead = weight.kn() + above_dead;
    let used = BOX_USE_LOAD * area + above_use;
    let across = (weight.centre[1] / b.width).clamp(0.0, 1.0);
    let share = across.max(1.0 - across);
    let total = GAMMA_G * dead * share + GAMMA_Q * used / 2.0;
    let service = dead * share + used / 2.0;
    let span = b.length / 2.0;
    let q = total / b.length;
    let qs = service / b.length;
    let tube = b.chassis.edge;
    let ei = STEEL_E * 1e3 * tube.inertia();
    Some(InPlaceCheck {
        load: q,
        span,
        moment: q * span * span / 8.0,
        resistance: tube.plastic_modulus() * STEEL_FY * 1e3 / GAMMA_M0,
        // Two equal spans: 2,08 qL⁴/384EI at most.
        deflection: 2.08 * qs * span.powi(4) / (384.0 * ei) * 1000.0,
        deflection_limit: span / 300.0 * 1000.0,
    })
}

/// The weight and use of the boxes stacked over one, kN.
fn stack_above(model: &Model, id: BoxId) -> (f64, f64) {
    let mut dead = 0.0;
    let mut used = 0.0;
    for up in model.boxes_above(id) {
        if let (Some(w), Some(b)) = (box_weight(model, up), model.modular_box(up)) {
            dead += w.kn();
            used += BOX_USE_LOAD * b.length * b.width;
        }
        let (d, u) = stack_above(model, up);
        dead += d;
        used += u;
    }
    (dead, used)
}

/// The usual edge tubes of a chassis, lightest first (depth × width ×
/// wall, mm): what [`suggest_edge`] picks from.
pub const EDGE_TUBES: [(f64, f64, f64); 7] = [
    (120.0, 100.0, 4.0),
    (120.0, 100.0, 5.0),
    (150.0, 100.0, 5.0),
    (160.0, 80.0, 6.0),
    (200.0, 100.0, 5.0),
    (200.0, 100.0, 6.3),
    (250.0, 150.0, 6.3),
];

/// The lightest edge tube with which a box both hangs from its slings and
/// stands on its supports.
pub fn suggest_edge(model: &Model, id: BoxId) -> Option<archisim_core::BoxTube> {
    let mut m = model.clone();
    for (d, w, t) in EDGE_TUBES {
        let tube = archisim_core::BoxTube::mm(d, w, t);
        m.edit_box(id, |x| x.chassis.edge = tube).ok()?;
        let lifts = lift_check(&m, id).is_none_or(|c| c.passes());
        let stands = in_place_check(&m, id).is_none_or(|c| c.passes());
        if lifts && stands {
            return Some(tube);
        }
    }
    None
}

/// The checks of every box's chassis, for the calculation's list.
pub fn chassis_checks(model: &Model) -> Vec<crate::lightframe::Check> {
    use crate::lightframe::{Check, Status, Subject};
    let mut out = Vec::new();
    for b in model.boxes() {
        let edge = b.chassis.edge;
        if let Some(c) = in_place_check(model, b.id) {
            out.push(Check {
                subject: Subject::Building,
                what: format!(
                    "Chasis de {} sobre sus apoyos: borde {} en vanos de {} m con {} kN/m, {} / {} kNm; flecha {} / {} mm",
                    b.mark,
                    edge.label(),
                    num(c.span, 2),
                    num(c.load, 1),
                    num(c.moment, 1),
                    num(c.resistance, 1),
                    num(c.deflection, 1),
                    num(c.deflection_limit, 1)
                ),
                rule: "Chasis de la caja (DB-SE-A, 6.2.6)",
                ratio: c.usage().max(c.deflection / c.deflection_limit.max(1e-9)),
                status: if c.passes() { Status::Pass } else { Status::Fail },
                advice: (!c.passes()).then(|| "un borde de más canto".into()),
            });
        }
        if let Some(c) = lift_check(model, b.id) {
            out.push(Check {
                subject: Subject::Building,
                what: format!(
                    "Izado de {}: borde {}, {} / {} kNm; flecha {} / {} mm",
                    b.mark,
                    edge.label(),
                    num(c.moment, 1),
                    num(c.resistance, 1),
                    num(c.deflection, 1),
                    num(c.deflection_limit, 1)
                ),
                rule: "Izado de la caja (DB-SE-A, 6.2.6)",
                ratio: c.usage(),
                status: if c.passes() {
                    Status::Pass
                } else {
                    Status::Fail
                },
                advice: (!c.passes())
                    .then(|| "seis puntos de izado o un borde de más canto".into()),
            });
        }
    }
    out
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

/// A lift of the crane: a box, in its order, at its reach.
#[derive(Debug, Clone, PartialEq)]
pub struct CraneLift {
    pub id: BoxId,
    /// 1 for the first box up.
    pub order: usize,
    /// Its centre of gravity, in plan.
    pub at: Point2,
    /// From the crane's centre, m.
    pub radius: f64,
    /// On the hook: the box and the rigging, t.
    pub load: f64,
    /// What the crane lifts at that reach, t.
    pub capacity: f64,
}

impl CraneLift {
    pub fn usage(&self) -> f64 {
        if self.capacity <= 0.0 {
            f64::INFINITY
        } else {
            self.load / self.capacity
        }
    }

    pub fn passes(&self) -> bool {
        self.usage() <= 1.0
    }
}

/// The crane plan: where it stands (or where it would, if it has not been
/// placed) and each box's lift in the assembly order.
#[derive(Debug, Clone, PartialEq)]
pub struct CranePlan {
    pub at: Point2,
    /// It was not placed: this is the middle of the boxes.
    pub suggested: bool,
    pub chart: String,
    pub lifts: Vec<CraneLift>,
}

impl CranePlan {
    pub fn worst(&self) -> Option<&CraneLift> {
        self.lifts
            .iter()
            .max_by(|a, b| a.usage().total_cmp(&b.usage()))
    }
}

/// Where the crane would stand if it has not been placed: the middle of
/// the boxes' centres of gravity (the smallest reach to all of them).
pub fn suggested_crane(model: &Model) -> Option<Point2> {
    let pts: Vec<Point2> = model
        .boxes()
        .filter_map(|b| box_weight(model, b.id).map(|w| centre_in_plan(b, &w)))
        .collect();
    if pts.is_empty() {
        return None;
    }
    let (lo, hi) = pts.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    Some((lo + hi) * 0.5)
}

fn centre_in_plan(b: &ModularBox, w: &BoxWeight) -> Point2 {
    b.origin + b.along() * w.centre[0] + b.across() * w.centre[1]
}

/// The crane plan of the project's boxes (none without boxes).
pub fn crane_plan(model: &Model) -> Option<CranePlan> {
    let site = model.modular_site();
    let (at, suggested) = match site.crane {
        Some(p) => (p, false),
        None => (suggested_crane(model)?, true),
    };
    let lifts = assembly_order(model)
        .into_iter()
        .enumerate()
        .filter_map(|(n, id)| {
            let b = model.modular_box(id)?;
            let w = box_weight(model, id)?;
            let c = centre_in_plan(b, &w);
            let radius = (c - at).length();
            Some(CraneLift {
                id,
                order: n + 1,
                at: c,
                radius,
                load: w.tonnes() + site.rigging,
                capacity: site.crane_chart.capacity(radius),
            })
        })
        .collect();
    Some(CranePlan {
        at,
        suggested,
        chart: site.crane_chart.name.clone(),
        lifts,
    })
}

/// Use load on each box's floor (housing, DB-SE-AE category A), kN/m².
pub const BOX_USE_LOAD: f64 = 2.0;
/// Partial factors of the supports' check.
pub const GAMMA_G: f64 = 1.35;
pub const GAMMA_Q: f64 = 1.50;
/// Concrete's unit weight, kN/m³.
const CONCRETE: f64 = 25.0;

/// What a support carries and whether its pier and its plate are enough
/// (M10). The boxes stacked on a box on the ground come down through it.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportLoad {
    pub mark: String,
    pub at: Point2,
    /// Top of the plate over project zero.
    pub z: f64,
    /// Permanent and use loads, kN, without factors.
    pub g: f64,
    pub q: f64,
    /// Pier side, m (the one built, or the usual one).
    pub pier: f64,
    /// The steel plate on it, «250×250×15», and its bolts.
    pub plate: String,
    pub bolts: String,
    /// Design load, kN, and the pier's stress and limit, MPa.
    pub design: f64,
    pub stress: f64,
    pub limit: f64,
    /// The boxes it carries.
    pub boxes: Vec<BoxId>,
}

impl SupportLoad {
    pub fn passes(&self) -> bool {
        self.stress <= self.limit + 1e-9
    }
}

/// The base's check: each support and the ground under the slab.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseCheck {
    pub supports: Vec<SupportLoad>,
    /// Slab area, m², and the pressure on the ground and what it admits,
    /// kN/m² (service loads and the slab and plinth's own weight).
    pub area: f64,
    pub pressure: f64,
    pub bearing: f64,
}

impl BaseCheck {
    pub fn ground_passes(&self) -> bool {
        self.pressure <= self.bearing + 1e-9
    }
}

/// The weight of a box with every box stacked on it, kN, and the floor
/// area under use, m².
fn stack(model: &Model, id: BoxId, seen: &mut Vec<BoxId>) -> (f64, f64) {
    if seen.contains(&id) {
        return (0.0, 0.0);
    }
    seen.push(id);
    let Some(b) = model.modular_box(id) else {
        return (0.0, 0.0);
    };
    let own = box_weight(model, id).map_or(0.0, |w| w.kg) / KG_PER_KN;
    let (mut g, mut area) = (own, b.length * b.width);
    for up in model.boxes_above(id) {
        let (ug, ua) = stack(model, up, seen);
        g += ug;
        area += ua;
    }
    (g, area)
}

/// What each support of the boxes on the ground carries (M10).
pub fn base_check(model: &Model) -> Option<BaseCheck> {
    let supports = model.box_supports();
    if supports.is_empty() {
        return None;
    }
    let site = model.structural_site();
    let fck = site.materials.concrete_fck;
    let limit = 0.85 * fck / 1.5;
    let base = model
        .levels()
        .into_iter()
        .find(|l| l.name == archisim_core::BASE_LEVEL)
        .map(|l| l.id);
    let defaults = archisim_core::BaseOptions::default();
    let mut stacks: std::collections::BTreeMap<BoxId, (f64, f64)> = Default::default();
    let mut out = Vec::new();
    let (mut total_g, mut total_q) = (0.0, 0.0);
    for s in &supports {
        let (mut g, mut q) = (0.0, 0.0);
        for (id, share) in &s.boxes {
            let (sg, sa) = *stacks
                .entry(*id)
                .or_insert_with(|| stack(model, *id, &mut Vec::new()));
            g += sg * share;
            q += BOX_USE_LOAD * sa * share;
        }
        total_g += g;
        total_q += q;
        // The pier built there, if there is one.
        let built = base.and_then(|lv| {
            model
                .columns()
                .filter(|c| c.level == lv && c.position.distance(s.at) < 0.05)
                .find_map(|c| match model.member_type(c.member_type)?.profile {
                    archisim_core::Profile::Rectangle { width, depth } => Some(width.min(depth)),
                    _ => None,
                })
        });
        let pier = built.unwrap_or(if s.shared() {
            defaults.shared_pier
        } else {
            defaults.pier
        });
        let design = GAMMA_G * g + GAMMA_Q * q;
        let stress = design / (pier * pier) / 1000.0;
        // Two bolts per box it carries, on a plate that takes them.
        let plate = match s.boxes.len() {
            1 => "250×250×15",
            2 => "400×250×15",
            _ => "400×400×15",
        };
        let bolts = format!("{} pernos M16", 2 * s.boxes.len());
        out.push(SupportLoad {
            mark: s.mark.clone(),
            at: s.at,
            z: s.z,
            g,
            q,
            pier,
            plate: plate.into(),
            bolts,
            design,
            stress,
            limit,
            boxes: s.boxes.iter().map(|(id, _)| *id).collect(),
        });
    }
    // The slab (the one built, or the outline it would have).
    let (mut area, mut own) = (0.0, 0.0);
    let built: Vec<_> = base
        .map(|lv| model.slabs().filter(|sl| sl.level == lv).collect())
        .unwrap_or_default();
    if built.is_empty() {
        for o in model.ground_box_outlines(defaults.margin) {
            let a = signed_area(&o).abs();
            area += a;
            own += a * defaults.slab * CONCRETE;
        }
    } else {
        for sl in built {
            let a = signed_area(&sl.outline).abs();
            area += a;
            own += a * slab_type_weight(model, sl.slab_type);
        }
    }
    if let Some(lv) = base {
        for w in model.walls().filter(|w| w.level == lv) {
            own += wall_type_weight(model, w.wall_type) * w.start.distance(w.end) * w.height;
        }
    }
    let pressure = if area > 0.0 {
        (total_g + total_q + own) / area
    } else {
        0.0
    };
    Some(BaseCheck {
        supports: out,
        area,
        pressure,
        bearing: site.bearing,
    })
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
    if let Some(plan) = crane_plan(model) {
        t.push_str(&format!(
            "\nPLAN DE GRÚA\n{} en ({}, {}){}; con {} t de aparejo.\n",
            plan.chart,
            num(plan.at.x, 2),
            num(plan.at.y, 2),
            if plan.suggested {
                " (propuesta: no se ha colocado; en el centro de las cajas)"
            } else {
                ""
            },
            num(model.modular_site().rigging, 2)
        ));
        for l in &plan.lifts {
            let b = model.modular_box(l.id).expect("box");
            t.push_str(&format!(
                "{}º {}: radio {} m · carga {} t · admite {} t ({} %) → {}\n",
                l.order,
                b.mark,
                num(l.radius, 1),
                num(l.load, 2),
                num(l.capacity, 2),
                (l.usage() * 100.0).round(),
                if l.passes() { "cumple" } else { "NO CUMPLE" }
            ));
        }
    }
    if let Some(base) = base_check(model) {
        t.push_str(&format!(
            "\nAPOYOS (OBRA PREVIA)\nPeso de las cajas con las que tienen encima; uso {} kN/m²; {} G + {} Q. \
             Pilar de hormigón de fck {} MPa: hasta 0,85 fck / 1,5.\n",
            num(BOX_USE_LOAD, 1),
            num(GAMMA_G, 2),
            num(GAMMA_Q, 2),
            num(model.structural_site().materials.concrete_fck, 0)
        ));
        for s in &base.supports {
            let marks: Vec<&str> = s
                .boxes
                .iter()
                .filter_map(|b| model.modular_box(*b).map(|b| b.mark.as_str()))
                .collect();
            t.push_str(&format!(
                "{} ({}): G {} kN · Q {} kN · pilar {}×{} · {} MPa de {} · placa {}, {} → {}\n",
                s.mark,
                marks.join(", "),
                num(s.g, 1),
                num(s.q, 1),
                (s.pier * 100.0).round(),
                (s.pier * 100.0).round(),
                num(s.stress, 2),
                num(s.limit, 2),
                s.plate,
                s.bolts,
                if s.passes() { "cumple" } else { "NO CUMPLE" }
            ));
        }
        t.push_str(&format!(
            "Terreno: {} kN/m² en {} m² de losa, admite {} → {}\n",
            num(base.pressure, 1),
            num(base.area, 1),
            num(base.bearing, 0),
            if base.ground_passes() {
                "cumple"
            } else {
                "NO CUMPLE"
            }
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
        // The crane: in the middle when it has not been placed; far away,
        // too far.
        let plan = crane_plan(&m).unwrap();
        assert!(plan.suggested);
        assert_eq!(plan.lifts.len(), 2);
        assert!(plan.lifts.iter().all(|l| l.passes()), "{plan:?}");
        let mut site = m.modular_site();
        site.crane = Some(Point2::new(60.0, 0.0));
        m.set_modular_site(site).unwrap();
        let plan = crane_plan(&m).unwrap();
        assert!(!plan.suggested);
        assert!(plan.lifts.iter().all(|l| !l.passes() && l.radius > 50.0));
        assert_eq!(plan.worst().unwrap().capacity, 0.0);
        let text = boxes_report(&m);
        assert!(text.contains("PLAN DE GRÚA"), "{text}");
        assert!(text.contains("1º MOD-01"), "{text}");
        assert!(text.contains("2º MOD-02"));
        assert!(
            text.contains("Pilar de esquina 120×100×4 · 4 × 3580 mm"),
            "{text}"
        );
        let _ = SideState::Free;
        let _ = b;
    }

    #[test]
    fn the_supports_carry_the_boxes_and_what_is_on_them() {
        let (mut m, a, _) = houses();
        let up = m.add_level("P1", 3.6);
        m.add_box(NewBox::usual(up, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let before = base_check(&m).unwrap();
        // Two boxes side by side: nine supports, three shared.
        assert_eq!(before.supports.len(), 9);
        assert_eq!(
            before
                .supports
                .iter()
                .filter(|s| s.boxes.len() == 2)
                .count(),
            3
        );
        // All the weight comes down: the boxes' and the one on top.
        let g: f64 = before.supports.iter().map(|s| s.g).sum();
        let weight: f64 = m
            .boxes()
            .filter_map(|b| box_weight(&m, b.id))
            .map(|w| w.kg / KG_PER_KN)
            .sum();
        assert!((g - weight).abs() < 1e-6 * weight, "{g} {weight}");
        // Under box A, which has a box on it, more than under the other.
        let under = |id: BoxId| -> f64 {
            before
                .supports
                .iter()
                .filter(|s| s.boxes == vec![id])
                .map(|s| s.g)
                .sum()
        };
        let other = m
            .boxes()
            .find(|b| b.id != a && b.level == m.modular_box(a).unwrap().level)
            .unwrap()
            .id;
        assert!(under(a) > under(other));
        // Built: its piers are the ones checked, and the slab weighs.
        m.build_box_base(&archisim_core::BaseOptions::default())
            .unwrap();
        let after = base_check(&m).unwrap();
        assert!(after.supports.iter().all(|s| s.passes()));
        assert!(
            after.pressure > 0.0 && after.ground_passes(),
            "{}",
            after.pressure
        );
        assert!(boxes_report(&m).contains("APOYOS (OBRA PREVIA)"));
    }
}
