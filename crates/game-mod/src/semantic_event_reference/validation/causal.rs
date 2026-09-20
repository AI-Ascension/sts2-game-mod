// SPDX-License-Identifier: MIT

//! Causality as a stated fact: admitted, present, existing, and earlier.

use std::collections::BTreeMap;

use super::super::{SemanticCausalProvenance, SemanticEventError, SemanticEventInput};

/// Validates the causal parent of one record against the events this history observed.
///
/// A parent is never inferred from a difference between two snapshots, so the only parents this
/// boundary accepts are ones the host stated. A stated parent that this history does not contain, or
/// that does not precede its child, is refused rather than kept as unverified causality.
pub(super) fn validate_causal_parent(
    event: &SemanticEventInput,
    observed: &BTreeMap<&str, u64>,
) -> Result<(), SemanticEventError> {
    if !event.is_observed() {
        return Ok(());
    }
    let Some(kind) = event.kind else {
        return Ok(());
    };
    let admits_cause = kind.admits_cause();
    let admits_stated = admits_cause
        && event
            .origin
            .is_some_and(|origin| origin.admits_stated_parent());
    match &event.causal_parent {
        Some(parent) => {
            let admitted = match parent.provenance {
                SemanticCausalProvenance::Stated => admits_stated,
                SemanticCausalProvenance::NotStated => admits_cause,
            };
            if !admitted {
                return Err(SemanticEventError::CausalityNotAdmitted(
                    event.event_id.clone(),
                ));
            }
            validate_parent(
                parent.provenance,
                parent.parent_event_id.as_deref(),
                event,
                observed,
            )
        }
        None if admits_cause => Err(SemanticEventError::MissingCausalParent(
            event.event_id.clone(),
        )),
        None => Ok(()),
    }
}

fn validate_parent(
    provenance: SemanticCausalProvenance,
    parent_id: Option<&str>,
    event: &SemanticEventInput,
    observed: &BTreeMap<&str, u64>,
) -> Result<(), SemanticEventError> {
    match (provenance, parent_id) {
        (SemanticCausalProvenance::NotStated, None) => Ok(()),
        (SemanticCausalProvenance::Stated, Some(parent_id)) => {
            let Some(parent_sequence) = observed.get(parent_id) else {
                return Err(SemanticEventError::StatedParentUnknown(
                    parent_id.to_owned(),
                ));
            };
            if *parent_sequence >= event.sequence {
                return Err(SemanticEventError::StatedParentNotBefore(
                    parent_id.to_owned(),
                ));
            }
            Ok(())
        }
        _ => Err(SemanticEventError::CausalProvenanceMismatch(
            event.event_id.clone(),
        )),
    }
}
