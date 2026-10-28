//! The study of the eight collections (paquete 6, tanda 6b): the desk
//! and the chair to work at it, each as the collection makes it. The
//! bookcases of the collections are those of the living room
//! (`storage::bookcase`).
//!
//! Desks: `width`, `depth`, `height`, `pedestal` (1 none, 2 left, 3 right)
//! and `drawers`; the back at `y` 0 against the wall, the front at
//! `depth` where one sits. Roles: `top`, `frame` (wood or steel of the
//! frame), `front` (drawer fronts), `metal` (pulls, shoes), `accent`
//! (a leather writing surface) and `dark`.
//!
//! Chairs: `width` and `seat`; centred on `(0, width / 2)`, facing `+y`.
//! Roles: `frame`, `cushion`, `metal`, `dark`, `accent`, `weave`, `mesh`.

use super::shape::{bar, turned};
use super::sofas::{compass_leg, upholstered};
use super::soft::tube;
use super::storage::{drawers, front, pull, tapered_leg, Panel, Pull, T};
use crate::build::{Builder, P3};
use crate::Params;
use std::f64::consts::{FRAC_PI_4, PI, SQRT_2, TAU};

/// The collection when the model chosen is one of those listed after the
/// family's `first` own models.
fn look(p: &Params, first: i32) -> Option<usize> {
    let s = p.style() - first;
    (1..=8).contains(&s).then(|| (s - 1) as usize)
}

/// A turned post from the floor up to `top` at `(x, y)`: `r` thick, with
/// a ring and a vase low down (a Mediterranean table's leg).
fn turned_post(b: &mut Builder, role: &'static str, x: f64, y: f64, top: f64, r: f64) {
    turned(
        b,
        role,
        [x, y, 0.0],
        &[
            (0.0, 0.0),
            (r * 0.8, 0.0),
            (r * 0.7, 0.06),
            (r * 1.15, 0.13),
            (r * 0.8, 0.2),
            (r * 1.1, 0.24),
            (r * 0.85, 0.3),
            (r, top - 0.14),
            (r * 1.2, top - 0.12),
            (r * 1.2, top),
            (0.0, top),
        ],
        16,
        0.0,
    );
}

/// A square leg tapering down to a brass shoe, fluted (a Louis XVI desk):
/// turned with four sides.
fn square_tapered(b: &mut Builder, x: f64, y: f64, top: f64, s: f64) {
    let r = s * SQRT_2 / 2.0;
    turned(
        b,
        "frame",
        [x, y, 0.05],
        &[
            (0.0, 0.0),
            (r * 0.6, 0.0),
            (r, top - 0.05),
            (0.0, top - 0.05),
        ],
        4,
        FRAC_PI_4,
    );
    turned(
        b,
        "metal",
        [x, y, 0.0],
        &[(0.0, 0.0), (r * 0.62, 0.0), (r * 0.62, 0.05), (0.0, 0.05)],
        4,
        FRAC_PI_4,
    );
}

/// Where the drawer pedestal goes, if any: from `xa` to `xb`.
fn pedestal(p: &Params, w: f64) -> Option<(f64, f64)> {
    let x0 = -w / 2.0;
    let pw = 0.42_f64.min(w * 0.4);
    match p.get("pedestal", 3.0).round() as i32 {
        2 => Some((x0 + 0.05, x0 + 0.05 + pw)),
        3 => Some((-x0 - 0.05 - pw, -x0 - 0.05)),
        _ => None,
    }
}

