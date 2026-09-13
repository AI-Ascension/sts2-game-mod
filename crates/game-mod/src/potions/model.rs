// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const POTION_ENTITY_KIND: &str = "potion";
/// Source-only producer version. This is not a transport or native ABI version.
pub const POTION_PRODUCER_VERSION: &str = "game-potions-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const POTION_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one title, description, label, or rule value.
pub const POTION_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes returned by one owner-local definition.
pub const POTION_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum aggregate bytes returned by one live instance.
pub const POTION_MAX_LIVE_DETAIL_BYTES: usize = 16 * 1024;
/// Maximum entries returned by one bounded definition page.
pub const POTION_MAX_PAGE_ITEMS: usize = 64;
/// Maximum acquisition restrictions on one definition.
pub const POTION_MAX_ACQUISITION_RULES: usize = 32;
/// Maximum static effects on one definition.
pub const POTION_MAX_EFFECTS: usize = 64;
/// Maximum alternatives on one effect.
pub const POTION_MAX_ALTERNATIVES: usize = 32;
/// Maximum semantic references on one definition/effect.
pub const POTION_MAX_REFERENCES: usize = 64;
/// Maximum visible parameter declarations on one definition.
pub const POTION_MAX_PARAMETERS: usize = 64;
/// Maximum modifiers attached to one live instance.
pub const POTION_MAX_MODIFIERS: usize = 64;
/// Maximum permitted target references attached to one live instance.
pub const POTION_MAX_TARGETS: usize = 128;
/// Maximum live potion instances retained in one coherent snapshot.
pub const POTION_MAX_INSTANCES: usize = 256;
/// Maximum slots retained in one inventory projection.
pub const POTION_MAX_SLOTS: usize = 64;
/// Maximum reward/shop offers retained in one snapshot.
pub const POTION_MAX_OFFERS: usize = 256;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized value.
    pub locale: String,
    /// Exact owner-local producer version.
    pub producer_version: String,
}

/// Exact static definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionDefinitionReference {
    /// Catalog witness that owns the definition.
    pub catalog: PotionCatalogBinding,
    /// Namespaced content definition identity.
    pub potion_id: String,
}

/// Live identity adds game instance, run, snapshot, and epoch fences to catalog identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionLiveBinding {
    /// Static catalog identity used by the live read.
    pub catalog: PotionCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Coherent observation identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

/// Opaque owner identity on a live potion instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionOwnerId(String);

impl PotionOwnerId {
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

/// Owner-defined rarity vocabulary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionRarity(String);

impl PotionRarity {
    /// Creates a bounded rarity token without guessing game vocabulary.
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

/// Owner-defined acquisition pool identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionPool(String);

impl PotionPool {
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
pub struct PotionOrigin {
    /// Owner-defined origin kind, for example a package or character.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional package version, kept separate from semantic rules.
    pub package_version: Option<String>,
}

/// Typed visibility marker attached to fields and effects.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionVisibility {
    /// Value is visible on the supported public surface.
    Visible,
    /// Value is visible only in an explicitly owner-authorized surface.
    OwnerOnly,
    /// Source knows the field exists but cannot expose it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Static visibility scope used by catalog and live reads.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionVisibilityScope {
    /// Publicly visible, unlocked content and fields only.
    Public,
    /// Reference/search scope, including locked definitions but not owner-only fields.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Targeting mode declared by a static potion definition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionTargetMode {
    /// The potion has no target.
    None,
    /// The potion targets the current player.
    SelfPlayer,
    /// One allied character is selected.
    SingleAlly,
    /// Every allied character is affected.
    AllAllies,
    /// One enemy is selected.
    SingleEnemy,
    /// Every enemy is affected.
    AllEnemies,
    /// One character from either side is selected.
    AnyCharacter,
    /// One enemy is selected by the game.
    RandomEnemy,
    /// The source could not classify the target mode.
    Unknown,
}

/// Static usability rule; current legality belongs to live state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionUseRule {
    /// The potion may be used in any phase supported by the source.
    Anytime,
    /// The potion is usable only during combat.
    Combat,
    /// The potion is usable only outside combat.
    OutOfCombat,
    /// A bounded owner-defined condition controls usability.
    Conditional(String),
    /// Source could not classify the rule.
    Unknown,
}

/// Unit for magnitudes and resolved parameters.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionUnit(String);

impl PotionUnit {
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

/// Collection in which a slot or offer exists.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionCollectionKind {
    /// The owner's bounded inventory.
    Inventory,
    /// A reward offer collection with an owner-defined identity.
    Reward(String),
    /// A shop offer collection with an owner-defined identity.
    Shop(String),
    /// A bounded source-defined collection.
    Other(String),
}

impl PotionCollectionKind {
    /// Returns the stable collection code.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Inventory => "inventory",
            Self::Reward(value) | Self::Shop(value) | Self::Other(value) => value,
        }
    }
}

/// Stable slot identity, intentionally distinct from a live instance identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionSlotReference {
    /// Inventory/reward/shop collection.
    pub collection: PotionCollectionKind,
    /// Stable source slot identity.
    pub slot_id: String,
    /// Position when the source supplies one.
    pub index: u16,
}

/// Explicit availability state for a typed source value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionField<T> {
    /// A value was observed, including zero and empty collections.
    Available(T),
    /// The field has no meaning for this potion or phase.
    NotApplicable,
    /// The field is supported but outside the observed source surface.
    NotObserved,
    /// No extractor is supported for this field.
    Unsupported,
    /// A read was attempted but failed or was denied.
    Unavailable,
    /// The source could not classify the value without inventing one.
    Unknown,
}

impl<T> PotionField<T> {
    /// Returns the explicit availability status.
    #[must_use]
    pub const fn status(&self) -> PotionFieldStatus {
        match self {
            Self::Available(_) => PotionFieldStatus::Available,
            Self::NotApplicable => PotionFieldStatus::NotApplicable,
            Self::NotObserved => PotionFieldStatus::NotObserved,
            Self::Unsupported => PotionFieldStatus::Unsupported,
            Self::Unavailable => PotionFieldStatus::Unavailable,
            Self::Unknown => PotionFieldStatus::Unknown,
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

/// Status projection for [`PotionField`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionFieldStatus {
    Available,
    NotApplicable,
    NotObserved,
    Unsupported,
    Unavailable,
    Unknown,
}

pub(crate) fn validate_identity(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > POTION_MAX_IDENTITY_BYTES
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
        || value.len() > POTION_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(field);
    }
    Ok(())
}
