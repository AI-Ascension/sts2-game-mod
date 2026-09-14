// SPDX-License-Identifier: MIT

use super::super::model::{
    EnemyField, EnemyKind, EnemyNumericValue, EnemyOrigin, EnemyProbability, EnemyTargeting,
    EnemyText,
};
use super::{
    EnemyBehaviorTransition, EnemyConditionReference, EnemyCooldownRule, EnemyDefinitionInput,
    EnemyEncounterReference, EnemyMoveDefinitionInput, EnemyMoveEffect, EnemyMoveEffectKind,
    EnemyOriginVariant, EnemyParameter, EnemyPhaseDefinition, EnemyRepetitionRule,
    EnemySemanticReference, EnemySemanticReferenceKind, EnemyStat, EnemyStatProfile, EnemyStats,
    EnemyTag,
};

/// Returns a conservative byte estimate for one source-owned definition.
pub(crate) fn definition_bytes(input: &EnemyDefinitionInput) -> usize {
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
