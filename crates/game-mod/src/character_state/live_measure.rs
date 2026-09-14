// SPDX-License-Identifier: MIT

use super::{
    definition::{CharacterResourceDefinition, CharacterResourceValue, SecondaryEntityDefinition},
    live_model::{
        CharacterResourceInput, CharacterResourceSlot, CharacterResourceSlotContent,
        SecondaryEntityControllerReference, SecondaryEntityInput, SecondaryEntityIntent,
        SecondaryEntityStatus, SecondaryEntityTarget,
    },
    model::{CharacterStateLiveBinding, CharacterStateOwnerKind},
    sizes::{catalog_binding_bytes, resource_definition_bytes, secondary_definition_bytes},
    validation::field_bytes,
};

pub(super) fn resource_bytes(resource: &CharacterResourceInput) -> usize {
    resource.instance_id.len()
        + resource.definition_id.len()
        + owner_kind_bytes(&resource.owner.kind)
        + resource.owner.id.as_str().len()
        + resource.owner.character_id.len()
        + field_bytes(
            &resource.owner.label,
            resource.owner.label.value().map_or(1, String::len),
        )
        + field_bytes(
            &resource.current,
            resource_value_bytes(resource.current.value()),
        )
        + field_bytes(
            &resource.maximum,
            resource_value_bytes(resource.maximum.value()),
        )
        + field_bytes(&resource.slots, slots_bytes(resource.slots.value()))
        + field_bytes(&resource.active, 1)
        + 8
}

pub(super) fn entity_bytes(entity: &SecondaryEntityInput) -> usize {
    entity.instance_id.len()
        + entity.definition_id.len()
        + owner_kind_bytes(&entity.owner.kind)
        + entity.owner.id.as_str().len()
        + entity.owner.character_id.len()
        + field_bytes(
            &entity.owner.label,
            entity.owner.label.value().map_or(1, String::len),
        )
        + field_bytes(
            &entity.controller,
            controller_bytes(entity.controller.value()),
        )
        + field_bytes(&entity.hp, 8)
        + field_bytes(&entity.maximum_hp, 8)
        + field_bytes(&entity.block, 8)
        + field_bytes(&entity.statuses, statuses_bytes(entity.statuses.value()))
        + field_bytes(&entity.intent, intent_bytes(entity.intent.value()))
        + field_bytes(&entity.active, 1)
        + 8
}

pub(super) fn resource_detail_bytes(
    resource: &CharacterResourceInput,
    definition: &CharacterResourceDefinition,
    binding: &CharacterStateLiveBinding,
) -> usize {
    resource_bytes(resource)
        + resource_definition_bytes(definition)
        + resource_reference_bytes(binding, resource)
}

pub(super) fn entity_detail_bytes(
    entity: &SecondaryEntityInput,
    definition: &SecondaryEntityDefinition,
    binding: &CharacterStateLiveBinding,
) -> usize {
    entity_bytes(entity)
        + secondary_definition_bytes(definition)
        + secondary_reference_bytes(binding, entity)
}

fn live_binding_bytes(binding: &CharacterStateLiveBinding) -> usize {
    catalog_binding_bytes(&binding.catalog)
        + binding.game_instance_id.len()
        + binding.run_id.len()
        + binding.mode_id.len()
        + binding.snapshot_id.len()
        + 8
}

fn resource_reference_bytes(
    binding: &CharacterStateLiveBinding,
    resource: &CharacterResourceInput,
) -> usize {
    live_binding_bytes(binding)
        + resource.instance_id.len()
        + resource.definition_id.len()
        + resource.owner.id.as_str().len()
}

fn secondary_reference_bytes(
    binding: &CharacterStateLiveBinding,
    entity: &SecondaryEntityInput,
) -> usize {
    live_binding_bytes(binding)
        + entity.instance_id.len()
        + entity.definition_id.len()
        + entity.owner.id.as_str().len()
}

fn resource_value_bytes(value: Option<&CharacterResourceValue>) -> usize {
    value.map_or(1, |value| match value {
        CharacterResourceValue::Integer { unit, .. }
        | CharacterResourceValue::Boolean { unit, .. } => unit.as_str().len() + 8,
        CharacterResourceValue::Decimal { unit, .. } => unit.as_str().len() + 10,
        CharacterResourceValue::Text(value) => value.len(),
        CharacterResourceValue::Custom { kind, value, unit } => {
            kind.len() + value.len() + unit.as_ref().map_or(0, |unit| unit.as_str().len())
        }
    })
}

fn slots_bytes(slots: Option<&Vec<CharacterResourceSlot>>) -> usize {
    slots.map_or(1, |slots| {
        slots
            .iter()
            .map(|slot| slot.slot_id.len() + 8 + slot_content_bytes(slot.content.value()))
            .sum()
    })
}

fn slot_content_bytes(content: Option<&CharacterResourceSlotContent>) -> usize {
    content.map_or(1, |content| match content {
        CharacterResourceSlotContent::Empty => 1,
        CharacterResourceSlotContent::Definition(value)
        | CharacterResourceSlotContent::Instance(value)
        | CharacterResourceSlotContent::Text(value) => value.len(),
        CharacterResourceSlotContent::Custom { kind, value } => kind.len() + value.len(),
    })
}

fn controller_bytes(controller: Option<&SecondaryEntityControllerReference>) -> usize {
    controller.map_or(1, |controller| {
        controller.owner_id.as_str().len()
            + controller.character_id.len()
            + owner_kind_bytes(&controller.owner_kind)
            + controller.label.value().map_or(1, String::len)
    })
}

fn statuses_bytes(statuses: Option<&Vec<SecondaryEntityStatus>>) -> usize {
    statuses.map_or(1, |statuses| {
        statuses
            .iter()
            .map(|status| {
                status.instance_id.len()
                    + status.definition_id.len()
                    + status.label.value().map_or(1, String::len)
                    + 8
            })
            .sum()
    })
}

fn intent_bytes(intent: Option<&SecondaryEntityIntent>) -> usize {
    intent.map_or(1, |intent| {
        intent_kind_bytes(&intent.kind)
            + intent.rule_reference.value().map_or(1, String::len)
            + intent.label.value().map_or(1, String::len)
            + intent.target.value().map_or(1, target_bytes)
            + 16
    })
}

fn intent_kind_bytes(kind: &super::live_model::SecondaryEntityIntentKind) -> usize {
    match kind {
        super::live_model::SecondaryEntityIntentKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn owner_kind_bytes(kind: &CharacterStateOwnerKind) -> usize {
    match kind {
        CharacterStateOwnerKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn target_bytes(target: &SecondaryEntityTarget) -> usize {
    match target {
        SecondaryEntityTarget::Owner(value)
        | SecondaryEntityTarget::Entity(value)
        | SecondaryEntityTarget::Custom(value) => value.len(),
        SecondaryEntityTarget::All => 1,
    }
}
