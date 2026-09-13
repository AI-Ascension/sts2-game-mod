// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    catalog_reader::PotionCatalog,
    definition::{PotionDefinition, PotionParameterValue},
    error::PotionLiveError,
    live_model::{
        PotionInstanceInput, PotionOfferInput, PotionOfferKind, PotionSlotState,
        PotionUsabilityReason,
    },
    live_sizes::instance_bytes,
    model::{
        POTION_MAX_ALTERNATIVES, POTION_MAX_LIVE_DETAIL_BYTES, POTION_MAX_MODIFIERS,
        POTION_MAX_OFFERS, POTION_MAX_SLOTS, POTION_MAX_TARGETS, POTION_PRODUCER_VERSION,
        PotionCollectionKind, PotionField, PotionLiveBinding, PotionVisibility,
        PotionVisibilityScope, validate_identity, validate_text,
    },
};

pub(super) fn validate_binding(binding: &PotionLiveBinding) -> Result<(), PotionLiveError> {
    if binding.catalog.producer_version != POTION_PRODUCER_VERSION {
        return Err(PotionLiveError::ProducerVersionMismatch);
    }
    for (value, field) in [
        (&binding.catalog.locale, "locale"),
        (&binding.game_instance_id, "game_instance_id"),
        (&binding.run_id, "run_id"),
        (&binding.snapshot_id, "snapshot_id"),
    ] {
        validate_identity(value, field).map_err(PotionLiveError::InvalidBinding)?;
    }
    Ok(())
}

pub(super) fn validate_instance_identity(
    instance: &PotionInstanceInput,
) -> Result<(), PotionLiveError> {
    validate_identity(&instance.instance_id, "instance_id")
        .map_err(PotionLiveError::InvalidInput)?;
    validate_identity(&instance.definition_id, "definition_id")
        .map_err(PotionLiveError::InvalidInput)?;
    validate_slot(&instance.slot)?;
    Ok(())
}

