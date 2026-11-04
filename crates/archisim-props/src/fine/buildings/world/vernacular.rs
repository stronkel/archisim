//! The popular buildings that were left (tanda 15): the granary on its
//! feet (hórreo, Galician of stone or Asturian of wood), the round stone
//! house under its thatch (palloza), the stone house of the sierra, the
//! courtyard house of one floor and the block of social housing (VPO).

use super::super::*;
use super::civic::{tree, walls_box, Ctx};
use super::plan::{building, Plan, Side};
use super::roofs::{self, RoofPlan};
use super::skins::*;
use super::types::{shape_of, Typology};
use super::{look, Look};
use crate::build::Builder;
use std::f64::consts::PI;

fn roof_plan(
    x: (f64, f64),
    y: (f64, f64),
    z: f64,
    form: Roof,
    pitch: f64,
    light: bool,
) -> RoofPlan {
    RoofPlan {
        x,
        y,
        z,
        form,
        cover: Cover::Curved,
        pitch,
        eaves: 0.35,
        party: false,
        dormers: Vec::new(),
        gable: Gable::Plain,
        seed: 1,
        extras: 0,
        top: Top::None,
        block: false,
        light,
    }
}

/// The granary of the place: of wood on four feet in Asturias and the
/// mountain, long and of stone in Galicia and the rest.
pub(super) fn granary(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let l = Look {
        roof: "r_curved",
        wall: "s_granite",
        ..l0
    };
    b.rbox("lawn", [-c.w / 2.0, 0.0, 0.0], [c.w / 2.0, c.d, 0.03], 0.0);
    if c.sk.key == "montana" {
        asturian(b, &l, c);
    } else {
        galician(b, &l, c);
    }
    l
}

/// The Galician hórreo: stone feet with their round slabs, a long chamber
/// of stone slats, solid gables with a cross and a pinnacle.
fn galician(b: &mut Builder, l: &Look, c: &Ctx) {
    let len = (c.w - 2.0).clamp(5.0, 11.0);
    let wd = 1.7;
    let cy = c.d / 2.0;
    let (x0, x1) = (-len / 2.0, len / 2.0);
    let (y0, y1) = (cy - wd / 2.0, cy + wd / 2.0);
    let n = ((len / 2.2).round() as usize).max(2);
    let hf = 1.3;
    for i in 0..=n {
        let x = x0 + 0.3 + (len - 0.6) * i as f64 / n as f64;
        for y in [y0 + 0.3, y1 - 0.3] {
            b.rbox(
                "s_granite",
                [x - 0.18, y - 0.18, 0.0],
                [x + 0.18, y + 0.18, hf],
                0.0,
            );
            b.cone("s_granite", [x, y, hf], [x, y, hf + 0.12], 0.42, 0.42, 12);
        }
    }
    let zb = hf + 0.12;
    b.rbox(
        "s_granite",
        [x0 - 0.1, y0 - 0.1, zb],
        [x1 + 0.1, y1 + 0.1, zb + 0.2],
        0.0,
    );
    let zc = zb + 0.2;
    let h = 1.8;
    // The gables, solid, and the corner posts.
    for x in [x0, x1 - 0.25] {
        b.rbox("s_granite", [x, y0, zc], [x + 0.25, y1, zc + h], 0.0);
    }
    // The slats.
    let slats = ((len - 0.5) / 0.32).floor() as usize;
    for i in 0..slats {
        let x = x0 + 0.25 + 0.04 + 0.32 * i as f64;
        if c.light {
            break;
        }
        for y in [y0, y1 - 0.12] {
            b.rbox("s_granite", [x, y, zc], [x + 0.2, y + 0.12, zc + h], 0.0);
        }
    }
    b.rbox(
        "interior",
        [x0 + 0.25, y0 + 0.13, zc],
        [x1 - 0.25, y1 - 0.13, zc + h],
        0.0,
    );
    if c.light {
        b.rbox("s_granite", [x0, y0, zc], [x1, y1, zc + h], 0.0);
    }
    let zt = zc + h;
    b.rbox(
        "s_granite",
        [x0, y0 - 0.05, zt],
        [x1, y1 + 0.05, zt + 0.12],
        0.0,
    );
    roofs::roof(
        b,
        &RoofPlan {
            eaves: 0.25,
            ..roof_plan(
                (x0, x1),
                (y0 - 0.05, y1 + 0.05),
                zt + 0.12,
                Roof::Gable,
                0.55,
                c.light,
            )
        },
        l,
    );
    // The cross and the pinnacle on the gables.
    let apex = zt + 0.12 + (wd / 2.0 + 0.05) * 0.55;
    b.rbox(
        "s_granite",
        [x0 - 0.02, cy - 0.06, apex - 0.1],
        [x0 + 0.1, cy + 0.06, apex + 0.9],
        0.0,
    );
    b.rbox(
        "s_granite",
        [x0 - 0.02, cy - 0.3, apex + 0.5],
        [x0 + 0.1, cy + 0.3, apex + 0.62],
        0.0,
    );
    b.cone(
        "s_granite",
        [x1 - 0.06, cy, apex - 0.1],
        [x1 - 0.06, cy, apex + 0.9],
        0.12,
        0.02,
        8,
    );
    // The steps beside it, not touching (the rats).
    if !c.light {
        for k in 0..4 {
            let zz = 0.3 * (k + 1) as f64;
            b.rbox(
                "s_granite",
                [x1 + 0.4 + 0.3 * (3 - k) as f64, y0, 0.0],
                [x1 + 0.4 + 0.3 * (4 - k) as f64, y1, zz],
                0.0,
            );
        }
    }
}

