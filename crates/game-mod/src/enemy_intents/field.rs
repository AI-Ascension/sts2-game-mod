// SPDX-License-Identifier: MIT

/// Public or explicitly owner-authorized visibility scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentVisibilityScope {
    /// Ordinary player-visible values only.
    Public,
    /// Explicit owner-authorized values.
    Owner,
}

/// Explicit availability of a source-owned intent or enemy field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyIntentField<T> {
    /// A value was observed coherently, including zero and empty collections.
    Available(T),
    /// The field has no meaning for this intent or phase.
    NotApplicable,
    /// The source supports the field but did not expose it in this snapshot.
    NotObserved,
    /// No extractor exists for this field.
    Unsupported,
    /// The caller's visibility scope denied the field.
    Denied,
    /// The source knows a value exists but it is not public.
    Hidden,
    /// The source changed while the field was copied.
    Stale,
    /// The source could not classify the field.
    Unknown,
}

impl<T> EnemyIntentField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> EnemyIntentFieldStatus {
        match self {
            Self::Available(_) => EnemyIntentFieldStatus::Available,
            Self::NotApplicable => EnemyIntentFieldStatus::NotApplicable,
            Self::NotObserved => EnemyIntentFieldStatus::NotObserved,
            Self::Unsupported => EnemyIntentFieldStatus::Unsupported,
            Self::Denied => EnemyIntentFieldStatus::Denied,
            Self::Hidden => EnemyIntentFieldStatus::Hidden,
            Self::Stale => EnemyIntentFieldStatus::Stale,
            Self::Unknown => EnemyIntentFieldStatus::Unknown,
        }
    }

    /// Returns a value only when the source observed it.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            _ => None,
        }
    }
}

/// Stable status code for an explicit field state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentFieldStatus {
    /// A value was observed.
    Available,
    /// The field does not apply.
    NotApplicable,
    /// The field was not on the current observable surface.
    NotObserved,
    /// No extractor exists.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// The value is known to exist but is not public.
    Hidden,
    /// The source changed during the read.
    Stale,
    /// The source could not classify the field.
    Unknown,
}

impl EnemyIntentFieldStatus {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotApplicable => "not_applicable",
            Self::NotObserved => "not_observed",
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::Hidden => "hidden",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}
