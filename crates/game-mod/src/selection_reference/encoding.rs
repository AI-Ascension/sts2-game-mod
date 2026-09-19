// SPDX-License-Identifier: MIT

use super::{
    candidate::{
        SelectionBlockReason, SelectionCandidateInput, SelectionEligibility,
        SelectionProspectiveEffect,
    },
    definition::{SelectionCoverageRecord, SelectionDefinitionInput},
    field::{SelectionField, SelectionText},
    model::{SelectionCancellation, SelectionNextDomain, SelectionSemanticReference},
};

/// Fixed weight charged for one nested record beyond its own byte content.
///
/// The bound is an estimate of retained size rather than an exact allocation measurement: it counts
/// every identity and text byte a record retains and adds this constant so a snapshot of many small
/// records cannot pass the aggregate bound by carrying almost no text.
const RECORD_WEIGHT: usize = 64;

/// Estimates the aggregate bytes one validated definition retains.
pub(super) fn definition_bytes(input: &SelectionDefinitionInput) -> usize {
    let mut total = RECORD_WEIGHT
        + input.selection_id.len()
        + text_bytes(&input.prompt)
        + kind_bytes(&input.kind)
        + kind_bytes(&input.parent)
        + kind_bytes(&input.ordering)
        + kind_bytes(&input.confirmation)
        + cancellation_bytes(&input.cancellation)
        + next_bytes(&input.next)
        + references_bytes(&input.references)
        + action_kind_bytes(&input.action_kind);
    total += input
        .observed_candidates
        .iter()
        .map(|candidate_id| RECORD_WEIGHT + candidate_id.len())
        .sum::<usize>();
    total += input.candidates.iter().map(candidate_bytes).sum::<usize>();
    total += input.coverage.iter().map(coverage_bytes).sum::<usize>();
    total
}

fn candidate_bytes(candidate: &SelectionCandidateInput) -> usize {
    let mut total = RECORD_WEIGHT
        + candidate.candidate_id.len()
        + text_bytes(&candidate.label)
        + kind_bytes(&candidate.kind)
        + reference_bytes(&candidate.definition)
        + eligibility_bytes(&candidate.eligibility)
        + references_bytes(&candidate.references);
    if let SelectionField::Available(detail) = &candidate.detail {
        total += detail.len();
    }
    total += candidate
        .prospective
        .iter()
        .map(effect_bytes)
        .sum::<usize>();
    total
}

fn eligibility_bytes(eligibility: &SelectionEligibility) -> usize {
    kind_bytes(&eligibility.state)
        + reason_bytes(&eligibility.reason)
        + references_bytes(&eligibility.references)
}

fn effect_bytes(effect: &SelectionProspectiveEffect) -> usize {
    let mut total = RECORD_WEIGHT
        + effect.effect_id.len()
        + text_bytes(&effect.label)
        + kind_bytes(&effect.kind)
        + references_bytes(&effect.references);
    if let SelectionField::Available(target) = &effect.target {
        total += reference_bytes(target);
    }
    for value in [effect.before.value(), effect.after.value()]
        .into_iter()
        .flatten()
    {
        total += value.len();
    }
    total
}

fn coverage_bytes(record: &SelectionCoverageRecord) -> usize {
    RECORD_WEIGHT + record.target_id.len() + text_bytes(&record.reason)
}

fn references_bytes(references: &[SelectionSemanticReference]) -> usize {
    references.iter().map(reference_bytes).sum()
}

fn reference_bytes(reference: &SelectionSemanticReference) -> usize {
    RECORD_WEIGHT + reference.id.len() + text_bytes(&reference.label) + kind_bytes(&reference.kind)
}

fn next_bytes(field: &SelectionField<SelectionNextDomain>) -> usize {
    match field {
        SelectionField::Available(next) => {
            RECORD_WEIGHT + next.selection_id.len() + kind_bytes(&next.candidate_kind)
        }
        SelectionField::Unavailable(reason) => kind_bytes(reason),
    }
}

fn cancellation_bytes(field: &SelectionField<SelectionCancellation>) -> usize {
    match field {
        SelectionField::Available(value) => kind_bytes(value),
        SelectionField::Unavailable(reason) => kind_bytes(reason),
    }
}

fn reason_bytes(field: &SelectionField<SelectionBlockReason>) -> usize {
    match field {
        SelectionField::Available(value) => kind_bytes(value),
        SelectionField::Unavailable(reason) => kind_bytes(reason),
    }
}

fn text_bytes(text: &SelectionText) -> usize {
    match text {
        SelectionText::Available(value) => value.len(),
        SelectionText::Unavailable(reason) => kind_bytes(reason),
    }
}

fn action_kind_bytes(action_kind: &SelectionField<String>) -> usize {
    match action_kind {
        SelectionField::Available(value) => value.len(),
        SelectionField::Unavailable(reason) => kind_bytes(reason),
    }
}

/// Charges a fixed weight for one classified token, where `Debug` length is a stable proxy.
fn kind_bytes(kind: &impl std::fmt::Debug) -> usize {
    RECORD_WEIGHT + format!("{kind:?}").len()
}
