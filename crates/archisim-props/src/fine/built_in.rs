//! The built-in wardrobe (E10, D25: «armario empotrado»): a carcass that
//! fills a recess or the room between two partitions, from the floor to
//! (nearly) the ceiling, with hinged or sliding fronts, cover strips
//! round it and its inside: shelves, a hanging rail or drawers.
//!
//! Axes as every piece of furniture: `x` across, centred; `y` from the
//! back (against the wall, 0) to the front (`depth`); `z` up.
//!
//! Parameters: `width`, `depth`, `height`, `doors`, `sliding` (0 hinged,
//! 1 sliding), `interior` (1 shelves, 2 hanging rail and a shelf, 3
//! drawers below and a rail), `trims` (cover strips, 0 or 1) and `open`
//! (0–100: how open its fronts are in 3D, to see inside).
//!
//! Roles: `frame` (carcass), `inner` (shelves and partitions), `front`
//! (doors and drawer fronts), `metal` (rail, track, pulls), `trim`.

use crate::build::Builder;
use crate::Params;

/// Board thickness.
const T: f64 = 0.019;
/// Front thickness.
const F: f64 = 0.019;
/// Gap between fronts.
const GAP: f64 = 0.003;

pub fn wardrobe(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.8).clamp(0.4, 6.0);
    let d = p.get("depth", 0.6).clamp(0.3, 1.2);
    let h = p.get("height", 2.5).clamp(0.8, 3.5);
    let doors = p.get("doors", 2.0).round().clamp(1.0, 12.0) as usize;
    let sliding = p.get("sliding", 0.0) > 0.5;
    // (By number, as the family's choice: 1 shelves, 2 rail, 3 drawers.)
    let interior = (p.get("interior", 2.0).round() - 1.0).clamp(0.0, 2.0) as usize;
    let trim = p.get("trims", 1.0) > 0.5;
    let open = (p.get("open", 0.0) / 100.0).clamp(0.0, 1.0);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let plinth = 0.08;
    // Room for the fronts in front of the carcass.
    let fronts = if sliding { 0.07 } else { F + 0.002 };
    let cy = d - fronts;

    // The carcass: sides, bottom, top and back.
    b.rbox("frame", [x0, 0.0, 0.0], [x0 + T, cy, h], 0.0);
    b.rbox("frame", [x1 - T, 0.0, 0.0], [x1, cy, h], 0.0);
    b.rbox(
        "frame",
        [x0 + T, 0.0, plinth],
        [x1 - T, cy, plinth + T],
        0.0,
    );
    b.rbox("frame", [x0 + T, 0.0, h - T], [x1 - T, cy, h], 0.0);
    b.rbox(
        "frame",
        [x0 + T, 0.0, plinth + T],
        [x1 - T, 0.008, h - T],
        0.0,
    );
    b.rbox(
        "frame",
        [x0 + T, cy - 0.06, 0.0],
        [x1 - T, cy - 0.04, plinth],
        0.0,
    );

    // Bays: one each two doors (one for a single door).
    let bays = (doors / 2).max(1);
    let inner_w = w - 2.0 * T;
    let bay = (inner_w - (bays - 1) as f64 * T) / bays as f64;
    let (zb, zt) = (plinth + T, h - T);
    for k in 0..bays {
        let bx0 = x0 + T + k as f64 * (bay + T);
        let bx1 = bx0 + bay;
        if k + 1 < bays {
            b.rbox("inner", [bx1, 0.008, zb], [bx1 + T, cy, zt], 0.0);
        }
        let shelf = |b: &mut Builder, z: f64| {
            b.rbox("inner", [bx0, 0.008, z], [bx1, cy - 0.01, z + T], 0.0);
        };
        let rail = |b: &mut Builder, z: f64| {
            let y = (cy / 2.0).max(0.05);
            b.cyl("metal", [bx0, y, z], [bx1, y, z], 0.012);
        };
        match interior {
            0 => {
                let n = ((zt - zb) / 0.4).floor().max(1.0) as usize;
                let step = (zt - zb) / (n as f64 + 1.0);
                for i in 1..=n {
                    shelf(b, zb + i as f64 * step);
                }
            }
            1 => {
                let z = (zt - 0.32).max(zb + 0.4);
                shelf(b, z);
                rail(b, z - 0.06);
            }
            _ => {
                // Three drawers below, a shelf over them and a rail.
                let dh = 0.2;
                for i in 0..3 {
                    let z = zb + i as f64 * dh;
                    // (Stacked on the bottom, between the sides, 2 mm
                    // apart.)
                    b.rbox(
                        "front",
                        [bx0, 0.02, z],
                        [bx1, cy - 0.01, z + dh - 0.002],
                        0.0,
                    );
                    let mid = (bx0 + bx1) / 2.0;
                    b.rbox(
                        "metal",
                        [mid - 0.06, cy - 0.01, z + dh / 2.0 - 0.006],
                        [mid + 0.06, cy - 0.002, z + dh / 2.0 + 0.006],
                        0.0,
                    );
                }
                let zs = zb + 3.0 * dh;
                shelf(b, zs);
                let z = (zt - 0.32).max(zs + 0.5);
                shelf(b, z);
                rail(b, z - 0.06);
            }
        }
    }

    // The fronts, from the floor's plinth to the top.
    let (fz0, fz1) = (plinth + GAP, h - GAP);
    let fw = (w - (doors + 1) as f64 * GAP) / doors as f64;
    if sliding {
        // Two tracks, at the top and the bottom; doors on alternate ones,
        // overlapping 3 cm. Open: the front ones slide over the back ones.
        b.rbox("metal", [x0 + T, cy, h - 0.035], [x1 - T, d, h], 0.0);
        b.rbox(
            "metal",
            [x0 + T, cy, plinth - 0.01],
            [x1 - T, d, plinth],
            0.0,
        );
        let sw = (w - 2.0 * T + (doors - 1) as f64 * 0.03) / doors as f64;
        for i in 0..doors {
            let front = i % 2 == 1;
            let (ya, yb) = if front {
                (d - 0.03, d - 0.03 + F)
            } else {
                (cy + 0.005, cy + 0.005 + F)
            };
            let mut xa = x0 + T + i as f64 * (sw - 0.03);
            if front {
                xa -= open * (sw - 0.03);
            }
            b.rbox("front", [xa, ya, fz0], [xa + sw, yb, fz1 - 0.035], 0.0);
            // A recessed pull: a slim bar on the face.
            let px = if front { xa + 0.04 } else { xa + sw - 0.05 };
            b.rbox("metal", [px, yb, 0.95], [px + 0.012, yb + 0.006, 1.25], 0.0);
        }
    } else {
        for i in 0..doors {
            let xa = x0 + GAP + i as f64 * (fw + GAP);
            // Pairs open from the middle: the left one of a pair hinges
            // on its left, the right one on its right.
            let hinge_left = i % 2 == 0 || doors == 1;
            let (hx, sign) = if hinge_left {
                (xa, 1.0)
            } else {
                (xa + fw, -1.0)
            };
            let a = open * 100f64.to_radians();
            let (c, s) = (a.cos(), a.sin());
            // The leaf turned about its hinge (on the front face's line).
            let half = fw / 2.0;
            let centre = [
                hx + sign * half * c,
                cy + F / 2.0 + half * s,
                (fz0 + fz1) / 2.0,
            ];
            b.obox(
                "front",
                centre,
                [sign * half * c, half * s, 0.0],
                [-s * F / 2.0 * sign, c * F / 2.0, 0.0],
                [0.0, 0.0, (fz1 - fz0) / 2.0],
            );
            // Its pull, near the free edge, on the face.
            let fx = hx + sign * (fw - 0.05) * c;
            let fy = cy + (fw - 0.05) * s + F * c;
            b.rbox(
                "metal",
                [fx - 0.006, fy, 0.95],
                [fx + 0.006, fy + 0.012, 1.25],
                0.0,
            );
        }
    }

    // Cover strips round the opening, on the wall's face.
    if trim {
        let t = 0.05;
        b.rbox("trim", [x0 - t, d - 0.012, 0.0], [x0, d, h + t], 0.0);
        b.rbox("trim", [x1, d - 0.012, 0.0], [x1 + t, d, h + t], 0.0);
        b.rbox("trim", [x0, d - 0.012, h], [x1, d, h + t], 0.0);
    }
}
