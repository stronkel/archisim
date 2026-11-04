//! The three typologies of the world's buildings: the house between party
//! walls of a street, the block of flats standing free and the detached
//! house in its plot. Each takes its skin by number (`skin`) and its
//! look by its variant.

use super::super::more::{fruit_tree, palm};
use super::super::*;
use super::facade::{Front, REVEAL};
use super::plan::{building, Plan, Shape, Side};
use super::roofs::faces;
use super::skins::*;
use super::state::State;
use super::{look, recolour, Look};
use crate::build::Builder;
use crate::Params;

/// The usual front of the street's house of a place.
pub fn usual_width(sk: &Skin) -> f64 {
    match sk.key {
        "paises_bajos" | "reino_unido" => 5.6,
        "nueva_york" | "san_francisco" | "tropico" | "japon" => 6.5,
        "andalucia" | "algarve" | "grecia" => 7.5,
        "magreb" => 8.0,
        "lisboa" | "china" => 9.0,
        "paris" | "mediterraneo" | "latam" | "corea" | "este" | "california" => 12.5,
        _ => 10.0,
    }
}

pub(super) fn seed_of(p: &Params) -> u64 {
    p.get("variant", 1.0).round().max(0.0) as u64
}

pub(super) fn floors_of(p: &Params, usual: usize, max: usize) -> usize {
    let n = p.get("floors", 0.0).round();
    if n < 1.0 {
        usual
    } else {
        (n as usize).clamp(1, max)
    }
}

/// The roles the shared pieces draw in, given the building's.
pub fn finish(b: &mut Builder, l: &Look) {
    recolour(b, "tiles", l.roof);
    recolour(b, "roof", l.roof);
    recolour(b, "wall", l.wall);
    recolour(b, "trim", l.trim);
}

/// The three typologies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Typology {
    /// The house between party walls of a street.
    Townhouse,
    /// The block of flats.
    Block,
    /// The detached house.
    House,
    /// The tower.
    Tower,
    /// The house round a courtyard.
    Courtyard,
    /// A row of houses of the street with their front gardens.
    Row,
}

/// The skin of a place as a typology wears it: the block and the house
/// take what the place says of them.
pub fn adapted(base: &Skin, t: Typology) -> Skin {
    let mut sk = *base;
    match t {
        Typology::Townhouse | Typology::Row | Typology::Courtyard => {}
        Typology::Block | Typology::Tower => {
            if !base.block.walls.is_empty() {
                sk.walls = base.block.walls;
            }
            sk.balc = base.block.balc;
            sk.ground = match base.block.ground {
                Ground::Stoop | Ground::Lattice => Ground::Door,
                g => g,
            };
            sk.top = base.block.top;
            sk.extras = (base.extras & !(PENT | HOIST)) | base.block.extras;
            if sk.balc == Balc::Bay && sk.ground == Ground::Garage {
                sk.ground = Ground::Shop;
            }
        }
        Typology::House => {
            let h = base.house;
            if !h.walls.is_empty() {
                sk.walls = h.walls;
            }
            sk.shut = h.shut;
            sk.balc = h.balc;
            sk.ground = Ground::Door;
            sk.extras = h.extras;
            sk.storey = (3.0, 2.9);
            sk.head = match base.head {
                Head::Pediment => Head::Hood,
                x => x,
            };
            sk.top = if h.roof == Roof::Flat {
                Top::Band
            } else {
                Top::None
            };
        }
    }
    sk
}

/// The usual floors of a typology in a place.
pub fn usual_floors(base: &Skin, t: Typology) -> usize {
    match t {
        Typology::Townhouse => base.floors,
        Typology::Block => base.block.floors,
        Typology::House => base.house.floors,
        Typology::Tower => base.tower.floors,
        Typology::Courtyard => base.house.floors.clamp(1, 2),
        Typology::Row => base.floors.min(3),
    }
}

