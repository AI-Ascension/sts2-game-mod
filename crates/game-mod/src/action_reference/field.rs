// SPDX-License-Identifier: MIT

/// Explicit reason an action field was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionUnavailableReason {
    /// The host supports the field but did not observe it.
    NotObserved,
    /// The supported build has no extractor for the field.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The host could not classify the field.
    Unknown,
    /// The field has no meaning for the observed action or mode.
    NotApplicable,
    /// The value exists but must not be revealed.
    Withheld,
    /// The linked feature is not integrated into the manifest.
    NotIntegrated,
    /// The action or target exists but the supported source does not describe it.
    Undescribed,
}

/// Coarse availability status for one action field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionFieldStatus {
    /// The host observed the field.
    Available,
    /// The field has no meaning for the observed action or mode.
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

impl ActionUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> ActionFieldStatus {
        match self {
            Self::NotObserved => ActionFieldStatus::NotObserved,
            Self::Unsupported | Self::NotIntegrated | Self::Undescribed => {
                ActionFieldStatus::Unsupported
            }
            Self::Denied => ActionFieldStatus::Denied,
            Self::Failed => ActionFieldStatus::Failed,
            Self::Unknown => ActionFieldStatus::Unknown,
            Self::NotApplicable => ActionFieldStatus::NotApplicable,
            Self::Withheld => ActionFieldStatus::Withheld,
        }
    }
}

/// An action field that distinguishes an observed value from an unavailable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionField<T> {
    /// The host supplied the field; `T` may be a known empty value.
    Available(T),
    /// The host could not safely supply the field for the stated reason.
    Unavailable(ActionUnavailableReason),
}

impl<T> ActionField<T> {
    /// Creates a field the host observed.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self::Available(value)
    }

    /// Creates a field with an explicit reason instead of a substituted value.
    #[must_use]
    pub const fn unavailable(reason: ActionUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }

    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> ActionFieldStatus {
        match self {
            Self::Available(_) => ActionFieldStatus::Available,
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
    pub const fn reason(&self) -> Option<ActionUnavailableReason> {
        match self {
            Self::Available(_) => None,
            Self::Unavailable(reason) => Some(*reason),
        }
    }

    /// Returns whether the field was withheld by scope rather than missing.
    #[must_use]
    pub const fn is_withheld(&self) -> bool {
        matches!(
            self,
            Self::Unavailable(ActionUnavailableReason::Withheld)
                | Self::Unavailable(ActionUnavailableReason::Denied)
        )
    }
}

/// Localized text or an explicit non-value.
///
/// An unobserved reason stays an explicit non-value; no explanation text is invented in its place.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(ActionUnavailableReason),
}

impl ActionText {
    /// Returns the explicit text availability status.
    #[must_use]
    pub const fn status(&self) -> ActionFieldStatus {
        match self {
            Self::Available(_) => ActionFieldStatus::Available,
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

    /// Returns whether the text was observed.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available(_))
    }

    /// Returns an explicitly withheld text value.
    #[must_use]
    pub const fn withheld() -> Self {
        Self::Unavailable(ActionUnavailableReason::Withheld)
    }
}
