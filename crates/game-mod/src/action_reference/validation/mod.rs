// SPDX-License-Identifier: MIT

mod consequences;
mod costs;
pub(super) mod manifest;
mod previews;
mod references;
mod targets;

use std::collections::{BTreeMap, BTreeSet};

use self::costs::{validate_costs, validate_refusal_support, validate_restrictions};
use self::previews::validate_previews;
use self::references::RefContext;
use self::targets::{eligibility_is_inconsistent, validate_target};
use super::{
    definition::{ActionCoverageRecord, ActionDefinitionInput},
    error::ActionError,
    field::ActionField,
    identity::{validate_identity, validate_text, validate_text_value},
    kind::{ActionKind, ActionReferenceKind, ActionRefusalReason},
    model::{
        ACTION_MAX_COVERAGE_RECORDS, ACTION_MAX_PREVIEW_CLASSES, ACTION_MAX_TARGETS,
        ActionVisibility,
    },
};

/// Same-snapshot target available for one cross-action reference.
#[derive(Clone, Copy, Debug)]
pub(super) struct ActionFrameTarget {
    /// Visibility of the target definition.
    pub(super) visibility: ActionVisibility,
    /// Legal-action generation of the target definition.
    pub(super) instance_generation: u64,
}

/// Validates one legal-action definition against every same-snapshot target.
pub(super) fn validate_definition(
    input: &ActionDefinitionInput,
    actions: &BTreeMap<String, ActionFrameTarget>,
) -> Result<(), ActionError> {
    validate_identity(&input.action_id, "action_id")?;
    validate_text_value(&input.label, "label")?;
    validate_kind(&input.kind)?;
    if input.targets.len() > ACTION_MAX_TARGETS {
        return Err(ActionError::InvalidInput("targets"));
    }
    if input.coverage.len() > ACTION_MAX_COVERAGE_RECORDS {
        return Err(ActionError::InvalidInput("coverage"));
    }
    if input.observed_targets.len() > ACTION_MAX_TARGETS + ACTION_MAX_COVERAGE_RECORDS {
        return Err(ActionError::InvalidInput("observed_targets"));
    }
    if input.preview_classes.len() > ACTION_MAX_PREVIEW_CLASSES {
        return Err(ActionError::InvalidInput("preview_classes"));
    }
    for observed in &input.observed_targets {
        validate_identity(observed, "observed_targets")?;
    }
    let target_ids = collect_target_ids(input)?;
    let covered = collect_coverage_ids(&input.coverage)?;
    ensure_observed_targets_accounted(input, &target_ids, &covered)?;
    validate_coverage_targets(input, &target_ids)?;
    ensure_uncovered_action_is_covered(input, &target_ids, &covered)?;
    validate_kind_agreement(input)?;
    validate_cross_action_generations(input, actions)?;
    validate_eligibility(input)?;
    if input.instance.is_available() {
        return Err(ActionError::InvalidInstanceReference {
            action_id: input.action_id.clone(),
        });
    }
    validate_restrictions(input)?;
    validate_refusal_support(input)?;
    let context = RefContext::new(&input.action_id, actions, &target_ids);
    validate_costs(input, &context)?;
    validate_previews(input, &context)?;
    context.check_all(&input.references, input.visibility)?;
    for target in &input.targets {
        validate_target(target, &input.action_id, &context)?;
    }
    Ok(())
}

/// Refuses a same-snapshot reference to an action the source presented at another generation.
///
/// Every definition in one snapshot describes the same presented frame, so a cross-action
/// reference whose target moved to another legal-action generation is a stale reference rather than
/// a second live frame smuggled into this catalog.
fn validate_cross_action_generations(
    input: &ActionDefinitionInput,
    actions: &BTreeMap<String, ActionFrameTarget>,
) -> Result<(), ActionError> {
    for reference in &input.references {
        if reference.kind != ActionReferenceKind::Action {
            continue;
        }
        let Some(target) = actions.get(&reference.id) else {
            continue;
        };
        if target.instance_generation != input.instance_generation {
            return Err(ActionError::StaleActionReference {
                action_id: input.action_id.clone(),
                referenced: target.instance_generation,
                current: input.instance_generation,
            });
        }
    }
    Ok(())
}

/// Validates one owner-defined legal-action family token.
fn validate_kind(kind: &ActionKind) -> Result<(), ActionError> {
    match kind {
        ActionKind::Custom(kind) | ActionKind::Unsupported(kind) => validate_text(kind, "kind"),
        _ => Ok(()),
    }
}

