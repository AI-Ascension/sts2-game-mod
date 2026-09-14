// SPDX-License-Identifier: MIT

use crate::ContentUnlockState;

use super::model::{
    EnemyCatalogBinding, EnemyDefinitionReference, EnemyField, EnemyKind, EnemyMoveReference,
    EnemyNumericValue, EnemyOrigin, EnemyProbability, EnemyResetBoundary, EnemyTargeting,
    EnemyText, EnemyVisibility,
};

/// One named stat with an explicit unit and fixed/formula/unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStat {
    /// Stable owner-defined stat identity.
    pub stat_id: String,
    /// Optional unit for the value, such as `hp` or `count`.
    pub unit: Option<String>,
    /// Observed, dynamic, or unavailable value.
    pub value: EnemyNumericValue,
}

/// One mode/difficulty-scaled stat profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStatProfile {
    /// Stable profile identity scoped by the enemy.
    pub profile_id: String,
    /// Source-defined mode identity, if present.
    pub mode: Option<String>,
    /// Source-defined difficulty identity, if present.
    pub difficulty: Option<String>,
    /// Profile-specific stats.
    pub stats: Vec<EnemyStat>,
}

/// Base stats and mode/difficulty-scaled profiles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStats {
    /// Base/normal definition values.
    pub base: EnemyField<Vec<EnemyStat>>,
    /// Mode/difficulty variants, including explicit empty/unknown states.
    pub scaled: EnemyField<Vec<EnemyStatProfile>>,
}

/// Stable tag reference attached to an enemy definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyTag {
    /// Owner-defined tag identity.
    pub tag_id: String,
    /// Localized/source-defined tag label.
    pub label: EnemyText,
}

/// Typed semantic reference to a status, encounter, effect, condition, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemySemanticReference {
    /// Reference family.
    pub kind: EnemySemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Localized/source-defined label.
    pub label: EnemyText,
}

/// Supported semantic reference families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemySemanticReferenceKind {
    /// A power/status definition in the content manifest.
    Status,
    /// An encounter definition in the content manifest.
    Encounter,
    /// An effect definition or rule link.
    Effect,
    /// A transition or timing rule.
    Rule,
    /// A source-owned condition.
    Condition,
    /// An owner-defined content family not otherwise named here.
    Content { entity_kind: String },
    /// A family not classified by the source.
    Unknown,
}

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

/// One encounter where an enemy can spawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyEncounterReference {
    /// Encounter definition identity.
    pub encounter_id: String,
    /// Optional localized/source-defined encounter label.
    pub label: EnemyText,
    /// Optional owner-defined encounter role.
    pub role: Option<String>,
}

/// An origin/package variant retained separately from the base definition origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyOriginVariant {
    /// Stable variant identity scoped by the enemy.
    pub variant_id: String,
    /// Localized/source-defined variant label.
    pub label: EnemyText,
    /// Variant provenance.
    pub origin: EnemyOrigin,
    /// Variant stats when the source can provide them.
    pub stats: EnemyField<EnemyStats>,
    /// Variant move IDs when the source overrides the base move set.
    pub move_ids: EnemyField<Vec<String>>,
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

/// One behavior phase and its move membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyPhaseDefinition {
    /// Stable phase identity.
    pub phase_id: String,
    /// Localized phase name.
    pub name: EnemyText,
    /// Localized phase description.
    pub description: EnemyText,
    /// Stable ordering within the definition.
    pub order: u16,
    /// Moves available in this phase.
    pub move_ids: Vec<String>,
    /// Optional entry predicate.
    pub entry_condition: EnemyField<EnemyConditionReference>,
}

/// One transition between behavior phases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyBehaviorTransition {
    /// Stable transition identity.
    pub transition_id: String,
    /// Optional source phase; `None` means the initial phase.
    pub from_phase: Option<String>,
    /// Destination phase identity.
    pub to_phase: String,
    /// Predicate for the transition.
    pub condition: EnemyConditionReference,
    /// Optional source-defined probability/weight.
    pub probability: EnemyProbability,
    /// Typed rule/effect/status references.
    pub references: Vec<EnemySemanticReference>,
}

