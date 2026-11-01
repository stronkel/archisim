//! The devices of the services, drawn with the care of the other pieces
//! (G4): switches and sockets with their plate, key and holes, the
//! distribution board with its door and the row of breakers, aluminium
//! radiators element by element with their valves, grilles with their
//! slats, ceiling diffusers with their rings, downlights, a boiler, and
//! risers and downpipes with their collars and sockets.
//!
//! They are not families: a device has a kind and a box (`width` along
//! `x`, `depth` along `y`, `height` up), and its model fills that box.
//! Axes: centred on `x = 0`, `y = 0`; the back on the wall at
//! `y = -depth / 2`, the face looking to `+y`; from `z = 0` up to
//! `z = height` (a ceiling device's top, at `height`, is on the ceiling).
//!
//! Roles ([`ROLES`]): `plate` the white plastic of a frame, `key` a key or
//! a cover, `metal` a valve, a screw or an earth clip, `dark` a hole or a
//! display, `glass` the smoked window of a board, `body` a radiator's or a
//! boiler's enamel, `pipe` a pipe (the colour of its system), `shade` what
//! glows of a light, `red` a pilot or what the fire protection paints red,
//! `sign` the lit green of an exit sign, `cell` a photovoltaic panel's
//! cells.
//!
//! The fire protection, the water's equipment and the solar panels (H3)
//! follow: a pendent sprinkler with its bulb, a hose reel in its cabinet,
//! an extinguisher on its hook, detectors, call points, sirens, emergency
//! lights and exit signs; a stop valve with its lever, a meter in its box,
//! a tank, a booster set, a water heater; and a row of panels or collectors
//! tilted in their box.

use crate::build::{Builder, ROUND};
use crate::PropMesh;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// The roles of the devices' models, in the order their colours are given.
pub const ROLES: [&str; 11] = [
    "plate", "key", "metal", "dark", "glass", "body", "pipe", "shade", "red", "sign", "cell",
];

/// The kinds that have a model: the codes of the devices' kinds.
pub const KINDS: [&str; 29] = [
    "equipment",
    "light",
    "socket",
    "switch",
    "radiator",
    "grille",
    "diffuser",
    "two_way_switch",
    "power_socket",
    "tv_socket",
    "data_socket",
    "panel",
    "riser",
    "downpipe",
    "sprinkler",
    "hose_reel",
    "extinguisher",
    "smoke_detector",
    "call_point",
    "siren",
    "emergency_light",
    "exit_sign",
    "valve",
    "water_meter",
    "tank",
    "pump",
    "water_heater",
    "solar_panel",
    "solar_collector",
];

/// How far a key or a shutter is tilted, radians.
const TILT: f64 = 0.06;

type Cache = Mutex<HashMap<(String, [i64; 3]), Arc<PropMesh>>>;

/// The model of a device of that kind (a code of [`KINDS`]) in that box,
/// made once for each kind and size.
pub fn device_model(kind: &str, w: f64, d: f64, h: f64) -> Option<Arc<PropMesh>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let key = (
        kind.to_string(),
        [w, d, h].map(|v| (v * 10_000.0).round() as i64),
    );
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().ok()?.get(&key) {
        return Some(m.clone());
    }
    let m = Arc::new(make(kind, w.max(0.01), d.max(0.005), h.max(0.005))?);
    if let Ok(mut c) = cache.lock() {
        if c.len() > 500 {
            c.clear();
        }
        c.insert(key, m.clone());
    }
    Some(m)
}

fn make(kind: &str, w: f64, d: f64, h: f64) -> Option<PropMesh> {
    let mut b = Builder::default();
    let back = -d / 2.0;
    match kind {
        "switch" => switch(&mut b, w, back, h, false),
        "two_way_switch" => switch(&mut b, w, back, h, true),
        "socket" => socket(&mut b, w, back, h, false),
        "power_socket" => socket(&mut b, w, back, h, true),
        "tv_socket" => tv_socket(&mut b, w, back, h),
        "data_socket" => data_socket(&mut b, w, back, h),
        "panel" => panel(&mut b, w, d, h),
        "radiator" => radiator(&mut b, w, d, h),
        "grille" => grille(&mut b, w, d, h),
        "diffuser" => diffuser(&mut b, w, d, h),
        "light" => downlight(&mut b, w, d, h),
        "equipment" => boiler(&mut b, w, d, h),
        "riser" => riser(&mut b, w, h, false),
        "downpipe" => riser(&mut b, w, h, true),
        "sprinkler" => sprinkler(&mut b, w, h),
        "hose_reel" => hose_reel(&mut b, w, d, h),
        "extinguisher" => extinguisher(&mut b, w, d, h),
        "smoke_detector" => smoke_detector(&mut b, w, h),
        "call_point" => call_point(&mut b, w, d, h),
        "siren" => siren(&mut b, w, d, h),
        "emergency_light" => emergency_light(&mut b, w, d, h),
        "exit_sign" => exit_sign(&mut b, w, d, h),
        "valve" => valve(&mut b, w, d, h),
        "water_meter" => water_meter(&mut b, w, d, h),
        "tank" => tank(&mut b, w, d, h),
        "pump" => pump(&mut b, w, d, h),
        "water_heater" => water_heater(&mut b, w, d, h),
        "solar_panel" => solar_row(&mut b, w, d, h, false),
        "solar_collector" => solar_row(&mut b, w, d, h, true),
        _ => return None,
    }
    Some(PropMesh {
        positions: b.positions,
        parts: b.parts.into_iter().collect(),
        soft: Vec::new(),
        fine: Vec::new(),
    })
}

/// The square plate of a mechanism on the wall, its face `t` out from
/// the wall: `(side, middle height, face)`.
fn plate(b: &mut Builder, w: f64, back: f64, h: f64) -> (f64, f64, f64) {
    let side = w.min(h).clamp(0.05, 0.12);
    let zc = h / 2.0;
    let t = 0.009;
    // The frame, its edges rounded as a plastic frame's are.
    b.rbox(
        "plate",
        [-side / 2.0, back, zc - side / 2.0],
        [side / 2.0, back + t, zc + side / 2.0],
        0.003,
    );
    (side, zc, back + t)
}

