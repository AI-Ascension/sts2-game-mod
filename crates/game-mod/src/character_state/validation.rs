// SPDX-License-Identifier: MIT

use super::model::{
    CharacterStateUnit, CharacterStateVisibility, validate_identity, validate_text,
};
use super::{
    definition::{
        CharacterResourceDefinitionInput, CharacterResourceKind, CharacterResourceValue,
        CharacterResourceValueDefinition, SecondaryEntityDefinitionInput, SecondaryEntityKind,
    },
    error::CharacterStateLiveError,
    model::{
        CHARACTER_STATE_MAX_INTENTS, CHARACTER_STATE_MAX_SLOTS, CHARACTER_STATE_MAX_STATUSES,
        CharacterMechanicCoverage, CharacterStateField, CharacterStateOwner,
        CharacterStateOwnerKind,
    },
};

pub(super) fn validate_coverage(coverage: &CharacterMechanicCoverage) -> Result<(), &'static str> {
    validate_identity(&coverage.character_id, "character_id")?;
    validate_identity(&coverage.mode_id, "mode_id")?;
    Ok(())
}

pub(super) fn validate_resource_definition(
    input: &CharacterResourceDefinitionInput,
) -> Result<(), &'static str> {
    validate_identity(&input.definition_id, "resource_definition_id")?;
    validate_identity(&input.character_id, "character_id")?;
    validate_identity(&input.mode_id, "mode_id")?;
    validate_text(&input.label, "resource_label")?;
    validate_identity(&input.rule_reference, "resource_rule_reference")?;
    validate_resource_kind(&input.kind)?;
    validate_resource_value_definition(&input.value)?;
    if input.maximum.is_some_and(|value| value < 0) {
        return Err("resource_maximum");
    }
    validate_visibility(input.visibility)?;
    validate_visibility(input.slots_visibility)?;
    Ok(())
}

pub(super) fn validate_secondary_entity_definition(
    input: &SecondaryEntityDefinitionInput,
) -> Result<(), &'static str> {
    validate_identity(&input.definition_id, "secondary_definition_id")?;
    validate_identity(&input.character_id, "character_id")?;
    validate_identity(&input.mode_id, "mode_id")?;
    validate_text(&input.label, "secondary_label")?;
    validate_identity(&input.rule_reference, "secondary_rule_reference")?;
    validate_entity_kind(&input.kind)?;
    validate_visibility(input.visibility)?;
    Ok(())
}

pub(super) fn validate_owner(owner: &CharacterStateOwner) -> Result<(), CharacterStateLiveError> {
    validate_identity(owner.id.as_str(), "owner_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    validate_identity(&owner.character_id, "owner_character_id")
        .map_err(CharacterStateLiveError::InvalidInput)?;
    validate_owner_kind(&owner.kind).map_err(CharacterStateLiveError::InvalidInput)?;
    if let CharacterStateField::Available(label) = &owner.label {
        validate_text(label, "owner_label").map_err(CharacterStateLiveError::InvalidInput)?;
    }
    Ok(())
}

pub(super) fn validate_resource_value(
    value: &CharacterResourceValue,
) -> Result<(), CharacterStateLiveError> {
    match value {
        CharacterResourceValue::Integer { unit, .. }
        | CharacterResourceValue::Boolean { unit, .. } => {
            validate_unit(unit).map_err(CharacterStateLiveError::InvalidInput)?;
        }
        CharacterResourceValue::Decimal { unit, scale, .. } => {
            validate_unit(unit).map_err(CharacterStateLiveError::InvalidInput)?;
            if *scale > 9 {
                return Err(CharacterStateLiveError::ValueShapeMismatch("value_scale"));
            }
        }
        CharacterResourceValue::Text(value) => {
            validate_text(value, "resource_value_text")
                .map_err(CharacterStateLiveError::InvalidInput)?;
        }
        CharacterResourceValue::Custom { kind, value, unit } => {
            validate_identity(kind, "resource_value_kind")
                .map_err(CharacterStateLiveError::InvalidInput)?;
            validate_text(value, "resource_value")
                .map_err(CharacterStateLiveError::InvalidInput)?;
            if let Some(unit) = unit {
                validate_unit(unit).map_err(CharacterStateLiveError::InvalidInput)?;
            }
        }
    }
    Ok(())
}

pub(super) fn value_matches_definition(
    definition: &CharacterResourceValueDefinition,
    value: &CharacterResourceValue,
) -> Result<(), CharacterStateLiveError> {
    match (definition, value) {
        (
            CharacterResourceValueDefinition::Integer { unit: expected },
            CharacterResourceValue::Integer { unit: observed, .. },
        )
        | (
            CharacterResourceValueDefinition::Boolean { unit: expected },
            CharacterResourceValue::Boolean { unit: observed, .. },
        ) if expected == observed => Ok(()),
        (
            CharacterResourceValueDefinition::Decimal {
                unit: expected,
                scale,
            },
            CharacterResourceValue::Decimal {
                unit: observed,
                scale: observed_scale,
                ..
            },
        ) if expected == observed && scale == observed_scale => Ok(()),
        (CharacterResourceValueDefinition::Text, CharacterResourceValue::Text(_)) => Ok(()),
        (
            CharacterResourceValueDefinition::Custom {
                kind: expected_kind,
                unit: expected_unit,
            },
            CharacterResourceValue::Custom {
                kind: observed_kind,
                unit: observed_unit,
                ..
            },
        ) if expected_kind == observed_kind && expected_unit == observed_unit => Ok(()),
        _ => Err(CharacterStateLiveError::ValueShapeMismatch(
            "resource_value",
        )),
    }
}

pub(super) fn integer_value(value: &CharacterResourceValue) -> Option<i64> {
    match value {
        CharacterResourceValue::Integer { value, .. } => Some(*value),
        CharacterResourceValue::Decimal { value, .. } => Some(*value),
        _ => None,
    }
}

pub(super) fn validate_slot_count(count: usize) -> Result<(), CharacterStateLiveError> {
    if count > CHARACTER_STATE_MAX_SLOTS {
        return Err(CharacterStateLiveError::InvalidInput("slot_count"));
    }
    Ok(())
}

pub(super) fn validate_status_count(count: usize) -> Result<(), CharacterStateLiveError> {
    if count > CHARACTER_STATE_MAX_STATUSES {
        return Err(CharacterStateLiveError::InvalidInput("status_count"));
    }
    Ok(())
}

pub(super) fn validate_intent_count(count: usize) -> Result<(), CharacterStateLiveError> {
    if count > CHARACTER_STATE_MAX_INTENTS {
        return Err(CharacterStateLiveError::InvalidInput("intent_count"));
    }
    Ok(())
}

pub(super) fn field_bytes<T>(field: &CharacterStateField<T>, value_bytes: usize) -> usize {
    match field {
        CharacterStateField::Available(_) => value_bytes,
        CharacterStateField::NotApplicable
        | CharacterStateField::NotObserved
        | CharacterStateField::Unsupported
        | CharacterStateField::Denied
        | CharacterStateField::Unavailable
        | CharacterStateField::Unknown => 1,
    }
}

pub(super) fn visibility_allowed(
    visibility: CharacterStateVisibility,
    scope: super::model::CharacterStateVisibilityScope,
) -> bool {
    match visibility {
        CharacterStateVisibility::Visible => true,
        CharacterStateVisibility::OwnerOnly => {
            matches!(scope, super::model::CharacterStateVisibilityScope::Owner)
        }
        CharacterStateVisibility::Hidden | CharacterStateVisibility::Unknown => false,
    }
}

fn validate_resource_kind(kind: &CharacterResourceKind) -> Result<(), &'static str> {
    if let CharacterResourceKind::Custom(value) = kind {
        validate_identity(value, "resource_kind")?;
    }
    Ok(())
}

