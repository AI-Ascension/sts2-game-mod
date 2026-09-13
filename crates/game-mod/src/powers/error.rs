// SPDX-License-Identifier: MIT

/// Failure before an owned power/status catalog or live snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerStatusSourceError {
    /// No active supported source exists.
    NoActiveSource,
    /// The selected source denied the read.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for PowerStatusSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PowerStatusSourceError {}

/// Sanitized failures while producing or querying static definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusCatalogError {
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
    /// Manifest does not contain the expected family.
    MissingFamily,
    /// Source explicitly reports unsupported family.
    UnsupportedFamily,
    /// Source explicitly reports unavailable family.
    UnavailableFamily,
    /// Source family identity was wrong.
    FamilyIdentityMismatch,
    /// Source/manifest family counts disagree.
    FamilyCountMismatch,
    /// Manifest definition has no source record.
    MissingDefinition(String),
    /// Source returned an ID absent from the manifest.
    UnknownDefinition(String),
    /// Source returned a duplicate definition ID.
    DuplicateDefinition(String),
    /// Static definition validation failed.
    InvalidInput(&'static str),
    /// Static definition exceeded the local byte bound.
    DefinitionTooLarge { limit: usize, actual: usize },
    /// Page size was zero or exceeded the local bound.
    InvalidPageSize,
    /// Continuation belonged to another reader or was already consumed.
    InvalidContinuation,
    /// Definition was excluded by visibility or unlock scope.
    ExcludedByScope,
    /// Definition ID was not found.
    NotFound,
    /// Reference names a previous manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for PowerStatusCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PowerStatusCatalogError {}

/// Sanitized failures while validating or reading live instances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusLiveError {
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
    /// Producer identity was not supported.
    ProducerVersionMismatch,
    /// Live binding identity was malformed.
    InvalidBinding(&'static str),
    /// Live instance identity or nested value was malformed.
    InvalidInput(&'static str),
    /// A snapshot repeated one live instance ID.
    DuplicateInstance(String),
    /// Instance references an unknown definition.
    UnknownDefinition(String),
    /// Amount variant does not match the static amount schema.
    AmountShapeMismatch(&'static str),
    /// Instance references an unknown counter.
    UnknownCounter {
        definition_id: String,
        counter_id: String,
    },
    /// A declared counter was not supplied.
    MissingCounter {
        definition_id: String,
        counter_id: String,
    },
    /// One counter ID occurred twice.
    DuplicateCounter {
        definition_id: String,
        counter_id: String,
    },
    /// A source value is not visible in the selected scope.
    VisibilityDenied(&'static str),
    /// A typed live state violates its static duration/decay rule.
    DurationMismatch(&'static str),
    /// The live detail exceeds the local byte bound.
    DetailTooLarge { limit: usize, actual: usize },
    /// A reference points to a previous snapshot identity.
    StaleReference,
    /// A snapshot epoch did not advance.
    NonMonotonicEpoch { current: u64, supplied: u64 },
    /// Live instance reference no longer exists.
    NotFound,
}

impl std::fmt::Display for PowerStatusLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PowerStatusLiveError {}

pub(super) fn map_source_error(error: PowerStatusSourceError) -> PowerStatusCatalogError {
    match error {
        PowerStatusSourceError::NoActiveSource => PowerStatusCatalogError::NoActiveSource,
        PowerStatusSourceError::AccessDenied => PowerStatusCatalogError::SourceAccessDenied,
        PowerStatusSourceError::Malformed => PowerStatusCatalogError::MalformedSource,
    }
}

pub(super) fn map_live_source_error(error: PowerStatusSourceError) -> PowerStatusLiveError {
    match error {
        PowerStatusSourceError::NoActiveSource => PowerStatusLiveError::NoActiveSource,
        PowerStatusSourceError::AccessDenied => PowerStatusLiveError::SourceAccessDenied,
        PowerStatusSourceError::Malformed => PowerStatusLiveError::MalformedSource,
    }
}
