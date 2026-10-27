//! Kitchen: base units with worktop, sink and hob; wall units; fridge and
//! washing machine. Axes as the furniture's: the back on the wall.

use crate::build::Builder;
use crate::fronts::{self, Handle, Style};
use crate::Params;

/// Doors of a run of units: as many as fit, about 60 cm each, with the
/// handle chosen.
#[allow(clippy::too_many_arguments)]
fn fronts(
    b: &mut Builder,
    x0: f64,
    w: f64,
    y: f64,
    z0: f64,
    z1: f64,
    handles_low: bool,
    kind: Handle,
) {
    let n = (w / 0.6).round().max(1.0) as usize;
    let fw = w / n as f64;
    for i in 0..n {
        let a = x0 + i as f64 * fw;
        let hz = if handles_low { z1 - 0.08 } else { z0 + 0.08 };
        fronts::door(b, a, a + fw, z0, z1, y, kind, i % 2 == 0, hz);
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
    // A drawer unit under the hob (or at the right end), with the doors
    // beside it.
    let kind = Handle::of(p, Handle::Bar);
    let rows = p.get("drawers", 0.0).round().clamp(0.0, 6.0) as usize;
    let unit = if rows > 0 { 0.6_f64.min(w * 0.5) } else { 0.0 };
    if unit > 0.0 {
        fronts::drawers(
            b,
            -x0 - unit,
            -x0,
            0.105,
            h - top - 0.005,
            d - 0.04,
            rows,
            kind,
        );
    }
    fronts(
        b,
        x0,
        w - unit,
        d - 0.04,
        0.105,
        h - top - 0.005,
        true,
        kind,
    );
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

/// What a base unit of a modular kitchen holds, as the `unit` choice of
/// its family lists them.
pub const UNITS: &[&str] = &[
    "Cajonera",
    "Puertas",
    "Fregadero",
    "Horno",
    "Lavavajillas",
    "Placa",
    "Botellero",
    "Esquina",
];

/// A base unit of a modular kitchen (concepto de interfaz del 24/10/2026,
/// as in Paralives): one module with its worktop, which meets the
/// worktop of the module beside it flush, so a run of them reads as one
/// top. `unit` says what it is: drawers, doors, the sink (two doors under
/// a basin and tap), an oven under a drawer, a dishwasher, a hob over
/// drawers, a bottle rack or a blind corner.
pub fn base_unit(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2a).
    if crate::fine::kitchen::base_unit(b, p) {
        return;
    }
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 0.9);
    let unit = p.get("unit", 1.0).round() as i32;
    let kind = Handle::of(p, Handle::Bar);
    let style = Style::of(p);
    let x0 = -w / 2.0;
    let x1 = w / 2.0;
    let top = WORKTOP;
    let face = d - 0.04;
    let (z0, z1) = (0.105, h - top - 0.005);
    b.rbox(
        "plinth",
        [x0 + 0.002, 0.0, 0.0],
        [x1 - 0.002, d - 0.08, 0.1],
        0.0,
    );
    let sink = unit == 3;
    let hole = sink_hole(w, d);
    if sink && hole.is_some() && !p.flag("fine_sink", false) {
        // Lower under the basin, so it is seen.
        b.rbox("mat", [x0, 0.0, 0.1], [x1, face, h - 0.21], 0.0);
    } else {
        // Its top is always under a worktop (its own or its run's).
        b.open_box("mat", [x0, 0.0, 0.1], [x1, face, h - top]);
    }
    let drawers = p.get("drawers", 3.0).round().clamp(0.0, 4.0) as usize;
    let doors = p
        .get("doors", if w > 0.6 { 2.0 } else { 1.0 })
        .round()
        .clamp(1.0, 2.0) as usize;
    let door_row = |b: &mut Builder, za: f64, zb: f64, n: usize| {
        let fw = w / n as f64;
        for i in 0..n {
            let a = x0 + i as f64 * fw;
            fronts::styled_door(
                b,
                a,
                a + fw,
                za,
                zb,
                face,
                kind,
                i % 2 == 1 || n == 1,
                zb - 0.08,
                style,
            );
        }
    };
    match unit {
        // Drawers (none: one door).
        1 if drawers > 0 => {
            fronts::styled_drawers(b, x0, x1, z0, z1, face, drawers, kind, style, true)
        }
        1 | 2 => door_row(b, z0, z1, doors),
        3 => door_row(b, z0, z1, doors),
        4 => {
            // A drawer, the oven's door with its glass and handle.
            let zd = z1 - 0.14;
            fronts::styled_drawer(b, x0, x1, zd, z1, face, kind, style);
            b.rbox(
                "dark",
                [x0 + 0.01, face, z0 + 0.02],
                [x1 - 0.01, face + 0.02, zd - 0.01],
                0.004,
            );
            b.rbox(
                "glass",
                [x0 + 0.06, face + 0.02, z0 + 0.12],
                [x1 - 0.06, face + 0.022, zd - 0.12],
                0.0,
            );
            b.cyl(
                "metal",
                [x0 + 0.08, face + 0.05, zd - 0.05],
                [x1 - 0.08, face + 0.05, zd - 0.05],
                0.008,
            );
            b.rbox(
                "dark",
                [x0 + 0.04, face + 0.02, zd - 0.04],
                [x1 - 0.04, face + 0.022, zd - 0.015],
                0.0,
            );
        }
        // A dishwasher: an integrated front, its handle along the top.
        5 => fronts::styled_drawer(b, x0, x1, z0, z1, face, kind, style),
        // A hob over drawers.
        6 => fronts::styled_drawers(b, x0, x1, z0, z1, face, drawers.max(2), kind, style, true),
        // A bottle rack: open, with its shelves.
        7 => {
            let n = ((z1 - z0) / 0.13).floor().max(1.0) as usize;
            b.rbox(
                "dark",
                [x0 + 0.015, 0.01, z0],
                [x1 - 0.015, face - 0.01, z1],
                0.0,
            );
            for i in 0..=n {
                let z = z0 + i as f64 * (z1 - z0) / n as f64;
                b.rbox(
                    "mat",
                    [x0 + 0.01, 0.02, z],
                    [x1 - 0.01, face + 0.01, z + 0.015],
                    0.0,
                );
            }
        }
        // A blind corner: a fixed panel and a door.
        _ => {
            let blind = (w * 0.45).max(0.0);
            fronts::styled_door(
                b,
                x0,
                x0 + blind,
                z0,
                z1,
                face,
                Handle::None,
                true,
                z1 - 0.08,
                style,
            );
            fronts::styled_door(
                b,
                x0 + blind,
                x1,
                z0,
                z1,
                face,
                kind,
                false,
                z1 - 0.08,
                style,
            );
        }
    }
    // The worktop: flush with the module's sides, so the next one meets
    // it; it may overhang in front (a breakfast bar) or behind (an
    // island), and carry an upstand on its back edge.
    let front = p.get("overhang", 0.0).max(0.0);
    let back = p.get("overhang_back", 0.0).max(0.0);
    let up = p.get("upstand", 0.0).max(0.0);
    let (tlo, thi) = ([x0, -back, h - top], [x1, d + front, h]);
    if up > 1e-3 {
        b.rbox("top", [x0, -back, h], [x1, -back + UPSTAND, h + up], 0.0);
    }
    if sink && p.flag("fine_sink", false) {
        // The workshop's sink module makes its own worktop, basin and tap
        // (`fine::sink`).
    } else if let (true, Some(([sx0, sy0], [sx1, sy1]))) = (sink, hole) {
        b.framed_top("top", tlo, thi, [sx0, sy0], [sx1, sy1]);
        b.hollow("metal", [sx0, sy0, h - 0.2], [sx1, sy1, h], 0.03);
        tap(b, (sx0 + sx1) / 2.0, 0.05, h, 0.16);
    } else {
        b.rbox("top", tlo, thi, 0.0);
    }
    if unit == 6 {
        let (cx, cy) = (0.0, d / 2.0);
        let hw = (w / 2.0 - 0.04).min(0.38);
        b.rbox(
            "dark",
            [cx - hw, cy - 0.25, h],
            [cx + hw, cy + 0.25, h + 0.006],
            0.004,
        );
        let burners: &[(f64, f64, f64)] = if hw > 0.33 {
            &[
                (-0.2, -0.11, 0.09),
                (0.0, 0.1, 0.1),
                (0.2, -0.11, 0.07),
                (-0.2, 0.12, 0.07),
                (0.2, 0.12, 0.07),
            ]
        } else {
            &[
                (-0.13, -0.11, 0.09),
                (0.13, -0.11, 0.07),
                (-0.13, 0.11, 0.07),
                (0.13, 0.11, 0.09),
            ]
        };
        for (dx, dy, r) in burners {
            b.cone(
                "ring",
                [cx + dx, cy + dy, h + 0.006],
                [cx + dx, cy + dy, h + 0.008],
                *r,
                *r,
                24,
            );
        }
    }
}

