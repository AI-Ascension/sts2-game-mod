// SPDX-License-Identifier: MIT

use crate::live_card_state::identity::validate_identity;
use crate::live_card_state::{
    CardCost, CardCostAmount, CardCostSemantics, CardEffectValue, CardExpiration, CardLocation,
    CardModifierValue, CardPile, LIVE_CARD_MAX_EFFECT_LIST_ITEMS, LIVE_CARD_MAX_ID_BYTES,
    LIVE_CARD_MAX_TEXT_BYTES, LiveCardCollection, LiveCardCollectionInventory, LiveCardError,
    LiveCardProjection,
};

/// Measures the complete projection conservatively without trusting a source estimate.
pub(super) fn measure_projection_bytes(
    projection: &LiveCardProjection,
) -> Result<usize, LiveCardError> {
    let mut total = 64usize;
    for value in [
        projection.instance.read.instance_id.as_str(),
        projection.instance.read.run_id.as_str(),
        projection.instance.read.content_manifest.as_str(),
        projection.instance.read.snapshot.as_str(),
        projection.instance.instance_id.as_str(),
        projection.definition.manifest.as_str(),
        projection.definition.definition_id.as_str(),
    ] {
        add_bytes(&mut total, value.len())?;
    }
    if let Some(owner_id) = projection.owner.owner_id.as_deref() {
        add_bytes(&mut total, owner_id.len())?;
    }
    match &projection.location {
        CardLocation::Pile {
            pile: CardPile::Other(value),
            ..
        } => add_bytes(&mut total, value.len())?,
        CardLocation::Selector { selector_id, .. } => add_bytes(&mut total, selector_id.len())?,
        CardLocation::Pile { .. } | CardLocation::Unavailable { .. } => {}
    }
    for value in [&projection.upgrade.variant, &projection.upgrade.path]
        .into_iter()
        .flatten()
    {
        add_bytes(&mut total, value.len())?;
    }
    for modifier in &projection.modifiers {
        add_bytes(&mut total, 16)?;
        add_bytes(&mut total, modifier.source_ref.len())?;
        if modifier.amount.is_some() {
            add_bytes(&mut total, std::mem::size_of::<i64>())?;
        }
        measure_modifier_value(&mut total, &modifier.value)?;
        measure_expiration(&mut total, &modifier.expiration)?;
    }
    for flag in &projection.flags.other {
        add_bytes(&mut total, flag.len())?;
    }
    for (key, value) in &projection.effect_parameter_overrides {
        add_bytes(&mut total, key.len())?;
        measure_effect_value(&mut total, value)?;
    }
    measure_cost_semantics(&mut total, &projection.cost)?;
    Ok(total)
}

fn measure_cost_semantics(
    total: &mut usize,
    cost: &CardCostSemantics,
) -> Result<(), LiveCardError> {
    add_bytes(total, 8)?;
    measure_cost(total, &cost.base)?;
    measure_cost(total, &cost.current)?;
    measure_cost(total, &cost.effective)?;
    for contributor in &cost.contributors {
        add_bytes(total, 16)?;
        add_bytes(total, contributor.source_ref.len())?;
        if contributor.amount.is_some() {
            add_bytes(total, std::mem::size_of::<i64>())?;
        }
        measure_cost(total, &contributor.value)?;
        measure_expiration(total, &contributor.expiration)?;
    }
    Ok(())
}

fn measure_cost(total: &mut usize, cost: &CardCost) -> Result<(), LiveCardError> {
    add_bytes(total, 4)?;
    match cost {
        CardCost::AlternateResource { resource, amount } => {
            add_bytes(total, resource.len())?;
            measure_cost_amount(total, amount)?;
        }
        CardCost::Unknown {
            observed: Some(_), ..
        } => add_bytes(total, std::mem::size_of::<i64>())?,
        CardCost::Unknown { observed: None, .. }
        | CardCost::Fixed(_)
        | CardCost::X
        | CardCost::Free
        | CardCost::Unplayable => {}
    }
    Ok(())
}

fn measure_cost_amount(total: &mut usize, amount: &CardCostAmount) -> Result<(), LiveCardError> {
    add_bytes(total, 4)?;
    if let CardCostAmount::Unknown {
        observed: Some(_), ..
    } = amount
    {
        add_bytes(total, std::mem::size_of::<i64>())?;
    }
    Ok(())
}

fn measure_modifier_value(
    total: &mut usize,
    value: &CardModifierValue,
) -> Result<(), LiveCardError> {
    add_bytes(total, 4)?;
    match value {
        CardModifierValue::Text(text) => add_bytes(total, text.len())?,
        CardModifierValue::Cost(cost) => measure_cost(total, cost)?,
        CardModifierValue::Integer(_)
        | CardModifierValue::Boolean(_)
        | CardModifierValue::Marker
        | CardModifierValue::Unknown => {}
    }
    Ok(())
}

