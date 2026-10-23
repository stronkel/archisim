//! Furniture: sofas and armchairs, chairs, tables (with their chairs),
//! coffee tables, beds, bedside tables, wardrobes, chests of drawers,
//! desks, bookcases and TV units.
//! Axes of the library's furniture: origin in the middle of the back,
//! `x` to the right, `y` forwards (away from the wall), `z` up.

use crate::build::Builder;
use crate::fronts::{self, Handle, FRONT};
use crate::Params;

/// Sofas (and armchairs, with one place): 1 contemporary, on a plinth,
/// with loose cushions; 2 classic, rolled arms and a skirt; 3 Nordic, on
/// slim wooden legs.
pub fn sofa(b: &mut Builder, p: &Params) {
    let w = p.get("width", 2.0);
    let d = p.get("depth", 0.9);
    let seat = p.get("seat", 0.42);
    let h = p.get("height", 0.8);
    let places = p.get("places", 3.0).round().max(1.0);
    let style = p.style();
    let x0 = -w / 2.0;
    let arm = (w * 0.1).clamp(0.12, 0.22);
    let arm_h = (seat + 0.2).min(h - 0.02);
    let back = 0.2;
    let (foot, base_r) = match style {
        3 => (0.16, 0.03),
        2 => (0.0, 0.04),
        _ => (0.05, 0.02),
    };
    // Legs or plinth.
    match style {
        3 => {
            for (x, y) in [
                (x0 + 0.08, 0.08),
                (-x0 - 0.08, 0.08),
                (x0 + 0.08, d - 0.08),
                (-x0 - 0.08, d - 0.08),
            ] {
                let dx = if x < 0.0 { -0.03 } else { 0.03 };
                let dy = if y < d / 2.0 { -0.03 } else { 0.03 };
                b.cone(
                    "frame",
                    [x + dx, y + dy, 0.004],
                    [x, y, foot],
                    0.015,
                    0.022,
                    10,
                );
            }
        }
        1 => b.rbox(
            "frame",
            [x0 + 0.04, 0.04, 0.0],
            [-x0 - 0.04, d - 0.04, foot],
            0.005,
        ),
        _ => {}
    }
    // The frame: back and base.
    b.rbox("mat", [x0, 0.0, foot], [-x0, back, h], base_r);
    let base_top = seat - 0.12;
    b.rbox("mat", [x0, back - 0.02, foot], [-x0, d, base_top], base_r);
    // Arms.
    for side in [-1.0, 1.0] {
        let (a0, a1) = if side < 0.0 {
            (x0, x0 + arm)
        } else {
            (-x0 - arm, -x0)
        };
        match style {
            2 => {
                b.rbox("mat", [a0, 0.0, foot], [a1, d, arm_h - 0.05], 0.03);
                let r = (arm * 0.6).min(0.1);
                let xc = (a0 + a1) / 2.0;
                b.cyl(
                    "mat",
                    [xc, 0.02, arm_h - r * 0.6],
                    [xc, d, arm_h - r * 0.6],
                    r,
                );
            }
            _ => b.rbox(
                "mat",
                [a0, 0.0, foot],
                [a1, d, arm_h],
                if style == 3 { 0.04 } else { 0.03 },
            ),
        }
    }
    // Seat and back cushions, one per place.
    let inner = w - 2.0 * arm;
    let pw = inner / places;
    for i in 0..places as usize {
        let a = x0 + arm + i as f64 * pw + 0.005;
        let c = a + pw - 0.01;
        b.pillow(
            "cushion",
            [a, back, base_top],
            [c, d - 0.01, seat],
            0.05,
            0.025,
        );
        let top = if style == 2 { h - 0.02 } else { h + 0.04 };
        b.pillow(
            "cushion",
            [a, back - 0.02, seat - 0.02],
            [c, back + 0.17, top],
            0.06,
            0.035,
        );
    }
}

