//! More typologies of the world (tanda 8): the tower of each place —
//! flats piled up with the place's balconies, a curtain wall of glass, or
//! the brick and stone of New York stepping back to its crown and spire —,
//! the house round a courtyard (patio, riad, siheyuan) and the row of
//! houses with their front gardens.

use super::super::more::{curtain_wall, fruit_tree, well};
use super::super::*;
use super::plan::{area, building, face_of, inset, prism_poly, Plan, Shape, Side};
use super::skins::*;
use super::types::{finish, floors_of, light_of, seed_of, shape_of, usual_width, Typology};
use super::{look, recolour, Look};
use crate::build::Builder;
use crate::Params;

/// A tower of a place over a plan; its look.
pub fn tower_on(
    b: &mut Builder,
    plan: &Plan,
    base: &Skin,
    floors: usize,
    seed: u64,
    light: bool,
) -> Look {
    let mut sk = *base;
    if !base.tower.walls.is_empty() {
        sk.block.walls = base.tower.walls;
    }
    match base.tower.style {
        TowerStyle::Residential => {
            let (sk2, l, shape) = shape_of(&sk, Typology::Tower, floors, seed, light);
            let shape = Shape {
                roof: Roof::Flat,
                ..shape
            };
            let l = Look {
                roof: "r_gravel",
                ..l
            };
            building(b, plan, &sk2, &l, &shape);
            l
        }
        TowerStyle::Glass => glass_tower(b, plan, &sk, floors, seed, light),
        TowerStyle::Deco => deco_tower(b, plan, &sk, floors, seed, light),
    }
}

/// A tower of glass: a lobby, the curtain wall, the screen of its plant.
fn glass_tower(
    b: &mut Builder,
    plan: &Plan,
    sk: &Skin,
    floors: usize,
    seed: u64,
    light: bool,
) -> Look {
    let plan = plan.ccw();
    let poly = &plan.poly;
    let n = poly.len();
    let l = Look {
        roof: "r_gravel",
        ..look(sk, seed, false)
    };
    let lobby = 5.0;
    let fh = 3.6;
    let top = lobby + fh * (floors.max(2) - 1) as f64;
    prism_poly(b, "interior", &inset(poly, &vec![0.3; n]), 0.0, top);
    for i in 0..n {
        let f = face_of(poly, i);
        if f.len < 0.5 {
            continue;
        }
        if plan.sides[i] == Side::Party {
            wall(b, "c_concrete", &f, (0.0, top), 0.3, &[]);
            continue;
        }
        // The lobby: tall glass set back behind its columns.
        f.bx(b, "glass", (0.0, f.len), (0.0, lobby - 0.4), (-1.2, -1.15));
        f.bx(b, "f_alu", (0.0, f.len), (lobby - 0.4, lobby), (-1.2, 0.0));
        let m = (f.len / 6.0).round().max(1.0) as usize;
        for k in 0..=m {
            let x = (f.len * k as f64 / m as f64).clamp(0.3, f.len - 0.3);
            f.bx(
                b,
                "c_concrete",
                (x - 0.3, x + 0.3),
                (0.0, lobby - 0.4),
                (-0.6, 0.0),
            );
        }
        f.bx(b, "paving", (0.0, f.len), (0.0, 0.03), (-1.2, 0.0));
        if !light {
            let k = ((f.len - 1.0) / 1.5).round().max(1.0) as usize;
            for j in 0..=k {
                let x = 0.5 + (f.len - 1.0) * j as f64 / k as f64;
                f.bx(
                    b,
                    "f_alu",
                    (x - 0.03, x + 0.03),
                    (0.0, lobby - 0.4),
                    (-1.2, -1.1),
                );
            }
        }
        if plan.sides[i] == Side::Street && f.len > 6.0 {
            let c = f.len / 2.0;
            f.bx(b, "f_alu", (c - 2.5, c + 2.5), (3.4, 3.6), (-1.2, 2.5));
            f.bx(b, "paving", (0.0, f.len), (0.0, 0.03), (0.0, 3.0));
        }
        // The curtain wall up the rest.
        if light {
            f.bx(b, "g_reflect", (0.0, f.len), (lobby, top), (-0.02, 0.0));
            let mut z = lobby;
            while z < top - 0.1 {
                f.bx(b, "g_opaque", (0.0, f.len), (z, z + 0.9), (0.0, 0.02));
                z += fh;
            }
        } else {
            curtain_wall(b, &f, (0.0, f.len), (lobby, top), 1.5, fh, 0.14);
        }
    }
    // The roof and the screen round its plant.
    prism_poly(
        b,
        l.roof,
        &inset(poly, &vec![0.0; n]),
        top - 0.05,
        top + 0.1,
    );
    let screen = inset(poly, &vec![1.5; n]);
    if area(&screen) > 20.0 {
        for i in 0..n {
            let f = face_of(&screen, i);
            if light {
                f.bx(
                    b,
                    "f_alu",
                    (0.0, f.len),
                    (top + 0.1, top + 3.0),
                    (-0.05, 0.0),
                );
            } else {
                let mut x = 0.1;
                while x < f.len - 0.05 {
                    f.bx(
                        b,
                        "f_alu",
                        (x, x + 0.06),
                        (top + 0.1, top + 3.0),
                        (-0.25, 0.0),
                    );
                    x += 0.35;
                }
                f.bx(
                    b,
                    "f_alu",
                    (0.0, f.len),
                    (top + 2.9, top + 3.0),
                    (-0.25, 0.0),
                );
            }
        }
        prism_poly(
            b,
            "appliance",
            &inset(&screen, &vec![0.8; n]),
            top + 0.1,
            top + 2.2,
        );
    }
    recolour(b, "mullion", "f_alu");
    recolour(b, "spandrel", "g_opaque");
    recolour(b, "curtain_glass", "g_reflect");
    l
}

