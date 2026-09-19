// SPDX-License-Identifier: MIT

//! Source-only locale-qualified rendered game text, fallback chains, and text provenance.
//!
//! This module copies the owner registry's ordered supported locales, each locale's explicit
//! fallback chain and text direction, and the owner-supplied rendered text for allowlisted
//! `(entity_kind, namespaced_id)` references into an immutable catalog fenced by the existing
//! content cursor binding and the producer identity.  It exists so an agent can ask for one
//! reference in one language and receive the requested locale, the effective locale, the exact
//! fallback chain that was consulted, the text revision and an explicit completeness report.
//!
//! Four invariants are deliberate:
//!
//! * a definition's stable identity stays exactly the same string in every language, so a
//!   localized answer never changes which entity it describes;
//! * a missing translation is never replaced by an empty string, a zero or an invented
//!   description, and an unresolved placeholder stays explicit together with the input it needs;
//! * an effect amount is carried verbatim, so localization can never silently change a number;
//! * reading is one-way.  There is no setter, no locale switch, no profile or configuration
//!   mutation, no live run read, no native extractor and no host compatibility claim.

mod catalog;
mod error;
mod listing;
mod model;
mod normalize;
mod render;
mod validation;

pub use catalog::{
    LocaleCatalog, LocaleCatalogBinding, LocaleCatalogProducer, LocaleCatalogSnapshot,
    LocaleContinuation, LocaleEntryListQuery, LocaleEntryPage, LocaleEntrySummary,
    LocaleRenderRequest, LocaleRenderSource, LocaleRenderedText, LocaleTextReference,
};
pub use error::{LocaleCatalogError, LocaleSourceError};
pub use model::*;

/// Owner-local producer identity for the source-only locale reference catalog.
pub const LOCALE_REFERENCE_PRODUCER_VERSION: &str = "game-locale-reference-producer-v1";
/// Maximum length of one identity token: locale tag, entity ID, revision or placeholder name.
pub const LOCALE_REFERENCE_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum number of supported locales carried by one catalog.
pub const LOCALE_REFERENCE_MAX_LOCALES: usize = 64;
/// Maximum length of one explicit fallback chain, including the requested locale.
pub const LOCALE_REFERENCE_MAX_FALLBACK_DEPTH: usize = 8;
/// Maximum aggregate rendered-text bytes retained for one entry in one locale.
pub const LOCALE_REFERENCE_MAX_TEXT_BYTES: usize = 64 * 1024;
/// Maximum number of ordered segments in one rendered text.
pub const LOCALE_REFERENCE_MAX_SEGMENTS: usize = 64;
/// Maximum number of declared placeholder names in one rendered text.
pub const LOCALE_REFERENCE_MAX_PLACEHOLDERS: usize = 32;
/// Maximum number of source entries accepted from one catalog snapshot.
pub const LOCALE_REFERENCE_MAX_ENTRIES: usize = 4096;
/// Maximum number of locale/plural variants retained for one entity reference.
pub const LOCALE_REFERENCE_MAX_VARIANTS: usize =
    LOCALE_REFERENCE_MAX_LOCALES * LocalePluralCategory::COUNT;
/// Maximum page size for a locale-partitioned entry listing.
pub const LOCALE_REFERENCE_MAX_PAGE_ITEMS: usize = 64;
/// Default page size for a locale-partitioned entry listing.
pub const LOCALE_REFERENCE_DEFAULT_PAGE_ITEMS: usize = 16;
