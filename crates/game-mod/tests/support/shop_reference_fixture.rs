// SPDX-License-Identifier: MIT

//! Shared shop fixture: two visible shops and one fully hidden shop, plus the per-definition builders.

use sts2_game_mod::{
    ContentManifest, ShopBlockedReason, ShopCatalog, ShopDefinitionInput, ShopDiscountTier,
    ShopField, ShopFieldStatus, ShopItemKind, ShopNumber, ShopReferenceKind, ShopRequirement,
    ShopRequirementState, ShopRestriction, ShopRestrictionKind, ShopSaleState, ShopServiceKind,
    ShopServiceLimit, ShopServiceLimitUnit, ShopServiceSelectionDomain, ShopStockState,
    ShopVisibility,
};

use super::*;

/// Manifest for the shared fixture: two visible shops and one fully hidden shop.
pub fn fixture_manifest() -> ContentManifest {
    manifest(&["shop.alpha", "shop.beta", "shop.gamma"])
}

pub fn tier(tier_id: &str, percent: i64) -> ShopDiscountTier {
    ShopDiscountTier {
        tier_id: tier_id.to_owned(),
        percent: ShopNumber::Fixed(percent),
        source: reference(ShopReferenceKind::Unknown, "modifier.spring"),
    }
}

fn full_slot_restriction() -> ShopRestriction {
    ShopRestriction {
        restriction_id: "restriction.slots".to_owned(),
        kind: ShopRestrictionKind::CapacityFull,
        label: text("no free slot"),
        capacity: ShopField::available(3),
        satisfied: ShopField::available(false),
        requirements: vec![ShopRequirement {
            requirement_id: "req.relic_slots".to_owned(),
            label: text("relic slots"),
            state: ShopRequirementState::Unmet,
            references: Vec::new(),
        }],
        requirements_status: ShopFieldStatus::Available,
        references: Vec::new(),
    }
}

fn per_run_limit() -> ShopServiceLimit {
    ShopServiceLimit {
        limit_id: "limit.per_run".to_owned(),
        label: text("per run"),
        unit: ShopServiceLimitUnit::PerRun,
        value: ShopField::available(2),
        remaining: ShopField::available(1),
        references: Vec::new(),
    }
}

/// Owner-declared shop whose entries cover every supported item family and price case.
pub fn alpha() -> ShopDefinitionInput {
    let mut sale = card_entry("entry.sale");
    sale.price = price(75, 100);
    sale.sale = ShopSaleState::OnSale {
        sale_id: "sale.spring".to_owned(),
        percent: ShopNumber::Fixed(25),
        label: text("spring sale"),
    };
    let mut stacked = card_entry("entry.stacked");
    stacked.price = price(70, 100);
    stacked.sale = ShopSaleState::Stacked {
        tiers: vec![tier("tier.one", 10), tier("tier.two", 15)],
        combined_percent: ShopNumber::Fixed(25),
    };
    let mut unaffordable = card_entry("entry.unaffordable");
    unaffordable.price = price(120, 120);
    unaffordable.price.blocked_reason = ShopField::available(ShopBlockedReason::Unaffordable);
    let mut full_slot = card_entry("entry.full_slot");
    full_slot.price.blocked_reason = ShopField::available(ShopBlockedReason::FullSlot);
    full_slot.restrictions = vec![full_slot_restriction()];
    let mut sold_out = card_entry("entry.sold_out");
    sold_out.stock = ShopStockState::SoldOut;
    let mut owner = card_entry("entry.owner");
    owner.visibility = ShopVisibility::OwnerOnly;
    let mut private = card_entry("entry.private");
    private.label = withheld_text();
    private.visibility = ShopVisibility::Hidden;
    let mut card = card_entry("entry.card");
    card.references = vec![reference(ShopReferenceKind::Entry, "entry.relic")];
    let mut service_entry = entry(
        "entry.service",
        ShopItemKind::Service,
        reference(ShopReferenceKind::Service, "shop-service.removal"),
    );
    service_entry.price = formula_price(150, "formula.service");
    let mut removal = service(
        "service.removal",
        ShopServiceKind::CardRemoval,
        ShopServiceSelectionDomain::DeckCard,
        sts2_game_mod::ShopProspectiveChange::RemoveFromDeck,
    );
    removal.limits = vec![per_run_limit()];
    removal.eligibility = vec![ShopRequirement {
        requirement_id: "req.deck_size".to_owned(),
        label: text("deck has at least five cards"),
        state: ShopRequirementState::Met,
        references: Vec::new(),
    }];
    let mut secret = service(
        "service.secret",
        ShopServiceKind::Custom("service.custom".to_owned()),
        ShopServiceSelectionDomain::DeckCard,
        sts2_game_mod::ShopProspectiveChange::Custom("change.custom".to_owned()),
    );
    secret.visibility = ShopVisibility::OwnerOnly;
    ShopDefinitionInput {
        entries: vec![
            card,
            entry(
                "entry.relic",
                ShopItemKind::Relic,
                reference(ShopReferenceKind::Relic, "relic.burning_blood"),
            ),
            entry(
                "entry.potion",
                ShopItemKind::Potion,
                reference(ShopReferenceKind::Potion, "potion.fire"),
            ),
            service_entry,
            sale,
            stacked,
            unaffordable,
            full_slot,
            sold_out,
            owner,
            private,
        ],
        services: vec![
            removal,
            service(
                "service.upgrade",
                ShopServiceKind::CardUpgrade,
                ShopServiceSelectionDomain::DeckCard,
                sts2_game_mod::ShopProspectiveChange::UpgradeInDeck,
            ),
            service(
                "service.transform",
                ShopServiceKind::CardTransform,
                ShopServiceSelectionDomain::DeckCard,
                sts2_game_mod::ShopProspectiveChange::TransformInDeck,
            ),
            secret,
        ],
        restock: vec![restock_rule(
            "rule.restock",
            4,
            vec![reference(ShopReferenceKind::Entry, "entry.sold_out")],
        )],
        references: vec![
            reference(ShopReferenceKind::Card, "card.strike"),
            reference(ShopReferenceKind::Currency, CURRENCY_ID),
            reference(ShopReferenceKind::Shop, "shop.beta"),
            reference(ShopReferenceKind::RestockRule, "rule.restock"),
        ],
        ..definition("shop.alpha", 3)
    }
}

pub fn beta() -> ShopDefinitionInput {
    ShopDefinitionInput {
        entries: vec![card_entry("entry.beta")],
        ..definition("shop.beta", 1)
    }
}

pub fn gamma() -> ShopDefinitionInput {
    let mut entry = card_entry("entry.gamma");
    entry.label = withheld_text();
    entry.visibility = ShopVisibility::Hidden;
    ShopDefinitionInput {
        entries: vec![entry],
        ..hidden_definition("shop.gamma", 5)
    }
}

pub fn fixture_definitions() -> Vec<ShopDefinitionInput> {
    vec![alpha(), beta(), gamma()]
}

/// Shared fixture: manifest plus the produced immutable catalog.
pub fn fixture() -> (ContentManifest, ShopCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, snapshot(&manifest, fixture_definitions())).expect("catalog");
    (manifest, catalog)
}
