// SPDX-License-Identifier: MIT

/// Explicit availability of a retained value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetainedMapField<T> {
    /// A value was observed coherently, including an empty collection.
    Available(T),
    /// The field has no meaning for this node.
    NotApplicable,
    /// The source supports the field but did not expose it.
    NotObserved,
    /// No extractor exists for the field.
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

impl<T> RetainedMapField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> RetainedMapFieldStatus {
        match self {
            Self::Available(_) => RetainedMapFieldStatus::Available,
            Self::NotApplicable => RetainedMapFieldStatus::NotApplicable,
            Self::NotObserved => RetainedMapFieldStatus::NotObserved,
            Self::Unsupported => RetainedMapFieldStatus::Unsupported,
            Self::Denied => RetainedMapFieldStatus::Denied,
            Self::Hidden => RetainedMapFieldStatus::Hidden,
            Self::Stale => RetainedMapFieldStatus::Stale,
            Self::Unknown => RetainedMapFieldStatus::Unknown,
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
pub enum RetainedMapFieldStatus {
    /// A value was observed.
    Available,
    /// The field does not apply.
    NotApplicable,
    /// The field was not on the observable surface.
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

impl RetainedMapFieldStatus {
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
