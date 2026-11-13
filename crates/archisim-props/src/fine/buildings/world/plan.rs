//! A building of the world over any plan: a polygon whose sides are the
//! street's front, the back, free sides or party walls. Each side is a
//! front of the skin; the core fills the polygon; the roof is flat over
//! the whole polygon or pitched over the rectangles that make it up (an
//! L is two roofs crossing, as built).

use super::super::*;
use super::facade::{front, Front};
use super::roofs::{self, faces, RoofPlan};
use super::skins::*;
use super::Look;
use crate::build::Builder;

/// What a side of the plan is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    /// The main front on the street: shops, balconies, the entrance.
    Street,
    /// A second dressed front (the back of a block): balconies, a door.
    Rear,
    /// A free side: windows.
    Free,
    /// The back of a house of the street: windows and a door.
    Back,
    /// Against the neighbour: a blind wall.
    Party,
}

/// A plan: a polygon (anticlockwise or not) and what each side is, the
/// side `i` going from point `i` to point `i + 1`.
#[derive(Clone, Debug)]
pub struct Plan {
    pub poly: Vec<[f64; 2]>,
    pub sides: Vec<Side>,
}

impl Plan {
    /// A rectangle `w` × `d` centred on x, from y = 0 (the back) to y = d
    /// (the street), its sides back, right, street, left.
    pub fn rect(w: f64, d: f64, sides: [Side; 4]) -> Plan {
        Plan {
            poly: vec![[-w / 2.0, 0.0], [w / 2.0, 0.0], [w / 2.0, d], [-w / 2.0, d]],
            sides: sides.to_vec(),
        }
    }

    /// The same plan anticlockwise.
    pub fn ccw(&self) -> Plan {
        if area(&self.poly) >= 0.0 {
            return self.clone();
        }
        let n = self.poly.len();
        let poly: Vec<[f64; 2]> = self.poly.iter().rev().copied().collect();
        let sides = (0..n).map(|k| self.sides[(2 * n - 2 - k) % n]).collect();
        Plan { poly, sides }
    }
}

/// How the typology wants its building.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub floors: usize,
    pub seed: u64,
    pub light: bool,
    pub roof: Roof,
    pub cover: Cover,
    pub pitch: f64,
    pub eaves: f64,
    pub gable: Gable,
    pub top: Top,
    pub extras: u32,
    /// A block: the box of its stair on a flat roof.
    pub block: bool,
    /// The pavement along the street.
    pub pavement: bool,
    /// Free sides with windows on every other axis.
    pub sparse_free: bool,
    /// The ground floor over the ground, its steps, on stilts.
    pub lift: Option<f64>,
    pub steps: bool,
    pub open_below: bool,
}

/// What a building over a plan tells its typology.
pub struct Built {
    pub top: f64,
    /// Each side's front (none for party walls).
    pub fronts: Vec<Option<Front>>,
    /// Each side's axes of windows.
    pub axes: Vec<Vec<f64>>,
}

/// The face of side `i` of an anticlockwise polygon, seen from outside.
pub fn face_of(poly: &[[f64; 2]], i: usize) -> Face {
    let a = poly[i];
    let e = poly[(i + 1) % poly.len()];
    let len = ((a[0] - e[0]).powi(2) + (a[1] - e[1]).powi(2))
        .sqrt()
        .max(1e-6);
    Face {
        o: e,
        u: [(a[0] - e[0]) / len, (a[1] - e[1]) / len],
        len,
    }
}

