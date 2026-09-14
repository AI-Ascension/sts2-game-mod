// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::RewardCatalogError;
use super::super::definition::{
    RewardActionKind, RewardGenerationRule, RewardLegalAction, RewardModifier, RewardModifierKind,
    RewardParameter, RewardRequirement, RewardRequirementKind, RewardStatePolicy,
};
use super::super::model::{
    REWARD_MAX_LEGAL_ACTIONS, REWARD_MAX_MODIFIERS, REWARD_MAX_PARAMETERS,
    REWARD_MAX_RARITY_WEIGHTS, REWARD_MAX_REFERENCES, REWARD_MAX_REQUIREMENTS, REWARD_MAX_STAGES,
    RewardField, RewardRarityWeight, RewardSemanticReference, RewardSemanticReferenceKind,
    RewardText, RewardVisibility, validate_identity, validate_text,
};
use super::quantities::{
    modifier_is_magnitude, validate_numeric, validate_probability, validate_quantity,
};

pub(super) fn validate_text_value(
    value: &RewardText,
    field: &'static str,
) -> Result<(), RewardCatalogError> {
    if let RewardText::Available(text) = value {
        validate_text(text, field)?;
    }
    Ok(())
}

pub(super) fn validate_visibility(visibility: RewardVisibility) -> Result<(), RewardCatalogError> {
    if matches!(visibility, RewardVisibility::Unknown) {
        return Err(RewardCatalogError::InvalidInput("visibility"));
    }
    Ok(())
}

pub(super) fn validate_references(
    references: &[RewardSemanticReference],
) -> Result<(), RewardCatalogError> {
    if references.len() > REWARD_MAX_REFERENCES {
        return Err(RewardCatalogError::InvalidInput("references"));
    }
    let mut ids = BTreeSet::new();
    for reference in references {
        validate_reference_kind(&reference.kind)?;
        validate_identity(&reference.id, "reference_id")?;
        validate_text_value(&reference.label, "reference_label")?;
        if !ids.insert((&reference.kind, reference.id.as_str())) {
            return Err(RewardCatalogError::InvalidInput("duplicate_reference"));
        }
    }
    Ok(())
}

pub(super) fn validate_reference_kind(
    kind: &RewardSemanticReferenceKind,
) -> Result<(), RewardCatalogError> {
    if let RewardSemanticReferenceKind::Content { entity_kind } = kind {
        validate_identity(entity_kind, "reference_entity_kind")?;
    }
    Ok(())
}

pub(super) fn validate_parameters(
    parameters: &[RewardParameter],
) -> Result<(), RewardCatalogError> {
    if parameters.len() > REWARD_MAX_PARAMETERS {
        return Err(RewardCatalogError::InvalidInput("parameters"));
    }
    let mut ids = BTreeSet::new();
    for parameter in parameters {
        validate_identity(&parameter.parameter_id, "parameter_id")?;
        validate_text_value(&parameter.label, "parameter_label")?;
        if let Some(unit) = &parameter.unit {
            validate_identity(unit, "parameter_unit")?;
        }
        validate_numeric(&parameter.value)?;
        if !ids.insert(parameter.parameter_id.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_parameter"));
        }
    }
    Ok(())
}

pub(super) fn validate_requirement_kind(
    kind: &RewardRequirementKind,
) -> Result<(), RewardCatalogError> {
    if let RewardRequirementKind::Custom(value) = kind {
        validate_identity(value, "requirement_kind")?;
    }
    Ok(())
}

pub(super) fn validate_requirement(
    requirement: &RewardRequirement,
) -> Result<(), RewardCatalogError> {
    validate_identity(&requirement.requirement_id, "requirement_id")?;
    validate_requirement_kind(&requirement.kind)?;
    validate_text_value(&requirement.label, "requirement_label")?;
    validate_parameters(&requirement.parameters)?;
    validate_references(&requirement.references)?;
    validate_visibility(requirement.visibility)
}

pub(super) fn validate_requirement_list(
    requirements: &[RewardRequirement],
) -> Result<(), RewardCatalogError> {
    if requirements.len() > REWARD_MAX_REQUIREMENTS {
        return Err(RewardCatalogError::InvalidInput("requirements"));
    }
    let mut ids = BTreeSet::new();
    for requirement in requirements {
        validate_requirement(requirement)?;
        if !ids.insert(requirement.requirement_id.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_requirement"));
        }
    }
    Ok(())
}

pub(super) fn validate_action_kind(kind: &RewardActionKind) -> Result<(), RewardCatalogError> {
    if let RewardActionKind::Custom(value) | RewardActionKind::Unsupported(value) = kind {
        validate_identity(value, "action_kind")?;
    }
    Ok(())
}

