// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    CardCost, CardCostAmount, CardCostContributor, CardExpiration, CardLocation, CardModifier,
    CardModifierValue, CardPile, LIVE_CARD_MAX_COST_CONTRIBUTORS, LIVE_CARD_MAX_EFFECT_OVERRIDES,
    LIVE_CARD_MAX_FLAGS, LIVE_CARD_MAX_MODIFIERS, LiveCardCollection, LiveCardError, LiveCardField,
    LiveCardFixture, LiveCardProjection, LiveCardSnapshot, LiveCardValue,
};
use crate::live_card_state::LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES;
#[path = "validation_values.rs"]
mod validation_values;
use validation_values::{
    measure_projection_bytes, validate_collection_inventory, validate_effect_value,
    validate_identity_field, validate_local_key, validate_reference, validate_text,
};

pub(super) fn collection_matches(collection: &LiveCardCollection, location: &CardLocation) -> bool {
    match (collection, location) {
        (LiveCardCollection::AllVisible, _) => true,
        (LiveCardCollection::Pile(expected), CardLocation::Pile { pile, .. }) => expected == pile,
        (LiveCardCollection::Selector(expected), CardLocation::Selector { selector_id, .. }) => {
            expected == selector_id
        }
        _ => false,
    }
}

pub(super) fn validate_snapshot(snapshot: &LiveCardSnapshot) -> Result<(), LiveCardError> {
    validate_reference(&snapshot.reference)?;
    validate_collection_inventory(&snapshot.collection_inventory)?;
    let mut instance_ids = BTreeSet::new();
    for fixture in &snapshot.cards {
        let projection = &fixture.projection;
        if projection.instance.read != snapshot.reference {
            return Err(LiveCardError::StaleReference);
        }
        if projection.definition.manifest != snapshot.reference.content_manifest {
            return Err(LiveCardError::InvalidProjection(
                "definition manifest does not match read fence",
            ));
        }
        if !instance_ids.insert(projection.instance.instance_id.clone()) {
            return Err(LiveCardError::InvalidProjection(
                "duplicate live card instance identity",
            ));
        }
        if fixture.estimated_bytes == 0 {
            return Err(LiveCardError::InvalidProjection(
                "detail size estimate is unavailable",
            ));
        }
        validate_projection(projection)?;
    }
    Ok(())
}

pub(super) fn validate_fixture_for_read(
    fixture: &LiveCardFixture,
    max_detail_bytes: usize,
) -> Result<(), LiveCardError> {
    if fixture.estimated_bytes == 0 {
        return Err(LiveCardError::InvalidProjection(
            "detail size estimate is unavailable",
        ));
    }
    let measured_bytes = measure_projection_bytes(&fixture.projection)?;
    let actual_bytes = fixture.estimated_bytes.max(measured_bytes);
    if actual_bytes > max_detail_bytes {
        return Err(LiveCardError::DetailTooLarge {
            limit: max_detail_bytes,
            actual: actual_bytes,
        });
    }
    Ok(())
}

fn validate_projection(projection: &LiveCardProjection) -> Result<(), LiveCardError> {
    validate_identity_field(&projection.instance.instance_id, "card_instance_id")?;
    validate_identity_field(&projection.definition.manifest, "definition_manifest")?;
    validate_identity_field(&projection.definition.definition_id, "definition_id")?;
    if let Some(owner_id) = &projection.owner.owner_id {
        validate_identity_field(owner_id, "owner_id")?;
    }
    if let Some(variant) = &projection.upgrade.variant {
        validate_identity_field(variant, "upgrade_variant")?;
    }
    if let Some(path) = &projection.upgrade.path {
        validate_identity_field(path, "upgrade_path")?;
    }
    if projection.modifiers.len() > LIVE_CARD_MAX_MODIFIERS {
        return Err(LiveCardError::InvalidProjection(
            "modifier count exceeds local bound",
        ));
    }
    if projection.effect_parameter_overrides.len() > LIVE_CARD_MAX_EFFECT_OVERRIDES {
        return Err(LiveCardError::InvalidProjection(
            "effect override count exceeds local bound",
        ));
    }
    let mut effect_override_bytes = 0usize;
    for (key, value) in &projection.effect_parameter_overrides {
        validate_local_key(key, "effect_parameter")?;
        let value_bytes = validate_effect_value(value)?;
        effect_override_bytes = effect_override_bytes.checked_add(value_bytes).ok_or(
            LiveCardError::InvalidProjection("effect override byte measurement overflowed"),
        )?;
        if effect_override_bytes > LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES {
            return Err(LiveCardError::InvalidProjection(
                "effect override bytes exceed local bound",
            ));
        }
    }
    for flag in &projection.flags.other {
        validate_local_key(flag, "flag")?;
    }
    let flag_count = projection.flags.other.len()
        + if projection.flags.retained { 1 } else { 0 }
        + if projection.flags.exhaust { 1 } else { 0 }
        + if projection.flags.ethereal { 1 } else { 0 };
    if flag_count > LIVE_CARD_MAX_FLAGS {
        return Err(LiveCardError::InvalidProjection(
            "flag count exceeds local bound",
        ));
    }
    let mut previous_order = None;
    for modifier in &projection.modifiers {
        validate_modifier(modifier)?;
        if previous_order.is_some_and(|previous| modifier.order <= previous) {
            return Err(LiveCardError::InvalidProjection(
                "modifier order is not strictly increasing",
            ));
        }
        previous_order = Some(modifier.order);
    }
    validate_cost_semantics(&projection.cost)?;
    validate_location(&projection.location)?;
    Ok(())
}

