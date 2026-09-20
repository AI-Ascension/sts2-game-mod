// SPDX-License-Identifier: MIT

//! Who an event is about: the roles it must name, and the namespaces it may not alias.

use std::collections::BTreeMap;

use super::super::definition::SemanticEventInput;
use super::super::model::SemanticEventScope;
use super::super::{
    SemanticEventError, SemanticIdentityNamespace, SemanticSubjectRole, is_opaque_semantic_identity,
};

/// Validates the subjects of one record against the kind it states.
///
/// A targeted kind without a target is a refusal rather than an event whose target is unknown, and an
/// actor that is absent where the boundary observed one is the same refusal: a consumer may not read
/// a missing subject as "no subject existed".
pub(super) fn validate_subjects(
    event: &SemanticEventInput,
    scope: &SemanticEventScope,
) -> Result<(), SemanticEventError> {
    if !event.is_observed() {
        return Ok(());
    }
    let Some(kind) = event.kind else {
        return Ok(());
    };
    let mut seen: BTreeMap<SemanticSubjectRole, &SemanticIdentityNamespace> = BTreeMap::new();
    for subject in &event.subjects {
        if !is_opaque_semantic_identity(&subject.subject_id) {
            return Err(SemanticEventError::NonOpaqueIdentity("subject_id"));
        }
        if !subject.namespace.admits_subject_role() {
            return Err(SemanticEventError::WrongSubjectNamespace(
                subject.role.name(),
            ));
        }
        if seen.insert(subject.role, &subject.namespace).is_some() {
            return Err(SemanticEventError::DuplicateSubjectRole(
                subject.role.name(),
            ));
        }
        if subject.subject_id == event.event_id {
            return Err(SemanticEventError::IdentityNamespaceCollision("event_id"));
        }
        if subject.subject_id == scope.run_id || subject.subject_id == scope.branch_id {
            return Err(SemanticEventError::IdentityNamespaceCollision("scope"));
        }
    }
    require_role(event, kind.requires_actor(), SemanticSubjectRole::Actor)?;
    require_role(event, kind.requires_target(), SemanticSubjectRole::Target)
}

fn require_role(
    event: &SemanticEventInput,
    required: bool,
    role: SemanticSubjectRole,
) -> Result<(), SemanticEventError> {
    let present = event.subjects.iter().any(|subject| subject.role == role);
    if required && !present {
        return Err(SemanticEventError::MissingSubject(role.name()));
    }
    if !required && present && role == SemanticSubjectRole::Target {
        return Err(SemanticEventError::UnexpectedSubjectRole(role.name()));
    }
    Ok(())
}

/// Returns the bytes one event's subjects and identity contribute.
pub(super) fn event_bytes(event: &SemanticEventInput) -> usize {
    event.event_id.len()
        + event
            .subjects
            .iter()
            .map(|subject| subject.subject_id.len())
            .sum::<usize>()
        + super::bounds::label_bytes(event)
        + event.value.as_ref().map_or(0, |value| value.unit.len())
        + event.reference.as_ref().map_or(0, |reference| {
            reference.entity_kind.len() + reference.namespaced_id.len()
        })
        + event.causal_parent.as_ref().map_or(0, |parent| {
            parent.parent_event_id.as_ref().map_or(0, String::len)
        })
}