/// A switch: the frame and a wide key, rocked a little; a two-way switch
/// has the two arrows of its key.
fn switch(b: &mut Builder, w: f64, back: f64, h: f64, two_way: bool) {
    let (side, zc, face) = plate(b, w, back, h);
    let k = side * 0.62;
    // The key, a little proud of the frame and tilted (its lower half out).
    b.obox(
        "key",
        [0.0, face + 0.004, zc],
        [k / 2.0, 0.0, 0.0],
        [0.0, 0.004 * TILT.cos(), -0.004 * TILT.sin()],
        [0.0, k / 2.0 * TILT.sin(), k / 2.0 * TILT.cos()],
    );
    // The line across the key where it rocks.
    b.rbox(
        "plate",
        [-k / 2.0 + 0.002, face + 0.0075, zc - 0.0004],
        [k / 2.0 - 0.002, face + 0.0085, zc + 0.0004],
        0.0,
    );
    // The pilot light, low on the key.
    b.rbox(
        "red",
        [-0.002, face + 0.0078, zc - k * 0.38],
        [0.002, face + 0.0088, zc - k * 0.38 + 0.0018],
        0.0,
    );
    if two_way {
        // Up and down arrows printed on the key.
        for s in [-1.0, 1.0] {
            let z = zc + s * k * 0.22;
            b.obox(
                "dark",
                [0.0, face + 0.0083, z],
                [0.0035, 0.0, 0.0],
                [0.0, 0.0004, 0.0],
                [0.0, 0.0, 0.0006],
            );
            b.obox(
                "dark",
                [0.0, face + 0.0083, z + s * 0.0018],
                [0.0016, 0.0, 0.0],
                [0.0, 0.0004, 0.0],
                [0.0, 0.0, 0.0006],
            );
        }
    }
}

/// A round cup sunk in the cover of a socket: its rim and its dark floor.
fn cup(b: &mut Builder, face: f64, zc: f64, r: f64) {
    b.cone(
        "key",
        [0.0, face, zc],
        [0.0, face + 0.006, zc],
        r + 0.004,
        r + 0.003,
        ROUND * 2,
    );
    b.cone(
        "dark",
        [0.0, face + 0.0035, zc],
        [0.0, face + 0.0062, zc],
        r,
        r,
        ROUND * 2,
    );
    // The floor of the cup, back to the cover's colour.
    b.cone(
        "key",
        [0.0, face + 0.0055, zc],
        [0.0, face + 0.0063, zc],
        r - 0.0012,
        r - 0.0012,
        ROUND * 2,
    );
}

/// A socket: the frame, the cover with its cup, the two holes and the
/// earth clips; the 25 A one of the cooker is bigger, with a red pilot.
fn socket(b: &mut Builder, w: f64, back: f64, h: f64, power: bool) {
    let (side, zc, face) = plate(b, w, back, h);
    let r = side * if power { 0.3 } else { 0.27 };
    cup(b, face, zc, r);
    let y = face + 0.0063;
    let gap = r * 0.5;
    let hole = if power { 0.0028 } else { 0.0022 };
    for s in [-1.0, 1.0] {
        b.cone(
            "dark",
            [s * gap, y - 0.0003, zc],
            [s * gap, y + 0.0004, zc],
            hole,
            hole,
            12,
        );
    }
    // The earth clips, top and bottom of the cup.
    for s in [-1.0, 1.0] {
        b.rbox(
            "metal",
            [-0.003, y - 0.0006, zc + s * (r - 0.0015) - 0.0012],
            [0.003, y + 0.0006, zc + s * (r - 0.0015) + 0.0012],
            0.0,
        );
    }
    if power {
        b.rbox(
            "red",
            [-0.003, face + 0.0005, zc - side * 0.44],
            [0.003, face + 0.0015, zc - side * 0.41],
            0.0,
        );
    }
}

/// An aerial socket: the cover and its two connectors, the male and the
/// female, each a short metal barrel with a dark centre.
fn tv_socket(b: &mut Builder, w: f64, back: f64, h: f64) {
    let (side, zc, face) = plate(b, w, back, h);
    let k = side * 0.62;
    b.rbox(
        "key",
        [-k / 2.0, face, zc - k / 2.0],
        [k / 2.0, face + 0.004, zc + k / 2.0],
        0.002,
    );
    for (s, r) in [(-1.0, 0.0045), (1.0, 0.004)] {
        let z = zc + s * k * 0.22;
        b.cone(
            "metal",
            [0.0, face + 0.004, z],
            [0.0, face + 0.012, z],
            r,
            r,
            ROUND,
        );
        b.cone(
            "dark",
            [0.0, face + 0.0115, z],
            [0.0, face + 0.0125, z],
            r * 0.45,
            r * 0.45,
            10,
        );
    }
}

/// A data socket: the cover with the square mouth of an RJ45 and its
/// sprung shutter, raised a little.
fn data_socket(b: &mut Builder, w: f64, back: f64, h: f64) {
    let (side, zc, face) = plate(b, w, back, h);
    let k = side * 0.62;
    b.rbox(
        "key",
        [-k / 2.0, face, zc - k / 2.0],
        [k / 2.0, face + 0.004, zc + k / 2.0],
        0.002,
    );
    b.rbox(
        "dark",
        [-0.0075, face + 0.0035, zc - 0.006],
        [0.0075, face + 0.0048, zc + 0.006],
        0.0,
    );
    // The shutter, hinged at the top, open a little.
    b.obox(
        "key",
        [0.0, face + 0.0062, zc + 0.002],
        [0.0078, 0.0, 0.0],
        [0.0, 0.0008 * TILT.cos(), -0.0008 * TILT.sin()],
        [0.0, 0.0045 * TILT.sin(), 0.0045 * TILT.cos()],
    );
}

