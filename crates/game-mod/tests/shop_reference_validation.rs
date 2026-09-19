// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{
    ShopBlockedReason, ShopCatalog, ShopCatalogError, ShopDefinitionInput, ShopField, ShopItemKind,
    ShopNumber, ShopReferenceKind, ShopRestrictionKind, ShopSaleState, ShopServiceSelectionDomain,
    ShopUnavailableReason, ShopVisibility,
};
use support::{
    CURRENCY_ID, alpha, card_entry, definition, fixture_definitions, fixture_manifest,
    hidden_definition, manifest, manifest_of, money, produce, reference, snapshot, text, tier,
    withheld_text,
};

fn failure(definitions: Vec<ShopDefinitionInput>) -> Result<ShopCatalog, ShopCatalogError> {
    let manifest = manifest(&["shop.alpha"]);
    produce(&manifest, snapshot(&manifest, definitions))
}

fn one(definition: ShopDefinitionInput) -> Result<ShopCatalog, ShopCatalogError> {
    failure(vec![definition])
}

#[test]
fn a_kind_that_contradicts_its_definition_is_rejected_not_left_unknown() {
    let mut entry = card_entry("entry.card");
    entry.item_kind = ShopItemKind::Unknown;
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::KindDisagreesWithDefinition {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.card".to_owned(),
            reported: ShopItemKind::Unknown,
            family: "card".to_owned(),
        })
    );
    let mut entry = card_entry("entry.relic");
    entry.definition = reference(ShopReferenceKind::Relic, "relic.burning_blood");
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::KindDisagreesWithDefinition {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.relic".to_owned(),
            reported: ShopItemKind::Card,
            family: "relic".to_owned(),
        })
    );
}

#[test]
fn repeated_entry_service_definition_and_contributor_identities_are_rejected() {
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![card_entry("entry.card"), card_entry("entry.card")],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::DuplicateEntry {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.card".to_owned(),
        })
    );
    let removal = support::service(
        "service.removal",
        sts2_game_mod::ShopServiceKind::CardRemoval,
        ShopServiceSelectionDomain::DeckCard,
        sts2_game_mod::ShopProspectiveChange::RemoveFromDeck,
    );
    assert_eq!(
        one(ShopDefinitionInput {
            services: vec![removal.clone(), removal],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::DuplicateService {
            shop_id: "shop.alpha".to_owned(),
            service_id: "service.removal".to_owned(),
        })
    );
    let alpha_manifest = manifest(&["shop.alpha"]);
    let mut entry = card_entry("entry.card");
    entry.price.contributors = vec![
        contributor("contributor.base", 100),
        contributor("contributor.base", 50),
    ];
    assert_eq!(
        produce(
            &alpha_manifest,
            snapshot(
                &alpha_manifest,
                vec![ShopDefinitionInput {
                    entries: vec![entry],
                    ..definition("shop.alpha", 3)
                }]
            )
        ),
        Err(ShopCatalogError::DuplicateContributor {
            shop_id: "shop.alpha".to_owned(),
            owner_id: "entry.card".to_owned(),
            contributor_id: "contributor.base".to_owned(),
        })
    );
    let duplicated = vec![definition("shop.alpha", 3), definition("shop.alpha", 3)];
    let dup_manifest = manifest(&["shop.alpha"]);
    let mut dup_snapshot = snapshot(&dup_manifest, duplicated);
    dup_snapshot.family.definition_count = 1;
    assert_eq!(
        produce(&dup_manifest, dup_snapshot),
        Err(ShopCatalogError::DuplicateDefinition(
            "shop.alpha".to_owned()
        ))
    );
}

fn contributor(contributor_id: &str, delta: i64) -> sts2_game_mod::ShopPricingContributor {
    sts2_game_mod::ShopPricingContributor {
        contributor_id: contributor_id.to_owned(),
        kind: sts2_game_mod::ShopPricingContributorKind::Base,
        label: text(contributor_id),
        delta: ShopNumber::Fixed(delta),
    }
}

