//! The 3D view: the whole building as IFC would show it, drawn with the
//! graphics card. The geometry comes from `archisim_geom::solid`, the same
//! shapes the IFC export writes.

pub mod camera;
pub mod render;
pub mod scene;

#[cfg(test)]
mod gpu_tests;
