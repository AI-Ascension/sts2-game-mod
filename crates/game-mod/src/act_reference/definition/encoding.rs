// SPDX-License-Identifier: MIT

use super::super::model::{
    ActField, ActNumericValue, ActSemanticReference, ActSemanticReferenceKind, ActText,
    GenerationWeight,
};
use super::*;

/// Returns a conservative byte estimate for one source-owned act definition.
pub(crate) fn definition_bytes(input: &ActDefinitionInput) -> usize {
    let mut total =
        input.act_id.len() + text_bytes(&input.name) + text_bytes(&input.description) + 8;
    total += input
        .room_categories
        .iter()
        .map(room_category_bytes)
        .sum::<usize>();
    total += input.encounters.iter().map(encounter_bytes).sum::<usize>();
    total += field_bytes(&input.pools, |pools| pools.iter().map(pool_bytes).sum());
    total += field_bytes(&input.constraints, |constraints| {
        constraints.iter().map(constraint_bytes).sum()
    });
    total += input.references.iter().map(reference_bytes).sum::<usize>();
    total
}

fn text_bytes(value: &ActText) -> usize {
    match value {
        ActText::Available(value) => value.len(),
        ActText::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &ActField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        ActField::Available(value) => available(value),
        ActField::Unavailable(_) => 1,
    }
}

fn numeric_bytes(value: &ActNumericValue) -> usize {
    match value {
        ActNumericValue::Fixed(_) => 8,
        ActNumericValue::Formula(formula) => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        ActNumericValue::Unavailable(_) => 1,
    }
}

fn weight_bytes(weight: &GenerationWeight) -> usize {
    match weight {
        GenerationWeight::Exact { .. } => 12,
        GenerationWeight::Rule { rule_reference, .. } => rule_reference.len() + 1,
        GenerationWeight::Unavailable(_) => 1,
    }
}

fn semantic_kind_bytes(kind: &ActSemanticReferenceKind) -> usize {
    match kind {
        ActSemanticReferenceKind::Content { entity_kind } => entity_kind.len(),
        ActSemanticReferenceKind::Enemy
        | ActSemanticReferenceKind::Encounter
        | ActSemanticReferenceKind::Act
        | ActSemanticReferenceKind::RoomCategory
        | ActSemanticReferenceKind::Effect
        | ActSemanticReferenceKind::Rule
        | ActSemanticReferenceKind::Condition
        | ActSemanticReferenceKind::Unknown => 1,
    }
}

fn reference_bytes(reference: &ActSemanticReference) -> usize {
    semantic_kind_bytes(&reference.kind) + reference.id.len() + text_bytes(&reference.label) + 2
}

fn parameter_bytes(parameter: &ActParameter) -> usize {
    parameter.parameter_id.len()
        + text_bytes(&parameter.label)
        + parameter.unit.as_deref().map_or(0, str::len)
        + numeric_bytes(&parameter.value)
        + 2
}

fn eligibility_kind_bytes(kind: &EligibilityKind) -> usize {
    match kind {
        EligibilityKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn eligibility_bytes(condition: &EligibilityCondition) -> usize {
    condition.condition_id.len()
        + eligibility_kind_bytes(&condition.kind)
        + text_bytes(&condition.label)
        + condition
            .parameters
            .iter()
            .map(parameter_bytes)
            .sum::<usize>()
        + condition
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}

fn room_category_kind_bytes(kind: &RoomCategoryKind) -> usize {
    match kind {
        RoomCategoryKind::Custom(value) | RoomCategoryKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn room_category_bytes(category: &RoomCategoryDefinition) -> usize {
    category.category_id.len()
        + text_bytes(&category.name)
        + room_category_kind_bytes(&category.kind)
        + text_bytes(&category.description)
        + category
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 3
}

fn enemy_bytes(enemy: &EncounterEnemy) -> usize {
    enemy.enemy_id.len()
        + numeric_bytes(&enemy.quantity)
        + enemy.variant_ids.iter().map(String::len).sum::<usize>()
        + enemy.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn group_bytes(group: &EncounterEnemyGroup) -> usize {
    group.group_id.len()
        + text_bytes(&group.label)
        + group.enemies.iter().map(enemy_bytes).sum::<usize>()
        + group.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn encounter_kind_bytes(kind: &EncounterKind) -> usize {
    match kind {
        EncounterKind::Custom(value) | EncounterKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn encounter_bytes(encounter: &EncounterDefinitionInput) -> usize {
    encounter.encounter_id.len()
        + text_bytes(&encounter.name)
        + encounter_kind_bytes(&encounter.kind)
        + encounter.room_category_id.as_deref().map_or(0, str::len)
        + encounter.groups.iter().map(group_bytes).sum::<usize>()
        + encounter
            .eligibility
            .iter()
            .map(eligibility_bytes)
            .sum::<usize>()
        + weight_bytes(&encounter.weight)
        + encounter
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}

fn pool_kind_bytes(kind: &EncounterPoolKind) -> usize {
    match kind {
        EncounterPoolKind::Custom(value) | EncounterPoolKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn pool_entry_bytes(entry: &EncounterPoolEntry) -> usize {
    entry.entry_id.len() + entry.encounter_id.len() + weight_bytes(&entry.weight) + 2
}

fn pool_bytes(pool: &EncounterPool) -> usize {
    pool.pool_id.len()
        + text_bytes(&pool.name)
        + pool_kind_bytes(&pool.kind)
        + pool.room_category_id.as_deref().map_or(0, str::len)
        + pool.entries.iter().map(pool_entry_bytes).sum::<usize>()
        + pool.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn constraint_kind_bytes(kind: &MapConstraintKind) -> usize {
    match kind {
        MapConstraintKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn constraint_bytes(constraint: &MapGenerationConstraint) -> usize {
    constraint.constraint_id.len()
        + constraint_kind_bytes(&constraint.kind)
        + text_bytes(&constraint.label)
        + field_bytes(&constraint.rule_reference, String::len)
        + constraint.mode.as_deref().map_or(0, str::len)
        + constraint.difficulty.as_deref().map_or(0, str::len)
        + constraint
            .parameters
            .iter()
            .map(parameter_bytes)
            .sum::<usize>()
        + constraint
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 3
}