/// Builds a building of skin `sk` over `plan`.
pub fn building(b: &mut Builder, plan: &Plan, sk: &Skin, l: &Look, o: &Shape) -> Built {
    let plan = plan.ccw();
    let poly = &plan.poly;
    let n = poly.len();
    let mut fronts: Vec<Option<Front>> = Vec::with_capacity(n);
    let mut axes = vec![Vec::new(); n];
    let mut top = 0.0;
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        let f = face_of(poly, i);
        let side = plan.sides[i];
        let prev = plan.sides[(i + n - 1) % n];
        let next = plan.sides[(i + 1) % n];
        let mut base = Front::of(f, sk, o.floors, o.seed + i as u64 * 7);
        base.light = o.light;
        base.party = prev == Side::Party || next == Side::Party;
        if let Some(lift) = o.lift {
            base.lift = lift;
        }
        base.steps = o.steps;
        base.open_below = o.open_below;
        let recess_ok = prev == Side::Party && next == Side::Party;
        if matches!(base.ground, Ground::Arcade | Ground::Pilotis) && (!recess_ok || o.floors < 2) {
            base.ground = if base.ground == Ground::Arcade {
                Ground::Shop
            } else {
                Ground::Door
            };
            base.recess = 0.0;
        }
        if base.ground == Ground::Pilotis {
            base.recess = base.recess.min(f.len.max(4.0));
        }
        top = base.top();
        let fr = match side {
            Side::Party => None,
            _ if f.len < 2.2 => None,
            Side::Street => Some(base),
            Side::Rear => Some(Front {
                ground: Ground::Door,
                recess: 0.0,
                extras: base.extras & !(SIGNS | AWNINGS),
                balc: if base.balc == Balc::FireEscape {
                    Balc::None
                } else {
                    base.balc
                },
                ..base
            }),
            Side::Free => Some(base.plain_of(f, false, o.sparse_free)),
            Side::Back => Some(base.plain_of(f, true, false)),
        };
        match fr {
            Some(fr) => {
                let made = front(b, &fr, sk, l);
                axes[i] = made.axes;
            }
            None => {
                let from = if o.open_below { base.lift } else { 0.0 };
                wall(b, l.wall, &f, (from, top), 0.3, &[]);
            }
        }
        if side == Side::Street && o.pavement {
            f.bx(b, "paving", (0.0, f.len), (0.0, 0.03), (0.0, 2.2));
        }
        fronts.push(fr);
    }
    // The core.
    let thick = 0.3;
    let from = if o.open_below {
        o.lift.unwrap_or(0.0)
    } else {
        0.0
    };
    let recessed: Vec<f64> = fronts
        .iter()
        .map(|f| match f {
            Some(f) if f.recess > 0.0 && !f.plain => f.recess,
            _ => 0.0,
        })
        .collect();
    let all = vec![thick; n];
    if recessed.iter().any(|r| *r > 0.0) {
        let zg = fronts.iter().flatten().next().map_or(3.0, |f| f.z(1) - 0.3);
        let ground: Vec<f64> = recessed.iter().map(|r| thick + r).collect();
        prism_poly(b, "interior", &inset(poly, &ground), from, zg);
        prism_poly(b, "interior", &inset(poly, &all), zg, top);
    } else {
        prism_poly(b, "interior", &inset(poly, &all), from, top);
    }
    if o.open_below {
        // The floor the walls stand on, and posts under its corners.
        let lift = o.lift.unwrap_or(0.0);
        prism_poly(b, l.door, poly, lift - 0.25, lift);
        for p in inset(poly, &vec![0.25; n]) {
            b.rbox(
                "c_concrete",
                [p[0] - 0.12, p[1] - 0.12, 0.0],
                [p[0] + 0.12, p[1] + 0.12, lift - 0.25],
                0.0,
            );
        }
    }
    roof_over(b, &plan, sk, l, o, top, &axes);
    Built { top, fronts, axes }
}

