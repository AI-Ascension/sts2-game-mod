// SPDX-License-Identifier: MIT

use super::super::{RewardItem, RewardOfferDefinition, RewardVisibilityScope};
use super::collection_status;
use super::page::{RewardDefinitionSummary, RewardItemSummary};
use super::{visible_item, visible_legal_action, visible_rule};

/// Builds a bounded reward summary that preserves per-collection availability.
pub(super) fn reward_summary(
    definition: &RewardOfferDefinition,
    scope: RewardVisibilityScope,
) -> RewardDefinitionSummary {
    RewardDefinitionSummary {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        kind: definition.kind.clone(),
        item_count: definition
            .items
            .iter()
            .filter(|item| visible_item(item, scope))
            .count(),
        items_status: collection_status(&definition.items, scope, visible_item),
        rule_count: definition
            .generation
            .iter()
            .filter(|rule| visible_rule(rule, scope))
            .count(),
        generation_status: collection_status(&definition.generation, scope, visible_rule),
        legal_actions_status: collection_status(
            &definition.selection.legal_actions,
            scope,
            visible_legal_action,
        ),
    }
}

/// Builds a bounded offered-item summary that preserves field availability.
pub(super) fn item_summary(item: &RewardItem) -> RewardItemSummary {
    RewardItemSummary {
        reference: item.reference.clone(),
        label: item.label.clone(),
        definition: item.definition.clone(),
        quantity: item.quantity.clone(),
        unit_status: item.quantity.unit.status(),
        instance_status: item.instance.status(),
        visibility: item.visibility,
    }
}
