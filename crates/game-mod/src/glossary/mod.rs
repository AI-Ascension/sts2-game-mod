// SPDX-License-Identifier: MIT

//! Source-only mechanics terminology and cross-reference inventory.
//!
//! The glossary binds localized term records to one [`crate::ContentManifest`] and keeps
//! cross-references shallow. A related term is represented as one bounded edge; the reader never
//! recursively expands a graph, so cycles remain finite and deterministic. This module does not
//! define a wire contract, a native registry adapter, or a live-host claim.

mod catalog;
mod error;
mod model;
mod producer;
mod projection;
mod query;
mod reader;
mod validation;

pub use catalog::{
    GlossaryCatalog, GlossaryContentReference, GlossaryContentReferenceResolution,
    GlossaryRelatedTerm, GlossaryRelatedTermResolution, GlossaryTerm, GlossaryTermDetail,
    GlossaryTermReference, GlossaryTermSummary,
};
pub use error::{GlossaryCatalogError, GlossarySourceError};
pub use model::{
    GlossaryCatalogBinding, GlossaryContentReferenceInput, GlossaryContentReferenceSurface,
    GlossaryDefinitionText, GlossaryEvidence, GlossaryEvidenceKind, GlossaryQueryScope,
    GlossaryReferenceVisibilityPolicy, GlossaryTermInput, GlossaryTermVisibility,
    GlossaryUnresolvedReason,
};
pub use producer::{
    GlossaryCoverage, GlossaryCoverageStatus, GlossaryProducer, GlossarySnapshot, GlossarySource,
    GlossaryUnresolvedContentReference, GlossaryUnresolvedTermReference,
};
pub use query::{
    GlossaryCompleteness, GlossaryContinuation, GlossaryListPage, GlossaryListQuery,
    GlossaryQueryBinding, GlossaryQueryOperation, GlossarySearchPage, GlossarySearchQuery,
    GlossarySearchResult,
};
pub use reader::GlossaryReader;

/// Producer identity for this owner-local source slice.
///
/// This is not a protocol, gateway, MCP, or native ABI version.
pub const GLOSSARY_PRODUCER_VERSION: &str = "game-mechanics-glossary-producer-v1";
/// Maximum bytes accepted for one glossary identity.
pub const GLOSSARY_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized glossary value.
pub const GLOSSARY_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aliases retained on one term.
pub const GLOSSARY_MAX_ALIAS_COUNT: usize = 64;
/// Maximum parameter placeholders retained on one term.
pub const GLOSSARY_MAX_PARAMETER_COUNT: usize = 32;
/// Maximum related-term edges retained on one term.
pub const GLOSSARY_MAX_RELATED_TERM_COUNT: usize = 64;
/// Maximum rule references retained on one term.
pub const GLOSSARY_MAX_RULE_REFERENCE_COUNT: usize = 64;
/// Maximum content references retained on one term.
pub const GLOSSARY_MAX_CONTENT_REFERENCE_COUNT: usize = 64;
/// Maximum terms retained in one immutable glossary snapshot.
pub const GLOSSARY_MAX_TERM_COUNT: usize = 4096;
/// Maximum entries returned by one page.
pub const GLOSSARY_MAX_PAGE_ITEMS: usize = 64;
/// Maximum aggregate bytes returned by one exact term lookup.
pub const GLOSSARY_MAX_DETAIL_BYTES: usize = 64 * 1024;
/// Maximum unresolved references retained in coverage diagnostics.
pub const GLOSSARY_MAX_COVERAGE_REFERENCES: usize = 4096;
