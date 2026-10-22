//! Kitchen: base units with worktop, sink and hob; wall units; fridge and
//! washing machine. Axes as the furniture's: the back on the wall.

use crate::build::Builder;
use crate::Params;

/// Doors of a run of units: as many as fit, about 60 cm each.
fn fronts(b: &mut Builder, x0: f64, w: f64, y: f64, z0: f64, z1: f64, handles_low: bool) {
    let n = (w / 0.6).round().max(1.0) as usize;
    let fw = w / n as f64;
    for i in 0..n {
        let a = x0 + i as f64 * fw + 0.002;
        b.rbox(
            "mat",
            [a, y - 0.002, z0],
            [a + fw - 0.004, y + 0.018, z1],
            0.002,
        );
        let hz = if handles_low { z1 - 0.05 } else { z0 + 0.05 };
        let xc = a + fw / 2.0;
        b.rbox(
            "metal",
            [xc - 0.08, y + 0.018, hz - 0.006],
            [xc + 0.08, y + 0.03, hz + 0.006],
            0.004,
        );
    }
}

/// A tap: a column and a spout over `(x, y)` on a surface at `z`.
pub fn tap(b: &mut Builder, x: f64, y: f64, z: f64, reach: f64) {
    b.cyl("metal", [x, y, z], [x, y, z + 0.28], 0.014);
    b.cyl("metal", [x, y, z + 0.28], [x, y + reach, z + 0.28], 0.011);
    b.cyl(
        "metal",
        [x, y + reach, z + 0.28],
        [x, y + reach, z + 0.24],
        0.011,
    );
    b.cyl(
        "metal",
        [x + 0.035, y, z + 0.12],
        [x + 0.07, y, z + 0.14],
        0.007,
    );
}

/// Base units: plinth, carcass and doors, worktop; the sink near the left
/// end and the hob near the right, as the plan.
pub fn counter(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.4);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.9);
    let sink = p.flag("sink", true);
    let hob = p.flag("hob", true);
    let x0 = -w / 2.0;
    let top = 0.03;
    b.rbox("plinth", [x0, 0.0, 0.0], [-x0, d - 0.08, 0.1], 0.0);
    // The carcass, lower under the sink so the basin is seen.
    let (sx0, sx1) = (x0 + 0.12, x0 + 0.48);
    if sink {
        b.rbox("mat", [x0, 0.0, 0.1], [sx0, d - 0.04, h - top], 0.0);
        b.rbox("mat", [sx0, 0.0, 0.1], [sx1, d - 0.04, h - 0.21], 0.0);
        b.rbox("mat", [sx1, 0.0, 0.1], [-x0, d - 0.04, h - top], 0.0);
    } else {
        b.rbox("mat", [x0, 0.0, 0.1], [-x0, d - 0.04, h - top], 0.0);
    }
    fronts(b, x0, w, d - 0.04, 0.105, h - top - 0.005, true);
    // Worktop, cut for the sink.
    let (tlo, thi) = ([x0, 0.0, h - top], [-x0, d, h]);
    if sink {
        let (sy0, sy1) = (0.1, d - 0.12);
        b.framed_top("top", tlo, thi, [sx0, sy0], [sx1, sy1]);
        b.hollow("metal", [sx0, sy0, h - 0.2], [sx1, sy1, h], 0.03);
        tap(b, (sx0 + sx1) / 2.0, 0.05, h, 0.16);
    } else {
        b.rbox("top", tlo, thi, 0.0);
    }
    if hob {
        let (cx, cy) = (-x0 - 0.35, d / 2.0);
        b.rbox(
            "dark",
            [cx - 0.29, cy - 0.25, h],
            [cx + 0.29, cy + 0.25, h + 0.006],
            0.004,
        );
        for (dx, dy, r) in [
            (-0.13, -0.11, 0.09),
            (0.13, -0.11, 0.07),
            (-0.13, 0.11, 0.07),
            (0.13, 0.11, 0.09),
        ] {
            b.cone(
                "ring",
                [cx + dx, cy + dy, h + 0.006],
                [cx + dx, cy + dy, h + 0.008],
                r,
                r,
                24,
            );
        }
    }
}

/// Wall units, at their height over the floor.
pub fn wall_cabinet(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.2);
    let d = p.get("depth", 0.35);
    let h = p.get("height", 0.7);
    let z0 = p.get("elevation", 1.45);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, z0], [-x0, d - 0.02, z0 + h], 0.0);
    let doors = p.get("doors", 0.0).round() as usize;
    let n = if doors > 0 {
        doors
    } else {
        (w / 0.6).round().max(1.0) as usize
    };
    let fw = w / n as f64;
    for i in 0..n {
        let a = x0 + i as f64 * fw + 0.002;
        b.rbox(
            "mat",
            [a, d - 0.02, z0 + 0.003],
            [a + fw - 0.004, d, z0 + h - 0.003],
            0.002,
        );
        let xc = a + fw / 2.0;
        b.rbox(
            "metal",
            [xc - 0.06, d, z0 + 0.04],
            [xc + 0.06, d + 0.012, z0 + 0.052],
            0.004,
        );
    }
}

/// Fridge: rounded box, freezer door below, two long handles.
pub fn fridge(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.65);
    let h = p.get("height", 1.85);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, 0.0], [-x0, d - 0.05, h], 0.02);
    let split = h * 0.38;
    b.rbox("mat", [x0, d - 0.05, 0.02], [-x0, d, split - 0.004], 0.015);
    b.rbox(
        "mat",
        [x0, d - 0.05, split + 0.004],
        [-x0, d, h - 0.004],
        0.015,
    );
    let hx = -x0 - 0.05;
    b.rbox(
        "metal",
        [hx - 0.012, d, split - 0.3],
        [hx + 0.012, d + 0.03, split - 0.06],
        0.008,
    );
    b.rbox(
        "metal",
        [hx - 0.012, d, split + 0.06],
        [hx + 0.012, d + 0.03, split + 0.5],
        0.008,
    );
}

/// Washing machine: a box with a control strip and a round door.
pub fn washer(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.85);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, 0.0], [-x0, d, h], 0.015);
    b.rbox(
        "dark",
        [x0 + 0.02, d, h - 0.11],
        [-x0 - 0.02, d + 0.004, h - 0.02],
        0.004,
    );
    b.cyl(
        "metal",
        [-x0 - 0.08, d, h - 0.065],
        [-x0 - 0.08, d + 0.02, h - 0.065],
        0.025,
    );
    let (cz, r) = (h * 0.45, (w * 0.32).min(h * 0.3));
    b.cyl("metal", [0.0, d, cz], [0.0, d + 0.03, cz], r);
    b.cone(
        "glass",
        [0.0, d + 0.03, cz],
        [0.0, d + 0.045, cz],
        r * 0.78,
        r * 0.7,
        24,
    );
}
