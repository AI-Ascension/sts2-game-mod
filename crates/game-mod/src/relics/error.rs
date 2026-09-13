// SPDX-License-Identifier: MIT

/// Failure before an owned relic catalog or live snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelicSourceError {
    NoActiveSource,
    AccessDenied,
    Malformed,
}

impl std::fmt::Display for RelicSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelicSourceError {}

/// Sanitized failures while producing or querying static relic definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelicCatalogError {
    NoActiveSource,
    SourceAccessDenied,
    MalformedSource,
    ManifestMismatch,
    LocaleMismatch,
    ProducerVersionMismatch,
    MissingFamily,
    UnsupportedFamily,
    UnavailableFamily,
    FamilyIdentityMismatch,
    FamilyCountMismatch,
    MissingDefinition(String),
    UnknownDefinition(String),
    DuplicateDefinition(String),
    InvalidInput(&'static str),
    DefinitionTooLarge { limit: usize, actual: usize },
    InvalidPageSize,
    InvalidContinuation,
    ExcludedByScope,
    NotFound,
    StaleReference,
}

impl std::fmt::Display for RelicCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelicCatalogError {}

/// Sanitized failures while validating or reading live relic state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelicLiveError {
    NoActiveSource,
    SourceAccessDenied,
    MalformedSource,
    CatalogMismatch,
    ProducerVersionMismatch,
    InvalidBinding(&'static str),
    InvalidInput(&'static str),
    DuplicateInstance(String),
    UnknownDefinition(String),
    UnknownCounter {
        relic_id: String,
        counter_id: String,
    },
    MissingCounter {
        relic_id: String,
        counter_id: String,
    },
    DuplicateCounter {
        relic_id: String,
        counter_id: String,
    },
    UnknownParameter {
        relic_id: String,
        parameter_id: String,
    },
    DuplicateParameter {
        relic_id: String,
        parameter_id: String,
    },
    UnknownTrigger {
        relic_id: String,
        trigger_id: String,
    },
    DuplicateTrigger {
        relic_id: String,
        trigger_id: String,
    },
    InvalidState(&'static str),
    DetailTooLarge {
        limit: usize,
        actual: usize,
    },
    StaleReference,
    NotFound,
}

impl std::fmt::Display for RelicLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelicLiveError {}

pub(super) fn map_source_error(error: RelicSourceError) -> RelicCatalogError {
    match error {
        RelicSourceError::NoActiveSource => RelicCatalogError::NoActiveSource,
        RelicSourceError::AccessDenied => RelicCatalogError::SourceAccessDenied,
        RelicSourceError::Malformed => RelicCatalogError::MalformedSource,
    }
}

pub(super) fn map_live_source_error(error: RelicSourceError) -> RelicLiveError {
    match error {
        RelicSourceError::NoActiveSource => RelicLiveError::NoActiveSource,
        RelicSourceError::AccessDenied => RelicLiveError::SourceAccessDenied,
        RelicSourceError::Malformed => RelicLiveError::MalformedSource,
    }
}
