//! Office buildings (tanda 5): the tower of curtain wall, the office block
//! of the 1970s with its bands of precast spandrels, and the low building
//! of a business park among its car park and solar canopies.

use super::more::*;
use super::*;
use crate::Params;

/// The office tower: a glass box on a recessed glazed lobby, the
/// entrance's canopy and the company's name, a crown of louvres hiding the
/// machines, the paved forecourt with benches and trees.
pub fn office_tower(b: &mut Builder, p: &Params) {
    let w = p.get("width", 24.0).clamp(14.0, 50.0);
    let d = p.get("depth", 20.0).clamp(12.0, 40.0);
    let floors = p.get("floors", 12.0).round().clamp(4.0, 40.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let lobby = 5.0;
    let fl = 3.6;
    let top = lobby + fl * (floors - 1) as f64;
    let faces = Face::sides(w, d);
    // The lobby: set back behind the columns, all glass.
    let set = 1.5;
    let inner = Face::sides(w - 2.0 * set, d - 2.0 * set).map(|f| Face {
        o: [f.o[0], f.o[1] + set],
        ..f
    });
    for (k, f) in inner.iter().enumerate() {
        curtain_wall(b, f, (0.0, f.len), (0.0, lobby), 1.5, lobby, 0.0);
        if k == 0 {
            let c = f.len / 2.0;
            f.bx(
                b,
                "door_frame",
                (c - 1.6, c + 1.6),
                (0.0, 2.8),
                (-0.02, 0.06),
            );
            f.bx(
                b,
                "curtain_glass",
                (c - 1.5, c + 1.5),
                (0.0, 2.7),
                (0.06, 0.07),
            );
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + set + 0.2, set + 0.2, 0.0],
        [w / 2.0 - set - 0.2, d - set - 0.2, lobby],
        0.0,
    );
    for f in &faces {
        let n = (f.len / 6.0).round().max(1.0) as usize;
        for i in 0..=n {
            let x = (f.len * i as f64 / n as f64).clamp(0.4, f.len - 0.4);
            let c = f.at(x, 0.0, -0.6);
            b.rbox(
                "structure",
                [c[0] - 0.35, c[1] - 0.35, 0.0],
                [c[0] + 0.35, c[1] + 0.35, lobby],
                0.0,
            );
        }
    }
    b.rbox(
        "structure",
        [-w / 2.0, 0.0, lobby - 0.6],
        [w / 2.0, d, lobby],
        0.0,
    );
    // The tower: curtain walls on the four faces.
    for f in &faces {
        curtain_wall(b, f, (0.0, f.len), (lobby, top), 1.5, fl, 0.0);
    }
    b.rbox(
        "interior",
        [-w / 2.0 + 0.2, 0.2, lobby],
        [w / 2.0 - 0.2, d - 0.2, top],
        0.0,
    );
    // Blinds down behind some panes.
    for (k, f) in faces.iter().enumerate() {
        let n = (f.len / 1.5).round() as usize;
        for fl_i in 1..floors {
            let z0 = lobby + fl * (fl_i - 1) as f64 + 0.9;
            for i in 0..n {
                let r = unit(seed, (k * 10_000 + fl_i * 100 + i) as u64);
                if r > 0.75 {
                    let x = f.len * i as f64 / n as f64;
                    let down = 0.3 + 1.6 * (r - 0.75) * 4.0;
                    f.bx(
                        b,
                        "blind",
                        (x + 0.06, x + 1.44),
                        (z0 + 2.6 - down, z0 + 2.6),
                        (-0.13, -0.1),
                    );
                }
            }
        }
    }
    // The crown: louvres round the machines.
    let crown = 3.0;
    for f in &faces {
        f.bx(b, "mullion", (0.0, f.len), (top, top + 0.3), (-0.3, 0.05));
        let mut x = 0.1;
        while x < f.len {
            f.bx(
                b,
                "louvre_metal",
                (x, x + 0.06),
                (top + 0.3, top + crown),
                (-0.25, 0.0),
            );
            x += 0.25;
        }
        f.bx(
            b,
            "mullion",
            (0.0, f.len),
            (top + crown, top + crown + 0.15),
            (-0.3, 0.05),
        );
    }
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.3, 0.3, top],
        [w / 2.0 - 0.3, d - 0.3, top + 0.1],
        0.0,
    );
    for i in 0..3 {
        let x = -w / 2.0 + 3.0 + i as f64 * (w - 6.0) / 2.0;
        b.rbox(
            "appliance",
            [x - 1.5, d * 0.35, top + 0.1],
            [x + 1.5, d * 0.65, top + 2.2],
            0.0,
        );
    }
    // The canopy, the name, the forecourt.
    let front = faces[0];
    front.bx(
        b,
        "canopy",
        (w / 2.0 - 4.0, w / 2.0 + 4.0),
        (4.0, 4.25),
        (-set, 2.5),
    );
    front.bx(
        b,
        "sign",
        (w / 2.0 - 3.0, w / 2.0 + 3.0),
        (lobby - 0.55, lobby - 0.05),
        (0.0, 0.08),
    );
    b.rbox(
        "paving",
        [-w / 2.0 - 4.0, -2.0, 0.0],
        [w / 2.0 + 4.0, d + 10.0, 0.03],
        0.0,
    );
    for k in 0..4 {
        let x = -w / 2.0 + 2.0 + k as f64 * (w / 3.0);
        b.rbox(
            "stone",
            [x - 1.0, d + 6.0, 0.0],
            [x + 1.0, d + 8.0, 0.45],
            0.0,
        );
        fruit_tree(b, [x, d + 7.0], 5.5, seed + k, false);
        b.rbox(
            "stone",
            [x - 1.0, d + 4.2, 0.0],
            [x + 1.0, d + 4.6, 0.45],
            0.0,
        );
    }
}