/// The distribution board: its box, the door with the smoked window over
/// the row of breakers, the lock and the hinges.
fn panel(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let front = d / 2.0;
    let r = 0.008;
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.0],
        [w / 2.0, front - 0.012, h],
        r,
    );
    // The door, a little in from the box's edge.
    let m = 0.012;
    let door = [-w / 2.0 + m, front - 0.014, m];
    let door_hi = [w / 2.0 - m, front, h - m];
    b.rbox("key", door, door_hi, 0.004);
    // Rows of breakers behind the window: one row each 0.15 m.
    let rows = ((h - 0.1) / 0.15).floor().max(1.0) as usize;
    let pitch = (h - 2.0 * m) / rows as f64;
    let pole = 0.0178;
    for i in 0..rows {
        let zc = m + pitch * (i as f64 + 0.5);
        let win_lo = [-w / 2.0 + m * 3.0, front - 0.001, zc - 0.045];
        let win_hi = [w / 2.0 - m * 3.0, front + 0.002, zc + 0.045];
        b.rbox("glass", win_lo, win_hi, 0.003);
        // The breakers seen through it: their dark faces and white levers.
        let n = ((win_hi[0] - win_lo[0] - 0.01) / pole).floor().max(1.0) as usize;
        let x0 = -(n as f64) * pole / 2.0;
        for k in 0..n {
            let x = x0 + pole * (k as f64 + 0.5);
            b.rbox(
                "plate",
                [x - 0.0035, front + 0.002, zc - 0.012],
                [x + 0.0035, front + 0.006, zc + 0.006],
                0.0,
            );
            b.rbox(
                "dark",
                [x - pole * 0.42, front + 0.002, zc - 0.035],
                [x + pole * 0.42, front + 0.0035, zc - 0.018],
                0.0,
            );
        }
    }
    // The lock and the two hinges.
    b.cone(
        "metal",
        [w / 2.0 - m * 2.5, front, h / 2.0],
        [w / 2.0 - m * 2.5, front + 0.006, h / 2.0],
        0.006,
        0.006,
        ROUND,
    );
    b.rbox(
        "dark",
        [w / 2.0 - m * 2.5 - 0.0008, front + 0.0055, h / 2.0 - 0.003],
        [w / 2.0 - m * 2.5 + 0.0008, front + 0.0065, h / 2.0 + 0.003],
        0.0,
    );
    for z in [h * 0.2, h * 0.8] {
        b.cyl(
            "metal",
            [-w / 2.0 + m, front - 0.004, z - 0.02],
            [-w / 2.0 + m, front - 0.004, z + 0.02],
            0.004,
        );
    }
}

/// An aluminium radiator, element by element: each with its fins and its
/// two hubs, the manifolds through them, the valve at one end and the
/// lockshield at the other, the bleed screw and the brackets to the wall.
fn radiator(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let gap = 0.035;
    let deep = (d - gap).max(0.04);
    let y0 = back + gap;
    let y1 = y0 + deep;
    // The valve and the lockshield take the ends.
    let end = 0.045;
    let span = (w - 2.0 * end).max(0.08);
    let pitch = 0.08;
    let n = (span / pitch).round().max(1.0) as usize;
    let pitch = span / n as f64;
    let x0 = -span / 2.0;
    let foot = 0.0;
    let z0 = foot + 0.04;
    let z1 = h - 0.01;
    for i in 0..n {
        let x = x0 + pitch * (i as f64 + 0.5);
        let half = pitch / 2.0 - 0.003;
        // The front face of the element, rounded, and the fins behind it.
        b.rbox(
            "body",
            [x - half, y1 - 0.016, z0],
            [x + half, y1, z1],
            0.006,
        );
        b.rbox(
            "body",
            [x - 0.004, y0 + 0.004, z0 + 0.02],
            [x + 0.004, y1 - 0.01, z1 - 0.02],
            0.002,
        );
        for s in [-1.0, 1.0] {
            b.rbox(
                "body",
                [x + s * half * 0.55 - 0.001, y0 + 0.01, z0 + 0.05],
                [x + s * half * 0.55 + 0.001, y1 - 0.01, z1 - 0.05],
                0.0,
            );
        }
        // Its hubs top and bottom, where the manifolds go through.
        for z in [z0 + 0.03, z1 - 0.03] {
            b.cone(
                "body",
                [x - half, y0 + deep * 0.45, z],
                [x + half, y0 + deep * 0.45, z],
                0.022,
                0.022,
                ROUND,
            );
        }
    }
    // The manifolds out to the ends, with their plugs.
    let ym = y0 + deep * 0.45;
    for z in [z0 + 0.03, z1 - 0.03] {
        b.cone(
            "metal",
            [-span / 2.0 - 0.012, ym, z],
            [span / 2.0 + 0.012, ym, z],
            0.016,
            0.016,
            ROUND,
        );
    }
    // The bleed screw, top right.
    b.cone(
        "metal",
        [span / 2.0 + 0.012, ym, z1 - 0.03],
        [span / 2.0 + 0.02, ym, z1 - 0.03],
        0.006,
        0.006,
        12,
    );
    // The thermostatic valve (left, its head up) and the lockshield
    // (right), both on the bottom manifold, their pipes down to the floor.
    let zv = z0 + 0.03;
    let xl = -span / 2.0 - 0.025;
    b.cone(
        "metal",
        [xl + 0.013, ym, zv],
        [xl - 0.005, ym, zv],
        0.011,
        0.011,
        ROUND,
    );
    b.cone(
        "body",
        [xl, ym, zv],
        [xl, ym, zv + 0.05],
        0.014,
        0.013,
        ROUND,
    );
    b.cone(
        "body",
        [xl, ym, zv + 0.05],
        [xl, ym, zv + 0.075],
        0.016,
        0.016,
        ROUND,
    );
    let xr = span / 2.0 + 0.025;
    b.cone(
        "metal",
        [xr - 0.013, ym, zv],
        [xr + 0.005, ym, zv],
        0.011,
        0.011,
        ROUND,
    );
    b.cone("metal", [xr, ym, zv], [xr, ym, zv + 0.02], 0.009, 0.008, 12);
    for x in [xl, xr] {
        b.cyl("pipe", [x, ym, -0.002], [x, ym, zv - 0.008], 0.0075);
    }
    // Two brackets from the wall to the top manifold.
    for s in [-1.0, 1.0] {
        let x = s * span * 0.3;
        b.rbox(
            "metal",
            [x - 0.012, back, z1 - 0.05],
            [x + 0.012, y0 + 0.006, z1 - 0.038],
            0.0,
        );
    }
}