/// A tower of brick and stone stepping back: its base, its shaft, its
/// crown and its spire.
fn deco_tower(
    b: &mut Builder,
    plan: &Plan,
    sk: &Skin,
    floors: usize,
    seed: u64,
    light: bool,
) -> Look {
    let plan = plan.ccw();
    let n = plan.poly.len();
    let (sk2, l, shape) = shape_of(sk, Typology::Tower, floors, seed, light);
    let l = Look {
        roof: "r_gravel",
        ..l
    };
    let base_floors = floors.clamp(2, 6);
    let crown = if floors > base_floors + 6 { 4 } else { 0 };
    let shaft = floors.saturating_sub(base_floors + crown);
    let mut stages: Vec<(Plan, usize)> = vec![(plan.clone(), base_floors)];
    let mut poly = plan.poly.clone();
    for (setback, f) in [(2.5, shaft), (2.5, crown)] {
        if f == 0 {
            continue;
        }
        let next = inset(&poly, &vec![setback; n]);
        if area(&next) < 60.0 {
            stages.last_mut().unwrap().1 += f;
            continue;
        }
        poly = next.clone();
        stages.push((
            Plan {
                poly: next,
                sides: vec![Side::Free; n],
            },
            f,
        ));
    }
    let mut z = 0.0;
    let last = stages.len() - 1;
    for (k, (p, f)) in stages.iter().enumerate() {
        let start = b.mark();
        let upper = k > 0;
        let mut sk3 = sk2;
        if upper {
            sk3.storey = (sk2.storey.1, sk2.storey.1);
            sk3.ground = Ground::Door;
            sk3.balc = Balc::None;
        }
        let shape = Shape {
            floors: *f,
            seed: seed + k as u64,
            roof: Roof::Flat,
            pavement: !upper,
            sparse_free: false,
            extras: if k == last {
                shape.extras
            } else {
                shape.extras & !(WATER_TOWER | TANK)
            },
            block: k == last,
            ..shape
        };
        let built = building(b, p, &sk3, &l, &shape);
        if upper {
            b.transform_from(start, |q| [q[0], q[1], q[2] + z]);
        }
        z += built.top;
    }
    // The crown's spire.
    let c = poly.iter().fold([0.0, 0.0], |a, p| {
        [a[0] + p[0] / n as f64, a[1] + p[1] / n as f64]
    });
    let r = (area(&poly).sqrt() * 0.25).clamp(1.0, 4.0);
    b.cone(
        l.trim,
        [c[0], c[1], z + 0.9],
        [c[0], c[1], z + 0.9 + r * 2.2],
        r,
        0.25,
        4,
    );
    b.cone(
        "metal",
        [c[0], c[1], z + 0.9 + r * 2.0],
        [c[0], c[1], z + 0.9 + r * 2.0 + 6.0],
        0.12,
        0.02,
        6,
    );
    let _ = light;
    l
}

