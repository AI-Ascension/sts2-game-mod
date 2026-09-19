// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{
    ShopBlockedReason, ShopCatalogError, ShopDefinitionReference, ShopFieldStatus, ShopItemKind,
    ShopListQuery, ShopReferenceKind, ShopRoundingMode, ShopSaleState, ShopServiceSelectionDomain,
    ShopStockState, ShopVisibilityScope,
};
use support::{fixture, price};

fn definition<'a>(
    catalog: &'a sts2_game_mod::ShopCatalog,
    shop_id: &str,
) -> &'a sts2_game_mod::ShopDefinition {
    catalog.definition(shop_id).expect("definition")
}

fn shop_reference(catalog: &sts2_game_mod::ShopCatalog, shop_id: &str) -> ShopDefinitionReference {
    definition(catalog, shop_id).reference.clone()
}

#[test]
fn every_supported_entry_reports_its_kind_and_resolves_its_definition() {
    let (_manifest, catalog) = fixture();
    assert_eq!(catalog.len(), 3);
    let alpha = definition(&catalog, "shop.alpha");
    assert_eq!(alpha.generation(), 3);
    assert_eq!(alpha.entry_count(), 11);
    assert_eq!(alpha.service_count(), 4);

    for (entry_id, item_kind, reference_kind) in [
        ("entry.card", ShopItemKind::Card, ShopReferenceKind::Card),
        ("entry.relic", ShopItemKind::Relic, ShopReferenceKind::Relic),
        (
            "entry.potion",
            ShopItemKind::Potion,
            ShopReferenceKind::Potion,
        ),
        (
            "entry.service",
            ShopItemKind::Service,
            ShopReferenceKind::Service,
        ),
    ] {
        let entry = alpha.entry(entry_id).expect("entry");
        assert_eq!(entry.item_kind, item_kind, "{entry_id}");
        assert_eq!(entry.definition.kind, reference_kind, "{entry_id}");
        assert_ne!(entry.item_kind, ShopItemKind::Unknown, "{entry_id}");
        assert!(entry.definition.id.starts_with(match reference_kind {
            ShopReferenceKind::Card => "card.",
            ShopReferenceKind::Relic => "relic.",
            ShopReferenceKind::Potion => "potion.",
            _ => "shop-service.",
        }));
    }

    let resolved = alpha.entry("entry.card").expect("card");
    assert_eq!(resolved.definition.id, "card.strike");
    assert_eq!(
        resolved.references[0].kind,
        ShopReferenceKind::Entry,
        "an entry reference stays local and resolves to a sibling entry"
    );
    assert!(alpha.entry(&resolved.references[0].id).is_some());

    let service = alpha.service("service.removal").expect("service");
    assert_eq!(service.kind, sts2_game_mod::ShopServiceKind::CardRemoval);
    assert_eq!(
        service.selection_domain,
        ShopServiceSelectionDomain::DeckCard
    );
    assert_eq!(service.limits.len(), 1);
    assert_eq!(service.eligibility.len(), 1);
}

#[test]
fn sale_and_stacked_discounts_report_their_own_percentages_and_price() {
    let (_manifest, catalog) = fixture();
    let alpha = definition(&catalog, "shop.alpha");

    let sale = alpha.entry("entry.sale").expect("sale");
    assert!(sale.sale.is_active());
    assert_eq!(sale.sale.tiers(), &[]);
    assert_eq!(
        sale.price
            .displayed
            .value()
            .and_then(|money| money.fixed_amount()),
        Some(75)
    );
    assert_eq!(sale.price.matches_documented(), Some(false));
    assert_eq!(
        sale.price.blocked_reason,
        sts2_game_mod::ShopField::unavailable(sts2_game_mod::ShopUnavailableReason::NotObserved)
    );

    let stacked = alpha.entry("entry.stacked").expect("stacked");
    assert!(stacked.sale.is_active());
    assert_eq!(stacked.sale.tiers().len(), 2);
    assert_eq!(stacked.sale.tiers()[0].tier_id, "tier.one");
    let total = stacked
        .sale
        .tiers()
        .iter()
        .filter_map(|tier| match tier.percent {
            sts2_game_mod::ShopNumber::Fixed(value) => Some(value),
            _ => None,
        })
        .sum::<i64>();
    assert_eq!(total, 25);
    let combined = match &stacked.sale {
        ShopSaleState::Stacked {
            combined_percent, ..
        } => Some(combined_percent),
        _ => None,
    };
    assert_eq!(combined, Some(&sts2_game_mod::ShopNumber::Fixed(25)));

    let plain = alpha.entry("entry.card").expect("card");
    assert!(!plain.sale.is_active());
    assert_eq!(plain.price.matches_documented(), Some(true));
    assert_eq!(plain.price.rounding, ShopRoundingMode::Exact);

    let formula = alpha.entry("entry.service").expect("service entry");
    assert_eq!(formula.price.rounding, ShopRoundingMode::Exact);
    assert_eq!(formula.price.matches_documented(), None);
    let comparison = sts2_game_mod::ShopPriceComparison::of(&formula.price);
    assert_eq!(comparison.displayed.value(), Some(&150));
    assert_eq!(comparison.documented.value(), None);
    assert_eq!(
        comparison.documented.reason(),
        Some(sts2_game_mod::ShopUnavailableReason::NotApplicable)
    );
    assert_eq!(comparison.consistent_with_contributors.value(), None);
    assert_eq!(
        formula.price.unresolved_inputs,
        Vec::<String>::new(),
        "the retained unresolved input lives inside the formula, not the price"
    );
}