/// Desks of the collections (after the family's 2 own models).
pub fn desk(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 1.4).clamp(0.8, 3.0);
    let d = p.get("depth", 0.7).clamp(0.5, 1.0);
    let h = p.get("height", 0.75).clamp(0.65, 1.1);
    let n = p.get("drawers", 3.0).round().clamp(1.0, 6.0) as usize;
    let ped = pedestal(p, w);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    match ix {
        // Nordic: an oak top on splayed tapered legs, a drawer in its
        // apron with a finger slot, and a small hanging pedestal.
        0 => {
            let t = 0.026;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            let ap = h - t - 0.07;
            for (lo, hi) in [
                ([x0 + 0.05, 0.05, ap], [x1 - 0.05, 0.07, h - t]),
                ([x0 + 0.05, 0.05, ap], [x0 + 0.07, d - 0.05, h - t]),
                ([x1 - 0.07, 0.05, ap], [x1 - 0.05, d - 0.05, h - t]),
            ] {
                b.rbox("frame", lo, hi, 0.0);
            }
            let (da, db) = (-w * 0.2, w * 0.2);
            b.rbox(
                "frame",
                [x0 + 0.05, d - 0.07, ap],
                [da, d - 0.05, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [db, d - 0.07, ap],
                [x1 - 0.05, d - 0.05, h - t],
                0.0,
            );
            front(b, Panel::Flat, "front", da, db, ap, h - t - 0.004, d - 0.07);
            pull(b, Pull::Slot, 0.0, h - t - 0.005, d - 0.07 + T);
            for (x, y, sx, sy) in [
                (x0 + 0.07, 0.07, -0.5, -0.5),
                (x1 - 0.07, 0.07, 0.5, -0.5),
                (x1 - 0.07, d - 0.07, 0.5, 0.5),
                (x0 + 0.07, d - 0.07, -0.5, 0.5),
            ] {
                tapered_leg(b, "frame", [x, y], ap + 0.01, [sx, sy], 0.03, 0.017);
            }
            if let Some((xa, xb)) = ped {
                let z0 = ap - 0.26;
                b.rbox("frame", [xa, 0.08, z0], [xb, d - 0.07, ap], 0.0);
                drawers(
                    b,
                    Panel::Flat,
                    "front",
                    xa,
                    xb,
                    z0,
                    ap - 0.003,
                    d - 0.07,
                    2,
                    Pull::Slot,
                );
            }
        }
        // Mediterranean: a painted desk on turned legs, two drawers in its
        // deep apron with iron rings, and a pedestal of drawers.
        1 => {
            let t = 0.03;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            let ap = h - t - 0.12;
            for (x, y) in [
                (x0 + 0.04, 0.04),
                (x1 - 0.04, 0.04),
                (x1 - 0.04, d - 0.04),
                (x0 + 0.04, d - 0.04),
            ] {
                turned_post(b, "frame", x, y, h - t, 0.024);
            }
            b.rbox(
                "frame",
                [x0 + 0.04, 0.03, ap],
                [x1 - 0.04, 0.05, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [x0 + 0.03, 0.04, ap],
                [x0 + 0.05, d - 0.04, h - t],
                0.0,
            );
            b.rbox(
                "frame",
                [x1 - 0.05, 0.04, ap],
                [x1 - 0.03, d - 0.04, h - t],
                0.0,
            );
            let k = 2;
            for i in 0..k {
                let a = x0 + 0.07 + (w - 0.14) * i as f64 / k as f64;
                let e = x0 + 0.07 + (w - 0.14) * (i + 1) as f64 / k as f64;
                front(
                    b,
                    Panel::Flat,
                    "front",
                    a + 0.003,
                    e - 0.003,
                    ap + 0.01,
                    h - t - 0.01,
                    d - 0.05,
                );
                pull(b, Pull::Ring, (a + e) / 2.0, ap + 0.06, d - 0.05 + T);
            }
            b.rbox(
                "frame",
                [x0 + 0.04, d - 0.05, ap],
                [x1 - 0.04, d - 0.04, ap + 0.01],
                0.0,
            );
            if let Some((xa, xb)) = ped {
                b.rbox("frame", [xa, 0.06, 0.08], [xb, d - 0.05, ap], 0.0);
                drawers(
                    b,
                    Panel::Flat,
                    "front",
                    xa,
                    xb,
                    0.08,
                    ap - 0.005,
                    d - 0.05,
                    n.min(3),
                    Pull::Ring,
                );
            }
        }
        // Fifties: a teak top on four compass legs, a floating block of
        // drawers under one end with brass knobs.
        2 => {
            let t = 0.025;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            let rail = h - t - 0.05;
            b.rbox(
                "frame",
                [x0 + 0.06, 0.06, rail],
                [x1 - 0.06, d - 0.06, h - t],
                0.0,
            );
            for (x, y, sx, sy) in [
                (x0 + 0.1, 0.1, -0.6, -0.5),
                (x1 - 0.1, 0.1, 0.6, -0.5),
                (x1 - 0.1, d - 0.1, 0.6, 0.5),
                (x0 + 0.1, d - 0.1, -0.6, 0.5),
            ] {
                compass_leg(b, [x, y], rail + 0.01, [sx, sy], 0.07);
            }
            if let Some((xa, xb)) = ped {
                let z0 = rail - 0.3;
                b.rbox("frame", [xa, 0.08, z0], [xb, d - 0.05, rail], 0.0);
                drawers(
                    b,
                    Panel::Flat,
                    "front",
                    xa + 0.01,
                    xb - 0.01,
                    z0 + 0.01,
                    rail - 0.01,
                    d - 0.05,
                    n.min(3),
                    Pull::Brass,
                );
            }
        }
        // Japandi: a slab of smoked oak on two slab ends, a shelf low
        // between them, and a small open box under the top.
        3 => {
            let t = 0.04;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            for x in [x0 + 0.06, x1 - 0.1] {
                b.rbox("frame", [x, 0.04, 0.0], [x + 0.04, d - 0.04, h - t], 0.0);
            }
            b.rbox(
                "frame",
                [x0 + 0.1, 0.06, 0.1],
                [x1 - 0.1, d - 0.06, 0.125],
                0.0,
            );
            if let Some((xa, xb)) = ped {
                let (xa, xb) = (xa.max(x0 + 0.1), xb.min(x1 - 0.1));
                let z0 = h - t - 0.13;
                b.rbox("frame", [xa, 0.05, z0], [xb, d - 0.06, z0 + 0.015], 0.0);
                b.rbox("frame", [xa, 0.05, z0], [xb, 0.065, h - t], 0.0);
                for x in [xa, xb - 0.015] {
                    b.rbox("frame", [x, 0.05, z0], [x + 0.015, d - 0.06, h - t], 0.0);
                }
            }
        }
        // Contemporary: a thin top on two sled frames of black steel, a
        // rail for cables and a drawer unit on castors under it.
        4 => {
            let t = 0.02;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            let s = 0.03;
            for x in [x0 + 0.05, x1 - 0.05 - s] {
                b.rbox("dark", [x, 0.04, 0.0], [x + s, d - 0.04, 0.015], 0.0);
                b.rbox(
                    "dark",
                    [x, 0.04, h - t - 0.03],
                    [x + s, d - 0.04, h - t],
                    0.0,
                );
                for y in [0.04, d - 0.04 - s] {
                    b.rbox("dark", [x, y, 0.015], [x + s, y + s, h - t - 0.03], 0.0);
                }
            }
            b.rbox(
                "dark",
                [x0 + 0.08, 0.04, h - t - 0.12],
                [x1 - 0.08, 0.06, h - t - 0.03],
                0.0,
            );
            if let Some((xa, xb)) = ped {
                let (xa, xb) = (xa.max(x0 + 0.12), xb.min(x1 - 0.12));
                let top = (h - t - 0.1).min(0.6);
                b.rbox("front", [xa, d * 0.3, 0.05], [xb, d - 0.06, top], 0.0);
                drawers(
                    b,
                    Panel::Flat,
                    "front",
                    xa,
                    xb,
                    0.05,
                    top,
                    d - 0.06,
                    n.min(3),
                    Pull::Slot,
                );
                for (x, y) in [
                    (xa + 0.04, d * 0.3 + 0.04),
                    (xb - 0.04, d * 0.3 + 0.04),
                    (xb - 0.04, d - 0.1),
                    (xa + 0.04, d - 0.1),
                ] {
                    b.ellipsoid("dark", [x, y, 0.025], [0.022, 0.012, 0.025]);
                }
            }
        }
        // Rustic: a thick plank on two trestles, a stretcher between them.
        5 => {
            let t = 0.055;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            for x in [x0 + 0.15, x1 - 0.2] {
                for (ya, yb) in [(0.06, d * 0.5 - 0.04), (d - 0.06, d * 0.5 + 0.04)] {
                    bar(
                        b,
                        "frame",
                        [x + 0.025, ya, 0.03],
                        [x + 0.025, yb, h - t - 0.07],
                        0.05,
                        [1.0, 0.0, 0.0],
                    );
                }
                b.rbox(
                    "frame",
                    [x, 0.08, h - t - 0.08],
                    [x + 0.05, d - 0.08, h - t],
                    0.0,
                );
                b.rbox(
                    "frame",
                    [x - 0.01, 0.03, 0.0],
                    [x + 0.06, d - 0.03, 0.05],
                    0.0,
                );
            }
            b.rbox(
                "frame",
                [x0 + 0.15, d * 0.5 - 0.03, 0.22],
                [x1 - 0.15, d * 0.5 + 0.03, 0.29],
                0.0,
            );
        }
        // Industrial: a wooden top on a frame of black steel, and a
        // cabinet of steel drawers with bar pulls.
        6 => {
            let t = 0.035;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h], 0.0);
            let s = 0.04;
            for (x, y) in [
                (x0 + 0.02, 0.02),
                (x1 - 0.02 - s, 0.02),
                (x1 - 0.02 - s, d - 0.02 - s),
                (x0 + 0.02, d - 0.02 - s),
            ] {
                b.rbox("dark", [x, y, 0.0], [x + s, y + s, h - t], 0.0);
            }
            for (lo, hi) in [
                (
                    [x0 + 0.02, 0.02, h - t - 0.04],
                    [x1 - 0.02, 0.02 + s, h - t],
                ),
                (
                    [x0 + 0.02, d - 0.02 - s, h - t - 0.04],
                    [x1 - 0.02, d - 0.02, h - t],
                ),
                ([x0 + 0.02, 0.02, 0.12], [x0 + 0.02 + s, d - 0.02, 0.12 + s]),
                ([x1 - 0.02 - s, 0.02, 0.12], [x1 - 0.02, d - 0.02, 0.12 + s]),
            ] {
                b.rbox("dark", lo, hi, 0.0);
            }
            if let Some((xa, xb)) = ped {
                let (xa, xb) = (xa.max(x0 + 0.08), xb.min(x1 - 0.08));
                let top = h - t - 0.05;
                b.rbox("metal", [xa, 0.06, 0.02], [xb, d - 0.06, top], 0.0);
                for i in 0..n.min(4) {
                    let a = 0.02 + (top - 0.02) * i as f64 / n.min(4) as f64;
                    let e = 0.02 + (top - 0.02) * (i + 1) as f64 / n.min(4) as f64;
                    front(
                        b,
                        Panel::Flat,
                        "metal",
                        xa + 0.005,
                        xb - 0.005,
                        a + 0.003,
                        e - 0.003,
                        d - 0.06,
                    );
                    pull(
                        b,
                        Pull::Bar(0.12),
                        (xa + xb) / 2.0,
                        (a + e) / 2.0 + 0.03,
                        d - 0.06 + T,
                    );
                }
            }
        }
        // Classic: a writing desk with a leather top inside a moulded edge,
        // fluted square legs on brass shoes, three drawers in its apron.
        _ => {
            let t = 0.035;
            b.rbox("top", [x0, 0.0, h - t], [x1, d, h - 0.004], 0.0);
            b.rbox(
                "accent",
                [x0 + 0.05, 0.05, h - 0.004],
                [x1 - 0.05, d - 0.05, h],
                0.0,
            );
            tube(
                b,
                "metal",
                &[
                    [x0, -0.004, h - t + 0.006],
                    [x1, -0.004, h - t + 0.006],
                    [x1, d + 0.004, h - t + 0.006],
                    [x0, d + 0.004, h - t + 0.006],
                ],
                true,
                0.006,
                6,
            );
            let ap = h - t - 0.12;
            b.rbox(
                "frame",
                [x0 + 0.04, 0.04, ap],
                [x1 - 0.04, d - 0.06, h - t],
                0.0,
            );
            let k = 3;
            for i in 0..k {
                let a = x0 + 0.08 + (w - 0.16) * i as f64 / k as f64;
                let e = x0 + 0.08 + (w - 0.16) * (i + 1) as f64 / k as f64;
                front(
                    b,
                    Panel::Moulded,
                    "front",
                    a + 0.004,
                    e - 0.004,
                    ap + 0.012,
                    h - t - 0.012,
                    d - 0.06,
                );
                pull(b, Pull::Brass, (a + e) / 2.0, ap + 0.06, d - 0.06 + T);
            }
            for (x, y) in [
                (x0 + 0.07, 0.07),
                (x1 - 0.07, 0.07),
                (x1 - 0.07, d - 0.08),
                (x0 + 0.07, d - 0.08),
            ] {
                square_tapered(b, x, y, ap + 0.005, 0.05);
                b.ellipsoid("metal", [x, y, ap - 0.02], [0.03, 0.03, 0.02]);
            }
        }
    }
    true
}

