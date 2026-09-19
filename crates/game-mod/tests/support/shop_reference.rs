// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    FixtureShopSource, SHOP_REFERENCE_CURRENCY_KIND, SHOP_REFERENCE_ENTITY_KIND,
    SHOP_REFERENCE_PRODUCER_VERSION, ShopCatalog, ShopCatalogError, ShopCatalogProducer,
    ShopCatalogSnapshot, ShopDefinitionInput, ShopEntryInput, ShopEvidence, ShopFamilyCoverage,
    ShopFamilyState, ShopField, ShopFieldStatus, ShopItemKind, ShopMoney, ShopNumber, ShopPrice,
    ShopReferenceKind, ShopRestockRule, ShopRestockTrigger, ShopRestockTriggerKind, ShopSaleState,
    ShopSemanticReference, ShopServiceInput, ShopServiceKind, ShopServiceSelectionDomain,
    ShopStockState, ShopText, ShopUnavailableReason, ShopVisibility,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn content_definition(entity_kind: &str, id: &str) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("{entity_kind}={id}"),
        localized_text: Some(id.to_owned()),
        origin: ContentOriginInput {
            package_id: Some("base:synthetic".to_owned()),
            package_version: Some("1".to_owned()),
        },
        override_chain: Vec::new(),
    }
}

/// Builds a manifest from explicit `(entity_kind, id)` pairs.
pub fn manifest_of(definitions: &[(&str, &str)]) -> ContentManifest {
    let mut kinds: Vec<String> = Vec::new();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (entity_kind, _) in definitions {
        if !kinds.iter().any(|kind| kind == entity_kind) {
            kinds.push((*entity_kind).to_owned());
        }
        *counts.entry((*entity_kind).to_owned()).or_insert(0) += 1;
    }
    let snapshot = ContentCatalogSnapshot {
        generation_before: 11,
        generation_after: 11,
        game_build: "sts2-build:synthetic".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![ContentPackageInput {
            package_id: "base:synthetic".to_owned(),
            package_version: Some("1".to_owned()),
            order: 0,
        }],
        available_entity_kinds: kinds.clone(),
        registry_definition_counts: counts,
        definitions: definitions
            .iter()
            .map(|(entity_kind, id)| content_definition(entity_kind, id))
            .collect(),
    };
    ContentManifestProducer::new("adapter-v1", kinds)
        .expect("producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

/// Manifest declaring the shop family plus the item families the fixture references.
pub fn manifest(shops: &[&str]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = shops
        .iter()
        .map(|shop_id| (SHOP_REFERENCE_ENTITY_KIND, *shop_id))
        .collect();
    definitions.extend(item_definitions());
    manifest_of(&definitions)
}

/// The item and currency definitions every fixture reference resolves to.
pub fn item_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("card", "card.strike"),
        ("card", "card.defend"),
        ("relic", "relic.burning_blood"),
        ("potion", "potion.fire"),
        ("shop-service", "shop-service.removal"),
        (SHOP_REFERENCE_CURRENCY_KIND, CURRENCY_ID),
    ]
}

/// Fixture manifest plus extra content definitions, used to shift the manifest fence.
pub fn manifest_with_extra(extra: &[(&str, &str)]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = vec![
        (SHOP_REFERENCE_ENTITY_KIND, "shop.alpha"),
        (SHOP_REFERENCE_ENTITY_KIND, "shop.beta"),
        (SHOP_REFERENCE_ENTITY_KIND, "shop.gamma"),
    ];
    definitions.extend(item_definitions());
    definitions.extend(extra.iter().copied());
    manifest_of(&definitions)
}

/// Currency unit every fixture price is denominated in.
pub const CURRENCY_ID: &str = "currency.gold";

pub fn text(value: &str) -> ShopText {
    ShopText::Available(value.to_owned())
}

/// Localized text the source knows exists but must not reveal.
pub fn withheld_text() -> ShopText {
    ShopText::Unavailable(ShopUnavailableReason::Withheld)
}

