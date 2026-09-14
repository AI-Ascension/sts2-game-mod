// SPDX-License-Identifier: MIT

use super::error::RetainedMapError;
use super::model::{RETAINED_MAP_MAX_IDENTITY_BYTES, RETAINED_MAP_MAX_TEXT_BYTES};
use crate::ContentCursorBinding;

/// Static content witness, locale, and producer used by retained knowledge.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RetainedMapCatalogBinding {
    /// Immutable content-manifest cursor witness.
    pub content_manifest: ContentCursorBinding,
    /// Locale used for localized map labels.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Full identity fence for one coherent retained map observation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RetainedMapLiveBinding {
    /// Static content and locale witness.
    pub catalog: RetainedMapCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Act identity; an act transition invalidates retained knowledge.
    pub act_id: String,
    /// Selected mode identity.
    pub mode_id: String,
    /// Map-instance identity retired by a run/act/restore transition.
    pub map_instance_id: String,
    /// Coherent source snapshot identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

impl RetainedMapLiveBinding {
    /// Returns whether two bindings match on the complete snapshot fence.
    ///
    /// The fence includes the catalog witness, instance, run, act, mode, map-instance,
    /// `snapshot_id`, and `epoch`. Reconciliation and replacement compare the whole value, so a
    /// changed snapshot identity is never mistaken for the same retained generation.
    #[must_use]
    pub fn same_snapshot_fence(&self, other: &Self) -> bool {
        self == other
    }
}

/// Observation state of the live map surface without opening it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapObservationState {
    /// The map surface is open and permitted for an observation.
    Observable,
    /// The map surface is closed; retained knowledge stays readable but not current.
    Closed,
    /// Policy forbids observing the surface even if it is open.
    Forbidden,
    /// The selected host/build has no observable map surface.
    Unsupported,
    /// The source could not classify the surface.
    Unknown,
}

impl RetainedMapObservationState {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Observable => "observable",
            Self::Closed => "closed",
            Self::Forbidden => "forbidden",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

/// Honest freshness/provenance of retained knowledge.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapFreshness {
    /// Retained at the live generation while the surface was permitted.
    Current,
    /// Previously observed topology retained after the surface closed.
    Retained,
    /// The owning generation or identity changed; not usable as current.
    Stale,
    /// Disclosure policy withholds the retained value.
    Withheld,
    /// The source supports the value but did not provide it.
    Unavailable,
    /// No permitted observation has happened yet.
    NeverObserved,
    /// The source could not classify freshness.
    Unknown,
}

impl RetainedMapFreshness {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Retained => "retained",
            Self::Stale => "stale",
            Self::Withheld => "withheld",
            Self::Unavailable => "unavailable",
            Self::NeverObserved => "never_observed",
            Self::Unknown => "unknown",
        }
    }

    /// Returns whether retained knowledge is verified at the live generation.
    #[must_use]
    pub const fn is_current(self) -> bool {
        matches!(self, Self::Current)
    }

    /// Returns whether topology may be served, even without current authority.
    #[must_use]
    pub const fn trusts_topology(self) -> bool {
        matches!(self, Self::Current | Self::Retained)
    }
}

/// Public or explicitly owner-authorized visibility scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapVisibilityScope {
    /// Ordinary player-visible values only.
    Public,
    /// Explicit owner-authorized values.
    Owner,
}

/// Visibility of one retained topology node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapNodeVisibility {
    /// Visible on the permitted public surface.
    Public,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// The source knows a value exists but must not reveal it.
    Hidden,
    /// The source could not classify visibility.
    Unknown,
}

/// Provenance label for retained knowledge.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapProvenance {
    /// Copied from a permitted public observation.
    ObservedPublic,
    /// Copied under an explicit owner-authorized scope.
    ObservedOwnerAuthorized,
    /// Derived from a source-owned description without runtime verification.
    SourceDerived,
    /// Evidence is insufficient to classify the value.
    Unknown,
}

/// Whether one retained travel binding may currently authorize navigation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapTravelActionability {
    /// Verified at the current generation while the surface is open.
    Current,
    /// Known topology retained while the surface is closed; not actionable.
    Retained,
    /// The owning generation changed; not actionable.
    Stale,
    /// Policy withholds the travel binding.
    Withheld,
    /// The source supports travel bindings but did not provide them.
    Unavailable,
    /// The source could not classify actionability.
    Unknown,
}

impl RetainedMapTravelActionability {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Retained => "retained",
            Self::Stale => "stale",
            Self::Withheld => "withheld",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }

    /// Returns whether this binding may authorize navigation.
    #[must_use]
    pub const fn is_actionable(self) -> bool {
        matches!(self, Self::Current)
    }
}

pub(super) fn validate_identity(value: &str, field: &'static str) -> Result<(), RetainedMapError> {
    if value.is_empty()
        || value.len() > RETAINED_MAP_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(RetainedMapError::InvalidBinding(field));
    }
    Ok(())
}

pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), RetainedMapError> {
    if value.is_empty()
        || value.len() > RETAINED_MAP_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(RetainedMapError::InvalidInput(field));
    }
    Ok(())
}

pub(super) fn visible(
    visibility: RetainedMapNodeVisibility,
    scope: RetainedMapVisibilityScope,
) -> bool {
    match visibility {
        RetainedMapNodeVisibility::Public => true,
        RetainedMapNodeVisibility::OwnerOnly => matches!(scope, RetainedMapVisibilityScope::Owner),
        RetainedMapNodeVisibility::Hidden | RetainedMapNodeVisibility::Unknown => false,
    }
}
