//! The bullring (plaza de toros): a round building of many sides, white
//! with bands and pilasters of albero, doors in round arches below and
//! arched windows above, the main gate with its doorway and flags; inside
//! the ring of sand with its two white circles, the red barrier with its
//! shields, the passage behind it, the stepped stands and the covered
//! gallery on columns under a roof of tiles falling to the ring.

use super::ensanche::arch_fill;
use super::more::*;
use super::*;
use crate::Params;

/// A piece of a ring between `r1` and `r2`, from angle `a0` to `a1`,
/// `z0` to `z1`, round `c`.
#[allow(clippy::too_many_arguments)]
fn ring(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    (r1, r2): (f64, f64),
    (a0, a1): (f64, f64),
    (z0, z1): (f64, f64),
) {
    let p = |r: f64, a: f64| [c[0] + r * a.cos(), c[1] + r * a.sin(), z0];
    plate(
        b,
        role,
        &[p(r1, a0), p(r2, a0), p(r2, a1), p(r1, a1)],
        z1 - z0,
    );
}

pub fn bullring(b: &mut Builder, p: &Params) {
    let big_r = p.get("radius", 28.0).clamp(18.0, 45.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let _ = seed;
    let n = ((big_r * std::f64::consts::TAU / 3.6).round() as usize).max(24);
    let n = n + n % 2;
    let c = [0.0, big_r];
    let r0 = big_r * 0.57; // the ring of sand
    let callejon = 1.6;
    let rows = ((big_r - 4.0 - r0 - callejon) / 0.8).floor().max(3.0) as usize;
    let r_stands = r0 + callejon + 0.2;
    let r_gallery = r_stands + 0.8 * rows as f64;
    let rise = 0.45;
    let z_stands = 1.9;
    let z_gallery = z_stands + rise * rows as f64 + 0.2;
    let h = z_gallery + 4.2;
    let da = std::f64::consts::TAU / n as f64;
    // The front (+y) is angle π/2: the main gate's side.
    let ang = |i: usize| std::f64::consts::FRAC_PI_2 - da / 2.0 + da * i as f64;

    // The sand, its circles, the barrier and its shields.
    let disk: Vec<P3> = (0..n)
        .map(|i| [c[0] + r0 * ang(i).cos(), c[1] + r0 * ang(i).sin(), 0.0])
        .collect();
    plate(b, "sand", &disk, 0.04);
    for i in 0..n {
        let (a0, a1) = (ang(i), ang(i + 1));
        for rr in [r0 - 9.0, r0 - 7.0] {
            if rr > 2.0 {
                ring(b, "line", c, (rr - 0.06, rr + 0.06), (a0, a1), (0.0, 0.05));
            }
        }
        ring(b, "barrier", c, (r0, r0 + 0.08), (a0, a1), (0.0, 1.6));
        ring(b, "trim", c, (r0 - 0.03, r0 + 0.11), (a0, a1), (1.6, 1.68));
        ring(
            b,
            "sand",
            c,
            (r0 + 0.08, r_stands - 0.2),
            (a0, a1),
            (0.0, 0.04),
        );
        // The wall of the stands over the passage.
        ring(
            b,
            "wall",
            c,
            (r_stands - 0.2, r_stands),
            (a0, a1),
            (0.0, z_stands + 0.4),
        );
        // The stands: each row down to the ground.
        for k in 0..rows {
            let r = r_stands + 0.8 * k as f64;
            let z = z_stands + rise * (k + 1) as f64;
            let role = if k % 2 == 0 { "seat" } else { "seat_b" };
            ring(b, role, c, (r, r + 0.8), (a0, a1), (0.0, z));
        }
        // The gallery's floor, its columns and its balustrade.
        ring(
            b,
            "seat",
            c,
            (r_gallery, big_r - 0.5),
            (a0, a1),
            (z_gallery - 0.2, z_gallery),
        );
        let col = [
            c[0] + (r_gallery + 0.3) * a0.cos(),
            c[1] + (r_gallery + 0.3) * a0.sin(),
        ];
        b.cyl(
            "trim",
            [col[0], col[1], z_gallery],
            [col[0], col[1], h - 1.05],
            0.14,
        );
        b.rbox(
            "trim",
            [col[0] - 0.2, col[1] - 0.2, h - 1.15],
            [col[0] + 0.2, col[1] + 0.2, h - 0.95],
            0.0,
        );
        ring(
            b,
            "trim",
            c,
            (r_gallery + 0.1, r_gallery + 0.5),
            (a0, a1),
            (h - 0.95, h - 0.65),
        );
        ring(
            b,
            "trim",
            c,
            (r_gallery + 0.15, r_gallery + 0.45),
            (a0, a1),
            (z_gallery + 0.85, z_gallery + 0.95),
        );
        for t in 1..8 {
            let a = a0 + (a1 - a0) * t as f64 / 8.0;
            let q = [
                c[0] + (r_gallery + 0.3) * a.cos(),
                c[1] + (r_gallery + 0.3) * a.sin(),
            ];
            b.cyl(
                "trim",
                [q[0], q[1], z_gallery],
                [q[0], q[1], z_gallery + 0.85],
                0.04,
            );
        }
        // The roof of the gallery: tiles falling to the ring.
        let pr = |r: f64, a: f64, z: f64| [c[0] + r * a.cos(), c[1] + r * a.sin(), z];
        let zi = h - 0.65;
        let zo = h + 0.6;
        plate(
            b,
            "roof",
            &[
                pr(r_gallery - 0.3, a0, zi - 0.1),
                pr(big_r + 0.4, a0, zo),
                pr(big_r + 0.4, a1, zo),
                pr(r_gallery - 0.3, a1, zi - 0.1),
            ],
            0.07,
        );
        for t in 0..4 {
            let a = a0 + (a1 - a0) * (t as f64 + 0.5) / 4.0;
            let ri = r_gallery - 0.35;
            let ro = big_r + 0.45;
            let lift = 0.11;
            let zz = |r: f64| {
                zi - 0.1
                    + (zo - zi + 0.1) * (r - (r_gallery - 0.3)) / (big_r + 0.4 - (r_gallery - 0.3))
                    + lift
            };
            b.cone(
                "tiles",
                pr(ri, a, zz(ri)),
                pr(ro, a, zz(ro)),
                0.07,
                0.065,
                6,
            );
        }
    }
    b.fine.insert("tiles");

    // The outer wall: one face per side, doors and windows in arches,
    // pilasters at the corners, bands and cornice.
    let p = |a: f64| [c[0] + big_r * a.cos(), c[1] + big_r * a.sin()];
    for i in 0..n {
        let (a0, a1) = (ang(i), ang(i + 1));
        let (q0, q1) = (p(a0), p(a1));
        let len = ((q1[0] - q0[0]).powi(2) + (q1[1] - q0[1]).powi(2)).sqrt();
        let f = Face {
            o: q1,
            u: [(q0[0] - q1[0]) / len, (q0[1] - q1[1]) / len],
            len,
        };
        let main = i == 0;
        let door = main || i % 4 == 2;
        let mut hs = Vec::new();
        let dw = if main { 1.3 } else { 0.85 };
        let spring = if main { 3.4 } else { 2.6 };
        if door {
            hs.push(Hole {
                s: (len / 2.0 - dw, len / 2.0 + dw),
                z: (0.1, spring + dw),
            });
        }
        let up = Hole {
            s: (len / 2.0 - 0.6, len / 2.0 + 0.6),
            z: (z_gallery + 0.7, z_gallery + 2.2 + 0.6),
        };
        hs.push(up);
        wall(b, "wall", &f, (0.0, h), 0.6, &hs);
        for hh in &hs {
            let r = (hh.s.1 - hh.s.0) / 2.0;
            let sp = hh.z.1 - r;
            arch_fill(b, &f, "wall", hh.s, sp, (-0.6, 0.0));
            surround(b, &f, *hh, "trim", 0.16, 0.04);
            if hh.z.0 < 1.0 {
                panel_door(
                    b,
                    &f,
                    Hole {
                        s: hh.s,
                        z: (hh.z.0, sp),
                    },
                    2,
                    0.0,
                    "wood",
                    "iron",
                    0.25,
                );
                f.bx(b, "glass", hh.s, (sp, hh.z.1), (-0.31, -0.3));
                grille_plane(b, &f, "iron", hh.s, (sp, hh.z.1), -0.28, 0.1, 2);
            } else {
                f.bx(b, "glass", hh.s, hh.z, (-0.4, -0.39));
                grille_plane(b, &f, "iron", hh.s, (hh.z.0, sp), -0.2, 0.12, 2);
                f.bx(
                    b,
                    "trim",
                    (hh.s.0 - 0.1, hh.s.1 + 0.1),
                    (hh.z.0 - 0.1, hh.z.0),
                    (-0.3, 0.08),
                );
            }
        }
        f.bx(b, "plinth", (0.0, len), (0.0, 1.0), (-0.01, 0.05));
        f.bx(
            b,
            "trim",
            (0.0, len),
            (z_gallery - 0.1, z_gallery + 0.15),
            (-0.01, 0.1),
        );
        f.bx(b, "trim", (-0.02, len + 0.02), (h - 0.4, h), (-0.01, 0.25));
        f.bx(b, "trim", (0.0, 0.35), (1.0, h - 0.4), (-0.01, 0.08));
        // The main gate: a stone doorway, its balcony and two flags.
        if main {
            let (s0, s1) = (len / 2.0 - dw - 0.5, len / 2.0 + dw + 0.5);
            f.bx(
                b,
                "stone",
                (s0, s0 + 0.4),
                (0.0, spring + dw + 0.6),
                (-0.01, 0.22),
            );
            f.bx(
                b,
                "stone",
                (s1 - 0.4, s1),
                (0.0, spring + dw + 0.6),
                (-0.01, 0.22),
            );
            f.bx(
                b,
                "stone",
                (s0 - 0.1, s1 + 0.1),
                (spring + dw + 0.6, spring + dw + 0.95),
                (-0.01, 0.3),
            );
            f.bx(
                b,
                "tile_panel",
                (len / 2.0 - 1.0, len / 2.0 + 1.0),
                (spring + dw + 0.15, spring + dw + 0.55),
                (-0.01, 0.05),
            );
            let bc = f.at(len / 2.0, 0.0, 0.0);
            for dx in [-1.0, 1.0] {
                let q = [
                    bc[0] + f.u[0] * dx * 2.2 + f.out()[0] * 0.4,
                    bc[1] + f.u[1] * dx * 2.2 + f.out()[1] * 0.4,
                ];
                flag_pole(b, q, h + 3.0, if dx < 0.0 { "flag" } else { "flag_b" });
            }
        }
    }
    // The dark inside under the stands, seen through the doors.
    for i in 0..n {
        ring(
            b,
            "interior",
            c,
            (r_gallery, big_r - 0.5),
            (ang(i), ang(i + 1)),
            (0.0, z_gallery - 0.2),
        );
    }
    // The square round it, paved.
    let pad: Vec<P3> = (0..n)
        .map(|i| {
            let a = ang(i);
            [
                c[0] + (big_r + 4.0) * a.cos(),
                c[1] + (big_r + 4.0) * a.sin(),
                0.0,
            ]
        })
        .collect();
    plate(b, "paving", &pad, 0.02);
}
