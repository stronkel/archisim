//! BCF (BIM Collaboration Format) of buildingSMART: issues with their
//! comments and saved views, exchanged with Revit, ArchiCAD, Solibri,
//! BIMcollab and other programs. Reads 2.1 and 3.0; writes 3.0 (or 2.1 for
//! older programs).
//!
//! The crate knows nothing of the model: elements are named by their IFC
//! `GlobalId`, cameras are in metres in the IFC coordinate system.

pub mod model;
pub mod read;
pub mod write;
pub mod zip;

pub use model::{
    iso_utc, new_guid, now, Bcf, BcfVersion, Camera, ClippingPlane, Comment, Extensions, FileRef,
    Line, Topic, Viewpoint,
};
pub use read::{read_bcf, BcfError};
pub use write::write_bcf;

#[cfg(test)]
mod tests;
