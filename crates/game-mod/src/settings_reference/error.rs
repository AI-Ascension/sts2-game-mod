// SPDX-License-Identifier: MIT

use super::SettingsUnavailableReason;

/// Failure before an owned settings snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSourceError {
    /// No supported settings registry is active for the selected host/build/profile.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for SettingsSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SettingsSourceError {}

/// Sanitized failures while producing or reading settings reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsReferenceError {
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
    /// The manifest does not inventory the settings family.
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
    /// A value does not match the declared value type.
    ValueTypeMismatch(String),
    /// A constraint does not apply to the declared value type.
    ConstraintValueTypeMismatch(String),
    /// An observed value is not one of the declared enumeration options.
    UnknownOption {
        /// Owning setting definition identity.
        setting_id: String,
        /// Selected option identity.
        option_id: String,
    },
    /// A stored value violates its declared range or length bound.
    ValueOutOfRange {
        /// Owning setting definition identity.
        setting_id: String,
    },
    /// A value was supplied for a setting whose scope must withhold it.
    ValueMustBeWithheld(String),
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
    /// A run-affecting setting has no matching run-configuration reference.
    MissingRunReference(String),
    /// A run-configuration reference exists without a run-affecting link.
    UnexpectedRunReference(String),
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(SettingsUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest, locale, profile, or producer.
    StaleReference,
}

impl std::fmt::Display for SettingsReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SettingsReferenceError {}

pub(super) fn map_source_error(error: SettingsSourceError) -> SettingsReferenceError {
    match error {
        SettingsSourceError::NoActiveSource => SettingsReferenceError::NoActiveSource,
        SettingsSourceError::AccessDenied => SettingsReferenceError::SourceAccessDenied,
        SettingsSourceError::Malformed => SettingsReferenceError::MalformedSource,
    }
}
