// SPDX-License-Identifier: MIT

use super::{field::RunConfigurationUnavailableReason, model::RunFieldKind};

/// Failure before an owned run-configuration snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunConfigurationSourceError {
    /// No supported run-configuration registry is active for the selected host/build/profile.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for RunConfigurationSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RunConfigurationSourceError {}

/// Sanitized failures while producing or reading run-configuration reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunConfigurationError {
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
    /// The source snapshot names another profile.
    ProfileMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The manifest does not inventory the run-configuration family.
    MissingFamily,
    /// The source advertises a family it cannot project.
    UnsupportedFamily,
    /// The source family is temporarily unavailable.
    UnavailableFamily,
    /// The family identity in the source snapshot is wrong.
    FamilyIdentityMismatch,
    /// Source and manifest family counts disagree.
    FamilyCountMismatch,
    /// A source record is absent from the manifest.
    UnknownDefinition(String),
    /// A manifest record is absent from the source snapshot.
    MissingDefinition(String),
    /// A reader lookup named a run identity the catalog does not retain.
    UnknownRun(String),
    /// A source record repeats a run identity.
    DuplicateDefinition(String),
    /// A run identity, modifier identity, or free text is malformed or over its bound.
    InvalidInput(&'static str),
    /// A record repeats a field kind.
    DuplicateField {
        /// Owning run identity.
        run_id: String,
        /// Repeated field kind.
        kind: RunFieldKind,
    },
    /// A value shape does not match its declared field kind.
    FieldShapeMismatch {
        /// Owning run identity.
        run_id: String,
        /// Field kind whose value shape was wrong.
        kind: RunFieldKind,
    },
    /// A required field carries no settled host value.
    MissingRequiredField {
        /// Owning run identity.
        run_id: String,
        /// Required field kind without a settled value.
        kind: RunFieldKind,
    },
    /// An optional-only outcome was declared for a field that always applies.
    NotApplicableRequiredField {
        /// Owning run identity.
        run_id: String,
        /// Required field kind that claimed no meaning.
        kind: RunFieldKind,
    },
    /// A modifier claims to alter a field whose settled value only echoes the request.
    ModifiedFieldEchoesRequest {
        /// Owning run identity.
        run_id: String,
        /// Field a modifier claims to alter.
        kind: RunFieldKind,
    },
    /// The declared active modifier set disagrees with the modifier list.
    ModifierSetMismatch(String),
    /// A modifier identity repeats, is malformed, or contradicts its state.
    InvalidModifier(String),
    /// A record or modifier names RNG state, which never belongs to a configuration read.
    RngStateNotPermitted(String),
    /// A field carries a value that its sensitivity or visibility must withhold.
    ValueMustBeWithheld(String),
    /// The seed field contradicts the declared seed policy.
    SeedPolicyMismatch(String),
    /// A requested field has no settled value for the stated reason.
    UnavailableField(RunConfigurationUnavailableReason),
    /// The record declares no field of the requested kind.
    NotFound(RunFieldKind),
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A reference was produced for another manifest, locale, profile, or producer.
    StaleReference,
    /// A reference names a configuration revision this reader no longer serves.
    StaleRevision {
        /// Revision the definition settled on.
        expected: u64,
        /// Revision the reference names.
        actual: u64,
    },
    /// A reference names another run identity.
    StaleRunIdentity,
    /// A seed-blind scope requested the seed field.
    SeedBlindScopeViolation,
    /// A record exceeds the aggregate byte bound.
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
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
}

impl std::fmt::Display for RunConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RunConfigurationError {}

pub(super) fn map_source_error(error: RunConfigurationSourceError) -> RunConfigurationError {
    match error {
        RunConfigurationSourceError::NoActiveSource => RunConfigurationError::NoActiveSource,
        RunConfigurationSourceError::AccessDenied => RunConfigurationError::SourceAccessDenied,
        RunConfigurationSourceError::Malformed => RunConfigurationError::MalformedSource,
    }
}
