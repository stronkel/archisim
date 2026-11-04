//! The state of a building of the world: as it stands, abandoned (its
//! windows boarded up or empty, nothing on its balconies, weeds at its
//! foot) or being built (concrete frame, brick going up on the lower
//! floors, scaffolding on the street, the site fence and a crane).

use super::plan::{area, inset, inside, Plan, Side};
use super::skins::Skin;
use crate::build::Builder;

/// What the state of a building is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Lived in.
    Standing,
    /// Abandoned.
    Abandoned,
    /// Being built.
    Building,
}

impl State {
    pub fn of(n: f64) -> State {
        match n.round() as i64 {
            1 => State::Abandoned,
            2 => State::Building,
            _ => State::Standing,
        }
    }
}

fn hash(a: i64, b: i64, c: i64, seed: u64) -> u64 {
    let mut h = seed ^ 0x9e37_79b9_7f4a_7c15;
    for v in [a, b, c] {
        h ^= v as u64;
        h = h.wrapping_mul(0x0100_0000_01b3).rotate_left(23);
    }
    h
}

/// The pieces of life a building left behind loses (the glass comes back
/// as boards or as dark holes).
const LIFE: &[&str] = &[
    "flowers",
    "fruit",
    "cloth_a",
    "cloth_b",
    "cloth_c",
    "line",
    "curtain",
    "blind",
    "car_a",
    "car_b",
    "car_c",
    "car_glass",
    "tyre",
];

