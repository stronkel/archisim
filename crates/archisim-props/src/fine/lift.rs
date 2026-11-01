//! Lifts (H4): the whole lift of a building — its shaft with the pit and
//! the overrun, the landing doors at each stop, the guide rails, the
//! counterweight and the car — and the car alone, for a shaft drawn with
//! walls.
//!
//! Axes: the inside of the shaft centred on `x = 0`, `y = 0`, the landing
//! doors on its `+y` side; `z = 0` the floor of the lowest stop.
//!
//! Roles ([`ROLES`]): `shaft` the concrete of the shaft, `steel` frames,
//! doors and rails, `panel` the car's walls, `floor` its floor, `mirror`,
//! `glass` a push panel's, `light` the car's ceiling light, `dark` the
//! gaps and the pit's ladder.

use crate::build::{Builder, ROUND};
use crate::{Params, PropMesh};

/// The roles of a lift's model, in the order their colours are given.
pub const ROLES: [&str; 8] = [
    "shaft", "steel", "panel", "floor", "mirror", "glass", "light", "dark",
];

/// What a lift is made of: all lengths in metres.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftSpec {
    /// The inside of the shaft, across (`x`) and deep (`y`).
    pub shaft_w: f64,
    pub shaft_d: f64,
    /// Its walls' thickness; 0: the shaft is drawn with walls apart, and
    /// only the pit is built.
    pub wall: f64,
    /// Depth of the pit under the lowest stop.
    pub pit: f64,
    /// From the floor of the top stop to the shaft's ceiling.
    pub overrun: f64,
    /// Height of each stop over the lowest, from 0, going up.
    pub stops: Vec<f64>,
    /// The car's inside.
    pub car_w: f64,
    pub car_d: f64,
    pub car_h: f64,
    /// Clear width and height of the doors.
    pub door_w: f64,
    pub door_h: f64,
    /// The stop the car is at.
    pub car_stop: usize,
}

impl LiftSpec {
    pub fn top(&self) -> f64 {
        self.stops.last().copied().unwrap_or(0.0)
    }
}