/// The Asturian hórreo: a square chamber of boards on four stone feet, a
/// gallery round it and a hipped roof of tiles.
fn asturian(b: &mut Builder, l: &Look, c: &Ctx) {
    let s = (c.w.min(c.d) - 2.0).clamp(4.0, 6.5);
    let cy = c.d / 2.0;
    let (x0, x1, y0, y1) = (-s / 2.0, s / 2.0, cy - s / 2.0, cy + s / 2.0);
    let hf = 1.6;
    for (x, y) in [
        (x0 + 0.5, y0 + 0.5),
        (x1 - 0.5, y0 + 0.5),
        (x1 - 0.5, y1 - 0.5),
        (x0 + 0.5, y1 - 0.5),
    ] {
        b.cone("s_granite", [x, y, 0.0], [x, y, hf], 0.45, 0.22, 4);
        b.rbox(
            "s_granite",
            [x - 0.4, y - 0.4, hf],
            [x + 0.4, y + 0.4, hf + 0.12],
            0.0,
        );
    }
    let zb = hf + 0.12;
    b.rbox("b_cedar", [x0, y0, zb], [x1, y1, zb + 0.25], 0.0);
    let zc = zb + 0.25;
    let h = 2.0;
    let door = Hole {
        s: (s / 2.0 - 0.45, s / 2.0 + 0.45),
        z: (zc, zc + 1.6),
    };
    let fs = walls_box(
        b,
        (x0 + 0.4, x1 - 0.4),
        (y0 + 0.4, y1 - 0.4),
        (zc, zc + h),
        "b_cedar",
        [
            &[Hole {
                s: (door.s.0 - 0.4, door.s.1 - 0.4),
                z: door.z,
            }],
            &[],
            &[],
            &[],
        ],
        true,
    );
    fs[0].bx(
        b,
        "b_dark",
        (door.s.0 - 0.4, door.s.1 - 0.4),
        door.z,
        (-0.22, -0.16),
    );
    if !c.light {
        // The boards' joints.
        for f in &fs {
            let n = (f.len / 0.35).floor() as usize;
            for i in 1..n {
                let x = 0.35 * i as f64;
                f.bx(
                    b,
                    "b_dark",
                    (x - 0.015, x + 0.015),
                    (zc, zc + h),
                    (0.0, 0.02),
                );
            }
        }
        // The gallery's rail.
        for f in roofs::faces((x0, x1), (y0, y1)) {
            let n = (f.len / 0.25).floor() as usize;
            for i in 0..=n {
                let x = f.len * i as f64 / n as f64;
                f.bx(
                    b,
                    "b_cedar",
                    (x - 0.03, x + 0.03),
                    (zc, zc + 0.9),
                    (-0.08, 0.0),
                );
            }
            f.bx(
                b,
                "b_cedar",
                (0.0, f.len),
                (zc + 0.9, zc + 0.98),
                (-0.1, 0.0),
            );
        }
    }
    b.rbox(
        "b_cedar",
        [x0, y0, zc + h - 0.06],
        [x1, y1, zc + h + 0.06],
        0.0,
    );
    roofs::roof(
        b,
        &RoofPlan {
            eaves: 0.5,
            ..roof_plan((x0, x1), (y0, y1), zc + h, Roof::Hip, 0.5, c.light)
        },
        l,
    );
    // Posts of the gallery's roof at the corners.
    for (x, y) in [
        (x0 + 0.1, y0 + 0.1),
        (x1 - 0.1, y0 + 0.1),
        (x1 - 0.1, y1 - 0.1),
        (x0 + 0.1, y1 - 0.1),
    ] {
        b.rbox(
            "b_cedar",
            [x - 0.08, y - 0.08, zc],
            [x + 0.08, y + 0.08, zc + h],
            0.0,
        );
    }
}