fn validate_modifier(modifier: &CardModifier) -> Result<(), LiveCardError> {
    validate_local_key(&modifier.source_ref, "modifier_source")?;
    validate_expiration(&modifier.expiration)?;
    validate_modifier_value(&modifier.value)
}

fn validate_modifier_value(value: &CardModifierValue) -> Result<(), LiveCardError> {
    match value {
        CardModifierValue::Text(text) => validate_text(text, "modifier_text"),
        CardModifierValue::Cost(cost) => validate_cost(cost),
        CardModifierValue::Integer(_)
        | CardModifierValue::Boolean(_)
        | CardModifierValue::Marker
        | CardModifierValue::Unknown => Ok(()),
    }
}

fn validate_cost_semantics(cost: &super::super::CardCostSemantics) -> Result<(), LiveCardError> {
    if cost.contributors.len() > LIVE_CARD_MAX_COST_CONTRIBUTORS {
        return Err(LiveCardError::InvalidProjection(
            "cost contributor count exceeds local bound",
        ));
    }
    validate_cost(&cost.base)?;
    validate_cost(&cost.current)?;
    validate_cost(&cost.effective)?;
    let mut previous_order = None;
    for contributor in &cost.contributors {
        validate_cost_contributor(contributor)?;
        if previous_order.is_some_and(|previous| contributor.order <= previous) {
            return Err(LiveCardError::InvalidProjection(
                "cost contributor order is not strictly increasing",
            ));
        }
        previous_order = Some(contributor.order);
    }
    Ok(())
}

fn validate_cost_contributor(contributor: &CardCostContributor) -> Result<(), LiveCardError> {
    validate_local_key(&contributor.source_ref, "cost_source")?;
    validate_expiration(&contributor.expiration)?;
    validate_cost(&contributor.value)
}

fn validate_cost(cost: &CardCost) -> Result<(), LiveCardError> {
    match cost {
        CardCost::AlternateResource { resource, amount } => {
            validate_local_key(resource, "alternate_resource")?;
            validate_cost_amount(amount)
        }
        CardCost::Unknown { .. }
        | CardCost::Fixed(_)
        | CardCost::X
        | CardCost::Free
        | CardCost::Unplayable => Ok(()),
    }
}

fn validate_cost_amount(amount: &CardCostAmount) -> Result<(), LiveCardError> {
    match amount {
        CardCostAmount::Unknown { .. } | CardCostAmount::Fixed(_) | CardCostAmount::X => Ok(()),
    }
}

fn validate_expiration(expiration: &CardExpiration) -> Result<(), LiveCardError> {
    if let CardExpiration::Condition(value) = expiration {
        validate_local_key(value, "expiration")?;
    }
    Ok(())
}

fn validate_location(location: &CardLocation) -> Result<(), LiveCardError> {
    match location {
        CardLocation::Selector { selector_id, .. } => {
            validate_local_key(selector_id, "selector_id")
        }
        CardLocation::Pile {
            pile: CardPile::Other(value),
            ..
        } => validate_local_key(value, "pile"),
        CardLocation::Pile { .. } | CardLocation::Unavailable { .. } => Ok(()),
    }
}

pub(super) fn validate_collection(collection: &LiveCardCollection) -> Result<(), LiveCardError> {
    match collection {
        LiveCardCollection::Selector(value) => {
            if value.is_empty() {
                Err(LiveCardError::UnknownCollection)
            } else {
                validate_local_key(value, "selector_id")
            }
        }
        LiveCardCollection::Pile(CardPile::Other(value)) => {
            if value.is_empty() {
                Err(LiveCardError::UnknownCollection)
            } else {
                validate_local_key(value, "pile")
            }
        }
        LiveCardCollection::AllVisible | LiveCardCollection::Pile(_) => Ok(()),
    }
}

pub(super) fn value_for_field(
    projection: &LiveCardProjection,
    field: LiveCardField,
) -> LiveCardValue {
    match field {
        LiveCardField::Definition => LiveCardValue::Definition(projection.definition.clone()),
        LiveCardField::Owner => LiveCardValue::Owner(projection.owner.clone()),
        LiveCardField::Location => LiveCardValue::Location(projection.location.clone()),
        LiveCardField::Upgrade => LiveCardValue::Upgrade(projection.upgrade.clone()),
        LiveCardField::Modifiers => LiveCardValue::Modifiers(projection.modifiers.clone()),
        LiveCardField::Flags => LiveCardValue::Flags(projection.flags.clone()),
        LiveCardField::EffectParameters => {
            LiveCardValue::EffectParameters(projection.effect_parameter_overrides.clone())
        }
        LiveCardField::Cost => LiveCardValue::Cost(projection.cost.clone()),
    }
}
