// SPDX-License-Identifier: MIT

use super::{
    definition::RestSiteDefinitionInput,
    effect::{RestEffect, RestHealAmount, RestProspectiveComparison},
    field::{RestField, RestText},
    model::RestSemanticReference,
    option::{RestCoverageRecord, RestOptionInput},
    requirement::{RestCandidate, RestCost, RestLimit, RestRequirement},
};

/// Fixed weight charged for one nested record beyond its own byte content.
///
/// The bound is an estimate of retained size rather than an exact allocation measurement: it counts
/// every identity and text byte a record retains and adds this constant so a snapshot of many small
/// records cannot pass the aggregate bound by carrying almost no text.
const RECORD_WEIGHT: usize = 64;

/// Estimates the aggregate bytes one validated definition retains.
pub(super) fn definition_bytes(input: &RestSiteDefinitionInput) -> usize {
    let mut total = RECORD_WEIGHT
        + input.site_id.len()
        + text_bytes(&input.label)
        + text_bytes(&input.description);
    total += references_bytes(&input.references);
    total += input
        .observed_options
        .iter()
        .map(|option_id| RECORD_WEIGHT + option_id.len())
        .sum::<usize>();
    total += input.options.iter().map(option_bytes).sum::<usize>();
    total += input.coverage.iter().map(coverage_bytes).sum::<usize>();
    total
}

fn option_bytes(option: &RestOptionInput) -> usize {
    let mut total = RECORD_WEIGHT
        + option.option_id.len()
        + text_bytes(&option.label)
        + text_bytes(&option.description)
        + kind_bytes(&option.kind)
        + references_bytes(&option.references)
        + action_kind_bytes(&option.action_kind);
    total += option
        .requirements
        .iter()
        .map(requirement_bytes)
        .sum::<usize>();
    total += option.costs.iter().map(cost_bytes).sum::<usize>();
    total += option.limits.iter().map(limit_bytes).sum::<usize>();
    total += option.effects.iter().map(effect_bytes).sum::<usize>();
    total += candidate_bytes_of(&option.selection.candidates);
    total += references_bytes(&option.selection.references);
    total += option.comparison.as_ref().map_or(0, comparison_bytes);
    total
}

fn coverage_bytes(record: &RestCoverageRecord) -> usize {
    RECORD_WEIGHT + record.option_id.len() + text_bytes(&record.reason)
}

fn requirement_bytes(requirement: &RestRequirement) -> usize {
    let mut total =
        RECORD_WEIGHT + requirement.requirement_id.len() + text_bytes(&requirement.label);
    if let RestField::Available(detail) = &requirement.detail {
        total += detail.len();
    }
    total + references_bytes(&requirement.references)
}

fn cost_bytes(cost: &RestCost) -> usize {
    RECORD_WEIGHT
        + cost.cost_id.len()
        + text_bytes(&cost.label)
        + kind_bytes(&cost.unit)
        + references_bytes(&cost.references)
}

fn limit_bytes(limit: &RestLimit) -> usize {
    RECORD_WEIGHT
        + limit.limit_id.len()
        + text_bytes(&limit.label)
        + kind_bytes(&limit.unit)
        + references_bytes(&limit.references)
}

fn effect_bytes(effect: &RestEffect) -> usize {
    let mut total = RECORD_WEIGHT
        + effect.effect_id.len()
        + text_bytes(&effect.label)
        + kind_bytes(&effect.kind)
        + references_bytes(&effect.references);
    if let RestField::Available(target) = &effect.target {
        total += reference_bytes(target);
    }
    total += effect.healing.as_ref().map_or(0, heal_bytes);
    total
}

fn heal_bytes(healing: &RestHealAmount) -> usize {
    let mut total = RECORD_WEIGHT;
    total += healing
        .modifiers
        .iter()
        .map(|modifier| {
            RECORD_WEIGHT
                + modifier.modifier_id.len()
                + text_bytes(&modifier.label)
                + kind_bytes(&modifier.kind)
                + references_bytes(&modifier.references)
        })
        .sum::<usize>();
    total
}

fn comparison_bytes(comparison: &RestProspectiveComparison) -> usize {
    let mut total = RECORD_WEIGHT + comparison.option_id.len();
    total += comparison
        .changes
        .iter()
        .map(|change| {
            let mut change_total = RECORD_WEIGHT
                + change.change_id.len()
                + text_bytes(&change.label)
                + kind_bytes(&change.kind)
                + references_bytes(&change.references);
            if let RestField::Available(target) = &change.target {
                change_total += reference_bytes(target);
            }
            for value in [change.before.value(), change.after.value()]
                .into_iter()
                .flatten()
            {
                change_total += value.len();
            }
            change_total
        })
        .sum::<usize>();
    if let RestField::Available(upgrade) = &comparison.upgrade {
        total += RECORD_WEIGHT + reference_bytes(&upgrade.candidate);
        total += references_bytes(&upgrade.references);
        for value in [upgrade.before.value(), upgrade.after.value()]
            .into_iter()
            .flatten()
        {
            total += value.len();
        }
    }
    total
}

fn candidate_bytes_of(candidates: &[RestCandidate]) -> usize {
    candidates
        .iter()
        .map(|candidate| {
            let mut total = RECORD_WEIGHT
                + candidate.candidate_id.len()
                + text_bytes(&candidate.label)
                + reference_bytes(&candidate.reference)
                + references_bytes(&candidate.references);
            if let RestField::Available(detail) = &candidate.detail {
                total += detail.len();
            }
            total
        })
        .sum()
}

fn references_bytes(references: &[RestSemanticReference]) -> usize {
    references.iter().map(reference_bytes).sum()
}

fn reference_bytes(reference: &RestSemanticReference) -> usize {
    RECORD_WEIGHT + reference.id.len() + text_bytes(&reference.label) + kind_bytes(&reference.kind)
}

fn text_bytes(text: &RestText) -> usize {
    match text {
        RestText::Available(value) => value.len(),
        RestText::Unavailable(reason) => kind_bytes(reason),
    }
}

fn action_kind_bytes(action_kind: &RestField<String>) -> usize {
    match action_kind {
        RestField::Available(value) => value.len(),
        RestField::Unavailable(reason) => kind_bytes(reason),
    }
}

/// Charges a fixed weight for one classified token, where `Debug` length is a stable proxy.
fn kind_bytes(kind: &impl std::fmt::Debug) -> usize {
    RECORD_WEIGHT + format!("{kind:?}").len()
}
