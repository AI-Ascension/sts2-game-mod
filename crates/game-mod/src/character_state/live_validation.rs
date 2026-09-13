// SPDX-License-Identifier: MIT

use super::{
    catalog::CharacterStateCatalog,
    definition::{CharacterResourceValue, CharacterResourceValueDefinition},
    error::CharacterStateLiveError,
    live_measure::{entity_bytes, resource_bytes},
    live_model::{CharacterResourceInput, SecondaryEntityInput},
    live_shape::{
        validate_controller, validate_intent, validate_nonnegative_field, validate_slots,
        validate_statuses,
    },
    model::{
        CHARACTER_STATE_MAX_DETAIL_BYTES, CharacterMechanicState, CharacterStateField,
        CharacterStateLiveBinding, CharacterStateOwner, CharacterStateVisibilityScope,
    },
    reader::CharacterStateDefinitionKind,
    validation::{
        integer_value, validate_owner, validate_resource_value, value_matches_definition,
        visibility_allowed,
    },
};

pub(super) fn validate_binding(
    binding: &CharacterStateLiveBinding,
) -> Result<(), CharacterStateLiveError> {
    if binding.catalog.producer_version != super::model::CHARACTER_STATE_PRODUCER_VERSION {
        return Err(CharacterStateLiveError::ProducerVersionMismatch);
    }
    for (value, field) in [
        (&binding.game_instance_id, "game_instance_id"),
        (&binding.run_id, "run_id"),
        (&binding.mode_id, "mode_id"),
        (&binding.snapshot_id, "snapshot_id"),
    ] {
        super::model::validate_identity(value, field)
            .map_err(CharacterStateLiveError::InvalidBinding)?;
    }
    Ok(())
}

pub(super) fn validate_snapshot(
    catalog: &CharacterStateCatalog,
    snapshot: &super::live_snapshot::CharacterStateLiveSnapshot,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    for resource in snapshot.resources.values() {
        validate_resource_against_catalog(catalog, &snapshot.binding.mode_id, resource, scope)?;
    }
    for entity in snapshot.secondary_entities.values() {
        validate_entity_against_catalog(catalog, &snapshot.binding.mode_id, entity, scope)?;
    }
    Ok(())
}

fn validate_resource_against_catalog(
    catalog: &CharacterStateCatalog,
    mode_id: &str,
    resource: &CharacterResourceInput,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    ensure_state_for_owner(
        catalog,
        &resource.owner,
        mode_id,
        CharacterStateDefinitionKind::Resource,
    )?;
    validate_owner(&resource.owner)?;
    let definition = catalog
        .resource_definition(&resource.definition_id)
        .ok_or_else(|| {
            CharacterStateLiveError::UnknownResourceDefinition(resource.definition_id.clone())
        })?;
    if definition.character_id != resource.owner.character_id || definition.mode_id != mode_id {
        return Err(CharacterStateLiveError::InvalidInput("resource_owner"));
    }
    if !visibility_allowed(definition.visibility, scope) {
        return Err(CharacterStateLiveError::VisibilityDenied("resource"));
    }
    validate_resource_field(&resource.current, &definition.value)?;
    validate_resource_field(&resource.maximum, &definition.value)?;
    if let Some(maximum) = definition.maximum {
        for (field, name) in [
            (&resource.current, "resource_current"),
            (&resource.maximum, "resource_maximum"),
        ] {
            if let Some(value) = field.value().and_then(integer_value)
                && value > maximum
            {
                return Err(CharacterStateLiveError::ValueOutOfRange(name));
            }
        }
    }
    if let (Some(current), Some(maximum)) = (
        resource.current.value().and_then(integer_value),
        resource.maximum.value().and_then(integer_value),
    ) && current > maximum
    {
        return Err(CharacterStateLiveError::ValueOutOfRange("resource_current"));
    }
    validate_slots(resource, definition, scope)?;
    if resource_bytes(resource) > CHARACTER_STATE_MAX_DETAIL_BYTES {
        return Err(CharacterStateLiveError::DetailTooLarge {
            limit: CHARACTER_STATE_MAX_DETAIL_BYTES,
            actual: resource_bytes(resource),
        });
    }
    Ok(())
}

fn validate_resource_field(
    field: &CharacterStateField<CharacterResourceValue>,
    definition: &CharacterResourceValueDefinition,
) -> Result<(), CharacterStateLiveError> {
    let Some(value) = field.value() else {
        return Ok(());
    };
    validate_resource_value(value)?;
    value_matches_definition(definition, value)
}

