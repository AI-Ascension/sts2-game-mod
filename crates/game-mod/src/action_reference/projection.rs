// SPDX-License-Identifier: MIT

use super::{
    definition::{ActionCoverageRecord, ActionDefinition},
    field::{ActionFieldStatus, ActionUnavailableReason},
    model::{ActionVisibility, ActionVisibilityScope},
    page::{ActionDefinitionSummary, ActionTargetSummary},
    preview::ActionPreview,
    target::ActionTarget,
};

/// Returns whether one visibility label may be observed under one scope.
///
/// An owner-only reference is reachable from the locked-reference and owner scopes but not from the
/// public one, and an identity the source marked hidden or could not classify is never disclosed at
/// any scope, so widening a scope cannot reveal a value the source refused to publish.
pub(super) fn visibility_allowed(
    visibility: ActionVisibility,
    scope: ActionVisibilityScope,
) -> bool {
    match visibility {
        ActionVisibility::Visible => true,
        ActionVisibility::OwnerOnly => scope != ActionVisibilityScope::Public,
        ActionVisibility::Hidden | ActionVisibility::Unknown => false,
    }
}

/// Returns whether one observed target may be observed under one scope.
pub(super) fn visible_target(target: &ActionTarget, scope: ActionVisibilityScope) -> bool {
    visibility_allowed(target.visibility, scope)
}

/// Returns whether one declared preview may be observed under one scope.
///
/// A preview that names a target follows that target's visibility; a preview for an untargeted
/// action follows its owning definition, so a preview never discloses more than the subject that
/// carries it.
pub(super) fn visible_preview(
    preview: &ActionPreview,
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> bool {
    match preview.target.value() {
        Some(target) => match definition.targets.get(&target.target_id) {
            Some(observed) => visible_target(observed, scope),
            None => visibility_allowed(definition.visibility, scope),
        },
        None => visibility_allowed(definition.visibility, scope),
    }
}

/// Returns whether one coverage record may be observed under one scope.
pub(super) fn visible_coverage(
    record: &ActionCoverageRecord,
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> bool {
    match definition.targets.get(&record.target_id) {
        Some(target) => visible_target(target, scope),
        None => visibility_allowed(definition.visibility, scope),
    }
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection whose every record is withheld is
/// `Denied`; a mixed collection is `Withheld`. A withheld record is dropped rather than replaced by
/// a placeholder, so no restricted identity is disclosed.
pub(super) fn scoped_status(
    total: usize,
    visible: usize,
    source: ActionFieldStatus,
) -> ActionFieldStatus {
    if source != ActionFieldStatus::Available {
        return source;
    }
    if visible == total {
        ActionFieldStatus::Available
    } else if visible == 0 {
        ActionUnavailableReason::Denied.status()
    } else {
        ActionUnavailableReason::Withheld.status()
    }
}

/// Reports the availability of one keyed collection after scope withholding.
pub(super) fn map_status<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    source: ActionFieldStatus,
    visible: impl Fn(&T) -> bool,
) -> ActionFieldStatus {
    let mut total = 0;
    let mut shown = 0;
    for item in items {
        total += 1;
        if visible(item) {
            shown += 1;
        }
    }
    scoped_status(total, shown, source)
}

fn coverage_status(
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> ActionFieldStatus {
    let shown = definition
        .coverage
        .iter()
        .filter(|record| visible_coverage(record, definition, scope))
        .count();
    scoped_status(
        definition.coverage.len(),
        shown,
        ActionFieldStatus::Available,
    )
}

/// Projects a definition to the requested scope, dropping records the scope may not observe.
pub(super) fn project_definition(
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> ActionDefinition {
    ActionDefinition {
        reference: definition.reference.clone(),
        parent: definition.parent.clone(),
        kind: definition.kind.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        instance_generation: definition.instance_generation,
        eligibility: definition.eligibility.clone(),
        costs: definition.costs.clone(),
        restrictions: definition.restrictions.clone(),
        observed_target_count: definition.observed_target_count,
        targets: definition
            .targets
            .values()
            .filter(|target| visible_target(target, scope))
            .map(|target| (target.reference.target_id.clone(), target.clone()))
            .collect(),
        targets_status: map_status(
            definition.targets.values(),
            definition.targets_status,
            |target| visible_target(target, scope),
        ),
        coverage: definition
            .coverage
            .iter()
            .filter(|record| visible_coverage(record, definition, scope))
            .cloned()
            .collect(),
        coverage_status: coverage_status(definition, scope),
        preview_classes: definition.preview_classes.clone(),
        previews: definition
            .previews
            .values()
            .filter(|preview| visible_preview(preview, definition, scope))
            .map(|preview| (preview.reference.preview_id.clone(), preview.clone()))
            .collect(),
        previews_status: map_status(
            definition.previews.values(),
            definition.previews_status,
            |preview| visible_preview(preview, definition, scope),
        ),
        references: definition.references.clone(),
    }
}

/// Projects a preview to the requested scope, dropping affected targets the scope may not observe.
pub(super) fn project_preview(
    preview: &ActionPreview,
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> ActionPreview {
    ActionPreview {
        reference: preview.reference.clone(),
        label: preview.label.clone(),
        class: preview.class,
        target: preview.target.clone(),
        affected: preview
            .affected
            .iter()
            .filter(|target| match definition.targets.get(&target.target_id) {
                Some(observed) => visible_target(observed, scope),
                None => visibility_allowed(definition.visibility, scope),
            })
            .cloned()
            .collect(),
        changes: preview.changes.clone(),
        statuses: preview.statuses.clone(),
        movements: preview.movements.clone(),
        selections: preview.selections.clone(),
        assumptions: preview.assumptions.clone(),
        omissions: preview.omissions.clone(),
        provenance: preview.provenance.clone(),
        evidence: preview.evidence,
        references: preview.references.clone(),
    }
}

/// Builds the bounded page summary for one action definition under one scope.
pub(super) fn action_summary(
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> ActionDefinitionSummary {
    ActionDefinitionSummary {
        reference: definition.reference.clone(),
        parent: definition.parent.clone(),
        kind: definition.kind.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        instance_generation: definition.instance_generation,
        eligibility: definition.eligibility.clone(),
        cost_count: definition.costs.len(),
        blocking_cost_count: definition.blocking_costs().count(),
        restriction_count: definition.restrictions.len(),
        blocking_restriction_count: definition.blocking_restrictions().count(),
        target_count: definition
            .targets
            .values()
            .filter(|target| visible_target(target, scope))
            .count(),
        targets_status: map_status(
            definition.targets.values(),
            definition.targets_status,
            |target| visible_target(target, scope),
        ),
        preview_count: definition
            .previews
            .values()
            .filter(|preview| visible_preview(preview, definition, scope))
            .count(),
        previews_status: map_status(
            definition.previews.values(),
            definition.previews_status,
            |preview| visible_preview(preview, definition, scope),
        ),
    }
}

/// Builds a bounded target summary that preserves eligibility and refusal state.
pub(super) fn target_summary(target: &ActionTarget) -> ActionTargetSummary {
    ActionTargetSummary {
        reference: target.reference.clone(),
        label: target.label.clone(),
        kind: target.kind.clone(),
        eligibility: target.eligibility.clone(),
        detail_status: target.detail.status(),
        visibility: target.visibility,
    }
}
