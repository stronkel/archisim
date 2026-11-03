//! The block of the «ensanche» (1900–1930): a ground floor of ashlar with
//! round arches (the doorway and the shops), plastered floors above with
//! moulded surrounds, a balcony of wrought iron for every hole on brackets,
//! the glazed bays (miradores) of white wood stacked in the middle, the
//! rolled «alicantinas», the string courses and a cornice on corbels, an
//! attic floor behind a balustrade.

use super::*;
use crate::Params;

const WALL: f64 = 0.45;
const REVEAL: f64 = 0.16;

pub fn ensanche(b: &mut Builder, p: &Params) {
    let w = p.get("width", 14.0).clamp(8.0, 30.0);
    let d = p.get("depth", 16.0).clamp(8.0, 30.0);
    let floors = p.get("floors", 5.0).round().clamp(3.0, 8.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let bays_glazed = p.flag("miradores", true);
    let ground = 4.5;
    let upper = 3.3;
    let level = |k: usize| {
        if k == 0 {
            0.0
        } else {
            ground + upper * (k - 1) as f64
        }
    };
    let roof_z = level(floors);
    let [front, right, back, left] = Face::sides(w, d);

    let mut n = ((w - 0.8) / 2.5).floor().max(3.0) as usize;
    if n.is_multiple_of(2) {
        n -= 1;
    }
    let bay = (w - 0.8) / n as f64;
    let axis = |i: usize| 0.4 + bay * (i as f64 + 0.5);
    let mid = n / 2;

    // The ground floor: arches.
    let mut arches = Vec::new();
    for i in 0..n {
        let hw = if i == mid {
            0.95
        } else {
            (bay * 0.36).min(1.0)
        };
        let c = axis(i);
        let spring = 3.0;
        arches.push((
            i,
            Hole {
                s: (c - hw, c + hw),
                z: (if i == mid { 0.18 } else { 0.0 }, spring + hw),
            },
            spring,
        ));
    }
    let holes: Vec<Hole> = arches.iter().map(|a| a.1).collect();
    wall(b, "ashlar", &front, (0.0, ground), WALL, &holes);
    // The joints of the ashlar: shallow grooves as bands.
    let mut z = 0.5;
    while z < ground - 0.3 {
        front.bx(b, "ashlar", (0.0, w), (z, z + 0.04), (-0.02, 0.012));
        z += 0.5;
    }
    front.bx(b, "plinth", (0.0, w), (0.0, 0.5), (-0.02, 0.05));
    for (i, h, spring) in &arches {
        arch_fill(b, &front, "ashlar", h.s, *spring, (-WALL, 0.0));
        voussoirs(b, &front, h.s, *spring);
        if *i == mid {
            let door = Hole {
                s: h.s,
                z: (h.z.0, *spring),
            };
            panel_door(b, &front, door, 2, 0.0, "wood", "iron", REVEAL);
            front.bx(
                b,
                "wood",
                h.s,
                (*spring - 0.02, *spring + 0.06),
                (-REVEAL - 0.1, -REVEAL),
            );
            fan(b, &front, h.s, *spring);
            front.bx(
                b,
                "plinth",
                (h.s.0 - 0.15, h.s.1 + 0.15),
                (0.0, 0.18),
                (-REVEAL, 0.4),
            );
        } else {
            let r = unit(seed, 10 + *i as u64);
            let shop = Hole {
                s: h.s,
                z: (0.0, *spring),
            };
            shop_front(
                b,
                &front,
                shop,
                if r < 0.5 { 0.0 } else { 0.4 },
                false,
                "wood",
                REVEAL,
            );
            front.bx(
                b,
                "wood",
                h.s,
                (*spring - 0.02, *spring + 0.06),
                (-REVEAL - 0.08, -REVEAL),
            );
            fan(b, &front, h.s, *spring);
        }
    }
    front.bx(
        b,
        "trim",
        (0.0, w),
        (ground - 0.35, ground - 0.1),
        (-0.01, 0.12),
    );

    // The floors above.
    let attic = floors >= 4;
    let mut holes = Vec::new();
    let mut parts = Vec::new();
    for k in 1..floors {
        let zf = level(k);
        let is_attic = attic && k == floors - 1;
        for i in 0..n {
            let c = axis(i);
            let glazed =
                bays_glazed && !is_attic && (i == mid.saturating_sub(1) || i == mid + 1) && n >= 5;
            let hw = if glazed { (bay * 0.42).min(1.0) } else { 0.55 };
            let h = Hole {
                s: (c - hw, c + hw),
                z: (zf + 0.02, zf + if is_attic { 2.2 } else { 2.7 }),
            };
            holes.push(h);
            parts.push((k, i, h, glazed, is_attic));
        }
    }
    wall(b, "wall", &front, (ground, roof_z), WALL, &holes);
    for k in 2..floors {
        let z = level(k);
        front.bx(b, "trim", (0.0, w), (z - 0.25, z - 0.1), (-0.01, 0.06));
    }
    let mut bh = Vec::new();
    for k in 0..floors {
        for i in 0..n {
            bh.push(Hole {
                s: (axis(i) - 0.5, axis(i) + 0.5),
                z: (level(k) + 0.9, level(k) + 2.4),
            });
        }
    }
    wall(b, "wall", &back, (0.0, roof_z), WALL, &bh);
    for f in [right, left] {
        wall(b, "party", &f, (0.0, roof_z), WALL, &[]);
    }
    core(b, w, d, WALL, (0.0, roof_z));

    for (j, (k, _i, h, glazed, is_attic)) in parts.iter().enumerate() {
        let key = 200 + j as u64 * 9;
        let win = Win {
            leaves: 2,
            reveal: REVEAL,
            bars: 3,
            fanlight: if *is_attic { 0.0 } else { 0.5 },
            curtain: unit(seed, key) > 0.4,
            ..Default::default()
        };
        if *glazed {
            let last = if attic { floors - 2 } else { floors - 1 };
            mirador(b, &front, h, level(*k), upper, *k == last);
            continue;
        }
        window(b, &front, *h, &win, "frame");
        surround(b, &front, *h, "trim", 0.16, 0.04);
        front.bx(
            b,
            "trim",
            (h.s.0 - 0.28, h.s.1 + 0.28),
            (h.z.1 + 0.16, h.z.1 + 0.26),
            (-0.01, 0.12),
        );
        // A keystone.
        let c = (h.s.0 + h.s.1) / 2.0;
        front.bx(
            b,
            "trim",
            (c - 0.1, c + 0.1),
            (h.z.1 - 0.06, h.z.1 + 0.16),
            (-0.01, 0.07),
        );
        if *is_attic {
            sill(b, &front, *h, "stone", 0.06, REVEAL);
            continue;
        }
        let out = if *k == 1 { 0.7 } else { 0.5 };
        balcony(
            b,
            &front,
            (h.s.0 - 0.3, h.s.1 + 0.3),
            h.z.0,
            out,
            "stone",
            "iron",
            true,
        );
        // The «alicantina»: rolled up under the hood, or let down.
        let r = unit(seed, key + 1);
        if r > 0.35 {
            let down = if r > 0.75 { 0.55 } else { 0.0 };
            alicantina(b, &front, h, down, out);
        }
        if unit(seed, key + 2) > 0.45 {
            standing_pot(
                b,
                front.at(h.s.1 + 0.05, h.z.0, out * 0.55),
                0.8,
                seed + key,
            );
        }
    }
    for h in &bh {
        let win = Win {
            leaves: 2,
            reveal: 0.12,
            blind: 0.5 * unit(seed, (h.z.0 * 10.0) as u64 + (h.s.0 * 10.0) as u64),
            ..Default::default()
        };
        window(b, &back, *h, &win, "frame");
        sill(b, &back, *h, "stone", 0.04, 0.12);
    }

    // The cornice on corbels; the balustrade of the attic (or the
    // parapet).
    let z = if attic { level(floors - 1) } else { roof_z };
    front.bx(b, "trim", (0.0, w), (z - 0.55, z - 0.4), (-0.01, 0.1));
    let mut x = 0.3;
    while x < w - 0.2 {
        front.bx(
            b,
            "trim",
            (x - 0.07, x + 0.07),
            (z - 0.4, z - 0.12),
            (-0.01, 0.42),
        );
        front.bx(
            b,
            "trim",
            (x - 0.05, x + 0.05),
            (z - 0.4, z - 0.3),
            (0.42, 0.48),
        );
        x += 0.6;
    }
    front.bx(
        b,
        "trim",
        (-0.05, w + 0.05),
        (z - 0.12, z + 0.08),
        (-0.01, 0.62),
    );
    if attic {
        // The attic's floor is set back behind a balustrade.
        let bz = z + 0.08;
        let mut x = 0.15;
        while x < w - 0.1 {
            let c = front.at(x, 0.0, 0.42);
            b.lathe(
                "trim",
                [c[0], c[1], bz],
                &[
                    (0.07, 0.0),
                    (0.05, 0.12),
                    (0.09, 0.32),
                    (0.04, 0.6),
                    (0.07, 0.68),
                    (0.0, 0.7),
                ],
                1.0,
                1.0,
            );
            x += 0.24;
        }
        front.bx(
            b,
            "trim",
            (-0.05, w + 0.05),
            (bz + 0.68, bz + 0.8),
            (0.3, 0.56),
        );
        for i in 0..=n {
            let s = 0.4 + bay * i as f64;
            front.bx(b, "trim", (s - 0.15, s + 0.15), (bz, bz + 0.8), (0.3, 0.56));
        }
    }
    parapet_all(b, w, d, roof_z);
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, roof_z - 0.2],
        [w / 2.0 - 0.2, d - 0.2, roof_z + 0.03],
        0.0,
    );
    aerial(b, [w * 0.2, d * 0.4], roof_z + 0.03, seed);
    aerial(b, [-w * 0.25, d * 0.55], roof_z + 0.03, seed + 1);
    water_tank(b, [-w * 0.3, d * 0.25], roof_z + 0.03);
}

