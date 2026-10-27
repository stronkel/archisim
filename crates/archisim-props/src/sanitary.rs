//! Sanitary ware: toilets, bidets, washbasins, baths and shower trays.
//! The back on the wall (`y = 0`), the front towards +y.

use crate::build::Builder;
use crate::kitchen::tap;
use crate::Params;

/// Toilets: 1 on the floor with its cistern; 2 hung on the wall, with the
/// flush plate.
pub fn wc(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2b).
    if crate::fine::bath::wc(b, p) {
        return;
    }
    let w = p.get("width", 0.38);
    let d = p.get("depth", 0.68);
    let hung = p.style() == 2;
    let seat = 0.42;
    let (bowl_y, bowl_r) = (d * 0.6, w / 2.0);
    // The bowl reaches the front.
    let stretch = ((d - bowl_y - 0.01) / bowl_r).max(1.0);
    let base = if hung { 0.22 } else { 0.0 };
    // The bowl: an oval tapering to its foot (or to the wall, hung).
    let profile: Vec<(f64, f64)> = if hung {
        vec![
            (0.55, 0.0),
            (0.85, 0.08),
            (1.0, seat - base - 0.04),
            (0.98, seat - base - 0.02),
        ]
    } else {
        vec![
            (0.45, 0.0),
            (0.5, 0.18),
            (0.85, 0.3),
            (1.0, seat - 0.04),
            (0.98, seat - 0.02),
        ]
    }
    .into_iter()
    .map(|(k, z)| (k * bowl_r, z))
    .collect();
    b.lathe("mat", [0.0, bowl_y, base], &profile, 1.0, stretch);
    // Back of the bowl to the wall or cistern.
    b.rbox(
        "mat",
        [-w * 0.32, 0.12, base],
        [w * 0.32, bowl_y, seat - 0.04],
        0.04,
    );
    // Seat and lid, closed.
    b.lathe(
        "seat",
        [0.0, bowl_y, seat - 0.02],
        &[(bowl_r, 0.0), (bowl_r, 0.025), (0.0, 0.025)],
        1.0,
        stretch,
    );
    b.rbox(
        "seat",
        [-0.06, 0.13, seat - 0.02],
        [0.06, 0.18, seat + 0.01],
        0.01,
    );
    if hung {
        b.rbox("metal", [-0.11, 0.0, 1.0], [0.11, 0.012, 1.15], 0.008);
    } else {
        b.rbox(
            "mat",
            [-w * 0.48, 0.0, seat - 0.05],
            [w * 0.48, 0.17, 0.82],
            0.03,
        );
        b.rbox(
            "mat",
            [-w * 0.5, -0.005, 0.81],
            [w * 0.5, 0.18, 0.84],
            0.015,
        );
        b.cyl("metal", [0.0, 0.08, 0.84], [0.0, 0.08, 0.85], 0.025);
    }
}

/// Bidets, hung or on the floor, with their tap.
pub fn bidet(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2b).
    if crate::fine::bath::bidet(b, p) {
        return;
    }
    let w = p.get("width", 0.36);
    let d = p.get("depth", 0.55);
    let hung = p.style() == 2;
    let base = if hung { 0.18 } else { 0.0 };
    let top = 0.4;
    let r = w / 2.0;
    let stretch = ((d * 0.42 - 0.01) / r).max(1.0);
    let profile: Vec<(f64, f64)> = if hung {
        vec![(0.6 * r, 0.0), (0.9 * r, 0.06), (r, top - base)]
    } else {
        vec![(0.45 * r, 0.0), (0.5 * r, 0.15), (0.9 * r, 0.28), (r, top)]
    };
    b.lathe("mat", [0.0, d * 0.58, base], &profile, 1.0, stretch);
    b.rbox("mat", [-w * 0.3, 0.0, base], [w * 0.3, d * 0.58, top], 0.03);
    // The basin, seen from above.
    b.lathe(
        "water",
        [0.0, d * 0.58, top - 0.001],
        &[(r * 0.75, 0.0), (r * 0.75, 0.002), (0.0, 0.002)],
        1.0,
        stretch,
    );
    tap(b, 0.0, 0.08, top, 0.06);
}