/// Validates action availability and its stated refusal reason and text.
fn validate_eligibility(input: &ActionDefinitionInput) -> Result<(), ActionError> {
    let inconsistent = eligibility_is_inconsistent(&input.eligibility, input.visibility);
    if inconsistent {
        return Err(ActionError::InvalidEligibility {
            action_id: input.action_id.clone(),
        });
    }
    if let ActionField::Available(reason) = &input.eligibility.reason
        && let ActionRefusalReason::Custom(reason) = reason
    {
        validate_text(reason, "refusal_reason")?;
    }
    Ok(())
}

fn collect_target_ids(input: &ActionDefinitionInput) -> Result<BTreeSet<String>, ActionError> {
    let mut ids = BTreeSet::new();
    for target in &input.targets {
        validate_identity(&target.target_id, "target_id")?;
        if !ids.insert(target.target_id.clone()) {
            return Err(ActionError::DuplicateTarget {
                action_id: input.action_id.clone(),
                target_id: target.target_id.clone(),
            });
        }
    }
    Ok(ids)
}

fn collect_coverage_ids(
    coverage: &[ActionCoverageRecord],
) -> Result<BTreeSet<String>, ActionError> {
    let mut ids = BTreeSet::new();
    for record in coverage {
        validate_identity(&record.target_id, "coverage")?;
        validate_text_value(&record.reason, "coverage")?;
        if !ids.insert(record.target_id.clone()) {
            return Err(ActionError::InvalidInput("coverage"));
        }
    }
    Ok(ids)
}

/// Requires every host-reported target to be described or explicitly covered by name.
///
/// A newly audited entry is never silently omitted.
fn ensure_observed_targets_accounted(
    input: &ActionDefinitionInput,
    target_ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Result<(), ActionError> {
    for observed in &input.observed_targets {
        if !target_ids.contains(observed) && !covered.contains(observed) {
            return Err(ActionError::UncoveredTarget {
                action_id: input.action_id.clone(),
                target_id: observed.clone(),
            });
        }
    }
    Ok(())
}

/// Requires a legal action this producer cannot type to carry a named coverage record.
///
/// An untyped action names itself and every target it accepts.
fn ensure_uncovered_action_is_covered(
    input: &ActionDefinitionInput,
    target_ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Result<(), ActionError> {
    let untyped = matches!(input.kind, ActionKind::Unsupported(_) | ActionKind::Unknown);
    if !untyped {
        return Ok(());
    }
    if !covered.contains(&input.action_id) {
        return Err(ActionError::UncoveredAction {
            action_id: input.action_id.clone(),
        });
    }
    for target_id in target_ids {
        if !covered.contains(target_id) {
            return Err(ActionError::UncoveredTarget {
                action_id: input.action_id.clone(),
                target_id: target_id.clone(),
            });
        }
    }
    Ok(())
}

/// Rejects a coverage record that names neither the action nor one of its reported targets.
///
/// An audit record that names nothing observed is refused rather than standing in for a target.
fn validate_coverage_targets(
    input: &ActionDefinitionInput,
    target_ids: &BTreeSet<String>,
) -> Result<(), ActionError> {
    for record in &input.coverage {
        let observed = record.target_id == input.action_id
            || target_ids.contains(&record.target_id)
            || input.observed_targets.contains(&record.target_id);
        if !observed {
            return Err(ActionError::InvalidCoverageRecord {
                action_id: input.action_id.clone(),
                target_id: record.target_id.clone(),
            });
        }
    }
    Ok(())
}

/// Rejects a legal action that contradicts the definition family its targets resolve to.
///
/// A supported action may not report itself unknown while a target names a known family.
fn validate_kind_agreement(input: &ActionDefinitionInput) -> Result<(), ActionError> {
    if !matches!(input.kind, ActionKind::Unknown) {
        return Ok(());
    }
    let family = input
        .targets
        .iter()
        .find_map(|target| manifest::reference_family(&target.definition.kind));
    if let Some(family) = family {
        return Err(ActionError::KindDisagreesWithDefinition {
            action_id: input.action_id.clone(),
            reported: input.kind.clone(),
            family,
        });
    }
    Ok(())
}

/// Builds the same-snapshot targets for one set of legal-action identities.
pub(super) fn target_map(
    definitions: &[ActionDefinitionInput],
) -> BTreeMap<String, ActionFrameTarget> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.action_id.clone(),
                ActionFrameTarget {
                    visibility: definition.visibility,
                    instance_generation: definition.instance_generation,
                },
            )
        })
        .collect()
}
