// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family covered by the owner-local character-state producer.
pub const CHARACTER_STATE_ENTITY_KIND: &str = "character_state";
/// Source-only producer version; this is not a transport or native ABI version.
pub const CHARACTER_STATE_PRODUCER_VERSION: &str = "game-character-state-producer-v1";
/// Maximum bytes accepted for one identity token.
pub const CHARACTER_STATE_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const CHARACTER_STATE_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum bytes accepted for one static definition.
pub const CHARACTER_STATE_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum bytes accepted for one complete live detail.
pub const CHARACTER_STATE_MAX_DETAIL_BYTES: usize = 32 * 1024;
/// Maximum static definitions retained in one source snapshot.
pub const CHARACTER_STATE_MAX_DEFINITIONS: usize = 16 * 1024;
/// Maximum character/mode coverage records retained in one source snapshot.
pub const CHARACTER_STATE_MAX_COVERAGE: usize = 512;
/// Maximum entries returned by one static definition page.
pub const CHARACTER_STATE_MAX_PAGE_ITEMS: usize = 64;
/// Maximum live resource instances in one coherent snapshot.
pub const CHARACTER_STATE_MAX_LIVE_RESOURCES: usize = 512;
/// Maximum live secondary entities in one coherent snapshot.
pub const CHARACTER_STATE_MAX_LIVE_SECONDARY_ENTITIES: usize = 512;
/// Maximum ordered slots on one resource.
pub const CHARACTER_STATE_MAX_SLOTS: usize = 64;
/// Maximum statuses attached to one secondary entity.
pub const CHARACTER_STATE_MAX_STATUSES: usize = 64;
/// Maximum intents retained on one secondary entity.
pub const CHARACTER_STATE_MAX_INTENTS: usize = 16;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterStateCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized value.
    pub locale: String,
    /// Exact owner-local producer version.
    pub producer_version: String,
}

/// Live identity adds game, run, mode, snapshot, and epoch fences to catalog identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterStateLiveBinding {
    /// Static catalog identity used by the live read.
    pub catalog: CharacterStateCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Character mode/difficulty identity.
    pub mode_id: String,
    /// Coherent observation identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

/// Opaque identity of a live owner.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterStateOwnerId(String);

impl CharacterStateOwnerId {
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

/// Owner family for character resources and secondary entities.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterStateOwnerKind {
    /// A player-controlled owner.
    Player,
    /// A friendly co-op player, ally, or companion controller.
    Ally,
    /// A hostile owner when a character-specific state is visible.
    Enemy,
    /// A controlled secondary entity.
    Secondary,
    /// A source-defined owner family.
    Custom(String),
}

/// Live owner identity and visible character link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateOwner {
    /// Owner family.
    pub kind: CharacterStateOwnerKind,
    /// Stable live owner identity.
    pub id: CharacterStateOwnerId,
    /// Character definition identity controlling this state.
    pub character_id: String,
    /// Localized owner label when visible.
    pub label: CharacterStateField<String>,
}

/// Unit used by a typed resource value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterStateUnit(String);

impl CharacterStateUnit {
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

/// Visibility of static or live source values.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterStateVisibility {
    /// Value is visible on the supported public surface.
    Visible,
    /// Value is visible only to an explicit owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope used for static and live reads.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterStateVisibilityScope {
    /// Public player-visible values.
    Public,
    /// Explicit owner-authorized values.
    Owner,
}

/// Explicit availability state for one source-owned field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterStateField<T> {
    /// A value was observed, including zero, false, and an explicit empty collection.
    Available(T),
    /// The field has no meaning for this owner or phase.
    NotApplicable,
    /// The field is supported but outside this snapshot's observable surface.
    NotObserved,
    /// No extractor is supported for this field.
    Unsupported,
    /// The caller or source scope denied the field.
    Denied,
    /// A read was attempted but the source was unavailable.
    Unavailable,
    /// The source could not classify the field without inventing a value.
    Unknown,
}

impl<T> CharacterStateField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> CharacterStateFieldStatus {
        match self {
            Self::Available(_) => CharacterStateFieldStatus::Available,
            Self::NotApplicable => CharacterStateFieldStatus::NotApplicable,
            Self::NotObserved => CharacterStateFieldStatus::NotObserved,
            Self::Unsupported => CharacterStateFieldStatus::Unsupported,
            Self::Denied => CharacterStateFieldStatus::Denied,
            Self::Unavailable => CharacterStateFieldStatus::Unavailable,
            Self::Unknown => CharacterStateFieldStatus::Unknown,
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

/// Projection for [`CharacterStateField`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterStateFieldStatus {
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
    /// The read was unavailable.
    Unavailable,
    /// The source could not classify the field.
    Unknown,
}

/// Explicit coverage for one character and mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterMechanicCoverage {
    /// Character definition identity.
    pub character_id: String,
    /// Mode/difficulty identity.
    pub mode_id: String,
    /// Resource support state.
    pub resources: CharacterMechanicState,
    /// Secondary-entity support state.
    pub secondary_entities: CharacterMechanicState,
}

/// Source support state; absence is never interpreted as an empty mechanic.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterMechanicState {
    /// Typed records are supported by this source.
    Supported,
    /// The selected build has no supported extractor.
    Unsupported,
    /// The mechanic does not apply to this character/mode.
    NotApplicable,
    /// The source is currently unavailable.
    Unavailable,
    /// The source could not classify support.
    Unknown,
}

/// Exact live resource reference bound to one coherent snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterResourceReference {
    /// Coherent live identity.
    pub live: CharacterStateLiveBinding,
    /// Distinct live resource identity.
    pub instance_id: String,
    /// Static resource definition identity.
    pub definition_id: String,
    /// Owner identity at the same snapshot.
    pub owner_id: CharacterStateOwnerId,
}

/// Exact live secondary-entity reference bound to one coherent snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterSecondaryEntityReference {
    /// Coherent live identity.
    pub live: CharacterStateLiveBinding,
    /// Distinct live entity identity.
    pub instance_id: String,
    /// Static entity definition identity.
    pub definition_id: String,
    /// Owner identity at the same snapshot.
    pub owner_id: CharacterStateOwnerId,
}

pub(crate) fn validate_identity(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > CHARACTER_STATE_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(field);
    }
    Ok(())
}

pub(crate) fn validate_text(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.len() > CHARACTER_STATE_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(field);
    }
    Ok(())
}
