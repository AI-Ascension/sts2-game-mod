// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::error::CardDefinitionInputError;
use super::model::{
    CardCost, CardEffectParameter, CardEffectValue, CardFormula, CardNumericValue,
    CardOptionalText, CardStructuralModifier, CardTargeting, CardTextValue, CardUnavailableReason,
    validate_identity_value, validate_text_value,
};
use super::rules::{
    CardAcquisitionRule, CardUnlockRule, validate_acquisition_kind, validate_condition,
    validate_rule_provenance,
};
use super::variants::{CardDefinitionInput, CardUpgradePath, CardVariantInput, CardVariantKind};

pub(super) fn validate_input(input: &CardDefinitionInput) -> Result<(), CardDefinitionInputError> {
    validate_identity_value(&input.namespaced_id, "namespaced_id")?;
    validate_optional_text(&input.character_or_pool)?;
    validate_variants(&input.variants)?;
    validate_upgrade_paths(&input.variants, &input.upgrade_paths)?;
    if input.upgrade_paths.len() > super::CARD_DEFINITION_MAX_UPGRADE_PATHS {
        return Err(CardDefinitionInputError::CollectionTooLarge(
            "upgrade_paths",
        ));
    }
    if input.acquisition.len() > super::CARD_DEFINITION_MAX_ACQUISITION_RULES {
        return Err(CardDefinitionInputError::CollectionTooLarge("acquisition"));
    }
    for rule in &input.acquisition {
        validate_acquisition_rule(rule)?;
    }
    if let Some(unlock) = &input.unlock {
        validate_unlock_rule(unlock)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &CardFormula) -> Result<(), CardDefinitionInputError> {
    validate_identity_value(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > super::CARD_DEFINITION_MAX_FORMULA_INPUTS {
        return Err(CardDefinitionInputError::CollectionTooLarge(
            "formula_inputs",
        ));
    }
    let mut seen = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity_value(input, "formula_input")?;
        if !seen.insert(input) {
            return Err(CardDefinitionInputError::DuplicateIdentity("formula_input"));
        }
    }
    Ok(())
}

fn validate_variants(variants: &[CardVariantInput]) -> Result<(), CardDefinitionInputError> {
    if variants.is_empty() {
        return Err(CardDefinitionInputError::InvalidBaseVariant);
    }
    if variants.len() > super::CARD_DEFINITION_MAX_VARIANTS {
        return Err(CardDefinitionInputError::CollectionTooLarge("variants"));
    }
    let mut seen = BTreeSet::new();
    let mut base_count = 0;
    for variant in variants {
        validate_identity_value(&variant.variant_id, "variant_id")?;
        if !seen.insert(variant.variant_id.as_str()) {
            return Err(CardDefinitionInputError::DuplicateIdentity("variant_id"));
        }
        match (variant.kind, variant.upgrade_level) {
            (CardVariantKind::Base, None) => base_count += 1,
            (CardVariantKind::Base, Some(_)) => {
                return Err(CardDefinitionInputError::InvalidUpgradeVariant);
            }
            (CardVariantKind::Upgrade, Some(level)) if level > 0 => {}
            (CardVariantKind::Upgrade, _) => {
                return Err(CardDefinitionInputError::InvalidUpgradeVariant);
            }
            (_, Some(_)) => return Err(CardDefinitionInputError::InvalidUpgradeVariant),
            (_, None) => {}
        }
        validate_variant(variant)?;
    }
    if base_count != 1 {
        return Err(CardDefinitionInputError::InvalidBaseVariant);
    }
    Ok(())
}

fn validate_variant(variant: &CardVariantInput) -> Result<(), CardDefinitionInputError> {
    validate_text(&variant.title)?;
    validate_text(&variant.description)?;
    validate_cost(&variant.cost)?;
    validate_targeting(&variant.targeting)?;
    if variant.keywords.len() > super::CARD_DEFINITION_MAX_KEYWORDS {
        return Err(CardDefinitionInputError::CollectionTooLarge("keywords"));
    }
    let mut keywords = BTreeSet::new();
    for keyword in &variant.keywords {
        validate_identity_value(keyword, "keyword")?;
        if !keywords.insert(keyword) {
            return Err(CardDefinitionInputError::DuplicateIdentity("keyword"));
        }
    }
    if variant.effects.len() > super::CARD_DEFINITION_MAX_EFFECTS {
        return Err(CardDefinitionInputError::CollectionTooLarge("effects"));
    }
    let mut effects = BTreeSet::new();
    for effect in &variant.effects {
        validate_effect(effect)?;
        if !effects.insert(effect.key.as_str()) {
            return Err(CardDefinitionInputError::DuplicateIdentity("effect_key"));
        }
    }
    if variant.structural_modifiers.len() > super::CARD_DEFINITION_MAX_MODIFIERS {
        return Err(CardDefinitionInputError::CollectionTooLarge(
            "structural_modifiers",
        ));
    }
    let mut modifiers = BTreeSet::new();
    for modifier in &variant.structural_modifiers {
        validate_modifier(modifier)?;
        if !modifiers.insert(modifier.key.as_str()) {
            return Err(CardDefinitionInputError::DuplicateIdentity("modifier_key"));
        }
    }
    Ok(())
}

fn validate_upgrade_paths(
    variants: &[CardVariantInput],
    paths: &[CardUpgradePath],
) -> Result<(), CardDefinitionInputError> {
    let variant_kinds = variants
        .iter()
        .map(|variant| {
            (
                variant.variant_id.as_str(),
                (variant.kind, variant.upgrade_level),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut path_ids = BTreeSet::new();
    let mut path_variants = BTreeSet::new();
    for path in paths {
        validate_identity_value(&path.path_id, "upgrade_path")?;
        if !path_ids.insert(path.path_id.as_str()) {
            return Err(CardDefinitionInputError::DuplicateIdentity("upgrade_path"));
        }
        if path.variant_ids.is_empty()
            || path.variant_ids.len() > super::CARD_DEFINITION_MAX_UPGRADE_LEVELS
        {
            return Err(CardDefinitionInputError::InvalidUpgradePath);
        }
        let mut previous_level = None;
        for variant_id in &path.variant_ids {
            validate_identity_value(variant_id, "upgrade_variant")?;
            let Some((kind, Some(level))) = variant_kinds.get(variant_id.as_str()) else {
                return Err(CardDefinitionInputError::InvalidUpgradePath);
            };
            if *kind != CardVariantKind::Upgrade || !path_variants.insert(variant_id.as_str()) {
                return Err(CardDefinitionInputError::InvalidUpgradePath);
            }
            if previous_level.is_some_and(|previous| *level <= previous) {
                return Err(CardDefinitionInputError::NonIncreasingUpgradeLevel);
            }
            previous_level = Some(*level);
        }
    }
    for (variant_id, (kind, _)) in variant_kinds {
        if kind == CardVariantKind::Upgrade && !path_variants.contains(variant_id) {
            return Err(CardDefinitionInputError::InvalidUpgradePath);
        }
    }
    Ok(())
}

fn validate_optional_text(value: &CardOptionalText) -> Result<(), CardDefinitionInputError> {
    if let CardOptionalText::Available(value) = value {
        validate_identity_value(value, "character_or_pool")?;
    }
    Ok(())
}

fn validate_text(value: &CardTextValue) -> Result<(), CardDefinitionInputError> {
    if let CardTextValue::Available(value) = value {
        validate_text_value(value)?;
    }
    Ok(())
}

fn validate_cost(cost: &CardCost) -> Result<(), CardDefinitionInputError> {
    validate_numeric(&cost.energy)?;
    if cost.additional.len() > super::CARD_DEFINITION_MAX_MODIFIERS {
        return Err(CardDefinitionInputError::CollectionTooLarge("costs"));
    }
    let mut keys = BTreeSet::new();
    for component in &cost.additional {
        validate_identity_value(&component.key, "cost_key")?;
        if !keys.insert(component.key.as_str()) {
            return Err(CardDefinitionInputError::DuplicateIdentity("cost_key"));
        }
        validate_numeric(&component.amount)?;
        if let super::model::CardCostUnit::Custom(unit) = &component.unit {
            validate_identity_value(unit, "cost_unit")?;
        }
    }
    Ok(())
}

fn validate_numeric(value: &CardNumericValue) -> Result<(), CardDefinitionInputError> {
    if let CardNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

fn validate_targeting(value: &CardTargeting) -> Result<(), CardDefinitionInputError> {
    match value {
        CardTargeting::Dynamic(formula) => validate_formula(formula),
        CardTargeting::Custom(value) => validate_identity_value(value, "targeting"),
        _ => Ok(()),
    }
}

fn validate_effect(effect: &CardEffectParameter) -> Result<(), CardDefinitionInputError> {
    validate_identity_value(&effect.key, "effect_key")?;
    validate_effect_value(&effect.value)
}

fn validate_effect_value(value: &CardEffectValue) -> Result<(), CardDefinitionInputError> {
    match value {
        CardEffectValue::Number(number) => validate_numeric(number),
        CardEffectValue::Text(text) => validate_text_value(text),
        CardEffectValue::Formula(formula) => validate_formula(formula),
        CardEffectValue::Boolean(_)
        | CardEffectValue::Unavailable(CardUnavailableReason::NotObserved)
        | CardEffectValue::Unavailable(CardUnavailableReason::Unsupported)
        | CardEffectValue::Unavailable(CardUnavailableReason::Denied)
        | CardEffectValue::Unavailable(CardUnavailableReason::Failed)
        | CardEffectValue::Unavailable(CardUnavailableReason::Unknown)
        | CardEffectValue::Unavailable(CardUnavailableReason::FormulaNotObserved) => Ok(()),
    }
}

fn validate_modifier(modifier: &CardStructuralModifier) -> Result<(), CardDefinitionInputError> {
    validate_identity_value(&modifier.key, "modifier_key")?;
    if let Some(value) = &modifier.value {
        validate_effect_value(value)?;
    }
    Ok(())
}

fn validate_acquisition_rule(rule: &CardAcquisitionRule) -> Result<(), CardDefinitionInputError> {
    validate_acquisition_kind(&rule.kind)?;
    validate_condition(&rule.condition)?;
    validate_rule_provenance(&rule.provenance)
}

fn validate_unlock_rule(rule: &CardUnlockRule) -> Result<(), CardDefinitionInputError> {
    validate_condition(&rule.condition)?;
    validate_rule_provenance(&rule.provenance)
}
