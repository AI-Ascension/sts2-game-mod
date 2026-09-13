// SPDX-License-Identifier: MIT

//! Owner-local immutable content inventory and bounded lookup.
//!
//! This module is a source-only producer slice. It binds typed metadata to an existing
//! [`ContentManifest`] and provides deterministic list, literal search, and exact definition
//! lookup before any transport or native-host integration is negotiated. The names and errors
//! here are deliberately game-mod-local; they are not a cross-repository wire contract.
//!
//! Search folds Unicode with locale-independent lowercase expansion and treats the query as a
//! literal substring. Exact display-name, display-name prefix, and display-name substring matches
//! rank before the corresponding alias tiers, which rank before rendered-description tiers; each
//! tier uses exact, prefix, then substring order. Empty search returns visible definitions in
//! identity order. Equal ranks always tie-break by entity kind and then namespaced definition ID.

mod engine;
mod errors;
mod helpers;
mod model;
mod query;
mod reader;
mod registry;
mod source;

pub use engine::ContentIndex;
pub use errors::ContentIndexError;
pub use model::{
    CONTENT_INDEX_MAX_ALIAS_COUNT, CONTENT_INDEX_MAX_DEFINITION_BYTES,
    CONTENT_INDEX_MAX_IDENTITY_BYTES, CONTENT_INDEX_MAX_PAGE_ITEMS,
    CONTENT_INDEX_MAX_TERM_REFERENCES, CONTENT_INDEX_MAX_TEXT_BYTES, ContentDefinition,
    ContentDefinitionReference, ContentDefinitionSummary, ContentDetailCapabilities,
    ContentIndexFamily, ContentIndexInputError, ContentIndexSnapshot, ContentIndexSource,
    ContentIndexSourceError, ContentQueryLocale, ContentRarity, ContentUnlockState,
};
pub use query::{
    ContentCompleteness, ContentContinuation, ContentFilterKind, ContentIndexDefinitionInput,
    ContentListPage, ContentListQuery, ContentQueryBinding, ContentQueryFilters,
    ContentQueryOperation, ContentQueryScope, ContentReferenceVisibilityPolicy, ContentSearchPage,
    ContentSearchQuery, ContentSearchResult,
};
pub use reader::ContentIndexReader;
pub use registry::{ContentKindAdapter, ContentKindAdapterRegistry};
pub use source::ContentIndexProducer;

/// Owner-local producer version. This is not a protocol version.
pub const CONTENT_INDEX_PRODUCER_VERSION: &str = "game-content-index-producer-v1";
