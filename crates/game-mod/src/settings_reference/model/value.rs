// SPDX-License-Identifier: MIT

use super::{SettingsReadSeam, SettingsValueType, validate_text};

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsUnavailableReason {
    /// The source supports the field but did not observe it.
    NotObserved,
    /// The supported host/build has no extractor for the field.
    Unsupported,
    /// The caller's source scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
    /// The field has no meaning for the selected setting.
    NotApplicable,
    /// The value exists but is withheld because the setting is private or hidden.
    Withheld,
    /// The linked run-configuration feature is not integrated into the manifest.
    NotIntegrated,
}

/// Coarse availability status for one source-owned setting field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsFieldStatus {
    /// The source observed the field, including a known empty value.
    Available,
    /// The field has no meaning for the selected definition.
    NotApplicable,
    /// The field is supported but was not observed.
    NotObserved,
    /// No extractor is supported for the field.
    Unsupported,
    /// The source denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
    /// The value exists but must not be revealed.
    Withheld,
}

impl SettingsUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> SettingsFieldStatus {
        match self {
            Self::NotObserved => SettingsFieldStatus::NotObserved,
            Self::Unsupported => SettingsFieldStatus::Unsupported,
            Self::Denied => SettingsFieldStatus::Denied,
            Self::Failed => SettingsFieldStatus::Failed,
            Self::Unknown => SettingsFieldStatus::Unknown,
            Self::NotApplicable => SettingsFieldStatus::NotApplicable,
            Self::Withheld => SettingsFieldStatus::Withheld,
            Self::NotIntegrated => SettingsFieldStatus::Unsupported,
        }
    }
}

/// A source field that distinguishes an observed value from an unavailable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(SettingsUnavailableReason),
}

impl<T> SettingsField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> SettingsFieldStatus {
        match self {
            Self::Available(_) => SettingsFieldStatus::Available,
            Self::Unavailable(reason) => reason.status(),
        }
    }

    /// Returns the observed value.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            Self::Unavailable(_) => None,
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(SettingsUnavailableReason),
}

impl SettingsText {
    /// Creates a bounded available localized text value.
    pub fn available(value: &str) -> Result<Self, super::super::SettingsReferenceError> {
        validate_text(value, "text")?;
        Ok(Self::Available(value.to_owned()))
    }

    /// Returns the explicit text availability status.
    #[must_use]
    pub const fn status(&self) -> SettingsFieldStatus {
        match self {
            Self::Available(_) => SettingsFieldStatus::Available,
            Self::Unavailable(reason) => reason.status(),
        }
    }

    /// Returns the localized text when it was observed.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Available(value) => Some(value.as_str()),
            Self::Unavailable(_) => None,
        }
    }

    pub(crate) fn from_validated(value: String) -> Self {
        Self::Available(value)
    }
}

/// Evidence label for one settings fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsEvidence {
    /// The owner exposes the fact as an authoritative preference.
    Authoritative,
    /// The fact was copied from an owner-owned record without runtime verification.
    SourceDerived,
    /// The value was independently authored and is not a host claim.
    IndependentlyAuthored,
    /// The value was observed on a visible surface.
    Observed,
    /// Evidence is insufficient to classify the value as authoritative.
    Unverified,
}

/// Typed setting value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingValue {
    /// Boolean toggle value.
    Boolean(bool),
    /// Integer value.
    Integer(i64),
    /// Selected enumeration option identity.
    Enumeration(String),
    /// Ordered input keys.
    Keys(Vec<String>),
    /// Bounded free text.
    Text(String),
}

impl SettingValue {
    /// Returns the typed value shape.
    #[must_use]
    pub const fn value_type(&self) -> SettingsValueType {
        match self {
            Self::Boolean(_) => SettingsValueType::Boolean,
            Self::Integer(_) => SettingsValueType::Integer,
            Self::Enumeration(_) => SettingsValueType::Enumeration,
            Self::Keys(_) => SettingsValueType::KeyBinding,
            Self::Text(_) => SettingsValueType::Text,
        }
    }
}

/// One stored, effective, or default value with its provenance.
///
/// A single field distinguishes the persisted preference from the runtime-effective value; this
/// source-only slice never writes either.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingValueField {
    /// Observed value or the explicit reason it is absent.
    pub value: SettingsField<SettingValue>,
    /// Evidence label for this value.
    pub evidence: SettingsEvidence,
    /// Supported read seam the value was copied through.
    pub seam: SettingsReadSeam,
}

impl SettingValueField {
    /// Creates an observed value with its provenance.
    #[must_use]
    pub const fn available(
        value: SettingValue,
        evidence: SettingsEvidence,
        seam: SettingsReadSeam,
    ) -> Self {
        Self {
            value: SettingsField::Available(value),
            evidence,
            seam,
        }
    }

    /// Creates an absent value with its reason and provenance.
    #[must_use]
    pub const fn unavailable(
        reason: SettingsUnavailableReason,
        evidence: SettingsEvidence,
        seam: SettingsReadSeam,
    ) -> Self {
        Self {
            value: SettingsField::Unavailable(reason),
            evidence,
            seam,
        }
    }

    /// Returns the explicit availability status.
    #[must_use]
    pub const fn status(&self) -> SettingsFieldStatus {
        self.value.status()
    }

    /// Returns the observed value.
    #[must_use]
    pub const fn get(&self) -> Option<&SettingValue> {
        self.value.value()
    }
}
