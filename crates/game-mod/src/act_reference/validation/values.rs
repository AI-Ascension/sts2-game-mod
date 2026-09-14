// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::ActReferenceError;
use super::super::definition::{
    ActParameter, EligibilityCondition, EligibilityKind, EncounterEnemy, EncounterEnemyGroup,
    EncounterPoolEntry, MapConstraintKind, MapGenerationConstraint, RoomCategoryDefinition,
    RoomCategoryKind,
};
use super::super::model::{
    ACT_MAX_FORMULA_INPUTS, ACT_MAX_PARAMETERS, ACT_MAX_REFERENCES, ActField, ActFormula,
    ActNumericValue, ActSemanticReference, ActSemanticReferenceKind, ActText, ActVisibility,
    GenerationWeight, validate_identity, validate_text,
};

pub(super) fn validate_text_value(
    value: &ActText,
    field: &'static str,
) -> Result<(), ActReferenceError> {
    if let ActText::Available(text) = value {
        validate_text(text, field)?;
    }
    Ok(())
}

pub(super) fn validate_visibility(visibility: ActVisibility) -> Result<(), ActReferenceError> {
    if matches!(visibility, ActVisibility::Unknown) {
        return Err(ActReferenceError::InvalidInput("visibility"));
    }
    Ok(())
}

pub(super) fn validate_references(
    references: &[ActSemanticReference],
) -> Result<(), ActReferenceError> {
    if references.len() > ACT_MAX_REFERENCES {
        return Err(ActReferenceError::InvalidInput("references"));
    }
    let mut ids = BTreeSet::new();
    for reference in references {
        validate_reference_kind(&reference.kind)?;
        validate_identity(&reference.id, "reference_id")?;
        validate_text_value(&reference.label, "reference_label")?;
        if !ids.insert((&reference.kind, reference.id.as_str())) {
            return Err(ActReferenceError::InvalidInput("duplicate_reference"));
        }
    }
    Ok(())
}

fn validate_reference_kind(kind: &ActSemanticReferenceKind) -> Result<(), ActReferenceError> {
    if let ActSemanticReferenceKind::Content { entity_kind } = kind {
        validate_identity(entity_kind, "reference_entity_kind")?;
    }
    Ok(())
}

pub(super) fn validate_parameters(parameters: &[ActParameter]) -> Result<(), ActReferenceError> {
    if parameters.len() > ACT_MAX_PARAMETERS {
        return Err(ActReferenceError::InvalidInput("parameters"));
    }
    let mut ids = BTreeSet::new();
    for parameter in parameters {
        validate_identity(&parameter.parameter_id, "parameter_id")?;
        validate_text_value(&parameter.label, "parameter_label")?;
        if let Some(unit) = &parameter.unit {
            validate_identity(unit, "parameter_unit")?;
        }
        validate_numeric(&parameter.value)?;
        if !ids.insert(parameter.parameter_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_parameter"));
        }
    }
    Ok(())
}

pub(super) fn validate_numeric(value: &ActNumericValue) -> Result<(), ActReferenceError> {
    if let ActNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &ActFormula) -> Result<(), ActReferenceError> {
    validate_identity(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > ACT_MAX_FORMULA_INPUTS {
        return Err(ActReferenceError::InvalidInput("formula_inputs"));
    }
    let mut ids = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input")?;
        if !ids.insert(input.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_formula_input"));
        }
    }
    Ok(())
}

pub(super) fn validate_weight(weight: &GenerationWeight) -> Result<(), ActReferenceError> {
    match weight {
        GenerationWeight::Exact {
            numerator,
            denominator,
            ..
        } => {
            if *denominator == 0 || numerator > denominator {
                return Err(ActReferenceError::InvalidInput("weight"));
            }
        }
        GenerationWeight::Rule { rule_reference, .. } => {
            validate_identity(rule_reference, "weight_rule")?;
        }
        GenerationWeight::Unavailable(_) => {}
    }
    Ok(())
}

pub(super) fn validate_room_category(
    category: &RoomCategoryDefinition,
) -> Result<(), ActReferenceError> {
    validate_identity(&category.category_id, "category_id")?;
    validate_text_value(&category.name, "category_name")?;
    validate_room_kind(&category.kind)?;
    validate_text_value(&category.description, "category_description")?;
    validate_references(&category.references)?;
    validate_visibility(category.visibility)
}

fn validate_room_kind(kind: &RoomCategoryKind) -> Result<(), ActReferenceError> {
    if let RoomCategoryKind::Custom(value) | RoomCategoryKind::Unsupported(value) = kind {
        validate_identity(value, "category_kind")?;
    }
    Ok(())
}

pub(super) fn validate_eligibility(
    condition: &EligibilityCondition,
) -> Result<(), ActReferenceError> {
    validate_identity(&condition.condition_id, "condition_id")?;
    if let EligibilityKind::Custom(value) = &condition.kind {
        validate_identity(value, "condition_kind")?;
    }
    validate_text_value(&condition.label, "condition_label")?;
    validate_parameters(&condition.parameters)?;
    validate_references(&condition.references)?;
    validate_visibility(condition.visibility)
}

pub(super) fn validate_enemy(enemy: &EncounterEnemy) -> Result<(), ActReferenceError> {
    validate_identity(&enemy.enemy_id, "enemy_id")?;
    validate_numeric(&enemy.quantity)?;
    if let ActNumericValue::Fixed(quantity) = enemy.quantity
        && quantity < 1
    {
        return Err(ActReferenceError::InvalidInput("enemy_quantity"));
    }
    super::validate_variant_ids(&enemy.variant_ids)?;
    validate_references(&enemy.references)
}

pub(super) fn validate_group(group: &EncounterEnemyGroup) -> Result<(), ActReferenceError> {
    validate_identity(&group.group_id, "group_id")?;
    validate_text_value(&group.label, "group_label")?;
    if group.enemies.is_empty() {
        return Err(ActReferenceError::InvalidInput("enemies"));
    }
    let mut ids = BTreeSet::new();
    for enemy in &group.enemies {
        validate_enemy(enemy)?;
        if !ids.insert(enemy.enemy_id.as_str()) {
            return Err(ActReferenceError::InvalidInput("duplicate_enemy"));
        }
    }
    validate_references(&group.references)
}

pub(super) fn validate_pool_entry(entry: &EncounterPoolEntry) -> Result<(), ActReferenceError> {
    validate_identity(&entry.entry_id, "entry_id")?;
    validate_identity(&entry.encounter_id, "encounter_id")?;
    validate_weight(&entry.weight)
}

pub(super) fn validate_constraint(
    constraint: &MapGenerationConstraint,
) -> Result<(), ActReferenceError> {
    validate_identity(&constraint.constraint_id, "constraint_id")?;
    validate_constraint_kind(&constraint.kind)?;
    validate_text_value(&constraint.label, "constraint_label")?;
    if let ActField::Available(rule) = &constraint.rule_reference {
        validate_identity(rule, "constraint_rule")?;
    }
    if let Some(mode) = &constraint.mode {
        validate_identity(mode, "constraint_mode")?;
    }
    if let Some(difficulty) = &constraint.difficulty {
        validate_identity(difficulty, "constraint_difficulty")?;
    }
    validate_parameters(&constraint.parameters)?;
    validate_references(&constraint.references)?;
    validate_visibility(constraint.visibility)
}

fn validate_constraint_kind(kind: &MapConstraintKind) -> Result<(), ActReferenceError> {
    if let MapConstraintKind::Custom(value) = kind {
        validate_identity(value, "constraint_kind")?;
    }
    Ok(())
}
