// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/act_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ACT_MAX_PAGE_ITEMS, ACT_REFERENCE_PRODUCER_VERSION, ActEncounterListQuery, ActEvidence,
    ActFamilyState, ActField, ActListQuery, ActMapNodeReference, ActReferenceError,
    ActUnavailableReason, ActVisibility, ActVisibilityScope, ContentUnlockState, EncounterKind,
    EncounterPoolKind, EncounterPossibility, GenerationWeight, MapConstraintKind, RoomCategoryKind,
};

fn list_query(locale: &str, scope: ActVisibilityScope, limit: usize) -> ActListQuery {
    ActListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}

fn full_manifest() -> sts2_game_mod::ContentManifest {
    manifest(
        &["act:one"],
        &["encounter:shared"],
        &["enemy:slime", "enemy:brute", "enemy:lord"],
    )
}

#[test]
fn catalog_binds_manifest_locale_and_producer_identity() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_act("act:one")]);
    assert_eq!(catalog.binding().manifest, content.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        ACT_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().entity_kind, "act");
    assert_eq!(catalog.family().state, ActFamilyState::Handled);
    assert_eq!(catalog.family().definition_count, 1);
    let reference = act_reference(&catalog, "act:one");
    assert_eq!(reference.catalog, *catalog.binding());
}

#[test]
fn deterministic_pages_expose_single_use_bound_continuations() {
    let ids = ["act:a", "act:b", "act:c"];
    let content = manifest(&ids, &[], &["enemy:slime"]);
    let catalog = catalog(&content, ids.iter().map(|id| simple_act(id)).collect());
    let mut reader = catalog.reader();
    let first = reader
        .list(&list_query("en-US", ActVisibilityScope::Owner, 2))
        .expect("first page");
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.act_id.as_str())
            .collect::<Vec<_>>(),
        ["act:a", "act:b"]
    );
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let token = first.continuation.expect("partial continuation");

    let mut second_query = list_query("en-US", ActVisibilityScope::Owner, 2);
    second_query.continuation = Some(token.clone());
    let second = reader.list(&second_query).expect("second page");
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].reference.act_id, "act:c");
    assert!(second.complete);
    assert!(second.continuation.is_none());

    let stale = reader
        .list(&list_query("en-US", ActVisibilityScope::Owner, 2))
        .expect("fresh page");
    let token = stale.continuation.expect("fresh continuation");
    let mut wrong_limit = list_query("en-US", ActVisibilityScope::Owner, 3);
    wrong_limit.continuation = Some(token.clone());
    assert_eq!(
        reader.list(&wrong_limit),
        Err(ActReferenceError::InvalidContinuation)
    );
    let mut reused = list_query("en-US", ActVisibilityScope::Owner, 3);
    reused.continuation = Some(token);
    assert_eq!(
        reader.list(&reused),
        Err(ActReferenceError::InvalidContinuation)
    );
}

#[test]
fn list_bounds_locale_and_foreign_continuations_are_enforced() {
    let content = manifest(&["act:a"], &[], &["enemy:slime"]);
    let catalog = catalog(&content, vec![simple_act("act:a")]);
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", ActVisibilityScope::Public, 0)),
        Err(ActReferenceError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query(
            "en-US",
            ActVisibilityScope::Public,
            ACT_MAX_PAGE_ITEMS + 1
        )),
        Err(ActReferenceError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query("fr-FR", ActVisibilityScope::Public, 8)),
        Err(ActReferenceError::LocaleMismatch)
    );

    let three = manifest(&["act:a", "act:b", "act:c"], &[], &["enemy:slime"]);
    let catalog = fixture::catalog(
        &three,
        ["act:a", "act:b", "act:c"]
            .iter()
            .map(|id| simple_act(id))
            .collect(),
    );
    let mut reader = catalog.reader();
    let first = reader
        .list(&list_query("en-US", ActVisibilityScope::Owner, 2))
        .expect("first");
    let token = first.continuation.expect("continuation");
    let mut foreign = catalog.reader();
    let mut query = list_query("en-US", ActVisibilityScope::Owner, 2);
    query.continuation = Some(token);
    assert_eq!(
        foreign.list(&query),
        Err(ActReferenceError::InvalidContinuation)
    );
}

