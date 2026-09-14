// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::definition::{
    EnemyConditionReference, EnemyEncounterReference, EnemyParameter, EnemySemanticReference,
    EnemySemanticReferenceKind, EnemyStat, EnemyStatProfile, EnemyStats, EnemyTag,
};
use super::super::model::{
    ENEMY_MAX_FORMULA_INPUTS, ENEMY_MAX_MOVES, ENEMY_MAX_PARAMETERS, ENEMY_MAX_REFERENCES,
    ENEMY_MAX_STAT_PROFILES, ENEMY_MAX_STATS, ENEMY_MAX_TAGS, EnemyField, EnemyFormula,
    EnemyNumericValue, EnemyText, validate_identity, validate_text,
};
use super::EnemyCatalogError;

pub(super) fn validate_tags(tags: &[EnemyTag]) -> Result<(), EnemyCatalogError> {
    if tags.len() > ENEMY_MAX_TAGS {
        return Err(EnemyCatalogError::InvalidInput("tags"));
    }
    let mut ids = BTreeSet::new();
    for tag in tags {
        validate_identity(&tag.tag_id, "tag_id")?;
        validate_text_value(&tag.label, "tag_label")?;
        if !ids.insert(tag.tag_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_tag"));
        }
    }
    Ok(())
}

pub(super) fn validate_stats(stats: &EnemyStats) -> Result<(), EnemyCatalogError> {
    validate_stat_field(&stats.base)?;
    if let EnemyField::Available(profiles) = &stats.scaled {
        if profiles.len() > ENEMY_MAX_STAT_PROFILES {
            return Err(EnemyCatalogError::InvalidInput("scaled_profiles"));
        }
        let mut ids = BTreeSet::new();
        for profile in profiles {
            validate_profile(profile)?;
            if !ids.insert(profile.profile_id.as_str()) {
                return Err(EnemyCatalogError::InvalidInput("duplicate_stat_profile"));
            }
        }
    }
    Ok(())
}

fn validate_stat_field(field: &EnemyField<Vec<EnemyStat>>) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(stats) = field {
        validate_stat_list(stats)?;
    }
    Ok(())
}

pub(super) fn validate_stats_field(
    field: &EnemyField<EnemyStats>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(stats) = field {
        validate_stats(stats)?;
    }
    Ok(())
}

fn validate_profile(profile: &EnemyStatProfile) -> Result<(), EnemyCatalogError> {
    validate_identity(&profile.profile_id, "profile_id")?;
    if let Some(mode) = &profile.mode {
        validate_identity(mode, "mode")?;
    }
    if let Some(difficulty) = &profile.difficulty {
        validate_identity(difficulty, "difficulty")?;
    }
    validate_stat_list(&profile.stats)
}

fn validate_stat_list(stats: &[EnemyStat]) -> Result<(), EnemyCatalogError> {
    if stats.len() > ENEMY_MAX_STATS {
        return Err(EnemyCatalogError::InvalidInput("stats"));
    }
    let mut ids = BTreeSet::new();
    for stat in stats {
        validate_identity(&stat.stat_id, "stat_id")?;
        if let Some(unit) = &stat.unit {
            validate_identity(unit, "stat_unit")?;
        }
        validate_numeric(&stat.value)?;
        if !ids.insert(stat.stat_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_stat"));
        }
    }
    Ok(())
}

pub(super) fn validate_conditions_field(
    field: &EnemyField<Vec<EnemyConditionReference>>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(conditions) = field {
        validate_condition_list(conditions)?;
    }
    Ok(())
}

pub(super) fn validate_condition_field(
    field: &EnemyField<EnemyConditionReference>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(condition) = field {
        validate_condition(condition)?;
    }
    Ok(())
}

pub(super) fn validate_condition_list(
    conditions: &[EnemyConditionReference],
) -> Result<(), EnemyCatalogError> {
    if conditions.len() > ENEMY_MAX_REFERENCES {
        return Err(EnemyCatalogError::InvalidInput("conditions"));
    }
    let mut ids = BTreeSet::new();
    for condition in conditions {
        validate_condition(condition)?;
        if !ids.insert(condition.condition_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_condition"));
        }
    }
    Ok(())
}