/// The roof over the plan.
fn roof_over(
    b: &mut Builder,
    plan: &Plan,
    sk: &Skin,
    l: &Look,
    o: &Shape,
    top: f64,
    axes: &[Vec<f64>],
) {
    let poly = &plan.poly;
    if o.roof == Roof::Flat {
        flat_roof(b, plan, l, o, top);
        return;
    }
    let theta = dominant(poly);
    let (sn, cs) = theta.sin_cos();
    let to_frame = |p: [f64; 2]| [p[0] * cs + p[1] * sn, -p[0] * sn + p[1] * cs];
    let to_world = |p: [f64; 2]| [p[0] * cs - p[1] * sn, p[0] * sn + p[1] * cs];
    let fpoly: Vec<[f64; 2]> = poly.iter().map(|p| to_frame(*p)).collect();
    let (rects, rest) = rect_cover(&fpoly);
    // Bits too small for a roof: flat.
    for (lo, hi) in rest {
        let pts: Vec<[f64; 2]> = [
            [lo[0], lo[1]],
            [hi[0], lo[1]],
            [hi[0], hi[1]],
            [lo[0], hi[1]],
        ]
        .iter()
        .map(|p| to_world(*p))
        .collect();
        prism_poly(b, l.roof, &pts, top - 0.1, top + 0.05);
    }
    let kind_at = |p: [f64; 2]| -> Option<Side> {
        let mut best = (0.3, None);
        for i in 0..fpoly.len() {
            let d = seg_dist(p, fpoly[i], fpoly[(i + 1) % fpoly.len()]);
            if d < best.0 {
                best = (d, Some(plan.sides[i]));
            }
        }
        best.1
    };
    let many_dormers = axes.iter().map(|a| a.len()).max().unwrap_or(0);
    for (k, (lo, hi)) in rects.iter().enumerate() {
        // The four sides: top (y hi), right, bottom, left, each what it
        // lies on.
        let mid = [
            [(lo[0] + hi[0]) / 2.0, hi[1]],
            [hi[0], (lo[1] + hi[1]) / 2.0],
            [(lo[0] + hi[0]) / 2.0, lo[1]],
            [lo[0], (lo[1] + hi[1]) / 2.0],
        ];
        let on: Vec<Option<Side>> = (0..4)
            .map(|s| {
                let (a, e) = match s {
                    0 => ([lo[0], hi[1]], [hi[0], hi[1]]),
                    1 => ([hi[0], lo[1]], [hi[0], hi[1]]),
                    2 => ([lo[0], lo[1]], [hi[0], lo[1]]),
                    _ => ([lo[0], lo[1]], [lo[0], hi[1]]),
                };
                let samples =
                    [0.25, 0.5, 0.75].map(|t| [a[0] + (e[0] - a[0]) * t, a[1] + (e[1] - a[1]) * t]);
                let kinds: Vec<Option<Side>> = samples.iter().map(|p| kind_at(*p)).collect();
                if kinds.contains(&Some(Side::Party)) {
                    Some(Side::Party)
                } else if kinds.contains(&Some(Side::Street)) {
                    Some(Side::Street)
                } else {
                    kinds.into_iter().flatten().next()
                }
            })
            .collect();
        let _ = mid;
        let party = |s: usize| on[s] == Some(Side::Party);
        let street = |s: usize| on[s] == Some(Side::Street);
        let (w, d) = (hi[0] - lo[0], hi[1] - lo[1]);
        // Which way the local front (y up) points: 0 top, 1 right, 2
        // bottom, 3 left; and the form.
        let mut form = o.roof;
        let front_side: usize;
        if o.roof == Roof::GableToStreet {
            front_side = (0..4)
                .find(|s| street(*s))
                .unwrap_or(if w > d { 0 } else { 1 });
        } else if (party(1) && party(3)) || (party(0) && party(2)) {
            // Between two party walls: the ridge from one to the other.
            front_side = if party(1) {
                if street(2) {
                    2
                } else {
                    0
                }
            } else if street(3) {
                3
            } else {
                1
            };
            if matches!(form, Roof::Hip | Roof::Asian) {
                form = Roof::Gable;
            }
        } else if let Some(p) = (0..4).find(|s| party(*s)) {
            let long = if p % 2 == 0 { w >= d } else { d >= w };
            if long || form == Roof::Mansard {
                // Against a long party wall: one slope away from it.
                front_side = (p + 2) % 4;
                if form != Roof::Mansard {
                    form = Roof::Pent;
                }
            } else {
                // A party wall at one end: a gable there.
                front_side = if p % 2 == 1 {
                    if street(2) {
                        2
                    } else {
                        0
                    }
                } else if street(3) {
                    3
                } else {
                    1
                };
                if matches!(form, Roof::Hip | Roof::Asian) {
                    form = Roof::Gable;
                }
            }
        } else {
            // Free: the ridge along the longer side, the front to the
            // street if the street is along it.
            let along_x = w >= d;
            front_side = if along_x {
                if street(2) {
                    2
                } else {
                    0
                }
            } else if street(3) {
                3
            } else {
                1
            };
        }
        // The local frame: x along the ridge, y towards the front.
        let (ax, ay) = match front_side {
            0 => ([1.0, 0.0], [0.0, 1.0]),
            1 => ([0.0, 1.0], [1.0, 0.0]),
            2 => ([-1.0, 0.0], [0.0, -1.0]),
            _ => ([0.0, -1.0], [-1.0, 0.0]),
        };
        let (len, depth) = if front_side.is_multiple_of(2) {
            (w, d)
        } else {
            (d, w)
        };
        let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let ends_party = if front_side.is_multiple_of(2) {
            party(1) || party(3)
        } else {
            party(0) || party(2)
        };
        let dormers: Vec<f64> = if many_dormers > 0 && len > 4.0 {
            let m = (len / sk.bay).round().max(1.0) as usize;
            (0..m)
                .map(|i| -len / 2.0 + len * (i as f64 + 0.5) / m as f64)
                .collect()
        } else {
            Vec::new()
        };
        let start = b.mark();
        roofs::roof(
            b,
            &RoofPlan {
                x: (-len / 2.0, len / 2.0),
                y: (0.0, depth),
                z: top,
                form,
                cover: o.cover,
                pitch: o.pitch,
                eaves: o.eaves,
                party: ends_party,
                dormers,
                gable: o.gable,
                seed: o.seed + k as u64,
                extras: if k == 0 {
                    o.extras
                } else {
                    o.extras & !(CHIMNEYS | TANK | SOLAR | WATER_TOWER)
                },
                top: o.top,
                block: o.block && k == 0,
                light: o.light,
            },
            l,
        );
        b.transform_from(start, |q| {
            let fx = centre[0] + q[0] * ax[0] + (q[1] - depth / 2.0) * ay[0];
            let fy = centre[1] + q[0] * ax[1] + (q[1] - depth / 2.0) * ay[1];
            let wp = to_world([fx, fy]);
            [wp[0], wp[1], q[2]]
        });
    }
}

