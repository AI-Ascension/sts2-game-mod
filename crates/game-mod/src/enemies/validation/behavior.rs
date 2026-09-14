// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::definition::{
    EnemyBehaviorTransition, EnemyCooldownRule, EnemyMoveDefinitionInput, EnemyMoveEffect,
    EnemyMoveEffectKind, EnemyRepetitionRule,
};
use super::super::model::{
    ENEMY_MAX_EFFECTS, EnemyNumericValue, EnemyProbability, EnemyTargetDomain, EnemyTargeting,
    validate_identity,
};
use super::EnemyCatalogError;
use super::validate_visibility;
use super::values::*;

pub(super) fn validate_move(
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

fn validate_targeting(targeting: &EnemyTargeting) -> Result<(), EnemyCatalogError> {
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

pub(super) fn validate_transition(
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
