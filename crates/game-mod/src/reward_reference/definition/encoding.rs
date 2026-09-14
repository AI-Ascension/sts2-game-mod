// SPDX-License-Identifier: MIT

use super::super::model::{
    RewardField, RewardNumericValue, RewardProbability, RewardQuantity, RewardRarityWeight,
    RewardSemanticReference, RewardSemanticReferenceKind, RewardText,
};
use super::*;

/// Returns a conservative byte estimate for one source-owned reward definition.
pub(crate) fn definition_bytes(input: &RewardOfferDefinitionInput) -> usize {
    let mut total = input.reward_id.len() + text_bytes(&input.label) + kind_bytes(&input.kind) + 2;
    total += selection_bytes(&input.selection);
    total += input.items.iter().map(item_bytes).sum::<usize>();
    total += input.generation.iter().map(rule_bytes).sum::<usize>();
    total += state_policy_bytes(&input.state_policy);
    total += input.references.iter().map(reference_bytes).sum::<usize>();
    total
}

fn text_bytes(value: &RewardText) -> usize {
    match value {
        RewardText::Available(value) => value.len(),
        RewardText::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &RewardField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        RewardField::Available(value) => available(value),
        RewardField::Unavailable(_) => 1,
    }
}

fn numeric_bytes(value: &RewardNumericValue) -> usize {
    match value {
        RewardNumericValue::Fixed(_) => 8,
        RewardNumericValue::Formula(formula) => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        RewardNumericValue::Unavailable(_) => 1,
    }
}

fn probability_bytes(probability: &RewardProbability) -> usize {
    match probability {
        RewardProbability::Exact { .. } => 12,
        RewardProbability::Rule { rule_reference, .. } => rule_reference.len() + 1,
        RewardProbability::Conditional {
            rule_reference,
            condition_reference,
            ..
        } => rule_reference.len() + condition_reference.len() + 2,
        RewardProbability::Unavailable(_) => 1,
    }
}

fn quantity_bytes(quantity: &RewardQuantity) -> usize {
    field_bytes(&quantity.unit, String::len)
        + numeric_bytes(&quantity.base_amount)
        + numeric_bytes(&quantity.visible_amount)
        + field_bytes(&quantity.modified, |_| 1)
        + 1
}

fn semantic_kind_bytes(kind: &RewardSemanticReferenceKind) -> usize {
    match kind {
        RewardSemanticReferenceKind::Content { entity_kind } => entity_kind.len(),
        RewardSemanticReferenceKind::Reward
        | RewardSemanticReferenceKind::Card
        | RewardSemanticReferenceKind::Relic
        | RewardSemanticReferenceKind::Potion
        | RewardSemanticReferenceKind::Currency
        | RewardSemanticReferenceKind::Item
        | RewardSemanticReferenceKind::Rule
        | RewardSemanticReferenceKind::Condition
        | RewardSemanticReferenceKind::Modifier
        | RewardSemanticReferenceKind::Unknown => 1,
    }
}

fn reference_bytes(reference: &RewardSemanticReference) -> usize {
    semantic_kind_bytes(&reference.kind) + reference.id.len() + text_bytes(&reference.label) + 2
}

fn action_kind_bytes(kind: &RewardActionKind) -> usize {
    match kind {
        RewardActionKind::Custom(value) | RewardActionKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn legal_action_bytes(action: &RewardLegalAction) -> usize {
    action_kind_bytes(&action.action_kind)
        + text_bytes(&action.label)
        + field_bytes(&action.condition, String::len)
        + 2
}

fn selection_bytes(selection: &RewardSelectionInput) -> usize {
    selection.group_id.len()
        + text_bytes(&selection.label)
        + selection
            .legal_actions
            .iter()
            .map(legal_action_bytes)
            .sum::<usize>()
        + selection
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 6
}

fn requirement_kind_bytes(kind: &RewardRequirementKind) -> usize {
    match kind {
        RewardRequirementKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn parameter_bytes(parameter: &RewardParameter) -> usize {
    parameter.parameter_id.len()
        + text_bytes(&parameter.label)
        + parameter.unit.as_deref().map_or(0, str::len)
        + numeric_bytes(&parameter.value)
        + 2
}

fn requirement_bytes(requirement: &RewardRequirement) -> usize {
    requirement.requirement_id.len()
        + requirement_kind_bytes(&requirement.kind)
        + text_bytes(&requirement.label)
        + requirement
            .parameters
            .iter()
            .map(parameter_bytes)
            .sum::<usize>()
        + requirement
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}

fn modifier_kind_bytes(kind: &RewardModifierKind) -> usize {
    match kind {
        RewardModifierKind::Custom(value) | RewardModifierKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn modifier_bytes(modifier: &RewardModifier) -> usize {
    modifier.modifier_id.len()
        + modifier_kind_bytes(&modifier.kind)
        + text_bytes(&modifier.label)
        + quantity_bytes(&modifier.amount)
        + field_bytes(&modifier.condition, String::len)
        + field_bytes(&modifier.rule_reference, String::len)
        + modifier
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 3
}

fn rarity_weight_bytes(weight: &RewardRarityWeight) -> usize {
    weight.rarity.len()
        + quantity_bytes(&weight.weight)
        + probability_bytes(&weight.probability)
        + 2
}

fn rule_bytes(rule: &RewardGenerationRule) -> usize {
    rule.rule_id.len()
        + text_bytes(&rule.label)
        + field_bytes(&rule.pool, |pool| {
            pool.iter().map(reference_bytes).sum::<usize>()
        })
        + rule
            .rarity_weights
            .iter()
            .map(rarity_weight_bytes)
            .sum::<usize>()
        + rule
            .eligibility
            .iter()
            .map(requirement_bytes)
            .sum::<usize>()
        + rule.modifiers.iter().map(modifier_bytes).sum::<usize>()
        + probability_bytes(&rule.probability)
        + rule.references.iter().map(reference_bytes).sum::<usize>()
        + 4
}

fn item_bytes(item: &RewardItemInput) -> usize {
    item.item_id.len()
        + text_bytes(&item.label)
        + reference_bytes(&item.reference)
        + quantity_bytes(&item.quantity)
        + field_bytes(&item.instance, |_| item.item_id.len() + 1)
        + 3
}

fn replacement_policy_bytes(policy: &RewardReplacementPolicy) -> usize {
    match policy {
        RewardReplacementPolicy::NotRequired
        | RewardReplacementPolicy::Required
        | RewardReplacementPolicy::Optional
        | RewardReplacementPolicy::Unknown => 1,
    }
}

fn state_policy_bytes(policy: &RewardStatePolicy) -> usize {
    field_bytes(&policy.claim_limit, |_| 4)
        + field_bytes(&policy.capacity, |_| 4)
        + field_bytes(&policy.replacement, replacement_policy_bytes)
        + field_bytes(&policy.multi_stage, |_| 4)
        + 2
}

fn kind_bytes(kind: &RewardKind) -> usize {
    match kind {
        RewardKind::Custom(value) | RewardKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}