fn validate_resource_value_definition(
    value: &CharacterResourceValueDefinition,
) -> Result<(), &'static str> {
    match value {
        CharacterResourceValueDefinition::Integer { unit }
        | CharacterResourceValueDefinition::Boolean { unit } => validate_unit(unit),
        CharacterResourceValueDefinition::Decimal { unit, scale } => {
            validate_unit(unit)?;
            if *scale > 9 {
                return Err("value_scale");
            }
            Ok(())
        }
        CharacterResourceValueDefinition::Text => Ok(()),
        CharacterResourceValueDefinition::Custom { kind, unit } => {
            validate_identity(kind, "value_kind")?;
            if let Some(unit) = unit {
                validate_unit(unit)?;
            }
            Ok(())
        }
    }
}

fn validate_entity_kind(kind: &SecondaryEntityKind) -> Result<(), &'static str> {
    if let SecondaryEntityKind::Custom(value) = kind {
        validate_identity(value, "secondary_kind")?;
    }
    Ok(())
}

fn validate_owner_kind(kind: &CharacterStateOwnerKind) -> Result<(), &'static str> {
    if let CharacterStateOwnerKind::Custom(value) = kind {
        validate_identity(value, "owner_kind")?;
    }
    Ok(())
}

fn validate_visibility(visibility: CharacterStateVisibility) -> Result<(), &'static str> {
    if matches!(visibility, CharacterStateVisibility::Unknown) {
        return Err("visibility");
    }
    Ok(())
}

fn validate_unit(unit: &CharacterStateUnit) -> Result<(), &'static str> {
    validate_identity(unit.as_str(), "unit")
}

pub(super) fn definition_bytes_resource(input: &CharacterResourceDefinitionInput) -> usize {
    input.definition_id.len()
        + input.character_id.len()
        + input.mode_id.len()
        + input.label.len()
        + input.rule_reference.len()
        + value_definition_bytes(&input.value)
        + 16
}

pub(super) fn definition_bytes_entity(input: &SecondaryEntityDefinitionInput) -> usize {
    input.definition_id.len()
        + input.character_id.len()
        + input.mode_id.len()
        + input.label.len()
        + input.rule_reference.len()
        + 16
}

fn value_definition_bytes(value: &CharacterResourceValueDefinition) -> usize {
    match value {
        CharacterResourceValueDefinition::Integer { unit }
        | CharacterResourceValueDefinition::Boolean { unit } => unit.as_str().len() + 1,
        CharacterResourceValueDefinition::Decimal { unit, .. } => unit.as_str().len() + 2,
        CharacterResourceValueDefinition::Text => 1,
        CharacterResourceValueDefinition::Custom { kind, unit } => {
            kind.len() + unit.as_ref().map_or(0, |unit| unit.as_str().len()) + 1
        }
    }
}