fn parapet_all(b: &mut Builder, w: f64, d: f64, z: f64) {
    parapet(b, w, d, z, 0.9, "wall", "trim");
}

/// The wall over a round arch from `s0` to `s1` springing at `spring`:
/// the corners of the hole's head between the curve and its top.
pub fn arch_fill(
    b: &mut Builder,
    f: &Face,
    role: &'static str,
    s: (f64, f64),
    spring: f64,
    d: (f64, f64),
) {
    let r = (s.1 - s.0) / 2.0;
    let c = s.0 + r;
    let top = spring + r;
    let n = 10;
    for i in 0..n {
        let a0 = std::f64::consts::PI * i as f64 / n as f64;
        let a1 = std::f64::consts::PI * (i + 1) as f64 / n as f64;
        let p0 = (c + r * a0.cos(), spring + r * a0.sin());
        let p1 = (c + r * a1.cos(), spring + r * a1.sin());
        let pts = [p0, (p0.0, top + 0.001), (p1.0, top + 0.001), p1];
        if (p0.0 - p1.0).abs() < 1e-4 {
            continue;
        }
        f.prism(b, role, &pts, d);
    }
}

/// The stones round an arch, the keystone bigger.
fn voussoirs(b: &mut Builder, f: &Face, s: (f64, f64), spring: f64) {
    let r = (s.1 - s.0) / 2.0;
    let c = s.0 + r;
    let n = 11;
    for i in 0..n {
        let a0 = std::f64::consts::PI * i as f64 / n as f64;
        let a1 = std::f64::consts::PI * (i + 1) as f64 / n as f64;
        let key = i == n / 2;
        let rr = if key { r + 0.42 } else { r + 0.3 };
        let gap = 0.012;
        let (a0, a1) = (a0 + gap / r, a1 - gap / r);
        let pts = [
            (c + r * a0.cos(), spring + r * a0.sin()),
            (c + rr * a0.cos(), spring + rr * a0.sin()),
            (c + rr * a1.cos(), spring + rr * a1.sin()),
            (c + r * a1.cos(), spring + r * a1.sin()),
        ];
        f.prism(b, "stone", &pts, (-0.01, if key { 0.07 } else { 0.035 }));
    }
}

