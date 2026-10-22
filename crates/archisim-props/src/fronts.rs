//! Fronts of furniture (bloque 5 de la revisión del 22/10/2026): doors and
//! drawers with the handle chosen — a bar, a knob, a finger groove, a
//! D-shaped pull or none (push to open). Axes as the furniture's: the back
//! on the wall at `y = 0`, the front looking to `+y`.

use crate::build::Builder;
use crate::Params;

/// The handles a front can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Bar,
    Knob,
    Groove,
    Pull,
    None,
}

/// Their names, as the `handle` choice of the families lists them (its
/// value is the position here, from 1).
pub const HANDLES: &[&str] = &["Barra", "Pomo", "Uñero", "Asa", "Sin tirador"];

impl Handle {
    /// The handle asked for (`handle`: 1 bar … 5 none), or `default`.
    pub fn of(p: &Params, default: Handle) -> Handle {
        match p.get("handle", 0.0).round() as i32 {
            1 => Handle::Bar,
            2 => Handle::Knob,
            3 => Handle::Groove,
            4 => Handle::Pull,
            5 => Handle::None,
            _ => default,
        }
    }
}

/// How far a front stands out of its carcass.
pub const FRONT: f64 = 0.018;

/// A handle centred at `(x, z)` on a front whose face is at `y`, lying
/// along the front (`horizontal`) or up it, `len` long.
pub fn handle(b: &mut Builder, kind: Handle, x: f64, y: f64, z: f64, horizontal: bool, len: f64) {
    let along = |t: f64| {
        if horizontal {
            [x + t, z]
        } else {
            [x, z + t]
        }
    };
    match kind {
        Handle::Bar => {
            let [ax, az] = along(-len / 2.0);
            let [bx, bz] = along(len / 2.0);
            b.cyl("metal", [ax, y + 0.03, az], [bx, y + 0.03, bz], 0.006);
            for [sx, sz] in [along(-len / 2.0 + 0.012), along(len / 2.0 - 0.012)] {
                b.cyl("metal", [sx, y, sz], [sx, y + 0.03, sz], 0.0045);
            }
        }
        Handle::Knob => {
            b.cyl("metal", [x, y, z], [x, y + 0.018, z], 0.006);
            b.ellipsoid("metal", [x, y + 0.026, z], [0.014, 0.01, 0.014]);
        }
        Handle::Pull => {
            // A D: two posts and a thicker grip, short.
            let l = len.min(0.16);
            let [ax, az] = along(-l / 2.0);
            let [bx, bz] = along(l / 2.0);
            b.cyl("metal", [ax, y, az], [ax, y + 0.035, az], 0.007);
            b.cyl("metal", [bx, y, bz], [bx, y + 0.035, bz], 0.007);
            b.cyl("metal", [ax, y + 0.035, az], [bx, y + 0.035, bz], 0.009);
        }
        Handle::Groove => {
            // A dark slot along the edge of the front.
            let (lo, hi) = if horizontal {
                (
                    [x - len / 2.0, y - 0.001, z - 0.007],
                    [x + len / 2.0, y + 0.002, z + 0.007],
                )
            } else {
                (
                    [x - 0.007, y - 0.001, z - len / 2.0],
                    [x + 0.007, y + 0.002, z + len / 2.0],
                )
            };
            b.rbox("dark", lo, hi, 0.0);
        }
        Handle::None => {}
    }
}

/// A drawer front from `x0` to `x1` and `z0` to `z1`, on a carcass whose
/// face is at `y`; its handle near the top, across.
#[allow(clippy::too_many_arguments)]
pub fn drawer(b: &mut Builder, x0: f64, x1: f64, z0: f64, z1: f64, y: f64, kind: Handle) {
    b.rbox(
        "mat",
        [x0 + 0.002, y - 0.002, z0 + 0.002],
        [x1 - 0.002, y + FRONT, z1 - 0.002],
        0.002,
    );
    let len = ((x1 - x0) * 0.4).clamp(0.08, 0.36);
    let hz = match kind {
        Handle::Groove => z1 - 0.012,
        _ => z1 - ((z1 - z0) * 0.35).min(0.05),
    };
    let len = if kind == Handle::Groove {
        (x1 - x0) - 0.03
    } else {
        len
    };
    handle(b, kind, (x0 + x1) / 2.0, y + FRONT, hz, true, len);
}

/// A door from `x0` to `x1` and `z0` to `z1`, hinged at its left or its
/// right; its handle near the edge that opens, upright, at `hz`.
#[allow(clippy::too_many_arguments)]
pub fn door(
    b: &mut Builder,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    kind: Handle,
    hinge_left: bool,
    hz: f64,
) {
    b.rbox(
        "mat",
        [x0 + 0.002, y - 0.002, z0 + 0.002],
        [x1 - 0.002, y + FRONT, z1 - 0.002],
        0.002,
    );
    let edge = if hinge_left { x1 - 0.045 } else { x0 + 0.045 };
    let len = ((z1 - z0) * 0.18).clamp(0.1, 0.32);
    let (hz, len) = match kind {
        Handle::Groove => (z1 - len / 2.0 - 0.01, len),
        _ => (hz.clamp(z0 + len / 2.0 + 0.02, z1 - len / 2.0 - 0.02), len),
    };
    let x = if kind == Handle::Groove {
        if hinge_left {
            x1 - 0.012
        } else {
            x0 + 0.012
        }
    } else {
        edge
    };
    handle(b, kind, x, y + FRONT, hz, false, len);
}

/// A column of `n` drawers of equal height from `z0` to `z1`.
#[allow(clippy::too_many_arguments)]
pub fn drawers(
    b: &mut Builder,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    y: f64,
    n: usize,
    kind: Handle,
) {
    let n = n.max(1);
    let h = (z1 - z0) / n as f64;
    for i in 0..n {
        let z = z0 + i as f64 * h;
        drawer(b, x0, x1, z, z + h, y, kind);
    }
}