/// How a typology of a place builds over its plan, and its look.
pub fn shape_of(
    base: &Skin,
    t: Typology,
    floors: usize,
    seed: u64,
    light: bool,
) -> (Skin, Look, Shape) {
    let sk = adapted(base, t);
    let l = look(&sk, seed, t == Typology::House);
    let shape = Shape {
        floors,
        seed,
        light,
        roof: sk.roof,
        cover: sk.cover,
        pitch: sk.pitch,
        eaves: sk.eaves,
        gable: sk.gable,
        top: sk.top,
        extras: sk.extras,
        block: false,
        pavement: true,
        sparse_free: false,
        lift: None,
        steps: true,
        open_below: false,
    };
    match t {
        Typology::Townhouse | Typology::Row | Typology::Courtyard => (sk, l, shape),
        Typology::Block | Typology::Tower => {
            let form = base.block.roof;
            let (cover, pitch) = if form == Roof::Flat {
                (sk.cover, sk.pitch)
            } else if sk.roof == Roof::Flat {
                (Cover::Curved, 0.4)
            } else {
                (sk.cover, sk.pitch.min(0.8))
            };
            let roof_mat = if form == Roof::Flat {
                if sk.roof == Roof::Flat {
                    sk.roof_mat
                } else {
                    "r_gravel"
                }
            } else if sk.roof == Roof::Flat {
                "r_curved"
            } else {
                l.roof
            };
            (
                sk,
                Look {
                    roof: roof_mat,
                    ..l
                },
                Shape {
                    roof: form,
                    cover,
                    pitch,
                    eaves: sk.eaves.max(0.3),
                    gable: Gable::Plain,
                    block: true,
                    sparse_free: true,
                    ..shape
                },
            )
        }
        Typology::House => {
            let h = base.house;
            let raised = h.raised;
            let deck = matches!(h.porch, Porch::Veranda | Porch::Wraparound);
            (
                sk,
                Look {
                    roof: if h.roof_mat.is_empty() {
                        l.roof
                    } else {
                        h.roof_mat
                    },
                    ..l
                },
                Shape {
                    roof: h.roof,
                    cover: h.cover,
                    pitch: h.pitch,
                    eaves: if h.roof == Roof::Asian { 0.9 } else { 0.5 },
                    gable: Gable::Plain,
                    sparse_free: true,
                    pavement: false,
                    lift: Some(raised),
                    steps: !deck,
                    open_below: raised > 0.5,
                    ..shape
                },
            )
        }
    }
}

/// A building of a typology of a place over any plan (the light version
/// for afar), as a model: what the surroundings brought from a map wear.
pub fn dress(
    plan: &Plan,
    t: Typology,
    skin: usize,
    floors: usize,
    seed: u64,
    light: bool,
) -> crate::PropMesh {
    dress_with(
        plan,
        t,
        skin,
        Era::Old,
        State::Standing,
        floors,
        seed,
        light,
    )
}

/// When a building of a place is from: the old one of the place or the
/// one built today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Era {
    Old,
    Today,
}

impl Era {
    pub fn of(n: f64) -> Era {
        if n.round() as i64 == 1 {
            Era::Today
        } else {
            Era::Old
        }
    }
}

/// The skin of a place in an era.
pub fn skin_with(skin: usize, era: Era) -> Skin {
    let base = skin_of(skin as f64);
    match era {
        Era::Old => *base,
        Era::Today => super::skins::contemporary(base),
    }
}

/// The skin a recipe asks for (`skin` and `era`).
pub(super) fn skin_for(p: &Params) -> Skin {
    skin_with(
        p.get("skin", 0.0).round().max(0.0) as usize,
        Era::of(p.get("era", 0.0)),
    )
}

/// [`dress`] of an era and in a state.
#[allow(clippy::too_many_arguments)]
pub fn dress_with(
    plan: &Plan,
    t: Typology,
    skin: usize,
    era: Era,
    state: State,
    floors: usize,
    seed: u64,
    light: bool,
) -> crate::PropMesh {
    let owned = skin_with(skin, era);
    let base = &owned;
    let floors = if floors == 0 {
        usual_floors(base, t)
    } else {
        floors
    };
    let mut b = Builder::default();
    if state == State::Building {
        super::state::site_on(&mut b, plan, base, floors, seed, light);
        return crate::PropMesh {
            positions: b.positions,
            parts: b.parts.into_iter().collect(),
            soft: b.soft.into_iter().collect(),
            fine: b.fine.into_iter().collect(),
        };
    }
    match t {
        Typology::Tower => {
            let l = super::towers::tower_on(&mut b, plan, base, floors, seed, light);
            finish(&mut b, &l);
        }
        Typology::Courtyard => {
            let l = super::towers::courtyard_on(&mut b, plan, base, floors, seed, light);
            finish(&mut b, &l);
        }
        _ => {
            let t = if t == Typology::Row {
                Typology::Townhouse
            } else {
                t
            };
            let (sk, l, shape) = shape_of(base, t, floors, seed, light);
            building(&mut b, plan, &sk, &l, &shape);
            finish(&mut b, &l);
        }
    }
    if state == State::Abandoned {
        super::state::abandon(&mut b, plan, seed);
    }
    crate::PropMesh {
        positions: b.positions,
        parts: b.parts.into_iter().collect(),
        soft: b.soft.into_iter().collect(),
        fine: b.fine.into_iter().collect(),
    }
}