pub(super) fn validate_condition(
    condition: &EnemyConditionReference,
) -> Result<(), EnemyCatalogError> {
    validate_identity(&condition.condition_id, "condition_id")?;
    validate_text_value(&condition.label, "condition_label")?;
    validate_parameters(&condition.parameters)
}

pub(super) fn validate_encounters_field(
    field: &EnemyField<Vec<EnemyEncounterReference>>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(encounters) = field {
        if encounters.len() > ENEMY_MAX_REFERENCES {
            return Err(EnemyCatalogError::InvalidInput("encounters"));
        }
        let mut ids = BTreeSet::new();
        for encounter in encounters {
            validate_encounter(encounter)?;
            if !ids.insert(encounter.encounter_id.as_str()) {
                return Err(EnemyCatalogError::InvalidInput("duplicate_encounter"));
            }
        }
    }
    Ok(())
}

fn validate_encounter(encounter: &EnemyEncounterReference) -> Result<(), EnemyCatalogError> {
    validate_identity(&encounter.encounter_id, "encounter_id")?;
    validate_text_value(&encounter.label, "encounter_label")?;
    if let Some(role) = &encounter.role {
        validate_identity(role, "encounter_role")?;
    }
    Ok(())
}

pub(super) fn validate_move_ids(move_ids: &[String]) -> Result<(), EnemyCatalogError> {
    if move_ids.len() > ENEMY_MAX_MOVES {
        return Err(EnemyCatalogError::InvalidInput("move_ids"));
    }
    let mut ids = BTreeSet::new();
    for move_id in move_ids {
        validate_identity(move_id, "move_id")?;
        if !ids.insert(move_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_move_id"));
        }
    }
    Ok(())
}

pub(super) fn validate_parameters(parameters: &[EnemyParameter]) -> Result<(), EnemyCatalogError> {
    if parameters.len() > ENEMY_MAX_PARAMETERS {
        return Err(EnemyCatalogError::InvalidInput("parameters"));
    }
    let mut ids = BTreeSet::new();
    for parameter in parameters {
        validate_parameter(parameter)?;
        if !ids.insert(parameter.parameter_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_parameter"));
        }
    }
    Ok(())
}

fn validate_parameter(parameter: &EnemyParameter) -> Result<(), EnemyCatalogError> {
    validate_identity(&parameter.parameter_id, "parameter_id")?;
    validate_text_value(&parameter.label, "parameter_label")?;
    if let Some(unit) = &parameter.unit {
        validate_identity(unit, "parameter_unit")?;
    }
    validate_numeric(&parameter.value)
}

pub(super) fn validate_numeric(value: &EnemyNumericValue) -> Result<(), EnemyCatalogError> {
    if let EnemyNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &EnemyFormula) -> Result<(), EnemyCatalogError> {
    validate_identity(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > ENEMY_MAX_FORMULA_INPUTS {
        return Err(EnemyCatalogError::InvalidInput("formula_inputs"));
    }
    let mut ids = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input")?;
        if !ids.insert(input.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_formula_input"));
        }
    }
    Ok(())
}

pub(super) fn validate_references(
    references: &[EnemySemanticReference],
) -> Result<(), EnemyCatalogError> {
    if references.len() > ENEMY_MAX_REFERENCES {
        return Err(EnemyCatalogError::InvalidInput("references"));
    }
    let mut ids = BTreeSet::new();
    for reference in references {
        validate_semantic_kind(&reference.kind)?;
        validate_identity(&reference.id, "reference_id")?;
        validate_text_value(&reference.label, "reference_label")?;
        if !ids.insert((&reference.kind, reference.id.as_str())) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_reference"));
        }
    }
    Ok(())
}

fn validate_semantic_kind(kind: &EnemySemanticReferenceKind) -> Result<(), EnemyCatalogError> {
    if let EnemySemanticReferenceKind::Content { entity_kind } = kind {
        validate_identity(entity_kind, "reference_entity_kind")?;
    }
    Ok(())
}

pub(super) fn validate_text_value(
    value: &EnemyText,
    field: &'static str,
) -> Result<(), EnemyCatalogError> {
    if let EnemyText::Available(text) = value {
        validate_text(text, field)?;
    }
    Ok(())
}