/// A flat roof over the whole plan: the slab, the parapets (a balustrade
/// to the street), and what stands on it.
// `i` is both the face of the polygon and its side.
#[allow(clippy::needless_range_loop)]
fn flat_roof(b: &mut Builder, plan: &Plan, l: &Look, o: &Shape, top: f64) {
    let poly = &plan.poly;
    let n = poly.len();
    prism_poly(
        b,
        l.roof,
        &inset(poly, &vec![0.15; n]),
        top - 0.12,
        top + 0.04,
    );
    let high = if o.extras & ALGARVE != 0 { 0.7 } else { 1.0 };
    for i in 0..n {
        let f = face_of(poly, i);
        if plan.sides[i] == Side::Street && o.top == Top::Balustrade && !o.light {
            roofs::balustrade(b, &f, top, l);
            continue;
        }
        f.bx(b, l.wall, (0.0, f.len), (top, top + high), (-0.2, 0.0));
        f.bx(
            b,
            l.trim,
            (-0.03, f.len + 0.03),
            (top + high, top + high + 0.06),
            (-0.23, 0.03),
        );
        if plan.sides[i] == Side::Street && o.extras & ALGARVE != 0 {
            f.bx(
                b,
                l.accent,
                (0.0, f.len),
                (top + 0.1, top + 0.35),
                (-0.01, 0.012),
            );
        }
    }
    let zr = top + 0.04;
    // Spots on the roof, from the middle outwards, inside the plan.
    let c = centroid(poly);
    let spot = |dx: f64, dy: f64, room: f64| -> Option<[f64; 2]> {
        let p = [c[0] + dx, c[1] + dy];
        let ok = inside(poly, p) && (0..n).all(|i| seg_dist(p, poly[i], poly[(i + 1) % n]) > room);
        ok.then_some(p)
    };
    if o.block {
        if let Some(p) = spot(0.0, 0.0, 2.2) {
            b.rbox(
                l.wall,
                [p[0] - 1.6, p[1] - 1.5, zr],
                [p[0] + 1.6, p[1] + 1.5, zr + 2.6],
                0.0,
            );
            b.rbox(
                l.trim,
                [p[0] - 1.65, p[1] - 1.55, zr + 2.6],
                [p[0] + 1.65, p[1] + 1.55, zr + 2.7],
                0.0,
            );
            b.rbox(
                l.door,
                [p[0] - 1.2, p[1] + 1.48, zr],
                [p[0] - 0.3, p[1] + 1.53, zr + 2.1],
                0.0,
            );
        }
    }
    if o.light {
        return;
    }
    if o.extras & TANK != 0 {
        if let Some(p) = spot(-3.0, 0.5, 0.9).or_else(|| spot(0.0, 0.0, 0.9)) {
            water_tank(b, p, zr);
        }
        if let Some(p) = spot(2.5, -1.0, 0.6) {
            aerial(b, p, zr, o.seed);
        }
    }
    if o.extras & LAUNDRY != 0 {
        if let (Some(a), Some(e)) = (spot(-2.5, 2.0, 0.5), spot(2.0, 2.0, 0.5)) {
            roof_clothes(b, a, e, zr, o.seed);
        }
    }
    if o.extras & SOLAR != 0 {
        for k in 0..3 {
            if let Some(p) = spot(-3.0 + 2.8 * k as f64, -2.5, 1.3) {
                roofs::solar_heater(b, p, zr);
            }
        }
    }
    if o.extras & WATER_TOWER != 0 {
        if let Some(p) = spot(3.5, 1.0, 1.6).or_else(|| spot(0.0, 2.0, 1.6)) {
            roofs::water_tower(b, p, zr);
        }
    }
    if o.extras & ALGARVE != 0 {
        if let Some(p) = spot(-1.5, -1.0, 0.6).or_else(|| spot(0.0, 0.0, 0.5)) {
            roofs::algarve_chimney(b, p, zr, l);
        }
    }
}