/// A house round a courtyard over a plan: a wing along each side, the
/// courtyard paved with its well, its pots and its orange tree.
pub fn courtyard_on(
    b: &mut Builder,
    plan: &Plan,
    base: &Skin,
    floors: usize,
    seed: u64,
    light: bool,
) -> Look {
    let plan = plan.ccw();
    let poly = &plan.poly;
    let n = poly.len();
    // The wings are the place's house.
    let (sk, l, shape) = shape_of(base, Typology::House, floors, seed, light);
    let shape = Shape {
        lift: None,
        open_below: false,
        steps: true,
        ..shape
    };
    let ext = |k: usize| {
        let v: Vec<f64> = poly.iter().map(|p| p[k]).collect();
        v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
    };
    let depth = (ext(0).min(ext(1)) * 0.28).clamp(4.0, 7.0);
    let inner = inset(poly, &vec![depth; n]);
    if area(&inner) < 12.0 {
        building(b, &plan, &sk, &l, &shape);
        return l;
    }
    let wings: Vec<Plan> = if n == 4 {
        // Four sides: the front and the back wings whole, the side wings
        // between them, as they are built.
        let normal = |i: usize| {
            let (a, e) = (poly[i], poly[(i + 1) % n]);
            let l = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2))
                .sqrt()
                .max(1e-9);
            (
                [(e[0] - a[0]) / l, (e[1] - a[1]) / l],
                [-(e[1] - a[1]) / l, (e[0] - a[0]) / l],
                l,
            )
        };
        (0..4)
            .map(|i| {
                let (u, nn, len) = normal(i);
                let a = poly[i];
                let at =
                    |s: f64, d: f64| [a[0] + u[0] * s + nn[0] * d, a[1] + u[1] * s + nn[1] * d];
                if i % 2 == 0 {
                    Plan {
                        poly: vec![at(0.0, 0.0), at(len, 0.0), at(len, depth), at(0.0, depth)],
                        sides: vec![
                            plan.sides[i],
                            plan.sides[(i + 1) % 4],
                            Side::Free,
                            plan.sides[(i + 3) % 4],
                        ],
                    }
                } else {
                    Plan {
                        poly: vec![
                            at(depth, 0.0),
                            at(len - depth, 0.0),
                            at(len - depth, depth),
                            at(depth, depth),
                        ],
                        sides: vec![plan.sides[i], Side::Party, Side::Free, Side::Party],
                    }
                }
            })
            .collect()
    } else {
        (0..n)
            .map(|i| {
                let j = (i + 1) % n;
                Plan {
                    poly: vec![poly[i], poly[j], inner[j], inner[i]],
                    sides: vec![plan.sides[i], Side::Party, Side::Free, Side::Party],
                }
            })
            .collect()
    };
    for (i, wing) in wings.into_iter().enumerate() {
        let s = Shape {
            seed: seed + i as u64,
            pavement: plan.sides[i] == Side::Street,
            ..shape
        };
        building(b, &wing, &sk, &l, &s);
    }
    // The courtyard.
    prism_poly(b, "paving", &inner, 0.0, 0.03);
    let c = inner.iter().fold([0.0, 0.0], |a, p| {
        [a[0] + p[0] / n as f64, a[1] + p[1] / n as f64]
    });
    if !light {
        well(b, c);
        for (k, p) in inner.iter().enumerate() {
            let q = [p[0] + (c[0] - p[0]) * 0.18, p[1] + (c[1] - p[1]) * 0.18];
            standing_pot(b, [q[0], q[1], 0.03], 1.0, seed + k as u64);
        }
        let t = [
            c[0] + (inner[0][0] - c[0]) * 0.45,
            c[1] + (inner[0][1] - c[1]) * 0.45,
        ];
        fruit_tree(b, t, 3.5, seed, true);
        recolour(b, "stone", "s_lime");
    }
    l
}

/// The tower of a recipe: a rectangle, free on its four sides.
pub fn tower(b: &mut Builder, p: &Params) {
    let base = skin_of(p.get("skin", 0.0));
    let w = p.get("width", 24.0).clamp(12.0, 60.0);
    let d = p.get("depth", 20.0).clamp(12.0, 60.0);
    let floors = floors_of(p, base.tower.floors, 60);
    let plan = Plan::rect(w, d, [Side::Rear, Side::Free, Side::Street, Side::Free]);
    let l = tower_on(b, &plan, base, floors, seed_of(p), light_of(p));
    finish(b, &l);
}

/// The courtyard house of a recipe: a rectangle between party walls.
pub fn courtyard(b: &mut Builder, p: &Params) {
    let base = skin_of(p.get("skin", 0.0));
    let w = p.get("width", 16.0).clamp(10.0, 40.0);
    let d = p.get("depth", 18.0).clamp(10.0, 40.0);
    let floors = floors_of(p, base.house.floors.clamp(1, 2), 3);
    let plan = Plan::rect(w, d, [Side::Back, Side::Party, Side::Street, Side::Party]);
    let l = courtyard_on(b, &plan, base, floors, seed_of(p), light_of(p));
    finish(b, &l);
}

