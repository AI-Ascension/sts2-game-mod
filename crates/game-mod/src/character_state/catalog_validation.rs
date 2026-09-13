// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::reader::CharacterStateDefinitionKind;
use super::{
    definition::{
        CharacterResourceDefinition, CharacterResourceDefinitionInput,
        CharacterResourceDefinitionReference, SecondaryEntityDefinition,
        SecondaryEntityDefinitionInput, SecondaryEntityDefinitionReference,
    },
    error::CharacterStateCatalogError,
    model::{
        CHARACTER_STATE_MAX_COVERAGE, CHARACTER_STATE_MAX_DEFINITION_BYTES,
        CHARACTER_STATE_MAX_DEFINITIONS, CharacterMechanicCoverage, CharacterMechanicState,
        CharacterStateCatalogBinding,
    },
    validation::{
        definition_bytes_entity, definition_bytes_resource, validate_coverage,
        validate_resource_definition, validate_secondary_entity_definition,
    },
};

pub(super) fn collect_coverage(
    inputs: Vec<CharacterMechanicCoverage>,
) -> Result<BTreeMap<(String, String), CharacterMechanicCoverage>, CharacterStateCatalogError> {
    if inputs.is_empty() || inputs.len() > CHARACTER_STATE_MAX_COVERAGE {
        return Err(CharacterStateCatalogError::InvalidInput("coverage"));
    }
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_coverage(&input).map_err(CharacterStateCatalogError::InvalidInput)?;
        let key = (input.character_id.clone(), input.mode_id.clone());
        if records.insert(key.clone(), input).is_some() {
            return Err(CharacterStateCatalogError::DuplicateCoverage {
                character_id: key.0,
                mode_id: key.1,
            });
        }
    }
    Ok(records)
}

pub(super) fn collect_resources(
    inputs: Vec<CharacterResourceDefinitionInput>,
    binding: &CharacterStateCatalogBinding,
) -> Result<BTreeMap<String, CharacterResourceDefinition>, CharacterStateCatalogError> {
    if inputs.len() > CHARACTER_STATE_MAX_DEFINITIONS {
        return Err(CharacterStateCatalogError::InvalidInput(
            "resource_definitions",
        ));
    }
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_resource_definition(&input).map_err(CharacterStateCatalogError::InvalidInput)?;
        let bytes = definition_bytes_resource(&input);
        if bytes > CHARACTER_STATE_MAX_DEFINITION_BYTES {
            return Err(CharacterStateCatalogError::DefinitionTooLarge {
                limit: CHARACTER_STATE_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let id = input.definition_id.clone();
        let definition = CharacterResourceDefinition {
            reference: CharacterResourceDefinitionReference {
                catalog: binding.clone(),
                definition_id: id.clone(),
            },
            character_id: input.character_id,
            mode_id: input.mode_id,
            kind: input.kind,
            label: input.label,
            rule_reference: input.rule_reference,
            value: input.value,
            maximum: input.maximum,
            visibility: input.visibility,
            slots_visibility: input.slots_visibility,
        };
        if records.insert(id.clone(), definition).is_some() {
            return Err(CharacterStateCatalogError::DuplicateDefinition(id));
        }
    }
    Ok(records)
}

pub(super) fn collect_entities(
    inputs: Vec<SecondaryEntityDefinitionInput>,
    binding: &CharacterStateCatalogBinding,
) -> Result<BTreeMap<String, SecondaryEntityDefinition>, CharacterStateCatalogError> {
    if inputs.len() > CHARACTER_STATE_MAX_DEFINITIONS {
        return Err(CharacterStateCatalogError::InvalidInput(
            "secondary_entity_definitions",
        ));
    }
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_secondary_entity_definition(&input)
            .map_err(CharacterStateCatalogError::InvalidInput)?;
        let bytes = definition_bytes_entity(&input);
        if bytes > CHARACTER_STATE_MAX_DEFINITION_BYTES {
            return Err(CharacterStateCatalogError::DefinitionTooLarge {
                limit: CHARACTER_STATE_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let id = input.definition_id.clone();
        let definition = SecondaryEntityDefinition {
            reference: SecondaryEntityDefinitionReference {
                catalog: binding.clone(),
                definition_id: id.clone(),
            },
            character_id: input.character_id,
            mode_id: input.mode_id,
            kind: input.kind,
            label: input.label,
            rule_reference: input.rule_reference,
            visibility: input.visibility,
        };
        if records.insert(id.clone(), definition).is_some() {
            return Err(CharacterStateCatalogError::DuplicateDefinition(id));
        }
    }
    Ok(records)
}

pub(super) fn validate_definition_coverage(
    coverage: &BTreeMap<(String, String), CharacterMechanicCoverage>,
    resources: &BTreeMap<String, CharacterResourceDefinition>,
    entities: &BTreeMap<String, SecondaryEntityDefinition>,
) -> Result<(), CharacterStateCatalogError> {
    let mut all_ids = BTreeSet::new();
    for definition in resources.values() {
        if !all_ids.insert(definition.reference.definition_id.clone()) {
            return Err(CharacterStateCatalogError::DuplicateDefinition(
                definition.reference.definition_id.clone(),
            ));
        }
        ensure_definition_coverage(
            coverage,
            &definition.character_id,
            &definition.mode_id,
            CharacterStateDefinitionKind::Resource,
            Some(&definition.reference.definition_id),
        )?;
    }
    for definition in entities.values() {
        if !all_ids.insert(definition.reference.definition_id.clone()) {
            return Err(CharacterStateCatalogError::DuplicateDefinition(
                definition.reference.definition_id.clone(),
            ));
        }
        ensure_definition_coverage(
            coverage,
            &definition.character_id,
            &definition.mode_id,
            CharacterStateDefinitionKind::SecondaryEntity,
            Some(&definition.reference.definition_id),
        )?;
    }
    for ((character_id, mode_id), record) in coverage {
        if record.resources == CharacterMechanicState::Supported
            && !resources.values().any(|definition| {
                definition.character_id == *character_id && definition.mode_id == *mode_id
            })
        {
            return Err(CharacterStateCatalogError::MissingDefinition(format!(
                "resource:{character_id}:{mode_id}"
            )));
        }
        if record.secondary_entities == CharacterMechanicState::Supported
            && !entities.values().any(|definition| {
                definition.character_id == *character_id && definition.mode_id == *mode_id
            })
        {
            return Err(CharacterStateCatalogError::MissingDefinition(format!(
                "secondary:{character_id}:{mode_id}"
            )));
        }
    }
    Ok(())
}

fn ensure_definition_coverage(
    coverage: &BTreeMap<(String, String), CharacterMechanicCoverage>,
    character_id: &str,
    mode_id: &str,
    kind: CharacterStateDefinitionKind,
    definition_id: Option<&str>,
) -> Result<(), CharacterStateCatalogError> {
    let Some(record) = coverage.get(&(character_id.to_owned(), mode_id.to_owned())) else {
        return Err(CharacterStateCatalogError::DefinitionWithoutCoverage(
            definition_id.unwrap_or_default().to_owned(),
        ));
    };
    let state = match kind {
        CharacterStateDefinitionKind::Resource => record.resources,
        CharacterStateDefinitionKind::SecondaryEntity => record.secondary_entities,
    };
    if state != CharacterMechanicState::Supported {
        return Err(CharacterStateCatalogError::DefinitionWithoutCoverage(
            definition_id.unwrap_or_default().to_owned(),
        ));
    }
    Ok(())
}
