// SPDX-License-Identifier: MIT

mod edges;
mod quantities;
mod values;

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentUnlockState;

pub(super) use self::edges::RewardTarget;
use self::edges::{
    RewardScope, collect_items, most_restrictive, validate_item_membership,
    validate_reference_edges,
};
use self::quantities::validate_item_quantity;
use self::values::{
    validate_generation_rule, validate_legal_actions, validate_reference_kind, validate_references,
    validate_state_policy, validate_text_value, validate_visibility,
};

use super::RewardCatalogError;
use super::definition::{RewardItemInput, RewardKind, RewardOfferDefinitionInput};
use super::model::{
    REWARD_MAX_RULES, RewardField, RewardNumericValue, RewardProbability,
    RewardSemanticReferenceKind, RewardVisibility, validate_identity,
};

/// Validates one source-owned reward definition before it enters an immutable catalog.
pub(super) fn validate_definition(
    input: &RewardOfferDefinitionInput,
    reward_targets: &BTreeMap<String, RewardTarget>,
) -> Result<(), RewardCatalogError> {
    validate_identity(&input.reward_id, "reward_id")?;
    validate_text_value(&input.label, "title")?;
    validate_reward_kind(&input.kind)?;
    validate_visibility(input.visibility)?;
    validate_selection(input)?;
    let items = collect_items(&input.items)?;
    for item in &input.items {
        validate_item(item, &input.kind)?;
    }
    validate_rules(&input.generation)?;
    validate_item_membership(&input.reward_id, &input.generation, &items)?;
    validate_state_policy(&input.state_policy)?;

    let local_rules: BTreeMap<&str, RewardVisibility> = input
        .generation
        .iter()
        .map(|rule| (rule.rule_id.as_str(), rule.visibility))
        .collect();
    let mut local_modifiers: BTreeMap<&str, RewardVisibility> = BTreeMap::new();
    for rule in &input.generation {
        let parent = rule.visibility;
        for modifier in &rule.modifiers {
            let effective = most_restrictive(parent, modifier.visibility);
            local_modifiers
                .entry(modifier.modifier_id.as_str())
                .and_modify(|existing| *existing = most_restrictive(*existing, effective))
                .or_insert(effective);
        }
    }

    let scope = RewardScope::new(
        &input.reward_id,
        input.visibility,
        input.unlock_state,
        &items,
    );

    for item in &input.items {
        for value in [&item.quantity.base_amount, &item.quantity.visible_amount] {
            if let RewardNumericValue::Formula(formula) = value {
                reject_local_rule_leak(
                    input,
                    &local_rules,
                    item.visibility,
                    &formula.rule_reference,
                )?;
            }
        }
    }
    for rule in &input.generation {
        if let RewardProbability::Rule { rule_reference, .. }
        | RewardProbability::Conditional { rule_reference, .. } = &rule.probability
        {
            reject_local_rule_leak(input, &local_rules, rule.visibility, rule_reference)?;
        }
        for weight in &rule.rarity_weights {
            for value in [&weight.weight.base_amount, &weight.weight.visible_amount] {
                if let RewardNumericValue::Formula(formula) = value {
                    reject_local_rule_leak(
                        input,
                        &local_rules,
                        rule.visibility,
                        &formula.rule_reference,
                    )?;
                }
            }
            if let RewardProbability::Rule { rule_reference, .. }
            | RewardProbability::Conditional { rule_reference, .. } = &weight.probability
            {
                reject_local_rule_leak(input, &local_rules, rule.visibility, rule_reference)?;
            }
        }
        for requirement in &rule.eligibility {
            let containing = most_restrictive(rule.visibility, requirement.visibility);
            for parameter in &requirement.parameters {
                if let RewardNumericValue::Formula(formula) = &parameter.value {
                    reject_local_rule_leak(
                        input,
                        &local_rules,
                        containing,
                        &formula.rule_reference,
                    )?;
                }
            }
        }
        for modifier in &rule.modifiers {
            let containing = most_restrictive(rule.visibility, modifier.visibility);
            for value in [
                &modifier.amount.base_amount,
                &modifier.amount.visible_amount,
            ] {
                if let RewardNumericValue::Formula(formula) = value {
                    reject_local_rule_leak(
                        input,
                        &local_rules,
                        containing,
                        &formula.rule_reference,
                    )?;
                }
            }
            if let RewardField::Available(rule_reference) = &modifier.rule_reference {
                reject_local_rule_leak(input, &local_rules, containing, rule_reference)?;
            }
        }
    }
    validate_reference_edges(
        &scope,
        reward_targets,
        &local_rules,
        &local_modifiers,
        input.selection.visibility,
        &input.selection.references,
    )?;
    for rule in &input.generation {
        if let RewardField::Available(pool) = &rule.pool {
            validate_reference_edges(
                &scope,
                reward_targets,
                &local_rules,
                &local_modifiers,
                rule.visibility,
                pool,
            )?;
        }
        for requirement in &rule.eligibility {
            validate_reference_edges(
                &scope,
                reward_targets,
                &local_rules,
                &local_modifiers,
                most_restrictive(rule.visibility, requirement.visibility),
                &requirement.references,
            )?;
        }
        for modifier in &rule.modifiers {
            validate_reference_edges(
                &scope,
                reward_targets,
                &local_rules,
                &local_modifiers,
                most_restrictive(rule.visibility, modifier.visibility),
                &modifier.references,
            )?;
        }
        validate_reference_edges(
            &scope,
            reward_targets,
            &local_rules,
            &local_modifiers,
            rule.visibility,
            &rule.references,
        )?;
    }
    for item in &input.items {
        validate_reference_edges(
            &scope,
            reward_targets,
            &local_rules,
            &local_modifiers,
            item.visibility,
            std::slice::from_ref(&item.reference),
        )?;
    }
    validate_references(&input.references)?;
    validate_reference_edges(
        &scope,
        reward_targets,
        &local_rules,
        &local_modifiers,
        input.visibility,
        &input.references,
    )
}

