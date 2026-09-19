// SPDX-License-Identifier: MIT

/// Explicit reason that a run-configuration field was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunConfigurationUnavailableReason {
    /// The host supports the field but did not settle or observe it.
    NotObserved,
    /// The supported build has no extractor for the field.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The host could not classify the field.
    Unknown,
    /// The field has no meaning for the admitted mode.
    NotApplicable,
    /// The value exists but must not be revealed.
    Withheld,
    /// The linked feature is not integrated into the manifest.
    NotIntegrated,
}

/// Coarse availability status for one run-configuration field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunConfigurationFieldStatus {
    /// The host settled or observed the field.
    Available,
    /// The field has no meaning for the admitted mode.
    NotApplicable,
    /// The field is supported but was not observed.
    NotObserved,
    /// No extractor is supported for the field.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The host could not classify the field.
    Unknown,
    /// The value exists but must not be revealed.
    Withheld,
}

impl RunConfigurationUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> RunConfigurationFieldStatus {
        match self {
            Self::NotObserved => RunConfigurationFieldStatus::NotObserved,
            Self::Unsupported => RunConfigurationFieldStatus::Unsupported,
            Self::Denied => RunConfigurationFieldStatus::Denied,
            Self::Failed => RunConfigurationFieldStatus::Failed,
            Self::Unknown => RunConfigurationFieldStatus::Unknown,
            Self::NotApplicable => RunConfigurationFieldStatus::NotApplicable,
            Self::Withheld => RunConfigurationFieldStatus::Withheld,
            Self::NotIntegrated => RunConfigurationFieldStatus::Unsupported,
        }
    }
}

/// A run-configuration field that distinguishes a settled value from an unavailable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunConfigurationField<T> {
    /// The host supplied the field; `T` may be a known empty value.
    Available(T),
    /// The host could not safely supply the field for the stated reason.
    Unavailable(RunConfigurationUnavailableReason),
}

impl<T> RunConfigurationField<T> {
    /// Creates a field the host settled or observed.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self::Available(value)
    }

    /// Creates a field with an explicit reason instead of a substituted value.
    #[must_use]
    pub const fn unavailable(reason: RunConfigurationUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }

    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> RunConfigurationFieldStatus {
        match self {
            Self::Available(_) => RunConfigurationFieldStatus::Available,
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

    /// Returns whether a value was observed.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available(_))
    }

    /// Returns the explicit reason no value was observed.
    #[must_use]
    pub const fn reason(&self) -> Option<RunConfigurationUnavailableReason> {
        match self {
            Self::Available(_) => None,
            Self::Unavailable(reason) => Some(*reason),
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(RunConfigurationUnavailableReason),
}

impl RunText {
    /// Returns the explicit text availability status.
    #[must_use]
    pub const fn status(&self) -> RunConfigurationFieldStatus {
        match self {
            Self::Available(_) => RunConfigurationFieldStatus::Available,
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

    pub(super) fn from_validated(value: String) -> Self {
        Self::Available(value)
    }
}