fn measure_effect_value(total: &mut usize, value: &CardEffectValue) -> Result<(), LiveCardError> {
    add_bytes(total, 4)?;
    match value {
        CardEffectValue::Text(text) => add_bytes(total, text.len())?,
        CardEffectValue::IntegerList(values) => {
            add_bytes(
                total,
                values.len().checked_mul(std::mem::size_of::<i64>()).ok_or(
                    LiveCardError::InvalidProjection(
                        "effect integer-list byte measurement overflowed",
                    ),
                )?,
            )?;
        }
        CardEffectValue::Integer(_) | CardEffectValue::Boolean(_) | CardEffectValue::Unknown => {}
    }
    Ok(())
}

fn measure_expiration(total: &mut usize, expiration: &CardExpiration) -> Result<(), LiveCardError> {
    add_bytes(total, 4)?;
    if let CardExpiration::Condition(value) = expiration {
        add_bytes(total, value.len())?;
    }
    Ok(())
}

fn add_bytes(total: &mut usize, value: usize) -> Result<(), LiveCardError> {
    *total = total
        .checked_add(value)
        .ok_or(LiveCardError::InvalidProjection(
            "detail size measurement overflowed",
        ))?;
    Ok(())
}

pub(super) fn validate_local_key(value: &str, field: &'static str) -> Result<(), LiveCardError> {
    if value.is_empty() || value.len() > LIVE_CARD_MAX_ID_BYTES {
        return Err(LiveCardError::InvalidProjection(field));
    }
    if value.chars().any(|character| {
        character.is_control()
            || character.is_whitespace()
            || matches!(character, '"' | '\\' | '{' | '}' | '[' | ']')
    }) {
        return Err(LiveCardError::InvalidProjection(field));
    }
    Ok(())
}

pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), LiveCardError> {
    if value.len() > LIVE_CARD_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(LiveCardError::InvalidProjection(field));
    }
    Ok(())
}

pub(super) fn validate_effect_value(value: &CardEffectValue) -> Result<usize, LiveCardError> {
    match value {
        CardEffectValue::Integer(_) => Ok(std::mem::size_of::<i64>()),
        CardEffectValue::Boolean(_) => Ok(std::mem::size_of::<bool>()),
        CardEffectValue::Text(text) => {
            validate_text(text, "effect_parameter_text")?;
            Ok(text.len())
        }
        CardEffectValue::IntegerList(values) => {
            if values.len() > LIVE_CARD_MAX_EFFECT_LIST_ITEMS {
                return Err(LiveCardError::InvalidProjection(
                    "effect integer-list count exceeds local bound",
                ));
            }
            values.len().checked_mul(std::mem::size_of::<i64>()).ok_or(
                LiveCardError::InvalidProjection("effect integer-list byte measurement overflowed"),
            )
        }
        CardEffectValue::Unknown => Ok(0),
    }
}

pub(super) fn validate_reference(
    reference: &super::super::LiveCardReadReference,
) -> Result<(), LiveCardError> {
    for (field, value) in [
        ("read_instance_id", reference.instance_id.as_str()),
        ("run_id", reference.run_id.as_str()),
        ("content_manifest", reference.content_manifest.as_str()),
        ("snapshot", reference.snapshot.as_str()),
    ] {
        validate_identity_field(value, field)?;
    }
    Ok(())
}

pub(super) fn validate_identity_field(
    value: &str,
    field: &'static str,
) -> Result<(), LiveCardError> {
    validate_identity(field, value).map_err(|_| LiveCardError::InvalidProjection(field))
}

pub(super) fn validate_collection_inventory(
    inventory: &LiveCardCollectionInventory,
) -> Result<(), LiveCardError> {
    for collection in inventory.statuses.keys() {
        match collection {
            LiveCardCollection::AllVisible
            | LiveCardCollection::Pile(CardPile::Hand)
            | LiveCardCollection::Pile(CardPile::Draw)
            | LiveCardCollection::Pile(CardPile::Discard)
            | LiveCardCollection::Pile(CardPile::Exhaust)
            | LiveCardCollection::Pile(CardPile::Deck)
            | LiveCardCollection::Pile(CardPile::Reward)
            | LiveCardCollection::Pile(CardPile::Shop) => {}
            LiveCardCollection::Pile(CardPile::Other(value)) => {
                validate_local_key(value.as_str(), "pile")?;
            }
            LiveCardCollection::Selector(value) => {
                validate_local_key(value.as_str(), "selector_id")?;
            }
        }
    }
    Ok(())
}
