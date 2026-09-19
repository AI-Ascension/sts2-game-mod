// SPDX-License-Identifier: MIT

/// Explicit reason that a shop field was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopUnavailableReason {
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
    /// The field has no meaning for the observed shop or mode.
    NotApplicable,
    /// The value exists but must not be revealed.
    Withheld,
    /// The linked feature is not integrated into the manifest.
    NotIntegrated,
}

/// Coarse availability status for one shop field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopFieldStatus {
    /// The host observed the field.
    Available,
    /// The field has no meaning for the observed shop or mode.
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

impl ShopUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> ShopFieldStatus {
        match self {
            Self::NotObserved => ShopFieldStatus::NotObserved,
            Self::Unsupported => ShopFieldStatus::Unsupported,
            Self::Denied => ShopFieldStatus::Denied,
            Self::Failed => ShopFieldStatus::Failed,
            Self::Unknown => ShopFieldStatus::Unknown,
            Self::NotApplicable => ShopFieldStatus::NotApplicable,
            Self::Withheld => ShopFieldStatus::Withheld,
            Self::NotIntegrated => ShopFieldStatus::Unsupported,
        }
    }
}

/// A shop field that distinguishes an observed value from an unavailable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShopField<T> {
    /// The host supplied the field; `T` may be a known empty value.
    Available(T),
    /// The host could not safely supply the field for the stated reason.
    Unavailable(ShopUnavailableReason),
}

impl<T> ShopField<T> {
    /// Creates a field the host observed.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self::Available(value)
    }

    /// Creates a field with an explicit reason instead of a substituted value.
    #[must_use]
    pub const fn unavailable(reason: ShopUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }

    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> ShopFieldStatus {
        match self {
            Self::Available(_) => ShopFieldStatus::Available,
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
    pub const fn reason(&self) -> Option<ShopUnavailableReason> {
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
            Self::Unavailable(ShopUnavailableReason::Withheld)
                | Self::Unavailable(ShopUnavailableReason::Denied)
        )
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(ShopUnavailableReason),
}

impl ShopText {
    /// Returns the explicit text availability status.
    #[must_use]
    pub const fn status(&self) -> ShopFieldStatus {
        match self {
            Self::Available(_) => ShopFieldStatus::Available,
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
}