#[test]
fn dangling_and_scope_leaking_references_are_rejected() {
    let mut entry = card_entry("entry.card");
    entry.references = vec![reference(ShopReferenceKind::Entry, "entry.missing")];
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::DanglingReference {
            shop_id: "shop.alpha".to_owned(),
            reference_kind: ShopReferenceKind::Entry,
            id: "entry.missing".to_owned(),
        })
    );

    let leaky = ShopDefinitionInput {
        references: vec![reference(ShopReferenceKind::Shop, "shop.gamma")],
        ..definition("shop.alpha", 3)
    };
    let manifest = manifest(&["shop.alpha", "shop.gamma"]);
    assert_eq!(
        produce(
            &manifest,
            snapshot(&manifest, vec![leaky, hidden_definition("shop.gamma", 5)])
        ),
        Err(ShopCatalogError::HiddenReferenceLeak {
            shop_id: "shop.alpha".to_owned(),
            reference_kind: ShopReferenceKind::Shop,
        }),
        "a visible shop may not reference a hidden one"
    );

    let mut gamma = hidden_definition("shop.gamma", 5);
    gamma.visibility = ShopVisibility::OwnerOnly;
    let leaky = ShopDefinitionInput {
        references: vec![reference(ShopReferenceKind::Shop, "shop.gamma")],
        ..definition("shop.alpha", 3)
    };
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![leaky, gamma])),
        Err(ShopCatalogError::HiddenReferenceLeak {
            shop_id: "shop.alpha".to_owned(),
            reference_kind: ShopReferenceKind::Shop,
        }),
        "a public shop may not reference an owner-only one"
    );
}

