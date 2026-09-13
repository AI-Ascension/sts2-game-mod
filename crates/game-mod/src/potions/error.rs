// SPDX-License-Identifier: MIT

/// Failure before an owned potion catalog or live snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PotionSourceError {
    NoActiveSource,
    AccessDenied,
    Malformed,
}

impl std::fmt::Display for PotionSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PotionSourceError {}

/// Sanitized failures while producing or querying static potion definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionCatalogError {
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

impl std::fmt::Display for PotionCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PotionCatalogError {}

/// Sanitized failures while validating or reading live potion state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionLiveError {
    NoActiveSource,
    SourceAccessDenied,
    MalformedSource,
    CatalogMismatch,
    InstanceMismatch,
    RunMismatch,
    ProducerVersionMismatch,
    InvalidBinding(&'static str),
    InvalidInput(&'static str),
    DuplicateInstance(String),
    UnknownDefinition(String),
    DuplicateSlot(String),
    InvalidSlot(String),
    MissingSlot(String),
    DuplicateOffer(String),
    UnknownOfferInstance(String),
    DuplicateOfferInstance(String),
    UnknownParameter {
        potion_id: String,
        parameter_id: String,
    },
    DuplicateParameter {
        potion_id: String,
        parameter_id: String,
    },
    UnknownEffect(String),
    InvalidEffectState(&'static str),
    InvalidState(&'static str),
    DetailTooLarge {
        limit: usize,
        actual: usize,
    },
    NonMonotonicEpoch {
        current: u64,
        supplied: u64,
    },
    StaleReference,
    NotFound,
}

impl std::fmt::Display for PotionLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PotionLiveError {}

pub(super) fn map_source_error(error: PotionSourceError) -> PotionCatalogError {
    match error {
        PotionSourceError::NoActiveSource => PotionCatalogError::NoActiveSource,
        PotionSourceError::AccessDenied => PotionCatalogError::SourceAccessDenied,
        PotionSourceError::Malformed => PotionCatalogError::MalformedSource,
    }
}

pub(super) fn map_live_source_error(error: PotionSourceError) -> PotionLiveError {
    match error {
        PotionSourceError::NoActiveSource => PotionLiveError::NoActiveSource,
        PotionSourceError::AccessDenied => PotionLiveError::SourceAccessDenied,
        PotionSourceError::Malformed => PotionLiveError::MalformedSource,
    }
}
