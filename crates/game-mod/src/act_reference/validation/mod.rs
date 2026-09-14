// SPDX-License-Identifier: MIT

mod values;

use std::collections::BTreeSet;

use self::values::{
    validate_constraint, validate_eligibility, validate_group, validate_pool_entry,
    validate_references, validate_room_category, validate_text_value, validate_visibility,
    validate_weight,
};

use super::ActReferenceError;
use super::definition::{
    ActDefinitionInput, EncounterDefinitionInput, EncounterKind, EncounterPool, EncounterPoolKind,
    MapGenerationConstraint, RoomCategoryDefinition,
};
use super::model::{
    ACT_MAX_CONSTRAINTS, ACT_MAX_ELIGIBILITY, ACT_MAX_ENCOUNTERS, ACT_MAX_ENEMY_GROUPS,
    ACT_MAX_GROUP_ENEMIES, ACT_MAX_POOL_ENTRIES, ACT_MAX_POOLS, ACT_MAX_ROOM_CATEGORIES,
    ACT_MAX_VARIANTS, ActField, validate_identity,
};

/// Validates one source-owned act definition before it enters an immutable catalog.
pub(super) fn validate_definition(input: &ActDefinitionInput) -> Result<(), ActReferenceError> {
    validate_identity(&input.act_id, "act_id")?;
    validate_text_value(&input.name, "name")?;
    validate_text_value(&input.description, "description")?;
    validate_visibility(input.visibility)?;
    let categories = validate_room_categories(&input.room_categories)?;
    let encounters = validate_encounters(&input.act_id, &input.encounters, &categories)?;
    validate_pools(&input.act_id, &input.pools, &encounters, &categories)?;
    validate_constraints(&input.constraints)?;
    validate_references(&input.references)
}

fn validate_room_categories(
    categories: &[RoomCategoryDefinition],
) -> Result<BTreeSet<&str>, ActReferenceError> {
    if categories.len() > ACT_MAX_ROOM_CATEGORIES {
        return Err(ActReferenceError::InvalidInput("room_categories"));
    }
    let mut ids = BTreeSet::new();
    for category in categories {
        validate_room_category(category)?;
        if !ids.insert(category.category_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_room_category"));
        }
    }
    Ok(ids)
}

fn validate_encounters<'a>(
    act_id: &str,
    encounters: &'a [EncounterDefinitionInput],
    categories: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, ActReferenceError> {
    if encounters.len() > ACT_MAX_ENCOUNTERS {
        return Err(ActReferenceError::InvalidInput("encounters"));
    }
    let mut ids = BTreeSet::new();
    for encounter in encounters {
        validate_encounter(act_id, encounter, categories)?;
        if !ids.insert(encounter.encounter_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_encounter"));
        }
    }
    Ok(ids)
}

fn validate_encounter(
    act_id: &str,
    encounter: &EncounterDefinitionInput,
    categories: &BTreeSet<&str>,
) -> Result<(), ActReferenceError> {
    validate_identity(&encounter.encounter_id, "encounter_id")?;
    validate_text_value(&encounter.name, "encounter_name")?;
    validate_encounter_kind(&encounter.kind)?;
    if let Some(category_id) = &encounter.room_category_id {
        resolve_category(act_id, category_id, categories)?;
    }
    if encounter.groups.is_empty() || encounter.groups.len() > ACT_MAX_ENEMY_GROUPS {
        return Err(ActReferenceError::InvalidInput("groups"));
    }
    let mut group_ids = BTreeSet::new();
    for group in &encounter.groups {
        if group.enemies.len() > ACT_MAX_GROUP_ENEMIES {
            return Err(ActReferenceError::InvalidInput("enemies"));
        }
        validate_group(group)?;
        if !group_ids.insert(group.group_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_group"));
        }
    }
    if encounter.eligibility.len() > ACT_MAX_ELIGIBILITY {
        return Err(ActReferenceError::InvalidInput("eligibility"));
    }
    let mut condition_ids = BTreeSet::new();
    for condition in &encounter.eligibility {
        validate_eligibility(condition)?;
        if !condition_ids.insert(condition.condition_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_eligibility"));
        }
    }
    validate_weight(&encounter.weight)?;
    validate_references(&encounter.references)?;
    validate_visibility(encounter.visibility)
}