#[test]
fn exact_lookups_resolve_every_definition_kind() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_act("act:one")]);
    let act = act_reference(&catalog, "act:one");
    let definition = catalog.get(&act, ActVisibilityScope::Owner).expect("act");
    assert_eq!(definition.reference, act);
    assert_eq!(definition.room_categories.len(), 3);
    assert_eq!(definition.encounters.len(), 3);
    assert_eq!(definition.pools.value().expect("pools").len(), 3);
    assert_eq!(
        definition.constraints.value().expect("constraints").len(),
        2
    );

    let encounter = catalog
        .get_encounter(
            &encounter_reference(&catalog, "act:one", "encounter:elite"),
            ActVisibilityScope::Owner,
        )
        .expect("encounter");
    assert_eq!(encounter.kind, EncounterKind::Elite);
    assert_eq!(
        encounter.groups[0].enemies[0].variant_ids,
        ["variant:armored"]
    );
    assert!(matches!(encounter.weight, GenerationWeight::Exact { .. }));

    let category = catalog
        .get_room_category(
            &category_reference(&catalog, "act:one", "category:boss"),
            ActVisibilityScope::Owner,
        )
        .expect("category");
    assert_eq!(category.kind, RoomCategoryKind::Boss);

    let pool = catalog
        .get_pool(
            &pool_reference(&catalog, "act:one", "pool:elite"),
            ActVisibilityScope::Owner,
        )
        .expect("pool");
    assert_eq!(pool.kind, EncounterPoolKind::Elite);
    assert_eq!(pool.entries[0].encounter_id, "encounter:elite");

    let constraint = catalog
        .get_constraint(&act, "constraint:paths", ActVisibilityScope::Owner)
        .expect("constraint");
    assert_eq!(constraint.kind, MapConstraintKind::PathCount);
    assert_eq!(constraint.evidence, ActEvidence::SourceDerived);

    assert_eq!(
        catalog.get(
            &act_reference(&catalog, "act:missing"),
            ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::NotFound)
    );
    assert_eq!(
        catalog.get_encounter(
            &encounter_reference(&catalog, "act:one", "encounter:missing"),
            ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::NotFound)
    );
}

#[test]
fn encounter_pages_are_ordered_filter_bound_and_single_use() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_act("act:one")]);
    let mut reader = catalog.reader();
    let act = act_reference(&catalog, "act:one");
    let page = reader
        .list_encounters(&ActEncounterListQuery {
            act: act.clone(),
            kind: None,
            scope: ActVisibilityScope::Owner,
            limit: 2,
            continuation: None,
        })
        .expect("encounter page");
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.reference.encounter_id.as_str())
            .collect::<Vec<_>>(),
        ["encounter:normal", "encounter:elite"]
    );
    assert_eq!(page.total, 3);
    let token = page.continuation.expect("encounter continuation");
    let second = reader
        .list_encounters(&ActEncounterListQuery {
            act: act.clone(),
            kind: None,
            scope: ActVisibilityScope::Owner,
            limit: 2,
            continuation: Some(token.clone()),
        })
        .expect("second encounter page");
    assert_eq!(second.entries[0].reference.encounter_id, "encounter:boss");
    assert!(second.complete);
    assert_eq!(
        reader.list_encounters(&ActEncounterListQuery {
            act: act.clone(),
            kind: Some(EncounterKind::Elite),
            scope: ActVisibilityScope::Owner,
            limit: 2,
            continuation: Some(token),
        }),
        Err(ActReferenceError::InvalidContinuation)
    );

    let filtered = reader
        .list_encounters(&ActEncounterListQuery {
            act: act.clone(),
            kind: Some(EncounterKind::Elite),
            scope: ActVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        })
        .expect("filtered page");
    assert_eq!(filtered.total, 1);
    assert_eq!(
        filtered.entries[0].reference.encounter_id,
        "encounter:elite"
    );
    assert_eq!(filtered.entries[0].group_count, 1);
    assert_eq!(filtered.entries[0].enemy_count, 1);
}

