// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::super::model::{validate_identity, validate_text};
use super::super::super::{
    RewardActionKind, RewardField, RewardFieldStatus, RewardKind, RewardNumericValue,
};
use super::super::RewardLiveError;

pub(super) fn validate_collection<T>(
    collection: &super::super::snapshot::RewardLiveCollection<T>,
    field: &'static str,
) -> Result<(), RewardLiveError> {
    if !matches!(
        collection.status,
        RewardFieldStatus::Available | RewardFieldStatus::Partial
    ) && !collection.entries.is_empty()
    {
        return Err(RewardLiveError::InvalidInput(field));
    }
    Ok(())
}

pub(super) fn validate_kind(kind: &RewardKind) -> Result<(), RewardLiveError> {
    if let RewardKind::Custom(value) | RewardKind::Unsupported(value) = kind {
        validate_identity(value, "reward_kind").map_err(RewardLiveError::Catalog)?;
    }
    Ok(())
}

pub(super) fn validate_action_kind(kind: &RewardActionKind) -> Result<(), RewardLiveError> {
    if let RewardActionKind::Custom(value) | RewardActionKind::Unsupported(value) = kind {
        validate_identity(value, "action_kind").map_err(RewardLiveError::Catalog)?;
    }
    Ok(())
}

pub(super) fn validate_text_field(
    text: &super::super::super::RewardText,
    field: &'static str,
) -> Result<(), RewardLiveError> {
    if let super::super::super::RewardText::Available(value) = text {
        validate_text(value, field).map_err(RewardLiveError::Catalog)?;
    }
    Ok(())
}

pub(super) fn validate_semantic_reference(
    reference: &super::super::super::RewardSemanticReference,
) -> Result<(), RewardLiveError> {
    validate_identity(&reference.id, "content_reference").map_err(RewardLiveError::Catalog)?;
    validate_text_field(&reference.label, "content_label")?;
    if let super::super::super::RewardSemanticReferenceKind::Content { entity_kind } =
        &reference.kind
    {
        validate_identity(entity_kind, "content_family").map_err(RewardLiveError::Catalog)?;
    }
    Ok(())
}

pub(super) fn validate_quantity(
    quantity: &super::super::super::RewardQuantity,
    kind: &RewardKind,
) -> Result<(), RewardLiveError> {
    let (unit_required, magnitude) = match kind {
        RewardKind::Currency => (true, true),
        RewardKind::Card | RewardKind::Relic | RewardKind::Potion | RewardKind::SpecialGrant => {
            (false, true)
        }
        RewardKind::Custom(_) | RewardKind::Unsupported(_) | RewardKind::Unknown => (false, false),
    };
    if let RewardField::Available(unit) = &quantity.unit {
        validate_identity(unit, "quantity_unit").map_err(RewardLiveError::Catalog)?;
    } else if unit_required {
        return Err(RewardLiveError::InvalidInput("quantity_unit"));
    }
    for value in [&quantity.base_amount, &quantity.visible_amount] {
        match value {
            RewardNumericValue::Fixed(amount) if magnitude && *amount < 0 => {
                return Err(RewardLiveError::InvalidInput("quantity_magnitude"));
            }
            RewardNumericValue::Formula(formula) => validate_formula(formula)?,
            RewardNumericValue::Fixed(_) | RewardNumericValue::Unavailable(_) => {}
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
        return Err(RewardLiveError::InvalidInput("quantity_modified"));
    }
    Ok(())
}

fn validate_formula(formula: &super::super::super::RewardFormula) -> Result<(), RewardLiveError> {
    validate_identity(&formula.rule_reference, "quantity_formula")
        .map_err(RewardLiveError::Catalog)?;
    if formula.unresolved_inputs.len() > super::super::super::REWARD_MAX_FORMULA_INPUTS {
        return Err(RewardLiveError::InvalidInput("formula_inputs"));
    }
    let mut inputs = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input").map_err(RewardLiveError::Catalog)?;
        if !inputs.insert(input.as_str()) {
            return Err(RewardLiveError::InvalidInput("duplicate_formula_input"));
        }
    }
    Ok(())
}