/// The office block of the 1970s: horizontal bands of precast concrete
/// spandrels between strips of windows in anodised aluminium, a ground
/// floor on pilotis with the entrance, brise-soleil on the sunny face, the
/// stair's core of brick on the side and machines on the roof.
pub fn office_1970s(b: &mut Builder, p: &Params) {
    let w = p.get("width", 30.0).clamp(14.0, 60.0);
    let d = p.get("depth", 14.0).clamp(10.0, 24.0);
    let floors = p.get("floors", 6.0).round().clamp(3.0, 14.0) as usize;
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let fl = 3.3;
    let ground = 4.0;
    let top = ground + fl * (floors - 1) as f64;
    let faces = Face::sides(w, d);
    // The ground floor set back on pilotis.
    let set = 2.0;
    let inner = Face::sides(w - 2.0 * set, d - 2.0 * set).map(|f| Face {
        o: [f.o[0], f.o[1] + set],
        ..f
    });
    for (k, f) in inner.iter().enumerate() {
        let door = Hole {
            s: (f.len / 2.0 - 2.0, f.len / 2.0 + 2.0),
            z: (0.0, 3.0),
        };
        if k == 0 {
            curtain_wall(b, f, (0.0, f.len), (0.0, ground), 2.0, ground, 0.0);
            f.bx(b, "door_frame", door.s, (0.0, 3.0), (0.02, 0.1));
            f.bx(
                b,
                "curtain_glass",
                (door.s.0 + 0.1, door.s.1 - 0.1),
                (0.0, 2.9),
                (0.1, 0.11),
            );
        } else {
            wall(b, "panel", f, (0.0, ground), 0.3, &[]);
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + set + 0.3, set + 0.3, 0.0],
        [w / 2.0 - set - 0.3, d - set - 0.3, ground],
        0.0,
    );
    for f in &faces {
        let n = (f.len / 5.0).round().max(1.0) as usize;
        for i in 0..=n {
            let x = (f.len * i as f64 / n as f64).clamp(0.35, f.len - 0.35);
            let c = f.at(x, 0.0, -0.5);
            b.cyl("structure", [c[0], c[1], 0.0], [c[0], c[1], ground], 0.3);
        }
    }
    // The floors: a band of precast spandrels and a strip of windows on
    // each, the windows in panes.
    for (k, f) in faces.iter().enumerate() {
        for i in 1..floors {
            let zf = ground + fl * (i - 1) as f64;
            f.bx(
                b,
                "precast",
                (-0.05, f.len + 0.05),
                (zf - 0.35, zf + 1.0),
                (-0.3, 0.15),
            );
            f.bx(
                b,
                "precast",
                (-0.05, f.len + 0.05),
                (zf + 0.95, zf + 1.05),
                (-0.3, 0.2),
            );
            let h = Hole {
                s: (0.0, f.len),
                z: (zf + 1.0, zf + fl - 0.35),
            };
            f.bx(b, "frame", h.s, (h.z.0, h.z.0 + 0.06), (-0.25, -0.15));
            let n = (f.len / 1.25).round().max(1.0) as usize;
            for j in 0..=n {
                let x = f.len * j as f64 / n as f64;
                f.bx(
                    b,
                    "frame",
                    ((x - 0.03).max(0.0), (x + 0.03).min(f.len)),
                    h.z,
                    (-0.25, -0.15),
                );
            }
            f.bx(b, "glass", h.s, h.z, (-0.21, -0.2));
            // Vertical blinds or curtains behind some panes.
            for j in 0..n {
                if unit(seed, (k * 1000 + i * 50 + j) as u64) > 0.7 {
                    let x = f.len * j as f64 / n as f64;
                    f.bx(
                        b,
                        "curtain",
                        (x + 0.05, x + f.len / n as f64 - 0.05),
                        (h.z.0 + 0.1, h.z.1 - 0.05),
                        (-0.4, -0.37),
                    );
                }
            }
        }
        f.bx(
            b,
            "precast",
            (-0.05, f.len + 0.05),
            (top - 0.35, top + 0.9),
            (-0.3, 0.15),
        );
        // Brise-soleil on the front.
        if k == 0 {
            for i in 1..floors {
                let zf = ground + fl * (i - 1) as f64;
                let mut x = 0.6;
                while x < f.len - 0.3 {
                    f.bx(
                        b,
                        "brise",
                        (x - 0.04, x + 0.04),
                        (zf + 1.05, zf + fl - 0.35),
                        (0.15, 0.75),
                    );
                    x += 1.25;
                }
            }
        }
    }
    b.rbox(
        "interior",
        [-w / 2.0 + 0.4, 0.4, ground],
        [w / 2.0 - 0.4, d - 0.4, top],
        0.0,
    );
    // The stair's core on the side, of brick, over the roof.
    let core_x = w / 2.0 - 0.5;
    b.rbox(
        "brick",
        [core_x, d * 0.3, 0.0],
        [core_x + 4.0, d * 0.7, top + 3.0],
        0.0,
    );
    for i in 0..floors {
        let z = 1.5 + fl * i as f64;
        b.rbox(
            "glass",
            [core_x + 4.0 - 0.01, d * 0.47, z],
            [core_x + 4.02, d * 0.53, z + 1.4],
            0.0,
        );
    }
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.3, 0.3, top + 0.8],
        [w / 2.0 - 0.3, d - 0.3, top + 0.9],
        0.0,
    );
    for i in 0..4 {
        let x = -w / 2.0 + 3.0 + i as f64 * 3.5;
        b.rbox(
            "appliance",
            [x, d * 0.4, top + 0.9],
            [x + 2.4, d * 0.4 + 1.2, top + 2.3],
            0.0,
        );
    }
    let front = faces[0];
    front.bx(
        b,
        "sign",
        (w / 2.0 - 4.0, w / 2.0 + 4.0),
        (ground - 0.6, ground - 0.1),
        (0.15, 0.22),
    );
    b.rbox(
        "paving",
        [-w / 2.0 - 2.0, -1.0, 0.0],
        [w / 2.0 + 6.0, d + 6.0, 0.03],
        0.0,
    );
    for k in 0..3 {
        b.rbox(
            "stone",
            [-w / 2.0 + 4.0 + 8.0 * k as f64, d + 3.0, 0.0],
            [-w / 2.0 + 6.0 + 8.0 * k as f64, d + 3.5, 0.45],
            0.0,
        );
    }
}

