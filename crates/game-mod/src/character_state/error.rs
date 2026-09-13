// SPDX-License-Identifier: MIT

/// Failure before an owned character-state catalog or live snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterStateSourceError {
    /// No active supported source exists.
    NoActiveSource,
    /// The selected source denied the read.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for CharacterStateSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CharacterStateSourceError {}

/// Sanitized failures while producing or querying static character-state definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterStateCatalogError {
    /// Source access failed before an owned snapshot was available.
    NoActiveSource,
    /// Source access was denied.
    SourceAccessDenied,
    /// Source returned malformed data.
    MalformedSource,
    /// Manifest cursor identity disagreed with the source snapshot.
    ManifestMismatch,
    /// Snapshot locale disagreed with the manifest.
    LocaleMismatch,
    /// Producer identity was not supported.
    ProducerVersionMismatch,
    /// No explicit coverage record exists for the selected character and mode.
    MissingCoverage {
        character_id: String,
        mode_id: String,
    },
    /// A coverage record is duplicated.
    DuplicateCoverage {
        character_id: String,
        mode_id: String,
    },
    /// A static definition is duplicated.
    DuplicateDefinition(String),
    /// A definition does not correspond to explicit mechanic coverage.
    DefinitionWithoutCoverage(String),
    /// A handled mechanic has no definition record.
    MissingDefinition(String),
    /// A source-owned value failed validation.
    InvalidInput(&'static str),
    /// A static definition exceeded the local byte bound.
    DefinitionTooLarge { limit: usize, actual: usize },
    /// A page size was zero or exceeded the local bound.
    InvalidPageSize,
    /// A continuation belonged to another reader or was already consumed.
    InvalidContinuation,
    /// A definition was excluded by visibility scope.
    ExcludedByScope,
    /// A definition ID was not found.
    NotFound,
    /// A reference points to a previous manifest, locale, or producer.
    StaleReference,
    /// The selected character/mode explicitly has no supported mechanic.
    UnsupportedMechanic,
    /// The selected character/mode does not apply the mechanic.
    NotApplicableMechanic,
    /// The selected source is currently unavailable.
    UnavailableMechanic,
    /// The selected source could not classify mechanic support.
    UnknownMechanic,
}

impl std::fmt::Display for CharacterStateCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CharacterStateCatalogError {}

/// Sanitized failures while validating or reading live character state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterStateLiveError {
    /// Source access failed before an owned snapshot was available.
    NoActiveSource,
    /// Source access was denied.
    SourceAccessDenied,
    /// Source returned malformed data.
    MalformedSource,
    /// Snapshot catalog identity disagreed with the selected catalog.
    CatalogMismatch,
    /// Snapshot game identity disagreed with the reader.
    GameInstanceMismatch,
    /// Snapshot run identity disagreed with the reader.
    RunMismatch,
    /// Snapshot mode identity disagreed with the reader.
    ModeMismatch,
    /// The selected character/mode explicitly has no supported resource extractor.
    UnsupportedResources,
    /// The selected character/mode has no resource mechanic.
    ResourcesNotApplicable,
    /// The selected resource source is unavailable.
    ResourcesUnavailable,
    /// Resource support could not be classified.
    ResourcesUnknown,
    /// The selected character/mode explicitly has no supported entity extractor.
    UnsupportedSecondaryEntities,
    /// The selected character/mode has no secondary-entity mechanic.
    SecondaryEntitiesNotApplicable,
    /// The selected secondary-entity source is unavailable.
    SecondaryEntitiesUnavailable,
    /// Secondary-entity support could not be classified.
    SecondaryEntitiesUnknown,
    /// Producer identity was not supported.
    ProducerVersionMismatch,
    /// Live binding identity was malformed.
    InvalidBinding(&'static str),
    /// No explicit coverage record exists for the selected character and mode.
    MissingCoverage {
        character_id: String,
        mode_id: String,
    },
    /// Live input or nested value was malformed.
    InvalidInput(&'static str),
    /// A snapshot repeated one live instance ID.
    DuplicateInstance(String),
    /// A live resource references an unknown definition.
    UnknownResourceDefinition(String),
    /// A live secondary entity references an unknown definition.
    UnknownSecondaryEntityDefinition(String),
    /// A live value does not match its static definition.
    ValueShapeMismatch(&'static str),
    /// A resource value exceeds its static maximum.
    ValueOutOfRange(&'static str),
    /// An ordered slot was duplicated or out of order.
    InvalidSlotOrder,
    /// A status or intent is not visible in the selected scope.
    VisibilityDenied(&'static str),
    /// A live detail exceeded the local byte bound.
    DetailTooLarge { limit: usize, actual: usize },
    /// A reference points to a previous snapshot identity.
    StaleReference,
    /// A snapshot epoch did not advance.
    NonMonotonicEpoch { current: u64, supplied: u64 },
    /// A live instance reference no longer exists.
    NotFound,
}

impl std::fmt::Display for CharacterStateLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CharacterStateLiveError {}

pub(super) fn map_source_error(error: CharacterStateSourceError) -> CharacterStateCatalogError {
    match error {
        CharacterStateSourceError::NoActiveSource => CharacterStateCatalogError::NoActiveSource,
        CharacterStateSourceError::AccessDenied => CharacterStateCatalogError::SourceAccessDenied,
        CharacterStateSourceError::Malformed => CharacterStateCatalogError::MalformedSource,
    }
}

pub(super) fn map_live_source_error(error: CharacterStateSourceError) -> CharacterStateLiveError {
    match error {
        CharacterStateSourceError::NoActiveSource => CharacterStateLiveError::NoActiveSource,
        CharacterStateSourceError::AccessDenied => CharacterStateLiveError::SourceAccessDenied,
        CharacterStateSourceError::Malformed => CharacterStateLiveError::MalformedSource,
    }
}
