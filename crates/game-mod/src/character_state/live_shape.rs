// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    definition::{CharacterResourceDefinition, CharacterResourceValue},
    error::CharacterStateLiveError,
    live_model::{
        CharacterResourceInput, CharacterResourceSlot, CharacterResourceSlotContent,
        SecondaryEntityControllerReference, SecondaryEntityInput, SecondaryEntityIntent,
        SecondaryEntityIntentKind, SecondaryEntityStatus, SecondaryEntityTarget,
    },
    model::{CharacterStateField, CharacterStateVisibilityScope},
    validation::{
        validate_intent_count, validate_owner, validate_resource_value, validate_slot_count,
        validate_status_count, visibility_allowed,
    },
};

pub(super) fn validate_resource_shape_public(
    resource: &CharacterResourceInput,
) -> Result<(), CharacterStateLiveError> {
    super::model::validate_identity(&resource.instance_id, "resource_instance_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    super::model::validate_identity(&resource.definition_id, "resource_definition_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    validate_owner(&resource.owner)?;
    validate_optional_resource_value(&resource.current)?;
    validate_optional_resource_value(&resource.maximum)?;
    validate_slots_shape(&resource.slots)?;
    Ok(())
}

pub(super) fn validate_entity_shape(
    entity: &SecondaryEntityInput,
) -> Result<(), CharacterStateLiveError> {
    super::model::validate_identity(&entity.instance_id, "secondary_instance_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    super::model::validate_identity(&entity.definition_id, "secondary_definition_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    validate_owner(&entity.owner)?;
    validate_controller(&entity.controller)?;
    validate_nonnegative_field(&entity.hp, "hp")?;
    validate_nonnegative_field(&entity.maximum_hp, "maximum_hp")?;
    validate_nonnegative_field(&entity.block, "block")?;
    validate_statuses_shape(&entity.statuses)?;
    validate_intent_shape(&entity.intent)?;
    Ok(())
}

fn validate_optional_resource_value(
    field: &CharacterStateField<CharacterResourceValue>,
) -> Result<(), CharacterStateLiveError> {
    if let CharacterStateField::Available(value) = field {
        validate_resource_value(value)?;
    }
    Ok(())
}

pub(super) fn validate_slots(
    resource: &CharacterResourceInput,
    definition: &CharacterResourceDefinition,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    let CharacterStateField::Available(slots) = &resource.slots else {
        return Ok(());
    };
    if !visibility_allowed(definition.slots_visibility, scope) {
        return Err(CharacterStateLiveError::VisibilityDenied("resource_slots"));
    }
    validate_slot_count(slots.len())?;
    validate_slots_shape(&resource.slots)?;
    Ok(())
}

fn validate_slots_shape(
    field: &CharacterStateField<Vec<CharacterResourceSlot>>,
) -> Result<(), CharacterStateLiveError> {
    let Some(slots) = field.value() else {
        return Ok(());
    };
    validate_slot_count(slots.len())?;
    let mut previous = None;
    for slot in slots {
        super::model::validate_identity(&slot.slot_id, "slot_id")
            .map_err(CharacterStateLiveError::InvalidInput)?;
        if previous.is_some_and(|position| position >= slot.position) {
            return Err(CharacterStateLiveError::InvalidSlotOrder);
        }
        previous = Some(slot.position);
        if let CharacterStateField::Available(content) = &slot.content {
            validate_slot_content(content)?;
        }
    }
    Ok(())
}

fn validate_slot_content(
    content: &CharacterResourceSlotContent,
) -> Result<(), CharacterStateLiveError> {
    match content {
        CharacterResourceSlotContent::Empty => Ok(()),
        CharacterResourceSlotContent::Definition(value)
        | CharacterResourceSlotContent::Instance(value) => {
            super::model::validate_identity(value, "slot_reference")
                .map_err(CharacterStateLiveError::InvalidInput)
        }
        CharacterResourceSlotContent::Text(value) => {
            super::model::validate_text(value, "slot_text")
                .map_err(CharacterStateLiveError::InvalidInput)
        }
        CharacterResourceSlotContent::Custom { kind, value } => {
            super::model::validate_identity(kind, "slot_kind")
                .map_err(CharacterStateLiveError::InvalidInput)?;
            super::model::validate_text(value, "slot_value")
                .map_err(CharacterStateLiveError::InvalidInput)
        }
    }
}

pub(super) fn validate_controller(
    field: &CharacterStateField<SecondaryEntityControllerReference>,
) -> Result<(), CharacterStateLiveError> {
    let Some(controller) = field.value() else {
        return Ok(());
    };
    super::model::validate_identity(controller.owner_id.as_str(), "controller_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    super::model::validate_identity(&controller.character_id, "controller_character_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    if let super::model::CharacterStateOwnerKind::Custom(kind) = &controller.owner_kind {
        super::model::validate_identity(kind, "controller_kind")
            .map_err(CharacterStateLiveError::InvalidInput)?;
    }
    if let CharacterStateField::Available(label) = &controller.label {
        super::model::validate_text(label, "controller_label")
            .map_err(CharacterStateLiveError::InvalidInput)?;
    }
    Ok(())
}

pub(super) fn validate_nonnegative_field(
    field: &CharacterStateField<i64>,
    name: &'static str,
) -> Result<(), CharacterStateLiveError> {
    if field.value().is_some_and(|value| *value < 0) {
        return Err(CharacterStateLiveError::ValueOutOfRange(name));
    }
    Ok(())
}

fn validate_statuses_shape(
    field: &CharacterStateField<Vec<SecondaryEntityStatus>>,
) -> Result<(), CharacterStateLiveError> {
    let Some(statuses) = field.value() else {
        return Ok(());
    };
    validate_status_count(statuses.len())?;
    let mut ids = BTreeSet::new();
    for status in statuses {
        super::model::validate_identity(&status.instance_id, "status_instance_id")
            .map_err(CharacterStateLiveError::InvalidInput)?;
        super::model::validate_identity(&status.definition_id, "status_definition_id")
            .map_err(CharacterStateLiveError::InvalidInput)?;
        if !ids.insert(status.instance_id.as_str()) {
            return Err(CharacterStateLiveError::InvalidInput("status_instance_id"));
        }
        validate_nonnegative_field(&status.amount, "status_amount")?;
        if let CharacterStateField::Available(label) = &status.label {
            super::model::validate_text(label, "status_label")
                .map_err(CharacterStateLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

pub(super) fn validate_statuses(
    field: &CharacterStateField<Vec<SecondaryEntityStatus>>,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    validate_statuses_shape(field)?;
    if let Some(statuses) = field.value() {
        for status in statuses {
            if !visibility_allowed(status.visibility, scope) {
                return Err(CharacterStateLiveError::VisibilityDenied(
                    "secondary_status",
                ));
            }
        }
    }
    Ok(())
}

fn validate_intent_shape(
    field: &CharacterStateField<SecondaryEntityIntent>,
) -> Result<(), CharacterStateLiveError> {
    let Some(intent) = field.value() else {
        return Ok(());
    };
    validate_intent_count(1)?;
    if let SecondaryEntityIntentKind::Custom(kind) = &intent.kind {
        super::model::validate_identity(kind, "intent_kind")
            .map_err(CharacterStateLiveError::InvalidInput)?;
    }
    if let CharacterStateField::Available(rule) = &intent.rule_reference {
        super::model::validate_identity(rule, "intent_rule_reference")
            .map_err(CharacterStateLiveError::InvalidInput)?;
    }
    if let CharacterStateField::Available(target) = &intent.target {
        match target {
            SecondaryEntityTarget::Owner(id) | SecondaryEntityTarget::Entity(id) => {
                super::model::validate_identity(id, "intent_target")
                    .map_err(CharacterStateLiveError::InvalidInput)?;
            }
            SecondaryEntityTarget::All => {}
            SecondaryEntityTarget::Custom(value) => {
                super::model::validate_identity(value, "intent_target")
                    .map_err(CharacterStateLiveError::InvalidInput)?;
            }
        }
    }
    validate_nonnegative_field(&intent.amount, "intent_amount")?;
    if let CharacterStateField::Available(label) = &intent.label {
        super::model::validate_text(label, "intent_label")
            .map_err(CharacterStateLiveError::InvalidInput)?;
    }
    Ok(())
}

pub(super) fn validate_intent(
    field: &CharacterStateField<SecondaryEntityIntent>,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    validate_intent_shape(field)?;
    if let Some(intent) = field.value()
        && !visibility_allowed(intent.visibility, scope)
    {
        return Err(CharacterStateLiveError::VisibilityDenied(
            "secondary_intent",
        ));
    }
    Ok(())
}
