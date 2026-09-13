// SPDX-License-Identifier: MIT

use super::model::GlossaryUnresolvedReason;

/// Failure before a bounded owned glossary snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlossarySourceError {
    /// No supported glossary source exists for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce an owned bounded snapshot.
    Malformed,
}

impl std::fmt::Display for GlossarySourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GlossarySourceError {}

/// Sanitized producer and query errors for the owner-local glossary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryCatalogError {
    /// No supported source exists for the selected host/build.
    NoActiveSource,
    /// The source refused a glossary read.
    SourceAccessDenied,
    /// The source returned malformed bounded data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot or query selected another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The source returned more terms than the local bound.
    TermCountTooLarge { limit: usize, actual: usize },
    /// Two source records share one term identity.
    DuplicateTerm(String),
    /// A source record has an identity that is absent from the manifest or glossary.
    UnknownContentReference {
        entity_kind: String,
        namespaced_id: String,
    },
    /// An input collection exceeds its local bound.
    CollectionTooLarge {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    /// An input identity or locale is empty, oversized, or contains controls.
    InvalidIdentity(&'static str),
    /// A searchable value is oversized or contains controls.
    InvalidText,
    /// A term carries the same alias more than once.
    DuplicateAlias,
    /// A term carries the same parameter placeholder more than once.
    DuplicateParameter,
    /// A term carries the same related-term ID more than once.
    DuplicateRelatedTerm,
    /// A term carries the same rule reference more than once.
    DuplicateRuleReference,
    /// A term carries the same content reference more than once.
    DuplicateContentReference,
    /// A detail response exceeds the aggregate local byte bound.
    DetailTooLarge { limit: usize, actual: usize },
    /// A requested page size is zero or exceeds the local bound.
    InvalidPageSize,
    /// A continuation is malformed, consumed, cross-reader, or mismatched.
    InvalidContinuation,
    /// An exact reference names another manifest/locale.
    StaleReference,
    /// A term exists but its visibility excludes the selected scope.
    ExcludedByScope,
    /// A term ID is absent from the glossary.
    NotFound,
    /// The query scope requests references that policy excludes.
    ReferenceScopeDisabled,
    /// An evidence tag is required for owner-authored documentation.
    MissingEvidenceTag,
    /// A source supplied an explicit unresolved reference with an invalid reason.
    InvalidUnresolvedReference(GlossaryUnresolvedReason),
}

impl std::fmt::Display for GlossaryCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GlossaryCatalogError {}

pub(super) fn map_source_error(error: GlossarySourceError) -> GlossaryCatalogError {
    match error {
        GlossarySourceError::NoActiveSource => GlossaryCatalogError::NoActiveSource,
        GlossarySourceError::AccessDenied => GlossaryCatalogError::SourceAccessDenied,
        GlossarySourceError::Malformed => GlossaryCatalogError::MalformedSource,
    }
}
