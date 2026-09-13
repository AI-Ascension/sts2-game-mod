// SPDX-License-Identifier: MIT

/// Failure before an owned character snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterSourceError {
    /// No supported character registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for CharacterSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CharacterSourceError {}

/// Sanitized failures while producing or reading character definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterCatalogError {
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
    /// The manifest does not inventory a character family.
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
    DefinitionTooLarge { limit: usize, actual: usize },
    /// A source-owned package provenance disagrees with the manifest.
    OriginMismatch(String),
    /// A loadout references a definition absent from the manifest.
    UnknownLoadoutReference {
        entity_kind: String,
        namespaced_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for CharacterCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CharacterCatalogError {}

pub(super) fn map_source_error(error: CharacterSourceError) -> CharacterCatalogError {
    match error {
        CharacterSourceError::NoActiveSource => CharacterCatalogError::NoActiveSource,
        CharacterSourceError::AccessDenied => CharacterCatalogError::SourceAccessDenied,
        CharacterSourceError::Malformed => CharacterCatalogError::MalformedSource,
    }
}