/// Chairs: 1 wooden, with back slats; 2 bent metal tube with a shell;
/// 3 upholstered. `dir` > 0: the chair looks towards +y (at the back of a
/// table); its seat from `y0` to `y0 + d`.
pub fn chair_at(b: &mut Builder, p: &Params, x: f64, y0: f64, dir: f64, style: i32) {
    let w = p.get("width", 0.45);
    let d = p.get("depth", 0.5);
    let seat = p.get("seat", 0.45);
    let h = p.get("height", 0.9);
    let (x0, x1) = (x - w / 2.0, x + w / 2.0);
    // Along the chair: 0 at its front edge, d at its back (seen from the
    // chair's front: dir > 0 means its back is at y0 + d).
    let y = |t: f64| if dir > 0.0 { y0 + t } else { y0 + d - t };
    let span = |t0: f64, t1: f64| {
        let (a, c) = (y(t0), y(t1));
        (a.min(c), a.max(c))
    };
    match style {
        2 => {
            let r = 0.011;
            // Two side frames of tube, joined.
            for xs in [x0 + 0.02, x1 - 0.02] {
                b.cyl("frame", [xs, y(0.03), r], [xs, y(0.03), seat - 0.02], r);
                b.cyl(
                    "frame",
                    [xs, y(d - 0.03), r],
                    [xs, y(d - 0.03), h - 0.05],
                    r,
                );
                b.cyl("frame", [xs, y(0.03), r], [xs, y(d - 0.03), r], r);
                b.cyl(
                    "frame",
                    [xs, y(0.03), seat - 0.02],
                    [xs, y(d - 0.03), seat - 0.02],
                    r,
                );
            }
            let (a, c) = span(0.0, d - 0.02);
            b.rbox("mat", [x0, a, seat - 0.02], [x1, c, seat + 0.01], 0.012);
            let (a, c) = span(d - 0.04, d - 0.01);
            b.rbox("mat", [x0 + 0.01, a, seat + 0.12], [x1 - 0.01, c, h], 0.012);
        }
        3 => {
            let leg = 0.035;
            for (lx, t) in [
                (x0 + 0.03, 0.03),
                (x1 - 0.03, 0.03),
                (x0 + 0.03, d - 0.04),
                (x1 - 0.03, d - 0.04),
            ] {
                let yy = y(t);
                b.cone(
                    "frame",
                    [lx, yy, 0.0],
                    [lx, yy, seat - 0.06],
                    leg * 0.35,
                    leg * 0.5,
                    8,
                );
            }
            let (a, c) = span(0.0, d);
            b.pillow(
                "mat",
                [x0, a, seat - 0.07],
                [x1, c, seat + 0.02],
                0.035,
                0.015,
            );
            let (a, c) = span(d - 0.07, d);
            b.pillow("mat", [x0, a, seat], [x1, c, h], 0.035, 0.015);
        }
        _ => {
            let leg = 0.038;
            for (lx, t) in [
                (x0, 0.0),
                (x1 - leg, 0.0),
                (x0, d - leg),
                (x1 - leg, d - leg),
            ] {
                let (a, c) = span(t, t + leg);
                let top = if t > d / 2.0 { h } else { seat - 0.02 };
                b.rbox("mat", [lx, a, 0.0], [lx + leg, c, top], 0.004);
            }
            let (a, c) = span(0.0, d);
            b.rbox(
                "mat",
                [x0 - 0.005, a, seat - 0.03],
                [x1 + 0.005, c, seat],
                0.008,
            );
            // Rails under the seat and slats of the back.
            let (a, c) = span(0.02, d - 0.02);
            b.rbox(
                "mat",
                [x0 + 0.005, a, seat - 0.09],
                [x0 + 0.025, c, seat - 0.03],
                0.0,
            );
            b.rbox(
                "mat",
                [x1 - 0.025, a, seat - 0.09],
                [x1 - 0.005, c, seat - 0.03],
                0.0,
            );
            let (a, c) = span(d - leg + 0.008, d - 0.008);
            for k in 0..3 {
                let z = seat + 0.18 + k as f64 * (h - seat - 0.24) / 2.0;
                b.rbox("mat", [x0 + leg, a, z], [x1 - leg, c, z + 0.05], 0.006);
            }
        }
    }
}

