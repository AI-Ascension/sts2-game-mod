// SPDX-License-Identifier: MIT

use super::{
    candidate::SelectionCandidate,
    definition::{SelectionCoverageRecord, SelectionDefinition},
    field::{SelectionFieldStatus, SelectionUnavailableReason},
    model::{SelectionDuplicateRule, SelectionVisibility, SelectionVisibilityScope},
    page::{SelectionCandidateSummary, SelectionDefinitionSummary},
};

/// Returns whether a visibility label may be observed under one scope.
pub(super) fn visibility_allowed(
    visibility: SelectionVisibility,
    scope: SelectionVisibilityScope,
) -> bool {
    match visibility {
        SelectionVisibility::Visible => true,
        SelectionVisibility::OwnerOnly => scope == SelectionVisibilityScope::Owner,
        SelectionVisibility::Hidden | SelectionVisibility::Unknown => false,
    }
}

/// Returns whether one candidate may be observed under one scope.
pub(super) fn visible_candidate(
    candidate: &SelectionCandidate,
    scope: SelectionVisibilityScope,
) -> bool {
    visibility_allowed(candidate.visibility, scope)
}

/// Returns whether one coverage record may be observed under one scope.
///
/// A record covering a typed candidate follows that candidate's visibility; a record covering a
/// target this producer did not type follows its owning definition's visibility, so an audit record
/// never discloses more than the selection that carries it.
pub(super) fn visible_coverage(
    record: &SelectionCoverageRecord,
    definition: &SelectionDefinition,
    scope: SelectionVisibilityScope,
) -> bool {
    match definition.candidates.get(&record.target_id) {
        Some(candidate) => visible_candidate(candidate, scope),
        None => visibility_allowed(definition.visibility, scope),
    }
}

/// Returns whether one candidate is still selectable for the picks made so far.
///
/// A candidate the caller already picked is no longer selectable when the selector does not allow
/// repeats, so the list a caller pages through reflects the prompt rather than an untouched one.
pub(super) fn selectable_candidate(
    candidate: &SelectionCandidate,
    scope: SelectionVisibilityScope,
    selected: &[String],
    duplicate: SelectionDuplicateRule,
) -> bool {
    if !visible_candidate(candidate, scope) {
        return false;
    }
    if duplicate != SelectionDuplicateRule::Distinct {
        return true;
    }
    !selected
        .iter()
        .any(|picked| picked == &candidate.reference.candidate_id)
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection whose every record is withheld is
/// `Denied`; a mixed collection is `Withheld`. A withheld record is dropped rather than replaced
/// with a placeholder, so no restricted identity is disclosed.
fn scoped_status(
    total: usize,
    visible: usize,
    source: SelectionFieldStatus,
) -> SelectionFieldStatus {
    if source != SelectionFieldStatus::Available {
        return source;
    }
    if visible == total {
        SelectionFieldStatus::Available
    } else if visible == 0 {
        SelectionUnavailableReason::Denied.status()
    } else {
        SelectionUnavailableReason::Withheld.status()
    }
}

/// Reports the availability of one keyed collection after scope withholding.
pub(super) fn map_status<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    scope: SelectionVisibilityScope,
    visible: fn(&T, SelectionVisibilityScope) -> bool,
) -> SelectionFieldStatus {
    let mut total = 0;
    let mut shown = 0;
    for item in items {
        total += 1;
        if visible(item, scope) {
            shown += 1;
        }
    }
    scoped_status(total, shown, SelectionFieldStatus::Available)
}

/// Reports the availability of one coverage list after scope withholding.
fn coverage_status(
    definition: &SelectionDefinition,
    scope: SelectionVisibilityScope,
) -> SelectionFieldStatus {
    let shown = definition
        .coverage
        .iter()
        .filter(|record| visible_coverage(record, definition, scope))
        .count();
    scoped_status(
        definition.coverage.len(),
        shown,
        SelectionFieldStatus::Available,
    )
}

/// Projects a definition to the requested scope, dropping records the scope may not observe.
pub(super) fn project_definition(
    definition: &SelectionDefinition,
    scope: SelectionVisibilityScope,
) -> SelectionDefinition {
    SelectionDefinition {
        reference: definition.reference.clone(),
        parent: definition.parent.clone(),
        kind: definition.kind.clone(),
        prompt: definition.prompt.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        selector_generation: definition.selector_generation,
        picks: definition.picks.clone(),
        ordering: definition.ordering.clone(),
        duplicate: definition.duplicate,
        confirmation: definition.confirmation,
        cancellation: definition.cancellation.clone(),
        steps: definition.steps.clone(),
        next: definition.next.clone(),
        observed_candidate_count: definition.observed_candidate_count,
        candidates: definition
            .candidates
            .values()
            .filter(|candidate| visible_candidate(candidate, scope))
            .map(|candidate| (candidate.reference.candidate_id.clone(), candidate.clone()))
            .collect(),
        coverage: definition
            .coverage
            .iter()
            .filter(|record| visible_coverage(record, definition, scope))
            .cloned()
            .collect(),
        candidates_status: map_status(definition.candidates.values(), scope, visible_candidate),
        coverage_status: coverage_status(definition, scope),
        action_kind: definition.action_kind.clone(),
        references: definition.references.clone(),
    }
}

/// Builds the bounded page summary for one selection definition under one scope.
pub(super) fn selection_summary(
    definition: &SelectionDefinition,
    scope: SelectionVisibilityScope,
) -> SelectionDefinitionSummary {
    SelectionDefinitionSummary {
        reference: definition.reference.clone(),
        parent: definition.parent.clone(),
        kind: definition.kind.clone(),
        prompt: definition.prompt.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        selector_generation: definition.selector_generation,
        picks: definition.picks.clone(),
        candidate_count: definition
            .candidates
            .values()
            .filter(|candidate| visible_candidate(candidate, scope))
            .count(),
        candidates_status: map_status(definition.candidates.values(), scope, visible_candidate),
        coverage_count: definition
            .coverage
            .iter()
            .filter(|record| visible_coverage(record, definition, scope))
            .count(),
        coverage_status: coverage_status(definition, scope),
    }
}

/// Builds a bounded candidate summary that preserves eligibility and refusal state.
pub(super) fn candidate_summary(candidate: &SelectionCandidate) -> SelectionCandidateSummary {
    SelectionCandidateSummary {
        reference: candidate.reference.clone(),
        label: candidate.label.clone(),
        kind: candidate.kind.clone(),
        definition: candidate.definition.clone(),
        eligibility: candidate.eligibility.clone(),
        detail_status: candidate.detail.status(),
        effect_count: candidate.prospective.len(),
        visibility: candidate.visibility,
    }
}
