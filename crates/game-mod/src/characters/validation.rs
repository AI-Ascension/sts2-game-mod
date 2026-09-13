// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::CharacterCatalogError;
use super::definition::{
    CharacterDefinitionInput, CharacterLoadoutInput, CharacterLoadoutRequirement,
};
use super::model::{
    CHARACTER_MAX_FORMULA_INPUTS, CHARACTER_MAX_LOADOUTS, CHARACTER_MAX_MECHANIC_REFERENCES,
    CHARACTER_MAX_POOLS, CHARACTER_MAX_REFERENCES, CHARACTER_MAX_REQUIREMENTS,
    CHARACTER_MAX_RESOURCES, CharacterContentReference, CharacterField, CharacterFormula,
    CharacterNumericValue, CharacterOrigin, CharacterText, validate_identity, validate_text,
};

pub(super) fn validate_definition(
    input: &CharacterDefinitionInput,
) -> Result<(), CharacterCatalogError> {
    validate_identity(&input.character_id, "character_id")?;
    validate_text_value(&input.name, "name")?;
    validate_text_value(&input.description, "description")?;
    validate_origin(&input.origin)?;
    if input.loadouts.is_empty() {
        return Err(CharacterCatalogError::InvalidInput("loadouts"));
    }
    if input.loadouts.len() > CHARACTER_MAX_LOADOUTS {
        return Err(CharacterCatalogError::InvalidInput("loadouts"));
    }
    let mut loadout_ids = BTreeSet::new();
    validate_unlock(&input.unlock)?;
    for loadout in &input.loadouts {
        if !loadout_ids.insert(loadout.loadout_id.as_str()) {
            return Err(CharacterCatalogError::InvalidInput("duplicate_loadout"));
        }
        validate_loadout(loadout)?;
    }
    Ok(())
}

fn validate_loadout(loadout: &CharacterLoadoutInput) -> Result<(), CharacterCatalogError> {
    validate_identity(&loadout.loadout_id, "loadout_id")?;
    if let Some(mode) = &loadout.mode {
        validate_identity(mode, "mode")?;
    }
    if let Some(difficulty) = &loadout.difficulty {
        validate_identity(difficulty, "difficulty")?;
    }
    validate_origin(&loadout.origin)?;
    validate_starting(&loadout.starting)?;
    validate_optional_content_field(&loadout.starting_deck)?;
    validate_optional_content_field(&loadout.starting_relics)?;
    validate_pools(&loadout.pools)?;
    validate_mechanics(&loadout.mechanics)?;
    validate_requirements(&loadout.prerequisites)?;
    Ok(())
}

fn validate_starting(
    starting: &super::definition::CharacterStartingConfiguration,
) -> Result<(), CharacterCatalogError> {
    validate_numeric(&starting.starting_hp)?;
    validate_numeric(&starting.max_hp)?;
    validate_numeric(&starting.gold)?;
    validate_numeric(&starting.potion_capacity)?;
    if let CharacterField::Available(resources) = &starting.resources {
        if resources.len() > CHARACTER_MAX_RESOURCES {
            return Err(CharacterCatalogError::InvalidInput("resources"));
        }
        let mut resource_ids = BTreeSet::new();
        for resource in resources {
            validate_identity(&resource.resource_id, "resource_id")?;
            if !resource_ids.insert(resource.resource_id.as_str()) {
                return Err(CharacterCatalogError::InvalidInput("duplicate_resource"));
            }
            if let Some(unit) = &resource.unit {
                validate_identity(unit, "resource_unit")?;
            }
            validate_numeric(&resource.amount)?;
            if let Some(capacity) = &resource.capacity {
                validate_numeric(capacity)?;
            }
        }
    }
    Ok(())
}

fn validate_optional_content_field(
    field: &CharacterField<Vec<CharacterContentReference>>,
) -> Result<(), CharacterCatalogError> {
    if let CharacterField::Available(references) = field {
        validate_content_references(references)?;
    }
    Ok(())
}

fn validate_content_references(
    references: &[CharacterContentReference],
) -> Result<(), CharacterCatalogError> {
    if references.len() > CHARACTER_MAX_REFERENCES {
        return Err(CharacterCatalogError::InvalidInput("content_references"));
    }
    let mut identities = BTreeSet::new();
    for reference in references {
        validate_identity(&reference.entity_kind, "content_entity_kind")?;
        validate_identity(&reference.namespaced_id, "content_id")?;
        if reference.quantity == 0 {
            return Err(CharacterCatalogError::InvalidInput("content_quantity"));
        }
        if !identities.insert((&reference.entity_kind, &reference.namespaced_id)) {
            return Err(CharacterCatalogError::InvalidInput(
                "duplicate_content_reference",
            ));
        }
    }
    Ok(())
}

