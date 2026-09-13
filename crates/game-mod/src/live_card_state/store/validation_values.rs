// SPDX-License-Identifier: MIT

use crate::live_card_state::identity::validate_identity;
use crate::live_card_state::{
    CardEffectValue, CardPile, LIVE_CARD_MAX_EFFECT_LIST_ITEMS, LIVE_CARD_MAX_ID_BYTES,
    LIVE_CARD_MAX_TEXT_BYTES, LiveCardCollection, LiveCardCollectionInventory, LiveCardError,
};

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
