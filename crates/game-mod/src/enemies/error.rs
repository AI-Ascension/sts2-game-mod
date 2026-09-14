// SPDX-License-Identifier: MIT

/// Failure before an owned enemy snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemySourceError {
    /// No supported enemy registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for EnemySourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EnemySourceError {}

/// Sanitized failures while producing or reading enemy reference definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyCatalogError {
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
    /// The manifest does not inventory an enemy family.
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
    /// An origin variant refers to a package absent from the active manifest.
    UnknownOriginPackage(String),
    /// A typed status, encounter, or content reference is absent from the manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// A phase referenced by a move or transition is absent from this enemy.
    UnknownPhaseReference {
        /// Owning enemy identity when known.
        enemy_id: String,
        /// Missing phase identity.
        phase_id: String,
    },
    /// A move referenced by a phase or origin variant is absent from this enemy.
    UnknownMoveReference {
        /// Owning enemy identity.
        enemy_id: String,
        /// Missing move identity.
        move_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition or move is hidden by the selected visibility scope.
    ExcludedByScope,
    /// No definition or move has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for EnemyCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EnemyCatalogError {}

pub(super) fn map_source_error(error: EnemySourceError) -> EnemyCatalogError {
    match error {
        EnemySourceError::NoActiveSource => EnemyCatalogError::NoActiveSource,
        EnemySourceError::AccessDenied => EnemyCatalogError::SourceAccessDenied,
        EnemySourceError::Malformed => EnemyCatalogError::MalformedSource,
    }
}
