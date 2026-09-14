// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    REWARD_MAX_PAGE_ITEMS, REWARD_REFERENCE_PRODUCER_VERSION, RewardCatalogError,
    RewardItemListQuery, RewardKind, RewardNumericValue, RewardVisibility, RewardVisibilityScope,
};

#[test]
fn catalog_binds_manifest_locale_and_producer_identity() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    assert_eq!(catalog.binding().manifest, content.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        REWARD_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().entity_kind, "reward");
    assert_eq!(catalog.family().definition_count, 4);
    let reference = reward_definition(&catalog, "reward:card");
    assert_eq!(reference.catalog, *catalog.binding());
}

#[test]
fn deterministic_pages_expose_single_use_bound_continuations() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let mut reader = catalog.reader();
    let first = reader
        .list(&list_query("en-US", RewardVisibilityScope::Owner, 2))
        .expect("first page");
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.reward_id.as_str())
            .collect::<Vec<_>>(),
        ["reward:card", "reward:gold"]
    );
    assert_eq!(first.total, 4);
    assert!(!first.complete);
    let token = first.continuation.expect("partial continuation");

    let mut second_query = list_query("en-US", RewardVisibilityScope::Owner, 2);
    second_query.continuation = Some(token.clone());
    let second = reader.list(&second_query).expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert!(second.complete);
    assert!(second.continuation.is_none());

    let stale = reader
        .list(&list_query("en-US", RewardVisibilityScope::Owner, 2))
        .expect("fresh page");
    let token = stale.continuation.expect("fresh continuation");
    let mut wrong_limit = list_query("en-US", RewardVisibilityScope::Owner, 3);
    wrong_limit.continuation = Some(token.clone());
    assert_eq!(
        reader.list(&wrong_limit),
        Err(RewardCatalogError::InvalidContinuation)
    );
    let mut reused = list_query("en-US", RewardVisibilityScope::Owner, 3);
    reused.continuation = Some(token);
    assert_eq!(
        reader.list(&reused),
        Err(RewardCatalogError::InvalidContinuation)
    );
}

#[test]
fn list_bounds_locale_and_foreign_continuations_are_enforced() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", RewardVisibilityScope::Public, 0)),
        Err(RewardCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query(
            "en-US",
            RewardVisibilityScope::Public,
            REWARD_MAX_PAGE_ITEMS + 1
        )),
        Err(RewardCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query("fr-FR", RewardVisibilityScope::Public, 8)),
        Err(RewardCatalogError::LocaleMismatch)
    );

    let first = reader
        .list(&list_query("en-US", RewardVisibilityScope::Owner, 2))
        .expect("first");
    let token = first.continuation.expect("continuation");
    let mut foreign = catalog.reader();
    let mut query = list_query("en-US", RewardVisibilityScope::Owner, 2);
    query.continuation = Some(token);
    assert_eq!(
        foreign.list(&query),
        Err(RewardCatalogError::InvalidContinuation)
    );
}

#[test]
fn exact_lookups_resolve_reward_and_item_with_modified_currency() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:card"),
            RewardVisibilityScope::Owner,
        )
        .expect("reward");
    assert_eq!(definition.kind, RewardKind::Card);
    assert_eq!(definition.items.len(), 2);
    assert_eq!(definition.generation.len(), 1);

    let item = catalog
        .get_item(
            &item_reference(&catalog, "reward:card", "item:strike"),
            RewardVisibilityScope::Owner,
        )
        .expect("item");
    assert_eq!(item.reference.item_id, "item:strike");

    let gold = catalog
        .get_item(
            &item_reference(&catalog, "reward:gold", "item:gold"),
            RewardVisibilityScope::Owner,
        )
        .expect("gold item");
    assert_eq!(gold.quantity.unit_value(), Some("currency:gold"));
    assert_eq!(gold.quantity.base_amount, RewardNumericValue::Fixed(25));
    assert_eq!(gold.quantity.visible_amount, RewardNumericValue::Fixed(40));
    assert_eq!(
        gold.quantity.modified,
        sts2_game_mod::RewardField::Available(true)
    );

    assert_eq!(
        catalog.get(
            &reward_definition(&catalog, "reward:missing"),
            RewardVisibilityScope::Owner
        ),
        Err(RewardCatalogError::NotFound)
    );
    assert_eq!(
        catalog.get_item(
            &item_reference(&catalog, "reward:card", "item:missing"),
            RewardVisibilityScope::Owner
        ),
        Err(RewardCatalogError::NotFound)
    );
}

#[test]
fn item_pages_are_ordered_bound_and_single_use() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let mut reader = catalog.reader();
    let reward = reward_definition(&catalog, "reward:card");
    let page = reader
        .list_items(&item_list_query(
            reward.clone(),
            RewardVisibilityScope::Owner,
            1,
        ))
        .expect("item page");
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.reference.item_id.as_str())
            .collect::<Vec<_>>(),
        ["item:strike"]
    );
    assert_eq!(
        page.entries[0].definition.kind,
        sts2_game_mod::RewardSemanticReferenceKind::Card
    );
    assert_eq!(page.total, 2);
    let token = page.continuation.expect("item continuation");
    let second = reader
        .list_items(&RewardItemListQuery {
            reward,
            scope: RewardVisibilityScope::Owner,
            limit: 1,
            continuation: Some(token.clone()),
        })
        .expect("second item page");
    assert_eq!(second.entries[0].reference.item_id, "item:defend");
    assert!(second.complete);
    assert_eq!(
        reader.list_items(&RewardItemListQuery {
            reward: reward_definition(&catalog, "reward:card"),
            scope: RewardVisibilityScope::Owner,
            limit: 1,
            continuation: Some(token),
        }),
        Err(RewardCatalogError::InvalidContinuation)
    );
}