/// A row of houses of the street, each its own colour, with their front
/// gardens and their fences.
pub fn row(b: &mut Builder, p: &Params) {
    let base = skin_of(p.get("skin", 0.0));
    let seed = seed_of(p);
    let w = p.get("width", 0.0);
    let unit = usual_width(base).min(9.0);
    let w = if w <= 0.0 {
        unit * 4.0
    } else {
        w.clamp(8.0, 80.0)
    };
    let d = p.get("depth", 10.0).clamp(7.0, 20.0);
    let floors = floors_of(p, base.floors.clamp(2, 3), 5);
    let light = light_of(p);
    let n = (w / unit).round().max(2.0) as usize;
    let uw = w / n as f64;
    let garden = 3.0;
    let mut last = None;
    for k in 0..n {
        let x0 = -w / 2.0 + uw * k as f64;
        let x1 = x0 + uw;
        let plan = Plan {
            poly: vec![[x0, 0.0], [x1, 0.0], [x1, d], [x0, d]],
            sides: vec![
                Side::Back,
                if k + 1 == n { Side::Free } else { Side::Party },
                Side::Street,
                if k == 0 { Side::Free } else { Side::Party },
            ],
        };
        let s = seed * 31 + k as u64;
        let (sk, l, shape) = shape_of(base, Typology::Townhouse, floors, s, light);
        let shape = Shape {
            pavement: false,
            ..shape
        };
        let start = b.mark();
        building(b, &plan, &sk, &l, &shape);
        finish_part(b, start, &l);
        last = Some(l);
    }
    let l = last.unwrap_or_else(|| look(base, seed, false));
    // The front gardens, the fence along the pavement and between them.
    b.rbox("lawn", [-w / 2.0, d, 0.0], [w / 2.0, d + garden, 0.02], 0.0);
    b.rbox(
        "paving",
        [-w / 2.0, d + garden, 0.0],
        [w / 2.0, d + garden + 2.0, 0.03],
        0.0,
    );
    let fence = base.house.fence;
    for k in 0..n {
        let x0 = -w / 2.0 + uw * k as f64;
        let c = x0 + uw / 2.0;
        b.rbox(
            "paving",
            [c - 0.6, d, 0.0],
            [c + 0.6, d + garden, 0.035],
            0.0,
        );
        let f = Face {
            o: [x0, d + garden],
            u: [1.0, 0.0],
            len: uw,
        };
        for (a, e) in [(0.15, uw / 2.0 - 0.6), (uw / 2.0 + 0.6, uw - 0.15)] {
            if e - a < 0.2 {
                continue;
            }
            match fence {
                Fence::Hedge => b.rbox(
                    "hedge",
                    [x0 + a, d + garden - 0.6, 0.0],
                    [x0 + e, d + garden - 0.05, 1.0],
                    0.2,
                ),
                Fence::Picket => {
                    f.bx(b, "b_white", (a, e), (0.3, 0.38), (-0.06, -0.03));
                    let mut x = a + 0.05;
                    while x < e - 0.03 && !light {
                        f.bx(
                            b,
                            "b_white",
                            (x - 0.03, x + 0.03),
                            (0.0, 0.95),
                            (-0.03, 0.0),
                        );
                        x += 0.13;
                    }
                }
                _ => {
                    f.bx(b, l.wall, (a, e), (0.0, 0.5), (-0.25, 0.0));
                    if light {
                        f.bx(b, "iron", (a, e), (0.5, 1.2), (-0.14, -0.11));
                    } else {
                        railing(b, &f, "iron", (a + 0.05, e - 0.05), 0.5, -0.125, 0.7, 0.12);
                    }
                }
            }
        }
        if k > 0 && fence != Fence::Hedge {
            b.rbox(l.wall, [x0 - 0.1, d, 0.0], [x0 + 0.1, d + garden, 0.8], 0.0);
        }
        if !light && unit_pick(seed, k) {
            standing_pot(b, [c + 1.2, d + 0.6, 0.02], 1.0, seed + k as u64);
        }
    }
}

fn unit_pick(seed: u64, k: usize) -> bool {
    unit(seed, 500 + k as u64) > 0.4
}

/// The roles of the shared pieces given to one house of a row (each has
/// its own colours).
fn finish_part(b: &mut Builder, start: usize, l: &Look) {
    let from = start as u32;
    for (shared, own) in [
        ("tiles", l.roof),
        ("roof", l.roof),
        ("wall", l.wall),
        ("trim", l.trim),
    ] {
        let Some(tris) = b.parts.get_mut(shared) else {
            continue;
        };
        let (mine, rest): (Vec<[u32; 3]>, Vec<[u32; 3]>) =
            tris.drain(..).partition(|t| t.iter().all(|i| *i >= from));
        if rest.is_empty() {
            b.parts.remove(shared);
        } else {
            *tris = rest;
        }
        let fine = b.fine.contains(shared);
        b.parts.entry(own).or_default().extend(mine);
        if fine {
            b.fine.insert(own);
        }
    }
}
