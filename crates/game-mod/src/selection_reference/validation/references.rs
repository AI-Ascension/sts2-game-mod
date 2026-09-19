// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    error::SelectionError,
    field::SelectionField,
    identity::{validate_identity, validate_text_value},
    model::{
        SEL_MAX_REFERENCES, SelectionReferenceKind, SelectionSemanticReference,
        SelectionVisibility, selection_visibility_rank,
    },
};
use super::SelectionTarget;

/// Same-snapshot context every typed reference is checked against.
pub(super) struct RefContext<'a> {
    pub(super) selection_id: &'a str,
    selections: &'a BTreeMap<String, SelectionTarget>,
    candidate_ids: &'a BTreeSet<String>,
}

impl RefContext<'_> {
    /// Creates the reference context one definition is validated against.
    pub(super) fn new<'a>(
        selection_id: &'a str,
        selections: &'a BTreeMap<String, SelectionTarget>,
        candidate_ids: &'a BTreeSet<String>,
    ) -> RefContext<'a> {
        RefContext {
            selection_id,
            selections,
            candidate_ids,
        }
    }

    /// Validates one typed reference against the snapshot and the referring visibility.
    pub(super) fn check(
        &self,
        reference: &SelectionSemanticReference,
        owner_visibility: SelectionVisibility,
    ) -> Result<(), SelectionError> {
        validate_text_value(&reference.label, "reference_label")?;
        validate_identity(&reference.id, "reference_id")?;
        let target = match &reference.kind {
            SelectionReferenceKind::Selection => self
                .selections
                .get(&reference.id)
                .map(|target| target.visibility),
            SelectionReferenceKind::Candidate => self
                .candidate_ids
                .contains(&reference.id)
                .then_some(SelectionVisibility::Visible),
            SelectionReferenceKind::Effect
            | SelectionReferenceKind::Card
            | SelectionReferenceKind::Relic
            | SelectionReferenceKind::Potion
            | SelectionReferenceKind::Player
            | SelectionReferenceKind::Content { .. }
            | SelectionReferenceKind::Unknown => return Ok(()),
        };
        let Some(target) = target else {
            return Err(SelectionError::DanglingReference {
                selection_id: self.selection_id.to_owned(),
                reference_kind: reference.kind.clone(),
                id: reference.id.clone(),
            });
        };
        if selection_visibility_rank(target) < selection_visibility_rank(owner_visibility) {
            return Err(SelectionError::HiddenReferenceLeak {
                selection_id: self.selection_id.to_owned(),
                reference_kind: reference.kind.clone(),
            });
        }
        Ok(())
    }

    /// Validates one bounded reference list.
    pub(super) fn check_all(
        &self,
        references: &[SelectionSemanticReference],
        owner_visibility: SelectionVisibility,
    ) -> Result<(), SelectionError> {
        if references.len() > SEL_MAX_REFERENCES {
            return Err(SelectionError::InvalidInput("references"));
        }
        for reference in references {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }

    /// Validates one optional typed reference.
    pub(super) fn check_optional(
        &self,
        field: &SelectionField<SelectionSemanticReference>,
        owner_visibility: SelectionVisibility,
    ) -> Result<(), SelectionError> {
        if let SelectionField::Available(reference) = field {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }
}
