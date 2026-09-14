// SPDX-License-Identifier: MIT

mod behavior;
mod values;

use std::collections::BTreeSet;

use self::behavior::{validate_move, validate_transition};
use self::values::{
    validate_condition_field, validate_conditions_field, validate_encounters_field,
    validate_move_ids, validate_references, validate_stats, validate_stats_field, validate_tags,
    validate_text_value,
};
use super::EnemyCatalogError;
use super::definition::{EnemyDefinitionInput, EnemyOriginVariant};
use super::model::{
    ENEMY_MAX_MOVES, ENEMY_MAX_ORIGIN_VARIANTS, ENEMY_MAX_PHASES, ENEMY_MAX_TRANSITIONS,
    EnemyField, EnemyKind, EnemyOrigin, EnemyVisibility, validate_identity,
};

/// Validates one source-owned enemy definition before it enters an immutable catalog.
pub(super) fn validate_definition(input: &EnemyDefinitionInput) -> Result<(), EnemyCatalogError> {
    validate_identity(&input.enemy_id, "enemy_id")?;
    validate_text_value(&input.name, "name")?;
    validate_text_value(&input.description, "description")?;
    validate_kind(&input.kind)?;
    validate_origin(&input.origin)?;
    validate_visibility(input.visibility)?;
    validate_tags(&input.tags)?;
    validate_stats(&input.stats)?;
    validate_conditions_field(&input.spawn_conditions)?;
    validate_encounters_field(&input.encounters)?;
    if input.origin_variants.len() > ENEMY_MAX_ORIGIN_VARIANTS {
        return Err(EnemyCatalogError::InvalidInput("origin_variants"));
    }
    let mut variant_ids = BTreeSet::new();
    for variant in &input.origin_variants {
        validate_origin_variant(variant)?;
        if !variant_ids.insert(variant.variant_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_origin_variant"));
        }
    }
    if input.phases.is_empty() {
        return Err(EnemyCatalogError::InvalidInput("phases"));
    }
    if input.phases.len() > ENEMY_MAX_PHASES {
        return Err(EnemyCatalogError::InvalidInput("phases"));
    }
    let mut phase_ids = BTreeSet::new();
    for phase in &input.phases {
        validate_identity(&phase.phase_id, "phase_id")?;
        validate_text_value(&phase.name, "phase_name")?;
        validate_text_value(&phase.description, "phase_description")?;
        validate_move_ids(&phase.move_ids)?;
        validate_condition_field(&phase.entry_condition)?;
        if !phase_ids.insert(phase.phase_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_phase"));
        }
    }
    if input.moves.is_empty() {
        return Err(EnemyCatalogError::InvalidInput("moves"));
    }
    if input.moves.len() > ENEMY_MAX_MOVES {
        return Err(EnemyCatalogError::InvalidInput("moves"));
    }
    let mut move_ids = BTreeSet::new();
    for movement in &input.moves {
        validate_move(movement, &phase_ids)?;
        if !move_ids.insert(movement.move_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_move"));
        }
    }
    for phase in &input.phases {
        resolve_move_references(&input.enemy_id, &phase.move_ids, &move_ids)?;
    }
    for variant in &input.origin_variants {
        if let EnemyField::Available(variant_moves) = &variant.move_ids {
            resolve_move_references(&input.enemy_id, variant_moves, &move_ids)?;
        }
    }
    if input.transitions.len() > ENEMY_MAX_TRANSITIONS {
        return Err(EnemyCatalogError::InvalidInput("transitions"));
    }
    let mut transition_ids = BTreeSet::new();
    for transition in &input.transitions {
        validate_transition(transition, &phase_ids)?;
        if !transition_ids.insert(transition.transition_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_transition"));
        }
    }
    validate_references(&input.references)?;
    Ok(())
}

fn resolve_move_references(
    enemy_id: &str,
    move_ids: &[String],
    defined: &BTreeSet<&str>,
) -> Result<(), EnemyCatalogError> {
    for move_id in move_ids {
        if !defined.contains(move_id.as_str()) {
            return Err(EnemyCatalogError::UnknownMoveReference {
                enemy_id: enemy_id.to_owned(),
                move_id: move_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_kind(kind: &EnemyKind) -> Result<(), EnemyCatalogError> {
    if let EnemyKind::Custom(value) = kind {
        validate_identity(value, "kind")?;
    }
    Ok(())
}

fn validate_visibility(visibility: EnemyVisibility) -> Result<(), EnemyCatalogError> {
    if matches!(visibility, EnemyVisibility::Unknown) {
        return Err(EnemyCatalogError::InvalidInput("visibility"));
    }
    Ok(())
}

fn validate_origin(origin: &EnemyOrigin) -> Result<(), EnemyCatalogError> {
    validate_identity(&origin.kind, "origin_kind")?;
    if let Some(package_id) = &origin.package_id {
        validate_identity(package_id, "package_id")?;
    }
    if let Some(package_version) = &origin.package_version {
        validate_identity(package_version, "package_version")?;
    }
    if origin.package_id.is_none() && origin.package_version.is_some() {
        return Err(EnemyCatalogError::InvalidInput("origin_package"));
    }
    Ok(())
}

fn validate_origin_variant(variant: &EnemyOriginVariant) -> Result<(), EnemyCatalogError> {
    validate_identity(&variant.variant_id, "variant_id")?;
    validate_text_value(&variant.label, "variant_label")?;
    validate_origin(&variant.origin)?;
    validate_stats_field(&variant.stats)?;
    if let EnemyField::Available(move_ids) = &variant.move_ids {
        validate_move_ids(move_ids)?;
    }
    Ok(())
}
