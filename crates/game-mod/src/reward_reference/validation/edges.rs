// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::RewardCatalogError;
use super::super::definition::{RewardGenerationRule, RewardItemInput};
use super::super::model::{
    REWARD_MAX_ITEMS, REWARD_REFERENCE_ENTITY_KIND, RewardField, RewardSemanticReference,
    RewardSemanticReferenceKind, RewardVisibility, validate_identity, visibility_rank,
};

/// Shallow reward context shared by scope-aware reference and membership checks.
pub(super) struct RewardScope<'a> {
    reward_id: &'a str,
    items: &'a BTreeMap<&'a str, RewardVisibility>,
}

impl<'a> RewardScope<'a> {
    pub(super) fn new(reward_id: &'a str, items: &'a BTreeMap<&'a str, RewardVisibility>) -> Self {
        Self { reward_id, items }
    }
}

/// Collects item identities with their visibility, rejecting duplicates.
pub(super) fn collect_items(
    items: &[RewardItemInput],
) -> Result<BTreeMap<&str, RewardVisibility>, RewardCatalogError> {
    if items.len() > REWARD_MAX_ITEMS {
        return Err(RewardCatalogError::InvalidInput("items"));
    }
    let mut identities = BTreeMap::new();
    for item in items {
        validate_identity(&item.item_id, "item_id")?;
        if identities
            .insert(item.item_id.as_str(), item.visibility)
            .is_some()
        {
            return Err(RewardCatalogError::InvalidInput("duplicate_item"));
        }
    }
    Ok(identities)
}

/// Validates the generation-rule-to-item membership relation.
///
/// Every offered item must be named by exactly one generation-rule pool, no rule may offer the
/// same item twice, and no item may remain uncovered. A rule more visible than an item it offers
/// is rejected as a hidden-future leak, so a visible rule never advertises a hidden item.
pub(super) fn validate_item_membership(
    reward_id: &str,
    rules: &[RewardGenerationRule],
    items: &BTreeMap<&str, RewardVisibility>,
) -> Result<(), RewardCatalogError> {
    let mut offered_by: BTreeMap<&str, &str> = BTreeMap::new();
    for rule in rules {
        let RewardField::Available(pool) = &rule.pool else {
            continue;
        };
        for reference in pool {
            if !matches!(reference.kind, RewardSemanticReferenceKind::Item) {
                continue;
            }
            let Some(item_visibility) = items.get(reference.id.as_str()) else {
                return Err(RewardCatalogError::UnknownItemReference {
                    reward_id: reward_id.to_owned(),
                    item_id: reference.id.clone(),
                });
            };
            if visibility_rank(rule.visibility) > visibility_rank(*item_visibility) {
                return Err(RewardCatalogError::HiddenFutureLeak {
                    reward_id: reward_id.to_owned(),
                    item_id: reference.id.clone(),
                });
            }
            if offered_by
                .insert(reference.id.as_str(), rule.rule_id.as_str())
                .is_some()
            {
                return Err(RewardCatalogError::DuplicateItemMembership {
                    reward_id: reward_id.to_owned(),
                    item_id: reference.id.clone(),
                });
            }
        }
    }
    for item_id in items.keys() {
        if !offered_by.contains_key(item_id) {
            return Err(RewardCatalogError::UncoveredItem {
                reward_id: reward_id.to_owned(),
                item_id: (*item_id).to_owned(),
            });
        }
    }
    Ok(())
}

/// Validates one list of semantic reference edges against target visibility.
///
/// A record more visible than its target is rejected: a visible record must never disclose a
/// hidden or owner-only target identity or label.
pub(super) fn validate_reference_edges(
    scope: &RewardScope<'_>,
    reward_visibility: &BTreeMap<String, RewardVisibility>,
    containing: RewardVisibility,
    references: &[RewardSemanticReference],
) -> Result<(), RewardCatalogError> {
    for reference in references {
        if let Some(canonical) = canonical_reserved_family_alias(&reference.kind) {
            reject_more_visible_reward(
                scope.reward_id,
                reward_visibility,
                containing,
                &reference.id,
                &canonical,
            )?;
            continue;
        }
        match &reference.kind {
            RewardSemanticReferenceKind::Reward => {
                reject_more_visible_reward(
                    scope.reward_id,
                    reward_visibility,
                    containing,
                    &reference.id,
                    &reference.kind,
                )?;
            }
            RewardSemanticReferenceKind::Item => {
                let Some(target) = scope.items.get(reference.id.as_str()) else {
                    return Err(RewardCatalogError::UnknownItemReference {
                        reward_id: scope.reward_id.to_owned(),
                        item_id: reference.id.clone(),
                    });
                };
                reject_more_visible(scope.reward_id, containing, *target, &reference.kind)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Normalizes a reserved-family generic spelling to its canonical reference kind.
///
/// A [`RewardSemanticReferenceKind::Content`] whose `entity_kind` names the reward family is an
/// alias for the canonical [`RewardSemanticReferenceKind::Reward`] family. It must therefore be
/// subject to the same visibility enforcement as an explicit `Reward` edge rather than falling
/// through.
fn canonical_reserved_family_alias(
    kind: &RewardSemanticReferenceKind,
) -> Option<RewardSemanticReferenceKind> {
    match kind {
        RewardSemanticReferenceKind::Content { entity_kind }
            if entity_kind == REWARD_REFERENCE_ENTITY_KIND =>
        {
            Some(RewardSemanticReferenceKind::Reward)
        }
        _ => None,
    }
}

fn reject_more_visible_reward(
    reward_id: &str,
    reward_visibility: &BTreeMap<String, RewardVisibility>,
    containing: RewardVisibility,
    target_id: &str,
    kind: &RewardSemanticReferenceKind,
) -> Result<(), RewardCatalogError> {
    if let Some(target) = reward_visibility.get(target_id) {
        reject_more_visible(reward_id, containing, *target, kind)?;
    }
    Ok(())
}

fn reject_more_visible(
    reward_id: &str,
    containing: RewardVisibility,
    target: RewardVisibility,
    kind: &RewardSemanticReferenceKind,
) -> Result<(), RewardCatalogError> {
    if visibility_rank(containing) > visibility_rank(target) {
        return Err(RewardCatalogError::HiddenReferenceLeak {
            reward_id: reward_id.to_owned(),
            reference_kind: kind.clone(),
        });
    }
    Ok(())
}