fn validate_entity_against_catalog(
    catalog: &CharacterStateCatalog,
    mode_id: &str,
    entity: &SecondaryEntityInput,
    scope: CharacterStateVisibilityScope,
) -> Result<(), CharacterStateLiveError> {
    ensure_state_for_owner(
        catalog,
        &entity.owner,
        mode_id,
        CharacterStateDefinitionKind::SecondaryEntity,
    )?;
    validate_owner(&entity.owner)?;
    let definition = catalog
        .secondary_definition(&entity.definition_id)
        .ok_or_else(|| {
            CharacterStateLiveError::UnknownSecondaryEntityDefinition(entity.definition_id.clone())
        })?;
    if definition.character_id != entity.owner.character_id || definition.mode_id != mode_id {
        return Err(CharacterStateLiveError::InvalidInput("secondary_owner"));
    }
    if !visibility_allowed(definition.visibility, scope) {
        return Err(CharacterStateLiveError::VisibilityDenied(
            "secondary_entity",
        ));
    }
    validate_controller(&entity.controller)?;
    validate_nonnegative_field(&entity.hp, "hp")?;
    validate_nonnegative_field(&entity.maximum_hp, "maximum_hp")?;
    validate_nonnegative_field(&entity.block, "block")?;
    if let (Some(hp), Some(maximum_hp)) = (entity.hp.value(), entity.maximum_hp.value())
        && hp > maximum_hp
    {
        return Err(CharacterStateLiveError::ValueOutOfRange("hp"));
    }
    validate_statuses(&entity.statuses, scope)?;
    validate_intent(&entity.intent, scope)?;
    if entity_bytes(entity) > CHARACTER_STATE_MAX_DETAIL_BYTES {
        return Err(CharacterStateLiveError::DetailTooLarge {
            limit: CHARACTER_STATE_MAX_DETAIL_BYTES,
            actual: entity_bytes(entity),
        });
    }
    Ok(())
}

pub(super) fn ensure_state_for_owner(
    catalog: &CharacterStateCatalog,
    owner: &CharacterStateOwner,
    mode_id: &str,
    kind: CharacterStateDefinitionKind,
) -> Result<(), CharacterStateLiveError> {
    let coverage = catalog
        .coverage(&owner.character_id, mode_id)
        .ok_or_else(|| CharacterStateLiveError::MissingCoverage {
            character_id: owner.character_id.clone(),
            mode_id: mode_id.to_owned(),
        })?;
    let state = match kind {
        CharacterStateDefinitionKind::Resource => coverage.resources,
        CharacterStateDefinitionKind::SecondaryEntity => coverage.secondary_entities,
    };
    ensure_state(state, kind)
}

pub(super) fn ensure_state_for_character(
    catalog: &CharacterStateCatalog,
    character_id: &str,
    mode_id: &str,
    kind: CharacterStateDefinitionKind,
) -> Result<(), CharacterStateLiveError> {
    let coverage = catalog.coverage(character_id, mode_id).ok_or_else(|| {
        CharacterStateLiveError::MissingCoverage {
            character_id: character_id.to_owned(),
            mode_id: mode_id.to_owned(),
        }
    })?;
    let state = match kind {
        CharacterStateDefinitionKind::Resource => coverage.resources,
        CharacterStateDefinitionKind::SecondaryEntity => coverage.secondary_entities,
    };
    ensure_state(state, kind)
}

fn ensure_state(
    state: CharacterMechanicState,
    kind: CharacterStateDefinitionKind,
) -> Result<(), CharacterStateLiveError> {
    match (kind, state) {
        (_, CharacterMechanicState::Supported) => Ok(()),
        (CharacterStateDefinitionKind::Resource, CharacterMechanicState::Unsupported) => {
            Err(CharacterStateLiveError::UnsupportedResources)
        }
        (CharacterStateDefinitionKind::Resource, CharacterMechanicState::NotApplicable) => {
            Err(CharacterStateLiveError::ResourcesNotApplicable)
        }
        (CharacterStateDefinitionKind::Resource, CharacterMechanicState::Unavailable) => {
            Err(CharacterStateLiveError::ResourcesUnavailable)
        }
        (CharacterStateDefinitionKind::Resource, CharacterMechanicState::Unknown) => {
            Err(CharacterStateLiveError::ResourcesUnknown)
        }
        (CharacterStateDefinitionKind::SecondaryEntity, CharacterMechanicState::Unsupported) => {
            Err(CharacterStateLiveError::UnsupportedSecondaryEntities)
        }
        (CharacterStateDefinitionKind::SecondaryEntity, CharacterMechanicState::NotApplicable) => {
            Err(CharacterStateLiveError::SecondaryEntitiesNotApplicable)
        }
        (CharacterStateDefinitionKind::SecondaryEntity, CharacterMechanicState::Unavailable) => {
            Err(CharacterStateLiveError::SecondaryEntitiesUnavailable)
        }
        (CharacterStateDefinitionKind::SecondaryEntity, CharacterMechanicState::Unknown) => {
            Err(CharacterStateLiveError::SecondaryEntitiesUnknown)
        }
    }
}
