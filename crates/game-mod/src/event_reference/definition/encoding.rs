// SPDX-License-Identifier: MIT

use super::super::model::{
    EventField, EventNumericValue, EventProbability, EventSemanticReference,
    EventSemanticReferenceKind, EventText,
};
use super::*;

/// Returns a conservative byte estimate for one source-owned event definition.
pub(crate) fn definition_bytes(input: &EventDefinitionInput) -> usize {
    let mut total =
        input.event_id.len() + text_bytes(&input.title) + event_kind_bytes(&input.kind) + 2;
    total += input.pages.iter().map(page_bytes).sum::<usize>();
    total += input
        .eligibility
        .iter()
        .map(requirement_bytes)
        .sum::<usize>();
    total += input.options.iter().map(option_bytes).sum::<usize>();
    total += input.references.iter().map(reference_bytes).sum::<usize>();
    total
}

fn text_bytes(value: &EventText) -> usize {
    match value {
        EventText::Available(value) => value.len(),
        EventText::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &EventField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        EventField::Available(value) => available(value),
        EventField::Unavailable(_) => 1,
    }
}

fn numeric_bytes(value: &EventNumericValue) -> usize {
    match value {
        EventNumericValue::Fixed(_) => 8,
        EventNumericValue::Formula(formula) => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        EventNumericValue::Unavailable(_) => 1,
    }
}

fn probability_bytes(probability: &EventProbability) -> usize {
    match probability {
        EventProbability::Exact { .. } => 12,
        EventProbability::Rule { rule_reference, .. } => rule_reference.len() + 1,
        EventProbability::Unavailable(_) => 1,
    }
}

fn semantic_kind_bytes(kind: &EventSemanticReferenceKind) -> usize {
    match kind {
        EventSemanticReferenceKind::Content { entity_kind } => entity_kind.len(),
        EventSemanticReferenceKind::Event
        | EventSemanticReferenceKind::Encounter
        | EventSemanticReferenceKind::Enemy
        | EventSemanticReferenceKind::Relic
        | EventSemanticReferenceKind::Card
        | EventSemanticReferenceKind::Potion
        | EventSemanticReferenceKind::Act
        | EventSemanticReferenceKind::Page
        | EventSemanticReferenceKind::Option
        | EventSemanticReferenceKind::Effect
        | EventSemanticReferenceKind::Rule
        | EventSemanticReferenceKind::Condition
        | EventSemanticReferenceKind::Unknown => 1,
    }
}

fn reference_bytes(reference: &EventSemanticReference) -> usize {
    semantic_kind_bytes(&reference.kind) + reference.id.len() + text_bytes(&reference.label) + 2
}

fn parameter_bytes(parameter: &EventParameter) -> usize {
    parameter.parameter_id.len()
        + text_bytes(&parameter.label)
        + parameter.unit.as_deref().map_or(0, str::len)
        + numeric_bytes(&parameter.value)
        + 2
}

fn requirement_kind_bytes(kind: &EventRequirementKind) -> usize {
    match kind {
        EventRequirementKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn requirement_bytes(requirement: &EventRequirement) -> usize {
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

fn cost_kind_bytes(kind: &EventCostKind) -> usize {
    match kind {
        EventCostKind::Custom(value) | EventCostKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn cost_bytes(cost: &EventCost) -> usize {
    cost.cost_id.len()
        + cost_kind_bytes(&cost.kind)
        + text_bytes(&cost.label)
        + numeric_bytes(&cost.amount)
        + field_bytes(&cost.resource, String::len)
        + field_bytes(&cost.rule_reference, String::len)
        + cost.references.iter().map(reference_bytes).sum::<usize>()
        + 3
}

fn effect_kind_bytes(kind: &EventEffectKind) -> usize {
    match kind {
        EventEffectKind::Custom(value) | EventEffectKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn effect_bytes(effect: &EventEffect) -> usize {
    effect.effect_id.len()
        + effect_kind_bytes(&effect.kind)
        + text_bytes(&effect.label)
        + numeric_bytes(&effect.amount)
        + field_bytes(&effect.target, String::len)
        + field_bytes(&effect.rule_reference, String::len)
        + effect.references.iter().map(reference_bytes).sum::<usize>()
        + 3
}

fn follow_up_bytes(follow_up: &EventFollowUp) -> usize {
    match follow_up {
        EventFollowUp::Page(page_id) => page_id.len() + 1,
        EventFollowUp::End | EventFollowUp::Unavailable(_) => 1,
    }
}

fn outcome_bytes(outcome: &EventOutcomeInput) -> usize {
    outcome.outcome_id.len()
        + text_bytes(&outcome.label)
        + probability_bytes(&outcome.probability)
        + outcome.effects.iter().map(effect_bytes).sum::<usize>()
        + follow_up_bytes(&outcome.follow_up)
        + outcome
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}

fn option_bytes(option: &EventOptionInput) -> usize {
    option.option_id.len()
        + text_bytes(&option.text)
        + option
            .requirements
            .iter()
            .map(requirement_bytes)
            .sum::<usize>()
        + option.costs.iter().map(cost_bytes).sum::<usize>()
        + option.outcomes.iter().map(outcome_bytes).sum::<usize>()
        + option.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn page_bytes(page: &EventNarrativePage) -> usize {
    page.page_id.len()
        + text_bytes(&page.narrative)
        + page.references.iter().map(reference_bytes).sum::<usize>()
        + 2
}

fn event_kind_bytes(kind: &EventKind) -> usize {
    match kind {
        EventKind::Custom(value) | EventKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}
