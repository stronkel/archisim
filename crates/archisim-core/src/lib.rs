//! ArchiSim core: the project document and its building elements.
//!
//! This crate has no knowledge of geometry kernels, file formats or the user
//! interface. Everything else in ArchiSim builds on it.

pub mod document;
pub mod ids;
pub mod math;
pub mod model;

pub use document::Document;
pub use ids::*;
pub use math::{Point2, Vec2, EPSILON};
pub use model::*;