#[test]
fn reference_possibilities_stay_distinct_from_seed_assignments() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_act("act:one")]);
    let act = act_reference(&catalog, "act:one");
    assert_eq!(
        catalog.possibility_for_category(&act, "category:normal", ActVisibilityScope::Owner),
        Ok(EncounterPossibility::Pool(pool_reference(
            &catalog,
            "act:one",
            "pool:normal"
        )))
    );
    assert_eq!(
        catalog.possibility_for_category(&act, "category:missing", ActVisibilityScope::Owner),
        Ok(EncounterPossibility::Unavailable(
            ActUnavailableReason::NotApplicable
        ))
    );

    let mut denied = rich_act("act:one");
    denied.pools = ActField::Unavailable(ActUnavailableReason::Denied);
    let denied_catalog = fixture::catalog(&content, vec![denied]);
    let denied_act = act_reference(&denied_catalog, "act:one");
    assert_eq!(
        denied_catalog.possibility_for_category(
            &denied_act,
            "category:normal",
            ActVisibilityScope::Owner
        ),
        Ok(EncounterPossibility::Withheld(ActUnavailableReason::Denied))
    );

    let node = ActMapNodeReference {
        map_id: "map:one".to_owned(),
        node_id: "node:7".to_owned(),
    };
    assert_eq!(node.node_id, "node:7");
    assert_eq!(node.map_id, "map:one");
    assert_ne!(node.node_id, denied_act.act_id);
}

#[test]
fn visibility_scope_gates_acts_and_excluded_lookups() {
    let ids = ["act:public", "act:locked", "act:owner", "act:hidden"];
    let content = manifest(&ids, &[], &["enemy:slime"]);
    let public = simple_act("act:public");
    let mut locked = simple_act("act:locked");
    locked.unlock_state = ContentUnlockState::Locked;
    let mut owner = simple_act("act:owner");
    owner.visibility = ActVisibility::OwnerOnly;
    let mut hidden = simple_act("act:hidden");
    hidden.visibility = ActVisibility::Hidden;
    let catalog = catalog(&content, vec![public, locked, owner, hidden]);
    let mut reader = catalog.reader();

    assert_eq!(
        reader
            .list(&list_query("en-US", ActVisibilityScope::Public, 8))
            .expect("public")
            .total,
        1
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", ActVisibilityScope::Reference, 8))
            .expect("reference")
            .total,
        2
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", ActVisibilityScope::Owner, 8))
            .expect("owner")
            .total,
        3
    );
    let locked_reference = act_reference(&catalog, "act:locked");
    assert_eq!(
        catalog.get(&locked_reference, ActVisibilityScope::Public),
        Err(ActReferenceError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&locked_reference, ActVisibilityScope::Reference)
            .is_ok()
    );
    let owner_reference = act_reference(&catalog, "act:owner");
    assert_eq!(
        catalog.get(&owner_reference, ActVisibilityScope::Reference),
        Err(ActReferenceError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&owner_reference, ActVisibilityScope::Owner)
            .is_ok()
    );
    assert_eq!(
        catalog.get(
            &act_reference(&catalog, "act:hidden"),
            ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::ExcludedByScope)
    );
}

#[test]
fn stale_references_are_rejected() {
    let first = full_manifest();
    let catalog = catalog(&first, vec![rich_act("act:one")]);
    let second = manifest(
        &["act:two"],
        &["encounter:shared"],
        &["enemy:slime", "enemy:brute", "enemy:lord"],
    );
    let reference = act_reference(&catalog, "act:one");
    let mut stale = reference.clone();
    stale.catalog.manifest = second.cursor_binding();
    assert_eq!(
        catalog.get(&stale, ActVisibilityScope::Owner),
        Err(ActReferenceError::StaleReference)
    );
    let mut stale_locale = reference;
    stale_locale.catalog.locale = "fr-FR".to_owned();
    assert_eq!(
        catalog.get(&stale_locale, ActVisibilityScope::Owner),
        Err(ActReferenceError::StaleReference)
    );
}
