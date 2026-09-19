// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{
    ShopCatalogError, ShopDefinitionReference, ShopEntryListQuery, ShopEntryReference,
    ShopFieldStatus, ShopItemKind, ShopListQuery, ShopReferenceKind, ShopServiceListQuery,
    ShopServiceReference, ShopVisibilityScope,
};
use support::fixture;

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
fn entry_and_service_pages_are_scoped_and_continue_across_restarts() {
    let (_manifest, catalog) = fixture();
    let shop = shop_reference(&catalog, "shop.alpha");
    let mut reader = catalog.reader();
    let entry_query = |limit, continuation| ShopEntryListQuery {
        shop: shop.clone(),
        scope: ShopVisibilityScope::Public,
        limit,
        continuation,
    };
    let first = reader.list_entries(&entry_query(4, None)).expect("entries");
    assert_eq!(first.total, 9);
    assert_eq!(first.entries.len(), 4);
    assert!(!first.complete);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list_entries(&entry_query(4, Some(continuation.clone())))
        .expect("second page");
    assert_eq!(second.entries.len(), 4);
    let third = reader.list_entries(&entry_query(4, None)).expect("restart");
    assert_eq!(third.entries.len(), 4);
    assert!(third.entries[0].reference.entry_id < second.entries[0].reference.entry_id);
    assert_eq!(
        reader.list_entries(&entry_query(4, Some(continuation))),
        Err(ShopCatalogError::InvalidContinuation)
    );

    let owner = reader
        .list_entries(&ShopEntryListQuery {
            scope: ShopVisibilityScope::Owner,
            ..entry_query(16, None)
        })
        .expect("owner entries");
    assert_eq!(owner.total, 10, "owner scope adds the owner-only entry");
    assert!(
        owner
            .entries
            .iter()
            .any(|entry| entry.reference.entry_id == "entry.owner")
    );
    assert!(
        !owner
            .entries
            .iter()
            .any(|entry| entry.reference.entry_id == "entry.private")
    );

    let services = reader
        .list_services(&ShopServiceListQuery {
            shop: shop.clone(),
            scope: ShopVisibilityScope::Public,
            limit: 2,
            continuation: None,
        })
        .expect("services");
    assert_eq!(services.total, 3);
    assert_eq!(services.entries.len(), 2);
    assert_eq!(services.services_status, ShopFieldStatus::Withheld);
    let continuation = services.continuation.clone().expect("service cursor");
    let more = reader
        .list_services(&ShopServiceListQuery {
            shop: shop.clone(),
            scope: ShopVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation),
        })
        .expect("more services");
    assert_eq!(more.total, 3);
    assert!(more.complete);
    let empty = reader
        .list_services(&ShopServiceListQuery {
            shop: shop_reference(&catalog, "shop.beta"),
            scope: ShopVisibilityScope::Public,
            limit: 2,
            continuation: None,
        })
        .expect("no services");
    assert_eq!(empty.total, 0);
    assert_eq!(empty.services_status, ShopFieldStatus::Available);
    assert!(empty.complete);
    assert!(empty.continuation.is_none());
}