/// The building of a business park: three floors of composite panel and
/// glass, a glazed corner with the entrance under a canopy, the company's
/// sign, and its car park with trees, solar canopies over the spaces and
/// the cars under them.
pub fn business_park(b: &mut Builder, p: &Params) {
    let w = p.get("width", 36.0).clamp(18.0, 70.0);
    let d = p.get("depth", 16.0).clamp(10.0, 30.0);
    let seed = p.get("variant", 1.0).round().max(0.0) as u64;
    let fl = 3.8;
    let top = 3.0 * fl + 0.6;
    let faces = Face::sides(w, d);
    let glass_end = w * 0.3; // the glazed part of the front
    for (k, f) in faces.iter().enumerate() {
        // Bands of windows across the panel.
        let mut hs = Vec::new();
        for i in 0..3 {
            let z = fl * i as f64;
            let (a, e) = if k == 0 {
                (glass_end + 0.5, f.len - 0.6)
            } else {
                (0.6, f.len - 0.6)
            };
            hs.push(Hole {
                s: (a, e),
                z: (z + 0.9, z + 2.9),
            });
        }
        if k == 0 {
            hs.push(Hole {
                s: (0.0, glass_end),
                z: (0.0, top - 0.6),
            });
        }
        wall(b, "panel", f, (0.0, top), 0.3, &hs);
        for h in &hs {
            if h.s.0 < 0.1 && k == 0 {
                curtain_wall(b, f, h.s, h.z, 1.5, fl, -0.05);
                continue;
            }
            let n = ((h.s.1 - h.s.0) / 1.5).round().max(1.0) as usize;
            for j in 0..=n {
                let x = h.s.0 + (h.s.1 - h.s.0) * j as f64 / n as f64;
                f.bx(
                    b,
                    "frame",
                    ((x - 0.03).max(h.s.0), (x + 0.03).min(h.s.1)),
                    h.z,
                    (-0.2, -0.12),
                );
            }
            f.bx(b, "glass", h.s, h.z, (-0.17, -0.16));
            for j in 0..n {
                if unit(seed, (k * 100 + j) as u64 + (h.z.0 * 10.0) as u64) > 0.65 {
                    let x = h.s.0 + (h.s.1 - h.s.0) * j as f64 / n as f64;
                    f.bx(
                        b,
                        "blind",
                        (x + 0.05, x + (h.s.1 - h.s.0) / n as f64 - 0.05),
                        (h.z.1 - 0.9, h.z.1),
                        (-0.12, -0.1),
                    );
                }
            }
            // A thin coloured fin between the bands.
            f.bx(
                b,
                "accent",
                (h.s.0, h.s.1),
                (h.z.1 + 0.15, h.z.1 + 0.3),
                (-0.01, 0.35),
            );
        }
        f.bx(
            b,
            "panel",
            (-0.05, f.len + 0.05),
            (top - 0.3, top),
            (-0.01, 0.1),
        );
    }
    b.rbox(
        "interior",
        [-w / 2.0 + 0.3, 0.3, 0.0],
        [w / 2.0 - 0.3, d - 0.3, top - 0.4],
        0.0,
    );
    b.rbox(
        "roof_floor",
        [-w / 2.0 + 0.2, 0.2, top - 0.4],
        [w / 2.0 - 0.2, d - 0.2, top - 0.3],
        0.0,
    );
    let front = faces[0];
    front.bx(
        b,
        "canopy",
        (0.5, glass_end - 0.5),
        (3.2, 3.45),
        (-0.01, 3.0),
    );
    for x in [1.0, glass_end - 1.0] {
        front.bar(b, "mullion", (x, 0.0), (x, 3.2), 2.8, 0.08, 10);
    }
    front.bx(
        b,
        "sign",
        (glass_end + 2.0, glass_end + 10.0),
        (top - 1.6, top - 0.6),
        (-0.01, 0.1),
    );
    front.bx(
        b,
        "sign_text",
        (glass_end + 2.4, glass_end + 9.6),
        (top - 1.4, top - 0.8),
        (0.1, 0.12),
    );
    // The car park: asphalt, lines, trees, the solar canopies and cars.
    let py = d + 2.0;
    b.rbox(
        "paving",
        [-w / 2.0 - 2.0, d, 0.0],
        [w / 2.0 + 2.0, py, 0.03],
        0.0,
    );
    b.rbox(
        "yard",
        [-w / 2.0 - 2.0, py, 0.0],
        [w / 2.0 + 2.0, py + 16.0, 0.02],
        0.0,
    );
    let n = (w / 2.5) as usize;
    for row in 0..2 {
        let y0 = py + 1.0 + row as f64 * 9.0;
        for i in 0..=n {
            let x = -w / 2.0 + 2.5 * i as f64;
            b.rbox("line", [x, y0, 0.02], [x + 0.1, y0 + 5.0, 0.025], 0.0);
        }
        // The solar canopy over the row.
        for i in 0..(n / 4) {
            let x = -w / 2.0 + 2.5 + 10.0 * i as f64;
            b.rbox(
                "mullion",
                [x - 0.12, y0 + 2.4, 0.02],
                [x + 0.12, y0 + 2.6, 2.7],
                0.0,
            );
        }
        let t: f64 = 0.15;
        let c = [0.0, y0 + 2.5, 2.9];
        b.obox(
            "panel_solar",
            c,
            [w / 2.0 - 1.0, 0.0, 0.0],
            [0.0, 2.8 * t.cos(), 2.8 * t.sin()],
            [0.0, -0.03 * t.sin(), 0.03 * t.cos()],
        );
        b.rbox(
            "mullion",
            [-w / 2.0 + 1.0, y0 + 2.3, 2.7],
            [w / 2.0 - 1.0, y0 + 2.7, 2.85],
            0.0,
        );
        for i in 0..n {
            if unit(seed, (row * 100 + i) as u64) > 0.45 {
                let x = -w / 2.0 + 2.5 * i as f64 + 1.25;
                car(b, [x, y0 + 2.5], unit(seed, (row * 100 + i + 50) as u64));
            }
        }
    }
    for k in 0..5 {
        let x = -w / 2.0 + 1.0 + k as f64 * (w - 2.0) / 4.0;
        fruit_tree(b, [x, py + 7.5], 4.5, seed + k, false);
    }
}

/// A car seen from afar: body, cabin with its glass, four wheels; its
/// colour one of three.
pub fn car(b: &mut Builder, c: [f64; 2], pick: f64) {
    let role = ["car_a", "car_b", "car_c"][(pick * 3.0) as usize % 3];
    let (hl, hw) = (0.9, 2.15);
    b.rbox(
        role,
        [c[0] - hl, c[1] - hw, 0.3],
        [c[0] + hl, c[1] + hw, 0.85],
        0.12,
    );
    b.rbox(
        "car_glass",
        [c[0] - hl + 0.1, c[1] - 1.1, 0.85],
        [c[0] + hl - 0.1, c[1] + 0.9, 1.35],
        0.12,
    );
    b.rbox(
        role,
        [c[0] - hl + 0.15, c[1] - 0.9, 1.33],
        [c[0] + hl - 0.15, c[1] + 0.7, 1.42],
        0.05,
    );
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = [c[0] + dx * (hl - 0.05), c[1] + dy * (hw - 0.65)];
        b.cone(
            "tyre",
            [p[0] - 0.12, p[1], 0.32],
            [p[0] + 0.12, p[1], 0.32],
            0.32,
            0.32,
            12,
        );
    }
}
