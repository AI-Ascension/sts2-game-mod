// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::EventCatalogError;
use super::super::definition::{
    EventCost, EventCostKind, EventEffect, EventEffectKind, EventParameter, EventRequirement,
    EventRequirementKind,
};
use super::super::model::{
    EVENT_MAX_EFFECTS, EVENT_MAX_FORMULA_INPUTS, EVENT_MAX_PARAMETERS, EVENT_MAX_REFERENCES,
    EventField, EventFormula, EventNumericValue, EventProbability, EventSemanticReference,
    EventSemanticReferenceKind, EventText, EventVisibility, validate_identity, validate_text,
};

pub(super) fn validate_text_value(
    value: &EventText,
    field: &'static str,
) -> Result<(), EventCatalogError> {
    if let EventText::Available(text) = value {
        validate_text(text, field)?;
    }
    Ok(())
}

pub(super) fn validate_visibility(visibility: EventVisibility) -> Result<(), EventCatalogError> {
    if matches!(visibility, EventVisibility::Unknown) {
        return Err(EventCatalogError::InvalidInput("visibility"));
    }
    Ok(())
}

pub(super) fn validate_references(
    references: &[EventSemanticReference],
) -> Result<(), EventCatalogError> {
    if references.len() > EVENT_MAX_REFERENCES {
        return Err(EventCatalogError::InvalidInput("references"));
    }
    let mut ids = BTreeSet::new();
    for reference in references {
        validate_reference_kind(&reference.kind)?;
        validate_identity(&reference.id, "reference_id")?;
        validate_text_value(&reference.label, "reference_label")?;
        if !ids.insert((&reference.kind, reference.id.as_str())) {
            return Err(EventCatalogError::InvalidInput("duplicate_reference"));
        }
    }
    Ok(())
}

fn validate_reference_kind(kind: &EventSemanticReferenceKind) -> Result<(), EventCatalogError> {
    if let EventSemanticReferenceKind::Content { entity_kind } = kind {
        validate_identity(entity_kind, "reference_entity_kind")?;
    }
    Ok(())
}

pub(super) fn validate_parameters(parameters: &[EventParameter]) -> Result<(), EventCatalogError> {
    if parameters.len() > EVENT_MAX_PARAMETERS {
        return Err(EventCatalogError::InvalidInput("parameters"));
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
            return Err(EventCatalogError::InvalidInput("duplicate_parameter"));
        }
    }
    Ok(())
}

pub(super) fn validate_numeric(value: &EventNumericValue) -> Result<(), EventCatalogError> {
    if let EventNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &EventFormula) -> Result<(), EventCatalogError> {
    validate_identity(&formula.rule_reference, "formula_rule")?;
    if formula.unresolved_inputs.len() > EVENT_MAX_FORMULA_INPUTS {
        return Err(EventCatalogError::InvalidInput("formula_inputs"));
    }
    let mut ids = BTreeSet::new();
    for input in &formula.unresolved_inputs {
        validate_identity(input, "formula_input")?;
        if !ids.insert(input.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_formula_input"));
        }
    }
    Ok(())
}

pub(super) fn validate_probability(
    probability: &EventProbability,
) -> Result<(), EventCatalogError> {
    match probability {
        EventProbability::Exact {
            numerator,
            denominator,
            ..
        } => {
            if *denominator == 0 || numerator > denominator {
                return Err(EventCatalogError::InvalidInput("probability"));
            }
        }
        EventProbability::Rule { rule_reference, .. } => {
            validate_identity(rule_reference, "probability_rule")?;
        }
        EventProbability::Unavailable(_) => {}
    }
    Ok(())
}

fn validate_optional_field(
    field: &EventField<String>,
    name: &'static str,
) -> Result<(), EventCatalogError> {
    if let EventField::Available(value) = field {
        validate_identity(value, name)?;
    }
    Ok(())
}

pub(super) fn validate_requirement(
    requirement: &EventRequirement,
) -> Result<(), EventCatalogError> {
    validate_identity(&requirement.requirement_id, "requirement_id")?;
    if let EventRequirementKind::Custom(value) = &requirement.kind {
        validate_identity(value, "requirement_kind")?;
    }
    validate_text_value(&requirement.label, "requirement_label")?;
    validate_parameters(&requirement.parameters)?;
    validate_references(&requirement.references)?;
    validate_visibility(requirement.visibility)
}

pub(super) fn validate_cost(cost: &EventCost) -> Result<(), EventCatalogError> {
    validate_identity(&cost.cost_id, "cost_id")?;
    if let EventCostKind::Custom(value) | EventCostKind::Unsupported(value) = &cost.kind {
        validate_identity(value, "cost_kind")?;
    }
    validate_text_value(&cost.label, "cost_label")?;
    validate_numeric(&cost.amount)?;
    validate_optional_field(&cost.resource, "cost_resource")?;
    validate_optional_field(&cost.rule_reference, "cost_rule")?;
    validate_references(&cost.references)?;
    validate_visibility(cost.visibility)
}

pub(super) fn validate_effect(effect: &EventEffect) -> Result<(), EventCatalogError> {
    validate_identity(&effect.effect_id, "effect_id")?;
    if let EventEffectKind::Custom(value) | EventEffectKind::Unsupported(value) = &effect.kind {
        validate_identity(value, "effect_kind")?;
    }
    validate_text_value(&effect.label, "effect_label")?;
    validate_numeric(&effect.amount)?;
    validate_optional_field(&effect.target, "effect_target")?;
    validate_optional_field(&effect.rule_reference, "effect_rule")?;
    validate_references(&effect.references)?;
    validate_visibility(effect.visibility)
}

/// Validates a unique bounded collection of effects.
pub(super) fn validate_effects(effects: &[EventEffect]) -> Result<(), EventCatalogError> {
    if effects.len() > EVENT_MAX_EFFECTS {
        return Err(EventCatalogError::InvalidInput("effects"));
    }
    let mut ids = BTreeSet::new();
    for effect in effects {
        validate_effect(effect)?;
        if !ids.insert(effect.effect_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_effect"));
        }
    }
    Ok(())
}