#[test]
fn scope_projection_withholds_records_without_inventing_placeholders() {
    let (_manifest, catalog) = fixture();
    let alpha = shop_reference(&catalog, "shop.alpha");
    let reader = catalog.reader();

    let public = reader
        .get(&alpha, ShopVisibilityScope::Public)
        .expect("public projection");
    assert_eq!(public.entries.len(), 9);
    assert_eq!(public.services.len(), 3);
    assert_eq!(public.restock_status, ShopFieldStatus::Available);
    let owner = reader
        .get(&alpha, ShopVisibilityScope::Owner)
        .expect("owner projection");
    assert_eq!(owner.entries.len(), 10);
    assert_eq!(owner.services.len(), 4);
    assert_eq!(owner.label.value(), Some("shop.alpha"));
    assert!(owner.entry("entry.owner").is_some());
    assert!(owner.entry("entry.private").is_none());
    assert!(!public.entries.contains_key("entry.private"));

    assert_eq!(
        reader.get(
            &shop_reference(&catalog, "shop.gamma"),
            ShopVisibilityScope::Owner
        ),
        Err(ShopCatalogError::ExcludedByScope)
    );
    assert_eq!(
        reader.get(
            &ShopDefinitionReference {
                shop_id: "shop.missing".to_owned(),
                ..alpha.clone()
            },
            ShopVisibilityScope::Public
        ),
        Err(ShopCatalogError::NotFound)
    );

    let card = ShopEntryReference {
        catalog: alpha.catalog.clone(),
        shop_id: alpha.shop_id.clone(),
        entry_id: "entry.card".to_owned(),
    };
    let entry = reader
        .get_entry(&card, ShopVisibilityScope::Public)
        .expect("entry");
    assert_eq!(entry.item_kind, ShopItemKind::Card);
    assert_eq!(entry.stock, support::in_stock(1));
    assert_eq!(entry.definition.kind, ShopReferenceKind::Card);

    let private = ShopEntryReference {
        entry_id: "entry.private".to_owned(),
        ..card.clone()
    };
    assert_eq!(
        reader.get_entry(&private, ShopVisibilityScope::Owner),
        Err(ShopCatalogError::ExcludedByScope)
    );
    assert_eq!(
        reader.get_entry(&private, ShopVisibilityScope::Public),
        Err(ShopCatalogError::ExcludedByScope)
    );

    let secret = ShopServiceReference {
        catalog: alpha.catalog.clone(),
        shop_id: alpha.shop_id.clone(),
        service_id: "service.secret".to_owned(),
    };
    assert_eq!(
        reader.get_service(&secret, ShopVisibilityScope::Public),
        Err(ShopCatalogError::ExcludedByScope)
    );
    assert_eq!(
        reader
            .get_service(&secret, ShopVisibilityScope::Owner)
            .expect("owner service")
            .kind,
        sts2_game_mod::ShopServiceKind::Custom("service.custom".to_owned())
    );

    let other_manifest = support::manifest_with_extra(&[("currency", "currency.gems")]);
    let other = support::produce(
        &other_manifest,
        support::snapshot(&other_manifest, support::fixture_definitions()),
    )
    .expect("other catalog");
    assert_ne!(other.binding(), catalog.binding());
    assert_eq!(
        other.reader().get_entry(&card, ShopVisibilityScope::Public),
        Err(ShopCatalogError::StaleReference)
    );
}

#[test]
fn family_state_that_cannot_project_is_refused_consistently() {
    use support::{manifest, produce, snapshot};
    let manifest = manifest(&["shop.alpha"]);
    let mut unimplemented = snapshot(&manifest, Vec::new());
    unimplemented.family.state = sts2_game_mod::ShopFamilyState::Unsupported;
    unimplemented.family.definition_count = 1;
    let catalog = produce(&manifest, unimplemented).expect("catalog");
    assert!(catalog.is_empty());
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&ShopListQuery {
            locale: "en-US".to_owned(),
            scope: ShopVisibilityScope::Public,
            limit: 4,
            continuation: None,
        }),
        Err(ShopCatalogError::UnsupportedFamily)
    );
    let mut unavailable = snapshot(&manifest, Vec::new());
    unavailable.family.state = sts2_game_mod::ShopFamilyState::Unavailable;
    unavailable.family.definition_count = 1;
    let catalog = produce(&manifest, unavailable).expect("catalog");
    assert_eq!(
        catalog.reader().list(&ShopListQuery {
            locale: "en-US".to_owned(),
            scope: ShopVisibilityScope::Public,
            limit: 4,
            continuation: None,
        }),
        Err(ShopCatalogError::UnavailableFamily)
    );
}
