// SPDX-License-Identifier: MIT

//! Source-only reference text and public-screen text.
//!
//! This module copies the owner's inventoried non-gameplay reference text (tutorial explanations,
//! help material, lore, credits, and the public UI text a screen currently displays) and the
//! semantic description of supported public screens into an immutable catalog fenced by the
//! existing content-manifest cursor binding and an owner-local producer identity.  It exists so an
//! agent can search and read one reference without parsing the game, and can read what a blocking
//! tutorial or message says without dismissing it.
//!
//! Three invariants are deliberate:
//!
//! * a family or screen kind the producer cannot project is reported as explicit unsupported scope
//!   with the value that could not be supplied, never dropped, emptied, or replaced by an invented
//!   description, so "not authored" and "not projected" stay distinguishable;
//! * formatting is retained as inert structure.  Markup, non-Latin text, and long text are carried
//!   verbatim inside an explicit local bound, while executable presentation and text shaped as an
//!   instruction to its consumer are refused rather than repaired;
//! * reading is one-way.  There is no setter, no click, confirmation or dismissal, no private
//!   input, no live run mutation, no native extractor and no host compatibility claim.

mod catalog;
mod error;
mod hygiene;
mod listing;
mod model;
mod read;
mod screen;
mod validation;

pub use catalog::{
    ReferenceCatalogBinding, ReferenceTextCatalog, ReferenceTextCatalogProducer,
    ReferenceTextSnapshot, ReferenceTextSource,
};
pub use error::{ReferenceTextError, ReferenceTextSourceError};
pub use listing::{
    ReferenceContinuation, ReferenceCoverage, ReferenceFamilyCoverage, ReferenceListQuery,
    ReferencePage, ReferenceSummary,
};
pub use model::*;
pub use read::{
    PublicControlRead, PublicScreenRead, ReferenceCompleteness, ReferenceDocumentRead,
    ReferenceSectionRead, ScreenReadEffect,
};
pub use screen::*;

/// Owner-local producer identity for the source-only reference-text catalog.
pub const REFERENCE_TEXT_PRODUCER_VERSION: &str = "game-reference-text-producer-v1";
/// Maximum length of one identity token: entity ID, section ID, screen ID or revision.
pub const REFERENCE_TEXT_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum number of reference documents accepted from one catalog snapshot.
pub const REFERENCE_TEXT_MAX_DOCUMENTS: usize = 2048;
/// Maximum number of sections in one reference document.
pub const REFERENCE_TEXT_MAX_SECTIONS: usize = 128;
/// Maximum number of ordered segments in one text: title, section, or visible screen text.
pub const REFERENCE_TEXT_MAX_SEGMENTS: usize = 512;
/// Maximum number of searchable keywords declared by one document or section.
pub const REFERENCE_TEXT_MAX_KEYWORDS: usize = 64;
/// Maximum number of declared related references in one document.
pub const REFERENCE_TEXT_MAX_RELATED: usize = 128;
/// Maximum number of described controls on one public screen.
pub const REFERENCE_TEXT_MAX_CONTROLS: usize = 64;
/// Maximum number of public screens accepted from one catalog snapshot.
pub const REFERENCE_TEXT_MAX_SCREENS: usize = 64;
/// Maximum aggregate retained text bytes for one reference document.
pub const REFERENCE_TEXT_MAX_TEXT_BYTES: usize = 256 * 1024;
/// Maximum aggregate retained visible-text bytes for one public screen.
pub const REFERENCE_TEXT_MAX_VISIBLE_BYTES: usize = 64 * 1024;
/// Maximum page size for a family-partitioned reference listing.
pub const REFERENCE_TEXT_MAX_PAGE_ITEMS: usize = 64;
/// Default page size for a family-partitioned reference listing.
pub const REFERENCE_TEXT_DEFAULT_PAGE_ITEMS: usize = 16;