/// The signed area of a polygon (positive anticlockwise).
pub fn area(poly: &[[f64; 2]]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        / 2.0
}

fn centroid(poly: &[[f64; 2]]) -> [f64; 2] {
    let n = poly.len() as f64;
    let s = poly
        .iter()
        .fold([0.0, 0.0], |a, p| [a[0] + p[0], a[1] + p[1]]);
    let c = [s[0] / n, s[1] / n];
    if inside(poly, c) {
        return c;
    }
    // A point surely inside: the middle of the widest piece of a
    // horizontal line through the plan.
    let y = c[1];
    let mut xs = Vec::new();
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        if (p[1] - y) * (q[1] - y) < 0.0 {
            xs.push(p[0] + (y - p[1]) / (q[1] - p[1]) * (q[0] - p[0]));
        }
    }
    xs.sort_by(f64::total_cmp);
    let best = xs
        .chunks(2)
        .filter(|c| c.len() == 2)
        .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])));
    best.map_or(c, |c2| [(c2[0] + c2[1]) / 2.0, y])
}

/// Whether `p` is inside the polygon.
pub fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (a, e) = (poly[i], poly[(i + 1) % n]);
        if (a[1] > p[1]) != (e[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (e[1] - a[1]) * (e[0] - a[0]);
            if p[0] < x {
                odd = !odd;
            }
        }
    }
    odd
}

fn seg_dist(p: [f64; 2], a: [f64; 2], e: [f64; 2]) -> f64 {
    let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-12 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    };
    ((a[0] + dx * t - p[0]).powi(2) + (a[1] + dy * t - p[1]).powi(2)).sqrt()
}

