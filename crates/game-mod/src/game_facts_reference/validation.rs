// SPDX-License-Identifier: MIT

//! Fail-closed validation of one owner inventory before it becomes immutable.

use std::collections::BTreeSet;

use super::GameFactsError;
use super::identity::validate_opaque_identity;
use super::model::{
    FactsBuildBinding, FactsRepresentation, FactsRuleEntry, FactsUnsupportedCombination,
    GAME_FACTS_MAX_COMBINATION_MEMBERS, GAME_FACTS_MAX_INPUTS_PER_RULE, GAME_FACTS_MAX_LABEL_BYTES,
    GAME_FACTS_MAX_RULES, GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS,
};

/// Validates one owner inventory before it enters an immutable value.
pub(super) fn validate_inventory(
    build: &FactsBuildBinding,
    representation: &FactsRepresentation,
    rules: &[FactsRuleEntry],
    unsupported: &[FactsUnsupportedCombination],
) -> Result<(), GameFactsError> {
    validate_opaque_identity(&build.build_id, "build_id")?;
    validate_opaque_identity(&build.mode_id, "mode_id")?;
    validate_opaque_identity(&representation.version, "representation_version")?;
    validate_opaque_identity(&representation.encoding, "representation_encoding")?;

    if rules.is_empty() {
        return Err(GameFactsError::EmptyPresentCollection("rules"));
    }
    require_within(rules.len(), GAME_FACTS_MAX_RULES)?;

    let mut declared = BTreeSet::new();
    for rule in rules {
        validate_rule(rule, &mut declared)?;
    }

    if unsupported.len() > GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS {
        return Err(GameFactsError::InventoryTooLarge {
            limit: GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS,
            actual: unsupported.len(),
        });
    }
    for (index, combination) in unsupported.iter().enumerate() {
        validate_combination(index, combination, &declared)?;
    }
    Ok(())
}

/// Validates one rule and records its id among the declared rules.
fn validate_rule<'a>(
    rule: &'a FactsRuleEntry,
    declared: &mut BTreeSet<&'a str>,
) -> Result<(), GameFactsError> {
    validate_opaque_identity(&rule.rule_id, "rule_id")?;
    if !declared.insert(rule.rule_id.as_str()) {
        return Err(GameFactsError::DuplicateRule(rule.rule_id.clone()));
    }
    if rule.inputs.is_empty() {
        return Err(GameFactsError::EmptyPresentCollection("inputs"));
    }
    require_within(rule.inputs.len(), GAME_FACTS_MAX_INPUTS_PER_RULE)?;
    let mut names = BTreeSet::new();
    for input in &rule.inputs {
        validate_opaque_identity(&input.name, "input_name")?;
        validate_opaque_identity(&input.unit, "input_unit")?;
        if !names.insert(input.name.as_str()) {
            return Err(GameFactsError::DuplicateInput {
                rule_id: rule.rule_id.clone(),
                name: input.name.clone(),
            });
        }
    }
    Ok(())
}

/// Validates one declared unsupported combination against the declared rules.
fn validate_combination(
    index: usize,
    combination: &FactsUnsupportedCombination,
    declared: &BTreeSet<&str>,
) -> Result<(), GameFactsError> {
    if combination.rule_ids.len() < 2 {
        return Err(GameFactsError::InvalidInput("unsupported_combination"));
    }
    require_within(
        combination.rule_ids.len(),
        GAME_FACTS_MAX_COMBINATION_MEMBERS,
    )?;
    if combination.reason.is_empty() || combination.reason.len() > GAME_FACTS_MAX_LABEL_BYTES {
        return Err(GameFactsError::InvalidInput("unsupported_reason"));
    }
    let mut members = BTreeSet::new();
    for rule_id in &combination.rule_ids {
        validate_opaque_identity(rule_id, "unsupported_rule_id")?;
        if !declared.contains(rule_id.as_str()) {
            return Err(GameFactsError::UnknownRuleReference {
                combination: index,
                rule_id: rule_id.clone(),
            });
        }
        if !members.insert(rule_id.as_str()) {
            return Err(GameFactsError::InvalidInput(
                "unsupported_combination_member",
            ));
        }
    }
    Ok(())
}

/// Refuses a measured count that exceeds a local bound.
fn require_within(actual: usize, limit: usize) -> Result<(), GameFactsError> {
    if actual <= limit {
        Ok(())
    } else {
        Err(GameFactsError::InventoryTooLarge { limit, actual })
    }
}
