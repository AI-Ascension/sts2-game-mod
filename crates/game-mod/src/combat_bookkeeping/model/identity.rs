// SPDX-License-Identifier: MIT

use super::super::{
    COMBAT_BOOKKEEPING_MAX_IDENTITY_BYTES, COMBAT_BOOKKEEPING_MAX_TEXT_BYTES,
    CombatBookkeepingError,
};

/// Public or owner-authorized read scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatVisibilityScope {
    /// Player-visible facts only.
    Public,
    /// Explicitly authorized owner facts.
    Owner,
}

/// Explicit availability of one source-owned field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatField<T> {
    /// A value was observed coherently, including zero, false, and empty collections.
    Available(T),
    /// The field has no meaning for this phase.
    NotApplicable,
    /// The source supports the field but did not expose it in this snapshot.
    NotObserved,
    /// No extractor exists for this field.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// The field is visible only to an owner-authorized scope.
    OwnerOnly,
    /// The host is currently resolving this field.
    Busy,
    /// The field became stale while being copied.
    Stale,
    /// The source could not classify this field.
    Unknown,
}

impl<T> CombatField<T> {
    /// Returns the explicit availability state.
    #[must_use]
    pub const fn status(&self) -> CombatFieldStatus {
        match self {
            Self::Available(_) => CombatFieldStatus::Available,
            Self::NotApplicable => CombatFieldStatus::NotApplicable,
            Self::NotObserved => CombatFieldStatus::NotObserved,
            Self::Unsupported => CombatFieldStatus::Unsupported,
            Self::Denied => CombatFieldStatus::Denied,
            Self::OwnerOnly => CombatFieldStatus::OwnerOnly,
            Self::Busy => CombatFieldStatus::Busy,
            Self::Stale => CombatFieldStatus::Stale,
            Self::Unknown => CombatFieldStatus::Unknown,
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

/// Status projection for [`CombatField`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatFieldStatus {
    /// A value was observed.
    Available,
    /// The field does not apply.
    NotApplicable,
    /// A supported field was not exposed.
    NotObserved,
    /// No extractor exists.
    Unsupported,
    /// The caller's scope denied the field.
    Denied,
    /// The field is visible only to an owner.
    OwnerOnly,
    /// A transient resolution is in progress.
    Busy,
    /// The source changed during the read.
    Stale,
    /// The source could not classify the field.
    Unknown,
}

impl CombatFieldStatus {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotApplicable => "not_applicable",
            Self::NotObserved => "not_observed",
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::OwnerOnly => "owner_only",
            Self::Busy => "busy",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}

/// Identity fence for one coherent combat read.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CombatBookkeepingBinding {
    /// Content manifest that owns static card definitions.
    pub content_manifest: String,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Stable combat identity.
    pub combat_id: String,
    /// Coherent source snapshot identity.
    pub snapshot_id: String,
    /// Monotonic epoch for this combat snapshot.
    pub epoch: u64,
}

impl CombatBookkeepingBinding {
    /// Creates a bounded binding.
    pub fn new(
        content_manifest: impl Into<String>,
        game_instance_id: impl Into<String>,
        run_id: impl Into<String>,
        combat_id: impl Into<String>,
        snapshot_id: impl Into<String>,
        epoch: u64,
    ) -> Result<Self, CombatBookkeepingError> {
        let binding = Self {
            content_manifest: content_manifest.into(),
            game_instance_id: game_instance_id.into(),
            run_id: run_id.into(),
            combat_id: combat_id.into(),
            snapshot_id: snapshot_id.into(),
            epoch,
        };
        for (field, value) in [
            ("content_manifest", binding.content_manifest.as_str()),
            ("game_instance_id", binding.game_instance_id.as_str()),
            ("run_id", binding.run_id.as_str()),
            ("combat_id", binding.combat_id.as_str()),
            ("snapshot_id", binding.snapshot_id.as_str()),
        ] {
            validate_identity(value, field)?;
        }
        Ok(binding)
    }
}

/// Distinct live card identity; definition and instance IDs never collapse.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CombatCardInstanceReference {
    /// Snapshot fence that created this reference.
    pub binding: CombatBookkeepingBinding,
    /// Distinct live card identity.
    pub instance_id: String,
    /// Common static definition identity.
    pub definition_id: String,
    /// Owning entity identity.
    pub owner_id: String,
}

impl CombatCardInstanceReference {
    /// Creates a card reference bound to one coherent snapshot.
    pub fn new(
        binding: CombatBookkeepingBinding,
        instance_id: impl Into<String>,
        definition_id: impl Into<String>,
        owner_id: impl Into<String>,
    ) -> Result<Self, CombatBookkeepingError> {
        let reference = Self {
            binding,
            instance_id: instance_id.into(),
            definition_id: definition_id.into(),
            owner_id: owner_id.into(),
        };
        validate_identity(&reference.instance_id, "instance_id")?;
        validate_identity(&reference.definition_id, "definition_id")?;
        validate_identity(&reference.owner_id, "owner_id")?;
        Ok(reference)
    }
}

pub(crate) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), CombatBookkeepingError> {
    if value.is_empty()
        || value.len() > COMBAT_BOOKKEEPING_MAX_IDENTITY_BYTES
        || value.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(character, '"' | '\\' | '{' | '}' | '[' | ']')
        })
    {
        return Err(CombatBookkeepingError::InvalidBinding(field));
    }
    Ok(())
}

pub(crate) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), CombatBookkeepingError> {
    if value.is_empty()
        || value.len() > COMBAT_BOOKKEEPING_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(CombatBookkeepingError::InvalidInput(field));
    }
    Ok(())
}