/// The opening of a sink module's worktop for its basin, in the module's
/// own axes (`x` across from its centre, `y` from its back): none when
/// the module is too narrow for a basin. A worktop run over several
/// modules leaves the same opening.
pub fn sink_hole(width: f64, depth: f64) -> Option<([f64; 2], [f64; 2])> {
    (width >= 0.45).then(|| {
        (
            [-width / 2.0 + 0.06, 0.1],
            [width / 2.0 - 0.06, depth - 0.12],
        )
    })
}

/// The unit number of a sink among a base module's `unit` choices.
pub const SINK_UNIT: f64 = 3.0;

/// The thickness of a base module's worktop (m).
pub const WORKTOP: f64 = 0.03;

/// Thickness of a worktop's upstand (copete), m.
pub const UPSTAND: f64 = 0.02;

/// What a tall unit holds, as the `tall` choice of its family lists them.
pub const TALLS: &[&str] = &["Despensa", "Horno y microondas", "Frigorífico integrado"];

/// A tall unit of a modular kitchen: a larder with two doors, an oven and
/// a microwave between doors, or an integrated fridge.
pub fn tall_unit(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2a).
    if crate::fine::kitchen::tall_unit(b, p) {
        return;
    }
    let w = p.get("width", 0.6);
    let d = p.get("depth", 0.6);
    let h = p.get("height", 2.1);
    let tall = p.get("tall", 1.0).round() as i32;
    let kind = Handle::of(p, Handle::Bar);
    let style = Style::of(p);
    let (x0, x1) = (-w / 2.0, w / 2.0);
    let face = d - 0.02;
    b.rbox(
        "plinth",
        [x0 + 0.002, 0.0, 0.0],
        [x1 - 0.002, d - 0.06, 0.1],
        0.0,
    );
    b.rbox("mat", [x0, 0.0, 0.1], [x1, face, h], 0.0);
    let (z0, z1) = (0.105, h - 0.005);
    match tall {
        2 => {
            let (oa, ob) = (0.8, 1.38);
            fronts::styled_door(
                b,
                x0,
                x1,
                z0,
                oa - 0.005,
                face,
                kind,
                true,
                oa - 0.08,
                style,
            );
            b.rbox(
                "dark",
                [x0 + 0.01, face, oa],
                [x1 - 0.01, face + 0.02, ob],
                0.004,
            );
            b.rbox(
                "glass",
                [x0 + 0.06, face + 0.02, oa + 0.1],
                [x1 - 0.06, face + 0.022, ob - 0.14],
                0.0,
            );
            b.cyl(
                "metal",
                [x0 + 0.08, face + 0.05, ob - 0.07],
                [x1 - 0.08, face + 0.05, ob - 0.07],
                0.008,
            );
            let (ma, mb) = (ob + 0.01, ob + 0.39);
            b.rbox(
                "dark",
                [x0 + 0.01, face, ma],
                [x1 - 0.01, face + 0.02, mb],
                0.004,
            );
            b.rbox(
                "glass",
                [x0 + 0.05, face + 0.02, ma + 0.06],
                [x1 - 0.15, face + 0.022, mb - 0.06],
                0.0,
            );
            fronts::styled_door(
                b,
                x0,
                x1,
                mb + 0.005,
                z1,
                face,
                kind,
                true,
                mb + 0.08,
                style,
            );
        }
        3 => {
            let split = 0.1 + (h - 0.1) * 0.42;
            fronts::styled_door(
                b,
                x0,
                x1,
                z0,
                split - 0.003,
                face,
                kind,
                true,
                split - 0.1,
                style,
            );
            fronts::styled_door(
                b,
                x0,
                x1,
                split + 0.003,
                z1,
                face,
                kind,
                true,
                split + 0.1,
                style,
            );
        }
        _ => {
            let split = 0.1 + (h - 0.1) * 0.62;
            let n = if w > 0.6 { 2 } else { 1 };
            let fw = w / n as f64;
            for i in 0..n {
                let a = x0 + i as f64 * fw;
                let left = i % 2 == 1 || n == 1;
                fronts::styled_door(
                    b,
                    a,
                    a + fw,
                    z0,
                    split - 0.003,
                    face,
                    kind,
                    left,
                    split - 0.1,
                    style,
                );
                fronts::styled_door(
                    b,
                    a,
                    a + fw,
                    split + 0.003,
                    z1,
                    face,
                    kind,
                    left,
                    split + 0.1,
                    style,
                );
            }
        }
    }
}

/// Wall units, at their height over the floor.
pub fn wall_cabinet(b: &mut Builder, p: &Params) {
    // A collection's (paquete 2a).
    if crate::fine::kitchen::wall_cabinet(b, p) {
        return;
    }
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
    let kind = Handle::of(p, Handle::Bar);
    let style = Style::of(p);
    for i in 0..n {
        let a = x0 + i as f64 * fw;
        fronts::styled_door(
            b,
            a,
            a + fw,
            z0 + 0.003,
            z0 + h - 0.003,
            d - 0.02,
            kind,
            i % 2 == 0,
            z0 + 0.08,
            style,
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