/// The whole lift.
pub fn lift(spec: &LiftSpec) -> PropMesh {
    let mut b = Builder::default();
    let (hw, hd) = (spec.shaft_w / 2.0, spec.shaft_d / 2.0);
    let t = spec.wall.max(0.0);
    let top = spec.top() + spec.overrun;
    let z0 = -spec.pit;
    let door_hole = (spec.door_w / 2.0 + 0.06).min(hw);
    let hole_h = spec.door_h + 0.08;
    let pit_wall = if t > 0.0 { t } else { 0.2 };
    // The pit: its floor and, when the shaft is drawn apart, its own walls
    // down from the lowest floor.
    b.rbox(
        "shaft",
        [-hw - pit_wall, -hd - pit_wall, z0 - 0.3],
        [hw + pit_wall, hd + pit_wall, z0],
        0.0,
    );
    if t <= 0.0 && spec.pit > 0.0 {
        shaft_walls(&mut b, hw, hd, pit_wall, z0, 0.0, &[], door_hole, hole_h);
    }
    // The pit's ladder, on the side.
    if spec.pit > 0.4 {
        let x = hw - 0.08;
        for s in [-0.2, 0.2] {
            b.rbox(
                "dark",
                [x - 0.015, s - 0.015, z0],
                [x + 0.015, s + 0.015, 0.0],
                0.0,
            );
        }
        let mut z = z0 + 0.25;
        while z < -0.05 {
            b.cyl("dark", [x, -0.2, z], [x, 0.2, z], 0.01);
            z += 0.28;
        }
    }
    if t > 0.0 {
        shaft_walls(&mut b, hw, hd, t, z0, top, &spec.stops, door_hole, hole_h);
        // Its ceiling.
        b.rbox(
            "shaft",
            [-hw - t, -hd - t, top],
            [hw + t, hd + t, top + 0.2],
            0.0,
        );
    }
    // Landing doors: in the face of the shaft's front wall, outside.
    let face = hd + t;
    for &z in &spec.stops {
        landing_door(&mut b, face, z, spec.door_w, spec.door_h);
    }
    // Guide rails at the sides of the car, and the counterweight's.
    let rx = (spec.car_w / 2.0 + 0.09).min(hw - 0.03);
    let car_y = (hd - 0.05 - spec.car_d / 2.0).max(-hd + spec.car_d / 2.0);
    for s in [-1.0, 1.0] {
        let x = s * rx;
        b.rbox(
            "steel",
            [x - 0.008, car_y - 0.045, z0 + 0.1],
            [x + 0.008, car_y + 0.045, top - 0.2],
            0.0,
        );
        // Its blade, towards the car.
        let (a, c) = (x, x - s * 0.06);
        b.rbox(
            "steel",
            [a.min(c), car_y - 0.008, z0 + 0.1],
            [a.max(c), car_y + 0.008, top - 0.2],
            0.0,
        );
    }
    let car_z = spec.stops.get(spec.car_stop).copied().unwrap_or(0.0);
    // The counterweight goes down as the car goes up.
    let cw_z = (spec.top() - car_z).clamp(z0 + 0.3, (top - 2.6).max(z0 + 0.3));
    let cx = -hw + 0.09;
    if cx < -rx + 0.05 || spec.shaft_w - spec.car_w > 0.3 {
        b.rbox(
            "steel",
            [cx - 0.06, -0.35, cw_z],
            [cx + 0.06, 0.35, cw_z + 2.2],
            0.0,
        );
        b.rbox(
            "dark",
            [cx - 0.065, -0.3, cw_z + 0.2],
            [cx + 0.065, 0.3, cw_z + 2.0],
            0.0,
        );
    }
    // The car.
    car(
        &mut b,
        [0.0, car_y, car_z],
        spec.car_w,
        spec.car_d,
        spec.car_h,
        spec.door_w,
        spec.door_h,
    );
    // The ropes from the car's roof up to the machine under the ceiling.
    let cy = car_y;
    for s in [-0.08, 0.08] {
        b.cyl(
            "dark",
            [s, cy, car_z + spec.car_h + 0.25],
            [s, cy, top - 0.4],
            0.006,
        );
    }
    b.rbox(
        "steel",
        [-0.35, cy - 0.25, top - 0.45],
        [0.35, cy + 0.25, top - 0.05],
        0.02,
    );
    PropMesh {
        positions: b.positions,
        parts: b.parts.into_iter().collect(),
        soft: Vec::new(),
        fine: Vec::new(),
    }
}

/// The four walls of the shaft from `z0` to `z1`, the front one with the
/// openings of the landing doors at each stop.
#[allow(clippy::too_many_arguments)]
fn shaft_walls(
    b: &mut Builder,
    hw: f64,
    hd: f64,
    t: f64,
    z0: f64,
    z1: f64,
    stops: &[f64],
    door: f64,
    hole_h: f64,
) {
    if z1 <= z0 {
        return;
    }
    // Back and sides.
    b.rbox("shaft", [-hw - t, -hd - t, z0], [hw + t, -hd, z1], 0.0);
    b.rbox("shaft", [-hw - t, -hd, z0], [-hw, hd, z1], 0.0);
    b.rbox("shaft", [hw, -hd, z0], [hw + t, hd, z1], 0.0);
    // The front, beside the doors and between them.
    b.rbox("shaft", [-hw - t, hd, z0], [-door, hd + t, z1], 0.0);
    b.rbox("shaft", [door, hd, z0], [hw + t, hd + t, z1], 0.0);
    let mut z = z0;
    let mut holes: Vec<f64> = stops
        .iter()
        .copied()
        .filter(|s| *s >= z0 && *s < z1)
        .collect();
    holes.sort_by(f64::total_cmp);
    for h in holes {
        if h > z + 1e-6 {
            b.rbox("shaft", [-door, hd, z], [door, hd + t, h], 0.0);
        }
        z = (h + hole_h).min(z1);
    }
    if z1 > z + 1e-6 {
        b.rbox("shaft", [-door, hd, z], [door, hd + t, z1], 0.0);
    }
}

