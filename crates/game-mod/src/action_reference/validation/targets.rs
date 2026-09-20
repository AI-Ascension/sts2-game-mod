// SPDX-License-Identifier: MIT

use super::super::{
    eligibility::ActionEligibility,
    error::ActionError,
    field::ActionField,
    identity::{validate_identity, validate_text_value},
    kind::{ActionEligibilityState, ActionRefusalReason},
    model::{
        ACTION_REFERENCE_CARD_KIND, ACTION_REFERENCE_ENEMY_KIND, ACTION_REFERENCE_PLAYER_KIND,
        ACTION_REFERENCE_POTION_KIND, ACTION_REFERENCE_RELIC_KIND, ActionVisibility,
    },
    target::{ActionTargetInput, ActionTargetKind},
};
use super::{RefContext, manifest};

/// Validates one observed target against its owning action and the same-snapshot references.
pub(super) fn validate_target(
    target: &ActionTargetInput,
    action_id: &str,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    validate_identity(&target.target_id, "target_id")?;
    validate_text_value(&target.label, "label")?;
    validate_target_family(target)?;
    context.check(&target.definition, target.visibility)?;
    if eligibility_is_inconsistent(&target.eligibility, target.visibility) {
        return Err(ActionError::InvalidTargetEligibility {
            action_id: action_id.to_owned(),
            target_id: target.target_id.clone(),
        });
    }
    if let ActionField::Available(reason) = &target.eligibility.reason
        && let ActionRefusalReason::Custom(reason) = reason
    {
        validate_identity(reason, "refusal_reason")?;
    }
    context.check_all(&target.references, target.visibility)
}

/// Requires a target that reports a manifest family to name that family it resolves to.
///
/// A target never silently resolves to a narrower or wider family than the one it publishes, and a
/// typed target never carries an unclassified definition reference.
fn validate_target_family(target: &ActionTargetInput) -> Result<(), ActionError> {
    let Some(reported) = target_family_token(&target.kind) else {
        return Ok(());
    };
    let resolved = manifest::reference_kind_token(&target.definition.kind);
    if resolved != Some(reported) {
        return Err(ActionError::InvalidInput("target_kind"));
    }
    Ok(())
}

/// Returns the manifest family one target kind resolves to, when it names one.
fn target_family_token(kind: &ActionTargetKind) -> Option<&'static str> {
    match kind {
        ActionTargetKind::Enemy => Some(ACTION_REFERENCE_ENEMY_KIND),
        ActionTargetKind::Player => Some(ACTION_REFERENCE_PLAYER_KIND),
        ActionTargetKind::Card => Some(ACTION_REFERENCE_CARD_KIND),
        ActionTargetKind::Potion => Some(ACTION_REFERENCE_POTION_KIND),
        ActionTargetKind::Relic => Some(ACTION_REFERENCE_RELIC_KIND),
        ActionTargetKind::ShopItem
        | ActionTargetKind::Destination
        | ActionTargetKind::Custom(_)
        | ActionTargetKind::Unknown => None,
    }
}

/// Returns whether an eligibility record contradicts its own state and reason.
///
/// A refused subject must state a reason and its host text, an available subject must state
/// neither, and a subject a caller may observe may not report a withheld refusal. A refusal that
/// exists but must not be revealed belongs to a hidden subject, not to a visible one.
pub(super) fn eligibility_is_inconsistent(
    eligibility: &ActionEligibility,
    visibility: ActionVisibility,
) -> bool {
    let withheld_but_visible = visibility == ActionVisibility::Visible
        && matches!(
            eligibility.reason,
            ActionField::Available(ActionRefusalReason::Withheld)
        );
    let refused = matches!(eligibility.state, ActionEligibilityState::Unavailable);
    let offered = matches!(eligibility.state, ActionEligibilityState::Available);
    let refuses_silently = refused && !eligibility.reason.is_available();
    let refuses_without_text = refused && !eligibility.reason_text.is_available();
    let offered_with_reason =
        offered && (eligibility.reason.is_available() || eligibility.reason_text.is_available());
    let reason_without_text =
        eligibility.reason.is_available() && !eligibility.reason_text.is_available();
    let text_without_reason =
        eligibility.reason_text.is_available() && !eligibility.reason.is_available();
    withheld_but_visible
        || refuses_silently
        || refuses_without_text
        || offered_with_reason
        || reason_without_text
        || text_without_reason
}
