// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::RewardCatalogError;
use super::super::definition::{RewardItemInput, RewardKind, RewardModifierKind};
use super::super::model::{
    REWARD_MAX_FORMULA_INPUTS, RewardField, RewardFormula, RewardNumericValue, RewardProbability,
    RewardQuantity, validate_identity,
};

pub(super) fn validate_numeric(value: &RewardNumericValue) -> Result<(), RewardCatalogError> {
    if let RewardNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &RewardFormula) -> Result<(), RewardCatalogError> {
    validate_identity(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > REWARD_MAX_FORMULA_INPUTS {
        return Err(RewardCatalogError::InvalidInput("formula_inputs"));
    }
    let mut ids = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input")?;
        if !ids.insert(input.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_formula_input"));
        }
    }
    Ok(())
}

pub(super) fn validate_probability(
    probability: &RewardProbability,
) -> Result<(), RewardCatalogError> {
    match probability {
        RewardProbability::Exact {
            numerator,
            denominator,
            ..
        } => {
            if *denominator == 0 || numerator > denominator {
                return Err(RewardCatalogError::InvalidInput("probability"));
            }
        }
        RewardProbability::Rule { rule_reference, .. } => {
            validate_identity(rule_reference, "probability_rule")?;
        }
        RewardProbability::Conditional {
            rule_reference,
            condition_reference,
            ..
        } => {
            validate_identity(rule_reference, "probability_rule")?;
            validate_identity(condition_reference, "probability_condition")?;
        }
        RewardProbability::Unavailable(_) => {}
    }
    Ok(())
}

/// Validates one typed quantity.
///
/// `require_unit` forces an observed currency unit; `magnitude` rejects a negative fixed amount
/// for a category whose kind already names the direction. A fixed quantity whose visible amount
/// contradicts its explicit `modified` witness is rejected rather than silently trusted.
pub(super) fn validate_quantity(
    quantity: &RewardQuantity,
    field_name: &'static str,
    require_unit: bool,
    magnitude: bool,
) -> Result<(), RewardCatalogError> {
    match &quantity.unit {
        RewardField::Available(unit) => {
            validate_identity(unit, "quantity_unit")?;
        }
        RewardField::Unavailable(_) if require_unit => {
            return Err(RewardCatalogError::InvalidInput("quantity_unit"));
        }
        RewardField::Unavailable(_) => {}
    }
    for value in [&quantity.base_amount, &quantity.visible_amount] {
        validate_numeric(value)?;
        if magnitude
            && let RewardNumericValue::Fixed(amount) = value
            && *amount < 0
        {
            return Err(RewardCatalogError::InvalidInput(field_name));
        }
    }
    if let (
        RewardNumericValue::Fixed(base),
        RewardNumericValue::Fixed(visible),
        RewardField::Available(modified),
    ) = (
        &quantity.base_amount,
        &quantity.visible_amount,
        &quantity.modified,
    ) && *modified != (base != visible)
    {
        return Err(RewardCatalogError::InvalidInput("quantity_modified"));
    }
    Ok(())
}

pub(super) fn modifier_is_magnitude(kind: &RewardModifierKind) -> bool {
    matches!(
        kind,
        RewardModifierKind::AddItem
            | RewardModifierKind::RemoveItem
            | RewardModifierKind::IncreaseQuantity
            | RewardModifierKind::DecreaseQuantity
            | RewardModifierKind::Reroll
            | RewardModifierKind::RarityUpgrade
            | RewardModifierKind::RarityDowngrade
    )
}

/// Bounds and validates one offered item's quantity against its reward kind.
pub(super) fn validate_item_quantity(
    item: &RewardItemInput,
    reward_kind: &RewardKind,
) -> Result<(), RewardCatalogError> {
    let (require_unit, magnitude) = match reward_kind {
        RewardKind::Currency => (true, true),
        RewardKind::Card | RewardKind::Relic | RewardKind::Potion | RewardKind::SpecialGrant => {
            (false, true)
        }
        RewardKind::Custom(_) | RewardKind::Unsupported(_) | RewardKind::Unknown => (false, false),
    };
    validate_quantity(&item.quantity, "item_quantity", require_unit, magnitude)
}
