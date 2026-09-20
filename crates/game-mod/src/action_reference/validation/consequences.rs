// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::{
    definition::ActionDefinitionInput,
    error::ActionError,
    identity::{validate_identity, validate_text, validate_text_value},
    kind::{ActionEffectKind, ActionOmissionKind, ActionReferenceKind},
    model::ACTION_MAX_REQUIRED_PICKS,
    preview::{ActionPreviewChange, ActionPreviewInput},
};
use super::RefContext;

/// Validates every stated consequence change of one preview.
pub(super) fn validate_changes(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    let mut seen = BTreeSet::new();
    for change in &preview.changes {
        validate_identity(&change.change_id, "change_id")?;
        if !seen.insert(change.change_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&change.label, "label")?;
        validate_text_value(&change.unit, "unit")?;
        if let ActionEffectKind::Custom(kind) = &change.kind {
            validate_text(kind, "effect_kind")?;
        }
        context.check_optional(&change.target, input.visibility)?;
        context.check_all(&change.references, input.visibility)?;
        if changes_nothing(change) {
            return Err(ActionError::InvalidPreviewChange {
                action_id: input.action_id.clone(),
                preview_id: preview.preview_id.clone(),
                change_id: change.change_id.clone(),
            });
        }
    }
    Ok(())
}

/// Returns whether one change would leave every value it states exactly as it was.
fn changes_nothing(change: &ActionPreviewChange) -> bool {
    if change.delta.value() == Some(&0) {
        return true;
    }
    match (change.before.value(), change.after.value()) {
        (None, None) => true,
        (Some(before), Some(after)) => before == after,
        _ => false,
    }
}

/// Validates every status or power transition of one preview.
pub(super) fn validate_statuses(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    let mut seen = BTreeSet::new();
    for status in &preview.statuses {
        validate_identity(&status.status_id, "status_id")?;
        if !seen.insert(status.status_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&status.label, "label")?;
        let typed = status.status.kind == ActionReferenceKind::Status;
        if !typed || status.magnitude.value() == Some(&0) {
            return Err(invalid(input, preview));
        }
        context.check(&status.status, input.visibility)?;
        context.check_optional(&status.target, input.visibility)?;
        context.check_all(&status.references, input.visibility)?;
    }
    Ok(())
}

/// Validates every card movement of one preview.
pub(super) fn validate_movements(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    let mut seen = BTreeSet::new();
    for movement in &preview.movements {
        validate_identity(&movement.movement_id, "movement_id")?;
        if !seen.insert(movement.movement_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&movement.label, "label")?;
        let typed = movement.card.kind == ActionReferenceKind::Card;
        if !typed || movement.count == 0 || movement.from == movement.to {
            return Err(invalid(input, preview));
        }
        context.check(&movement.card, input.visibility)?;
        context.check_all(&movement.references, input.visibility)?;
    }
    Ok(())
}

/// Validates every selection requirement of one preview.
pub(super) fn validate_selections(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    let mut seen = BTreeSet::new();
    for selection in &preview.selections {
        validate_identity(&selection.requirement_id, "requirement_id")?;
        if !seen.insert(selection.requirement_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&selection.label, "label")?;
        let typed = selection.selection.kind == ActionReferenceKind::Selection;
        let maximum = selection.maximum.value().copied();
        let bounded = selection.required > 0
            && selection.required <= ACTION_MAX_REQUIRED_PICKS
            && maximum.is_none_or(|maximum| {
                maximum >= selection.required && maximum <= ACTION_MAX_REQUIRED_PICKS
            });
        if !typed || !bounded {
            return Err(invalid(input, preview));
        }
        context.check(&selection.selection, input.visibility)?;
        context.check_all(&selection.references, input.visibility)?;
    }
    Ok(())
}

/// Validates the named assumptions and omissions of one preview.
///
/// A preview that names what it assumed and what it did not describe is the only honest way to
/// publish a consequence set it cannot fully establish, so both lists are bounded and unique.
pub(super) fn validate_named(
    input: &ActionDefinitionInput,
    preview: &ActionPreviewInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    let mut assumptions = BTreeSet::new();
    for record in &preview.assumptions {
        validate_identity(&record.assumption_id, "assumption_id")?;
        if !assumptions.insert(record.assumption_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&record.label, "label")?;
        context.check_all(&record.references, input.visibility)?;
    }
    let mut omissions = BTreeSet::new();
    for record in &preview.omissions {
        validate_identity(&record.omission_id, "omission_id")?;
        if !omissions.insert(record.omission_id.as_str()) {
            return Err(invalid(input, preview));
        }
        validate_text_value(&record.label, "label")?;
        if let ActionOmissionKind::Custom(kind) = &record.kind {
            validate_text(kind, "omission_kind")?;
        }
        context.check_all(&record.references, input.visibility)?;
    }
    Ok(())
}

fn invalid(input: &ActionDefinitionInput, preview: &ActionPreviewInput) -> ActionError {
    ActionError::InvalidPreview {
        action_id: input.action_id.clone(),
        preview_id: preview.preview_id.clone(),
    }
}
