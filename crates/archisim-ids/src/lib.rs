//! Checking IFC models against information requirements written in
//! buildingSMART's IDS 1.0 (Information Delivery Specification).
//!
//! An IDS lists specifications: which elements each one applies to
//! (applicability facets) and what they must have (requirement facets:
//! class and predefined type, attributes, properties and quantities,
//! materials, classifications, and what they are part of). [`check`] reads
//! any IFC2X3, IFC4 or IFC4X3 file through the STEP reader of
//! `archisim-ifc`, so it checks what ArchiSim exports exactly as another
//! program would receive it, and also files from other programs.
//!
//! The rules follow the IDS 1.0 implementers' documentation (values,
//! tolerances, units, inheritance from types); `tests/conformance.rs` runs
//! buildingSMART's own test cases when they are downloaded.

pub mod check;
pub mod model;
pub mod schema;
mod schema_data;
pub mod spec;

pub use check::{check, ElementFailure, IdsReport, SpecResult};
pub use spec::{parse_ids, Ids, IdsError};

/// Reads an IDS and an IFC file and checks one against the other.
pub fn check_texts(ids_xml: &str, ifc_text: &str) -> Result<IdsReport, String> {
    let ids = parse_ids(ids_xml).map_err(|e| e.to_string())?;
    let file = archisim_ifc::step_read::parse(ifc_text).map_err(|e| e.to_string())?;
    Ok(check(&ids, &file))
}
