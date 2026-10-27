//! Bathroom and kitchen accessories (bloque 5 de la revisión del
//! 22/10/2026): mirrors, towel rails, toilet roll holders, grab bars,
//! cooker hoods and shelves. They hang on the wall at `y = 0`, at their
//! `elevation` over the floor (the bottom of a mirror or a hood, the
//! height of a bar).

use crate::build::Builder;
use crate::Params;

/// Mirrors: 1 framed, 2 frameless, 3 round.
pub fn mirror(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8);
    let h = p.get("height", 1.0);
    let z0 = p.get("elevation", 1.0);
    let x0 = -w / 2.0;
    match p.style() {
        3 => {
            let r = w.min(h) / 2.0;
            let c = [0.0, 0.0, z0 + r];
            b.cone("frame", [c[0], 0.0, c[2]], [c[0], 0.02, c[2]], r, r, 40);
            b.cone(
                "glass",
                [c[0], 0.02, c[2]],
                [c[0], 0.026, c[2]],
                r - 0.02,
                r - 0.02,
                40,
            );
        }
        2 => {
            b.rbox("glass", [x0, 0.0, z0], [-x0, 0.006, z0 + h], 0.004);
        }
        _ => {
            b.rbox("frame", [x0, 0.0, z0], [-x0, 0.025, z0 + h], 0.004);
            b.rbox(
                "glass",
                [x0 + 0.04, 0.025, z0 + 0.04],
                [-x0 - 0.04, 0.03, z0 + h - 0.04],
                0.0,
            );
        }
    }
}

/// Towel rails: 1 a bar on two posts, 2 a heated ladder.
pub fn towel_rail(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.6);
    let z = p.get("elevation", 1.1);
    let x0 = -w / 2.0;
    if p.style() == 2 {
        let h = p.get("height", 0.9);
        for x in [x0 + 0.02, -x0 - 0.02] {
            b.cyl("metal", [x, 0.06, z], [x, 0.06, z + h], 0.014);
            b.cyl("metal", [x, 0.0, z + 0.08], [x, 0.06, z + 0.08], 0.008);
            b.cyl(
                "metal",
                [x, 0.0, z + h - 0.08],
                [x, 0.06, z + h - 0.08],
                0.008,
            );
        }
        let rungs = ((h - 0.1) / 0.06).floor().max(2.0) as usize;
        for k in 0..rungs {
            let zz = z + 0.05 + k as f64 * (h - 0.1) / (rungs - 1) as f64;
            b.cyl(
                "metal",
                [x0 + 0.02, 0.06, zz],
                [-x0 - 0.02, 0.06, zz],
                0.009,
            );
        }
        return;
    }
    for x in [x0 + 0.02, -x0 - 0.02] {
        b.cyl("metal", [x, 0.0, z], [x, 0.08, z], 0.01);
        b.cone("metal", [x, 0.0, z], [x, 0.008, z], 0.025, 0.025, 16);
    }
    b.cyl("metal", [x0, 0.08, z], [-x0, 0.08, z], 0.011);
}

/// Toilet roll holders: a post with an arm and its roll.
pub fn toilet_roll(b: &mut Builder, p: &Params) {
    let z = p.get("elevation", 0.7);
    b.cone("metal", [0.0, 0.0, z], [0.0, 0.008, z], 0.03, 0.03, 16);
    b.cyl("metal", [0.0, 0.0, z], [0.0, 0.06, z], 0.008);
    b.cyl("metal", [0.0, 0.06, z], [0.13, 0.06, z], 0.007);
    b.cone("linen", [0.0, 0.06, z], [0.11, 0.06, z], 0.055, 0.055, 20);
}

/// Grab bars (accessible bathrooms): 1 fixed, 2 folding (up from a wall
/// plate, lying forwards).
pub fn grab_bar(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.7);
    let z = p.get("elevation", 0.75);
    if p.style() == 2 {
        b.rbox("metal", [-0.06, 0.0, z - 0.1], [0.06, 0.01, z + 0.1], 0.004);
        let r = 0.016;
        b.cyl("metal", [-0.04, 0.01, z], [-0.04, w, z], r);
        b.cyl(
            "metal",
            [0.04, 0.01, z - 0.06],
            [0.04, w - 0.04, z - 0.06],
            r,
        );
        b.cyl("metal", [-0.04, w, z], [0.04, w - 0.04, z - 0.06], r);
        return;
    }
    let x0 = -w / 2.0;
    for x in [x0, -x0] {
        b.cone("metal", [x, 0.0, z], [x, 0.008, z], 0.035, 0.035, 16);
        b.cyl("metal", [x, 0.0, z], [x, 0.06, z], 0.016);
    }
    b.cyl("metal", [x0, 0.06, z], [-x0, 0.06, z], 0.016);
}

/// Cooker hoods: 1 a chimney hood, 2 built into the wall units.
pub fn hood(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2a).
    if crate::fine::kitchen::hood(b, p) {
        return;
    }
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.5);
    let z0 = p.get("elevation", 1.6);
    let x0 = -w / 2.0;
    if p.style() == 2 {
        b.rbox("metal", [x0, 0.0, z0], [-x0, d, z0 + 0.06], 0.003);
        b.rbox(
            "dark",
            [x0 + 0.03, 0.05, z0 - 0.002],
            [-x0 - 0.03, d - 0.05, z0],
            0.0,
        );
        return;
    }
    let top = p.get("height", 0.9) + z0;
    b.rbox("metal", [x0, 0.0, z0], [-x0, d, z0 + 0.05], 0.004);
    // The canopy narrows towards the chimney.
    b.slab(
        "metal",
        &[
            [x0 + 0.02, 0.0],
            [-x0 - 0.02, 0.0],
            [-x0 - 0.02, d - 0.02],
            [x0 + 0.02, d - 0.02],
        ],
        z0 + 0.05,
        z0 + 0.12,
    );
    let cw = (w * 0.45).min(0.3);
    b.rbox(
        "metal",
        [-cw / 2.0, 0.0, z0 + 0.12],
        [cw / 2.0, 0.26, top],
        0.0,
    );
    b.rbox(
        "dark",
        [x0 + 0.05, 0.05, z0 - 0.002],
        [-x0 - 0.05, d - 0.05, z0],
        0.0,
    );
}

/// Shelves: a board on two brackets (1) or floating (2).
pub fn shelf(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8);
    let d = p.get("depth", 0.25);
    let z = p.get("elevation", 1.4);
    let t = p.get("board", 0.025);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, z], [-x0, d, z + t], 0.003);
    if p.style() == 1 {
        for x in [x0 + w * 0.18, -x0 - w * 0.18] {
            b.rbox(
                "frame",
                [x - 0.01, 0.0, z - 0.18],
                [x + 0.01, 0.012, z],
                0.0,
            );
            b.rbox(
                "frame",
                [x - 0.01, 0.0, z - 0.012],
                [x + 0.01, d * 0.8, z],
                0.0,
            );
        }
    }
}
