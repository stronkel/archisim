//! Team work on an ArchiSim project through a shared folder.
//!
//! Several people work on the same project from their own computers, each
//! on a copy of it, also offline. The project lives in a folder that a sync
//! service shares (Google Drive, Dropbox, a NAS): see [`folder`] for what is
//! in it. Nothing needs a server: every person writes only files under
//! their own name, and reads everybody else's.
//!
//! - **Publish**: my changes since I last received go into my changes
//!   file, element by element ([`diff`]).
//! - **Receive**: everybody's changes, applied in the same order on every
//!   computer, on top of the shared base; my unpublished changes stay on
//!   top.
//! - **Reservations**: an element (or a zone of a level) is modified by
//!   one person at a time; whoever reserved first keeps it.
//! - **Ids**: every person hands out new element ids from a block of their
//!   own, so two people adding elements at the same time never clash.
//!
//! Described for Antonio in `docs/EQUIPO.md`.

pub mod diff;
pub mod folder;
mod session;

pub use diff::Op;
pub use folder::{slug, Member, Message, Presence, TeamFile, Zone, EXTENSION, TEAM_FILE};
pub use session::{
    doc_of, inside, is_team_folder, members, model_of, team_folder_of, zones_overlap, Locator,
    NoZones, Refresh, Report, Session,
};

#[cfg(test)]
mod tests;
