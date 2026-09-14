// SPDX-License-Identifier: MIT

mod catalog;
mod lookup;
mod page;
mod reader;
mod summary;

pub use catalog::RewardCatalog;
pub use page::*;
pub use reader::RewardCatalogReader;

use crate::ContentUnlockState;

use super::{
    RewardField, RewardFieldStatus, RewardGenerationRule, RewardItem, RewardLegalAction,
    RewardModifier, RewardOfferDefinition, RewardRequirement, RewardSelection, RewardStatePolicy,
    RewardText, RewardUnavailableReason, RewardVisibility, RewardVisibilityScope,
};

fn visible_reward(definition: &RewardOfferDefinition, scope: RewardVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => {
            matches!(
                scope,
                RewardVisibilityScope::Reference | RewardVisibilityScope::Owner
            )
        }
        ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visible_item(item: &RewardItem, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(item.visibility, scope)
}

fn visible_rule(rule: &RewardGenerationRule, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(rule.visibility, scope)
}

fn visible_requirement(requirement: &RewardRequirement, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(requirement.visibility, scope)
}

fn visible_modifier(modifier: &RewardModifier, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(modifier.visibility, scope)
}

fn visible_legal_action(action: &RewardLegalAction, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(action.visibility, scope)
}

pub(super) fn visible_selection(selection: &RewardSelection, scope: RewardVisibilityScope) -> bool {
    visibility_allowed(selection.visibility, scope)
}

pub(super) fn visible_state_policy(
    policy: &RewardStatePolicy,
    scope: RewardVisibilityScope,
) -> bool {
    visibility_allowed(policy.visibility, scope)
}

fn visibility_allowed(visibility: RewardVisibility, scope: RewardVisibilityScope) -> bool {
    match visibility {
        RewardVisibility::Visible => true,
        RewardVisibility::OwnerOnly => matches!(scope, RewardVisibilityScope::Owner),
        RewardVisibility::Hidden | RewardVisibility::Unknown => false,
    }
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection with only restricted entries is
/// `Denied`; a mixed collection is `Partial`. The visible remainder never includes restricted data.
fn collection_status<T>(
    items: &[T],
    scope: RewardVisibilityScope,
    visible: fn(&T, RewardVisibilityScope) -> bool,
) -> RewardFieldStatus {
    scoped_status(RewardFieldStatus::Available, items, scope, visible)
}

/// Combines a source availability status with scope withholding.
///
/// A source status other than `Available` (`Denied`, `NotObserved`, `Unsupported`, `Failed`,
/// `Unknown`, `NotApplicable`, `Partial`) already states that the source could not safely retain
/// the whole collection and is preserved verbatim. An `Available` source status is degraded only
/// by scope: `Denied` when every entry is withheld, `Partial` when some are, and `Available`
/// otherwise (including a genuinely observed-empty collection).
fn scoped_status<T>(
    source: RewardFieldStatus,
    items: &[T],
    scope: RewardVisibilityScope,
    visible: fn(&T, RewardVisibilityScope) -> bool,
) -> RewardFieldStatus {
    if source != RewardFieldStatus::Available {
        return source;
    }
    let visible_count = items.iter().filter(|item| visible(item, scope)).count();
    if visible_count == items.len() {
        RewardFieldStatus::Available
    } else if visible_count == 0 {
        RewardUnavailableReason::Denied.status()
    } else {
        RewardFieldStatus::Partial
    }
}

/// Projects a definition to the requested scope, withholding hidden nested records.
pub(super) fn project_definition(
    definition: &RewardOfferDefinition,
    scope: RewardVisibilityScope,
) -> RewardOfferDefinition {
    let show_selection = visible_selection(&definition.selection, scope);
    let show_policy = visible_state_policy(&definition.state_policy, scope);
    RewardOfferDefinition {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        kind: definition.kind.clone(),
        unlock_state: definition.unlock_state,
        visibility: definition.visibility,
        selection: if show_selection {
            project_selection(&definition.selection, scope)
        } else {
            withheld_selection(&definition.selection)
        },
        selection_status: availability(show_selection),
        items: definition
            .items
            .iter()
            .filter(|item| visible_item(item, scope))
            .cloned()
            .collect(),
        items_status: collection_status(&definition.items, scope, visible_item),
        generation: definition
            .generation
            .iter()
            .filter(|rule| visible_rule(rule, scope))
            .map(|rule| project_rule(rule, scope))
            .collect(),
        generation_status: collection_status(&definition.generation, scope, visible_rule),
        state_policy: if show_policy {
            definition.state_policy.clone()
        } else {
            withheld_state_policy(&definition.state_policy)
        },
        state_policy_status: availability(show_policy),
        references: definition.references.clone(),
    }
}

fn availability(visible: bool) -> RewardFieldStatus {
    if visible {
        RewardFieldStatus::Available
    } else {
        RewardUnavailableReason::Denied.status()
    }
}

/// Replaces a restricted selection group with an explicit, data-free withheld record.
fn withheld_selection(selection: &RewardSelection) -> RewardSelection {
    RewardSelection {
        group_id: String::new(),
        label: RewardText::Unavailable(RewardUnavailableReason::Denied),
        choose_min: RewardField::Unavailable(RewardUnavailableReason::Denied),
        choose_max: RewardField::Unavailable(RewardUnavailableReason::Denied),
        optional_skip: RewardField::Unavailable(RewardUnavailableReason::Denied),
        legal_actions: Vec::new(),
        legal_actions_status: RewardUnavailableReason::Denied.status(),
        references: Vec::new(),
        visibility: selection.visibility,
    }
}

/// Replaces a restricted state policy with an explicit, data-free withheld record.
fn withheld_state_policy(policy: &RewardStatePolicy) -> RewardStatePolicy {
    let denied = RewardUnavailableReason::Denied;
    RewardStatePolicy {
        claim_limit: RewardField::Unavailable(denied),
        capacity: RewardField::Unavailable(denied),
        replacement: RewardField::Unavailable(denied),
        multi_stage: RewardField::Unavailable(denied),
        visibility: policy.visibility,
    }
}

fn project_selection(selection: &RewardSelection, scope: RewardVisibilityScope) -> RewardSelection {
    RewardSelection {
        group_id: selection.group_id.clone(),
        label: selection.label.clone(),
        choose_min: selection.choose_min.clone(),
        choose_max: selection.choose_max.clone(),
        optional_skip: selection.optional_skip.clone(),
        legal_actions: selection
            .legal_actions
            .iter()
            .filter(|action| visible_legal_action(action, scope))
            .cloned()
            .collect(),
        legal_actions_status: collection_status(
            &selection.legal_actions,
            scope,
            visible_legal_action,
        ),
        references: selection.references.clone(),
        visibility: selection.visibility,
    }
}

fn project_rule(rule: &RewardGenerationRule, scope: RewardVisibilityScope) -> RewardGenerationRule {
    RewardGenerationRule {
        rule_id: rule.rule_id.clone(),
        label: rule.label.clone(),
        pool: rule.pool.clone(),
        rarity_weights: rule.rarity_weights.clone(),
        rarity_status: rule.rarity_status,
        eligibility: rule
            .eligibility
            .iter()
            .filter(|requirement| visible_requirement(requirement, scope))
            .cloned()
            .collect(),
        eligibility_status: scoped_status(
            rule.eligibility_status,
            &rule.eligibility,
            scope,
            visible_requirement,
        ),
        modifiers: rule
            .modifiers
            .iter()
            .filter(|modifier| visible_modifier(modifier, scope))
            .cloned()
            .collect(),
        modifiers_status: scoped_status(
            rule.modifiers_status,
            &rule.modifiers,
            scope,
            visible_modifier,
        ),
        probability: rule.probability.clone(),
        references: rule.references.clone(),
        evidence: rule.evidence,
        visibility: rule.visibility,
    }
}
