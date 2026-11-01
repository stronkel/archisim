//! Lifts in 3D (H4): the model of [`archisim_core::props::fine::lift`]
//! placed in the shaft, its pieces by role. The shaft's concrete is its
//! material (or a concrete grey); the rest are marked with
//! [`ELEVATOR_ROLE_BASE`] for [`crate::solid::solid_color`].

use archisim_core::{ElevatorId, MaterialId, Model};

use crate::solid::{Solid, SolidOwner, P3};

/// Materials from here up are the roles of a lift's model.
pub const ELEVATOR_ROLE_BASE: u64 = u64::MAX - 127;

/// The colour of a role of a lift's model.
pub fn role_color(role: usize) -> [u8; 3] {
    match archisim_core::props::fine::lift::ROLES.get(role).copied() {
        Some("shaft") => [190, 188, 182],
        Some("steel") => [176, 180, 184],
        Some("panel") => [214, 206, 192],
        Some("floor") => [92, 90, 88],
        Some("mirror") => [196, 210, 216],
        Some("glass") => [240, 240, 236],
        Some("light") => [255, 246, 220],
        Some("dark") => [52, 52, 56],
        _ => [200, 200, 200],
    }
}

/// All its pieces, in place.
pub fn elevator_solids(model: &Model, id: ElevatorId) -> Vec<Solid> {
    let (Some(e), Some(spec)) = (model.elevator(id), model.elevator_spec(id)) else {
        return Vec::new();
    };
    let mesh = archisim_core::props::fine::lift::lift(&spec);
    let z = model.level(e.level).map_or(0.0, |l| l.elevation);
    let (c, s) = (e.rotation.cos(), e.rotation.sin());
    let at = |i: u32| -> P3 {
        let p = mesh.positions[i as usize];
        let (x, y) = (p[0] as f64, p[1] as f64);
        [
            e.position.x + x * c - y * s,
            e.position.y + x * s + y * c,
            z + p[2] as f64,
        ]
    };
    mesh.parts
        .iter()
        .filter_map(|(role, tris)| {
            let k = archisim_core::props::fine::lift::ROLES
                .iter()
                .position(|r| r == role)?;
            let material = match (*role, e.material) {
                ("shaft", Some(m)) => m,
                _ => MaterialId(ELEVATOR_ROLE_BASE + k as u64),
            };
            Some(Solid {
                owner: SolidOwner::Elevator(id),
                material,
                triangles: tris
                    .iter()
                    .map(|t| [at(t[0]), at(t[1]), at(t[2])])
                    .collect(),
            })
        })
        .collect()
}

/// Whether a solid of a lift is its shaft.
pub fn is_shaft(model: &Model, s: &Solid) -> bool {
    let SolidOwner::Elevator(id) = s.owner else {
        return false;
    };
    let shaft = archisim_core::props::fine::lift::ROLES
        .iter()
        .position(|r| *r == "shaft")
        .unwrap_or(0) as u64;
    s.material.0 == ELEVATOR_ROLE_BASE + shaft
        || model
            .elevator(id)
            .and_then(|e| e.material)
            .is_some_and(|m| m == s.material)
}
