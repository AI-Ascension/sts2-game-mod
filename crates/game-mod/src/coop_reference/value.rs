// SPDX-License-Identifier: MIT

//! Typed party values whose availability is stated, so an unknown value is never a zero.

use super::CoopFieldStatus;

/// A value whose availability is explicit.
///
/// The status and the value are one field, not two, because a caller that can read the value
/// without reading its status will eventually read a missing value as a default. The constructors
/// are the only way to build one, so `Present` always carries a value and every other status
/// carries none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopFieldValue<T> {
    status: CoopFieldStatus,
    value: Option<T>,
}

impl<T> CoopFieldValue<T> {
    /// States that a value was reported.
    #[must_use]
    pub fn present(value: T) -> Self {
        Self {
            status: CoopFieldStatus::Present,
            value: Some(value),
        }
    }

    /// States that the host reports the field as absent for this member.
    #[must_use]
    pub const fn absent() -> Self {
        Self {
            status: CoopFieldStatus::Absent,
            value: None,
        }
    }

    /// States that the supported build does not carry the field.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            status: CoopFieldStatus::Unsupported,
            value: None,
        }
    }

    /// States that the field exists but its owner withholds it.
    #[must_use]
    pub const fn withheld() -> Self {
        Self {
            status: CoopFieldStatus::Withheld,
            value: None,
        }
    }

    /// States that this scope is not permitted to observe the field.
    ///
    /// A refused field is stated as `NotPermitted` rather than dropped or emptied, so a reader can
    /// tell "you may not see this" apart from "there is nothing here".
    #[must_use]
    pub const fn not_permitted() -> Self {
        Self {
            status: CoopFieldStatus::NotPermitted,
            value: None,
        }
    }

    /// States that the retained value is older than the snapshot the fence names.
    #[must_use]
    pub const fn stale() -> Self {
        Self {
            status: CoopFieldStatus::Stale,
            value: None,
        }
    }

    /// Returns the availability status.
    #[must_use]
    pub const fn status(&self) -> CoopFieldStatus {
        self.status
    }

    /// Returns whether a value was reported.
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.status.is_present()
    }

    /// Returns whether this scope is refused the field.
    #[must_use]
    pub const fn is_not_permitted(&self) -> bool {
        matches!(self.status, CoopFieldStatus::NotPermitted)
    }

    /// Returns the reported value, or `None` when the status states no value.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Consumes the field and returns the reported value, or `None`.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        self.value
    }

    /// Returns whether the status and the value agree.
    ///
    /// Every constructor agrees, so a disagreement means a caller built one by hand through a path
    /// this module does not offer.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.status.is_present() == self.value.is_some()
    }
}

/// Unit of a reported quantity, such as a health point or a stack count.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopUnit {
    /// Bounded unit identity, empty when the host reports a unitless count.
    pub unit: String,
}

/// A typed quantity with its exact unit preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopQuantity {
    /// Signed amount as reported, never rescaled.
    pub amount: i64,
    /// Exact unit the host reported.
    pub unit: CoopUnit,
}

/// Current and maximum health of one member.
///
/// The two values are one field because a current value published without its maximum cannot be
/// told apart from a maximum that changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopHealth {
    /// Current health as reported.
    pub current: i64,
    /// Maximum health as reported.
    pub maximum: i64,
}

/// One character-specific resource of a member, such as energy or focus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopResource {
    /// Owner-defined resource identity.
    pub resource_id: String,
    /// Reported amount with its unit.
    pub amount: CoopQuantity,
}