pub(super) fn light_of(p: &Params) -> bool {
    p.get("lod", 0.0) > 0.5
}

/// The house between party walls of a street.
pub fn townhouse(b: &mut Builder, p: &Params) {
    let owned = skin_for(p);
    let base = &owned;
    let seed = seed_of(p);
    let w = p.get("width", 0.0);
    let w = if w <= 0.0 {
        usual_width(base)
    } else {
        w.clamp(4.0, 30.0)
    };
    let d = p.get("depth", 12.0).clamp(7.0, 25.0);
    let floors = floors_of(p, base.floors, 10);
    let (sk, l, shape) = shape_of(base, Typology::Townhouse, floors, seed, light_of(p));
    let plan = Plan::rect(w, d, [Side::Back, Side::Party, Side::Street, Side::Party]);
    building(b, &plan, &sk, &l, &shape);
    finish(b, &l);
}

/// The block of flats, free on its four sides.
pub fn block(b: &mut Builder, p: &Params) {
    let owned = skin_for(p);
    let base = &owned;
    let seed = seed_of(p);
    let w = p.get("width", 22.0).clamp(10.0, 60.0);
    let d = p.get("depth", 14.0).clamp(9.0, 24.0);
    let floors = floors_of(p, base.block.floors, 16);
    let (sk, l, shape) = shape_of(base, Typology::Block, floors, seed, light_of(p));
    let plan = Plan::rect(w, d, [Side::Rear, Side::Free, Side::Street, Side::Free]);
    building(b, &plan, &sk, &l, &shape);
    finish(b, &l);
}