fn validate_encounter_kind(kind: &EncounterKind) -> Result<(), ActReferenceError> {
    if let EncounterKind::Custom(value) | EncounterKind::Unsupported(value) = kind {
        validate_identity(value, "encounter_kind")?;
    }
    Ok(())
}

fn validate_pools(
    act_id: &str,
    field: &ActField<Vec<EncounterPool>>,
    encounters: &BTreeSet<&str>,
    categories: &BTreeSet<&str>,
) -> Result<(), ActReferenceError> {
    let ActField::Available(pools) = field else {
        return Ok(());
    };
    if pools.len() > ACT_MAX_POOLS {
        return Err(ActReferenceError::InvalidInput("pools"));
    }
    let mut ids = BTreeSet::new();
    for pool in pools {
        validate_pool(act_id, pool, encounters, categories)?;
        if !ids.insert(pool.pool_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_pool"));
        }
    }
    Ok(())
}

fn validate_pool(
    act_id: &str,
    pool: &EncounterPool,
    encounters: &BTreeSet<&str>,
    categories: &BTreeSet<&str>,
) -> Result<(), ActReferenceError> {
    validate_identity(&pool.pool_id, "pool_id")?;
    validate_text_value(&pool.name, "pool_name")?;
    if let EncounterPoolKind::Custom(value) | EncounterPoolKind::Unsupported(value) = &pool.kind {
        validate_identity(value, "pool_kind")?;
    }
    if let Some(category_id) = &pool.room_category_id {
        resolve_category(act_id, category_id, categories)?;
    }
    if pool.entries.is_empty() || pool.entries.len() > ACT_MAX_POOL_ENTRIES {
        return Err(ActReferenceError::InvalidInput("pool_entries"));
    }
    let mut entry_ids = BTreeSet::new();
    for entry in &pool.entries {
        validate_pool_entry(entry)?;
        if !entry_ids.insert(entry.entry_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_pool_entry"));
        }
        if !encounters.contains(entry.encounter_id.as_str()) {
            return Err(ActReferenceError::UnknownEncounterReference {
                act_id: act_id.to_owned(),
                encounter_id: entry.encounter_id.clone(),
            });
        }
    }
    validate_references(&pool.references)?;
    validate_visibility(pool.visibility)
}

fn resolve_category(
    act_id: &str,
    category_id: &str,
    categories: &BTreeSet<&str>,
) -> Result<(), ActReferenceError> {
    validate_identity(category_id, "category_id")?;
    if categories.contains(category_id) {
        return Ok(());
    }
    Err(ActReferenceError::UnknownRoomCategoryReference {
        act_id: act_id.to_owned(),
        category_id: category_id.to_owned(),
    })
}

fn validate_constraints(
    field: &ActField<Vec<MapGenerationConstraint>>,
) -> Result<(), ActReferenceError> {
    let ActField::Available(constraints) = field else {
        return Ok(());
    };
    if constraints.len() > ACT_MAX_CONSTRAINTS {
        return Err(ActReferenceError::InvalidInput("constraints"));
    }
    let mut ids = BTreeSet::new();
    for constraint in constraints {
        validate_constraint(constraint)?;
        if !ids.insert(constraint.constraint_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_constraint"));
        }
    }
    Ok(())
}

pub(super) fn validate_variant_ids(ids: &[String]) -> Result<(), ActReferenceError> {
    if ids.len() > ACT_MAX_VARIANTS {
        return Err(ActReferenceError::InvalidInput("variants"));
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        validate_identity(id, "variant_id")?;
        if !seen.insert(id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_variant"));
        }
    }
    Ok(())
}