/// The glass fan in an arch over the transom, with its radiating bars.
fn fan(b: &mut Builder, f: &Face, s: (f64, f64), spring: f64) {
    let r = (s.1 - s.0) / 2.0;
    let c = s.0 + r;
    f.bx(
        b,
        "glass",
        s,
        (spring + 0.06, spring + r),
        (-REVEAL - 0.05, -REVEAL - 0.04),
    );
    for k in 1..6 {
        let a = std::f64::consts::PI * k as f64 / 6.0;
        f.bar(
            b,
            "frame",
            (c, spring + 0.06),
            (
                c + (r + 0.02) * a.cos(),
                spring + 0.06 + (r - 0.02) * a.sin(),
            ),
            -REVEAL - 0.035,
            0.014,
            4,
        );
    }
}

/// A glazed bay (mirador) out of the hole `h`: its floor on brackets,
/// white wooden frames with a panel under the glass, and its roof of zinc
/// on the top one.
fn mirador(b: &mut Builder, f: &Face, h: &Hole, zf: f64, upper: f64, top: bool) {
    let out = 0.62;
    let (s0, s1) = (h.s.0 - 0.15, h.s.1 + 0.15);
    let z1 = zf + upper - 0.05;
    f.bx(
        b,
        "stone",
        (s0 - 0.08, s1 + 0.08),
        (zf - 0.14, zf + 0.04),
        (-0.01, out + 0.08),
    );
    for x in [s0 + 0.15, s1 - 0.15] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: out,
        };
        side.prism(
            b,
            "stone",
            &[
                (-0.01, zf - 0.14),
                (out * 0.7, zf - 0.14),
                (-0.01, zf - 0.55),
            ],
            (-0.06, 0.06),
        );
    }
    // The panel under the glass, the frames, the glass.
    let pz = zf + 0.9;
    f.bx(b, "bay", (s0, s1), (zf + 0.04, pz), (out - 0.06, out));
    let n = ((s1 - s0) / 0.55).round().max(2.0) as usize;
    for i in 0..=n {
        let x = s0 + (s1 - s0) * i as f64 / n as f64;
        f.bx(
            b,
            "bay",
            (x - 0.035, x + 0.035),
            (zf + 0.04, z1),
            (out - 0.08, out),
        );
    }
    for zz in [pz, pz + 1.2, z1 - 0.08] {
        f.bx(b, "bay", (s0, s1), (zz, zz + 0.07), (out - 0.08, out));
    }
    f.bx(b, "glass", (s0, s1), (pz, z1), (out - 0.05, out - 0.04));
    for x in [s0, s1] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: out,
        };
        side.bx(b, "bay", (0.0, out), (zf + 0.04, pz), (-0.03, 0.03));
        side.bx(b, "bay", (0.0, out), (z1 - 0.08, z1), (-0.035, 0.035));
        side.bx(
            b,
            "bay",
            (out - 0.08, out),
            (zf + 0.04, z1),
            (-0.035, 0.035),
        );
        side.bx(
            b,
            "glass",
            (0.0, out - 0.06),
            (pz, z1 - 0.08),
            (-0.006, 0.006),
        );
    }
    // Panels moulded on the bottom.
    for i in 0..n {
        let a = s0 + (s1 - s0) * i as f64 / n as f64 + 0.08;
        let e = s0 + (s1 - s0) * (i + 1) as f64 / n as f64 - 0.08;
        f.bx(b, "bay", (a, e), (zf + 0.18, pz - 0.12), (out, out + 0.015));
    }
    f.bx(
        b,
        "bay",
        (s0 - 0.04, s1 + 0.04),
        (z1, z1 + 0.06),
        (-0.01, out + 0.05),
    );
    if top {
        f.leaning(
            b,
            "zinc",
            (s0 - 0.08, s1 + 0.08),
            (z1 + 0.45, 0.0),
            (z1 + 0.06, out + 0.12),
            0.03,
        );
    }
}

