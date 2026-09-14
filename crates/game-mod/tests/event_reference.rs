// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    EVENT_MAX_PAGE_ITEMS, EVENT_REFERENCE_PRODUCER_VERSION, EventActionReference,
    EventCatalogError, EventCostKind, EventFamilyState, EventFollowUp, EventInstanceReference,
    EventNumericValue, EventOptionListQuery, EventProbability, EventVisibility,
    EventVisibilityScope,
};

fn full_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[
        ("event", "event:one"),
        ("enemy", "enemy:slime"),
        ("relic", "relic:shrine"),
    ])
}

#[test]
fn catalog_binds_manifest_locale_and_producer_identity() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    assert_eq!(catalog.binding().manifest, content.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        EVENT_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().entity_kind, "event");
    assert_eq!(catalog.family().state, EventFamilyState::Handled);
    assert_eq!(catalog.family().definition_count, 1);
    let reference = event_reference(&catalog, "event:one");
    assert_eq!(reference.catalog, *catalog.binding());
}

#[test]
fn deterministic_pages_expose_single_use_bound_continuations() {
    let ids = ["event:a", "event:b", "event:c"];
    let content = manifest(&ids.map(|id| ("event", id)));
    let catalog = catalog(&content, ids.iter().map(|id| simple_event(id)).collect());
    let mut reader = catalog.reader();
    let first = reader
        .list(&list_query("en-US", EventVisibilityScope::Owner, 2))
        .expect("first page");
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.event_id.as_str())
            .collect::<Vec<_>>(),
        ["event:a", "event:b"]
    );
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let token = first.continuation.expect("partial continuation");

    let mut second_query = list_query("en-US", EventVisibilityScope::Owner, 2);
    second_query.continuation = Some(token.clone());
    let second = reader.list(&second_query).expect("second page");
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].reference.event_id, "event:c");
    assert!(second.complete);
    assert!(second.continuation.is_none());

    let stale = reader
        .list(&list_query("en-US", EventVisibilityScope::Owner, 2))
        .expect("fresh page");
    let token = stale.continuation.expect("fresh continuation");
    let mut wrong_limit = list_query("en-US", EventVisibilityScope::Owner, 3);
    wrong_limit.continuation = Some(token.clone());
    assert_eq!(
        reader.list(&wrong_limit),
        Err(EventCatalogError::InvalidContinuation)
    );
    let mut reused = list_query("en-US", EventVisibilityScope::Owner, 3);
    reused.continuation = Some(token);
    assert_eq!(
        reader.list(&reused),
        Err(EventCatalogError::InvalidContinuation)
    );
}

#[test]
fn list_bounds_locale_and_foreign_continuations_are_enforced() {
    let content = manifest(&[("event", "event:a")]);
    let catalog = catalog(&content, vec![simple_event("event:a")]);
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", EventVisibilityScope::Public, 0)),
        Err(EventCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query(
            "en-US",
            EventVisibilityScope::Public,
            EVENT_MAX_PAGE_ITEMS + 1
        )),
        Err(EventCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query("fr-FR", EventVisibilityScope::Public, 8)),
        Err(EventCatalogError::LocaleMismatch)
    );

    let three = manifest(&[
        ("event", "event:a"),
        ("event", "event:b"),
        ("event", "event:c"),
    ]);
    let catalog = fixture::catalog(
        &three,
        ["event:a", "event:b", "event:c"]
            .iter()
            .map(|id| simple_event(id))
            .collect(),
    );
    let mut reader = catalog.reader();
    let first = reader
        .list(&list_query("en-US", EventVisibilityScope::Owner, 2))
        .expect("first");
    let token = first.continuation.expect("continuation");
    let mut foreign = catalog.reader();
    let mut query = list_query("en-US", EventVisibilityScope::Owner, 2);
    query.continuation = Some(token);
    assert_eq!(
        foreign.list(&query),
        Err(EventCatalogError::InvalidContinuation)
    );
}

