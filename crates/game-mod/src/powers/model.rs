// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const POWER_STATUS_ENTITY_KIND: &str = "power_status";
/// Source-only producer version.  This is not a transport or native ABI version.
pub const POWER_STATUS_PRODUCER_VERSION: &str = "game-power-status-producer-v1";
/// Maximum bytes accepted for one identity token.
pub const POWER_STATUS_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const POWER_STATUS_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum bytes accepted for one semantic reference payload.
pub const POWER_STATUS_MAX_REFERENCE_BYTES: usize = 64 * 1024;
/// Maximum aggregate bytes returned by one static definition.
pub const POWER_STATUS_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum aggregate bytes returned by one live instance.
pub const POWER_STATUS_MAX_LIVE_DETAIL_BYTES: usize = 16 * 1024;
/// Maximum definitions retained by one source snapshot.
pub const POWER_STATUS_MAX_DEFINITIONS: usize = 16 * 1024;
/// Maximum entries returned by one definition page.
pub const POWER_STATUS_MAX_PAGE_ITEMS: usize = 64;
/// Maximum counter declarations or values attached to one definition/instance.
pub const POWER_STATUS_MAX_COUNTERS: usize = 32;
/// Maximum effect, keyword, or rule references attached to one definition.
pub const POWER_STATUS_MAX_REFERENCES: usize = 64;
/// Compatibility alias for the local cap/reference bound.
pub const POWER_STATUS_MAX_CAP_REFERENCES: usize = POWER_STATUS_MAX_REFERENCES;
/// Maximum live instances retained by one coherent snapshot.
pub const POWER_STATUS_MAX_INSTANCES: usize = 512;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized value.
    pub locale: String,
    /// Exact owner-local producer version.
    pub producer_version: String,
}

/// Exact static definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusDefinitionReference {
    /// Catalog witness that owns the definition.
    pub catalog: PowerStatusCatalogBinding,
    /// Namespaced content definition identity.
    pub definition_id: String,
}

/// Live identity adds run and coherent snapshot fences to catalog identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusLiveBinding {
    /// Static catalog identity used by the live read.
    pub catalog: PowerStatusCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Coherent observation identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

/// Opaque identity of a live owner entity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusOwnerId(String);

impl PowerStatusOwnerId {
    /// Creates a bounded owner identity.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "owner_id")?;
        Ok(Self(value))
    }

    /// Returns the owner identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined category label, kept separate from the power/status kind.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusCategory(String);

impl PowerStatusCategory {
    /// Creates a bounded category token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "category")?;
        Ok(Self(value))
    }

    /// Returns the category token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined unit for an amount, counter, or duration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusUnit(String);

impl PowerStatusUnit {
    /// Creates a bounded unit token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "unit")?;
        Ok(Self(value))
    }

    /// Returns the unit token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Power/status families copied from the source without inferring external vocabulary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusKind {
    /// An additive or beneficial power.
    Power,
    /// A status applied to an entity.
    Status,
    /// A harmful or debuff status.
    Debuff,
    /// A stance or mode-like status.
    Stance,
    /// A source-defined family.
    Custom(String),
}

/// Visibility of static or dynamic source values.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusVisibility {
    /// Value is visible on the supported public surface.
    Visible,
    /// Value is visible only to an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope used for static and live reads.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusVisibilityScope {
    /// Public player-visible values.
    Public,
    /// Explicit owner-authorized values.
    Owner,
}

/// Lifetime boundary used by duration and decay rules.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusReset {
    /// Value does not reset during the observed lifetime.
    Never,
    /// Reset at a turn boundary.
    Turn,
    /// Reset at a round boundary.
    Round,
    /// Reset when leaving a room.
    Room,
    /// Reset when combat ends.
    Combat,
    /// Reset when the run ends.
    Run,
    /// Reset when the source-defined activation ends.
    Activation,
    /// Source could not establish a reset boundary.
    Unknown,
}

/// Explicit availability state for one source-owned field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusField<T> {
    /// A value was observed, including zero, false, and empty collections.
    Available(T),
    /// The field has no meaning for this entity or phase.
    NotApplicable,
    /// The field is supported but outside this snapshot's observable surface.
    NotObserved,
    /// No extractor is supported for this field.
    Unsupported,
    /// The caller or source scope denied the field.
    Denied,
    /// A read was attempted but failed or was unavailable.
    Unavailable,
    /// The source could not classify the field without inventing one.
    Unknown,
}

impl<T> PowerStatusField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> PowerStatusFieldStatus {
        match self {
            Self::Available(_) => PowerStatusFieldStatus::Available,
            Self::NotApplicable => PowerStatusFieldStatus::NotApplicable,
            Self::NotObserved => PowerStatusFieldStatus::NotObserved,
            Self::Unsupported => PowerStatusFieldStatus::Unsupported,
            Self::Denied => PowerStatusFieldStatus::Denied,
            Self::Unavailable => PowerStatusFieldStatus::Unavailable,
            Self::Unknown => PowerStatusFieldStatus::Unknown,
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

/// Status projection for [`PowerStatusField`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusFieldStatus {
    /// A value was observed.
    Available,
    /// The field does not apply.
    NotApplicable,
    /// A supported field was not on the current surface.
    NotObserved,
    /// No extractor exists.
    Unsupported,
    /// Access was denied.
    Denied,
    /// The read failed or source was unavailable.
    Unavailable,
    /// The source could not classify the field.
    Unknown,
}

/// Source provenance independent of owner or static category.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusOrigin {
    /// Owner-defined origin kind, such as a package or character.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional package version.
    pub package_version: Option<String>,
}

pub(crate) fn validate_identity(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > POWER_STATUS_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(field);
    }
    Ok(())
}

pub(crate) fn validate_text(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > POWER_STATUS_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(field);
    }
    Ok(())
}
