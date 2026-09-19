// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{ShopCatalogError, ShopStockReference, ShopStockState, ShopVisibilityScope};
use support::{fixture, in_stock, manifest, produce, reference, restock_rule, snapshot};

fn stock(
    catalog: &sts2_game_mod::ShopCatalog,
    entry_id: &str,
    generation: u64,
) -> Result<ShopStockState, ShopCatalogError> {
    let entry = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry(entry_id)
        .expect("entry")
        .reference
        .clone();
    catalog.reader().stock_for_reference(
        &ShopStockReference { entry, generation },
        ShopVisibilityScope::Public,
    )
}

fn stock_of(catalog: &sts2_game_mod::ShopCatalog, entry_id: &str) -> ShopStockState {
    stock(catalog, entry_id, 3).expect("stock")
}

#[test]
fn stock_reference_is_fenced_to_the_generation_that_produced_it() {
    let (_manifest, catalog) = fixture();
    assert_eq!(stock_of(&catalog, "entry.card"), in_stock(1));
    assert_eq!(
        stock_of(&catalog, "entry.sold_out"),
        ShopStockState::SoldOut
    );
    assert_eq!(
        stock(&catalog, "entry.card", 2),
        Err(ShopCatalogError::StaleStockReference {
            entry_id: "entry.card".to_owned(),
            referenced: 2,
            current: 3,
        }),
        "an earlier restock generation is refused rather than answered"
    );
    assert_eq!(
        stock(&catalog, "entry.card", 4),
        Err(ShopCatalogError::StaleStockReference {
            entry_id: "entry.card".to_owned(),
            referenced: 4,
            current: 3,
        })
    );
}

#[test]
fn stock_reference_resolves_only_inside_its_own_catalog_and_scope() {
    let (_manifest, catalog) = fixture();
    let other_manifest = support::manifest_with_extra(&[("currency", "currency.gems")]);
    let other = produce(
        &other_manifest,
        snapshot(&other_manifest, support::fixture_definitions()),
    )
    .expect("other catalog");
    let entry = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry("entry.card")
        .expect("card")
        .reference
        .clone();
    assert_eq!(
        other.reader().stock_for_reference(
            &ShopStockReference {
                entry: entry.clone(),
                generation: 3,
            },
            ShopVisibilityScope::Public,
        ),
        Err(ShopCatalogError::StaleReference)
    );
    let hidden = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry("entry.private")
        .expect("private")
        .reference
        .clone();
    assert_eq!(
        catalog.reader().stock_for_reference(
            &ShopStockReference {
                entry: hidden,
                generation: 3,
            },
            ShopVisibilityScope::Owner,
        ),
        Err(ShopCatalogError::ExcludedByScope)
    );
    assert_eq!(
        catalog.reader().stock_for_reference(
            &ShopStockReference {
                entry: entry.clone(),
                generation: 3,
            },
            ShopVisibilityScope::Owner,
        ),
        Ok(in_stock(1)),
        "an owner-scope query still resolves a public entry"
    );
}

#[test]
fn restock_rules_publish_strictly_increasing_generations_for_their_entries() {
    let (_manifest, catalog) = fixture();
    let alpha = catalog.definition("shop.alpha").expect("alpha");
    assert_eq!(alpha.generation(), 3);
    assert_eq!(alpha.restock.len(), 1);
    let rule = &alpha.restock[0];
    assert_eq!(rule.generation, 4);
    assert!(rule.generation > alpha.generation());
    assert_eq!(rule.restocked_entry_count(), 1);
    assert_eq!(rule.restocks_entries[0].id, "entry.sold_out");
    assert_eq!(
        rule.trigger.kind,
        sts2_game_mod::ShopRestockTriggerKind::ShopVisit
    );
    assert_eq!(rule.replaces_sold_out.value(), Some(&true));
    assert!(alpha.entry("entry.sold_out").is_some());
}

#[test]
fn a_rule_that_reuses_the_observed_generation_is_rejected() {
    let manifest = manifest(&["shop.alpha"]);
    let mut definition = support::definition("shop.alpha", 3);
    definition.entries = vec![support::card_entry("entry.card")];
    definition.restock = vec![restock_rule(
        "rule.reused",
        3,
        vec![reference(
            sts2_game_mod::ShopReferenceKind::Entry,
            "entry.card",
        )],
    )];
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![definition.clone()])),
        Err(ShopCatalogError::InvalidRestock {
            shop_id: "shop.alpha".to_owned(),
            rule_id: "rule.reused".to_owned(),
        })
    );
    definition.restock[0].generation = 4;
    definition.restock.push(restock_rule(
        "rule.older",
        4,
        vec![reference(
            sts2_game_mod::ShopReferenceKind::Entry,
            "entry.card",
        )],
    ));
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![definition])),
        Err(ShopCatalogError::InvalidRestock {
            shop_id: "shop.alpha".to_owned(),
            rule_id: "rule.older".to_owned(),
        }),
        "generations must strictly increase across the rule chain"
    );
}