/// Complete source-owned static enemy definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinitionInput {
    /// Namespaced enemy identity.
    pub enemy_id: String,
    /// Localized enemy name.
    pub name: EnemyText,
    /// Localized enemy description.
    pub description: EnemyText,
    /// Enemy role.
    pub kind: EnemyKind,
    /// Source origin/provenance.
    pub origin: EnemyOrigin,
    /// Explicit unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition itself.
    pub visibility: EnemyVisibility,
    /// Stable tags.
    pub tags: Vec<EnemyTag>,
    /// Base and scaled stats.
    pub stats: EnemyStats,
    /// Spawn predicates, preserving unavailable/empty distinctions.
    pub spawn_conditions: EnemyField<Vec<EnemyConditionReference>>,
    /// Encounter references, preserving unavailable/empty distinctions.
    pub encounters: EnemyField<Vec<EnemyEncounterReference>>,
    /// Origin/package variants.
    pub origin_variants: Vec<EnemyOriginVariant>,
    /// Behavior phases.
    pub phases: Vec<EnemyPhaseDefinition>,
    /// Supported move rules.
    pub moves: Vec<EnemyMoveDefinitionInput>,
    /// Phase transitions and conditional behavior.
    pub transitions: Vec<EnemyBehaviorTransition>,
    /// Top-level status/encounter/rule links.
    pub references: Vec<EnemySemanticReference>,
}

/// Immutable enemy definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinition {
    /// Exact static definition reference.
    pub reference: EnemyDefinitionReference,
    /// Localized enemy name.
    pub name: EnemyText,
    /// Localized enemy description.
    pub description: EnemyText,
    /// Enemy role.
    pub kind: EnemyKind,
    /// Source origin/provenance.
    pub origin: EnemyOrigin,
    /// Unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition.
    pub visibility: EnemyVisibility,
    /// Stable tags.
    pub tags: Vec<EnemyTag>,
    /// Base and scaled stats.
    pub stats: EnemyStats,
    /// Spawn predicates.
    pub spawn_conditions: EnemyField<Vec<EnemyConditionReference>>,
    /// Encounter references.
    pub encounters: EnemyField<Vec<EnemyEncounterReference>>,
    /// Origin/package variants.
    pub origin_variants: Vec<EnemyOriginVariant>,
    /// Behavior phases.
    pub phases: Vec<EnemyPhaseDefinition>,
    /// Supported move rules.
    pub moves: Vec<EnemyMoveDefinition>,
    /// Phase transitions and conditional behavior.
    pub transitions: Vec<EnemyBehaviorTransition>,
    /// Top-level references.
    pub references: Vec<EnemySemanticReference>,
}

impl EnemyDefinition {
    /// Binds an input definition and all of its move references to one catalog.
    pub(super) fn from_input(binding: &EnemyCatalogBinding, input: EnemyDefinitionInput) -> Self {
        let enemy_id = input.enemy_id.clone();
        let moves = input
            .moves
            .into_iter()
            .map(|move_input| EnemyMoveDefinition {
                reference: EnemyMoveReference {
                    catalog: binding.clone(),
                    enemy_id: enemy_id.clone(),
                    move_id: move_input.move_id,
                },
                name: move_input.name,
                description: move_input.description,
                effects: move_input.effects,
                targeting: move_input.targeting,
                phase_ids: move_input.phase_ids,
                conditions: move_input.conditions,
                cooldown: move_input.cooldown,
                repetition: move_input.repetition,
                probability: move_input.probability,
                references: move_input.references,
                visibility: move_input.visibility,
            })
            .collect();
        Self {
            reference: EnemyDefinitionReference {
                catalog: binding.clone(),
                enemy_id,
            },
            name: input.name,
            description: input.description,
            kind: input.kind,
            origin: input.origin,
            unlock_state: input.unlock_state,
            visibility: input.visibility,
            tags: input.tags,
            stats: input.stats,
            spawn_conditions: input.spawn_conditions,
            encounters: input.encounters,
            origin_variants: input.origin_variants,
            phases: input.phases,
            moves,
            transitions: input.transitions,
            references: input.references,
        }
    }
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: EnemyFamilyState,
    /// Number of enemy definitions in the manifest.
    pub definition_count: usize,
}

/// Source support state for the enemy family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyFamilyState {
    /// Source can project all enemy records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Returns a conservative byte estimate for one source-owned definition.