#[test]
fn visibility_scope_gates_rewards_and_reports_withheld_items() {
    let ids = [
        "reward:public",
        "reward:locked",
        "reward:owner",
        "reward:hidden",
    ];
    let content = manifest(&[
        ("reward", "reward:public"),
        ("reward", "reward:locked"),
        ("reward", "reward:owner"),
        ("reward", "reward:hidden"),
        ("card", "card:strike"),
    ]);
    let mut locked = simple_reward(
        "reward:locked",
        "item:locked",
        sts2_game_mod::RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    locked.unlock_state = sts2_game_mod::ContentUnlockState::Locked;
    let mut owner = simple_reward(
        "reward:owner",
        "item:owner",
        sts2_game_mod::RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    owner.visibility = RewardVisibility::OwnerOnly;
    let mut hidden = simple_reward(
        "reward:hidden",
        "item:hidden",
        sts2_game_mod::RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    hidden.visibility = RewardVisibility::Hidden;
    let catalog = catalog(
        &content,
        vec![
            simple_reward(
                "reward:public",
                "item:public",
                sts2_game_mod::RewardSemanticReferenceKind::Card,
                "card:strike",
            ),
            locked,
            owner,
            hidden,
        ],
    );
    let mut reader = catalog.reader();
    assert_eq!(
        reader
            .list(&list_query("en-US", RewardVisibilityScope::Public, 8))
            .expect("public")
            .total,
        1
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", RewardVisibilityScope::Reference, 8))
            .expect("reference")
            .total,
        2
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", RewardVisibilityScope::Owner, 8))
            .expect("owner")
            .total,
        3
    );
    let locked_reference = reward_definition(&catalog, "reward:locked");
    assert_eq!(
        catalog.get(&locked_reference, RewardVisibilityScope::Public),
        Err(RewardCatalogError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&locked_reference, RewardVisibilityScope::Reference)
            .is_ok()
    );
    assert_eq!(
        catalog.get(
            &reward_definition(&catalog, "reward:hidden"),
            RewardVisibilityScope::Owner
        ),
        Err(RewardCatalogError::ExcludedByScope)
    );
    let _ = ids;
}

#[test]
fn stale_references_are_rejected() {
    let first = full_manifest();
    let catalog = rich_catalog(&first);
    let second = manifest(&[("reward", "reward:other"), ("card", "card:strike")]);
    let reference = reward_definition(&catalog, "reward:card");
    let mut stale = reference.clone();
    stale.catalog.manifest = second.cursor_binding();
    assert_eq!(
        catalog.get(&stale, RewardVisibilityScope::Owner),
        Err(RewardCatalogError::StaleReference)
    );
    let mut stale_locale = reference;
    stale_locale.catalog.locale = "fr-FR".to_owned();
    assert_eq!(
        catalog.get(&stale_locale, RewardVisibilityScope::Owner),
        Err(RewardCatalogError::StaleReference)
    );
}

#[test]
fn definition_item_live_and_action_identities_stay_distinct() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:card"),
            RewardVisibilityScope::Owner,
        )
        .expect("reward");
    let snapshot = sts2_game_mod::RewardSnapshotReference {
        run_id: "run:one".to_owned(),
        room_id: "room:5".to_owned(),
        snapshot_id: "snapshot:9".to_owned(),
    };
    let offer = sts2_game_mod::RewardOfferReference {
        snapshot: snapshot.clone(),
        offer_id: "offer:3".to_owned(),
    };
    let instance = sts2_game_mod::RewardItemInstanceReference {
        offer: offer.clone(),
        item_instance_id: "instance:7".to_owned(),
    };
    let action = sts2_game_mod::RewardActionReference {
        offer,
        action_id: "action:2".to_owned(),
    };
    assert_eq!(snapshot.room_id, "room:5");
    assert_eq!(instance.item_instance_id, "instance:7");
    assert_eq!(action.action_id, "action:2");
    assert_ne!(instance.item_instance_id, definition.reference.reward_id);
    assert_ne!(action.action_id, definition.items[0].reference.item_id);
}

#[test]
fn reads_are_repeatable_and_read_only() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let reference = reward_definition(&catalog, "reward:gold");
    let first = catalog
        .get(&reference, RewardVisibilityScope::Owner)
        .expect("first read");
    let second = catalog
        .get(&reference, RewardVisibilityScope::Owner)
        .expect("second read");
    assert_eq!(
        first, second,
        "repeated reads must not change the definition"
    );
    assert_eq!(catalog.family().definition_count, 4);
}
