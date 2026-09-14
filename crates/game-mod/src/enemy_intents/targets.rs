// SPDX-License-Identifier: MIT

use super::field::EnemyIntentField;
use super::model::validate_identity;

/// Broad target domain when the source can classify it without leaking a hidden target.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentTargetDomain {
    /// No entity is targeted.
    None,
    /// The acting enemy or another source owner is targeted.
    SelfEntity,
    /// One or more player characters are targeted.
    Players,
    /// One or more allied non-player entities are targeted.
    Allies,
    /// One or more enemy entities are targeted.
    Enemies,
    /// Any public combatant can be targeted.
    AnyCombatant,
    /// The source supplied explicit target identities.
    Explicit,
    /// The source could not classify the target domain.
    Unknown,
}

/// Target identities visible on the current public surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentTargetReference {
    /// Stable live target identity, distinct from an intent and enemy definition ID.
    pub target_id: String,
    /// Target family visible to the source.
    pub kind: EnemyIntentTargetKind,
    /// Optional localized label.
    pub label: EnemyIntentField<String>,
}

/// Family of a visible target identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentTargetKind {
    /// The local player character.
    Player,
    /// A co-op or allied player character.
    Ally,
    /// An enemy or hostile creature.
    Enemy,
    /// A summon or other secondary combatant.
    Secondary,
    /// Source supplied an identity but not its family.
    Unknown,
}

/// Target identities and visibility without sentinel IDs for hidden entities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyIntentTargets {
    /// The intent explicitly has no target.
    None,
    /// Target IDs are all visible on the selected source surface.
    Visible(Vec<EnemyIntentTargetReference>),
    /// A target exists or may exist, but its identity is not classified.
    Unknown,
    /// A target exists, but its identity is hidden and must not be represented by a fake ID.
    Hidden,
}

/// Target domain plus the explicit target visibility outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentTargetInfo {
    /// Broad source-reported target domain.
    pub domain: EnemyIntentTargetDomain,
    /// Exact IDs when public, or an explicit no/unknown/hidden state.
    pub targets: EnemyIntentTargets,
}

/// Typed unit for visible damage, block, and healing values.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyIntentUnit(String);

impl EnemyIntentUnit {
    /// Creates a bounded owner-defined unit token.
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        validate_identity(&value, "unit")?;
        Ok(Self(value))
    }

    /// Returns the stable unit token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Visible amount with an explicit unit; zero remains a real observed amount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentAmount {
    /// Non-negative visible amount.
    pub value: u32,
    /// Unit supplied by the source.
    pub unit: EnemyIntentUnit,
}

/// Per-hit and aggregate damage semantics for one attack component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentDamage {
    /// Visible damage applied by one hit when supplied.
    pub per_hit: EnemyIntentField<EnemyIntentAmount>,
    /// Visible number of repeats/hits when supplied.
    pub hits: EnemyIntentField<u16>,
    /// Visible aggregate damage for the component when supplied.
    pub total: EnemyIntentField<EnemyIntentAmount>,
}