pub(super) fn definition_bytes(input: &EnemyDefinitionInput) -> usize {
    let mut total = input.enemy_id.len()
        + text_bytes(&input.name)
        + text_bytes(&input.description)
        + kind_bytes(&input.kind)
        + origin_bytes(&input.origin)
        + 8;
    total += input.tags.iter().map(tag_bytes).sum::<usize>();
    total += stats_bytes(&input.stats);
    total += field_bytes(&input.spawn_conditions, |values| {
        values.iter().map(condition_bytes).sum()
    });
    total += field_bytes(&input.encounters, |values| {
        values.iter().map(encounter_bytes).sum()
    });
    total += input
        .origin_variants
        .iter()
        .map(origin_variant_bytes)
        .sum::<usize>();
    total += input.phases.iter().map(phase_bytes).sum::<usize>();
    total += input.moves.iter().map(move_bytes).sum::<usize>();
    total += input
        .transitions
        .iter()
        .map(transition_bytes)
        .sum::<usize>();
    total += input.references.iter().map(reference_bytes).sum::<usize>();
    total
}

fn text_bytes(value: &EnemyText) -> usize {
    match value {
        EnemyText::Available(value) => value.len(),
        EnemyText::Unavailable(_) => 1,
    }
}

fn numeric_bytes(value: &EnemyNumericValue) -> usize {
    match value {
        EnemyNumericValue::Fixed(_) => 8,
        EnemyNumericValue::Formula(formula) => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        EnemyNumericValue::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &EnemyField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        EnemyField::Available(value) => available(value),
        EnemyField::Unavailable(_) => 1,
    }
}

fn kind_bytes(kind: &EnemyKind) -> usize {
    match kind {
        EnemyKind::Custom(value) => value.len(),
        EnemyKind::Normal | EnemyKind::Elite | EnemyKind::Boss | EnemyKind::Minion => 1,
    }
}

fn origin_bytes(origin: &EnemyOrigin) -> usize {
    origin.kind.len()
        + origin.package_id.as_deref().map_or(0, str::len)
        + origin.package_version.as_deref().map_or(0, str::len)
        + 2
}

fn tag_bytes(tag: &EnemyTag) -> usize {
    tag.tag_id.len() + text_bytes(&tag.label) + 2
}

fn stats_bytes(stats: &EnemyStats) -> usize {
    field_bytes(&stats.base, |values| values.iter().map(stat_bytes).sum())
        + field_bytes(&stats.scaled, |profiles| {
            profiles.iter().map(profile_bytes).sum()
        })
}

fn stat_bytes(stat: &EnemyStat) -> usize {
    stat.stat_id.len() + stat.unit.as_deref().map_or(0, str::len) + numeric_bytes(&stat.value) + 2
}

fn profile_bytes(profile: &EnemyStatProfile) -> usize {
    profile.profile_id.len()
        + profile.mode.as_deref().map_or(0, str::len)
        + profile.difficulty.as_deref().map_or(0, str::len)
        + profile.stats.iter().map(stat_bytes).sum::<usize>()
        + 2
}

fn parameter_bytes(parameter: &EnemyParameter) -> usize {
    parameter.parameter_id.len()
        + text_bytes(&parameter.label)
        + parameter.unit.as_deref().map_or(0, str::len)
        + numeric_bytes(&parameter.value)
        + 2
}

fn condition_bytes(condition: &EnemyConditionReference) -> usize {
    condition.condition_id.len()
        + text_bytes(&condition.label)
        + condition
            .parameters
            .iter()
            .map(parameter_bytes)
            .sum::<usize>()
        + 2
}

fn encounter_bytes(encounter: &EnemyEncounterReference) -> usize {
    encounter.encounter_id.len()
        + text_bytes(&encounter.label)
        + encounter.role.as_deref().map_or(0, str::len)
        + 2
}

fn origin_variant_bytes(variant: &EnemyOriginVariant) -> usize {
    variant.variant_id.len()
        + text_bytes(&variant.label)
        + origin_bytes(&variant.origin)
        + field_bytes(&variant.stats, stats_bytes)
        + field_bytes(&variant.move_ids, |ids| ids.iter().map(String::len).sum())
        + 2
}