/// Washbasins: 1 on a pedestal; 2 on a vanity unit; 3 hung on the wall.
/// `height` is the rim's.
pub fn washbasin(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2b).
    if crate::fine::bath::washbasin(b, p) {
        return;
    }
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.45);
    let h = p.get("height", 0.85);
    let style = p.style();
    let x0 = -w / 2.0;
    let rim = 0.12;
    match style {
        2 => {
            b.rbox("unit", [x0, 0.0, 0.12], [-x0, d - 0.02, h - 0.03], 0.004);
            b.rbox(
                "unit",
                [x0 + 0.003, d - 0.02, 0.125],
                [-x0 - 0.003, d, h - 0.035],
                0.003,
            );
            b.rbox(
                "metal",
                [-0.1, d, h - 0.1],
                [0.1, d + 0.012, h - 0.088],
                0.004,
            );
            b.rbox(
                "unit",
                [x0 + 0.04, 0.0, 0.0],
                [-x0 - 0.04, d - 0.06, 0.12],
                0.0,
            );
        }
        1 => {
            b.lathe(
                "mat",
                [0.0, d * 0.35, 0.0],
                &[
                    (0.1, 0.0),
                    (0.08, 0.05),
                    (0.075, h - rim - 0.05),
                    (0.12, h - rim),
                ],
                1.0,
                0.8,
            );
        }
        _ => {}
    }
    // The basin: a slab with its bowl hollowed out.
    let (bx0, bx1) = (x0 + 0.06, -x0 - 0.06);
    let (by0, by1) = (0.12, d - 0.05);
    b.framed_top(
        "mat",
        [x0, 0.0, h - rim],
        [-x0, d, h],
        [bx0, by0],
        [bx1, by1],
    );
    b.hollow("mat", [bx0, by0, h - rim + 0.01], [bx1, by1, h], 0.06);
    b.rbox(
        "mat",
        [x0 + 0.03, 0.03, h - rim - 0.02],
        [-x0 - 0.03, d - 0.02, h - rim + 0.005],
        0.03,
    );
    tap(b, 0.0, 0.06, h, 0.1);
}

/// Baths: a panelled tub with its hollow and a tap at one end.
pub fn bath(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2b).
    if crate::fine::bath::bath(b, p) {
        return;
    }
    let l = p.get("width", 1.7);
    let w = p.get("depth", 0.7);
    let h = p.get("height", 0.55);
    let x0 = -l / 2.0;
    let rim = 0.06;
    let (hx0, hx1, hy0, hy1) = (x0 + rim + 0.04, -x0 - rim, rim, w - rim);
    // The panels round the tub, its bottom, and the hollow seen from above.
    b.framed_top("mat", [x0, 0.0, 0.0], [-x0, w, h], [hx0, hy0], [hx1, hy1]);
    b.rbox("mat", [hx0, hy0, 0.0], [hx1, hy1, 0.11], 0.0);
    b.hollow("mat", [hx0, hy0, 0.12], [hx1, hy1, h], 0.12);
    b.cyl(
        "metal",
        [x0 + 0.05, w / 2.0, h],
        [x0 + 0.05, w / 2.0, h + 0.06],
        0.015,
    );
    b.cyl(
        "metal",
        [x0 + 0.05, w / 2.0, h + 0.06],
        [x0 + 0.15, w / 2.0, h + 0.06],
        0.012,
    );
}

/// Shower trays, with their drain and (2, 3) a glass screen on one or
/// two sides and a shower column.
pub fn shower(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2b).
    if crate::fine::bath::shower(b, p) {
        return;
    }
    let w = p.get("width", 0.9);
    let d = p.get("depth", 0.9);
    let h = p.get("height", 0.04).max(0.01);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, 0.0], [-x0, d, h], 0.008);
    b.cyl("metal", [0.0, d / 2.0, h], [0.0, d / 2.0, h + 0.002], 0.05);
    let style = p.style();
    if style >= 2 {
        let top = 2.0;
        b.rbox("glass", [-x0 - 0.01, 0.0, h], [-x0, d, top], 0.0);
        if style == 3 {
            b.rbox("glass", [x0 + w * 0.4, d - 0.01, h], [-x0, d, top], 0.0);
        }
        b.cyl(
            "metal",
            [x0 + 0.1, 0.03, h],
            [x0 + 0.1, 0.03, top + 0.1],
            0.012,
        );
        b.cyl(
            "metal",
            [x0 + 0.1, 0.03, top + 0.1],
            [x0 + 0.1, 0.3, top + 0.1],
            0.01,
        );
        b.cone(
            "metal",
            [x0 + 0.1, 0.3, top + 0.1],
            [x0 + 0.1, 0.3, top + 0.08],
            0.1,
            0.1,
            24,
        );
    }
}