/// The polygon (anticlockwise) with each side moved in by its distance.
pub fn inset(poly: &[[f64; 2]], d: &[f64]) -> Vec<[f64; 2]> {
    let n = poly.len();
    let normal = |i: usize| {
        let (a, e) = (poly[i], poly[(i + 1) % n]);
        let l = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2))
            .sqrt()
            .max(1e-9);
        [-(e[1] - a[1]) / l, (e[0] - a[0]) / l]
    };
    let most = d.iter().cloned().fold(0.0, f64::max);
    (0..n)
        .map(|i| {
            let j = (i + n - 1) % n;
            let (n0, n1) = (normal(j), normal(i));
            let p0 = [poly[j][0] + n0[0] * d[j], poly[j][1] + n0[1] * d[j]];
            let p1 = [poly[i][0] + n1[0] * d[i], poly[i][1] + n1[1] * d[i]];
            let d0 = [poly[i][0] - poly[j][0], poly[i][1] - poly[j][1]];
            let d1 = [
                poly[(i + 1) % n][0] - poly[i][0],
                poly[(i + 1) % n][1] - poly[i][1],
            ];
            let den = d0[0] * d1[1] - d0[1] * d1[0];
            let fallback = {
                let m = [n0[0] + n1[0], n0[1] + n1[1]];
                let ml = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-9);
                [poly[i][0] + m[0] / ml * d[i], poly[i][1] + m[1] / ml * d[i]]
            };
            if den.abs() < 1e-9 {
                return fallback;
            }
            let t = ((p1[0] - p0[0]) * d1[1] - (p1[1] - p0[1]) * d1[0]) / den;
            let q = [p0[0] + d0[0] * t, p0[1] + d0[1] * t];
            let far = ((q[0] - poly[i][0]).powi(2) + (q[1] - poly[i][1]).powi(2)).sqrt();
            if far > 4.0 * most.max(0.05) {
                fallback
            } else {
                q
            }
        })
        .collect()
}

