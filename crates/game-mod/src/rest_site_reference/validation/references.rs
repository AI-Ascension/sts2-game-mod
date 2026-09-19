// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    error::RestSiteError,
    field::RestField,
    identity::{validate_identity, validate_text_value},
    model::{
        REST_MAX_REFERENCES, RestReferenceKind, RestSemanticReference, RestVisibility,
        visibility_rank,
    },
};
use super::RestTarget;

/// Same-snapshot context every typed reference is checked against.
pub(super) struct RefContext<'a> {
    pub(super) site_id: &'a str,
    sites: &'a BTreeMap<String, RestTarget>,
    option_ids: &'a BTreeSet<String>,
}

impl RefContext<'_> {
    /// Creates the reference context one definition is validated against.
    pub(super) fn new<'a>(
        site_id: &'a str,
        sites: &'a BTreeMap<String, RestTarget>,
        option_ids: &'a BTreeSet<String>,
    ) -> RefContext<'a> {
        RefContext {
            site_id,
            sites,
            option_ids,
        }
    }

    /// Validates one typed reference against the snapshot and the referring visibility.
    pub(super) fn check(
        &self,
        reference: &RestSemanticReference,
        owner_visibility: RestVisibility,
    ) -> Result<(), RestSiteError> {
        validate_text_value(&reference.label, "reference_label")?;
        validate_identity(&reference.id, "reference_id")?;
        let target = match &reference.kind {
            RestReferenceKind::Site => self
                .sites
                .get(&reference.id)
                .map(|target| target.visibility),
            RestReferenceKind::Option => self
                .option_ids
                .contains(&reference.id)
                .then_some(RestVisibility::Visible),
            RestReferenceKind::Effect
            | RestReferenceKind::Card
            | RestReferenceKind::Relic
            | RestReferenceKind::Potion
            | RestReferenceKind::Player
            | RestReferenceKind::Content { .. }
            | RestReferenceKind::Unknown => return Ok(()),
        };
        let Some(target) = target else {
            return Err(RestSiteError::DanglingReference {
                site_id: self.site_id.to_owned(),
                reference_kind: reference.kind.clone(),
                id: reference.id.clone(),
            });
        };
        if visibility_rank(target) < visibility_rank(owner_visibility) {
            return Err(RestSiteError::HiddenReferenceLeak {
                site_id: self.site_id.to_owned(),
                reference_kind: reference.kind.clone(),
            });
        }
        Ok(())
    }

    /// Validates one bounded reference list.
    pub(super) fn check_all(
        &self,
        references: &[RestSemanticReference],
        owner_visibility: RestVisibility,
    ) -> Result<(), RestSiteError> {
        if references.len() > REST_MAX_REFERENCES {
            return Err(RestSiteError::InvalidInput("references"));
        }
        for reference in references {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }

    /// Validates one optional typed reference.
    pub(super) fn check_optional(
        &self,
        field: &RestField<RestSemanticReference>,
        owner_visibility: RestVisibility,
    ) -> Result<(), RestSiteError> {
        if let RestField::Available(reference) = field {
            self.check(reference, owner_visibility)?;
        }
        Ok(())
    }
}