pub(super) fn validate_legal_actions(
    actions: &[RewardLegalAction],
) -> Result<(), RewardCatalogError> {
    if actions.len() > REWARD_MAX_LEGAL_ACTIONS {
        return Err(RewardCatalogError::InvalidInput("legal_actions"));
    }
    let mut ids = BTreeSet::new();
    for action in actions {
        validate_action_kind(&action.action_kind)?;
        validate_text_value(&action.label, "action_label")?;
        if let RewardField::Available(condition) = &action.condition {
            validate_identity(condition, "action_condition")?;
        }
        validate_visibility(action.visibility)?;
        if !ids.insert(&action.action_kind) {
            return Err(RewardCatalogError::InvalidInput("duplicate_action"));
        }
    }
    Ok(())
}

pub(super) fn validate_modifier_kind(kind: &RewardModifierKind) -> Result<(), RewardCatalogError> {
    if let RewardModifierKind::Custom(value) | RewardModifierKind::Unsupported(value) = kind {
        validate_identity(value, "modifier_kind")?;
    }
    Ok(())
}

pub(super) fn validate_modifier(modifier: &RewardModifier) -> Result<(), RewardCatalogError> {
    validate_identity(&modifier.modifier_id, "modifier_id")?;
    validate_modifier_kind(&modifier.kind)?;
    validate_text_value(&modifier.label, "modifier_label")?;
    validate_quantity(
        &modifier.amount,
        "modifier_amount",
        false,
        modifier_is_magnitude(&modifier.kind),
    )?;
    if let RewardField::Available(condition) = &modifier.condition {
        validate_identity(condition, "modifier_condition")?;
    }
    if let RewardField::Available(rule) = &modifier.rule_reference {
        validate_identity(rule, "modifier_rule")?;
    }
    validate_references(&modifier.references)?;
    validate_visibility(modifier.visibility)
}

pub(super) fn validate_modifiers(modifiers: &[RewardModifier]) -> Result<(), RewardCatalogError> {
    if modifiers.len() > REWARD_MAX_MODIFIERS {
        return Err(RewardCatalogError::InvalidInput("modifiers"));
    }
    let mut ids = BTreeSet::new();
    for modifier in modifiers {
        validate_modifier(modifier)?;
        if !ids.insert(modifier.modifier_id.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_modifier"));
        }
    }
    Ok(())
}

pub(super) fn validate_rarity_weights(
    weights: &[RewardRarityWeight],
) -> Result<(), RewardCatalogError> {
    if weights.len() > REWARD_MAX_RARITY_WEIGHTS {
        return Err(RewardCatalogError::InvalidInput("rarity_weights"));
    }
    let mut rarities = BTreeSet::new();
    for weight in weights {
        validate_identity(&weight.rarity, "rarity")?;
        validate_quantity(&weight.weight, "rarity_weight", false, true)?;
        validate_probability(&weight.probability)?;
        if !rarities.insert(weight.rarity.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_rarity"));
        }
    }
    Ok(())
}

pub(super) fn validate_generation_rule(
    rule: &RewardGenerationRule,
) -> Result<(), RewardCatalogError> {
    validate_identity(&rule.rule_id, "rule_id")?;
    validate_text_value(&rule.label, "rule_label")?;
    if let RewardField::Available(pool) = &rule.pool {
        validate_references(pool)?;
    }
    validate_rarity_weights(&rule.rarity_weights)?;
    validate_requirement_list(&rule.eligibility)?;
    validate_modifiers(&rule.modifiers)?;
    validate_probability(&rule.probability)?;
    validate_references(&rule.references)?;
    validate_visibility(rule.visibility)?;
    Ok(())
}

pub(super) fn validate_state_policy(policy: &RewardStatePolicy) -> Result<(), RewardCatalogError> {
    if let RewardField::Available(limit) = &policy.claim_limit
        && *limit == 0
    {
        return Err(RewardCatalogError::InvalidInput("claim_limit"));
    }
    if let RewardField::Available(capacity) = &policy.capacity
        && *capacity == 0
    {
        return Err(RewardCatalogError::InvalidInput("capacity"));
    }
    if let RewardField::Available(stages) = &policy.multi_stage
        && (*stages == 0 || usize::try_from(*stages).unwrap_or(usize::MAX) > REWARD_MAX_STAGES)
    {
        return Err(RewardCatalogError::InvalidInput("multi_stage"));
    }
    validate_visibility(policy.visibility)
}
