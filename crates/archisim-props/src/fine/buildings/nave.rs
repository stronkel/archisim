//! The warehouse of an industrial estate (nave de polígono): walls of
//! concrete block to three metres and ribbed panels above, the gable to
//! the street with the company's sign, the great sectional door with its
//! wicket, a loading dock with its seal, the offices in a corner with
//! strip windows and a canopy over their door, high strips of
//! polycarbonate along the sides, a roof of ribbed sheet with skylight
//! bands, gutters and downpipes, and the yard behind a wire fence.

use super::more::*;
use super::*;
use crate::Params;

const WALL: f64 = 0.25;

pub fn warehouse(b: &mut Builder, p: &Params) {
    let w = p.get("width", 24.0).clamp(12.0, 50.0);
    let d = p.get("depth", 40.0).clamp(15.0, 90.0);
    let h = p.get("height", 8.0).clamp(5.0, 14.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let offices = p.flag("offices", true);
    let pitch = 0.1;
    let block = 3.0;
    let rise = w / 2.0 * pitch;
    let [front, right, back, left] = Face::sides(w, d);

    // The front: the sectional door, the dock door, the offices' door and
    // windows.
    let door = Hole {
        s: (w * 0.5 - 2.6, w * 0.5 + 2.6),
        z: (0.0, 5.2),
    };
    let dock = Hole {
        s: (w * 0.18 - 1.5, w * 0.18 + 1.5),
        z: (1.2, 4.2),
    };
    let mut fh = vec![door, dock];
    let ow = (w * 0.28).max(5.0);
    let os = w - ow;
    let office_door = Hole {
        s: (os + 0.6, os + 1.8),
        z: (0.1, 2.5),
    };
    let strips = [
        Hole {
            s: (os + 2.2, w - 0.5),
            z: (1.0, 2.3),
        },
        Hole {
            s: (os + 0.6, w - 0.5),
            z: (4.1, 5.4),
        },
    ];
    if offices {
        fh.push(office_door);
        fh.extend(strips);
    }
    let fh_high: Vec<Hole> = fh.iter().copied().filter(|x| x.z.1 > block).collect();
    wall(
        b,
        "block",
        &front,
        (0.0, block),
        WALL,
        &fh.iter()
            .copied()
            .filter(|x| x.z.0 < block)
            .collect::<Vec<_>>(),
    );
    wall(b, "panel_wall", &front, (block, h), WALL, &fh_high);
    // The gable on the front and the back.
    for f in [&front, &back] {
        f.prism(
            b,
            "panel_wall",
            &[(0.0, h), (w, h), (w / 2.0, h + rise)],
            (-WALL, 0.0),
        );
    }
    // The sides: high strips of polycarbonate.
    let mut side_holes = Vec::new();
    let n = ((d - 2.0) / 6.0).floor().max(1.0) as usize;
    for i in 0..n {
        let a = 1.0 + (d - 2.0) * i as f64 / n as f64 + 0.3;
        let e = 1.0 + (d - 2.0) * (i + 1) as f64 / n as f64 - 0.3;
        side_holes.push(Hole {
            s: (a, e),
            z: (h - 1.6, h - 0.5),
        });
    }
    for f in [&right, &left] {
        wall(b, "block", f, (0.0, block), WALL, &[]);
        wall(b, "panel_wall", f, (block, h), WALL, &side_holes);
        for hh in &side_holes {
            f.bx(b, "skylight", hh.s, hh.z, (-WALL * 0.6, -WALL * 0.4));
            f.bx(b, "frame", hh.s, (hh.z.0 - 0.05, hh.z.0), (-WALL, 0.03));
        }
    }
    let back_door = Hole {
        s: (w * 0.5 - 2.0, w * 0.5 + 2.0),
        z: (0.0, 4.5),
    };
    wall(b, "block", &back, (0.0, block), WALL, &[back_door]);
    wall(b, "panel_wall", &back, (block, h), WALL, &[back_door]);
    core(b, w, d, WALL, (0.0, h));
    // The panels' ribs, every metre, on all four faces; the block's
    // courses.
    for f in [&front, &right, &back, &left] {
        let mut z = block + 0.5;
        while z < h - 0.1 {
            f.bx(
                b,
                "panel_wall",
                (0.0, f.len),
                (z - 0.02, z + 0.02),
                (-0.01, 0.025),
            );
            z += 1.0;
        }
        f.bx(
            b,
            "frame",
            (0.0, f.len),
            (block - 0.04, block + 0.04),
            (-0.01, 0.05),
        );
        let mut z = 0.2;
        while z < block - 0.1 {
            f.bx(
                b,
                "block",
                (0.0, f.len),
                (z - 0.006, z + 0.006),
                (-0.01, 0.006),
            );
            z += 0.2;
        }
        // The corners' flashing.
        f.bx(b, "frame", (0.0, 0.12), (block, h), (-0.01, 0.04));
        f.bx(b, "frame", (f.len - 0.12, f.len), (block, h), (-0.01, 0.04));
    }
    // The sectional door with its wicket.
    let fd = (-WALL - 0.02, -WALL + 0.06);
    front.bx(
        b,
        "frame",
        (door.s.0 - 0.15, door.s.0),
        door.z,
        (-0.01, 0.06),
    );
    front.bx(
        b,
        "frame",
        (door.s.1, door.s.1 + 0.15),
        door.z,
        (-0.01, 0.06),
    );
    front.bx(
        b,
        "frame",
        (door.s.0 - 0.15, door.s.1 + 0.15),
        (door.z.1, door.z.1 + 0.15),
        (-0.01, 0.06),
    );
    front.bx(b, "garage", door.s, door.z, fd);
    let mut z = 0.6;
    while z < door.z.1 - 0.1 {
        front.bx(
            b,
            "garage",
            door.s,
            (z - 0.015, z + 0.015),
            (fd.1, fd.1 + 0.012),
        );
        z += 0.6;
    }
    for x in [door.s.0 + 0.6, door.s.1 - 0.6] {
        front.bx(
            b,
            "glass",
            (x - 0.35, x + 0.35),
            (3.1, 3.5),
            (fd.1, fd.1 + 0.005),
        );
    }
    let wk = (door.s.0 + 1.2, door.s.0 + 2.2);
    front.bx(
        b,
        "frame",
        (wk.0 - 0.03, wk.0),
        (0.0, 2.2),
        (fd.1, fd.1 + 0.02),
    );
    front.bx(
        b,
        "frame",
        (wk.1, wk.1 + 0.03),
        (0.0, 2.2),
        (fd.1, fd.1 + 0.02),
    );
    front.bx(
        b,
        "frame",
        (wk.0 - 0.03, wk.1 + 0.03),
        (2.2, 2.23),
        (fd.1, fd.1 + 0.02),
    );
    b.ellipsoid(
        "metal",
        front.at(wk.1 - 0.12, 1.05, fd.1 + 0.03),
        [0.03, 0.03, 0.03],
    );
    // The loading dock: the platform, its seal and its door.
    front.bx(
        b,
        "dock",
        (dock.s.0 - 0.8, dock.s.1 + 0.8),
        (0.0, dock.z.0),
        (-0.01, 2.2),
    );
    // The seal: a frame of rubber round the door, standing out.
    front.bx(
        b,
        "rubber",
        (dock.s.0 - 0.6, dock.s.0),
        (dock.z.0, dock.z.1 + 0.6),
        (-0.01, 0.6),
    );
    front.bx(
        b,
        "rubber",
        (dock.s.1, dock.s.1 + 0.6),
        (dock.z.0, dock.z.1 + 0.6),
        (-0.01, 0.6),
    );
    front.bx(
        b,
        "rubber",
        (dock.s.0, dock.s.1),
        (dock.z.1, dock.z.1 + 0.6),
        (-0.01, 0.6),
    );
    front.bx(b, "garage", dock.s, dock.z, fd);
    for x in [dock.s.0 - 0.7, dock.s.1 + 0.7] {
        front.bx(
            b,
            "rubber",
            (x - 0.15, x + 0.15),
            (dock.z.0 - 0.45, dock.z.0 - 0.05),
            (2.2, 2.32),
        );
    }
    front.bx(
        b,
        "stripe",
        (dock.s.0 - 0.8, dock.s.1 + 0.8),
        (dock.z.0 - 0.15, dock.z.0),
        (2.2, 2.21),
    );
    // The offices: their door, canopy and strip windows.
    if offices {
        let win = Win {
            leaves: 2,
            reveal: 0.05,
            ..Default::default()
        };
        window(b, &front, office_door, &win, "frame");
        front.bx(
            b,
            "frame",
            (office_door.s.0 - 0.6, office_door.s.1 + 0.6),
            (2.8, 2.95),
            (-0.01, 1.6),
        );
        for x in [office_door.s.0 - 0.5, office_door.s.1 + 0.5] {
            front.bar(b, "frame", (x, 0.0), (x, 2.8), 1.45, 0.05, 8);
        }
        front.bx(
            b,
            "step",
            (office_door.s.0 - 0.3, office_door.s.1 + 0.3),
            (0.0, 0.1),
            (-0.05, 0.6),
        );
        for s in &strips {
            let n = ((s.s.1 - s.s.0) / 1.2).round().max(1.0) as usize;
            let len = (s.s.1 - s.s.0) / n as f64;
            for k in 0..n {
                let hh = Hole {
                    s: (s.s.0 + len * k as f64, s.s.0 + len * (k + 1) as f64),
                    z: s.z,
                };
                let win = Win {
                    leaves: 1,
                    reveal: 0.05,
                    blind: if unit(seed, k as u64) > 0.7 { 0.4 } else { 0.0 },
                    ..Default::default()
                };
                window(b, &front, hh, &win, "frame");
            }
            front.bx(
                b,
                "frame",
                (s.s.0 - 0.05, s.s.1 + 0.05),
                (s.z.0 - 0.06, s.z.0),
                (-0.05, 0.07),
            );
        }
        ac_unit(b, &right, 2.5, 4.4);
        ac_unit(b, &right, 3.6, 4.4);
    }
    // The sign along the gable.
    front.bx(
        b,
        "sign",
        (w * 0.2, w * 0.8),
        (h - 1.2, h + rise * 0.35),
        (-0.01, 0.08),
    );
    front.bx(
        b,
        "sign_text",
        (w * 0.25, w * 0.75),
        (h - 0.95, h - 0.45 + rise * 0.3),
        (0.08, 0.1),
    );

    // The roof: two plates of ribbed sheet, the skylight bands, the ridge,
    // the gutters and downpipes.
    let ov = 0.25;
    let ze = h - ov * pitch;
    for sgn in [-1.0, 1.0] {
        let xe = sgn * (w / 2.0 + ov);
        plate(
            b,
            "roof",
            &[
                [xe, -ov, ze],
                [0.0, -ov, h + rise],
                [0.0, d + ov, h + rise],
                [xe, d + ov, ze],
            ],
            0.06,
        );
        let n = ((w / 2.0 + ov) / 0.25).floor() as usize;
        for k in 0..n {
            let x = sgn * (0.12 + 0.25 * k as f64);
            let z = h + rise - x.abs() * pitch + 0.06;
            b.cone("roof", [x, -ov, z], [x, d + ov, z], 0.025, 0.025, 4);
        }
        let bands = ((d - 4.0) / 8.0).floor().max(1.0) as usize;
        for k in 0..bands {
            let y = 4.0 + (d - 8.0) * (k as f64 + 0.5) / bands as f64;
            let (x0, x1) = (sgn * 1.0, sgn * (w / 2.0 - 1.0));
            let (x0, x1) = (x0.min(x1), x0.max(x1));
            let z = |x: f64| h + rise - x.abs() * pitch + 0.09;
            plate(
                b,
                "skylight",
                &[
                    [x0, y - 0.8, z(x0)],
                    [x1, y - 0.8, z(x1)],
                    [x1, y + 0.8, z(x1)],
                    [x0, y + 0.8, z(x0)],
                ],
                0.04,
            );
        }
        // The gutter and two downpipes on each side.
        let gx = sgn * (w / 2.0 + ov);
        b.rbox(
            "metal",
            [gx - 0.12, -ov, ze - 0.2],
            [gx + 0.12, d + ov, ze + 0.02],
            0.0,
        );
        let side = if sgn > 0.0 { &right } else { &left };
        for y in [1.0, side.len - 1.0] {
            downpipe(b, side, y, ze - 0.1);
        }
    }
    b.cone(
        "metal",
        [0.0, -ov, h + rise + 0.08],
        [0.0, d + ov, h + rise + 0.08],
        0.14,
        0.14,
        4,
    );

    // The yard in front: concrete, the fence of wire on posts, the gate.
    let yd = 10.0;
    b.rbox(
        "yard",
        [-w / 2.0 - 2.0, d, 0.0],
        [w / 2.0 + 2.0, d + yd, 0.02],
        0.0,
    );
    let ff = Face {
        o: [-w / 2.0 - 2.0, d + yd],
        u: [1.0, 0.0],
        len: w + 4.0,
    };
    let gate = (w / 2.0 + 2.0 - 4.0, w / 2.0 + 2.0 + 4.0);
    for (a, e) in [(0.0, gate.0), (gate.1, ff.len)] {
        let n = ((e - a) / 2.5).ceil().max(1.0) as usize;
        for k in 0..=n {
            let x = a + (e - a) * k as f64 / n as f64;
            ff.bar(b, "fence", (x, 0.0), (x, 2.0), 0.0, 0.03, 8);
        }
        grille_plane(b, &ff, "fence", (a, e), (0.05, 1.98), 0.0, 0.15, 3);
        ff.bar(b, "fence", (a, 1.98), (e, 1.98), 0.0, 0.025, 6);
    }
    for x in [gate.0, gate.1] {
        ff.bx(b, "fence", (x - 0.08, x + 0.08), (0.0, 2.3), (-0.08, 0.08));
    }
    // Two parking lines and a car-sized pallet stack.
    for k in 0..4 {
        let x = -w / 2.0 + 1.0 + 2.6 * k as f64;
        b.rbox(
            "stripe",
            [x, d + yd - 5.5, 0.02],
            [x + 0.1, d + yd - 0.8, 0.025],
            0.0,
        );
    }
    for k in 0..3 {
        let x = w / 2.0 - 3.0 + 1.25 * k as f64 - 1.0;
        for j in 0..(1 + k % 2 + (seed as usize % 2)) {
            b.rbox(
                "pallet",
                [x, d + 0.8, 0.02 + 0.15 * j as f64],
                [x + 1.2, d + 1.6, 0.17 + 0.15 * j as f64],
                0.0,
            );
        }
    }
}