/// A curved band of a shell: the arc of radius `r` round `(cx, cy)` from
/// angle `a0` to `a1`, from `z0` to `z1`, `t` thick (outwards), in `n`
/// pieces; a closed solid.
#[allow(clippy::too_many_arguments)]
pub(super) fn band(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    r: f64,
    a0: f64,
    a1: f64,
    z0: f64,
    z1: f64,
    t: f64,
    n: usize,
) {
    let at = |b: &mut Builder, a: f64, rr: f64, z: f64| {
        b.point([c[0] + rr * a.cos(), c[1] + rr * a.sin(), z])
    };
    let mut cols = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let a = a0 + (a1 - a0) * k as f64 / n as f64;
        cols.push([
            at(b, a, r, z0),
            at(b, a, r + t, z0),
            at(b, a, r + t, z1),
            at(b, a, r, z1),
        ]);
    }
    for w in cols.windows(2) {
        let [i0, o0, o1, i1] = w[0];
        let [j0, p0, p1, j1] = w[1];
        // Outside, inside, top, bottom (a0 < a1: counter-clockwise).
        b.quad(role, o0, p0, p1, o1);
        b.quad(role, j0, i0, i1, j1);
        b.quad(role, i1, o1, p1, j1);
        b.quad(role, j0, p0, o0, i0);
    }
    let [i0, o0, o1, i1] = cols[0];
    b.quad(role, i0, o0, o1, i1);
    let [j0, p0, p1, j1] = cols[n];
    b.quad(role, p0, j0, j1, p1);
}