fn validate_reward_kind(kind: &RewardKind) -> Result<(), RewardCatalogError> {
    if let RewardKind::Custom(value) | RewardKind::Unsupported(value) = kind {
        validate_identity(value, "reward_kind")?;
    }
    Ok(())
}

fn validate_selection(input: &RewardOfferDefinitionInput) -> Result<(), RewardCatalogError> {
    let selection = &input.selection;
    validate_identity(&selection.group_id, "selection_group")?;
    validate_text_value(&selection.label, "selection_label")?;
    validate_visibility(selection.visibility)?;
    if let (RewardField::Available(min), RewardField::Available(max)) =
        (&selection.choose_min, &selection.choose_max)
        && min > max
    {
        return Err(RewardCatalogError::InvalidInput("choose_bounds"));
    }
    validate_legal_actions(&selection.legal_actions)?;
    validate_references(&selection.references)
}

fn validate_item(item: &RewardItemInput, kind: &RewardKind) -> Result<(), RewardCatalogError> {
    validate_text_value(&item.label, "item_label")?;
    validate_reference_kind(&item.reference.kind)?;
    validate_identity(&item.reference.id, "item_reference")?;
    validate_text_value(&item.reference.label, "item_reference_label")?;
    validate_item_quantity(item, kind)?;
    if let RewardField::Available(instance) = &item.instance {
        validate_identity(&instance.item_instance_id, "item_instance")?;
        validate_identity(&instance.offer.offer_id, "instance_offer")?;
        validate_identity(&instance.offer.snapshot.run_id, "instance_run")?;
        validate_identity(&instance.offer.snapshot.room_id, "instance_room")?;
        validate_identity(&instance.offer.snapshot.snapshot_id, "instance_snapshot")?;
    }
    validate_visibility(item.visibility)
}

fn validate_rules(
    rules: &[super::definition::RewardGenerationRule],
) -> Result<(), RewardCatalogError> {
    if rules.len() > REWARD_MAX_RULES {
        return Err(RewardCatalogError::InvalidInput("generation"));
    }
    let mut ids = BTreeSet::new();
    for rule in rules {
        validate_generation_rule(rule)?;
        if !ids.insert(rule.rule_id.as_str()) {
            return Err(RewardCatalogError::InvalidInput("duplicate_rule"));
        }
    }
    Ok(())
}

/// Rejects a non-semantic local rule-reference whose target is more restricted than its record.
fn reject_local_rule_leak(
    input: &RewardOfferDefinitionInput,
    local_rules: &BTreeMap<&str, RewardVisibility>,
    containing: RewardVisibility,
    rule_id: &str,
) -> Result<(), RewardCatalogError> {
    if let Some(target_visibility) = local_rules.get(rule_id) {
        let containing_scope =
            reward_containing_scope(input.visibility, input.unlock_state, containing);
        if containing_scope < local_label_scope(*target_visibility) {
            return Err(RewardCatalogError::HiddenReferenceLeak {
                reward_id: input.reward_id.clone(),
                reference_kind: RewardSemanticReferenceKind::Rule,
            });
        }
    }
    Ok(())
}

fn local_label_scope(visibility: RewardVisibility) -> u8 {
    match visibility {
        RewardVisibility::Visible => 0,
        RewardVisibility::OwnerOnly => 2,
        RewardVisibility::Hidden | RewardVisibility::Unknown => 3,
    }
}

fn reward_containing_scope(
    reward_visibility: RewardVisibility,
    unlock_state: ContentUnlockState,
    containing: RewardVisibility,
) -> u8 {
    let reward = match reward_visibility {
        RewardVisibility::Visible => match unlock_state {
            ContentUnlockState::Unlocked => 0,
            ContentUnlockState::Locked => 1,
            ContentUnlockState::Unknown => 3,
        },
        RewardVisibility::OwnerOnly => match unlock_state {
            ContentUnlockState::Unknown => 3,
            ContentUnlockState::Unlocked | ContentUnlockState::Locked => 2,
        },
        RewardVisibility::Hidden | RewardVisibility::Unknown => 3,
    };
    reward.max(local_label_scope(containing))
}
