//! ArchiSim core: the project document and its building elements.
//!
//! This crate has no knowledge of geometry kernels, file formats or the user
//! interface. Everything else in ArchiSim builds on it. Its only dependency
//! is the family language (`archisim-family`), which knows nothing of the
//! model.

pub mod document;
pub mod family;
pub mod ids;
pub mod math;
pub mod model;

pub use document::Document;
pub use family::{
    Evaluation, Family, FamilyDef, FamilyError, ParamKind, ParamScope, Part, Value, Values,
};
pub use ids::*;
pub use math::{is_simple_polygon, perimeter, signed_area, Point2, Vec2, EPSILON};
pub use model::*;
