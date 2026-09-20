// SPDX-License-Identifier: MIT

use super::{
    cost::{ActionCostContributor, ActionTargetRestriction},
    definition::{ActionCoverageRecord, ActionDefinitionInput},
    eligibility::ActionEligibility,
    field::{ActionField, ActionText},
    kind::ActionRefusalReason,
    model::{ActionInstanceReference, ActionSemanticReference},
    preview::{
        ActionPreviewChange, ActionPreviewInput, ActionPreviewMovement, ActionPreviewOmission,
        ActionPreviewSelection, ActionPreviewStatus,
    },
    target::ActionTargetInput,
};

/// Fixed weight charged for one nested record beyond its own byte content.
///
/// The bound is an estimate of retained size rather than an exact allocation measurement: it counts
/// every identity and text byte a record retains and adds this constant so a snapshot of many small
/// records cannot pass the aggregate bound by carrying almost no text.
const RECORD_WEIGHT: usize = 64;

/// Estimates the aggregate bytes one validated definition retains.
pub(super) fn definition_bytes(input: &ActionDefinitionInput) -> usize {
    let mut total = RECORD_WEIGHT
        + input.action_id.len()
        + text_bytes(&input.label)
        + kind_bytes(&input.kind)
        + kind_bytes(&input.parent)
        + eligibility_bytes(&input.eligibility)
        + references_bytes(&input.references)
        + instance_bytes(&input.instance);
    total += input
        .observed_targets
        .iter()
        .map(|target_id| RECORD_WEIGHT + target_id.len())
        .sum::<usize>();
    total += input.targets.iter().map(target_bytes).sum::<usize>();
    total += input.costs.iter().map(cost_bytes).sum::<usize>();
    total += input
        .restrictions
        .iter()
        .map(restriction_bytes)
        .sum::<usize>();
    total += input.coverage.iter().map(coverage_bytes).sum::<usize>();
    total += input.preview_classes.iter().map(kind_bytes).sum::<usize>();
    total += input.previews.iter().map(preview_bytes).sum::<usize>();
    total
}

fn target_bytes(target: &ActionTargetInput) -> usize {
    let mut total = RECORD_WEIGHT
        + target.target_id.len()
        + text_bytes(&target.label)
        + kind_bytes(&target.kind)
        + reference_bytes(&target.definition)
        + eligibility_bytes(&target.eligibility)
        + references_bytes(&target.references);
    if let ActionField::Available(detail) = &target.detail {
        total += detail.len();
    }
    total
}

fn cost_bytes(cost: &ActionCostContributor) -> usize {
    let mut total = RECORD_WEIGHT
        + cost.cost_id.len()
        + text_bytes(&cost.label)
        + text_bytes(&cost.unit)
        + kind_bytes(&cost.kind)
        + references_bytes(&cost.references);
    if let ActionField::Available(resource) = &cost.resource {
        total += reference_bytes(resource);
    }
    total
}

fn restriction_bytes(restriction: &ActionTargetRestriction) -> usize {
    let mut total = RECORD_WEIGHT
        + restriction.restriction_id.len()
        + text_bytes(&restriction.label)
        + kind_bytes(&restriction.kind)
        + references_bytes(&restriction.references);
    if let ActionField::Available(target_id) = &restriction.target {
        total += target_id.len();
    }
    if let ActionField::Available(reason) = &restriction.reason {
        total += kind_bytes(reason);
    }
    total
}

fn eligibility_bytes(eligibility: &ActionEligibility) -> usize {
    kind_bytes(&eligibility.state)
        + reason_bytes(&eligibility.reason)
        + text_bytes(&eligibility.reason_text)
        + references_bytes(&eligibility.references)
}