fn validate_requirements(
    field: &CharacterField<Vec<CharacterLoadoutRequirement>>,
) -> Result<(), CharacterCatalogError> {
    if let CharacterField::Available(requirements) = field {
        if requirements.len() > CHARACTER_MAX_REQUIREMENTS {
            return Err(CharacterCatalogError::InvalidInput("requirements"));
        }
        let mut ids = BTreeSet::new();
        for requirement in requirements {
            validate_identity(&requirement.progression_id, "progression_id")?;
            validate_text_value(&requirement.description, "requirement_description")?;
            if !ids.insert(requirement.progression_id.as_str()) {
                return Err(CharacterCatalogError::InvalidInput("duplicate_requirement"));
            }
        }
    }
    Ok(())
}

fn validate_unlock(
    field: &CharacterField<super::definition::CharacterUnlock>,
) -> Result<(), CharacterCatalogError> {
    if let CharacterField::Available(unlock) = field {
        validate_requirements(&unlock.requirements)?;
    }
    Ok(())
}

fn validate_pools(
    field: &CharacterField<Vec<super::definition::CharacterPoolReference>>,
) -> Result<(), CharacterCatalogError> {
    if let CharacterField::Available(pools) = field {
        if pools.len() > CHARACTER_MAX_POOLS {
            return Err(CharacterCatalogError::InvalidInput("pools"));
        }
        let mut pool_ids = BTreeSet::new();
        for pool in pools {
            validate_identity(&pool.entity_kind, "pool_entity_kind")?;
            validate_identity(&pool.pool_id, "pool_id")?;
            validate_text_value(&pool.label, "pool_label")?;
            if !pool_ids.insert((&pool.entity_kind, &pool.pool_id)) {
                return Err(CharacterCatalogError::InvalidInput("duplicate_pool"));
            }
        }
    }
    Ok(())
}

fn validate_mechanics(
    field: &CharacterField<Vec<super::definition::CharacterMechanicReference>>,
) -> Result<(), CharacterCatalogError> {
    if let CharacterField::Available(mechanics) = field {
        if mechanics.len() > CHARACTER_MAX_MECHANIC_REFERENCES {
            return Err(CharacterCatalogError::InvalidInput("mechanics"));
        }
        let mut mechanic_ids = BTreeSet::new();
        for mechanic in mechanics {
            validate_identity(&mechanic.mechanic_id, "mechanic_id")?;
            validate_text_value(&mechanic.label, "mechanic_label")?;
            if !mechanic_ids.insert(mechanic.mechanic_id.as_str()) {
                return Err(CharacterCatalogError::InvalidInput("duplicate_mechanic"));
            }
            if mechanic.dependencies.len() > CHARACTER_MAX_FORMULA_INPUTS {
                return Err(CharacterCatalogError::InvalidInput("mechanic_dependencies"));
            }
            for dependency in &mechanic.dependencies {
                validate_identity(dependency, "mechanic_dependency")?;
            }
        }
    }
    Ok(())
}

fn validate_numeric(value: &CharacterNumericValue) -> Result<(), CharacterCatalogError> {
    if let CharacterNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

fn validate_formula(formula: &CharacterFormula) -> Result<(), CharacterCatalogError> {
    validate_identity(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > CHARACTER_MAX_FORMULA_INPUTS {
        return Err(CharacterCatalogError::InvalidInput("formula_inputs"));
    }
    let mut inputs = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input")?;
        if !inputs.insert(input.as_str()) {
            return Err(CharacterCatalogError::InvalidInput(
                "duplicate_formula_input",
            ));
        }
    }
    Ok(())
}

fn validate_text_value(
    value: &CharacterText,
    field: &'static str,
) -> Result<(), CharacterCatalogError> {
    if let CharacterText::Available(text) = value {
        if text.is_empty() {
            return Err(CharacterCatalogError::InvalidInput(field));
        }
        validate_text(text, field)?;
    }
    Ok(())
}

fn validate_origin(origin: &CharacterOrigin) -> Result<(), CharacterCatalogError> {
    validate_identity(&origin.kind, "origin_kind")?;
    if let Some(package_id) = &origin.package_id {
        validate_identity(package_id, "package_id")?;
    }
    if let Some(package_version) = &origin.package_version {
        validate_identity(package_version, "package_version")?;
    }
    Ok(())
}
