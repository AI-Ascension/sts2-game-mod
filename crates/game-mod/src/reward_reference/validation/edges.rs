// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::ContentUnlockState;

use super::super::RewardCatalogError;
use super::super::definition::{RewardGenerationRule, RewardItemInput};
use super::super::model::{
    REWARD_MAX_ITEMS, REWARD_REFERENCE_ENTITY_KIND, RewardField, RewardSemanticReference,
    RewardSemanticReferenceKind, RewardVisibility, validate_identity, visibility_rank,
};

/// Resolved visibility and unlock state of a reward-family target.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RewardTarget {
    pub(crate) visibility: RewardVisibility,
    pub(crate) unlock_state: ContentUnlockState,
}

/// Shallow reward context shared by scope-aware reference and membership checks.
pub(super) struct RewardScope<'a> {
    reward_id: &'a str,
    visibility: RewardVisibility,
    unlock_state: ContentUnlockState,
    items: &'a BTreeMap<&'a str, RewardVisibility>,
}

impl<'a> RewardScope<'a> {
    pub(super) fn new(
        reward_id: &'a str,
        visibility: RewardVisibility,
        unlock_state: ContentUnlockState,
        items: &'a BTreeMap<&'a str, RewardVisibility>,
    ) -> Self {
        Self {
            reward_id,
            visibility,
            unlock_state,
            items,
        }
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

/// Validates one list of semantic reference edges against target effective scope.
///
/// A record observable in a less restrictive scope than its target is rejected: a public record
/// must never disclose a hidden, owner-only, or locked target identity or label. Reserved-family
/// alias spellings are normalized to the canonical reward kind first, and every pool, selection,
/// requirement, modifier, rule, item, and top-level reference uses this same check.
///
/// `local_rules` and `local_modifiers` map the definition's own generation-rule and modifier
/// identities to their visibility, so a `Rule` or `Modifier` edge that resolves to a restricted
/// local record is rejected in place of falling through. An identity that does not resolve locally
/// is treated as an external reference with no local visibility metadata.
pub(super) fn validate_reference_edges(
    scope: &RewardScope<'_>,
    reward_targets: &BTreeMap<String, RewardTarget>,
    local_rules: &BTreeMap<&str, RewardVisibility>,
    local_modifiers: &BTreeMap<&str, RewardVisibility>,
    containing: RewardVisibility,
    references: &[RewardSemanticReference],
) -> Result<(), RewardCatalogError> {
    let containing_scope = containing_min_scope(scope, containing);
    for reference in references {
        if let Some(canonical) = canonical_reserved_family_alias(&reference.kind) {
            reject_more_visible_reward(
                scope,
                reward_targets,
                containing_scope,
                &reference.id,
                &canonical,
            )?;
            continue;
        }
        match &reference.kind {
            RewardSemanticReferenceKind::Reward => {
                reject_more_visible_reward(
                    scope,
                    reward_targets,
                    containing_scope,
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
                reject_more_visible(
                    scope.reward_id,
                    containing_scope,
                    label_min_scope(*target),
                    &reference.kind,
                )?;
            }
            RewardSemanticReferenceKind::Rule => {
                if let Some(target_visibility) = local_rules.get(reference.id.as_str()) {
                    reject_more_visible(
                        scope.reward_id,
                        containing_scope,
                        label_min_scope(*target_visibility),
                        &reference.kind,
                    )?;
                }
            }
            RewardSemanticReferenceKind::Modifier => {
                if let Some(target_visibility) = local_modifiers.get(reference.id.as_str()) {
                    reject_more_visible(
                        scope.reward_id,
                        containing_scope,
                        label_min_scope(*target_visibility),
                        &reference.kind,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Returns the more restrictive of two visibility labels.
///
/// `visibility_rank` orders labels from most restrictive (`Hidden`/`Unknown`) to least
/// (`Visible`), so the smaller rank is the more restrictive label. The fold is fail-closed when a
/// single definition reuses an identity: a reference to that identity is judged against the most
/// restricted matching record.
pub(super) fn most_restrictive(
    left: RewardVisibility,
    right: RewardVisibility,
) -> RewardVisibility {
    if visibility_rank(left) <= visibility_rank(right) {
        left
    } else {
        right
    }
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

/// Rejects a referencing record observable in a scope where its reward target is not.
fn reject_more_visible_reward(
    scope: &RewardScope<'_>,
    reward_targets: &BTreeMap<String, RewardTarget>,
    containing_scope: u8,
    target_id: &str,
    kind: &RewardSemanticReferenceKind,
) -> Result<(), RewardCatalogError> {
    if let Some(target) = reward_targets.get(target_id) {
        reject_more_visible(
            scope.reward_id,
            containing_scope,
            reward_min_scope(target.visibility, target.unlock_state),
            kind,
        )?;
    }
    Ok(())
}

fn reject_more_visible(
    reward_id: &str,
    containing_scope: u8,
    target_scope: u8,
    kind: &RewardSemanticReferenceKind,
) -> Result<(), RewardCatalogError> {
    if containing_scope < target_scope {
        return Err(RewardCatalogError::HiddenReferenceLeak {
            reward_id: reward_id.to_owned(),
            reference_kind: kind.clone(),
        });
    }
    Ok(())
}

/// Least restrictive query scope in which a standalone visibility label is observable.
const SCOPE_PUBLIC: u8 = 0;
const SCOPE_REFERENCE: u8 = 1;
const SCOPE_OWNER: u8 = 2;
const SCOPE_NEVER: u8 = 3;

/// Returns the least restrictive scope in which a nested record visibility is observable.
fn label_min_scope(visibility: RewardVisibility) -> u8 {
    match visibility {
        RewardVisibility::Visible => SCOPE_PUBLIC,
        RewardVisibility::OwnerOnly => SCOPE_OWNER,
        RewardVisibility::Hidden | RewardVisibility::Unknown => SCOPE_NEVER,
    }
}

/// Returns the least restrictive scope in which a reward target is observable.
///
/// This mirrors exact lookup: an unlocked visible reward is public, a locked visible reward is
/// reachable only from the reference/owner scopes, an owner-only reward is owner-only, and a
/// hidden or unknown target is observable in no scope.
fn reward_min_scope(visibility: RewardVisibility, unlock_state: ContentUnlockState) -> u8 {
    match visibility {
        RewardVisibility::Visible => match unlock_state {
            ContentUnlockState::Unlocked => SCOPE_PUBLIC,
            ContentUnlockState::Locked => SCOPE_REFERENCE,
            ContentUnlockState::Unknown => SCOPE_NEVER,
        },
        RewardVisibility::OwnerOnly => match unlock_state {
            ContentUnlockState::Unknown => SCOPE_NEVER,
            ContentUnlockState::Unlocked | ContentUnlockState::Locked => SCOPE_OWNER,
        },
        RewardVisibility::Hidden | RewardVisibility::Unknown => SCOPE_NEVER,
    }
}

/// Returns the least restrictive scope in which a nested record is actually observable.
///
/// The containing record is observable only where both its owning reward and the record itself
/// are observable, so the effective scope is the more restrictive of the two.
fn containing_min_scope(scope: &RewardScope<'_>, containing: RewardVisibility) -> u8 {
    reward_min_scope(scope.visibility, scope.unlock_state).max(label_min_scope(containing))
}