fn targeting_bytes(targeting: &EnemyTargeting) -> usize {
    numeric_bytes(&targeting.count) + 1
}

fn semantic_kind_bytes(kind: &EnemySemanticReferenceKind) -> usize {
    match kind {
        EnemySemanticReferenceKind::Content { entity_kind } => entity_kind.len(),
        EnemySemanticReferenceKind::Status
        | EnemySemanticReferenceKind::Encounter
        | EnemySemanticReferenceKind::Effect
        | EnemySemanticReferenceKind::Rule
        | EnemySemanticReferenceKind::Condition
        | EnemySemanticReferenceKind::Unknown => 1,
    }
}

fn reference_bytes(reference: &EnemySemanticReference) -> usize {
    semantic_kind_bytes(&reference.kind) + reference.id.len() + text_bytes(&reference.label) + 2
}

fn effect_kind_bytes(kind: &EnemyMoveEffectKind) -> usize {
    match kind {
        EnemyMoveEffectKind::Custom(value) => value.len(),
        EnemyMoveEffectKind::Attack
        | EnemyMoveEffectKind::Block
        | EnemyMoveEffectKind::Heal
        | EnemyMoveEffectKind::ApplyStatus
        | EnemyMoveEffectKind::RemoveStatus
        | EnemyMoveEffectKind::Summon
        | EnemyMoveEffectKind::Escape
        | EnemyMoveEffectKind::PhaseChange
        | EnemyMoveEffectKind::Unknown => 1,
    }
}

fn effect_bytes(effect: &EnemyMoveEffect) -> usize {
    effect.effect_id.len()
        + effect_kind_bytes(&effect.kind)
        + text_bytes(&effect.description)
        + targeting_bytes(&effect.targeting)
        + effect.parameters.iter().map(parameter_bytes).sum::<usize>()
        + effect.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn cooldown_bytes(rule: &EnemyCooldownRule) -> usize {
    match rule {
        EnemyCooldownRule::Turns(value) => numeric_bytes(value),
        EnemyCooldownRule::UntilCondition(condition) => condition_bytes(condition),
        EnemyCooldownRule::Once(_) => 1,
        EnemyCooldownRule::None | EnemyCooldownRule::Unknown => 1,
    }
}

fn repetition_bytes(rule: &EnemyRepetitionRule) -> usize {
    match rule {
        EnemyRepetitionRule::Custom(value) => value.len(),
        EnemyRepetitionRule::MaxConsecutive(value) => {
            usize::try_from(*value).unwrap_or(usize::MAX).min(8)
        }
        EnemyRepetitionRule::Allow
        | EnemyRepetitionRule::NoImmediateRepeat
        | EnemyRepetitionRule::Unknown => 1,
    }
}

fn probability_bytes(probability: &EnemyProbability) -> usize {
    match probability {
        EnemyProbability::Exact { .. } => 12,
        EnemyProbability::Formula { formula, .. } => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        EnemyProbability::Unavailable(_) => 1,
    }
}

fn phase_bytes(phase: &EnemyPhaseDefinition) -> usize {
    phase.phase_id.len()
        + text_bytes(&phase.name)
        + text_bytes(&phase.description)
        + phase.move_ids.iter().map(String::len).sum::<usize>()
        + field_bytes(&phase.entry_condition, condition_bytes)
        + 4
}

fn move_bytes(move_input: &EnemyMoveDefinitionInput) -> usize {
    move_input.move_id.len()
        + text_bytes(&move_input.name)
        + text_bytes(&move_input.description)
        + move_input.effects.iter().map(effect_bytes).sum::<usize>()
        + targeting_bytes(&move_input.targeting)
        + move_input.phase_ids.iter().map(String::len).sum::<usize>()
        + move_input
            .conditions
            .iter()
            .map(condition_bytes)
            .sum::<usize>()
        + cooldown_bytes(&move_input.cooldown)
        + repetition_bytes(&move_input.repetition)
        + probability_bytes(&move_input.probability)
        + move_input
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 4
}

fn transition_bytes(transition: &EnemyBehaviorTransition) -> usize {
    transition.transition_id.len()
        + transition.from_phase.as_deref().map_or(0, str::len)
        + transition.to_phase.len()
        + condition_bytes(&transition.condition)
        + probability_bytes(&transition.probability)
        + transition
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}
