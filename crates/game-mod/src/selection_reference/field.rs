// SPDX-License-Identifier: MIT

/// Explicit reason that a selection field was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionUnavailableReason {
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
    /// The field has no meaning for the observed selector or mode.
    NotApplicable,
    /// The value exists but must not be revealed.
    Withheld,
    /// The linked feature is not integrated into the manifest.
    NotIntegrated,
    /// The candidate or selector exists but the supported source does not describe it.
    Undescribed,
}

/// Coarse availability status for one selection field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionFieldStatus {
    /// The host observed the field.
    Available,
    /// The field has no meaning for the observed selector or mode.
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

impl SelectionUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> SelectionFieldStatus {
        match self {
            Self::NotObserved => SelectionFieldStatus::NotObserved,
            Self::Unsupported | Self::NotIntegrated | Self::Undescribed => {
                SelectionFieldStatus::Unsupported
            }
            Self::Denied => SelectionFieldStatus::Denied,
            Self::Failed => SelectionFieldStatus::Failed,
            Self::Unknown => SelectionFieldStatus::Unknown,
            Self::NotApplicable => SelectionFieldStatus::NotApplicable,
            Self::Withheld => SelectionFieldStatus::Withheld,
        }
    }
}

/// A selection field that distinguishes an observed value from an unavailable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionField<T> {
    /// The host supplied the field; `T` may be a known empty value.
    Available(T),
    /// The host could not safely supply the field for the stated reason.
    Unavailable(SelectionUnavailableReason),
}

impl<T> SelectionField<T> {
    /// Creates a field the host observed.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self::Available(value)
    }

    /// Creates a field with an explicit reason instead of a substituted value.
    #[must_use]
    pub const fn unavailable(reason: SelectionUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }

    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> SelectionFieldStatus {
        match self {
            Self::Available(_) => SelectionFieldStatus::Available,
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
    pub const fn reason(&self) -> Option<SelectionUnavailableReason> {
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
            Self::Unavailable(SelectionUnavailableReason::Withheld)
                | Self::Unavailable(SelectionUnavailableReason::Denied)
        )
    }
}

/// Localized text or an explicit non-value.
///
/// An unobserved prompt stays an explicit non-value; no placeholder text is invented in its place.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(SelectionUnavailableReason),
}

impl SelectionText {
    /// Returns the explicit text availability status.
    #[must_use]
    pub const fn status(&self) -> SelectionFieldStatus {
        match self {
            Self::Available(_) => SelectionFieldStatus::Available,
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
        Self::Unavailable(SelectionUnavailableReason::Withheld)
    }
}
