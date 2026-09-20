// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    error::ActionError,
    field::ActionField,
    identity::{validate_identity, validate_text_value},
    model::{
        ACTION_MAX_REFERENCES, ActionReferenceKind, ActionSemanticReference, ActionVisibility,
        action_visibility_rank,
    },
};
use super::ActionFrameTarget;

/// Same-snapshot context every typed reference is checked against.
pub(super) struct RefContext<'a> {
    pub(super) action_id: &'a str,
    actions: &'a BTreeMap<String, ActionFrameTarget>,
    target_ids: &'a BTreeSet<String>,
}

impl RefContext<'_> {
    /// Creates the reference context one definition is validated against.
    pub(super) fn new<'a>(
        action_id: &'a str,
        actions: &'a BTreeMap<String, ActionFrameTarget>,
        target_ids: &'a BTreeSet<String>,
    ) -> RefContext<'a> {
        RefContext {
            action_id,
            actions,
            target_ids,
        }
    }

    /// Validates one typed reference against the snapshot and the referring visibility.
    ///
    /// A reference this producer resolves locally is refused when it names nothing in the snapshot,
    /// and a reference whose target is more restricted than the referring record is refused without
    /// naming the restricted identity.
    pub(super) fn check(
        &self,
        reference: &ActionSemanticReference,
        owner_visibility: ActionVisibility,
    ) -> Result<(), ActionError> {
        validate_text_value(&reference.label, "reference_label")?;
        validate_identity(&reference.id, "reference_id")?;
        let target = match &reference.kind {
            ActionReferenceKind::Action => self
                .actions
                .get(&reference.id)
                .map(|target| target.visibility),
            ActionReferenceKind::Destination => self
                .target_ids
                .contains(&reference.id)
                .then_some(ActionVisibility::Visible),
            ActionReferenceKind::Effect
            | ActionReferenceKind::Selection
            | ActionReferenceKind::Card
            | ActionReferenceKind::Enemy
            | ActionReferenceKind::Player
            | ActionReferenceKind::Status
            | ActionReferenceKind::Relic
            | ActionReferenceKind::Potion
            | ActionReferenceKind::Resource
            | ActionReferenceKind::Content { .. }
            | ActionReferenceKind::Unknown => return Ok(()),
        };
        let Some(target) = target else {
            return Err(ActionError::DanglingReference {
                action_id: self.action_id.to_owned(),
                reference_kind: reference.kind.clone(),
                id: reference.id.clone(),
            });
        };
        if action_visibility_rank(target) < action_visibility_rank(owner_visibility) {
            return Err(ActionError::HiddenReferenceLeak {
                action_id: self.action_id.to_owned(),
                reference_kind: reference.kind.clone(),
            });
        }
        Ok(())
    }

    /// Validates one bounded reference list.
    pub(super) fn check_all(
        &self,
        references: &[ActionSemanticReference],
        owner_visibility: ActionVisibility,
    ) -> Result<(), ActionError> {
        if references.len() > ACTION_MAX_REFERENCES {
            return Err(ActionError::InvalidInput("references"));
        }
        for reference in references {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }

    /// Validates one optional typed reference.
    pub(super) fn check_optional(
        &self,
        field: &ActionField<ActionSemanticReference>,
        owner_visibility: ActionVisibility,
    ) -> Result<(), ActionError> {
        if let ActionField::Available(reference) = field {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }
}