/// A star base of `legs` arms from the column at `c`, `reach` long, with
/// glides or castors at their ends.
fn star_base(
    b: &mut Builder,
    role: &'static str,
    c: [f64; 2],
    reach: f64,
    legs: usize,
    castors: bool,
) {
    let z = if castors { 0.075 } else { 0.03 };
    for k in 0..legs {
        let a = PI / 2.0 + TAU * k as f64 / legs as f64;
        let end = [c[0] + reach * a.cos(), c[1] + reach * a.sin()];
        bar(
            b,
            role,
            [c[0], c[1], z + 0.025],
            [end[0], end[1], z],
            0.032,
            [0.0, 0.0, 1.0],
        );
        if castors {
            b.ellipsoid("dark", [end[0], end[1], 0.03], [0.03, 0.022, 0.03]);
        } else {
            b.cone(
                "dark",
                [end[0], end[1], 0.0],
                [end[0], end[1], z - 0.016],
                0.018,
                0.014,
                10,
            );
        }
    }
}

/// Desk chairs of the collections (after the family's 2 own models).
pub fn office_chair(b: &mut Builder, p: &Params) -> bool {
    let Some(ix) = look(p, 2) else {
        return false;
    };
    let w = p.get("width", 0.62).clamp(0.45, 0.8);
    let seat = p.get("seat", 0.47).clamp(0.38, 0.56);
    let c = [0.0, w / 2.0];
    match ix {
        // The collections' dining chairs serve at their desks:
        // Mediterranean rush, japandi, rustic and classic.
        1 | 3 | 5 | 7 => {
            let mut q = Params::default();
            q.set("style", (ix + 1) as f64);
            q.set("width", (w * 0.75).clamp(0.42, 0.55));
            q.set("seat", seat.clamp(0.42, 0.48));
            q.set("depth", 0.52);
            let start = b.mark();
            super::chairs::chair(b, &q);
            // Its back at the back of the place the desk chair takes.
            let dy = c[1] - 0.26;
            b.transform_from(start, |v| [v[0], v[1] + dy, v[2]]);
        }
        // Nordic: an oak shell, its seat padded, on a four-star swivel
        // base of black steel with glides.
        0 => {
            star_base(b, "metal", c, w * 0.42, 4, false);
            b.cone(
                "metal",
                [c[0], c[1], 0.04],
                [c[0], c[1], seat - 0.05],
                0.022,
                0.022,
                14,
            );
            b.rbox(
                "metal",
                [c[0] - 0.12, c[1] - 0.1, seat - 0.06],
                [c[0] + 0.12, c[1] + 0.1, seat - 0.045],
                0.0,
            );
            let sw = w * 0.38;
            b.rbox(
                "frame",
                [c[0] - sw, c[1] - sw * 0.85, seat - 0.045],
                [c[0] + sw, c[1] + sw * 0.95, seat - 0.03],
                0.0,
            );
            upholstered(
                [c[0] - sw + 0.02, c[1] - sw * 0.85 + 0.03, seat - 0.032],
                [c[0] + sw - 0.02, c[1] + sw * 0.95 - 0.01, seat + 0.02],
                0.02,
                21,
            )
            .add(b, "cushion");
            band(
                b,
                "frame",
                [c[0], c[1] + 0.05],
                sw * 1.05,
                -PI * 0.92,
                -PI * 0.08,
                seat + 0.07,
                seat + 0.32,
                0.014,
                14,
            );
            for x in [c[0] - sw * 0.8, c[0] + sw * 0.8] {
                bar(
                    b,
                    "frame",
                    [x, c[1] - sw * 0.6, seat - 0.035],
                    [x, c[1] - sw * 0.75, seat + 0.1],
                    0.025,
                    [1.0, 0.0, 0.0],
                );
            }
        }
        // Fifties: a bucket of colour wrapping round, its cushion, on a
        // chromed four-star swivel base.
        2 => {
            star_base(b, "metal", c, w * 0.42, 4, false);
            b.cone(
                "metal",
                [c[0], c[1], 0.04],
                [c[0], c[1], seat - 0.06],
                0.02,
                0.02,
                14,
            );
            let r = w * 0.4;
            turned(
                b,
                "accent",
                [c[0], c[1], seat - 0.07],
                &[
                    (0.0, 0.0),
                    (r * 0.5, 0.0),
                    (r, 0.04),
                    (r, 0.06),
                    (0.0, 0.06),
                ],
                28,
                0.0,
            );
            upholstered(
                [c[0] - r * 0.62, c[1] - r * 0.62, seat - 0.015],
                [c[0] + r * 0.62, c[1] + r * 0.62, seat + 0.03],
                0.05,
                22,
            )
            .add(b, "cushion");
            band(
                b,
                "accent",
                c,
                r - 0.012,
                -PI * 1.05,
                PI * 0.05,
                seat - 0.02,
                seat + 0.27,
                0.012,
                20,
            );
        }
        // Contemporary: a task chair, its mesh back and padded seat, on
        // five castors (the library's own task chair).
        4 => {
            let mut q = p.clone();
            q.set("style", 2.0);
            crate::extras::office_chair(b, &q);
        }
        // Industrial: a stool chair of steel: four splayed legs with a
        // ring to rest the feet, a round seat of wood on its screw, a
        // curved steel back.
        _ => {
            let top = seat - 0.06;
            for k in 0..4 {
                let a = FRAC_PI_4 + PI / 2.0 * k as f64;
                let (ca, sa) = (a.cos(), a.sin());
                bar(
                    b,
                    "dark",
                    [c[0] + ca * w * 0.38, c[1] + sa * w * 0.38, 0.012],
                    [c[0] + ca * 0.08, c[1] + sa * 0.08, top],
                    0.025,
                    [0.0, 0.0, 1.0],
                );
                let foot = [c[0] + ca * w * 0.38, c[1] + sa * w * 0.38];
                b.cone(
                    "dark",
                    [foot[0], foot[1], 0.0],
                    [foot[0], foot[1], 0.03],
                    0.018,
                    0.016,
                    10,
                );
            }
            let rr = w * 0.29;
            let ring: Vec<P3> = (0..32)
                .map(|k| {
                    let a = TAU * k as f64 / 32.0;
                    [c[0] + rr * a.cos(), c[1] + rr * a.sin(), 0.2]
                })
                .collect();
            tube(b, "dark", &ring, true, 0.009, 6);
            b.cone(
                "metal",
                [c[0], c[1], top - 0.02],
                [c[0], c[1], seat - 0.035],
                0.018,
                0.018,
                12,
            );
            turned(
                b,
                "frame",
                [c[0], c[1], seat - 0.035],
                &[
                    (0.0, 0.0),
                    (w * 0.3, 0.0),
                    (w * 0.31, 0.02),
                    (w * 0.3, 0.035),
                    (0.0, 0.035),
                ],
                28,
                0.0,
            );
            bar(
                b,
                "dark",
                [c[0], c[1] - w * 0.2, seat - 0.035],
                [c[0], c[1] - w * 0.3, seat + 0.2],
                0.02,
                [1.0, 0.0, 0.0],
            );
            band(
                b,
                "dark",
                [c[0], c[1] + 0.02],
                w * 0.3,
                -PI * 0.8,
                -PI * 0.2,
                seat + 0.15,
                seat + 0.27,
                0.004,
                10,
            );
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::shape::signed_volume;

    #[test]
    fn the_study_of_the_eight_collections() {
        type Make = fn(&mut Builder, &Params) -> bool;
        let pieces: [(&str, Make); 2] = [("escritorio", desk), ("silla", office_chair)];
        for c in 0..8 {
            for (name, make) in pieces {
                let mut p = Params::default();
                p.set("style", (3 + c) as f64);
                let mut b = Builder::default();
                assert!(make(&mut b, &p), "{name}");
                let tris = b.triangle_count();
                assert!((50..40_000).contains(&tris), "{c} {name}: {tris}");
                for (role, t) in &b.parts {
                    let v = signed_volume(&b.positions, t);
                    assert!(v > -1e-7, "{c} {name} {role}: {v}");
                }
                let low = b.positions.iter().map(|q| q[2]).fold(f32::MAX, f32::min);
                assert!(low > -0.001, "{c} {name}: {low}");
            }
        }
    }
}