pub fn reference(kind: ShopReferenceKind, id: &str) -> ShopSemanticReference {
    ShopSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn money(amount: i64) -> ShopMoney {
    ShopMoney {
        currency_id: CURRENCY_ID.to_owned(),
        amount: ShopNumber::Fixed(amount),
    }
}

/// Price with an observed displayed and documented amount.
pub fn price(displayed: i64, documented: i64) -> ShopPrice {
    ShopPrice {
        currency: ShopField::available(CURRENCY_ID.to_owned()),
        displayed: ShopField::available(money(displayed)),
        documented: ShopField::available(money(documented)),
        contributors: Vec::new(),
        contributors_status: ShopFieldStatus::Available,
        rounding: sts2_game_mod::ShopRoundingMode::Exact,
        unresolved_inputs: Vec::new(),
        blocked_reason: ShopField::unavailable(ShopUnavailableReason::NotObserved),
    }
}

/// Price whose documented amount stays a formula instead of a folded total.
pub fn formula_price(displayed: i64, formula_id: &str) -> ShopPrice {
    let mut price = price(displayed, displayed);
    price.documented = ShopField::available(ShopMoney {
        currency_id: CURRENCY_ID.to_owned(),
        amount: ShopNumber::Formula(sts2_game_mod::ShopFormula {
            formula_id: formula_id.to_owned(),
            rounding: sts2_game_mod::ShopRoundingMode::HalfUp,
            unresolved_inputs: vec!["input:rarity".to_owned()],
        }),
    });
    price
}

pub fn in_stock(quantity: u32) -> ShopStockState {
    ShopStockState::InStock {
        quantity: ShopField::available(quantity),
    }
}

/// Transient purchase action reference the fixture proves cannot enter the static slice.
pub fn action_reference() -> sts2_game_mod::ShopPurchaseActionReference {
    sts2_game_mod::ShopPurchaseActionReference {
        snapshot: sts2_game_mod::ShopSnapshotReference {
            run_id: "run.alpha".to_owned(),
            instance_id: "instance:run.alpha".to_owned(),
            epoch: 1,
            snapshot_id: "snapshot.alpha".to_owned(),
        },
        action_id: "action.buy".to_owned(),
    }
}

pub fn entry(
    entry_id: &str,
    item_kind: ShopItemKind,
    definition: ShopSemanticReference,
) -> ShopEntryInput {
    ShopEntryInput {
        entry_id: entry_id.to_owned(),
        label: text(entry_id),
        item_kind,
        definition,
        price: price(100, 100),
        stock: in_stock(1),
        sale: ShopSaleState::NotOnSale,
        purchase_action: ShopField::unavailable(ShopUnavailableReason::NotIntegrated),
        restrictions: Vec::new(),
        references: Vec::new(),
        visibility: ShopVisibility::Visible,
        evidence: ShopEvidence::SourceDerived,
    }
}

pub fn card_entry(entry_id: &str) -> ShopEntryInput {
    entry(
        entry_id,
        ShopItemKind::Card,
        reference(ShopReferenceKind::Card, "card.strike"),
    )
}

pub fn service(
    service_id: &str,
    kind: ShopServiceKind,
    selection_domain: ShopServiceSelectionDomain,
    prospective_change: sts2_game_mod::ShopProspectiveChange,
) -> ShopServiceInput {
    ShopServiceInput {
        service_id: service_id.to_owned(),
        label: text(service_id),
        kind,
        cost: price(75, 75),
        selection_domain,
        selection_candidates: Vec::new(),
        selection_candidates_status: ShopFieldStatus::Available,
        limits: Vec::new(),
        eligibility: Vec::new(),
        eligibility_status: ShopFieldStatus::Available,
        prospective_change,
        stock: in_stock(1),
        purchase_action: ShopField::unavailable(ShopUnavailableReason::NotIntegrated),
        references: Vec::new(),
        visibility: ShopVisibility::Visible,
        evidence: ShopEvidence::SourceDerived,
    }
}

pub fn restock_rule(
    rule_id: &str,
    generation: u64,
    restocks_entries: Vec<ShopSemanticReference>,
) -> ShopRestockRule {
    ShopRestockRule {
        rule_id: rule_id.to_owned(),
        label: text(rule_id),
        trigger: ShopRestockTrigger {
            trigger_id: format!("trigger:{rule_id}"),
            kind: ShopRestockTriggerKind::ShopVisit,
            label: text("shop visit"),
        },
        generation,
        replaces_sold_out: ShopField::available(true),
        restocks_entries,
        references: Vec::new(),
        visibility: ShopVisibility::Visible,
        evidence: ShopEvidence::SourceDerived,
    }
}

/// Base definition with no entries, services, restock rules, or references.
pub fn definition(shop_id: &str, generation: u64) -> ShopDefinitionInput {
    ShopDefinitionInput {
        shop_id: shop_id.to_owned(),
        label: text(shop_id),
        visibility: ShopVisibility::Visible,
        evidence: ShopEvidence::SourceDerived,
        generation,
        entries: Vec::new(),
        services: Vec::new(),
        restock: Vec::new(),
        references: Vec::new(),
    }
}

/// Definition hidden from every scope, which requires an explicitly withheld label.
pub fn hidden_definition(shop_id: &str, generation: u64) -> ShopDefinitionInput {
    ShopDefinitionInput {
        label: withheld_text(),
        visibility: ShopVisibility::Hidden,
        ..definition(shop_id, generation)
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<ShopDefinitionInput>,
) -> ShopCatalogSnapshot {
    ShopCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: SHOP_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: ShopFamilyCoverage {
            entity_kind: SHOP_REFERENCE_ENTITY_KIND.to_owned(),
            state: ShopFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

pub fn produce(
    manifest: &ContentManifest,
    snapshot: ShopCatalogSnapshot,
) -> Result<ShopCatalog, ShopCatalogError> {
    ShopCatalogProducer::new().produce(manifest, &FixtureShopSource::new(snapshot))
}

/// Produces through an explicit source so a test can observe the read count.
pub fn produce_with<S: sts2_game_mod::ShopCatalogSource>(
    manifest: &ContentManifest,
    source: &S,
) -> Result<ShopCatalog, ShopCatalogError> {
    ShopCatalogProducer::new().produce(manifest, source)
}

#[path = "shop_reference_fixture.rs"]
mod fixture;

pub use fixture::*;
