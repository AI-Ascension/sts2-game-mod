// SPDX-License-Identifier: MIT

use super::ActUnavailableReason;

/// Failure before an owned act snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActSourceError {
    /// No supported act registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for ActSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActSourceError {}

/// Sanitized failures while producing or reading act, encounter, and map-reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActReferenceError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The manifest does not inventory an act family.
    MissingFamily,
    /// The source advertises a family it cannot project.
    UnsupportedFamily,
    /// The source family is temporarily unavailable.
    UnavailableFamily,
    /// The family identity in the source snapshot is wrong.
    FamilyIdentityMismatch,
    /// Source and manifest family counts disagree.
    FamilyCountMismatch,
    /// A source definition is absent from the manifest.
    UnknownDefinition(String),
    /// A manifest definition is absent from the source snapshot.
    MissingDefinition(String),
    /// A source definition repeats an identity.
    DuplicateDefinition(String),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A definition exceeds the aggregate byte bound.
    DefinitionTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A typed reference is absent from the manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// A pool entry references an encounter absent from its act.
    UnknownEncounterReference {
        /// Owning act definition identity.
        act_id: String,
        /// Missing encounter identity.
        encounter_id: String,
    },
    /// An encounter or pool references a category absent from its act.
    UnknownRoomCategoryReference {
        /// Owning act definition identity.
        act_id: String,
        /// Missing category identity.
        category_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(ActUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for ActReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActReferenceError {}

pub(super) fn map_source_error(error: ActSourceError) -> ActReferenceError {
    match error {
        ActSourceError::NoActiveSource => ActReferenceError::NoActiveSource,
        ActSourceError::AccessDenied => ActReferenceError::SourceAccessDenied,
        ActSourceError::Malformed => ActReferenceError::MalformedSource,
    }
}
