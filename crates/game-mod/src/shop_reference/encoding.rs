// SPDX-License-Identifier: MIT

use super::*;

/// Returns a conservative byte estimate for one source-owned shop definition.
pub(super) fn definition_bytes(input: &ShopDefinitionInput) -> usize {
    let mut total = input.shop_id.len() + text_bytes(&input.label) + 8;
    total += input.entries.iter().map(entry_bytes).sum::<usize>();
    total += input.services.iter().map(service_bytes).sum::<usize>();
    total += input.restock.iter().map(restock_rule_bytes).sum::<usize>();
    total += input.references.iter().map(reference_bytes).sum::<usize>();
    total
}

fn text_bytes(value: &ShopText) -> usize {
    match value {
        ShopText::Available(value) => value.len(),
        ShopText::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &ShopField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        ShopField::Available(value) => available(value),
        ShopField::Unavailable(_) => 1,
    }
}

fn number_bytes(number: &ShopNumber) -> usize {
    match number {
        ShopNumber::Fixed(_) => 8,
        ShopNumber::Formula(formula) => {
            formula.formula_id.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        ShopNumber::Unavailable(_) => 1,
    }
}

fn money_bytes(money: &ShopMoney) -> usize {
    money.currency_id.len() + number_bytes(&money.amount) + 1
}

fn semantic_kind_bytes(kind: &ShopReferenceKind) -> usize {
    match kind {
        ShopReferenceKind::Content { entity_kind } => entity_kind.len(),
        ShopReferenceKind::Shop
        | ShopReferenceKind::Card
        | ShopReferenceKind::Relic
        | ShopReferenceKind::Potion
        | ShopReferenceKind::Service
        | ShopReferenceKind::Currency
        | ShopReferenceKind::Entry
        | ShopReferenceKind::RestockRule
        | ShopReferenceKind::Unknown => 1,
    }
}

fn reference_bytes(reference: &ShopSemanticReference) -> usize {
    semantic_kind_bytes(&reference.kind) + reference.id.len() + text_bytes(&reference.label) + 2
}

fn snapshot_bytes(snapshot: &ShopSnapshotReference) -> usize {
    snapshot.run_id.len() + snapshot.instance_id.len() + snapshot.snapshot_id.len() + 8
}

fn purchase_action_bytes(action: &ShopPurchaseActionReference) -> usize {
    snapshot_bytes(&action.snapshot) + action.action_id.len() + 2
}

fn contributor_kind_bytes(kind: &ShopPricingContributorKind) -> usize {
    match kind {
        ShopPricingContributorKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn contributor_bytes(contributor: &ShopPricingContributor) -> usize {
    contributor.contributor_id.len()
        + contributor_kind_bytes(&contributor.kind)
        + text_bytes(&contributor.label)
        + number_bytes(&contributor.delta)
        + 2
}

fn price_bytes(price: &ShopPrice) -> usize {
    field_bytes(&price.currency, String::len)
        + field_bytes(&price.displayed, money_bytes)
        + field_bytes(&price.documented, money_bytes)
        + price
            .contributors
            .iter()
            .map(contributor_bytes)
            .sum::<usize>()
        + price
            .unresolved_inputs
            .iter()
            .map(String::len)
            .sum::<usize>()
        + field_bytes(&price.blocked_reason, |_| 1)
        + 4
}

fn stock_bytes(stock: &ShopStockState) -> usize {
    match stock {
        ShopStockState::InStock { quantity } => field_bytes(quantity, |_| 4) + 1,
        ShopStockState::SoldOut
        | ShopStockState::Restricted
        | ShopStockState::NotOffered
        | ShopStockState::Unknown => 1,
    }
}

fn discount_tier_bytes(tier: &ShopDiscountTier) -> usize {
    tier.tier_id.len() + number_bytes(&tier.percent) + reference_bytes(&tier.source) + 2
}

fn sale_bytes(sale: &ShopSaleState) -> usize {
    match sale {
        ShopSaleState::NotOnSale | ShopSaleState::Unknown => 1,
        ShopSaleState::OnSale {
            sale_id,
            percent,
            label,
        } => sale_id.len() + number_bytes(percent) + text_bytes(label) + 2,
        ShopSaleState::Stacked {
            tiers,
            combined_percent,
        } => {
            tiers.iter().map(discount_tier_bytes).sum::<usize>()
                + number_bytes(combined_percent)
                + 2
        }
    }
}

fn requirement_bytes(requirement: &ShopRequirement) -> usize {
    requirement.requirement_id.len()
        + text_bytes(&requirement.label)
        + requirement
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 2
}

fn restriction_kind_bytes(kind: &ShopRestrictionKind) -> usize {
    match kind {
        ShopRestrictionKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn restriction_bytes(restriction: &ShopRestriction) -> usize {
    restriction.restriction_id.len()
        + restriction_kind_bytes(&restriction.kind)
        + text_bytes(&restriction.label)
        + field_bytes(&restriction.capacity, |_| 4)
        + field_bytes(&restriction.satisfied, |_| 1)
        + restriction
            .requirements
            .iter()
            .map(requirement_bytes)
            .sum::<usize>()
        + restriction
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 3
}

fn item_kind_bytes(kind: &ShopItemKind) -> usize {
    match kind {
        ShopItemKind::Custom(value) | ShopItemKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn entry_bytes(entry: &ShopEntryInput) -> usize {
    entry.entry_id.len()
        + text_bytes(&entry.label)
        + item_kind_bytes(&entry.item_kind)
        + reference_bytes(&entry.definition)
        + price_bytes(&entry.price)
        + stock_bytes(&entry.stock)
        + sale_bytes(&entry.sale)
        + field_bytes(&entry.purchase_action, purchase_action_bytes)
        + entry
            .restrictions
            .iter()
            .map(restriction_bytes)
            .sum::<usize>()
        + entry.references.iter().map(reference_bytes).sum::<usize>()
        + 4
}

fn service_kind_bytes(kind: &ShopServiceKind) -> usize {
    match kind {
        ShopServiceKind::Custom(value) | ShopServiceKind::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn selection_domain_bytes(domain: &ShopServiceSelectionDomain) -> usize {
    match domain {
        ShopServiceSelectionDomain::Custom(value)
        | ShopServiceSelectionDomain::Unsupported(value) => value.len(),
        _ => 1,
    }
}

fn prospective_change_bytes(change: &ShopProspectiveChange) -> usize {
    match change {
        ShopProspectiveChange::Custom(value) | ShopProspectiveChange::Unsupported(value) => {
            value.len()
        }
        _ => 1,
    }
}

fn limit_unit_bytes(unit: &ShopServiceLimitUnit) -> usize {
    match unit {
        ShopServiceLimitUnit::Custom(value) => value.len(),
        _ => 1,
    }
}

fn limit_bytes(limit: &ShopServiceLimit) -> usize {
    limit.limit_id.len()
        + text_bytes(&limit.label)
        + limit_unit_bytes(&limit.unit)
        + field_bytes(&limit.value, |_| 4)
        + field_bytes(&limit.remaining, |_| 4)
        + limit.references.iter().map(reference_bytes).sum::<usize>()
        + 3
}

fn service_bytes(service: &ShopServiceInput) -> usize {
    service.service_id.len()
        + text_bytes(&service.label)
        + service_kind_bytes(&service.kind)
        + price_bytes(&service.cost)
        + selection_domain_bytes(&service.selection_domain)
        + service
            .selection_candidates
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + service.limits.iter().map(limit_bytes).sum::<usize>()
        + service
            .eligibility
            .iter()
            .map(requirement_bytes)
            .sum::<usize>()
        + prospective_change_bytes(&service.prospective_change)
        + stock_bytes(&service.stock)
        + field_bytes(&service.purchase_action, purchase_action_bytes)
        + service
            .references
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + 5
}

fn trigger_kind_bytes(kind: &ShopRestockTriggerKind) -> usize {
    match kind {
        ShopRestockTriggerKind::Custom(value) | ShopRestockTriggerKind::Unsupported(value) => {
            value.len()
        }
        _ => 1,
    }
}

fn restock_rule_bytes(rule: &ShopRestockRule) -> usize {
    rule.rule_id.len()
        + text_bytes(&rule.label)
        + rule.trigger.trigger_id.len()
        + trigger_kind_bytes(&rule.trigger.kind)
        + text_bytes(&rule.trigger.label)
        + field_bytes(&rule.replaces_sold_out, |_| 1)
        + rule
            .restocks_entries
            .iter()
            .map(reference_bytes)
            .sum::<usize>()
        + rule.references.iter().map(reference_bytes).sum::<usize>()
        + 10
}
