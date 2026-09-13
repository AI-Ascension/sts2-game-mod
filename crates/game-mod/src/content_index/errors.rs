// SPDX-License-Identifier: MIT

use super::{CONTENT_INDEX_MAX_TEXT_BYTES, ContentFilterKind};

/// Sanitized local producer, query, and lookup failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentIndexError {
    /// The source has no active content index for this host/build.
    NoActiveContentSource,
    /// The source refused an index read.
    SourceAccessDenied,
    /// The source returned malformed bounded data.
    MalformedSource,
    /// Source snapshot did not bind to the requested manifest.
    ManifestMismatch,
    /// Source snapshot or query selected another locale.
    LocaleMismatch,
    /// A required source record was absent for a handled manifest definition.
    MissingDefinitionRecord {
        /// Family identity.
        entity_kind: String,
        /// Definition identity.
        namespaced_id: String,
    },
    /// Source provided a record for a family with no handled adapter.
    UnsupportedKindRecord {
        /// Family identity.
        entity_kind: String,
    },
    /// Source provided an ID absent from the manifest.
    UnknownDefinition {
        /// Family identity.
        entity_kind: String,
        /// Definition identity.
        namespaced_id: String,
    },
    /// Source repeated one family/ID pair.
    DuplicateDefinitionRecord,
    /// A caller selected a kind absent from the manifest.
    UnknownKind,
    /// A known family has no typed adapter.
    UnsupportedKind,
    /// A query requested an adapter-specific filter that cannot be evaluated.
    UnsupportedFilter(ContentFilterKind),
    /// A requested definition ID is not in a handled family.
    NotFound,
    /// A definition exists but visibility policy excludes it.
    ExcludedByScope,
    /// The exact lookup reference names another manifest.
    StaleReference,
    /// A continuation was malformed, consumed, cross-reader, or mismatched.
    InvalidContinuation,
    /// A query page size is zero or exceeds the local bound.
    InvalidPageSize,
    /// Exact lookup is unavailable for the selected family.
    DetailUnavailable,
    /// Exact lookup would exceed the aggregate detail byte bound.
    DetailTooLarge {
        /// Maximum aggregate bytes allowed.
        limit: usize,
        /// Measured aggregate bytes.
        actual: usize,
    },
    /// A source supplied more aliases than the bounded index accepts.
    AliasCountTooLarge {
        /// Maximum aliases allowed.
        limit: usize,
        /// Supplied alias count.
        actual: usize,
    },
    /// A definition carries more glossary IDs than the local bound.
    CollectionTooLarge {
        /// Collection identity.
        field: &'static str,
        /// Maximum accepted entries.
        limit: usize,
        /// Supplied entry count.
        actual: usize,
    },
    /// Adapter capabilities claim a field that the source did not provide.
    CapabilityMismatch(&'static str),
    /// An identity or locale field is empty, oversized, or contains controls.
    InvalidIdentity(&'static str),
    /// A searchable text value is oversized or contains controls.
    InvalidText,
    /// An alias is repeated within one definition.
    DuplicateAlias,
    /// A glossary term ID is repeated within one definition.
    DuplicateTermReference,
    /// Adapter configuration names an unknown manifest family.
    AdapterForUnknownKind,
    /// Adapter configuration repeats one family.
    DuplicateAdapter,
}

impl std::fmt::Display for ContentIndexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContentIndexError {}

pub(crate) fn validate_text(value: &str) -> Result<(), ContentIndexError> {
    if value.len() > CONTENT_INDEX_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(ContentIndexError::InvalidText);
    }
    Ok(())
}