pub(super) fn validate_inventory(
    max_slots: &PotionField<u16>,
    slots: &[PotionSlotState],
) -> Result<(), PotionLiveError> {
    if slots.len() > POTION_MAX_SLOTS {
        return Err(PotionLiveError::InvalidInput("inventory_slots"));
    }
    if let Some(max_slots) = max_slots.value()
        && usize::from(*max_slots) > POTION_MAX_SLOTS
    {
        return Err(PotionLiveError::InvalidInput("inventory_max_slots"));
    }
    if let Some(max_slots) = max_slots.value()
        && usize::from(*max_slots) < slots.len()
    {
        return Err(PotionLiveError::InvalidState("inventory_max_slots"));
    }
    let mut seen = BTreeSet::new();
    for state in slots {
        let slot = match state {
            PotionSlotState::Empty { slot } | PotionSlotState::Occupied { slot, .. } => slot,
        };
        if !matches!(slot.collection, PotionCollectionKind::Inventory) {
            return Err(PotionLiveError::InvalidSlot(slot.slot_id.clone()));
        }
        validate_slot(slot)?;
        if !seen.insert(slot_key(slot)) {
            return Err(PotionLiveError::DuplicateSlot(slot.slot_id.clone()));
        }
        if let PotionSlotState::Occupied { instance_id, .. } = state {
            validate_identity(instance_id, "occupied_instance_id")
                .map_err(PotionLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

pub(super) fn validate_offers(offers: &[PotionOfferInput]) -> Result<(), PotionLiveError> {
    if offers.len() > POTION_MAX_OFFERS {
        return Err(PotionLiveError::InvalidInput("offers"));
    }
    let mut offer_ids = BTreeSet::new();
    let mut instance_ids = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for offer in offers {
        validate_identity(&offer.offer_id, "offer_id").map_err(PotionLiveError::InvalidInput)?;
        validate_identity(&offer.instance_id, "offer_instance_id")
            .map_err(PotionLiveError::InvalidInput)?;
        if !offer_ids.insert(offer.offer_id.as_str()) {
            return Err(PotionLiveError::DuplicateOffer(offer.offer_id.clone()));
        }
        if !instance_ids.insert(offer.instance_id.as_str()) {
            return Err(PotionLiveError::DuplicateOfferInstance(
                offer.instance_id.clone(),
            ));
        }
        validate_slot(&offer.slot)?;
        if !slots.insert(slot_key(&offer.slot)) {
            return Err(PotionLiveError::DuplicateSlot(offer.slot.slot_id.clone()));
        }
        match (&offer.kind, &offer.slot.collection) {
            (PotionOfferKind::Reward, PotionCollectionKind::Reward(_))
            | (PotionOfferKind::Shop, PotionCollectionKind::Shop(_)) => {}
            _ => return Err(PotionLiveError::InvalidState("offer_collection")),
        }
        validate_price(&offer.price)?;
    }
    Ok(())
}

pub(super) fn validate_against_catalog(
    catalog: &PotionCatalog,
    instance: &PotionInstanceInput,
    scope: PotionVisibilityScope,
) -> Result<(), PotionLiveError> {
    let definition = catalog
        .definition(&instance.definition_id)
        .ok_or_else(|| PotionLiveError::UnknownDefinition(instance.definition_id.clone()))?;
    validate_usability(&instance.usability)?;
    validate_modifiers(&instance.modifiers)?;
    validate_parameters(definition, &instance.effective_parameters, scope)?;
    validate_targets(&instance.permitted_targets, definition.target_mode)?;
    let detail_bytes = instance_bytes(instance);
    if detail_bytes > POTION_MAX_LIVE_DETAIL_BYTES {
        return Err(PotionLiveError::DetailTooLarge {
            limit: POTION_MAX_LIVE_DETAIL_BYTES,
            actual: detail_bytes,
        });
    }
    Ok(())
}

fn validate_slot(slot: &super::model::PotionSlotReference) -> Result<(), PotionLiveError> {
    validate_identity(&slot.slot_id, "slot_id").map_err(PotionLiveError::InvalidInput)?;
    match &slot.collection {
        PotionCollectionKind::Inventory => {}
        PotionCollectionKind::Reward(id)
        | PotionCollectionKind::Shop(id)
        | PotionCollectionKind::Other(id) => {
            validate_identity(id, "collection_id").map_err(PotionLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

fn slot_key(slot: &super::model::PotionSlotReference) -> String {
    format!("{}:{}:{}", slot.collection.code(), slot.slot_id, slot.index)
}

fn validate_price(
    price: &PotionField<super::live_model::PotionPrice>,
) -> Result<(), PotionLiveError> {
    let Some(price) = price.value() else {
        return Ok(());
    };
    validate_identity(&price.currency, "price_currency").map_err(PotionLiveError::InvalidInput)
}

fn validate_usability(
    usability: &PotionField<super::live_model::PotionUsabilityResult>,
) -> Result<(), PotionLiveError> {
    let Some(usability) = usability.value() else {
        return Ok(());
    };
    if usability.state == super::live_model::PotionUseState::Usable && usability.reason.is_some() {
        return Err(PotionLiveError::InvalidState("usable_reason"));
    }
    if let Some(PotionUsabilityReason::Condition(value)) = &usability.reason {
        validate_text(value, "usability_condition").map_err(PotionLiveError::InvalidInput)?;
    }
    Ok(())
}

fn validate_modifiers(
    modifiers: &PotionField<Vec<super::live_model::PotionModifier>>,
) -> Result<(), PotionLiveError> {
    let Some(modifiers) = modifiers.value() else {
        return Ok(());
    };
    if modifiers.len() > POTION_MAX_MODIFIERS {
        return Err(PotionLiveError::InvalidInput("modifiers"));
    }
    for modifier in modifiers {
        validate_identity(&modifier.source_ref, "modifier_source")
            .map_err(PotionLiveError::InvalidInput)?;
        match &modifier.value {
            super::live_model::PotionModifierValue::Text(value) => {
                validate_text(value, "modifier_text").map_err(PotionLiveError::InvalidInput)?;
            }
            super::live_model::PotionModifierValue::Parameter(value) => {
                validate_parameter_value(value)?;
            }
            _ => {}
        }
        if let super::live_model::PotionExpiration::Condition(value) = &modifier.expiration {
            validate_text(value, "modifier_expiration").map_err(PotionLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

fn validate_parameters(
    definition: &PotionDefinition,
    values: &PotionField<Vec<super::definition::PotionResolvedParameter>>,
    scope: PotionVisibilityScope,
) -> Result<(), PotionLiveError> {
    let Some(values) = values.value() else {
        return Ok(());
    };
    if definition.parameters.is_empty() && !values.is_empty() {
        return Err(PotionLiveError::InvalidState("parameter_not_declared"));
    }
    let mut seen = BTreeSet::new();
    for parameter in values {
        let Some(declaration) = definition
            .parameters
            .iter()
            .find(|declaration| declaration.id == parameter.id)
        else {
            return Err(PotionLiveError::UnknownParameter {
                potion_id: definition.reference.potion_id.clone(),
                parameter_id: parameter.id.clone(),
            });
        };
        if declaration.unit != parameter.unit {
            return Err(PotionLiveError::InvalidState("parameter_unit"));
        }
        if !visibility_allowed(declaration.visibility, scope) {
            return Err(PotionLiveError::InvalidState("parameter_visibility"));
        }
        if !seen.insert(parameter.id.as_str()) {
            return Err(PotionLiveError::DuplicateParameter {
                potion_id: definition.reference.potion_id.clone(),
                parameter_id: parameter.id.clone(),
            });
        }
        validate_parameter_value(&parameter.value)?;
    }
    Ok(())
}

fn validate_parameter_value(value: &PotionParameterValue) -> Result<(), PotionLiveError> {
    match value {
        PotionParameterValue::Text(value) => {
            validate_text(value, "parameter_text").map_err(PotionLiveError::InvalidInput)?;
        }
        PotionParameterValue::IntegerList(values) if values.len() > POTION_MAX_ALTERNATIVES => {
            return Err(PotionLiveError::InvalidInput("parameter_integer_list"));
        }
        _ => {}
    }
    Ok(())
}

fn validate_targets(
    targets: &PotionField<Vec<super::live_model::PotionTargetReference>>,
    target_mode: super::model::PotionTargetMode,
) -> Result<(), PotionLiveError> {
    let Some(targets) = targets.value() else {
        return Ok(());
    };
    if targets.len() > POTION_MAX_TARGETS {
        return Err(PotionLiveError::InvalidInput("permitted_targets"));
    }
    if target_mode == super::model::PotionTargetMode::None && !targets.is_empty() {
        return Err(PotionLiveError::InvalidState("targets_for_none"));
    }
    let mut seen = BTreeSet::new();
    for target in targets {
        validate_identity(&target.target_id, "target_id").map_err(PotionLiveError::InvalidInput)?;
        if !seen.insert(target.target_id.as_str()) {
            return Err(PotionLiveError::InvalidState("duplicate_target"));
        }
        if let Some(label) = &target.label {
            validate_text(label, "target_label").map_err(PotionLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

fn visibility_allowed(visibility: PotionVisibility, scope: PotionVisibilityScope) -> bool {
    match visibility {
        PotionVisibility::Visible => true,
        PotionVisibility::OwnerOnly => matches!(scope, PotionVisibilityScope::Owner),
        PotionVisibility::Hidden | PotionVisibility::Unknown => false,
    }
}
