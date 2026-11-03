//! The Ibizan house (casa payesa): white cubes added one to another, flat
//! roofs with their rounded edges, small deep windows, the porch (porxo)
//! of juniper beams on white pillars, the outside stair up to the roof of
//! the low cube, the chimney with its little hat, dry stone walls and an
//! olive tree.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.6;
const REVEAL: f64 = 0.3;

/// A white cube from `x0` to `x1` and `y0` to `y1`, `h` high, with its
/// holes on its faces (front, right, back, left), its roof and its dark
/// inside. Returns its faces.
#[allow(clippy::too_many_arguments)]
fn cube(
    b: &mut Builder,
    (x0, x1): (f64, f64),
    (y0, y1): (f64, f64),
    h: f64,
    holes: [&[Hole]; 4],
    hidden: [bool; 4],
) -> [Face; 4] {
    let (w, d) = (x1 - x0, y1 - y0);
    let faces = Face::sides(w, d).map(|f| Face {
        o: [f.o[0] + (x0 + x1) / 2.0, f.o[1] + y0],
        ..f
    });
    for (k, f) in faces.iter().enumerate() {
        if hidden[k] {
            continue;
        }
        wall(b, "wall", f, (0.0, h), WALL, holes[k]);
        // The roof's rounded edge.
        let s = f.at(0.0, 0.0, -0.15);
        let e = f.at(f.len, 0.0, -0.15);
        b.cone(
            "wall",
            [s[0], s[1], h + 0.12],
            [e[0], e[1], h + 0.12],
            0.17,
            0.17,
            10,
        );
        // A wash of grey at the foot.
        f.bx(b, "plinth", (0.0, f.len), (0.0, 0.35), (-0.01, 0.025));
    }
    b.rbox(
        "interior",
        [x0 + WALL, y0 + WALL, 0.0],
        [x1 - WALL, y1 - WALL, h - 0.1],
        0.0,
    );
    b.rbox(
        "roof_floor",
        [x0 + 0.1, y0 + 0.1, h - 0.1],
        [x1 - 0.1, y1 - 0.1, h + 0.05],
        0.0,
    );
    faces
}

