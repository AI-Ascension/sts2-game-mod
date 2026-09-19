// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::{
    candidate::{
        SelectionBlockReason, SelectionCandidateInput, SelectionEligibility,
        SelectionEligibilityState, SelectionProspectiveEffect, SelectionProspectiveEffectKind,
    },
    error::SelectionError,
    field::SelectionField,
    identity::{validate_identity, validate_text, validate_text_value},
    model::{
        SEL_MAX_EFFECTS, SELECTION_REFERENCE_CARD_KIND, SELECTION_REFERENCE_PLAYER_KIND,
        SELECTION_REFERENCE_POTION_KIND, SELECTION_REFERENCE_RELIC_KIND, SelectionCandidateKind,
        SelectionVisibility,
    },
};
use super::{RefContext, manifest};

/// Validates one candidate against its owning selection and the same-snapshot references.
pub(super) fn validate_candidate(
    candidate: &SelectionCandidateInput,
    selection_id: &str,
    context: &RefContext<'_>,
) -> Result<(), SelectionError> {
    validate_identity(&candidate.candidate_id, "candidate_id")?;
    validate_text_value(&candidate.label, "label")?;
    validate_candidate_family(candidate)?;
    context.check(&candidate.definition, candidate.visibility)?;
    validate_eligibility(&candidate.eligibility, candidate, selection_id)?;
    validate_prospective(candidate, selection_id, context)?;
    context.check_all(&candidate.references, candidate.visibility)
}

/// Requires a candidate that reports a manifest family to name that family it resolves to.
///
/// A candidate never silently resolves to a narrower or wider family than the one it publishes, and
/// a typed candidate never carries an unclassified definition reference.
fn validate_candidate_family(candidate: &SelectionCandidateInput) -> Result<(), SelectionError> {
    let Some(reported) = candidate_family_token(&candidate.kind) else {
        return Ok(());
    };
    let resolved = manifest::reference_kind_token(&candidate.definition.kind);
    if resolved != Some(reported) {
        return Err(SelectionError::InvalidInput("candidate_kind"));
    }
    Ok(())
}

/// Returns the manifest family one candidate kind resolves to, when it names one.
fn candidate_family_token(kind: &SelectionCandidateKind) -> Option<&'static str> {
    match kind {
        SelectionCandidateKind::Card | SelectionCandidateKind::UpgradeTarget => {
            Some(SELECTION_REFERENCE_CARD_KIND)
        }
        SelectionCandidateKind::Relic => Some(SELECTION_REFERENCE_RELIC_KIND),
        SelectionCandidateKind::Potion => Some(SELECTION_REFERENCE_POTION_KIND),
        SelectionCandidateKind::Player => Some(SELECTION_REFERENCE_PLAYER_KIND),
        SelectionCandidateKind::RewardItem
        | SelectionCandidateKind::ShopItem
        | SelectionCandidateKind::RestOption
        | SelectionCandidateKind::Custom(_)
        | SelectionCandidateKind::Unknown => None,
    }
}

/// Requires a refused candidate to explain itself and an eligible candidate not to carry a refusal.
///
/// A candidate the reader may observe also may not report a withheld refusal, so an inconsistent
/// disclosure is refused instead of published.
fn validate_eligibility(
    eligibility: &SelectionEligibility,
    candidate: &SelectionCandidateInput,
    selection_id: &str,
) -> Result<(), SelectionError> {
    if let SelectionField::Available(SelectionBlockReason::Custom(reason)) = &eligibility.reason {
        validate_text(reason, "block_reason")?;
    }
    let withheld_but_visible = candidate.visibility == SelectionVisibility::Visible
        && matches!(
            eligibility.reason,
            SelectionField::Available(SelectionBlockReason::Withheld)
        );
    let refused = eligibility.state == SelectionEligibilityState::Ineligible;
    let refuses_silently = refused && !eligibility.reason.is_available();
    let offered_with_reason = !refused && eligibility.reason.is_available();
    if withheld_but_visible || refuses_silently || offered_with_reason {
        return Err(SelectionError::InvalidEligibility {
            selection_id: selection_id.to_owned(),
            candidate_id: candidate.candidate_id.clone(),
        });
    }
    Ok(())
}

/// Requires each documented effect to stay bounded, unique, and a real change.
fn validate_prospective(
    candidate: &SelectionCandidateInput,
    selection_id: &str,
    context: &RefContext<'_>,
) -> Result<(), SelectionError> {
    if candidate.prospective.len() > SEL_MAX_EFFECTS {
        return Err(SelectionError::InvalidInput("prospective"));
    }
    let mut seen = BTreeSet::new();
    for effect in &candidate.prospective {
        validate_effect(effect, candidate, selection_id, context)?;
        if !seen.insert(effect.effect_id.as_str()) {
            return Err(SelectionError::InvalidInput("effect_id"));
        }
    }
    Ok(())
}

fn validate_effect(
    effect: &SelectionProspectiveEffect,
    candidate: &SelectionCandidateInput,
    selection_id: &str,
    context: &RefContext<'_>,
) -> Result<(), SelectionError> {
    validate_identity(&effect.effect_id, "effect_id")?;
    validate_text_value(&effect.label, "label")?;
    if let SelectionProspectiveEffectKind::Custom(kind) = &effect.kind {
        validate_text(kind, "effect_kind")?;
    }
    context.check_optional(&effect.target, candidate.visibility)?;
    context.check_all(&effect.references, candidate.visibility)?;
    let before = effect.before.value();
    let after = effect.after.value();
    let changes_nothing = match (before, after) {
        (None, None) => true,
        (Some(before), Some(after)) => before == after,
        _ => false,
    };
    if changes_nothing {
        return Err(SelectionError::InvalidProspectiveEffect {
            selection_id: selection_id.to_owned(),
            candidate_id: candidate.candidate_id.clone(),
            effect_id: effect.effect_id.clone(),
        });
    }
    Ok(())
}
