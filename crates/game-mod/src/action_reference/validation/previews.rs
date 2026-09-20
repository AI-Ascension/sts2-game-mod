// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::{
    definition::ActionDefinitionInput,
    error::ActionError,
    field::ActionField,
    identity::{validate_identity, validate_text_value},
    kind::{ActionOmissionKind, ActionPreviewClass, ActionPreviewProvenance},
    model::{
        ACTION_MAX_AFFECTED, ACTION_MAX_ASSUMPTIONS, ACTION_MAX_CHANGES, ACTION_MAX_MOVEMENTS,
        ACTION_MAX_OMISSIONS, ACTION_MAX_PREVIEWS, ACTION_MAX_SELECTION_REQUIREMENTS,
        ACTION_MAX_STATUSES,
    },
    preview::ActionPreviewInput,
};
use super::RefContext;
use super::consequences::{
    validate_changes, validate_movements, validate_named, validate_selections, validate_statuses,
};

/// Validates every declared preview of one action.
///
/// A preview must not overstate what it knows: an approximation that mutated the host is refused,
/// a classification the source does not declare is refused, a certainty claim that names an
/// omission or assumption is refused, and a consequence that would change nothing is refused.
pub(super) fn validate_previews(
    input: &ActionDefinitionInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    if input.previews.len() > ACTION_MAX_PREVIEWS {
        return Err(ActionError::InvalidInput("previews"));
    }
    let mut seen = BTreeSet::new();
    for preview in &input.previews {
        validate_identity(&preview.preview_id, "preview_id")?;
        if !seen.insert(preview.preview_id.as_str()) {
            return Err(ActionError::InvalidInput("preview_id"));
        }
        validate_preview(input, preview, context)?;
    }
    Ok(())
}

fn validate_preview(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    validate_text_value(&preview.label, "label")?;
    if !input.preview_classes.contains(&preview.class) {
        return Err(ActionError::UndeclaredPreviewClass {
            action_id: input.action_id.clone(),
            preview_id: preview.preview_id.clone(),
            class: preview.class,
        });
    }
    if preview.provenance == ActionPreviewProvenance::SimulatedFromPendingState {
        return Err(ActionError::SimulatedPreview {
            action_id: input.action_id.clone(),
            preview_id: preview.preview_id.clone(),
        });
    }
    if let ActionField::Available(target_id) = &preview.target
        && !input.observed_targets.contains(target_id)
    {
        return Err(ActionError::DanglingTarget {
            action_id: input.action_id.clone(),
            target_id: target_id.clone(),
        });
    }
    validate_affected(input, preview)?;
    validate_bounds(input, preview)?;
    validate_certainty(input, preview)?;
    validate_changes(input, preview, context)?;
    validate_statuses(input, preview, context)?;
    validate_movements(input, preview, context)?;
    validate_selections(input, preview, context)?;
    validate_named(input, preview, context)
}

/// Requires every affected target identity to be one this action actually presents.
fn validate_affected(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
) -> Result<(), ActionError> {
    if preview.affected.len() > ACTION_MAX_AFFECTED {
        return Err(ActionError::InvalidInput("affected"));
    }
    let mut seen = BTreeSet::new();
    for target_id in &preview.affected {
        validate_identity(target_id, "affected")?;
        if !seen.insert(target_id.as_str()) {
            return Err(ActionError::InvalidInput("affected"));
        }
        if !input.observed_targets.contains(target_id) {
            return Err(ActionError::DanglingTarget {
                action_id: input.action_id.clone(),
                target_id: target_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_bounds(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
) -> Result<(), ActionError> {
    let within = preview.changes.len() <= ACTION_MAX_CHANGES
        && preview.statuses.len() <= ACTION_MAX_STATUSES
        && preview.movements.len() <= ACTION_MAX_MOVEMENTS
        && preview.selections.len() <= ACTION_MAX_SELECTION_REQUIREMENTS
        && preview.assumptions.len() <= ACTION_MAX_ASSUMPTIONS
        && preview.omissions.len() <= ACTION_MAX_OMISSIONS;
    if within {
        return Ok(());
    }
    Err(ActionError::InvalidPreview {
        action_id: input.action_id.clone(),
        preview_id: preview.preview_id.clone(),
    })
}

/// Rejects a preview that overstates or understates what it knows.
///
/// `Unavailable` may not carry a consequence, `DeterministicExact` must state one and may not name
/// an omission or assumption, and a random or unsupported chain must never be reported as exact.
/// Exactness claimed over an unconsumed random draw is refused by name, so the caller learns the
/// chain is random rather than only that the declaration is unusable.
fn validate_certainty(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
) -> Result<(), ActionError> {
    let states_consequence = !preview.changes.is_empty()
        || !preview.statuses.is_empty()
        || !preview.movements.is_empty()
        || !preview.selections.is_empty();
    let exact = preview.class == ActionPreviewClass::DeterministicExact;
    let uncertain_omission = preview.omissions.iter().any(|omission| {
        matches!(
            omission.kind,
            ActionOmissionKind::RandomOutcome | ActionOmissionKind::UnsupportedChain
        )
    });
    let rejected = match preview.class {
        ActionPreviewClass::Unavailable => states_consequence || !preview.affected.is_empty(),
        ActionPreviewClass::DeterministicExact => {
            !states_consequence || !preview.omissions.is_empty() || !preview.assumptions.is_empty()
        }
        ActionPreviewClass::Conditional
        | ActionPreviewClass::RangeDistribution
        | ActionPreviewClass::Partial => false,
    };
    if exact && uncertain_omission {
        return Err(ActionError::UncertainPreview {
            action_id: input.action_id.clone(),
            preview_id: preview.preview_id.clone(),
            class: preview.class,
        });
    }
    if rejected {
        return Err(ActionError::InvalidPreview {
            action_id: input.action_id.clone(),
            preview_id: preview.preview_id.clone(),
        });
    }
    Ok(())
}