/// An «alicantina»: a blind of slats on cords hung from the head of a
/// balcony door, rolled up or let down part of the way.
fn alicantina(b: &mut Builder, f: &Face, h: &Hole, down: f64, out: f64) {
    let (s0, s1) = (h.s.0 - 0.05, h.s.1 + 0.05);
    let z1 = h.z.1 + 0.1;
    // The hooks on the wall.
    for x in [s0 + 0.1, s1 - 0.1] {
        f.bx(
            b,
            "iron",
            (x - 0.01, x + 0.01),
            (z1 - 0.01, z1 + 0.02),
            (-0.01, 0.12),
        );
    }
    if down < 0.05 {
        // Rolled: a fat roll held by its cords.
        let c0 = f.at(s0, z1 - 0.12, 0.12);
        let c1 = f.at(s1, z1 - 0.12, 0.12);
        b.cone("alicantina", c0, c1, 0.09, 0.09, 10);
        for x in [s0 + 0.1, s1 - 0.1] {
            f.bx(
                b,
                "line",
                (x - 0.006, x + 0.006),
                (z1 - 0.22, z1 + 0.01),
                (0.1, 0.12),
            );
        }
    } else {
        // Let down: the slats on their cords, leaning out over the
        // balcony's rail.
        let low = z1 - (h.z.1 - h.z.0) * down;
        let lean = (out * 0.6).min(0.35);
        f.leaning(
            b,
            "alicantina",
            (s0, s1),
            (z1, 0.12),
            (low, 0.12 + lean),
            0.015,
        );
        let c0 = f.at(s0, low, 0.13 + lean);
        let c1 = f.at(s1, low, 0.13 + lean);
        b.cone("alicantina", c0, c1, 0.06, 0.06, 10);
    }
}