/// Triangles of a simple polygon (anticlockwise), by cutting its ears.
pub fn triangulate(poly: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..poly.len()).collect();
    let mut out = Vec::new();
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let m = idx.len();
        let mut cut = false;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, p, c) = (poly[ia], poly[ib], poly[ic]);
            if cross(a, p, c) <= 1e-12 {
                continue;
            }
            let ear = idx.iter().all(|&j| {
                if j == ia || j == ib || j == ic {
                    return true;
                }
                let q = poly[j];
                !(cross(a, p, q) >= 0.0 && cross(p, c, q) >= 0.0 && cross(c, a, q) >= 0.0)
            });
            if ear {
                out.push([ia, ib, ic]);
                idx.remove(k);
                cut = true;
                break;
            }
        }
        if !cut {
            // Not simple or degenerate: a fan of what is left.
            for k in 1..idx.len() - 1 {
                out.push([idx[0], idx[k], idx[k + 1]]);
            }
            return out;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// A solid of the polygon (anticlockwise) from `z0` to `z1`.
pub fn prism_poly(b: &mut Builder, role: &'static str, poly: &[[f64; 2]], z0: f64, z1: f64) {
    let n = poly.len();
    if n < 3 || z1 - z0 < 1e-6 {
        return;
    }
    let poly: Vec<[f64; 2]> = if area(poly) < 0.0 {
        poly.iter().rev().copied().collect()
    } else {
        poly.to_vec()
    };
    let bot: Vec<u32> = poly.iter().map(|p| b.point([p[0], p[1], z0])).collect();
    let top: Vec<u32> = poly.iter().map(|p| b.point([p[0], p[1], z1])).collect();
    for t in triangulate(&poly) {
        b.tri(role, top[t[0]], top[t[1]], top[t[2]]);
        b.tri(role, bot[t[0]], bot[t[2]], bot[t[1]]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.quad(role, bot[i], bot[j], top[j], top[i]);
    }
}

/// The main direction of a plan (of its longest sides, folded to a
/// quarter turn).
pub fn dominant(poly: &[[f64; 2]]) -> f64 {
    let n = poly.len();
    let (mut c, mut s) = (0.0, 0.0);
    for i in 0..n {
        let (a, e) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt();
        let ang = dy.atan2(dx) * 4.0;
        c += l * ang.cos();
        s += l * ang.sin();
    }
    s.atan2(c) / 4.0
}

type Rect = ([f64; 2], [f64; 2]);

/// The rectangles (along the axes) that cover a polygon, overlapping where
/// they meet, and the bits too small for a roof of their own.
pub fn rect_cover(poly: &[[f64; 2]]) -> (Vec<Rect>, Vec<Rect>) {
    let coords = |k: usize| {
        let mut v: Vec<f64> = poly.iter().map(|p| p[k]).collect();
        v.sort_by(f64::total_cmp);
        let span = v[v.len() - 1] - v[0];
        let tol = (span / 30.0).max(0.4);
        let mut out: Vec<f64> = Vec::new();
        for x in v {
            if out.last().is_none_or(|l| x - l > tol) {
                out.push(x);
            } else if let Some(l) = out.last_mut() {
                *l = (*l + x) / 2.0;
            }
        }
        if out.len() < 2 {
            out.push(out[0] + 0.5);
        }
        // The ends exactly as the plan.
        let lo = poly.iter().map(|p| p[k]).fold(f64::MAX, f64::min);
        let hi = poly.iter().map(|p| p[k]).fold(f64::MIN, f64::max);
        out[0] = lo;
        let last = out.len() - 1;
        out[last] = hi;
        out
    };
    let xs = coords(0);
    let ys = coords(1);
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    let cell_in: Vec<Vec<bool>> = (0..nx)
        .map(|i| {
            (0..ny)
                .map(|j| inside(poly, [(xs[i] + xs[i + 1]) / 2.0, (ys[j] + ys[j + 1]) / 2.0]))
                .collect()
        })
        .collect();
    let mut covered = vec![vec![false; ny]; nx];
    let full = |i0: usize, i1: usize, j0: usize, j1: usize| {
        (i0..=i1).all(|i| (j0..=j1).all(|j| cell_in[i][j]))
    };
    let mut rects = Vec::new();
    let mut rest = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            if !cell_in[i][j] || covered[i][j] {
                continue;
            }
            let grow = |x_first: bool| {
                let (mut i0, mut i1, mut j0, mut j1) = (i, i, j, j);
                for pass in 0..2 {
                    if (pass == 0) == x_first {
                        while i0 > 0 && full(i0 - 1, i0 - 1, j0, j1) {
                            i0 -= 1;
                        }
                        while i1 + 1 < nx && full(i1 + 1, i1 + 1, j0, j1) {
                            i1 += 1;
                        }
                    } else {
                        while j0 > 0 && full(i0, i1, j0 - 1, j0 - 1) {
                            j0 -= 1;
                        }
                        while j1 + 1 < ny && full(i0, i1, j1 + 1, j1 + 1) {
                            j1 += 1;
                        }
                    }
                }
                (i0, i1, j0, j1)
            };
            let a = grow(true);
            let e = grow(false);
            let size =
                |r: (usize, usize, usize, usize)| (xs[r.1 + 1] - xs[r.0]) * (ys[r.3 + 1] - ys[r.2]);
            let r = if size(a) >= size(e) { a } else { e };
            for row in covered.iter_mut().take(r.1 + 1).skip(r.0) {
                for c in row.iter_mut().take(r.3 + 1).skip(r.2) {
                    *c = true;
                }
            }
            let rect = ([xs[r.0], ys[r.2]], [xs[r.1 + 1], ys[r.3 + 1]]);
            let small = (rect.1[0] - rect.0[0]).min(rect.1[1] - rect.0[1]) < 2.0;
            if small || rects.len() >= 12 {
                rest.push(rect);
            } else {
                rects.push(rect);
            }
        }
    }
    (rects, rest)
}

/// The four sides of a rectangle as a plan, for the typologies made on
/// rectangles.
pub fn rect_faces(w: f64, d: f64) -> [Face; 4] {
    faces((-w / 2.0, w / 2.0), (0.0, d))
}
