// SPDX-License-Identifier: MIT

use super::{
    definition::ShopDefinition,
    entry::ShopEntry,
    field::{ShopFieldStatus, ShopUnavailableReason},
    model::{ShopVisibility, ShopVisibilityScope},
    page::{ShopDefinitionSummary, ShopEntrySummary, ShopServiceSummary},
    restock::ShopRestockRule,
    service::ShopService,
};

/// Returns whether a visibility label may be observed under one scope.
pub(super) fn visibility_allowed(visibility: ShopVisibility, scope: ShopVisibilityScope) -> bool {
    match visibility {
        ShopVisibility::Visible => true,
        ShopVisibility::OwnerOnly => scope == ShopVisibilityScope::Owner,
        ShopVisibility::Hidden | ShopVisibility::Unknown => false,
    }
}

/// Returns whether one inventory entry may be observed under one scope.
pub(super) fn visible_entry(entry: &ShopEntry, scope: ShopVisibilityScope) -> bool {
    visibility_allowed(entry.visibility, scope)
}

/// Returns whether one service may be observed under one scope.
pub(super) fn visible_service(service: &ShopService, scope: ShopVisibilityScope) -> bool {
    visibility_allowed(service.visibility, scope)
}

/// Returns whether one restock rule may be observed under one scope.
pub(super) fn visible_rule(rule: &ShopRestockRule, scope: ShopVisibilityScope) -> bool {
    visibility_allowed(rule.visibility, scope)
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection whose every record is withheld is
/// `Denied`; a mixed collection is `Withheld`. A withheld record is dropped rather than replaced
/// with a placeholder, so no restricted identity is disclosed.
fn scoped_status(total: usize, visible: usize, source: ShopFieldStatus) -> ShopFieldStatus {
    if source != ShopFieldStatus::Available {
        return source;
    }
    if visible == total {
        ShopFieldStatus::Available
    } else if visible == 0 {
        ShopUnavailableReason::Denied.status()
    } else {
        ShopUnavailableReason::Withheld.status()
    }
}

/// Reports the availability of one slice-shaped collection after scope withholding.
pub(super) fn collection_status<T>(
    items: &[T],
    scope: ShopVisibilityScope,
    visible: fn(&T, ShopVisibilityScope) -> bool,
) -> ShopFieldStatus {
    let shown = items.iter().filter(|item| visible(item, scope)).count();
    scoped_status(items.len(), shown, ShopFieldStatus::Available)
}

/// Reports the availability of one keyed collection after scope withholding.
pub(super) fn map_status<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    scope: ShopVisibilityScope,
    visible: fn(&T, ShopVisibilityScope) -> bool,
) -> ShopFieldStatus {
    let mut total = 0;
    let mut shown = 0;
    for item in items {
        total += 1;
        if visible(item, scope) {
            shown += 1;
        }
    }
    scoped_status(total, shown, ShopFieldStatus::Available)
}

/// Projects a definition to the requested scope, dropping nested records the scope may not observe.
pub(super) fn project_definition(
    definition: &ShopDefinition,
    scope: ShopVisibilityScope,
) -> ShopDefinition {
    ShopDefinition {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        generation: definition.generation,
        entries: definition
            .entries
            .values()
            .filter(|entry| visible_entry(entry, scope))
            .map(|entry| (entry.reference.entry_id.clone(), entry.clone()))
            .collect(),
        services: definition
            .services
            .values()
            .filter(|service| visible_service(service, scope))
            .map(|service| (service.reference.service_id.clone(), service.clone()))
            .collect(),
        restock: definition
            .restock
            .iter()
            .filter(|rule| visible_rule(rule, scope))
            .cloned()
            .collect(),
        restock_status: collection_status(&definition.restock, scope, visible_rule),
        references: definition.references.clone(),
    }
}

/// Builds the bounded page summary for one shop definition under one scope.
pub(super) fn shop_summary(
    definition: &ShopDefinition,
    scope: ShopVisibilityScope,
) -> ShopDefinitionSummary {
    ShopDefinitionSummary {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        generation: definition.generation,
        entry_count: definition
            .entries
            .values()
            .filter(|entry| visible_entry(entry, scope))
            .count(),
        entries_status: map_status(definition.entries.values(), scope, visible_entry),
        service_count: definition
            .services
            .values()
            .filter(|service| visible_service(service, scope))
            .count(),
        services_status: map_status(definition.services.values(), scope, visible_service),
        restock_count: definition
            .restock
            .iter()
            .filter(|rule| visible_rule(rule, scope))
            .count(),
        restock_status: collection_status(&definition.restock, scope, visible_rule),
    }
}

/// Builds a bounded entry summary that preserves price and stock availability.
pub(super) fn entry_summary(entry: &ShopEntry) -> ShopEntrySummary {
    ShopEntrySummary {
        reference: entry.reference.clone(),
        label: entry.label.clone(),
        item_kind: entry.item_kind.clone(),
        definition: entry.definition.clone(),
        price: entry.price.clone(),
        stock: entry.stock.clone(),
        sale: entry.sale.clone(),
        visibility: entry.visibility,
    }
}

/// Builds a bounded service summary that preserves cost and stock availability.
pub(super) fn service_summary(service: &ShopService) -> ShopServiceSummary {
    ShopServiceSummary {
        reference: service.reference.clone(),
        label: service.label.clone(),
        kind: service.kind.clone(),
        cost: service.cost.clone(),
        selection_domain: service.selection_domain.clone(),
        prospective_change: service.prospective_change.clone(),
        stock: service.stock.clone(),
        visibility: service.visibility,
    }
}