pub fn chair(b: &mut Builder, p: &Params) {
    chair_at(b, p, 0.0, 0.0, -1.0, p.style());
}

/// Dining tables: 1 four square legs and a frame; 2 slim metal legs; 3
/// two wooden trestles. With their chairs round them, as the plan.
pub fn table(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.6);
    let d = p.get("depth", 0.9);
    let h = p.get("height", 0.75);
    let style = p.style();
    let t = 0.035;
    let r = if style == 2 { 0.004 } else { 0.006 };
    b.rbox("mat", [-w / 2.0, 0.0, h - t], [w / 2.0, d, h], r);
    let x0 = -w / 2.0;
    match style {
        2 => {
            for (x, y) in [
                (x0 + 0.08, 0.08),
                (-x0 - 0.08, 0.08),
                (x0 + 0.08, d - 0.08),
                (-x0 - 0.08, d - 0.08),
            ] {
                b.cyl("frame", [x, y, 0.0], [x, y, h - t], 0.015);
            }
            b.rbox(
                "frame",
                [x0 + 0.08, 0.07, h - t - 0.03],
                [-x0 - 0.08, 0.09, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + 0.08, d - 0.09, h - t - 0.03],
                [-x0 - 0.08, d - 0.07, h - t],
                0.0,
            );
        }
        3 => {
            for x in [x0 + 0.18, -x0 - 0.22] {
                b.rbox("mat", [x, 0.06, 0.0], [x + 0.04, d - 0.06, 0.04], 0.005);
                b.rbox(
                    "mat",
                    [x, d / 2.0 - 0.05, 0.04],
                    [x + 0.04, d / 2.0 + 0.05, h - t - 0.04],
                    0.005,
                );
                b.rbox(
                    "mat",
                    [x, 0.1, h - t - 0.05],
                    [x + 0.04, d - 0.1, h - t],
                    0.005,
                );
            }
            b.rbox(
                "mat",
                [x0 + 0.22, d / 2.0 - 0.02, 0.18],
                [-x0 - 0.22, d / 2.0 + 0.02, 0.24],
                0.005,
            );
        }
        _ => {
            let l = 0.06;
            for (x, y) in [
                (x0 + 0.04, 0.04),
                (-x0 - 0.04 - l, 0.04),
                (x0 + 0.04, d - 0.04 - l),
                (-x0 - 0.04 - l, d - 0.04 - l),
            ] {
                b.rbox("mat", [x, y, 0.0], [x + l, y + l, h - t], 0.004);
            }
            b.rbox(
                "mat",
                [x0 + 0.06, 0.06, h - t - 0.08],
                [-x0 - 0.06, 0.08, h - t],
                0.0,
            );
            b.rbox(
                "mat",
                [x0 + 0.06, d - 0.08, h - t - 0.08],
                [-x0 - 0.06, d - 0.06, h - t],
                0.0,
            );
        }
    }
    // Chairs: half in front (round up), half behind, as the plan draws.
    let chairs = p.get("chairs", 0.0).round().max(0.0) as usize;
    let front = chairs.div_ceil(2);
    let back = chairs - front;
    let mut cp = Params::default();
    cp.set("width", 0.44);
    cp.set("depth", 0.46);
    cp.set("seat", 0.45);
    cp.set("height", 0.88);
    let chair_style = match style {
        2 => 2,
        3 => 1,
        _ => 1,
    };
    for i in 0..front {
        let x = x0 + (i as f64 + 0.5) * w / front as f64;
        chair_at(b, &cp, x, d - 0.12, 1.0, chair_style);
    }
    for i in 0..back {
        let x = x0 + (i as f64 + 0.5) * w / back as f64;
        chair_at(b, &cp, x, 0.12 - 0.46, -1.0, chair_style);
    }
}

