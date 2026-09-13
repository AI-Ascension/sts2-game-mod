// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const RELIC_ENTITY_KIND: &str = "relic";
/// Source-only producer version. This is not a transport or native ABI version.
pub const RELIC_PRODUCER_VERSION: &str = "game-relics-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const RELIC_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one title, description, label, or rule value.
pub const RELIC_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes returned by one static definition.
pub const RELIC_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum aggregate bytes returned by one live instance.
pub const RELIC_MAX_LIVE_DETAIL_BYTES: usize = 16 * 1024;
/// Maximum entries returned by one bounded definition page.
pub const RELIC_MAX_PAGE_ITEMS: usize = 64;
/// Maximum acquisition restrictions on one definition.
pub const RELIC_MAX_ACQUISITION_RULES: usize = 32;
/// Maximum semantic references on one definition.
pub const RELIC_MAX_REFERENCES: usize = 64;
/// Maximum supported variants on one definition.
pub const RELIC_MAX_VARIANTS: usize = 32;
/// Maximum counter declarations on one definition.
pub const RELIC_MAX_COUNTERS: usize = 32;
/// Maximum visible parameter declarations on one definition.
pub const RELIC_MAX_PARAMETERS: usize = 32;
/// Maximum pending triggers on one live instance.
pub const RELIC_MAX_PENDING_TRIGGERS: usize = 32;
/// Maximum live instances retained in one coherent snapshot.
pub const RELIC_MAX_INSTANCES: usize = 256;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized value.
    pub locale: String,
    /// Exact owner-local producer version.
    pub producer_version: String,
}

/// Exact static definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicDefinitionReference {
    /// Catalog witness that owns the definition.
    pub catalog: RelicCatalogBinding,
    /// Namespaced content definition identity.
    pub relic_id: String,
}

/// Live identity adds run and coherent snapshot fences to the catalog identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicLiveBinding {
    /// Static catalog identity used by the live read.
    pub catalog: RelicCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Coherent observation identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

/// Opaque owner identity on a live relic instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicOwnerId(String);

impl RelicOwnerId {
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

/// Owner-defined rarity and tier values. No game vocabulary is inferred.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicRarity(String);

impl RelicRarity {
    /// Creates a bounded rarity token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "rarity")?;
        Ok(Self(value))
    }

    /// Returns the rarity token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined relic tier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicTier(String);

impl RelicTier {
    /// Creates a bounded tier token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "tier")?;
        Ok(Self(value))
    }

    /// Returns the tier token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined pool identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicPool(String);

impl RelicPool {
    /// Creates a bounded pool token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "pool")?;
        Ok(Self(value))
    }

    /// Returns the pool token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Source origin independent of acquisition pool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicOrigin {
    /// Owner-defined origin kind, for example a package or character.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional package version, kept separate from semantic rules.
    pub package_version: Option<String>,
}

/// Typed visibility marker attached to counters, parameters, and triggers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicVisibility {
    /// Value is visible on the supported public surface.
    Visible,
    /// Value is only visible in an explicitly owner-authorized surface.
    OwnerOnly,
    /// Source knows the field exists but cannot expose it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Unit for a counter or resolved parameter.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicUnit(String);

impl RelicUnit {
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

/// Reset/lifetime semantics for a typed counter or accumulated value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicCounterReset {
    Never,
    Turn,
    Room,
    Combat,
    Run,
    Activation,
    Unknown,
}

/// Explicit availability state for a typed source value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelicField<T> {
    /// A value was observed, including zero and empty collections.
    Available(T),
    /// The field has no meaning for this relic or phase.
    NotApplicable,
    /// The field is supported but outside the observed surface.
    NotObserved,
    /// No extractor is supported for this field.
    Unsupported,
    /// A read was attempted but failed or was denied.
    Unavailable,
    /// The source could not classify the value without inventing one.
    Unknown,
}

impl<T> RelicField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> RelicFieldStatus {
        match self {
            Self::Available(_) => RelicFieldStatus::Available,
            Self::NotApplicable => RelicFieldStatus::NotApplicable,
            Self::NotObserved => RelicFieldStatus::NotObserved,
            Self::Unsupported => RelicFieldStatus::Unsupported,
            Self::Unavailable => RelicFieldStatus::Unavailable,
            Self::Unknown => RelicFieldStatus::Unknown,
        }
    }

    /// Returns the value only when the source observed it.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            _ => None,
        }
    }
}

/// Status projection for [`RelicField`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicFieldStatus {
    Available,
    NotApplicable,
    NotObserved,
    Unsupported,
    Unavailable,
    Unknown,
}

pub(crate) fn validate_identity(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > RELIC_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(field);
    }
    Ok(())
}

pub(crate) fn validate_text(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty() || value.len() > RELIC_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(field);
    }
    Ok(())
}