/// A landing door on the face `y = face`, its sill at `z`: frame, sill,
/// two leaves closed and the call button.
fn landing_door(b: &mut Builder, face: f64, z: f64, w: f64, h: f64) {
    let f = 0.05;
    let hw = w / 2.0;
    let y0 = face;
    let y1 = face + 0.04;
    b.rbox("steel", [-hw - f, y0, z], [-hw, y1, z + h + f], 0.0);
    b.rbox("steel", [hw, y0, z], [hw + f, y1, z + h + f], 0.0);
    b.rbox("steel", [-hw, y0, z + h], [hw, y1, z + h + f], 0.0);
    b.rbox("steel", [-hw - f, y0 - 0.1, z - 0.01], [hw + f, y1, z], 0.0);
    // The leaves, a little behind the frame; the gap where they meet.
    b.rbox(
        "steel",
        [-hw, y0 - 0.03, z],
        [-0.003, y0 - 0.005, z + h],
        0.0,
    );
    b.rbox("steel", [0.003, y0 - 0.05, z], [hw, y0 - 0.025, z + h], 0.0);
    b.rbox(
        "dark",
        [-0.003, y0 - 0.05, z],
        [0.003, y0 - 0.005, z + h],
        0.0,
    );
    // The call button beside it.
    let x = hw + f + 0.12;
    b.rbox(
        "steel",
        [x - 0.04, y0, z + 1.0],
        [x + 0.04, y0 + 0.01, z + 1.18],
        0.004,
    );
    b.cone(
        "glass",
        [x, y0 + 0.01, z + 1.09],
        [x, y0 + 0.016, z + 1.09],
        0.014,
        0.012,
        ROUND,
    );
}

