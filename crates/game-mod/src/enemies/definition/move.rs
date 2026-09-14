// SPDX-License-Identifier: MIT

use super::super::model::{
    EnemyMoveReference, EnemyNumericValue, EnemyProbability, EnemyResetBoundary, EnemyTargeting,
    EnemyText, EnemyVisibility,
};
use super::stats::EnemySemanticReference;

/// One source-owned condition or transition predicate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyConditionReference {
    /// Stable condition identity.
    pub condition_id: String,
    /// Localized/source-defined condition label.
    pub label: EnemyText,
    /// Typed parameters used by the condition.
    pub parameters: Vec<EnemyParameter>,
}

/// One visible parameter retained without evaluating hidden state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyParameter {
    /// Stable parameter identity.
    pub parameter_id: String,
    /// Localized/source-defined label.
    pub label: EnemyText,
    /// Optional unit.
    pub unit: Option<String>,
    /// Fixed, formula-backed, or unavailable value.
    pub value: EnemyNumericValue,
}

/// Supported effect families in one move.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyMoveEffectKind {
    /// Deal one or more damage hits.
    Attack,
    /// Add block/guard.
    Block,
    /// Restore health.
    Heal,
    /// Apply a power or status.
    ApplyStatus,
    /// Remove a power or status.
    RemoveStatus,
    /// Create a subordinate enemy.
    Summon,
    /// Leave or escape the encounter.
    Escape,
    /// Enter another behavior phase.
    PhaseChange,
    /// Owner-defined effect family.
    Custom(String),
    /// Source could not classify the effect.
    Unknown,
}

/// One ordered effect in a compound move.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyMoveEffect {
    /// Stable effect identity scoped by the move.
    pub effect_id: String,
    /// Effect family.
    pub kind: EnemyMoveEffectKind,
    /// Localized/source-defined effect description.
    pub description: EnemyText,
    /// Public target domain and count.
    pub targeting: EnemyTargeting,
    /// Typed visible effect parameters.
    pub parameters: Vec<EnemyParameter>,
    /// Typed links to statuses, rules, or effect definitions.
    pub references: Vec<EnemySemanticReference>,
}

/// Cooldown semantics copied from the owner definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyCooldownRule {
    /// Move may be selected without a cooldown.
    None,
    /// Move is unavailable for a number of turns after use.
    Turns(EnemyNumericValue),
    /// Move is unavailable until a source-defined condition.
    UntilCondition(EnemyConditionReference),
    /// Move is available once during the reset boundary.
    Once(EnemyResetBoundary),
    /// Source could not establish cooldown semantics.
    Unknown,
}

/// Repetition restrictions copied without simulating the enemy AI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyRepetitionRule {
    /// Repeated selections are allowed.
    Allow,
    /// The move cannot repeat immediately.
    NoImmediateRepeat,
    /// A maximum number of consecutive selections.
    MaxConsecutive(u32),
    /// Owner-defined restriction.
    Custom(String),
    /// Source could not establish repetition semantics.
    Unknown,
}

/// A source-owned static move definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyMoveDefinitionInput {
    /// Stable move identity scoped by the enemy.
    pub move_id: String,
    /// Localized move name.
    pub name: EnemyText,
    /// Localized move description.
    pub description: EnemyText,
    /// Ordered effects; multiple effects model a compound move.
    pub effects: Vec<EnemyMoveEffect>,
    /// Default targeting domain for the move.
    pub targeting: EnemyTargeting,
    /// Phase identities in which the move is available.
    pub phase_ids: Vec<String>,
    /// Conditions that must hold for selection.
    pub conditions: Vec<EnemyConditionReference>,
    /// Cooldown restriction.
    pub cooldown: EnemyCooldownRule,
    /// Repetition restriction.
    pub repetition: EnemyRepetitionRule,
    /// Selection probability/weight, never evaluated as live RNG.
    pub probability: EnemyProbability,
    /// Typed rule/effect/status references.
    pub references: Vec<EnemySemanticReference>,
    /// Visibility of the static move rule.
    pub visibility: EnemyVisibility,
}

/// A static move definition bound to the owning enemy and catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyMoveDefinition {
    /// Exact move reference.
    pub reference: EnemyMoveReference,
    /// Localized move name.
    pub name: EnemyText,
    /// Localized move description.
    pub description: EnemyText,
    /// Ordered effects.
    pub effects: Vec<EnemyMoveEffect>,
    /// Default targeting domain.
    pub targeting: EnemyTargeting,
    /// Phase identities in which the move is available.
    pub phase_ids: Vec<String>,
    /// Selection conditions.
    pub conditions: Vec<EnemyConditionReference>,
    /// Cooldown restriction.
    pub cooldown: EnemyCooldownRule,
    /// Repetition restriction.
    pub repetition: EnemyRepetitionRule,
    /// Selection probability/weight with evidence label.
    pub probability: EnemyProbability,
    /// Typed references.
    pub references: Vec<EnemySemanticReference>,
    /// Visibility of the static move rule.
    pub visibility: EnemyVisibility,
}