#[test]
fn exact_lookups_resolve_event_page_and_option() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    let reference = event_reference(&catalog, "event:one");
    let definition = catalog
        .get(&reference, EventVisibilityScope::Owner)
        .expect("event");
    assert_eq!(definition.reference, reference);
    assert_eq!(definition.pages.len(), 2);
    assert_eq!(definition.options.len(), 2);
    assert_eq!(definition.eligibility.len(), 1);

    let page = catalog
        .get_page(
            &page_reference(&catalog, "event:one", "page:reward"),
            EventVisibilityScope::Owner,
        )
        .expect("page");
    assert_eq!(page.page_id, "page:reward");

    let option = catalog
        .get_option(
            &option_reference(&catalog, "event:one", "option:offer"),
            EventVisibilityScope::Owner,
        )
        .expect("option");
    assert_eq!(option.reference.option_id, "option:offer");
    assert_eq!(option.requirements.len(), 1);
    assert_eq!(
        option.requirements[0].kind,
        sts2_game_mod::EventRequirementKind::Resource
    );
    assert_eq!(option.costs.len(), 1);
    assert_eq!(option.costs[0].kind, EventCostKind::Gold);
    assert_eq!(option.costs[0].amount, EventNumericValue::Fixed(50));
    assert_eq!(option.outcomes.len(), 2);
    assert_eq!(
        option.outcomes[0].follow_up,
        EventFollowUp::Page("page:reward".to_owned())
    );
    assert!(matches!(
        option.outcomes[0].probability,
        EventProbability::Exact {
            numerator: 1,
            denominator: 2,
            ..
        }
    ));
    assert_eq!(
        option.outcomes[0].effects[0].kind,
        sts2_game_mod::EventEffectKind::GainRelic
    );

    assert_eq!(
        catalog.get(
            &event_reference(&catalog, "event:missing"),
            EventVisibilityScope::Owner
        ),
        Err(EventCatalogError::NotFound)
    );
    assert_eq!(
        catalog.get_page(
            &page_reference(&catalog, "event:one", "page:missing"),
            EventVisibilityScope::Owner
        ),
        Err(EventCatalogError::NotFound)
    );
    assert_eq!(
        catalog.get_option(
            &option_reference(&catalog, "event:one", "option:missing"),
            EventVisibilityScope::Owner
        ),
        Err(EventCatalogError::NotFound)
    );
}

#[test]
fn option_pages_are_ordered_bound_and_single_use() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    let mut reader = catalog.reader();
    let event = event_reference(&catalog, "event:one");
    let page = reader
        .list_options(&EventOptionListQuery {
            event: event.clone(),
            scope: EventVisibilityScope::Owner,
            limit: 1,
            continuation: None,
        })
        .expect("option page");
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.reference.option_id.as_str())
            .collect::<Vec<_>>(),
        ["option:offer"]
    );
    assert_eq!(page.entries[0].requirement_count, 1);
    assert_eq!(page.entries[0].cost_count, 1);
    assert_eq!(page.entries[0].outcome_count, 2);
    assert_eq!(page.total, 2);
    let token = page.continuation.expect("option continuation");
    let second = reader
        .list_options(&EventOptionListQuery {
            event: event.clone(),
            scope: EventVisibilityScope::Owner,
            limit: 1,
            continuation: Some(token.clone()),
        })
        .expect("second option page");
    assert_eq!(second.entries[0].reference.option_id, "option:leave");
    assert_eq!(second.entries[0].requirement_count, 0);
    assert_eq!(second.entries[0].cost_count, 0);
    assert_eq!(second.entries[0].outcome_count, 1);
    assert!(second.complete);
    assert_eq!(
        reader.list_options(&EventOptionListQuery {
            event,
            scope: EventVisibilityScope::Owner,
            limit: 1,
            continuation: Some(token),
        }),
        Err(EventCatalogError::InvalidContinuation)
    );
}