fn preview_bytes(preview: &ActionPreviewInput) -> usize {
    let mut total = RECORD_WEIGHT
        + preview.preview_id.len()
        + text_bytes(&preview.label)
        + kind_bytes(&preview.class)
        + kind_bytes(&preview.provenance)
        + references_bytes(&preview.references)
        + preview
            .affected
            .iter()
            .map(|target_id| RECORD_WEIGHT + target_id.len())
            .sum::<usize>();
    if let ActionField::Available(target_id) = &preview.target {
        total += target_id.len();
    }
    total += preview.changes.iter().map(change_bytes).sum::<usize>();
    total += preview.statuses.iter().map(status_bytes).sum::<usize>();
    total += preview.movements.iter().map(movement_bytes).sum::<usize>();
    total += preview
        .selections
        .iter()
        .map(selection_bytes)
        .sum::<usize>();
    total += preview
        .assumptions
        .iter()
        .map(|record| named_bytes(&record.assumption_id, &record.label, &record.references))
        .sum::<usize>();
    total += preview.omissions.iter().map(omission_bytes).sum::<usize>();
    total
}

fn change_bytes(change: &ActionPreviewChange) -> usize {
    let mut total = RECORD_WEIGHT
        + change.change_id.len()
        + text_bytes(&change.label)
        + text_bytes(&change.unit)
        + kind_bytes(&change.kind)
        + references_bytes(&change.references);
    if let ActionField::Available(target) = &change.target {
        total += reference_bytes(target);
    }
    for value in [change.before.value(), change.after.value()]
        .into_iter()
        .flatten()
    {
        total += value.len();
    }
    total
}

fn status_bytes(status: &ActionPreviewStatus) -> usize {
    let mut total = RECORD_WEIGHT
        + status.status_id.len()
        + text_bytes(&status.label)
        + kind_bytes(&status.transition)
        + reference_bytes(&status.status)
        + references_bytes(&status.references);
    if let ActionField::Available(target) = &status.target {
        total += reference_bytes(target);
    }
    total
}

fn movement_bytes(movement: &ActionPreviewMovement) -> usize {
    RECORD_WEIGHT
        + movement.movement_id.len()
        + text_bytes(&movement.label)
        + kind_bytes(&movement.from)
        + kind_bytes(&movement.to)
        + reference_bytes(&movement.card)
        + references_bytes(&movement.references)
}

fn selection_bytes(selection: &ActionPreviewSelection) -> usize {
    RECORD_WEIGHT
        + selection.requirement_id.len()
        + text_bytes(&selection.label)
        + reference_bytes(&selection.selection)
        + references_bytes(&selection.references)
}

fn named_bytes(id: &str, label: &ActionText, references: &[ActionSemanticReference]) -> usize {
    RECORD_WEIGHT + id.len() + text_bytes(label) + references_bytes(references)
}

fn omission_bytes(omission: &ActionPreviewOmission) -> usize {
    let mut total = named_bytes(&omission.omission_id, &omission.label, &omission.references);
    total += kind_bytes(&omission.kind);
    total
}

fn coverage_bytes(record: &ActionCoverageRecord) -> usize {
    RECORD_WEIGHT + record.target_id.len() + text_bytes(&record.reason)
}

fn references_bytes(references: &[ActionSemanticReference]) -> usize {
    references.iter().map(reference_bytes).sum()
}

fn reference_bytes(reference: &ActionSemanticReference) -> usize {
    RECORD_WEIGHT + reference.id.len() + text_bytes(&reference.label) + kind_bytes(&reference.kind)
}

fn instance_bytes(field: &ActionField<ActionInstanceReference>) -> usize {
    match field {
        ActionField::Available(instance) => {
            RECORD_WEIGHT + instance.snapshot.run_id.len() + instance.action_instance_id.len()
        }
        ActionField::Unavailable(reason) => kind_bytes(reason),
    }
}

fn reason_bytes(field: &ActionField<ActionRefusalReason>) -> usize {
    match field {
        ActionField::Available(value) => kind_bytes(value),
        ActionField::Unavailable(reason) => kind_bytes(reason),
    }
}

fn text_bytes(text: &ActionText) -> usize {
    match text {
        ActionText::Available(value) => value.len(),
        ActionText::Unavailable(reason) => kind_bytes(reason),
    }
}

/// Charges a fixed weight for one classified token, where `Debug` length is a stable proxy.
fn kind_bytes(kind: &impl std::fmt::Debug) -> usize {
    RECORD_WEIGHT + format!("{kind:?}").len()
}