/// The palloza: a round wall of stone, low, under a steep cone of thatch,
/// its door and a woodpile.
pub(super) fn palloza(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let l = Look {
        wall: "s_rubble",
        ..l0
    };
    let (w, d) = (c.w, c.d);
    b.rbox("lawn", [-w / 2.0, 0.0, 0.0], [w / 2.0, d, 0.03], 0.0);
    let r = (w.min(d) / 2.0 - 1.5).clamp(4.0, 8.0);
    let cy = d / 2.0;
    let sides = 24;
    let ring = |rr: f64, sx: f64| -> Vec<[f64; 2]> {
        (0..sides)
            .map(|k| {
                let a = 2.0 * PI * k as f64 / sides as f64;
                [rr * sx * a.cos(), cy + rr * a.sin()]
            })
            .collect()
    };
    let hw = 1.7;
    b.slab("s_rubble", &ring(r, 1.0), 0.0, hw);
    // The thatch, steep, from the top of the wall.
    b.cone(
        "r_thatch",
        [0.0, cy, hw - 0.1],
        [0.0, cy, hw + r * 1.2],
        r * 1.08,
        0.25,
        sides,
    );
    // The door in the wall to the street: its jambs and lintel.
    let f = Face {
        o: [-0.6, cy + r - 0.2],
        u: [1.0, 0.0],
        len: 1.2,
    };
    f.bx(b, "b_dark", (0.0, 1.2), (0.0, 1.5), (0.0, 0.25));
    f.bx(b, "s_granite", (-0.25, 0.0), (0.0, 1.6), (0.0, 0.3));
    f.bx(b, "s_granite", (1.2, 1.45), (0.0, 1.6), (0.0, 0.3));
    f.bx(b, "s_granite", (-0.3, 1.5), (1.5, 1.7), (0.0, 0.3));
    if !c.light {
        // The woodpile by the wall: the stack and the ends of its logs.
        let (lx, ly) = (r * 0.55, cy + r * 0.82);
        b.rbox("b_cedar", [lx, ly, 0.0], [lx + 1.0, ly + 1.0, 0.72], 0.0);
        for k in 0..12 {
            let x = lx + 0.13 + 0.245 * (k % 4) as f64;
            let z = 0.12 + 0.24 * (k / 4) as f64;
            b.cone("trunk", [x, ly + 1.0, z], [x, ly + 1.04, z], 0.11, 0.11, 8);
        }
        tree(b, [-w / 2.0 + 1.5, 1.5], 0.0, 7.0, c.seed);
    }
    l
}

/// The stone house of the sierra: the place's house built of rubble with
/// a roof of tiles, the chimney and its yard behind a dry-stone wall.
pub(super) fn stone_house(b: &mut Builder, c: &Ctx) -> Look {
    let mut base = *c.sk;
    base.house.walls = &["s_rubble", "s_granite"];
    base.house.roof = Roof::Gable;
    base.house.cover = Cover::Curved;
    base.house.roof_mat = "r_curved";
    base.house.pitch = 0.45;
    base.house.raised = 0.0;
    base.house.porch = Porch::None;
    base.house.balc = Balc::None;
    base.house.extras = CHIMNEYS;
    base.trims = &["s_granite"];
    base.base = &["s_granite"];
    let (w, d) = (c.w, c.d);
    let (hw, hd) = ((w - 3.0).clamp(7.0, 12.0), (d - 4.0).clamp(6.0, 9.0));
    let (sk, l, shape) = shape_of(
        &base,
        Typology::House,
        c.floors.clamp(1, 2),
        c.seed,
        c.light,
    );
    let plan = Plan {
        poly: vec![
            [-hw / 2.0, 1.0],
            [hw / 2.0, 1.0],
            [hw / 2.0, 1.0 + hd],
            [-hw / 2.0, 1.0 + hd],
        ],
        sides: vec![Side::Back, Side::Free, Side::Street, Side::Free],
    };
    building(b, &plan, &sk, &l, &shape);
    // The dry-stone wall round the yard, its gap at the front.
    if !c.light {
        for f in roofs::faces((-w / 2.0, w / 2.0), (0.0, d)) {
            let gap = f.u[0] > 0.5;
            let mid = f.len / 2.0;
            let spans: Vec<(f64, f64)> = if gap {
                vec![(0.0, mid - 1.2), (mid + 1.2, f.len)]
            } else {
                vec![(0.0, f.len)]
            };
            for (s0, s1) in spans {
                f.bx(b, "s_rubble", (s0, s1), (0.0, 1.0), (-0.5, 0.0));
            }
        }
    }
    l
}

/// The courtyard house of one floor.
pub(super) fn patio_house(b: &mut Builder, c: &Ctx) -> Look {
    let plan = Plan::rect(
        c.w,
        c.d,
        [Side::Back, Side::Party, Side::Street, Side::Party],
    );
    super::towers::courtyard_on(b, &plan, c.sk, 1, c.seed, c.light)
}

/// The block of social housing: brick, plain windows with their shutters,
/// a flat roof, the place's balconies left out.
pub(super) fn social_housing(b: &mut Builder, c: &Ctx) -> Look {
    let mut base = *c.sk;
    base.block.walls = &["k_red", "k_dark"];
    base.block.roof = Roof::Flat;
    base.block.balc = Balc::Slab;
    base.block.top = Top::Band;
    base.block.ground = Ground::Door;
    if base.shut == Shut::None {
        base.shut = Shut::Roller;
    }
    base.trims = &["c_concrete"];
    base.accents = &[];
    let (sk, l, shape) = shape_of(&base, Typology::Block, c.floors.max(3), c.seed, c.light);
    let plan = Plan::rect(c.w, c.d, [Side::Rear, Side::Free, Side::Street, Side::Free]);
    building(b, &plan, &sk, &l, &shape);
    l
}