/// Coffee tables: 1 wood with a shelf below; 2 a thick top on a plinth;
/// 3 glass on a metal frame.
pub fn coffee_table(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.1);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.4);
    let x0 = -w / 2.0;
    match p.style() {
        2 => {
            b.rbox("mat", [x0, 0.0, h - 0.08], [-x0, d, h], 0.02);
            b.rbox(
                "mat",
                [x0 + 0.12, 0.1, 0.0],
                [-x0 - 0.12, d - 0.1, h - 0.08],
                0.01,
            );
        }
        3 => {
            b.rbox("glass", [x0, 0.0, h - 0.012], [-x0, d, h], 0.003);
            let r = 0.009;
            for (x, y) in [
                (x0 + 0.05, 0.05),
                (-x0 - 0.05, 0.05),
                (x0 + 0.05, d - 0.05),
                (-x0 - 0.05, d - 0.05),
            ] {
                b.cyl("frame", [x, y, 0.0], [x, y, h - 0.012], r);
            }
            for y in [0.05, d - 0.05] {
                b.cyl(
                    "frame",
                    [x0 + 0.05, y, h - 0.02],
                    [-x0 - 0.05, y, h - 0.02],
                    r,
                );
            }
        }
        _ => {
            b.rbox("mat", [x0, 0.0, h - 0.03], [-x0, d, h], 0.006);
            b.rbox(
                "mat",
                [x0 + 0.05, 0.05, 0.1],
                [-x0 - 0.05, d - 0.05, 0.12],
                0.004,
            );
            let l = 0.045;
            for (x, y) in [
                (x0 + 0.03, 0.03),
                (-x0 - 0.03 - l, 0.03),
                (x0 + 0.03, d - 0.03 - l),
                (-x0 - 0.03 - l, d - 0.03 - l),
            ] {
                b.rbox("mat", [x, y, 0.0], [x + l, y + l, h - 0.03], 0.004);
            }
            // A couple of books on the shelf.
            b.rbox(
                "books",
                [x0 + 0.15, 0.15, 0.12],
                [x0 + 0.42, 0.36, 0.15],
                0.002,
            );
            b.rbox(
                "books",
                [x0 + 0.17, 0.17, 0.15],
                [x0 + 0.38, 0.33, 0.17],
                0.002,
            );
        }
    }
}

/// Beds: 1 upholstered headboard and base; 2 wooden frame on legs with a
/// panel headboard; 3 low platform. With mattress, duvet and pillows.
pub fn bed(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.5);
    let l = p.get("depth", 2.0);
    let top = p.get("height", 0.5);
    let head = p.get("head", 1.0);
    let x0 = -w / 2.0;
    let style = p.style();
    let mattress = 0.2;
    let base_top = (top - mattress).max(0.08);
    match style {
        2 => {
            b.rbox("mat", [x0, 0.05, base_top - 0.18], [-x0, l, base_top], 0.01);
            for (x, y) in [
                (x0 + 0.04, 0.08),
                (-x0 - 0.1, 0.08),
                (x0 + 0.04, l - 0.08),
                (-x0 - 0.1, l - 0.08),
            ] {
                b.rbox(
                    "mat",
                    [x, y - 0.03, 0.0],
                    [x + 0.06, y + 0.03, base_top - 0.18],
                    0.005,
                );
            }
            b.rbox("mat", [x0 - 0.02, 0.0, 0.0], [-x0 + 0.02, 0.05, head], 0.01);
        }
        3 => {
            b.rbox(
                "mat",
                [x0 - 0.1, 0.0, 0.0],
                [-x0 + 0.1, l + 0.1, base_top],
                0.02,
            );
            b.rbox(
                "mat",
                [x0 - 0.1, 0.0, base_top],
                [-x0 + 0.1, 0.05, head],
                0.01,
            );
        }
        _ => {
            b.rbox("cushion", [x0, 0.08, 0.06], [-x0, l, base_top], 0.03);
            b.rbox(
                "frame",
                [x0 + 0.05, 0.12, 0.0],
                [-x0 - 0.05, l - 0.05, 0.06],
                0.0,
            );
            b.rbox(
                "cushion",
                [x0 - 0.03, 0.0, 0.0],
                [-x0 + 0.03, 0.1, head],
                0.05,
            );
        }
    }
    // Mattress, the duvet over its lower part, and pillows.
    b.rbox(
        "linen",
        [x0 + 0.01, 0.06, base_top],
        [-x0 - 0.01, l - 0.01, top],
        0.06,
    );
    b.pillow(
        "duvet",
        [x0 - 0.01, l * 0.3, top - 0.12],
        [-x0 + 0.01, l + 0.01, top + 0.06],
        0.05,
        0.02,
    );
    let pillows = if w > 1.2 { 2 } else { 1 };
    let pw = (w - 0.1) / pillows as f64;
    for i in 0..pillows {
        let a = x0 + 0.05 + i as f64 * pw + 0.02;
        b.pillow(
            "linen",
            [a, 0.1, top - 0.01],
            [a + pw - 0.04, 0.48, top + 0.12],
            0.05,
            0.04,
        );
    }
}