/// The car, its centre in plan at `at` and its floor at `at.z`: floor, walls, front with its doors,
/// ceiling with the light, handrail and mirror, the operating panel.
fn car(b: &mut Builder, at: [f64; 3], w: f64, d: f64, h: f64, door_w: f64, door_h: f64) {
    let [x, y, z] = at;
    let (hw, hd) = (w / 2.0, d / 2.0);
    let t = 0.03;
    // Floor, ceiling and the frame round the car.
    b.rbox(
        "floor",
        [x - hw - t, y - hd - t, z - 0.1],
        [x + hw + t, y + hd + t, z],
        0.0,
    );
    b.rbox(
        "panel",
        [x - hw - t, y - hd - t, z + h],
        [x + hw + t, y + hd + t, z + h + 0.08],
        0.0,
    );
    b.rbox(
        "light",
        [x - hw * 0.6, y - hd * 0.6, z + h - 0.01],
        [x + hw * 0.6, y + hd * 0.6, z + h],
        0.0,
    );
    b.rbox(
        "steel",
        [x - hw - t, y - 0.06, z + h + 0.08],
        [x + hw + t, y + 0.06, z + h + 0.25],
        0.0,
    );
    // The walls: back and sides.
    b.rbox(
        "panel",
        [x - hw - t, y - hd - t, z],
        [x + hw + t, y - hd, z + h],
        0.0,
    );
    b.rbox(
        "panel",
        [x - hw - t, y - hd, z],
        [x - hw, y + hd, z + h],
        0.0,
    );
    b.rbox(
        "panel",
        [x + hw, y - hd, z],
        [x + hw + t, y + hd, z + h],
        0.0,
    );
    // The front: beside and over the door, and the door's leaves shut.
    let dw = (door_w / 2.0).min(hw);
    let dh = door_h.min(h);
    b.rbox(
        "steel",
        [x - hw - t, y + hd, z],
        [x - dw, y + hd + t, z + h],
        0.0,
    );
    b.rbox(
        "steel",
        [x + dw, y + hd, z],
        [x + hw + t, y + hd + t, z + h],
        0.0,
    );
    if h > dh + 1e-6 {
        b.rbox(
            "steel",
            [x - dw, y + hd, z + dh],
            [x + dw, y + hd + t, z + h],
            0.0,
        );
    }
    b.rbox(
        "steel",
        [x - dw, y + hd + 0.005, z],
        [x - 0.003, y + hd + 0.02, z + dh],
        0.0,
    );
    b.rbox(
        "steel",
        [x + 0.003, y + hd + 0.02, z],
        [x + dw, y + hd + 0.035, z + dh],
        0.0,
    );
    // The mirror on the back above the handrail, the handrail.
    b.rbox(
        "mirror",
        [x - hw * 0.8, y - hd, z + 1.0],
        [x + hw * 0.8, y - hd + 0.006, z + h - 0.15],
        0.0,
    );
    b.cyl(
        "steel",
        [x - hw * 0.85, y - hd + 0.05, z + 0.9],
        [x + hw * 0.85, y - hd + 0.05, z + 0.9],
        0.02,
    );
    for s in [-1.0, 1.0] {
        b.cyl(
            "steel",
            [x + s * hw * 0.8, y - hd, z + 0.9],
            [x + s * hw * 0.8, y - hd + 0.05, z + 0.9],
            0.012,
        );
    }
    // The operating panel on the side wall by the door.
    b.rbox(
        "steel",
        [x + hw - 0.006, y + hd - 0.25, z + 0.85],
        [x + hw, y + hd - 0.1, z + 1.25],
        0.0,
    );
    for k in 0..4 {
        let zz = z + 0.92 + 0.08 * k as f64;
        b.cone(
            "glass",
            [x + hw - 0.006, y + hd - 0.175, zz],
            [x + hw - 0.012, y + hd - 0.175, zz],
            0.012,
            0.011,
            ROUND,
        );
    }
}

/// The car alone (the «Cabina de ascensor» family): the car's inside
/// `width` × `depth` × `height`, its door `door_width` × `door_height`,
/// its back on `y = 0` (the shaft's wall) and its doors at `+y`, its
/// floor 10 cm over the floor it stands on.
pub fn lift_car(b: &mut Builder, p: &Params) {
    let w = p.get("width", 1.1).clamp(0.8, 2.5);
    let d = p.get("depth", 1.4).clamp(0.8, 3.0);
    let h = p.get("height", 2.2).clamp(2.0, 3.0);
    let dw = p.get("door_width", 0.9).clamp(0.6, w);
    let dh = p.get("door_height", 2.0).clamp(1.8, h);
    car(b, [0.0, d / 2.0 + 0.03, 0.1], w, d, h, dw, dh);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lift_stands_in_its_shaft() {
        let spec = LiftSpec {
            shaft_w: 1.65,
            shaft_d: 1.75,
            wall: 0.2,
            pit: 1.2,
            overrun: 3.6,
            stops: vec![0.0, 3.2, 6.4],
            car_w: 1.1,
            car_d: 1.4,
            car_h: 2.2,
            door_w: 0.9,
            door_h: 2.0,
            car_stop: 1,
        };
        let m = lift(&spec);
        assert!(m.triangle_count() > 300);
        for p in &m.positions {
            assert!((p[0] as f64).abs() <= 1.65 / 2.0 + 0.2 + 0.4, "{p:?}");
            assert!(p[2] as f64 >= -1.5 - 1e-6 && p[2] as f64 <= 6.4 + 3.6 + 0.2 + 1e-6);
        }
        for (role, _) in &m.parts {
            assert!(ROLES.contains(role), "{role}");
        }
        let mut b = Builder::default();
        lift_car(&mut b, &Params::default());
        assert!(b.triangle_count() > 100);
    }
}