/// A return grille on the wall: its frame and the slats tilted down.
fn grille(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let t = d.min(0.03);
    let f = 0.022;
    // The frame: four bars mitred at the corners.
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.0],
        [w / 2.0, back + t, f],
        0.002,
    );
    b.rbox(
        "plate",
        [-w / 2.0, back, h - f],
        [w / 2.0, back + t, h],
        0.002,
    );
    b.rbox(
        "plate",
        [-w / 2.0, back, f],
        [-w / 2.0 + f, back + t, h - f],
        0.002,
    );
    b.rbox(
        "plate",
        [w / 2.0 - f, back, f],
        [w / 2.0, back + t, h - f],
        0.002,
    );
    // The dark of the duct behind.
    b.rbox(
        "dark",
        [-w / 2.0 + f, back, f],
        [w / 2.0 - f, back + 0.003, h - f],
        0.0,
    );
    // The slats, one each 15 mm, tilted 40° down into the room.
    let inner = h - 2.0 * f;
    let n = (inner / 0.015).floor().max(2.0) as usize;
    let pitch = inner / n as f64;
    let (s, c) = (40f64.to_radians().sin(), 40f64.to_radians().cos());
    let half = pitch * 0.62;
    for i in 0..n {
        let z = f + pitch * (i as f64 + 0.5);
        b.obox(
            "key",
            [0.0, back + t * 0.55, z],
            [w / 2.0 - f, 0.0, 0.0],
            [0.0, half * c, -half * s],
            [0.0, 0.0006 * s, 0.0006 * c],
        );
    }
}

/// A square ceiling diffuser: the flat face on the ceiling and its rings
/// stepping up into the neck, with the round plug in the middle.
fn diffuser(b: &mut Builder, w: f64, d: f64, h: f64) {
    let top = h;
    let face = (h * 0.4).clamp(0.006, 0.02);
    // The outer flange on the ceiling.
    b.rbox(
        "plate",
        [-w / 2.0, -d / 2.0, top - face],
        [w / 2.0, d / 2.0, top],
        0.003,
    );
    // The rings, each smaller and higher: dark gaps between them.
    let rings = 4;
    for i in 0..rings {
        let k = 1.0 - (i as f64 + 1.0) / (rings as f64 + 1.5);
        let (hw, hd) = (w / 2.0 * k, d / 2.0 * k);
        b.rbox(
            "dark",
            [-hw - 0.004, -hd - 0.004, top - face - 0.0015],
            [hw + 0.004, hd + 0.004, top - face + 0.0005],
            0.0,
        );
        b.rbox(
            "plate",
            [-hw, -hd, top - face - 0.003 - 0.0025 * i as f64],
            [hw, hd, top - face],
            0.002,
        );
    }
    // The plug.
    b.cone(
        "plate",
        [0.0, 0.0, top - face - 0.016],
        [0.0, 0.0, top - face],
        w.min(d) * 0.08,
        w.min(d) * 0.1,
        ROUND,
    );
}

/// A downlight in the ceiling: its trim ring flush with the ceiling and
/// the lens set in, which glows.
fn downlight(b: &mut Builder, w: f64, d: f64, h: f64) {
    let r = w.min(d) / 2.0;
    let top = h;
    let t = h.min(0.012);
    // The trim, slightly bevelled.
    b.cone(
        "plate",
        [0.0, 0.0, top - t],
        [0.0, 0.0, top],
        r * 0.94,
        r,
        ROUND * 2,
    );
    b.cone(
        "metal",
        [0.0, 0.0, top - t - 0.002],
        [0.0, 0.0, top - t + 0.0005],
        r * 0.78,
        r * 0.8,
        ROUND * 2,
    );
    b.cone(
        "shade",
        [0.0, 0.0, top - t - 0.0035],
        [0.0, 0.0, top - t - 0.001],
        r * 0.62,
        r * 0.66,
        ROUND * 2,
    );
}

/// A wall-hung boiler: its rounded case, the panel at the bottom with its
/// display and knobs, the vent on top and the pipes down from the bottom.
fn boiler(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let pipes = (h * 0.12).clamp(0.0, 0.12);
    let z0 = pipes;
    b.rbox("body", [-w / 2.0, back, z0], [w / 2.0, d / 2.0, h], 0.025);
    // The control panel along the bottom of the front.
    let ph = ((h - z0) * 0.14).clamp(0.05, 0.12);
    b.rbox(
        "key",
        [-w / 2.0 + 0.02, d / 2.0 - 0.01, z0 + 0.015],
        [w / 2.0 - 0.02, d / 2.0 + 0.004, z0 + 0.015 + ph],
        0.006,
    );
    let zc = z0 + 0.015 + ph / 2.0;
    b.rbox(
        "dark",
        [-0.035, d / 2.0 + 0.003, zc - 0.015],
        [0.035, d / 2.0 + 0.006, zc + 0.015],
        0.002,
    );
    for s in [-1.0, 1.0] {
        let x = s * w * 0.3;
        b.cone(
            "metal",
            [x, d / 2.0 + 0.003, zc],
            [x, d / 2.0 + 0.016, zc],
            0.013,
            0.012,
            ROUND,
        );
    }
    b.rbox(
        "red",
        [w * 0.15, d / 2.0 + 0.003, zc - 0.003],
        [w * 0.15 + 0.006, d / 2.0 + 0.006, zc + 0.003],
        0.0,
    );
    // The flue on the top.
    b.cone(
        "metal",
        [0.0, 0.0, h - 0.002],
        [0.0, 0.0, h + 0.0],
        0.05,
        0.05,
        ROUND * 2,
    );
    // The pipes down from the bottom: gas, flow, return, hot and cold.
    if pipes > 0.01 {
        let n = 5;
        for k in 0..n {
            let x = -w * 0.32 + w * 0.64 * k as f64 / (n - 1) as f64;
            let y = back + d * 0.4;
            b.cyl("pipe", [x, y, 0.0], [x, y, z0 + 0.002], 0.009);
            b.cone(
                "metal",
                [x, y, z0 - 0.03],
                [x, y, z0 - 0.012],
                0.013,
                0.013,
                6,
            );
        }
    }
}

