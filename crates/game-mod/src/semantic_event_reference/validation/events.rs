// SPDX-License-Identifier: MIT

//! One record's shape, and the sequence order of all of them.

use crate::ContentManifest;

use super::super::model::SEMANTIC_MAX_LABEL_BYTES;
use super::super::{SemanticCaptureWindow, SemanticEventError, SemanticEventInput};
use super::bounds::require_within;
use super::manifest::validate_reference;

/// Validates one record: coverage decides which fields exist, and the kind decides the rest.
pub(super) fn validate_event(
    event: &SemanticEventInput,
    window: &SemanticCaptureWindow,
    manifest: &ContentManifest,
) -> Result<(), SemanticEventError> {
    super::super::identity::validate_opaque_identity(&event.event_id, "event_id")?;
    validate_label(event)?;
    if !event.is_observed() {
        return validate_gap(event, window);
    }
    if window.interval_covering(event.sequence).is_some() {
        return Err(SemanticEventError::CoverageContradiction(event.sequence));
    }
    let Some(kind) = event.kind else {
        return Err(SemanticEventError::MissingKind(event.event_id.clone()));
    };
    if event.origin.is_none() {
        return Err(SemanticEventError::MissingOrigin(event.event_id.clone()));
    }
    if kind.requires_quantity() != event.value.is_some() {
        return Err(SemanticEventError::KindDetailMismatch(
            event.event_id.clone(),
        ));
    }
    if kind.requires_reference() != event.reference.is_some() {
        return Err(SemanticEventError::KindDetailMismatch(
            event.event_id.clone(),
        ));
    }
    if let Some(reference) = &event.reference {
        validate_reference(manifest, reference)?;
    }
    if let Some(quantity) = &event.value {
        validate_unit(&quantity.unit)?;
    }
    Ok(())
}

/// Validates that a disclosed gap carries no observed field.
fn validate_gap(
    event: &SemanticEventInput,
    window: &SemanticCaptureWindow,
) -> Result<(), SemanticEventError> {
    if event.kind.is_some() || event.origin.is_some() {
        return Err(SemanticEventError::UnexpectedKind(event.event_id.clone()));
    }
    if !event.subjects.is_empty() || event.causal_parent.is_some() || event.value.is_some() {
        return Err(SemanticEventError::KindDetailMismatch(
            event.event_id.clone(),
        ));
    }
    if event.reference.is_some() || event.label.is_some() {
        return Err(SemanticEventError::KindDetailMismatch(
            event.event_id.clone(),
        ));
    }
    if window.interval_covering(event.sequence).is_none() {
        return Err(SemanticEventError::UndeclaredGap(event.sequence));
    }
    Ok(())
}

/// Validates one opaque quantity unit against its local bound.
fn validate_unit(unit: &str) -> Result<(), SemanticEventError> {
    if unit.is_empty() || unit.len() > super::super::model::SEMANTIC_MAX_UNIT_BYTES {
        return Err(SemanticEventError::InvalidInput("quantity.unit"));
    }
    if !super::super::identity::is_opaque_semantic_identity(unit) {
        return Err(SemanticEventError::NonOpaqueIdentity("quantity.unit"));
    }
    Ok(())
}

fn validate_label(event: &SemanticEventInput) -> Result<(), SemanticEventError> {
    require_within(
        event.coverage.label.as_ref().map_or(0, String::len),
        SEMANTIC_MAX_LABEL_BYTES,
        "coverage.label",
    )?;
    require_within(
        event.label.as_ref().map_or(0, String::len),
        SEMANTIC_MAX_LABEL_BYTES,
        "label",
    )
}

/// Validates that one sequence is contiguous from the capture point and free of repeats.
pub(super) fn validate_sequences(
    events: &[SemanticEventInput],
    window: &SemanticCaptureWindow,
) -> Result<(), SemanticEventError> {
    let mut expected = window.capture_start_sequence;
    for event in events {
        if event.sequence != expected {
            return Err(SemanticEventError::NonMonotonicSequence(event.sequence));
        }
        expected = expected.saturating_add(1);
    }
    Ok(())
}
