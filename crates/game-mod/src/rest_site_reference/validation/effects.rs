// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::effect::{RestEffect, RestEffectKind, RestHealAmount, RestProspectiveComparison};
use super::super::error::RestSiteError;
use super::super::field::RestField;
use super::super::identity::{validate_identity, validate_text, validate_text_value};
use super::super::model::{
    REST_MAX_CANDIDATES, REST_MAX_EFFECTS, REST_MAX_HEAL_MODIFIERS, REST_MAX_PROSPECTIVE_CHANGES,
    REST_MAX_REFERENCES, RestVisibility,
};
use super::super::option::RestOptionInput;
use super::super::requirement::{RestSelectionRequirement, domain_is_empty};
use super::references::RefContext;

pub(super) fn validate_effects(
    option: &RestOptionInput,
    site_id: &str,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    if option.effects.len() > REST_MAX_EFFECTS {
        return Err(RestSiteError::InvalidInput("effects"));
    }
    let mut seen = BTreeSet::new();
    for effect in &option.effects {
        if !seen.insert(effect.effect_id.clone()) {
            return Err(RestSiteError::DuplicateEffect {
                site_id: site_id.to_owned(),
                option_id: option.option_id.clone(),
                effect_id: effect.effect_id.clone(),
            });
        }
        validate_effect(effect, option, site_id, context)?;
    }
    Ok(())
}

fn validate_effect(
    effect: &RestEffect,
    option: &RestOptionInput,
    site_id: &str,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    let invalid = || RestSiteError::InvalidEffect {
        site_id: site_id.to_owned(),
        option_id: option.option_id.clone(),
        effect_id: effect.effect_id.clone(),
    };
    validate_identity(&effect.effect_id, "effect_id")?;
    validate_text_value(&effect.label, "label")?;
    if effect.references.len() > REST_MAX_REFERENCES {
        return Err(RestSiteError::InvalidInput("references"));
    }
    context.check_all(&effect.references, option.visibility)?;
    context.check_optional(&effect.target, option.visibility)?;
    let healing = effect.healing.as_ref();
    let healing_kind = matches!(effect.kind, RestEffectKind::Heal);
    if healing.is_some() != healing_kind {
        return Err(invalid());
    }
    let Some(healing) = healing else {
        return Ok(());
    };
    if healing.modifiers.len() > REST_MAX_HEAL_MODIFIERS {
        return Err(RestSiteError::InvalidInput("heal_modifiers"));
    }
    if healing
        .base_percent
        .value()
        .is_some_and(|value| *value > 100)
    {
        return Err(invalid());
    }
    validate_heal_modifiers(healing, &invalid)?;
    ensure_heal_total_is_not_invented(healing, &invalid)?;
    Ok(())
}

fn validate_heal_modifiers(
    healing: &RestHealAmount,
    invalid: &impl Fn() -> RestSiteError,
) -> Result<(), RestSiteError> {
    let mut seen = BTreeSet::new();
    for modifier in &healing.modifiers {
        validate_identity(&modifier.modifier_id, "modifier_id")?;
        if !seen.insert(modifier.modifier_id.clone()) {
            return Err(invalid());
        }
        validate_text_value(&modifier.label, "label")?;
    }
    Ok(())
}

/// Rejects a resolved amount that contradicts its own documented contributors.
///
/// When the source reports a resolved total that disagrees with the documented base while listing
/// no modifier at all, the contributors are incomplete and the total is not trustworthy, so the
/// snapshot is refused instead of publishing a folded number the source cannot justify.
fn ensure_heal_total_is_not_invented(
    healing: &RestHealAmount,
    invalid: &impl Fn() -> RestSiteError,
) -> Result<(), RestSiteError> {
    let (Some(base), Some(resolved)) = (
        healing.base_percent.value(),
        healing.resolved_percent.value(),
    ) else {
        return Ok(());
    };
    let unmodified = healing.modifiers.is_empty() && !healing.flat_bonus.is_available();
    if unmodified && base != resolved {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_selection(
    selection: &RestSelectionRequirement,
    site_id: &str,
    option_id: &str,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    let invalid = || RestSiteError::InvalidSelection {
        site_id: site_id.to_owned(),
        option_id: option_id.to_owned(),
    };
    if selection.candidates.len() > REST_MAX_CANDIDATES
        || selection.references.len() > REST_MAX_REFERENCES
    {
        return Err(RestSiteError::InvalidInput("selection"));
    }
    if domain_is_empty(&selection.domain)
        && (selection.required || !selection.candidates.is_empty())
    {
        return Err(invalid());
    }
    if let (Some(minimum), Some(maximum)) = (selection.minimum.value(), selection.maximum.value())
        && minimum > maximum
    {
        return Err(invalid());
    }
    if selection.required && selection.maximum.value() == Some(&0) {
        return Err(invalid());
    }
    context.check_all(&selection.references, RestVisibility::Visible)?;
    let mut seen = BTreeSet::new();
    for candidate in &selection.candidates {
        validate_identity(&candidate.candidate_id, "candidate_id")?;
        if !seen.insert(candidate.candidate_id.clone()) {
            return Err(RestSiteError::DuplicateCandidate {
                site_id: site_id.to_owned(),
                option_id: option_id.to_owned(),
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        validate_text_value(&candidate.label, "label")?;
        context.check(&candidate.reference, RestVisibility::Visible)?;
        context.check_all(&candidate.references, RestVisibility::Visible)?;
    }
    Ok(())
}

pub(super) fn validate_comparison(
    comparison: &RestProspectiveComparison,
    site_id: &str,
    option_id: &str,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    if comparison.changes.len() > REST_MAX_PROSPECTIVE_CHANGES {
        return Err(RestSiteError::InvalidInput("changes"));
    }
    if comparison.option_id != option_id {
        return Err(RestSiteError::InvalidInput("comparison_option"));
    }
    let invalid = |change_id: &str| RestSiteError::InvalidComparison {
        site_id: site_id.to_owned(),
        option_id: option_id.to_owned(),
        change_id: change_id.to_owned(),
    };
    let mut seen = BTreeSet::new();
    for change in &comparison.changes {
        validate_identity(&change.change_id, "change_id")?;
        if !seen.insert(change.change_id.clone()) {
            return Err(invalid(&change.change_id));
        }
        validate_text_value(&change.label, "label")?;
        for value in [change.before.value(), change.after.value()]
            .into_iter()
            .flatten()
        {
            validate_text(value, "comparison_value")?;
        }
        context.check_optional(&change.target, RestVisibility::Visible)?;
        context.check_all(&change.references, RestVisibility::Visible)?;
    }
    let derived = !comparison.changes.is_empty()
        && comparison
            .changes
            .iter()
            .all(|change| change.before.is_available() && change.after.is_available());
    if comparison.complete != derived {
        return Err(RestSiteError::InvalidInput("comparison_complete"));
    }
    if let RestField::Available(upgrade) = &comparison.upgrade {
        context.check(&upgrade.candidate, RestVisibility::Visible)?;
        context.check_all(&upgrade.references, RestVisibility::Visible)?;
    }
    Ok(())
}