/// A riser (copper, its collars each 1.5 m) or a downpipe (its sockets at
/// the bottom and the top and its clamps between).
fn riser(b: &mut Builder, w: f64, h: f64, downpipe: bool) {
    let r = w / 2.0;
    b.cone("pipe", [0.0, 0.0, 0.0], [0.0, 0.0, h], r, r, ROUND * 2);
    let collar = |b: &mut Builder, z: f64, k: f64, l: f64, role: &'static str| {
        b.cone(
            role,
            [0.0, 0.0, z - l / 2.0],
            [0.0, 0.0, z + l / 2.0],
            r * k,
            r * k,
            ROUND * 2,
        );
    };
    if downpipe {
        // The sockets where two lengths meet, and the clamps.
        collar(b, 0.05, 1.12, 0.1, "pipe");
        collar(b, h - 0.05, 1.12, 0.1, "pipe");
        let n = ((h / 1.0).floor() as usize).max(1);
        for k in 1..=n {
            let z = h * k as f64 / (n + 1) as f64;
            collar(b, z, 1.2, 0.03, "metal");
            // Its screw out to the wall side (back, -y).
            b.cone(
                "metal",
                [0.0, -r * 1.15, z],
                [0.0, -r * 1.15 - 0.03, z],
                0.004,
                0.004,
                8,
            );
        }
    } else {
        let n = ((h / 1.5).floor() as usize).max(1);
        for k in 0..n {
            let z = 0.75 + 1.5 * k as f64;
            if z >= h {
                break;
            }
            collar(b, z, 1.5, 0.02, "metal");
            b.cone(
                "metal",
                [0.0, -r * 1.45, z],
                [0.0, -r * 1.45 - 0.035, z],
                0.004,
                0.004,
                8,
            );
        }
        // The joints soldered, each 3 m.
        let mut z = 3.0;
        while z < h - 0.1 {
            collar(b, z, 1.12, 0.035, "pipe");
            z += 3.0;
        }
    }
}

// ---- Fire protection (H3) --------------------------------------------------

/// A pendent sprinkler: the escutcheon on the ceiling, the frame down to
/// its deflector, the red glass bulb between its arms.
fn sprinkler(b: &mut Builder, w: f64, h: f64) {
    let r = (w / 2.0).max(0.02);
    b.cone(
        "metal",
        [0.0, 0.0, h - 0.006],
        [0.0, 0.0, h],
        r,
        r,
        ROUND * 2,
    );
    b.cone(
        "metal",
        [0.0, 0.0, h * 0.55],
        [0.0, 0.0, h - 0.006],
        r * 0.25,
        r * 0.3,
        ROUND,
    );
    for s in [-1.0, 1.0] {
        b.obox(
            "metal",
            [s * r * 0.32, 0.0, h * 0.35],
            [0.003, 0.0, 0.0],
            [0.0, 0.004, 0.0],
            [0.0, 0.0, h * 0.2],
        );
    }
    b.ellipsoid("red", [0.0, 0.0, h * 0.35], [r * 0.12, r * 0.12, h * 0.15]);
    b.cone(
        "metal",
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.003],
        r * 0.75,
        r * 0.75,
        ROUND * 2,
    );
    b.cone(
        "metal",
        [0.0, 0.0, 0.003],
        [0.0, 0.0, h * 0.16],
        r * 0.08,
        r * 0.08,
        8,
    );
}

/// A hose reel cabinet: the red box, its glazed door, the drum with the
/// hose wound on it and its valve.
fn hose_reel(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let front = d / 2.0;
    let t = 0.012;
    b.hollow("red", [-w / 2.0, back, 0.0], [w / 2.0, front - 0.02, h], t);
    // The back of the box.
    b.rbox("red", [-w / 2.0, back, 0.0], [w / 2.0, back + t, h], 0.0);
    // Its door: the frame and the glass.
    let f = 0.035;
    b.rbox(
        "red",
        [-w / 2.0, front - 0.02, 0.0],
        [w / 2.0, front, f],
        0.0,
    );
    b.rbox(
        "red",
        [-w / 2.0, front - 0.02, h - f],
        [w / 2.0, front, h],
        0.0,
    );
    b.rbox(
        "red",
        [-w / 2.0, front - 0.02, f],
        [-w / 2.0 + f, front, h - f],
        0.0,
    );
    b.rbox(
        "red",
        [w / 2.0 - f, front - 0.02, f],
        [w / 2.0, front, h - f],
        0.0,
    );
    b.rbox(
        "glass",
        [-w / 2.0 + f, front - 0.012, f],
        [w / 2.0 - f, front - 0.008, h - f],
        0.0,
    );
    b.rbox(
        "key",
        [-0.05, front - 0.004, h - f - 0.05],
        [0.05, front + 0.002, h - f - 0.01],
        0.002,
    );
    // The drum, its axis out of the wall, with the hose wound on it.
    let zc = h * 0.55;
    let rr = (w.min(h) * 0.38).max(0.1);
    let y0 = back + t + 0.02;
    let y1 = (front - 0.04).max(y0 + 0.05);
    b.cone(
        "metal",
        [0.0, y0, zc],
        [0.0, y0 + 0.01, zc],
        rr,
        rr,
        ROUND * 2,
    );
    b.cone(
        "metal",
        [0.0, y1 - 0.01, zc],
        [0.0, y1, zc],
        rr,
        rr,
        ROUND * 2,
    );
    b.cone(
        "red",
        [0.0, y0 + 0.01, zc],
        [0.0, y1 - 0.01, zc],
        rr * 0.85,
        rr * 0.85,
        ROUND * 2,
    );
    b.cone(
        "metal",
        [0.0, y0, zc],
        [0.0, y1 + 0.005, zc],
        rr * 0.15,
        rr * 0.15,
        ROUND,
    );
    // The valve and the nozzle at the bottom.
    b.cone(
        "metal",
        [w * 0.3, y0 + 0.03, 0.04],
        [w * 0.3, y0 + 0.03, zc - rr],
        0.018,
        0.018,
        ROUND,
    );
    b.cone(
        "dark",
        [-w * 0.3, y1 - 0.03, 0.05],
        [-w * 0.3, y1 - 0.03, 0.2],
        0.015,
        0.02,
        ROUND,
    );
}

