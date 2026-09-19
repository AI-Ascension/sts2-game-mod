// SPDX-License-Identifier: MIT

mod candidates;
pub(super) mod manifest;
mod references;

use std::collections::{BTreeMap, BTreeSet};

use self::candidates::validate_candidate;
use self::references::RefContext;
use super::{
    definition::{SelectionCoverageRecord, SelectionDefinitionInput},
    error::SelectionError,
    field::SelectionField,
    identity::{validate_identity, validate_text, validate_text_value},
    model::{
        SEL_MAX_CANDIDATES, SEL_MAX_COVERAGE_RECORDS, SEL_MAX_PICKS, SEL_MAX_STEPS,
        SelectionCancellation, SelectionCandidateKind, SelectionConfirmation, SelectionKind,
        SelectionOrderingRule, SelectionReferenceKind, SelectionVisibility,
        selection_visibility_rank,
    },
};

/// Same-snapshot target available for one cross-selection reference.
#[derive(Clone, Copy, Debug)]
pub(super) struct SelectionTarget {
    /// Visibility of the target definition.
    pub(super) visibility: SelectionVisibility,
    /// Selector generation of the target definition.
    pub(super) selector_generation: u64,
}

/// Validates one selection definition against every same-snapshot target.
pub(super) fn validate_definition(
    input: &SelectionDefinitionInput,
    selections: &BTreeMap<String, SelectionTarget>,
) -> Result<(), SelectionError> {
    validate_identity(&input.selection_id, "selection_id")?;
    validate_text_value(&input.prompt, "prompt")?;
    if input.candidates.len() > SEL_MAX_CANDIDATES {
        return Err(SelectionError::InvalidInput("candidates"));
    }
    if input.coverage.len() > SEL_MAX_COVERAGE_RECORDS {
        return Err(SelectionError::InvalidInput("coverage"));
    }
    if input.observed_candidates.len() > SEL_MAX_CANDIDATES + SEL_MAX_COVERAGE_RECORDS {
        return Err(SelectionError::InvalidInput("observed_candidates"));
    }
    for observed in &input.observed_candidates {
        validate_identity(observed, "observed_candidates")?;
    }
    let candidate_ids = collect_candidate_ids(input)?;
    let covered = collect_coverage_ids(&input.coverage)?;
    ensure_observed_candidates_accounted(input, &candidate_ids, &covered)?;
    validate_coverage_targets(input, &candidate_ids)?;
    ensure_uncovered_selection_is_covered(input, &candidate_ids, &covered)?;
    validate_kind_agreement(input)?;
    validate_pick_rule(input)?;
    validate_confirmation(input)?;
    validate_ordering(input)?;
    validate_steps(input, selections)?;
    if input.action.is_available() {
        return Err(SelectionError::InvalidSelectionAction {
            selection_id: input.selection_id.clone(),
        });
    }
    let context = RefContext::new(&input.selection_id, selections, &candidate_ids);
    context.check_all(&input.references, input.visibility)?;
    for candidate in &input.candidates {
        validate_candidate(candidate, &input.selection_id, &context)?;
    }
    Ok(())
}

fn collect_candidate_ids(
    input: &SelectionDefinitionInput,
) -> Result<BTreeSet<String>, SelectionError> {
    let mut ids = BTreeSet::new();
    for candidate in &input.candidates {
        validate_identity(&candidate.candidate_id, "candidate_id")?;
        if !ids.insert(candidate.candidate_id.clone()) {
            return Err(SelectionError::DuplicateCandidate {
                selection_id: input.selection_id.clone(),
                candidate_id: candidate.candidate_id.clone(),
            });
        }
    }
    Ok(ids)
}

fn collect_coverage_ids(
    coverage: &[SelectionCoverageRecord],
) -> Result<BTreeSet<String>, SelectionError> {
    let mut ids = BTreeSet::new();
    for record in coverage {
        validate_identity(&record.target_id, "coverage")?;
        validate_text_value(&record.reason, "coverage")?;
        if !ids.insert(record.target_id.clone()) {
            return Err(SelectionError::InvalidInput("coverage"));
        }
    }
    Ok(ids)
}