#[test]
fn impossible_prices_discounts_stock_and_services_are_rejected() {
    let mut entry = card_entry("entry.card");
    entry.price.displayed = ShopField::available(money(-1));
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidPrice {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.card".to_owned(),
        })
    );
    let mut entry = card_entry("entry.card");
    entry.price.currency = ShopField::unavailable(ShopUnavailableReason::NotObserved);
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidPrice {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.card".to_owned(),
        })
    );
    let mut entry = card_entry("entry.card");
    entry.price.displayed = ShopField::available(sts2_game_mod::ShopMoney {
        currency_id: "currency.gems".to_owned(),
        amount: ShopNumber::Fixed(10),
    });
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidPrice {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.card".to_owned(),
        })
    );
    assert_eq!(CURRENCY_ID, "currency.gold");

    let mut entry = card_entry("entry.sale");
    entry.sale = ShopSaleState::OnSale {
        sale_id: "sale.spring".to_owned(),
        percent: ShopNumber::Fixed(101),
        label: text("spring sale"),
    };
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidDiscount {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.sale".to_owned(),
        })
    );
    let mut entry = card_entry("entry.stacked");
    entry.sale = ShopSaleState::Stacked {
        tiers: vec![tier("tier.one", 20)],
        combined_percent: ShopNumber::Fixed(20),
    };
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidDiscount {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.stacked".to_owned(),
        }),
        "a stack needs at least two tiers"
    );
    let mut entry = card_entry("entry.stacked");
    entry.sale = ShopSaleState::Stacked {
        tiers: vec![tier("tier.one", 10), tier("tier.two", 30)],
        combined_percent: ShopNumber::Fixed(20),
    };
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidDiscount {
            shop_id: "shop.alpha".to_owned(),
            entry_id: "entry.stacked".to_owned(),
        }),
        "a tier may not exceed the combined percentage"
    );

    let mut entry = card_entry("entry.card");
    entry.stock = support::in_stock(0);
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidInput("stock_quantity"))
    );

    let mut service = support::service(
        "service.broken",
        sts2_game_mod::ShopServiceKind::CardRemoval,
        ShopServiceSelectionDomain::ShopCard,
        sts2_game_mod::ShopProspectiveChange::RemoveFromDeck,
    );
    assert_eq!(
        one(ShopDefinitionInput {
            services: vec![service.clone()],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidService {
            shop_id: "shop.alpha".to_owned(),
            service_id: "service.broken".to_owned(),
        }),
        "a card service must select from the deck"
    );
    service.selection_domain = ShopServiceSelectionDomain::DeckCard;
    service.limits = vec![sts2_game_mod::ShopServiceLimit {
        limit_id: "limit.per_run".to_owned(),
        label: text("per run"),
        unit: sts2_game_mod::ShopServiceLimitUnit::PerRun,
        value: ShopField::available(1),
        remaining: ShopField::available(2),
        references: Vec::new(),
    }];
    assert_eq!(
        one(ShopDefinitionInput {
            services: vec![service],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidService {
            shop_id: "shop.alpha".to_owned(),
            service_id: "service.broken".to_owned(),
        }),
        "remaining allowance may not exceed the declared limit"
    );
    assert_eq!(
        ShopRestrictionKind::CapacityFull,
        support::alpha().entries[7].restrictions[0].kind
    );
}

#[test]
fn hidden_records_may_not_expose_a_value_and_blocked_reasons_stay_typed() {
    let mut entry = card_entry("entry.card");
    entry.visibility = ShopVisibility::Hidden;
    assert_eq!(
        one(ShopDefinitionInput {
            entries: vec![entry],
            ..definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidInput(
            "hidden_record_exposes_value"
        ))
    );
    assert_eq!(
        one(ShopDefinitionInput {
            label: text("shop.alpha"),
            ..hidden_definition("shop.alpha", 3)
        }),
        Err(ShopCatalogError::InvalidInput(
            "hidden_record_exposes_value"
        ))
    );
    let mut definition = hidden_definition("shop.alpha", 3);
    definition.label = withheld_text();
    assert!(one(definition).is_ok());
    assert_eq!(
        ShopBlockedReason::Unaffordable,
        ShopBlockedReason::Unaffordable
    );
}

#[test]
fn manifest_fence_and_family_mismatches_are_rejected_before_any_record_is_read() {
    let manifest = fixture_manifest();
    let definitions = fixture_definitions();
    let mut locale = snapshot(&manifest, definitions.clone());
    locale.locale = "de-DE".to_owned();
    assert_eq!(
        produce(&manifest, locale),
        Err(ShopCatalogError::LocaleMismatch)
    );
    let mut producer = snapshot(&manifest, definitions.clone());
    producer.producer_version = "other-producer".to_owned();
    assert_eq!(
        produce(&manifest, producer),
        Err(ShopCatalogError::ProducerVersionMismatch)
    );
    let mut family = snapshot(&manifest, definitions.clone());
    family.family.entity_kind = "settings".to_owned();
    assert_eq!(
        produce(&manifest, family),
        Err(ShopCatalogError::FamilyIdentityMismatch)
    );
    let mut count = snapshot(&manifest, definitions.clone());
    count.family.definition_count = 2;
    assert_eq!(
        produce(&manifest, count),
        Err(ShopCatalogError::FamilyCountMismatch)
    );
    let other = support::manifest_with_extra(&[("currency", "currency.gems")]);
    assert_eq!(
        produce(&manifest, snapshot(&other, definitions.clone())),
        Err(ShopCatalogError::ManifestMismatch)
    );
    let no_family = manifest_of(&support::item_definitions());
    assert_eq!(
        produce(&no_family, snapshot(&manifest, definitions.clone())),
        Err(ShopCatalogError::ManifestMismatch)
    );
    let mut missing = snapshot(&no_family, definitions);
    missing.manifest = no_family.cursor_binding();
    assert_eq!(
        produce(&no_family, missing),
        Err(ShopCatalogError::MissingFamily)
    );
    assert_eq!(alpha().entries.len(), 11);
}