/// An extinguisher on its hook: the red bottle, its valve and handle, the
/// hose down its side; the bracket on the wall.
fn extinguisher(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let r = (w.min(d) / 2.0 * 0.85).max(0.04);
    let yc = back + 0.012 + r;
    let body = h * 0.78;
    b.lathe(
        "red",
        [0.0, yc, 0.0],
        &[
            (0.0, 0.0),
            (r, 0.0),
            (r, body * 0.92),
            (r * 0.6, body),
            (0.0, body),
        ],
        1.0,
        1.0,
    );
    // Its valve, handle and gauge.
    b.cone(
        "metal",
        [0.0, yc, body],
        [0.0, yc, body + h * 0.08],
        r * 0.3,
        r * 0.25,
        ROUND,
    );
    b.obox(
        "dark",
        [0.0, yc + r * 0.3, body + h * 0.12],
        [0.012, 0.0, 0.0],
        [0.0, r * 0.9, 0.0],
        [0.0, 0.0, 0.008],
    );
    b.cone(
        "glass",
        [r * 0.3, yc, body + h * 0.05],
        [r * 0.3 + 0.012, yc, body + h * 0.05],
        0.015,
        0.015,
        ROUND,
    );
    // The hose down the front, to its nozzle.
    b.cone(
        "dark",
        [-r * 0.3, yc + r * 0.3, body + h * 0.06],
        [-r * 1.05, yc + r * 0.6, body * 0.25],
        0.008,
        0.008,
        10,
    );
    b.cone(
        "dark",
        [-r * 1.05, yc + r * 0.6, body * 0.25],
        [-r * 1.05, yc + r * 0.6, body * 0.15],
        0.008,
        0.014,
        10,
    );
    // The bracket on the wall and its hook.
    b.rbox(
        "metal",
        [-0.03, back, body * 0.55],
        [0.03, back + 0.006, body * 0.95],
        0.002,
    );
    b.rbox(
        "metal",
        [-0.02, back, body * 0.9],
        [0.02, yc - r + 0.002, body * 0.93],
        0.002,
    );
}

/// An optical smoke detector: its base on the ceiling and the dome, with
/// its red light.
fn smoke_detector(b: &mut Builder, w: f64, h: f64) {
    let r = (w / 2.0).max(0.03);
    b.cone("plate", [0.0, 0.0, h * 0.7], [0.0, 0.0, h], r, r, ROUND * 2);
    b.lathe(
        "plate",
        [0.0, 0.0, 0.0],
        &[
            (0.0, 0.0),
            (r * 0.55, 0.0),
            (r * 0.8, h * 0.35),
            (r * 0.85, h * 0.7),
        ],
        1.0,
        1.0,
    );
    b.ellipsoid("red", [r * 0.55, 0.0, h * 0.3], [0.004, 0.004, 0.004]);
}

/// A manual call point: the red box, its white window, the spot to press.
fn call_point(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    b.rbox(
        "red",
        [-w / 2.0, back, 0.0],
        [w / 2.0, d / 2.0 - 0.004, h],
        0.004,
    );
    let m = w.min(h) * 0.32;
    b.rbox(
        "key",
        [-m, d / 2.0 - 0.006, h / 2.0 - m],
        [m, d / 2.0, h / 2.0 + m],
        0.002,
    );
    b.ellipsoid("dark", [0.0, d / 2.0, h / 2.0], [m * 0.3, 0.003, m * 0.3]);
}

/// A sounder with its flash: a red body and the lens on its front.
fn siren(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.0],
        [w / 2.0, back + 0.012, h],
        0.004,
    );
    let r = w.min(h) / 2.0 * 0.9;
    b.cone(
        "red",
        [0.0, back + 0.012, h / 2.0],
        [0.0, d / 2.0 - d * 0.25, h / 2.0],
        r,
        r * 0.85,
        ROUND * 2,
    );
    b.cone(
        "shade",
        [0.0, d / 2.0 - d * 0.25, h / 2.0],
        [0.0, d / 2.0, h / 2.0],
        r * 0.55,
        r * 0.4,
        ROUND * 2,
    );
}

/// An emergency light: a white body and its diffuser, lit at night.
fn emergency_light(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.0],
        [w / 2.0, d / 2.0 - 0.012, h],
        0.01,
    );
    b.rbox(
        "shade",
        [-w / 2.0 + 0.02, d / 2.0 - 0.016, 0.012],
        [w / 2.0 - 0.02, d / 2.0, h - 0.012],
        0.006,
    );
    b.ellipsoid(
        "sign",
        [w / 2.0 - 0.03, d / 2.0 - 0.01, h * 0.5],
        [0.003, 0.003, 0.003],
    );
}

/// An exit sign: its frame and the lit green face with the running man
/// and the arrow in white.
fn exit_sign(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.0],
        [w / 2.0, d / 2.0 - 0.003, h],
        0.004,
    );
    let f = 0.012;
    b.rbox(
        "sign",
        [-w / 2.0 + f, d / 2.0 - 0.004, f],
        [w / 2.0 - f, d / 2.0, h - f],
        0.0,
    );
    // The figure running, and the arrow the way it runs.
    let y = d / 2.0 + 0.001;
    let (x0, z0) = (-w * 0.2, h / 2.0);
    let s = h * 0.32;
    b.ellipsoid(
        "shade",
        [x0 + s * 0.15, y, z0 + s * 0.8],
        [s * 0.12, 0.001, s * 0.12],
    );
    b.obox(
        "shade",
        [x0, y, z0 + s * 0.2],
        [s * 0.08, 0.0, s * 0.03],
        [0.0, 0.001, 0.0],
        [-s * 0.15, 0.0, s * 0.45],
    );
    b.obox(
        "shade",
        [x0 - s * 0.25, y, z0 - s * 0.45],
        [s * 0.07, 0.0, -s * 0.05],
        [0.0, 0.001, 0.0],
        [-s * 0.25, 0.0, -s * 0.3],
    );
    b.obox(
        "shade",
        [x0 + s * 0.2, y, z0 - s * 0.45],
        [s * 0.07, 0.0, s * 0.05],
        [0.0, 0.001, 0.0],
        [s * 0.2, 0.0, -s * 0.3],
    );
    b.rbox(
        "shade",
        [w * 0.08, y - 0.001, z0 - s * 0.06],
        [w * 0.36, y, z0 + s * 0.06],
        0.0,
    );
    b.obox(
        "shade",
        [w * 0.11, y, z0 + s * 0.18],
        [s * 0.2, 0.0, s * 0.2],
        [0.0, 0.001, 0.0],
        [-s * 0.05, 0.0, s * 0.05],
    );
    b.obox(
        "shade",
        [w * 0.11, y, z0 - s * 0.18],
        [s * 0.2, 0.0, -s * 0.2],
        [0.0, 0.001, 0.0],
        [s * 0.05, 0.0, s * 0.05],
    );
}

// ---- Water's equipment (H3) ------------------------------------------------

