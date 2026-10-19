//! Coordination with the other disciplines (phase 4 of the roadmap):
//!
//! - [`elements`]: any IFC file (structure, installations, another
//!   architect's model) read as elements with their bodies as triangles,
//!   their openings and a fingerprint of their data. That is what a linked
//!   model shows in ArchiSim, without importing it.
//! - [`versions`]: what changed between two versions of a file, matched by
//!   `GlobalId`.
//! - [`clash`]: clash detection between two sets of elements, with a
//!   tolerance, ignoring contact and openings left for the purpose.
//! - [`issues`]: clashes and requirement checks as BCF topics.

pub mod clash;
pub mod elements;
pub mod issues;
pub mod mesh;
pub mod versions;

pub use clash::{find_clashes, Clash, ClashOptions};
pub use elements::{elements_of, read_elements, Discipline, Element, ModelElements};
pub use issues::{clashes_to_bcf, ids_to_bcf, Author};
pub use versions::{compare, Change, VersionReport};

#[cfg(test)]
mod tests;