/// Bedside tables: a drawer unit with a lamp on it.
pub fn nightstand(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.45);
    let d = p.get("depth", 0.4);
    let h = p.get("height", 0.5);
    let x0 = -w / 2.0;
    let foot = if p.style() == 2 { 0.12 } else { 0.0 };
    if foot > 0.0 {
        for (x, y) in [
            (x0 + 0.04, 0.04),
            (-x0 - 0.04, 0.04),
            (x0 + 0.04, d - 0.04),
            (-x0 - 0.04, d - 0.04),
        ] {
            b.cone("frame", [x, y, 0.0], [x, y, foot], 0.012, 0.016, 8);
        }
    }
    b.rbox("mat", [x0, 0.0, foot], [-x0, d - FRONT, h], 0.008);
    // Its drawers, with the handle chosen (knobs, unless said).
    let n = p.get("drawers", 2.0).round().clamp(1.0, 6.0) as usize;
    let kind = Handle::of(p, Handle::Knob);
    fronts::drawers(
        b,
        x0 + 0.01,
        -x0 - 0.01,
        foot + 0.02,
        h - 0.02,
        d - FRONT,
        n,
        kind,
    );
    lamp(b, x0 + w * 0.6, d * 0.45, h);
}

/// A table lamp standing at `z`.
fn lamp(b: &mut Builder, x: f64, y: f64, z: f64) {
    b.lathe(
        "frame",
        [x, y, z],
        &[
            (0.06, 0.0),
            (0.06, 0.015),
            (0.012, 0.03),
            (0.01, 0.3),
            (0.0, 0.3),
        ],
        1.0,
        1.0,
    );
    b.lathe(
        "shade",
        [x, y, z + 0.24],
        &[(0.11, 0.0), (0.075, 0.17), (0.0, 0.17)],
        1.0,
        1.0,
    );
}

/// Wardrobes: a carcass on a plinth, doors with their gaps and handles.
pub fn wardrobe(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.2);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 2.4);
    let doors = p.get("doors", 2.0).round().max(1.0) as usize;
    let rows = p.get("drawers", 0.0).round().clamp(0.0, 4.0) as usize;
    let kind = Handle::of(p, Handle::Bar);
    let x0 = -w / 2.0;
    b.rbox(
        "mat",
        [x0 + 0.02, 0.02, 0.0],
        [-x0 - 0.02, d - 0.03, 0.08],
        0.0,
    );
    b.rbox("mat", [x0, 0.0, 0.08], [-x0, d - 0.02, h], 0.004);
    let dw = w / doors as f64;
    // Drawers under the doors, a row in each door's width.
    let top_of_drawers = 0.085 + rows as f64 * 0.22;
    if rows > 0 {
        for i in 0..doors {
            let a = x0 + i as f64 * dw;
            fronts::drawers(b, a, a + dw, 0.085, top_of_drawers, d - 0.02, rows, kind);
        }
    }
    for i in 0..doors {
        let a = x0 + i as f64 * dw;
        // Hinged at the outside of each pair.
        let hinge_left = i % 2 == 0;
        fronts::door(
            b,
            a,
            a + dw,
            top_of_drawers,
            h - 0.002,
            d - 0.02,
            kind,
            hinge_left,
            1.05,
        );
    }
}