/// A ball stop valve on its pipe, along `x`: the pipe, the brass body with
/// its nuts, the lever.
fn valve(b: &mut Builder, w: f64, d: f64, h: f64) {
    let zc = h * 0.4;
    let r = (d.min(h) * 0.22).max(0.008);
    b.cone("pipe", [-w / 2.0, 0.0, zc], [w / 2.0, 0.0, zc], r, r, ROUND);
    b.ellipsoid("metal", [0.0, 0.0, zc], [w * 0.2, r * 1.7, r * 1.7]);
    for s in [-1.0, 1.0] {
        b.cone(
            "metal",
            [s * w * 0.18, 0.0, zc],
            [s * w * 0.36, 0.0, zc],
            r * 1.5,
            r * 1.5,
            6,
        );
    }
    b.cone(
        "metal",
        [0.0, 0.0, zc],
        [0.0, 0.0, zc + r * 2.6],
        r * 0.5,
        r * 0.5,
        10,
    );
    b.obox(
        "red",
        [w * 0.18, 0.0, zc + r * 2.8],
        [w * 0.24, 0.0, 0.0],
        [0.0, r * 0.7, 0.0],
        [0.0, 0.0, r * 0.3],
    );
}

/// A water meter in its box: the box on the wall with its door and the
/// meter's window; the pipe in and out at the bottom.
fn water_meter(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    b.rbox(
        "plate",
        [-w / 2.0, back, 0.06],
        [w / 2.0, d / 2.0 - 0.01, h],
        0.008,
    );
    b.rbox(
        "key",
        [-w / 2.0 + 0.01, d / 2.0 - 0.012, 0.07],
        [w / 2.0 - 0.01, d / 2.0, h - 0.01],
        0.004,
    );
    let r = (w.min(h) * 0.14).max(0.02);
    b.cone(
        "glass",
        [0.0, d / 2.0, h * 0.6],
        [0.0, d / 2.0 + 0.003, h * 0.6],
        r,
        r,
        ROUND * 2,
    );
    b.cone(
        "dark",
        [0.0, d / 2.0 + 0.003, h * 0.6],
        [0.0, d / 2.0 + 0.004, h * 0.6],
        r * 0.25,
        r * 0.25,
        ROUND,
    );
    for s in [-1.0, 1.0] {
        b.cone(
            "pipe",
            [s * w * 0.3, 0.0, 0.0],
            [s * w * 0.3, 0.0, 0.07],
            0.012,
            0.012,
            ROUND,
        );
    }
}

/// A standing tank: its body, the domed lid, the manhole, the pipes in and
/// out.
fn tank(b: &mut Builder, w: f64, d: f64, h: f64) {
    let r = w.min(d) / 2.0;
    let dome = (r * 0.25).min(h * 0.15);
    b.lathe(
        "body",
        [0.0, 0.0, 0.0],
        &[
            (0.0, 0.0),
            (r, 0.0),
            (r, h - dome),
            (r * 0.6, h - dome * 0.2),
            (0.0, h),
        ],
        1.0,
        1.0,
    );
    // Its hoops.
    for k in 1..4 {
        let z = (h - dome) * k as f64 / 4.0;
        b.cone(
            "body",
            [0.0, 0.0, z - 0.015],
            [0.0, 0.0, z + 0.015],
            r * 1.015,
            r * 1.015,
            ROUND * 2,
        );
    }
    b.cone(
        "dark",
        [0.0, 0.0, h - dome * 0.3],
        [0.0, 0.0, h + 0.0],
        r * 0.28,
        r * 0.28,
        ROUND * 2,
    );
    b.cone(
        "pipe",
        [r * 0.95, 0.0, h * 0.85],
        [r * 1.0, 0.0, h * 0.85],
        0.03,
        0.03,
        ROUND,
    );
    b.cone(
        "pipe",
        [0.0, r * 0.95, 0.1],
        [0.0, r * 1.0, 0.1],
        0.03,
        0.03,
        ROUND,
    );
}

/// A booster set: two pumps standing on their frame, their manifolds and
/// the pressure vessel.
fn pump(b: &mut Builder, w: f64, d: f64, h: f64) {
    b.rbox(
        "metal",
        [-w / 2.0, -d / 2.0, 0.0],
        [w / 2.0, d / 2.0, 0.06],
        0.004,
    );
    let r = (d * 0.18).min(w * 0.12).max(0.04);
    let mut xs = vec![-w * 0.15, w * 0.1];
    xs.retain(|x| x.abs() + r < w / 2.0);
    for &x in &xs {
        b.cone("pipe", [x, 0.0, 0.06], [x, 0.0, h * 0.5], r, r, ROUND * 2);
        b.cone(
            "metal",
            [x, 0.0, h * 0.5],
            [x, 0.0, h * 0.95],
            r * 0.85,
            r * 0.8,
            ROUND * 2,
        );
        b.cone(
            "dark",
            [x, 0.0, h * 0.95],
            [x, 0.0, h],
            r * 0.5,
            r * 0.4,
            ROUND,
        );
    }
    let rp = r * 0.4;
    for (z, y) in [(h * 0.2, -d * 0.32), (h * 0.4, d * 0.32)] {
        b.cone(
            "pipe",
            [-w / 2.0 + 0.02, y, z],
            [w * 0.25, y, z],
            rp,
            rp,
            ROUND,
        );
        for &x in &xs {
            b.cone("pipe", [x, y, z], [x, 0.0, z], rp * 0.8, rp * 0.8, ROUND);
        }
    }
    let rv = (d * 0.3).min(w * 0.12).max(0.05);
    b.lathe(
        "red",
        [w / 2.0 - rv - 0.01, 0.0, 0.06],
        &[(0.0, 0.0), (rv, rv * 0.3), (rv, h * 0.55), (0.0, h * 0.7)],
        1.0,
        1.0,
    );
}

/// A water heater on the wall: the white cylinder, its thermostat cap and
/// the pipes down from it.
fn water_heater(b: &mut Builder, w: f64, d: f64, h: f64) {
    let back = -d / 2.0;
    let r = w.min(d) / 2.0;
    let pipes = (h * 0.1).min(0.1);
    b.lathe(
        "body",
        [0.0, back + r, pipes],
        &[
            (0.0, 0.0),
            (r * 0.85, 0.0),
            (r, r * 0.2),
            (r, h - pipes - r * 0.2),
            (r * 0.85, h - pipes),
            (0.0, h - pipes),
        ],
        1.0,
        1.0,
    );
    b.cone(
        "key",
        [0.0, back + r * 2.0 - 0.01, pipes + 0.15],
        [0.0, back + r * 2.0 + 0.01, pipes + 0.15],
        0.05,
        0.05,
        ROUND,
    );
    b.ellipsoid(
        "red",
        [0.0, back + r * 2.0 + 0.012, pipes + 0.15],
        [0.006, 0.003, 0.006],
    );
    for s in [-1.0, 1.0] {
        b.cone(
            "pipe",
            [s * r * 0.35, back + r, 0.0],
            [s * r * 0.35, back + r, pipes + 0.01],
            0.011,
            0.011,
            ROUND,
        );
    }
    // The hooks on the wall.
    b.rbox(
        "metal",
        [-r * 0.6, back, h * 0.85],
        [r * 0.6, back + 0.02, h * 0.88],
        0.0,
    );
}