#[test]
fn visibility_scope_gates_events_and_withholds_hidden_outcomes() {
    let ids = [
        "event:public",
        "event:locked",
        "event:owner",
        "event:hidden",
    ];
    let content = manifest(&ids.map(|id| ("event", id)));
    let public = simple_event("event:public");
    let mut locked = simple_event("event:locked");
    locked.unlock_state = sts2_game_mod::ContentUnlockState::Locked;
    let mut owner = simple_event("event:owner");
    owner.visibility = EventVisibility::OwnerOnly;
    let mut hidden = simple_event("event:hidden");
    hidden.visibility = EventVisibility::Hidden;
    let catalog = catalog(&content, vec![public, locked, owner, hidden]);
    let mut reader = catalog.reader();

    assert_eq!(
        reader
            .list(&list_query("en-US", EventVisibilityScope::Public, 8))
            .expect("public")
            .total,
        1
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", EventVisibilityScope::Reference, 8))
            .expect("reference")
            .total,
        2
    );
    assert_eq!(
        reader
            .list(&list_query("en-US", EventVisibilityScope::Owner, 8))
            .expect("owner")
            .total,
        3
    );
    let locked_reference = event_reference(&catalog, "event:locked");
    assert_eq!(
        catalog.get(&locked_reference, EventVisibilityScope::Public),
        Err(EventCatalogError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&locked_reference, EventVisibilityScope::Reference)
            .is_ok()
    );
    assert_eq!(
        catalog.get(
            &event_reference(&catalog, "event:hidden"),
            EventVisibilityScope::Owner
        ),
        Err(EventCatalogError::ExcludedByScope)
    );

    let content = full_manifest();
    let mut definition = rich_event("event:one");
    let mut hidden_outcome = outcome("outcome:hidden", EventFollowUp::End);
    hidden_outcome.visibility = EventVisibility::Hidden;
    definition.options[0].outcomes.push(hidden_outcome);
    let catalog = fixture::catalog(&content, vec![definition]);
    let option = catalog
        .get_option(
            &option_reference(&catalog, "event:one", "option:offer"),
            EventVisibilityScope::Owner,
        )
        .expect("option");
    assert_eq!(option.outcomes.len(), 2);
    assert!(
        option
            .outcomes
            .iter()
            .all(|outcome| outcome.reference.outcome_id != "outcome:hidden")
    );
}

#[test]
fn stale_references_are_rejected() {
    let first = full_manifest();
    let catalog = catalog(&first, vec![rich_event("event:one")]);
    let second = manifest(&[
        ("event", "event:two"),
        ("enemy", "enemy:slime"),
        ("relic", "relic:shrine"),
    ]);
    let reference = event_reference(&catalog, "event:one");
    let mut stale = reference.clone();
    stale.catalog.manifest = second.cursor_binding();
    assert_eq!(
        catalog.get(&stale, EventVisibilityScope::Owner),
        Err(EventCatalogError::StaleReference)
    );
    let mut stale_locale = reference;
    stale_locale.catalog.locale = "fr-FR".to_owned();
    assert_eq!(
        catalog.get(&stale_locale, EventVisibilityScope::Owner),
        Err(EventCatalogError::StaleReference)
    );
}

#[test]
fn definition_and_live_identities_stay_distinct() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    let definition = catalog
        .get(
            &event_reference(&catalog, "event:one"),
            EventVisibilityScope::Owner,
        )
        .expect("event");
    let instance = EventInstanceReference {
        run_id: "run:one".to_owned(),
        event_instance_id: "instance:7".to_owned(),
    };
    let action = EventActionReference {
        instance: instance.clone(),
        action_id: "action:3".to_owned(),
    };
    assert_eq!(instance.event_instance_id, "instance:7");
    assert_eq!(action.action_id, "action:3");
    assert_ne!(instance.event_instance_id, definition.reference.event_id);
    assert_ne!(action.action_id, definition.options[0].reference.option_id);
}