/// Chests of drawers: rows of drawers in one or two columns, on a plinth
/// (1) or on legs (2).
pub fn chest(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.0);
    let d = p.get("depth", 0.5);
    let h = p.get("height", 0.85);
    let rows = p.get("drawers", 4.0).round().clamp(1.0, 10.0) as usize;
    let cols = p.get("columns", 1.0).round().clamp(1.0, 4.0) as usize;
    let kind = Handle::of(p, Handle::Bar);
    let x0 = -w / 2.0;
    let foot = if p.style() == 2 { 0.15 } else { 0.07 };
    if p.style() == 2 {
        for (x, y) in [
            (x0 + 0.05, 0.05),
            (-x0 - 0.05, 0.05),
            (x0 + 0.05, d - 0.06),
            (-x0 - 0.05, d - 0.06),
        ] {
            b.cone("frame", [x, y, 0.0], [x, y, foot], 0.012, 0.018, 8);
        }
    } else {
        b.rbox(
            "mat",
            [x0 + 0.02, 0.02, 0.0],
            [-x0 - 0.02, d - 0.05, foot],
            0.0,
        );
    }
    b.rbox("mat", [x0, 0.0, foot], [-x0, d - FRONT, h], 0.006);
    let cw = (w - 0.02) / cols as f64;
    for c in 0..cols {
        let a = x0 + 0.01 + c as f64 * cw;
        fronts::drawers(b, a, a + cw, foot + 0.01, h - 0.025, d - FRONT, rows, kind);
    }
}

/// Desks: a top on legs (1, a drawer pedestal at one side, `pedestal`:
/// 1 none, 2 left, 3 right) or on metal frames (2).
pub fn desk(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.4);
    let d = p.get("depth", 0.7);
    let h = p.get("height", 0.75);
    let kind = Handle::of(p, Handle::Bar);
    let side = p.get("pedestal", 3.0).round() as i32;
    let n = p.get("drawers", 3.0).round().clamp(1.0, 6.0) as usize;
    let x0 = -w / 2.0;
    let t = 0.03;
    b.rbox("top", [x0, 0.0, h - t], [-x0, d, h], 0.004);
    let pw = 0.42_f64.min(w * 0.45);
    let ped = match side {
        2 => Some((x0 + 0.01, x0 + 0.01 + pw)),
        3 => Some((-x0 - 0.01 - pw, -x0 - 0.01)),
        _ => None,
    };
    if p.style() == 2 {
        // Two metal frames, like an upside-down U.
        for x in [x0 + 0.05, -x0 - 0.05] {
            b.rbox(
                "frame",
                [x - 0.02, 0.05, 0.0],
                [x + 0.02, 0.09, h - t],
                0.004,
            );
            b.rbox(
                "frame",
                [x - 0.02, d - 0.09, 0.0],
                [x + 0.02, d - 0.05, h - t],
                0.004,
            );
            b.rbox(
                "frame",
                [x - 0.02, 0.05, h - t - 0.04],
                [x + 0.02, d - 0.05, h - t],
                0.004,
            );
        }
    } else {
        for (x, y) in [
            (x0 + 0.04, 0.04),
            (-x0 - 0.04, 0.04),
            (x0 + 0.04, d - 0.04),
            (-x0 - 0.04, d - 0.04),
        ] {
            let inside = ped.is_some_and(|(a, b2)| x > a - 0.05 && x < b2 + 0.05);
            if !inside {
                b.rbox(
                    "mat",
                    [x - 0.025, y - 0.025, 0.0],
                    [x + 0.025, y + 0.025, h - t],
                    0.003,
                );
            }
        }
    }
    if let Some((a, b2)) = ped {
        b.rbox("mat", [a, 0.02, 0.0], [b2, d - 0.04 - FRONT, h - t], 0.004);
        fronts::drawers(b, a, b2, 0.02, h - t - 0.01, d - 0.04 - FRONT, n, kind);
    }
}