// ---- Solar panels (H3) -----------------------------------------------------

/// A row of photovoltaic panels or thermal collectors in its box: the
/// modules across `x`, each tilted from the front (`+y`, low) up to the
/// back (high) to fill the box's height over its depth, on their frame;
/// flat when the box is low.
fn solar_row(b: &mut Builder, w: f64, d: f64, h: f64, thermal: bool) {
    let n = ((w / 1.13).round() as usize).max(1);
    let step = w / n as f64;
    let gap = 0.02;
    let thick = if thermal { 0.08 } else { 0.035 };
    let flat = h <= 0.05;
    let rise = if flat { 0.0 } else { (h - thick).max(0.0) };
    let run = d;
    let len = (rise * rise + run * run).sqrt().max(1e-6);
    // Along the slope, up from the front; and its normal.
    let up = [0.0, -run / len, rise / len];
    let normal = [0.0, rise / len, run / len];
    let base = thick / 2.0;
    for i in 0..n {
        let xc = -w / 2.0 + step * (i as f64 + 0.5);
        let half_w = step / 2.0 - gap / 2.0;
        let c = [xc, 0.0, base + rise / 2.0];
        let ax = [half_w, 0.0, 0.0];
        let ay = [0.0, up[1] * len / 2.0, up[2] * len / 2.0];
        let az = [0.0, normal[1] * thick / 2.0, normal[2] * thick / 2.0];
        // The frame and the face.
        b.obox("metal", c, ax, ay, az);
        let face = [
            c[0],
            c[1] + normal[1] * (thick / 2.0 + 0.001),
            c[2] + normal[2] * (thick / 2.0 + 0.001),
        ];
        let fa = [half_w - 0.02, 0.0, 0.0];
        let fy = [0.0, ay[1] * (1.0 - 0.04 / len), ay[2] * (1.0 - 0.04 / len)];
        let fz = [0.0, normal[1] * 0.001, normal[2] * 0.001];
        b.obox(if thermal { "glass" } else { "cell" }, face, fa, fy, fz);
        if !thermal {
            // The cells' lines across.
            for j in 1..6 {
                let t = j as f64 / 6.0 - 0.5;
                let p = [
                    face[0],
                    face[1] + ay[1] * 2.0 * t * 0.96 + normal[1] * 0.0015,
                    face[2] + ay[2] * 2.0 * t * 0.96 + normal[2] * 0.0015,
                ];
                b.obox(
                    "metal",
                    p,
                    [half_w - 0.02, 0.0, 0.0],
                    [0.0, up[1] * 0.004, up[2] * 0.004],
                    fz,
                );
            }
        }
        // Its legs at the back, up to its high edge.
        if !flat && rise > 0.1 {
            for s in [-1.0, 1.0] {
                let x = xc + s * (half_w - 0.08);
                let y = -d / 2.0 + 0.03;
                b.rbox(
                    "metal",
                    [x - 0.02, y - 0.02, 0.0],
                    [x + 0.02, y + 0.02, rise],
                    0.0,
                );
                b.rbox(
                    "metal",
                    [x - 0.02, -d / 2.0, 0.0],
                    [x + 0.02, d / 2.0, 0.04],
                    0.0,
                );
            }
        }
    }
    if thermal {
        // The header along the top edge.
        let z = base + rise + 0.03;
        b.cone(
            "pipe",
            [-w / 2.0, -d / 2.0 + 0.03, z.min(h)],
            [w / 2.0, -d / 2.0 + 0.03, z.min(h)],
            0.02,
            0.02,
            ROUND,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_device_has_its_model_in_its_box() {
        for kind in KINDS {
            let (w, d, h) = match kind {
                "light" => (0.2, 0.2, 0.05),
                "diffuser" => (0.6, 0.6, 0.05),
                "radiator" => (0.8, 0.1, 0.6),
                "grille" => (0.4, 0.05, 0.2),
                "panel" => (0.4, 0.12, 0.5),
                "equipment" => (0.45, 0.35, 0.75),
                "riser" => (0.04, 0.04, 3.0),
                "downpipe" => (0.11, 0.11, 3.0),
                "sprinkler" => (0.08, 0.08, 0.08),
                "hose_reel" => (0.7, 0.25, 0.7),
                "extinguisher" => (0.2, 0.18, 0.6),
                "smoke_detector" => (0.11, 0.11, 0.06),
                "call_point" => (0.09, 0.05, 0.09),
                "siren" => (0.12, 0.08, 0.12),
                "emergency_light" => (0.3, 0.06, 0.1),
                "exit_sign" => (0.32, 0.03, 0.16),
                "valve" => (0.08, 0.06, 0.08),
                "water_meter" => (0.4, 0.15, 0.3),
                "tank" => (1.0, 1.0, 1.5),
                "pump" => (0.9, 0.5, 0.8),
                "water_heater" => (0.45, 0.45, 0.9),
                "solar_panel" => (3.4, 1.6, 0.9),
                "solar_collector" => (2.2, 1.7, 1.0),
                _ => (0.08, 0.04, 0.08),
            };
            let m = device_model(kind, w, d, h).expect(kind);
            assert!(m.triangle_count() > 20, "{kind}: {}", m.triangle_count());
            for p in &m.positions {
                // Within the box, give or take the screws to the wall and
                // a key a few millimetres proud.
                assert!(
                    (p[0] as f64).abs() <= w / 2.0 + 0.05
                        && (p[1] as f64).abs() <= d / 2.0 + 0.05
                        && (p[2] as f64) >= -0.01
                        && (p[2] as f64) <= h + 0.01,
                    "{kind}: {p:?}"
                );
            }
            for (role, _) in &m.parts {
                assert!(ROLES.contains(role), "{kind}: {role}");
            }
        }
    }
}