/// Requires every host-reported candidate to be described or explicitly covered by name.
///
/// A newly audited entry is never silently omitted.
fn ensure_observed_candidates_accounted(
    input: &SelectionDefinitionInput,
    candidate_ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Result<(), SelectionError> {
    for observed in &input.observed_candidates {
        if !candidate_ids.contains(observed) && !covered.contains(observed) {
            return Err(SelectionError::UncoveredCandidate {
                selection_id: input.selection_id.clone(),
                candidate_id: observed.clone(),
            });
        }
    }
    Ok(())
}

/// Requires a selector this producer cannot type to carry a named coverage record.
///
/// An untyped selector names itself and every candidate it presents.
fn ensure_uncovered_selection_is_covered(
    input: &SelectionDefinitionInput,
    candidate_ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Result<(), SelectionError> {
    let untyped = matches!(
        input.kind,
        SelectionKind::Unsupported(_) | SelectionKind::Unknown
    );
    if !untyped {
        return Ok(());
    }
    if !covered.contains(&input.selection_id) {
        return Err(SelectionError::UncoveredSelection {
            selection_id: input.selection_id.clone(),
        });
    }
    for candidate_id in candidate_ids {
        if !covered.contains(candidate_id) {
            return Err(SelectionError::UncoveredCandidate {
                selection_id: input.selection_id.clone(),
                candidate_id: candidate_id.clone(),
            });
        }
    }
    Ok(())
}

/// Rejects a coverage record that names neither the selector nor one of its reported candidates.
///
/// An audit record that names nothing observed is refused rather than standing in for a target.
fn validate_coverage_targets(
    input: &SelectionDefinitionInput,
    candidate_ids: &BTreeSet<String>,
) -> Result<(), SelectionError> {
    for record in &input.coverage {
        let observed = record.target_id == input.selection_id
            || candidate_ids.contains(&record.target_id)
            || input.observed_candidates.contains(&record.target_id);
        if !observed {
            return Err(SelectionError::InvalidCoverageRecord {
                selection_id: input.selection_id.clone(),
                target_id: record.target_id.clone(),
            });
        }
    }
    Ok(())
}

/// Rejects a selector that contradicts the definition family its candidates resolve to.
///
/// A supported selector may not report itself unknown while a candidate names a known family.
fn validate_kind_agreement(input: &SelectionDefinitionInput) -> Result<(), SelectionError> {
    if !matches!(input.kind, SelectionKind::Unknown) {
        return Ok(());
    }
    let family = input
        .candidates
        .iter()
        .find_map(|candidate| manifest::reference_family(&candidate.definition.kind));
    if let Some(family) = family {
        return Err(SelectionError::KindDisagreesWithDefinition {
            selection_id: input.selection_id.clone(),
            reported: input.kind.clone(),
            family,
        });
    }
    Ok(())
}

/// Rejects a required/minimum/maximum declaration no prompt could satisfy.
fn validate_pick_rule(input: &SelectionDefinitionInput) -> Result<(), SelectionError> {
    let minimum = input.picks.minimum_picks();
    let maximum = input.picks.maximum_picks();
    let impossible = match (minimum, maximum) {
        (Some(minimum), Some(maximum)) => {
            maximum < minimum || maximum == 0 || maximum > SEL_MAX_PICKS
        }
        (Some(minimum), None) => minimum == 0 || minimum > SEL_MAX_PICKS,
        (None, Some(maximum)) => maximum == 0 || maximum > SEL_MAX_PICKS,
        (None, None) => false,
    };
    if impossible {
        return Err(SelectionError::InvalidPickRule {
            selection_id: input.selection_id.clone(),
        });
    }
    Ok(())
}

/// Rejects completion semantics that contradict each other.
///
/// A prompt that closes on its own cannot also offer an explicit cancel.
fn validate_confirmation(input: &SelectionDefinitionInput) -> Result<(), SelectionError> {
    let automatic_with_cancel = input.confirmation == SelectionConfirmation::AutomaticClose
        && matches!(
            input.cancellation,
            SelectionField::Available(SelectionCancellation::Cancel)
        );
    if automatic_with_cancel {
        return Err(SelectionError::InvalidConfirmation {
            selection_id: input.selection_id.clone(),
        });
    }
    Ok(())
}

/// Validates owner-defined ordering and cancellation tokens.
fn validate_ordering(input: &SelectionDefinitionInput) -> Result<(), SelectionError> {
    if let SelectionOrderingRule::Ordered(key) = &input.ordering {
        validate_text(key, "ordering")?;
    }
    if let SelectionField::Available(SelectionCancellation::Custom(name)) = &input.cancellation {
        validate_text(name, "cancellation")?;
    }
    if let Some(kind) = input.action_kind.value() {
        validate_text(kind, "action_kind")?;
    }
    Ok(())
}

/// Requires a multi-step declaration and the next domain it names to agree.
fn validate_steps(
    input: &SelectionDefinitionInput,
    selections: &BTreeMap<String, SelectionTarget>,
) -> Result<(), SelectionError> {
    let steps = input.steps.value().copied();
    match (steps, input.next.value()) {
        (None, None) | (Some(1), None) => Ok(()),
        (Some(steps), Some(next)) if (2..=SEL_MAX_STEPS).contains(&steps) => {
            validate_next_domain(input, next, selections)
        }
        _ => Err(SelectionError::InvalidStepDeclaration {
            selection_id: input.selection_id.clone(),
        }),
    }
}

fn validate_next_domain(
    input: &SelectionDefinitionInput,
    next: &super::model::SelectionNextDomain,
    selections: &BTreeMap<String, SelectionTarget>,
) -> Result<(), SelectionError> {
    validate_identity(&next.selection_id, "next_selection_id")?;
    if next.candidate_kind == SelectionCandidateKind::Unknown {
        return Err(SelectionError::InvalidStepDeclaration {
            selection_id: input.selection_id.clone(),
        });
    }
    let Some(target) = selections.get(&next.selection_id) else {
        return Err(SelectionError::DanglingReference {
            selection_id: input.selection_id.clone(),
            reference_kind: SelectionReferenceKind::Selection,
            id: next.selection_id.clone(),
        });
    };
    if target.selector_generation != next.selector_generation {
        return Err(SelectionError::StaleSelectorReference {
            selection_id: input.selection_id.clone(),
            referenced: next.selector_generation,
            current: target.selector_generation,
        });
    }
    if selection_visibility_rank(target.visibility) < selection_visibility_rank(input.visibility) {
        return Err(SelectionError::HiddenReferenceLeak {
            selection_id: input.selection_id.clone(),
            reference_kind: SelectionReferenceKind::Selection,
        });
    }
    Ok(())
}

/// Builds the same-snapshot targets for one set of selection identities.
pub(super) fn target_map(
    definitions: &[SelectionDefinitionInput],
) -> BTreeMap<String, SelectionTarget> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.selection_id.clone(),
                SelectionTarget {
                    visibility: definition.visibility,
                    selector_generation: definition.selector_generation,
                },
            )
        })
        .collect()
}