#[test]
fn unaffordable_and_full_slot_reasons_stay_distinct_from_the_displayed_price() {
    let (_manifest, catalog) = fixture();
    let alpha = definition(&catalog, "shop.alpha");

    let unaffordable = alpha.entry("entry.unaffordable").expect("unaffordable");
    assert_eq!(
        unaffordable.price.blocked_reason.value(),
        Some(&ShopBlockedReason::Unaffordable)
    );
    assert_eq!(
        unaffordable
            .price
            .displayed
            .value()
            .and_then(|money| money.fixed_amount()),
        Some(120),
        "the blocked reason does not replace the displayed price"
    );
    assert!(unaffordable.restrictions.is_empty());

    let full_slot = alpha.entry("entry.full_slot").expect("full slot");
    assert_eq!(
        full_slot.price.blocked_reason.value(),
        Some(&ShopBlockedReason::FullSlot)
    );
    assert_eq!(full_slot.restrictions.len(), 1);
    let restriction = &full_slot.restrictions[0];
    assert_eq!(
        restriction.kind,
        sts2_game_mod::ShopRestrictionKind::CapacityFull
    );
    assert_eq!(restriction.capacity.value(), Some(&3));
    assert_eq!(restriction.satisfied.value(), Some(&false));
    assert_eq!(restriction.requirements.len(), 1);
    assert_eq!(
        restriction.requirements[0].state,
        sts2_game_mod::ShopRequirementState::Unmet
    );
    assert_eq!(
        unaffordable.price.blocked_reason,
        sts2_game_mod::ShopField::available(ShopBlockedReason::Unaffordable)
    );
    assert_ne!(
        full_slot.price.blocked_reason,
        unaffordable.price.blocked_reason
    );
}

#[test]
fn stock_states_keep_sold_out_and_quantity_separate_from_the_price() {
    let (_manifest, catalog) = fixture();
    let alpha = definition(&catalog, "shop.alpha");
    assert_eq!(
        alpha.entry("entry.sold_out").expect("sold out").stock,
        ShopStockState::SoldOut
    );
    assert_eq!(
        alpha.entry("entry.card").expect("card").stock,
        support::in_stock(1)
    );
    assert_eq!(
        alpha.entry("entry.sold_out").expect("sold out").price,
        price(100, 100)
    );
}

#[test]
fn pages_are_bounded_single_use_and_scope_filtered() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.reader();
    let query = |limit: usize, continuation| ShopListQuery {
        locale: "en-US".to_owned(),
        scope: ShopVisibilityScope::Public,
        limit,
        continuation,
    };
    let first = reader.list(&query(1, None)).expect("first");
    assert_eq!(first.total, 2, "the hidden shop is never visible");
    assert!(!first.complete);
    assert_eq!(first.entries[0].reference.shop_id, "shop.alpha");
    assert_eq!(first.entries[0].entry_count, 9);
    assert_eq!(first.entries[0].entries_status, ShopFieldStatus::Withheld);
    assert_eq!(first.entries[0].service_count, 3);
    assert_eq!(first.entries[0].restock_count, 1);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&query(1, Some(continuation.clone())))
        .expect("second");
    assert_eq!(second.entries[0].reference.shop_id, "shop.beta");
    assert!(second.complete);
    assert_eq!(
        reader.list(&query(1, Some(continuation))),
        Err(ShopCatalogError::InvalidContinuation)
    );
    assert_eq!(
        reader.list(&query(0, None)),
        Err(ShopCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&query(65, None)),
        Err(ShopCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&ShopListQuery {
            locale: "de-DE".to_owned(),
            ..query(8, None)
        }),
        Err(ShopCatalogError::LocaleMismatch)
    );
}