/// Bookcases: two sides, shelves, a back and books on each shelf.
pub fn bookcase(b: &mut Builder, p: &Params) {
    let w = p.get("width", 0.8);
    let d = p.get("depth", 0.3);
    let h = p.get("height", 1.8);
    let shelves = p.get("shelves", 4.0).round().max(0.0) as usize;
    let t = p.get("board", 0.02);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, 0.0], [x0 + t, d, h], 0.002);
    b.rbox("mat", [-x0 - t, 0.0, 0.0], [-x0, d, h], 0.002);
    b.rbox("mat", [x0 + t, 0.0, 0.0], [-x0 - t, 0.008, h], 0.0);
    let n = shelves + 2;
    let gap = (h - t) / (n - 1) as f64;
    let mut seed = 7u32;
    let mut rnd = || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        ((seed >> 16) & 0x7fff) as f64 / 32767.0
    };
    for i in 0..n {
        let z = i as f64 * gap;
        b.rbox("mat", [x0 + t, 0.008, z], [-x0 - t, d, z + t], 0.0);
        if i + 1 == n || i == 0 && shelves > 2 {
            continue;
        }
        // Books, leaning against each other, leaving a gap now and then.
        let mut x = x0 + t + 0.02;
        let end = -x0 - t - 0.04;
        while x < end {
            let bw = 0.02 + 0.03 * rnd();
            let bh = (gap - t) * (0.62 + 0.3 * rnd());
            if rnd() < 0.12 {
                x += 0.08;
                continue;
            }
            let bd = d * (0.7 + 0.2 * rnd());
            let role = if rnd() < 0.5 { "books" } else { "books2" };
            b.rbox(
                role,
                [x, 0.02, z + t],
                [(x + bw).min(end), 0.02 + bd, z + t + bh],
                0.002,
            );
            x += bw + 0.002;
        }
    }
}

/// TV units: a long low cabinet with a screen on its stand.
pub fn tv_unit(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.8);
    let d = p.get("depth", 0.42);
    let h = p.get("height", 0.45);
    let x0 = -w / 2.0;
    b.rbox("mat", [x0, 0.0, 0.06], [-x0, d, h], 0.006);
    for (x, y) in [
        (x0 + 0.06, 0.06),
        (-x0 - 0.06, 0.06),
        (x0 + 0.06, d - 0.06),
        (-x0 - 0.06, d - 0.06),
    ] {
        b.cone("frame", [x, y, 0.0], [x, y, 0.06], 0.012, 0.016, 8);
    }
    let count = (w / 0.6).round().max(1.0) as usize;
    let fw = w / count as f64;
    let kind = Handle::of(p, Handle::None);
    let as_drawers = p.flag("drawer_fronts", false);
    for i in 0..count {
        let a = x0 + i as f64 * fw;
        if as_drawers {
            fronts::drawer(b, a, a + fw, 0.08, h - 0.02, d - 0.008, kind);
        } else {
            fronts::door(
                b,
                a,
                a + fw,
                0.08,
                h - 0.02,
                d - 0.008,
                kind,
                i % 2 == 0,
                h / 2.0,
            );
        }
    }
    // The screen, 16:9, as wide as most of the unit (at most 1.6 m).
    let sw = (w * 0.75).min(1.6);
    let sh = sw * 9.0 / 16.0;
    b.rbox(
        "frame",
        [-0.15, d * 0.3, h],
        [0.15, d * 0.6, h + 0.015],
        0.005,
    );
    b.rbox(
        "frame",
        [-0.03, d * 0.42, h],
        [0.03, d * 0.48, h + 0.08],
        0.0,
    );
    b.rbox(
        "screen",
        [-sw / 2.0, d * 0.4, h + 0.06],
        [sw / 2.0, d * 0.4 + 0.03, h + 0.06 + sh],
        0.004,
    );
}
