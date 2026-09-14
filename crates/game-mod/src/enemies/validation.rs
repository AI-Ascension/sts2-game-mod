// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::definition::{EnemyCooldownRule, EnemyMoveEffectKind, EnemyRepetitionRule};
use super::{
    EnemyCatalogError,
    definition::{
        EnemyBehaviorTransition, EnemyConditionReference, EnemyDefinitionInput,
        EnemyEncounterReference, EnemyMoveDefinitionInput, EnemyMoveEffect, EnemyOriginVariant,
        EnemyParameter, EnemySemanticReference, EnemySemanticReferenceKind, EnemyStat,
        EnemyStatProfile,
    },
    model::{
        ENEMY_MAX_EFFECTS, ENEMY_MAX_FORMULA_INPUTS, ENEMY_MAX_MOVES, ENEMY_MAX_ORIGIN_VARIANTS,
        ENEMY_MAX_PARAMETERS, ENEMY_MAX_PHASES, ENEMY_MAX_REFERENCES, ENEMY_MAX_STAT_PROFILES,
        ENEMY_MAX_STATS, ENEMY_MAX_TAGS, EnemyField, EnemyFormula, EnemyKind, EnemyNumericValue,
        EnemyProbability, EnemyTargetDomain, EnemyText, EnemyVisibility, validate_identity,
        validate_text,
    },
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
    if input.transitions.len() > super::model::ENEMY_MAX_TRANSITIONS {
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

fn validate_origin(origin: &super::model::EnemyOrigin) -> Result<(), EnemyCatalogError> {
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

fn validate_tags(tags: &[super::definition::EnemyTag]) -> Result<(), EnemyCatalogError> {
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

fn validate_stats(stats: &super::definition::EnemyStats) -> Result<(), EnemyCatalogError> {
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

fn validate_stats_field(
    field: &EnemyField<super::definition::EnemyStats>,
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

fn validate_conditions_field(
    field: &EnemyField<Vec<EnemyConditionReference>>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(conditions) = field {
        validate_condition_list(conditions)?;
    }
    Ok(())
}

fn validate_condition_field(
    field: &EnemyField<EnemyConditionReference>,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(condition) = field {
        validate_condition(condition)?;
    }
    Ok(())
}

fn validate_condition_list(
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

fn validate_condition(condition: &EnemyConditionReference) -> Result<(), EnemyCatalogError> {
    validate_identity(&condition.condition_id, "condition_id")?;
    validate_text_value(&condition.label, "condition_label")?;
    validate_parameters(&condition.parameters)
}

fn validate_encounters_field(
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

fn validate_move_ids(move_ids: &[String]) -> Result<(), EnemyCatalogError> {
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

fn validate_move(
    movement: &EnemyMoveDefinitionInput,
    phase_ids: &BTreeSet<&str>,
) -> Result<(), EnemyCatalogError> {
    validate_identity(&movement.move_id, "move_id")?;
    validate_text_value(&movement.name, "move_name")?;
    validate_text_value(&movement.description, "move_description")?;
    if movement.effects.is_empty() || movement.effects.len() > ENEMY_MAX_EFFECTS {
        return Err(EnemyCatalogError::InvalidInput("effects"));
    }
    let mut effect_ids = BTreeSet::new();
    for effect in &movement.effects {
        validate_effect(effect)?;
        if !effect_ids.insert(effect.effect_id.as_str()) {
            return Err(EnemyCatalogError::InvalidInput("duplicate_effect"));
        }
    }
    validate_targeting(&movement.targeting)?;
    validate_move_ids(&movement.phase_ids)?;
    for phase_id in &movement.phase_ids {
        if !phase_ids.contains(phase_id.as_str()) {
            return Err(EnemyCatalogError::UnknownPhaseReference {
                enemy_id: String::new(),
                phase_id: phase_id.clone(),
            });
        }
    }
    validate_condition_list(&movement.conditions)?;
    validate_cooldown(&movement.cooldown)?;
    validate_repetition(&movement.repetition)?;
    validate_probability(&movement.probability)?;
    validate_references(&movement.references)?;
    validate_visibility(movement.visibility)?;
    Ok(())
}

fn validate_effect(effect: &EnemyMoveEffect) -> Result<(), EnemyCatalogError> {
    validate_identity(&effect.effect_id, "effect_id")?;
    validate_text_value(&effect.description, "effect_description")?;
    validate_effect_kind(&effect.kind)?;
    validate_targeting(&effect.targeting)?;
    validate_parameters(&effect.parameters)?;
    validate_references(&effect.references)
}

fn validate_effect_kind(kind: &EnemyMoveEffectKind) -> Result<(), EnemyCatalogError> {
    if let EnemyMoveEffectKind::Custom(value) = kind {
        validate_identity(value, "effect_kind")?;
    }
    Ok(())
}

fn validate_targeting(targeting: &super::model::EnemyTargeting) -> Result<(), EnemyCatalogError> {
    validate_numeric(&targeting.count)?;
    if let EnemyNumericValue::Fixed(count) = targeting.count
        && count <= 0
    {
        return Err(EnemyCatalogError::InvalidInput("target_count"));
    }
    if matches!(targeting.domain, EnemyTargetDomain::Unknown) {
        return Ok(());
    }
    if matches!(targeting.domain, EnemyTargetDomain::None)
        && !matches!(
            targeting.count,
            EnemyNumericValue::Fixed(0) | EnemyNumericValue::Unavailable(_)
        )
    {
        return Err(EnemyCatalogError::InvalidInput("target_count"));
    }
    Ok(())
}

fn validate_parameters(parameters: &[EnemyParameter]) -> Result<(), EnemyCatalogError> {
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

fn validate_numeric(value: &EnemyNumericValue) -> Result<(), EnemyCatalogError> {
    if let EnemyNumericValue::Formula(formula) = value {
        validate_formula(formula)?;
    }
    Ok(())
}

fn validate_formula(formula: &EnemyFormula) -> Result<(), EnemyCatalogError> {
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

fn validate_cooldown(rule: &EnemyCooldownRule) -> Result<(), EnemyCatalogError> {
    match rule {
        EnemyCooldownRule::Turns(value) => validate_numeric(value),
        EnemyCooldownRule::UntilCondition(condition) => validate_condition(condition),
        EnemyCooldownRule::Once(_) | EnemyCooldownRule::None | EnemyCooldownRule::Unknown => Ok(()),
    }
}

fn validate_repetition(rule: &EnemyRepetitionRule) -> Result<(), EnemyCatalogError> {
    match rule {
        EnemyRepetitionRule::MaxConsecutive(value) if *value == 0 => {
            Err(EnemyCatalogError::InvalidInput("max_consecutive"))
        }
        EnemyRepetitionRule::Custom(value) => validate_identity(value, "repetition_rule"),
        EnemyRepetitionRule::Allow
        | EnemyRepetitionRule::NoImmediateRepeat
        | EnemyRepetitionRule::MaxConsecutive(_)
        | EnemyRepetitionRule::Unknown => Ok(()),
    }
}

fn validate_probability(probability: &EnemyProbability) -> Result<(), EnemyCatalogError> {
    match probability {
        EnemyProbability::Exact {
            numerator,
            denominator,
            ..
        } => {
            if *denominator == 0 || numerator > denominator {
                return Err(EnemyCatalogError::InvalidInput("probability"));
            }
        }
        EnemyProbability::Formula { formula, .. } => validate_formula(formula)?,
        EnemyProbability::Unavailable(_) => {}
    }
    Ok(())
}

fn validate_references(references: &[EnemySemanticReference]) -> Result<(), EnemyCatalogError> {
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

fn validate_transition(
    transition: &EnemyBehaviorTransition,
    phase_ids: &BTreeSet<&str>,
) -> Result<(), EnemyCatalogError> {
    validate_identity(&transition.transition_id, "transition_id")?;
    if let Some(from_phase) = &transition.from_phase {
        validate_identity(from_phase, "from_phase")?;
        if !phase_ids.contains(from_phase.as_str()) {
            return Err(EnemyCatalogError::UnknownPhaseReference {
                enemy_id: String::new(),
                phase_id: from_phase.clone(),
            });
        }
    }
    validate_identity(&transition.to_phase, "to_phase")?;
    if !phase_ids.contains(transition.to_phase.as_str()) {
        return Err(EnemyCatalogError::UnknownPhaseReference {
            enemy_id: String::new(),
            phase_id: transition.to_phase.clone(),
        });
    }
    validate_condition(&transition.condition)?;
    validate_probability(&transition.probability)?;
    validate_references(&transition.references)
}

fn validate_text_value(value: &EnemyText, field: &'static str) -> Result<(), EnemyCatalogError> {
    if let EnemyText::Available(text) = value {
        validate_text(text, field)?;
    }
    Ok(())
}