pub fn ibizan_house(b: &mut Builder, p: &Params) {
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let low = 3.3;
    let high = 6.2;
    // The main cube, low; the tall cube at its east; a small cube at its
    // west (the old kitchen) with the chimney.
    let win = |s: f64, z: f64| Hole {
        s: (s - 0.3, s + 0.3),
        z: (z, z + 0.7),
    };
    let door = Hole {
        s: (3.4, 4.6),
        z: (0.0, 2.3),
    };
    let main_f = [win(1.2, 1.2), door, win(6.4, 1.2)];
    let main = cube(
        b,
        (-4.0, 4.0),
        (0.0, 7.0),
        low,
        [&main_f, &[], &[win(4.0, 1.3)], &[]],
        [false; 4],
    );
    let tall_f = [win(2.5, 1.3), win(2.5, 4.2)];
    let tall_side = [win(3.5, 4.2)];
    let tall = cube(
        b,
        (4.0, 9.0),
        (1.5, 6.5),
        high,
        [&tall_f, &tall_side, &[win(2.5, 4.4)], &[win(2.5, 4.3)]],
        [false, false, false, false],
    );
    let small = cube(
        b,
        (-7.5, -4.0),
        (1.0, 5.5),
        2.8,
        [&[win(1.7, 1.1)], &[], &[], &[win(2.2, 1.0)]],
        [false, true, false, false],
    );
    for (f, hs) in [
        (&main[0], &main_f[..]),
        (&tall[0], &tall_f[..]),
        (&tall[1], &tall_side[..]),
        (&small[0], &[win(1.7, 1.1)][..]),
    ] {
        for h in hs {
            if h.s.1 - h.s.0 > 1.0 {
                panel_door(b, f, *h, 2, 0.0, "wood", "iron", REVEAL);
            } else {
                let w = Win {
                    leaves: 1,
                    reveal: REVEAL,
                    ..Default::default()
                };
                window(b, f, *h, &w, "wood");
                f.bx(
                    b,
                    "wood",
                    (h.s.0, h.s.1),
                    (h.z.0 - 0.06, h.z.0),
                    (-REVEAL, 0.0),
                );
            }
        }
    }
    // The small windows of the backs and sides, plain.
    for (f, hs) in [
        (&main[2], vec![win(4.0, 1.3)]),
        (&tall[2], vec![win(2.5, 4.4)]),
        (&tall[3], vec![win(2.5, 4.3)]),
        (&small[3], vec![win(2.2, 1.0)]),
    ] {
        for h in hs {
            let w = Win {
                leaves: 1,
                reveal: REVEAL,
                ..Default::default()
            };
            window(b, f, h, &w, "wood");
        }
    }
    // The porch before the main cube: two white pillars, beams of
    // juniper, a covering of canes and a bench along the wall.
    let pd = 3.0;
    for x in [-3.3, 1.7] {
        b.rbox(
            "wall",
            [x - 0.3, 7.0 + pd - 0.6, 0.0],
            [x + 0.3, 7.0 + pd, 2.6],
            0.06,
        );
    }
    b.cyl(
        "juniper",
        [-3.8, 7.0 + pd - 0.3, 2.68],
        [2.2, 7.0 + pd - 0.3, 2.68],
        0.11,
    );
    let mut x = -3.5;
    while x < 2.0 {
        let wob = 0.04 * (unit(seed, (x * 10.0) as u64) - 0.5);
        b.cyl(
            "juniper",
            [x, 6.8, 2.75 + wob],
            [x + wob, 7.0 + pd + 0.1, 2.68 + wob],
            0.06,
        );
        x += 0.55;
    }
    b.rbox("canes", [-3.7, 6.9, 2.8], [2.1, 7.0 + pd + 0.05, 2.85], 0.0);
    b.rbox("wall", [-3.0, 7.0, 0.0], [0.5, 7.45, 0.45], 0.05);
    b.rbox("paving", [-4.0, 7.0, 0.0], [2.5, 7.0 + pd, 0.06], 0.0);
    // The outside stair up to the main cube's roof, against the tall
    // cube's west wall (outside, at its front).
    let steps = 11;
    let rise = (low + 0.05) / steps as f64;
    for k in 0..steps {
        let z = rise * (k + 1) as f64;
        let yy = 6.5 + 0.28 * (steps - k) as f64;
        b.rbox("wall", [4.0, yy - 0.28, 0.0], [5.0, yy, z], 0.02);
    }
    // The chimney with its hat, on the small cube.
    let cc = [-5.8, 3.2];
    b.rbox(
        "wall",
        [cc[0] - 0.45, cc[1] - 0.45, 2.8],
        [cc[0] + 0.45, cc[1] + 0.45, 4.6],
        0.12,
    );
    b.rbox(
        "wall",
        [cc[0] - 0.55, cc[1] - 0.55, 4.6],
        [cc[0] + 0.55, cc[1] + 0.55, 4.7],
        0.03,
    );
    for (dx, dy) in [(-0.35, -0.35), (0.35, -0.35), (0.35, 0.35), (-0.35, 0.35)] {
        b.cyl(
            "wall",
            [cc[0] + dx, cc[1] + dy, 4.7],
            [cc[0] + dx, cc[1] + dy, 5.0],
            0.08,
        );
    }
    b.cone(
        "wall",
        [cc[0], cc[1], 5.0],
        [cc[0], cc[1], 5.6],
        0.6,
        0.05,
        12,
    );
    // Dry stone walls round the yard and an olive tree.
    let yard = [(-10.0, 12.0), (12.0, 12.0)];
    let f = Face {
        o: [yard[0].0, yard[0].1],
        u: [1.0, 0.0],
        len: yard[1].0 - yard[0].0,
    };
    f.bx(b, "dry_stone", (0.0, 8.0), (0.0, 1.0), (-0.5, 0.0));
    f.bx(b, "dry_stone", (11.0, f.len), (0.0, 1.0), (-0.5, 0.0));
    let mut x = 0.2;
    while x < f.len {
        if !(8.0..11.0).contains(&x) {
            b.ellipsoid("dry_stone", f.at(x, 1.0, -0.25), [0.22, 0.2, 0.12]);
        }
        x += 0.45;
    }
    b.rbox("yard", [-10.0, -2.0, 0.0], [12.0, 12.0, 0.02], 0.0);
    fruit_tree(b, [-6.0, 9.0], 4.0, seed, false);
    fruit_tree(b, [8.5, 9.5], 3.2, seed + 1, true);
    for (k, x) in [-1.0, 0.0, 6.0].iter().enumerate() {
        standing_pot(b, [*x, 7.1, 0.06], 0.9, seed + 10 + k as u64);
    }
}
