// SPDX-License-Identifier: MIT

mod edges;
mod quantities;
mod values;

use std::collections::{BTreeMap, BTreeSet};

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
use super::model::{REWARD_MAX_RULES, RewardField, RewardVisibility, validate_identity};

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