/// Leaves the model of a standing building abandoned.
pub fn abandon(b: &mut Builder, plan: &Plan, seed: u64) {
    for r in LIFE {
        b.parts.remove(r);
        b.fine.remove(r);
        b.soft.remove(r);
    }
    let mut boards = Vec::new();
    let mut holes = Vec::new();
    for g in ["glass", "g_reflect"] {
        for t in b.parts.remove(g).unwrap_or_default() {
            let c = t
                .iter()
                .map(|&i| b.positions[i as usize])
                .fold([0.0f64; 3], |a, p| {
                    [a[0] + p[0] as f64, a[1] + p[1] as f64, a[2] + p[2] as f64]
                });
            // A window is a cell of a metre and a half: all boarded or all empty.
            let cell = |v: f64| (v / 3.0 / 1.5).floor() as i64;
            if hash(cell(c[0]), cell(c[1]), cell(c[2]), seed).is_multiple_of(3) {
                holes.push(t);
            } else {
                boards.push(t);
            }
        }
    }
    b.parts.entry("f_oak").or_default().extend(boards);
    b.parts.entry("g_opaque").or_default().extend(holes);
    // The colour of its stucco faded.
    let stucco: Vec<&'static str> = b
        .parts
        .keys()
        .copied()
        .filter(|k| k.starts_with("w_") && *k != "w_green")
        .collect();
    for w in stucco {
        super::recolour(b, w, "w_faded");
    }
    // Weeds along the foot of the walls.
    let n = plan.poly.len();
    let mut k = 0;
    for i in 0..n {
        let (a, e) = (plan.poly[i], plan.poly[(i + 1) % n]);
        let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
        let len = dx.hypot(dy);
        if len < 1e-6 || plan.sides.get(i) == Some(&Side::Party) {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (uy, -ux);
        let mut s = 0.6;
        while s < len - 0.4 {
            k += 1;
            let h = hash(i as i64, k, 7, seed);
            let r = 0.15 + (h % 100) as f64 / 400.0;
            let off = 0.12 + r * 0.5;
            let c = [a[0] + ux * s + nx * off, a[1] + uy * s + ny * off, r * 0.9];
            b.ellipsoid("leaves", c, [r, r, r * 0.9]);
            s += 0.8 + (h % 7) as f64 * 0.35;
        }
        // The damp rising up the foot of the wall, in steps.
        let mut s = 0.0;
        let mut j = 0;
        while s < len - 0.05 {
            j += 1;
            let h = hash(i as i64, j, 11, seed);
            let w = (0.7 + (h % 9) as f64 * 0.2).min(len - s);
            let up = 0.35 + ((h >> 8) % 10) as f64 * 0.09;
            let mid = s + w / 2.0;
            b.obox(
                "w_damp",
                [
                    a[0] + ux * mid + nx * 0.012,
                    a[1] + uy * mid + ny * 0.012,
                    up / 2.0,
                ],
                [ux * w / 2.0, uy * w / 2.0, 0.0],
                [nx * 0.012, ny * 0.012, 0.0],
                [0.0, 0.0, up / 2.0],
            );
            s += w;
        }
    }
}

/// A building of `floors` over `plan` being built: the frame up, the slab
/// of the top floor not yet poured (its pillars wait with their bars), the
/// brick of the lower half, scaffolding along the street, the site fence and,
/// from three floors, a crane.
pub fn site_on(b: &mut Builder, plan: &Plan, skin: &Skin, floors: usize, seed: u64, light: bool) {
    let floors = floors.max(1);
    let poly = &plan.poly;
    let n = poly.len();
    let (g, u) = (skin.storey.0.max(2.8), skin.storey.1.max(2.7));
    let z = |k: usize| if k == 0 { 0.0 } else { g + (k - 1) as f64 * u };
    let slab = 0.25;
    // The ground slab and one slab per floor (the last floor has none yet).
    b.slab("c_concrete", poly, 0.0, 0.15);
    for k in 1..floors {
        b.slab("c_concrete", poly, z(k) - slab, z(k));
    }
    // Pillars round the edge and on a grid inside.
    let edge = inset(poly, &vec![0.35; n]);
    let mut pillars: Vec<[f64; 2]> = Vec::new();
    for i in 0..edge.len() {
        let (a, e) = (edge[i], edge[(i + 1) % edge.len()]);
        let len = (e[0] - a[0]).hypot(e[1] - a[1]);
        let m = (len / 5.0).ceil().max(1.0) as usize;
        for j in 0..m {
            let t = j as f64 / m as f64;
            pillars.push([a[0] + (e[0] - a[0]) * t, a[1] + (e[1] - a[1]) * t]);
        }
    }
    let (lo, hi) = poly
        .iter()
        .fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            )
        });
    if area(poly) > 60.0 {
        let mut x = lo[0] + 5.0;
        while x < hi[0] - 2.0 {
            let mut y = lo[1] + 5.0;
            while y < hi[1] - 2.0 {
                let p = [x, y];
                if inside(&edge, p) && pillars.iter().all(|q| (q[0] - x).hypot(q[1] - y) > 3.0) {
                    pillars.push(p);
                }
                y += 5.0;
            }
            x += 5.0;
        }
    }
    let c = 0.15;
    for k in 0..floors {
        let z0 = if k == 0 { 0.15 } else { z(k) };
        let last = k + 1 == floors;
        let z1 = if last {
            z0 + 1.1
        } else {
            z(k + 1) - slab + 0.01
        };
        for p in &pillars {
            b.rbox(
                "c_concrete",
                [p[0] - c, p[1] - c, z0 - 0.01],
                [p[0] + c, p[1] + c, z1],
                0.0,
            );
            if last && !light {
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let q = [p[0] + dx * 0.09, p[1] + dy * 0.09];
                    b.cyl(
                        "iron",
                        [q[0], q[1], z1 - 0.05],
                        [q[0], q[1], z1 + 0.7],
                        0.008,
                    );
                }
            }
        }
    }
    // The brick of the lower half: panels with the gaps of the windows.
    let brick = (floors / 2).max(if floors > 1 { 1 } else { 0 });
    for k in 0..brick {
        let z0 = if k == 0 { 0.15 } else { z(k) };
        let z1 = z(k + 1) - slab;
        for i in 0..n {
            let (a, e) = (poly[i], poly[(i + 1) % n]);
            let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
            let len = dx.hypot(dy);
            if len < 0.5 {
                continue;
            }
            let (ux, uy) = (dx / len, dy / len);
            let (nx, ny) = (uy, -ux);
            let party = plan.sides.get(i) == Some(&Side::Party);
            let mut s = 0.0;
            let mut j = 0;
            while s < len - 0.05 {
                let solid = party || j % 2 == 0;
                let w: f64 = if solid { 1.6 } else { 1.2 };
                let w = w.min(len - s);
                // Between the panels, the sill of a window, if built yet.
                let t = if solid { z1 - z0 } else { 0.9 };
                if solid || !hash(i as i64, j, k as i64, seed).is_multiple_of(4) {
                    let mid = s + w / 2.0;
                    let cc = [
                        a[0] + ux * mid - nx * 0.06,
                        a[1] + uy * mid - ny * 0.06,
                        z0 + t / 2.0,
                    ];
                    b.obox(
                        "k_red",
                        cc,
                        [ux * w / 2.0, uy * w / 2.0, 0.0],
                        [nx * 0.06, ny * 0.06, 0.0],
                        [0.0, 0.0, t / 2.0],
                    );
                }
                s += w;
                j += 1;
            }
        }
    }
    // Scaffolding along the street.
    let top = z(floors) - 0.3;
    for i in 0..n {
        if plan.sides.get(i) != Some(&Side::Street) {
            continue;
        }
        let (a, e) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (e[0] - a[0], e[1] - a[1]);
        let len = dx.hypot(dy);
        if len < 3.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (uy, -ux);
        let at = |s: f64, o: f64, z: f64| [a[0] + ux * s + nx * o, a[1] + uy * s + ny * o, z];
        let m = (len / 2.5).ceil() as usize;
        let step = len / m as f64;
        for j in 0..=m {
            let s = j as f64 * step;
            for o in [0.4, 1.4] {
                b.cyl("metal", at(s, o, 0.0), at(s, o, top + 1.0), 0.024);
            }
        }
        let mut zl = 2.0;
        while zl <= top + 1.0 {
            for o in [0.4, 1.4] {
                b.cyl("metal", at(0.0, o, zl), at(len, o, zl), 0.024);
                if zl + 1.0 <= top + 1.0 {
                    b.cyl("metal", at(0.0, o, zl + 1.0), at(len, o, zl + 1.0), 0.02);
                }
            }
            if !light {
                for j in 0..m {
                    let s0 = j as f64 * step;
                    let mid = s0 + step / 2.0;
                    let cc = at(mid, 0.9, zl + 0.045);
                    b.obox(
                        "b_cedar",
                        cc,
                        [ux * step / 2.0, uy * step / 2.0, 0.0],
                        [nx * 0.5, ny * 0.5, 0.0],
                        [0.0, 0.0, 0.025],
                    );
                }
            }
            zl += 2.0;
        }
        // The net over the scaffolding, from the first lift to the
        // floors whose brick is going up.
        let net_top = z(brick.max(1)).min(top + 1.0);
        if !light && net_top > 3.0 {
            let net = at(len / 2.0, 1.43, (2.0 + net_top) / 2.0);
            b.obox(
                "w_green",
                net,
                [ux * len / 2.0, uy * len / 2.0, 0.0],
                [nx * 0.01, ny * 0.01, 0.0],
                [0.0, 0.0, (net_top - 2.0) / 2.0],
            );
        }
        // The site fence, two metres out from the scaffolding.
        let fence_o = 3.4;
        let m2 = (len / 2.0).ceil() as usize;
        let st = len / m2 as f64;
        for j in 0..=m2 {
            let s = j as f64 * st;
            b.cyl("metal", at(s, fence_o, 0.0), at(s, fence_o, 2.0), 0.03);
        }
        let cc = at(len / 2.0, fence_o + 0.03, 1.0);
        b.obox(
            "f_white",
            cc,
            [ux * len / 2.0, uy * len / 2.0, 0.0],
            [nx * 0.012, ny * 0.012, 0.0],
            [0.0, 0.0, 0.95],
        );
    }
    // The crane, its mast through the middle of the frame.
    if floors >= 3 {
        let mid = pillars
            .iter()
            .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
        let mid = [mid[0] / pillars.len() as f64, mid[1] / pillars.len() as f64];
        let p = if inside(poly, mid) { mid } else { pillars[0] };
        let h = z(floors) + 10.0;
        let m = 0.8;
        if light {
            b.rbox(
                "f_yellow",
                [p[0] - m, p[1] - m, 0.0],
                [p[0] + m, p[1] + m, h],
                0.0,
            );
        } else {
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let q = [p[0] + dx * m, p[1] + dy * m];
                b.cyl("f_yellow", [q[0], q[1], 0.0], [q[0], q[1], h], 0.07);
            }
            let mut zz = 0.5;
            while zz < h - 1.0 {
                let ring = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                for r in 0..4 {
                    let (a, e) = (ring[r], ring[(r + 1) % 4]);
                    let qa = [p[0] + a.0 * m, p[1] + a.1 * m, zz];
                    let qe = [p[0] + e.0 * m, p[1] + e.1 * m, zz + 1.6];
                    b.cyl("f_yellow", qa, qe, 0.035);
                }
                zz += 1.6;
            }
        }
        // The turntable, the cab, the jib and its counterweight.
        let turn = (hash(1, 2, 3, seed) % 360) as f64 * std::f64::consts::PI / 180.0;
        let (cx, cy) = (turn.cos(), turn.sin());
        b.rbox(
            "f_yellow",
            [p[0] - 1.1, p[1] - 1.1, h],
            [p[0] + 1.1, p[1] + 1.1, h + 0.5],
            0.0,
        );
        b.rbox(
            "f_white",
            [p[0] - 0.7, p[1] - 0.7, h + 0.5],
            [p[0] + 0.7, p[1] + 0.7, h + 2.3],
            0.0,
        );
        let jib = 25.0;
        let zj = h + 2.3;
        let along = |s: f64, zz: f64| [p[0] + cx * s, p[1] + cy * s, zz];
        if light {
            b.obox(
                "f_yellow",
                along(jib / 2.0 - 7.0, zj + 0.6),
                [cx * (jib + 7.0) / 2.0, cy * (jib + 7.0) / 2.0, 0.0],
                [-cy * 0.6, cx * 0.6, 0.0],
                [0.0, 0.0, 0.6],
            );
        } else {
            // A triangular lattice: two chords below, one on top, and
            // the diagonals between them every metre and a half.
            let side = |s: f64, k: f64, zz: f64| {
                let q = along(s, zz);
                [q[0] - cy * k, q[1] + cx * k, zz]
            };
            let (s0, s1, w, r) = (-7.0, jib, 0.6, 0.05);
            for k in [-w, w] {
                b.cyl("f_yellow", side(s0, k, zj + r), side(s1, k, zj + r), r);
            }
            b.cyl(
                "f_yellow",
                side(s0, 0.0, zj + 1.2),
                side(s1, 0.0, zj + 1.2),
                r,
            );
            let step = 1.5;
            let mut s = s0;
            let mut up = true;
            while s < s1 - 1e-6 {
                let e = (s + step).min(s1);
                let (sa, se) = if up { (s, e) } else { (e, s) };
                for k in [-w, w] {
                    b.cyl(
                        "f_yellow",
                        side(sa, k, zj + r),
                        side(se, 0.0, zj + 1.2),
                        0.03,
                    );
                }
                b.cyl("f_yellow", side(s, -w, zj + r), side(s, w, zj + r), 0.03);
                up = !up;
                s = e;
            }
            for k in [-w, w] {
                b.cyl(
                    "f_yellow",
                    side(s1, k, zj + r),
                    side(s1, 0.0, zj + 1.2),
                    0.03,
                );
            }
            b.cyl("f_yellow", side(s1, -w, zj + r), side(s1, w, zj + r), 0.03);
            // The peak over the cab with its ties to the two ends.
            let peak = along(0.0, zj + 5.0);
            b.cyl("f_yellow", side(0.0, 0.0, zj + 1.2), peak, 0.06);
            b.cyl("iron", peak, side(s1 * 0.55, 0.0, zj + 1.2), 0.02);
            b.cyl("iron", peak, side(s0, 0.0, zj + 1.2), 0.02);
            // The trolley the hook hangs from.
            b.obox(
                "iron",
                along(jib * 0.6, zj),
                [cx * 0.4, cy * 0.4, 0.0],
                [-cy * (w + 0.05), cx * (w + 0.05), 0.0],
                [0.0, 0.0, 0.08],
            );
        }
        b.rbox(
            "c_concrete",
            {
                let q = along(-6.0, 0.0);
                [q[0] - 1.0, q[1] - 1.0, zj - 1.4]
            },
            {
                let q = along(-6.0, 0.0);
                [q[0] + 1.0, q[1] + 1.0, zj + 0.05]
            },
            0.0,
        );
        // The hook hanging from the jib.
        let s = jib * 0.6;
        b.cyl("iron", along(s, zj - 0.06), along(s, zj - 8.0), 0.02);
        b.rbox(
            "f_yellow",
            {
                let q = along(s, 0.0);
                [q[0] - 0.25, q[1] - 0.25, zj - 8.6]
            },
            {
                let q = along(s, 0.0);
                [q[0] + 0.25, q[1] + 0.25, zj - 7.98]
            },
            0.0,
        );
    }
}