/// The detached house in its plot.
pub fn house(b: &mut Builder, p: &Params) {
    let owned = skin_for(p);
    let base = &owned;
    let seed = seed_of(p);
    let h = base.house;
    let w = p.get("width", 10.0).clamp(6.0, 20.0);
    let d = p.get("depth", 8.0).clamp(6.0, 16.0);
    let floors = floors_of(p, h.floors, 3);
    let (sk, l, shape) = shape_of(base, Typology::House, floors, seed, light_of(p));
    let plan = Plan::rect(w, d, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let built = building(b, &plan, &sk, &l, &shape);
    let raised = h.raised;
    if raised > 0.5 {
        stilts(b, w, d, raised, &l);
    }
    // The street's front (side 2) and its door, as the front put them.
    let fr = built.fronts[2].expect("the house's front");
    let axes = &built.axes[2];
    let n = axes.len();
    let door_s = axes[if n % 2 == 1 {
        n / 2
    } else if unit(fr.seed, 11) > 0.5 {
        0
    } else {
        n - 1
    }];
    // The face of side 2 runs from right to left seen from outside:
    // its `s` from x = -w/2.
    let fface = Face::sides(w, d)[0];
    let door_x = door_s - w / 2.0;
    porch(
        b,
        h.porch,
        &fface,
        w,
        d,
        raised,
        &fr,
        &l,
        h.roof == Roof::Flat,
        door_s,
    );
    if !shape.light {
        plot(b, h.fence, w, d, door_x, raised, seed, &l, h.extras);
    }
    finish(b, &l);
}

/// Posts under a house on stilts and the lattice round them.
fn stilts(b: &mut Builder, w: f64, d: f64, raised: f64, l: &Look) {
    let nx = (w / 2.5).ceil() as usize;
    let ny = (d / 2.5).ceil() as usize;
    for i in 0..=nx {
        for j in 0..=ny {
            let x = -w / 2.0 + 0.2 + (w - 0.4) * i as f64 / nx as f64;
            let y = 0.2 + (d - 0.4) * j as f64 / ny as f64;
            b.rbox(
                "c_concrete",
                [x - 0.12, y - 0.12, 0.0],
                [x + 0.12, y + 0.12, raised],
                0.0,
            );
        }
    }
    b.rbox(
        l.door,
        [-w / 2.0, 0.0, raised - 0.25],
        [w / 2.0, d, raised],
        0.0,
    );
    for f in Face::sides(w, d) {
        grille_plane(
            b,
            &f,
            l.trim,
            (0.3, f.len - 0.3),
            (0.0, raised - 0.25),
            -0.05,
            0.1,
            2,
        );
    }
}

/// The porch of a house.
#[allow(clippy::too_many_arguments)]
fn porch(
    b: &mut Builder,
    kind: Porch,
    f: &Face,
    w: f64,
    d: f64,
    raised: f64,
    fr: &Front,
    l: &Look,
    flat: bool,
    door_s: f64,
) {
    let zg = fr.z(1).min(fr.top());
    match kind {
        Porch::None => {}
        Porch::Portico => {
            let c = door_s;
            let (a, e) = ((c - 1.1).max(0.1), (c + 1.1).min(w - 0.1));
            let out = 1.7;
            let floor = raised.max(0.15);
            f.bx(b, l.base, (a, e), (0.0, floor), (-0.02, out));
            let zp = (fr.lift + 2.95).min(zg - 0.05);
            for x in [a + 0.2, e - 0.2] {
                f.bar(b, l.trim, (x, floor), (x, zp), out - 0.2, 0.09, 12);
            }
            f.bx(b, l.trim, (a, e), (zp, zp + 0.2), (-0.02, out));
            let fp = f.moved(out);
            fp.prism(
                b,
                l.roof,
                &[
                    (a - 0.05, zp + 0.2),
                    (e + 0.05, zp + 0.2),
                    ((a + e) / 2.0, zp + 0.7),
                ],
                (-out, 0.05),
            );
            fp.prism(
                b,
                l.trim,
                &[
                    (a + 0.1, zp + 0.24),
                    (e - 0.1, zp + 0.24),
                    ((a + e) / 2.0, zp + 0.6),
                ],
                (0.05, 0.08),
            );
            if raised > 0.3 {
                steps_out(b, f, (c - 0.7, c + 0.7), out, raised, l);
            }
        }
        Porch::Veranda | Porch::Wraparound => {
            let out = 2.4;
            let all = kind == Porch::Wraparound;
            let sides: Vec<Face> = if all {
                faces((-w / 2.0, w / 2.0), (0.0, d)).to_vec()
            } else {
                vec![*f]
            };
            let floor = raised.max(0.25);
            let zr = zg - 0.1;
            for (i, sf) in sides.iter().enumerate() {
                let ext = if all && i % 2 == 0 { out } else { 0.0 };
                let s = (-ext, sf.len + ext);
                sf.bx(b, l.door, s, (floor - 0.15, floor), (-0.02, out));
                if raised > 0.3 {
                    sf.bx(b, l.trim, s, (0.0, floor - 0.15), (out - 0.1, out));
                }
                // Posts and rails.
                let n = ((s.1 - s.0) / 2.3).round().max(1.0) as usize;
                for k in 0..=n {
                    let x = s.0 + 0.1 + (s.1 - s.0 - 0.2) * k as f64 / n as f64;
                    sf.bx(
                        b,
                        l.trim,
                        (x - 0.07, x + 0.07),
                        (floor, zr),
                        (out - 0.2, out - 0.06),
                    );
                }
                if raised > 0.3 {
                    let door = if i == 0 {
                        Some((door_s - 0.7, door_s + 0.7))
                    } else {
                        None
                    };
                    let mut runs = vec![(s.0 + 0.1, s.1 - 0.1)];
                    if let Some((a, e)) = door {
                        runs = vec![(s.0 + 0.1, a), (e, s.1 - 0.1)];
                    }
                    for (a, e) in runs {
                        if e - a > 0.3 {
                            railing(b, sf, l.trim, (a, e), floor, out - 0.13, 0.9, 0.12);
                        }
                    }
                } else {
                    let _ = 0;
                }
                sf.leaning(b, l.roof, s, (zr + 0.4, 0.0), (zr, out + 0.15), 0.05);
                sf.bx(
                    b,
                    l.trim,
                    s,
                    (zr - 0.2, zr + 0.08),
                    (out - 0.22, out - 0.04),
                );
            }
            if raised > 0.3 {
                steps_out(b, f, (door_s - 0.7, door_s + 0.7), out, raised, l);
            }
        }
        Porch::Arcade => {
            let out = 2.4;
            let n = ((w - 0.4) / 3.0).round().max(2.0) as usize;
            let top = zg - 0.05;
            let fp = f.moved(out);
            let mut piers = Vec::new();
            for k in 0..=n {
                let x = 0.2 + (w - 0.4) * k as f64 / n as f64;
                let x = x.clamp(0.25, w - 0.25);
                piers.push((x - 0.22, x + 0.22));
            }
            for &(a, e) in &piers {
                fp.bx(b, l.trim, (a, e), (0.0, top), (-0.44, 0.0));
            }
            for p in piers.windows(2) {
                let (a, e) = (p[0].1, p[1].0);
                let r = (e - a) / 2.0;
                let spring = top - 0.45 - r;
                if spring > 1.8 {
                    super::super::ensanche::arch_fill(b, &fp, l.wall, (a, e), spring, (-0.44, 0.0));
                }
            }
            fp.bx(b, l.wall, (0.0, w), (top - 0.45, top), (-0.44, 0.0));
            f.bx(b, "paving", (0.0, w), (0.0, 0.05), (0.0, out));
            if flat {
                f.bx(b, l.wall, (0.0, w), (top, top + 0.25), (-0.02, out));
                fp.bx(b, l.wall, (0.0, w), (top + 0.25, top + 1.0), (-0.2, 0.0));
            } else {
                f.leaning(
                    b,
                    l.roof,
                    (-0.1, w + 0.1),
                    (top + 0.45, 0.0),
                    (top, out + 0.2),
                    0.06,
                );
            }
        }
    }
}

/// Steps from a raised porch down to the garden, `out` from the wall.
fn steps_out(b: &mut Builder, f: &Face, s: (f64, f64), out: f64, lift: f64, l: &Look) {
    let n = (lift / 0.18).ceil().max(1.0) as usize;
    let rise = lift / n as f64;
    let run = 0.28;
    for j in 0..n.saturating_sub(1) {
        f.bx(
            b,
            l.base,
            s,
            (0.0, lift - rise * (j + 1) as f64),
            (out - 0.05, out + run * (j + 1) as f64),
        );
    }
}

/// The plot round a house: grass, the path to the door, the fence or the
/// wall with its gate, and a tree.
#[allow(clippy::too_many_arguments)]
fn plot(
    b: &mut Builder,
    fence: Fence,
    w: f64,
    d: f64,
    door_x: f64,
    raised: f64,
    seed: u64,
    l: &Look,
    extras: u32,
) {
    let px = (-w / 2.0 - 3.0, w / 2.0 + 3.0);
    let py = (-3.0, d + 5.5);
    b.rbox("lawn", [px.0, py.0, 0.0], [px.1, py.1, 0.02], 0.0);
    let path_from = d + if raised > 0.3 {
        2.4 + (raised / 0.18).ceil() * 0.28
    } else {
        0.0
    };
    if path_from < py.1 {
        b.rbox(
            "paving",
            [door_x - 0.6, path_from, 0.0],
            [door_x + 0.6, py.1, 0.035],
            0.0,
        );
    }
    let gate = (door_x - 0.7 - px.0, door_x + 0.7 - px.0);
    let fronts = faces(px, py);
    // The front with its gate, and the two sides.
    for (i, f) in fronts.iter().enumerate() {
        if i == 2 {
            continue;
        }
        let runs: Vec<(f64, f64)> = if i == 0 {
            vec![(0.0, gate.0), (gate.1, f.len)]
        } else {
            vec![(0.0, f.len)]
        };
        for (a, e) in runs {
            if e - a < 0.2 {
                continue;
            }
            match fence {
                Fence::None => {}
                Fence::Picket => {
                    f.bx(b, "b_white", (a, e), (0.3, 0.38), (-0.06, -0.03));
                    f.bx(b, "b_white", (a, e), (0.75, 0.83), (-0.06, -0.03));
                    let mut x = a + 0.05;
                    while x < e - 0.03 {
                        f.prism(
                            b,
                            "b_white",
                            &[
                                (x - 0.035, 0.0),
                                (x + 0.035, 0.0),
                                (x + 0.035, 0.95),
                                (x, 1.02),
                                (x - 0.035, 0.95),
                            ],
                            (-0.03, 0.0),
                        );
                        x += 0.13;
                    }
                }
                Fence::Wall => {
                    f.bx(b, l.wall, (a, e), (0.0, 1.1), (-0.25, 0.0));
                    f.bx(b, l.trim, (a - 0.02, e + 0.02), (1.1, 1.17), (-0.28, 0.03));
                }
                Fence::Hedge => {
                    let c = f.at((a + e) / 2.0, 0.0, -0.4);
                    let half = (e - a) / 2.0;
                    let lo = [
                        c[0] - (f.u[0] * half).abs() - 0.35 * f.u[1].abs(),
                        c[1] - (f.u[1] * half).abs() - 0.35 * f.u[0].abs(),
                        0.0,
                    ];
                    let hi = [
                        c[0] + (f.u[0] * half).abs() + 0.35 * f.u[1].abs(),
                        c[1] + (f.u[1] * half).abs() + 0.35 * f.u[0].abs(),
                        1.2,
                    ];
                    b.rbox("hedge", lo, hi, 0.25);
                }
                Fence::Iron => {
                    f.bx(b, l.wall, (a, e), (0.0, 0.5), (-0.25, 0.0));
                    f.bx(b, l.trim, (a - 0.02, e + 0.02), (0.5, 0.56), (-0.28, 0.03));
                    railing(b, f, "iron", (a + 0.05, e - 0.05), 0.56, -0.125, 0.95, 0.12);
                }
            }
        }
    }
    // The gate.
    let f = &fronts[0];
    match fence {
        Fence::Wall | Fence::Iron => {
            for x in [gate.0, gate.1] {
                f.bx(b, l.wall, (x - 0.22, x + 0.22), (0.0, 1.5), (-0.3, 0.05));
                f.bx(b, l.trim, (x - 0.26, x + 0.26), (1.5, 1.58), (-0.34, 0.09));
            }
            grille_plane(
                b,
                f,
                "iron",
                (gate.0 + 0.22, gate.1 - 0.22),
                (0.05, 1.3),
                -0.12,
                0.11,
                3,
            );
        }
        Fence::Picket => {
            for x in [gate.0, gate.1] {
                f.bx(b, "b_white", (x - 0.05, x + 0.05), (0.0, 1.1), (-0.08, 0.0));
            }
            f.bx(
                b,
                "b_white",
                (gate.0 + 0.05, gate.1 - 0.05),
                (0.3, 0.38),
                (-0.06, -0.03),
            );
            f.bx(
                b,
                "b_white",
                (gate.0 + 0.05, gate.1 - 0.05),
                (0.75, 0.83),
                (-0.06, -0.03),
            );
            let mut x = gate.0 + 0.1;
            while x < gate.1 - 0.05 {
                f.bx(
                    b,
                    "b_white",
                    (x - 0.03, x + 0.03),
                    (0.3, 0.95),
                    (-0.03, 0.0),
                );
                x += 0.13;
            }
        }
        _ => {}
    }
    // A mailbox for the picket fences, a tree, a palm.
    if fence == Fence::Picket {
        let p = f.at(gate.1 + 0.6, 0.0, -0.4);
        b.rbox(
            "metal",
            [p[0] - 0.03, p[1] - 0.03, 0.0],
            [p[0] + 0.03, p[1] + 0.03, 1.05],
            0.0,
        );
        b.rbox(
            l.door,
            [p[0] - 0.12, p[1] - 0.25, 1.05],
            [p[0] + 0.12, p[1] + 0.25, 1.3],
            0.06,
        );
    }
    if extras & TREE != 0 {
        fruit_tree(
            b,
            [px.0 + 1.6, d + 3.0],
            4.2 + 1.2 * unit(seed, 31),
            seed,
            false,
        );
    }
    if extras & PALM != 0 {
        palm(b, [px.1 - 1.4, d + 3.2], 6.5 + 2.0 * unit(seed, 32), seed);
    }
    let _ = REVEAL;
}

/// The sample plans of the workshop: an L on a corner, a corner with a
/// chamfer, a slanted plot between party walls.
pub fn sample_plans() -> Vec<Plan> {
    vec![
        Plan {
            poly: vec![
                [-8.0, 0.0],
                [8.0, 0.0],
                [8.0, 7.0],
                [-1.0, 7.0],
                [-1.0, 18.0],
                [-8.0, 18.0],
            ],
            sides: vec![
                Side::Free,
                Side::Street,
                Side::Free,
                Side::Street,
                Side::Free,
                Side::Party,
            ],
        },
        Plan {
            poly: vec![
                [-10.0, 0.0],
                [10.0, 0.0],
                [10.0, 12.0],
                [7.0, 15.0],
                [-10.0, 15.0],
            ],
            sides: vec![
                Side::Party,
                Side::Street,
                Side::Street,
                Side::Street,
                Side::Party,
            ],
        },
        Plan {
            poly: vec![[-6.0, 0.0], [6.0, 2.0], [5.0, 13.0], [-7.0, 11.0]],
            sides: vec![Side::Back, Side::Party, Side::Street, Side::Party],
        },
    ]
}

/// A sample of the workshop: a typology of a place over one of the sample
/// plans (`plan`, `typology` 0 street, 1 block, 2 house; `lod` 1 light).
pub fn sample(b: &mut Builder, p: &Params) {
    let plans = sample_plans();
    let plan = &plans[(p.get("plan", 0.0).round().max(0.0) as usize).min(plans.len() - 1)];
    let t = match p.get("typology", 0.0).round() as i64 {
        1 => Typology::Block,
        2 => Typology::House,
        3 => Typology::Tower,
        4 => Typology::Courtyard,
        _ => Typology::Townhouse,
    };
    let owned = skin_for(p);
    let floors = floors_of(p, usual_floors(&owned, t), 60);
    let m = dress_with(
        plan,
        t,
        p.get("skin", 0.0).round().max(0.0) as usize,
        Era::of(p.get("era", 0.0)),
        State::of(p.get("state", 0.0)),
        floors,
        seed_of(p),
        light_of(p),
    );
    let at = b.positions.len() as u32;
    b.positions.extend(m.positions);
    for (role, tris) in m.parts {
        b.parts
            .entry(role)
            .or_default()
            .extend(tris.into_iter().map(|t| t.map(|i| i + at)));
    }
    b.fine.extend(m.fine);
}

type DressCache = std::sync::Mutex<std::collections::HashMap<u64, std::sync::Arc<crate::PropMesh>>>;

/// [`dress`], kept: the same plan and choices make the same model once.
pub fn dress_cached(
    plan: &Plan,
    t: Typology,
    skin: usize,
    floors: usize,
    seed: u64,
    light: bool,
) -> std::sync::Arc<crate::PropMesh> {
    dress_cached_with(
        plan,
        t,
        skin,
        Era::Old,
        State::Standing,
        floors,
        seed,
        light,
    )
}

/// [`dress_with`], kept.
#[allow(clippy::too_many_arguments)]
pub fn dress_cached_with(
    plan: &Plan,
    t: Typology,
    skin: usize,
    era: Era,
    state: State,
    floors: usize,
    seed: u64,
    light: bool,
) -> std::sync::Arc<crate::PropMesh> {
    use std::hash::{Hash, Hasher};
    static CACHE: std::sync::OnceLock<DressCache> = std::sync::OnceLock::new();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in &plan.poly {
        p[0].to_bits().hash(&mut h);
        p[1].to_bits().hash(&mut h);
    }
    for s in &plan.sides {
        (*s as u8).hash(&mut h);
    }
    (t as u8, skin, era, state as u8, floors, seed, light).hash(&mut h);
    let key = h.finish();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return m;
    }
    let m = std::sync::Arc::new(dress_with(plan, t, skin, era, state, floors, seed, light));
    if let Ok(mut c) = cache.lock() {
        if c.len() > 4000 {
            c.clear();
        }
        c.insert(key, m.clone());
    }
    m
}
