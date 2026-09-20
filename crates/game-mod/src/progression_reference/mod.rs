// SPDX-License-Identifier: MIT

//! Source-only profile progression, unlocks, achievements, discoveries and records.
//!
//! This module copies an explicitly permitted opaque profile's progression state into an immutable
//! catalog fenced by the existing content manifest, locale, profile identity and profile revision.
//! It inventories the progression domains the supported build reports and states, per domain, per
//! field and per record, what the source could and could not project.
//!
//! The distinctions the rest of the boundary depends on are preserved rather than flattened: a
//! locked requirement is not an unlock, an entry the profile has never encountered is not a locked
//! one, a value the build does not track is never reported as zero, an entry the source could not
//! classify is never reported as undiscovered, and a partial page is never labelled complete.  The
//! slice is read-only by construction: it exposes no unlock, purchase, save or profile-selection
//! entry point, so a progression read cannot change state, and reading a profile never switches the
//! active one.  Account and private identifiers are refused rather than published beside game
//! progression, and an account-scoped domain is disclosed as unsupported instead of being reported
//! as an empty progression.  No native progression read, transport route or host compatibility is
//! claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{
    ProgressionCatalogProducer, ProgressionCatalogSnapshot, ProgressionReadPort,
    UnavailableProgressionHost,
};
pub use catalog_reader::*;
pub use definition::*;
pub use error::{
    ProgressionReadAuthority, ProgressionReadAvailability, ProgressionReferenceError,
    ProgressionSourceError,
};
pub use model::*;