/// The plan and floors a recipe of the workshop builds on (the same as
/// the recipe's own), for its state.
fn recipe_plan(kind: &str, p: &crate::Params, skin: &Skin) -> (Plan, usize) {
    use super::types::{floors_of, usual_width};
    let s = |a, b, c, d| [a, b, c, d];
    let (w, d, sides, floors) = match kind {
        "fine_world_block" => (
            p.get("width", 22.0).clamp(10.0, 60.0),
            p.get("depth", 14.0).clamp(9.0, 24.0),
            s(Side::Rear, Side::Free, Side::Street, Side::Free),
            floors_of(p, skin.block.floors, 16),
        ),
        "fine_world_house" => (
            p.get("width", 10.0).clamp(6.0, 20.0),
            p.get("depth", 8.0).clamp(6.0, 16.0),
            s(Side::Back, Side::Free, Side::Street, Side::Free),
            floors_of(p, skin.house.floors, 3),
        ),
        "fine_world_tower" => (
            p.get("width", 24.0).clamp(12.0, 60.0),
            p.get("depth", 20.0).clamp(12.0, 60.0),
            s(Side::Rear, Side::Free, Side::Street, Side::Free),
            floors_of(p, skin.tower.floors, 60),
        ),
        "fine_world_courtyard" => (
            p.get("width", 16.0).clamp(10.0, 40.0),
            p.get("depth", 18.0).clamp(10.0, 40.0),
            s(Side::Back, Side::Party, Side::Street, Side::Party),
            floors_of(p, skin.house.floors.clamp(1, 2), 3),
        ),
        "fine_world_row" => {
            let unit = usual_width(skin).min(9.0);
            let w = p.get("width", 0.0);
            (
                if w <= 0.0 {
                    unit * 4.0
                } else {
                    w.clamp(8.0, 80.0)
                },
                p.get("depth", 10.0).clamp(7.0, 20.0),
                s(Side::Back, Side::Free, Side::Street, Side::Free),
                floors_of(p, skin.floors.clamp(2, 3), 5),
            )
        }
        "fine_world_facility" => {
            let (w, d, floors) = super::civic::recipe_plan(p, skin);
            (
                w,
                d,
                s(Side::Back, Side::Free, Side::Street, Side::Free),
                floors,
            )
        }
        _ => {
            let w = p.get("width", 0.0);
            (
                if w <= 0.0 {
                    usual_width(skin)
                } else {
                    w.clamp(4.0, 30.0)
                },
                p.get("depth", 12.0).clamp(7.0, 25.0),
                s(Side::Back, Side::Party, Side::Street, Side::Party),
                floors_of(p, skin.floors, 10),
            )
        }
    };
    (Plan::rect(w, d, sides), floors)
}

/// A recipe of the workshop in the state it asks for (`state`: 0 standing,
/// 1 abandoned, 2 being built).
pub fn recipe(kind: &str, b: &mut Builder, p: &crate::Params, f: fn(&mut Builder, &crate::Params)) {
    let state = State::of(p.get("state", 0.0));
    if state == State::Standing || kind == "fine_world_sample" {
        f(b, p);
        return;
    }
    let skin = super::types::skin_for(p);
    let (plan, floors) = recipe_plan(kind, p, &skin);
    let seed = super::types::seed_of(p);
    if state == State::Building {
        site_on(b, &plan, &skin, floors, seed, super::types::light_of(p));
    } else {
        f(b, p);
        abandon(b, &plan, seed);
    }
}
