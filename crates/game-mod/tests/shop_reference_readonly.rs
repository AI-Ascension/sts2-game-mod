// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{
    FailingShopSource, FixtureShopFailure, FixtureShopSource, ShopCatalogError,
    ShopDefinitionInput, ShopEntryListQuery, ShopField, ShopFieldStatus, ShopListQuery,
    ShopServiceListQuery, ShopStockReference, ShopVisibilityScope,
};
use support::{
    action_reference, card_entry, definition, fixture, fixture_definitions, fixture_manifest,
    manifest, produce, produce_with, snapshot, text,
};

#[test]
fn producing_and_reading_never_mutates_or_extra_reads_the_source() {
    let manifest = fixture_manifest();
    let source = FixtureShopSource::new(snapshot(&manifest, fixture_definitions()));
    let catalog = produce_with(&manifest, &source).expect("catalog");
    assert_eq!(source.reads(), 1, "one owned snapshot read");
    assert_eq!(source.snapshot().definitions.len(), 3);

    let mut reader = catalog.reader();
    let alpha = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .reference
        .clone();
    let card = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry("entry.card")
        .expect("card")
        .reference
        .clone();
    let stock_before = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry("entry.card")
        .expect("card")
        .stock
        .clone();

    reader
        .list(&ShopListQuery {
            locale: "en-US".to_owned(),
            scope: ShopVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("list");
    reader
        .get(&alpha, ShopVisibilityScope::Public)
        .expect("get");
    reader
        .get_entry(&card, ShopVisibilityScope::Public)
        .expect("entry");
    reader
        .list_entries(&ShopEntryListQuery {
            shop: alpha.clone(),
            scope: ShopVisibilityScope::Owner,
            limit: 16,
            continuation: None,
        })
        .expect("entries");
    reader
        .list_services(&ShopServiceListQuery {
            shop: alpha.clone(),
            scope: ShopVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("services");
    let stock = reader
        .stock_for_reference(
            &ShopStockReference {
                entry: card.clone(),
                generation: 3,
            },
            ShopVisibilityScope::Public,
        )
        .expect("stock");

    assert_eq!(
        stock, stock_before,
        "a read reports stock without changing it"
    );
    assert_eq!(
        source.reads(),
        1,
        "no read triggered a hidden second source read"
    );
    let unchanged = catalog
        .definition("shop.alpha")
        .expect("alpha")
        .entry("entry.card")
        .expect("card");
    assert_eq!(unchanged.stock, stock_before);
    assert_eq!(
        catalog
            .definition("shop.alpha")
            .expect("alpha")
            .entry_count(),
        11,
        "no read removed or restocked an entry"
    );
    assert_eq!(
        source.snapshot().definitions[0].entries.len(),
        11,
        "the retained source snapshot is untouched"
    );
}

#[test]
fn a_transient_purchase_action_cannot_enter_the_static_slice() {
    let manifest = manifest(&["shop.alpha"]);
    let mut entry = card_entry("entry.buyable");
    entry.purchase_action = ShopField::available(action_reference());
    let definition = ShopDefinitionInput {
        entries: vec![entry],
        ..definition("shop.alpha", 3)
    };
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![definition])),
        Err(ShopCatalogError::InvalidInput("purchase_action"))
    );
}

#[test]
fn only_sanitized_source_failures_cross_the_reader_boundary() {
    let manifest = fixture_manifest();
    for (failure, expected) in [
        (
            FixtureShopFailure::NoActiveSource,
            ShopCatalogError::NoActiveSource,
        ),
        (
            FixtureShopFailure::AccessDenied,
            ShopCatalogError::SourceAccessDenied,
        ),
        (
            FixtureShopFailure::Malformed,
            ShopCatalogError::MalformedSource,
        ),
    ] {
        assert_eq!(
            produce_with(&manifest, &FailingShopSource(failure)),
            Err(expected)
        );
    }
}

#[test]
fn withheld_reasons_are_reported_without_substituted_values() {
    let (_manifest, catalog) = fixture();
    let alpha = catalog.definition("shop.alpha").expect("alpha");
    let private = alpha.entry("entry.private").expect("private");
    assert_eq!(private.label.value(), None);
    assert!(!private.label.is_available());
    assert_eq!(private.label.status(), ShopFieldStatus::Withheld);
    assert_ne!(private.label.status(), ShopFieldStatus::Available);
    assert_eq!(text("shop.alpha").value(), Some("shop.alpha"));
}
