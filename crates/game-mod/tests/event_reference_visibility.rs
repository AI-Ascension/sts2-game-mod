// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    EventCatalog, EventCatalogError, EventCatalogProducer, EventCostKind, EventDefinitionInput,
    EventFieldStatus, EventFollowUp, EventSemanticReferenceKind, EventVisibility,
    EventVisibilityScope,
};

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definitions: Vec<EventDefinitionInput>,
) -> Result<EventCatalog, EventCatalogError> {
    EventCatalogProducer::new().produce(
        content,
        &EventSource {
            snapshot: Ok(snapshot(content, definitions)),
        },
    )
}

fn full_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[
        ("event", "event:one"),
        ("enemy", "enemy:slime"),
        ("relic", "relic:shrine"),
    ])
}

#[test]
fn visible_page_referencing_hidden_page_is_rejected_without_leaking() {
    let content = full_manifest();
    let mut definition = rich_event("event:one");
    let mut secret = page("page:secret");
    secret.visibility = EventVisibility::Hidden;
    definition.pages.push(secret);
    definition.pages[0].references =
        vec![reference(EventSemanticReferenceKind::Page, "page:secret")];
    let error = produce(&content, vec![definition]).expect_err("hidden page edge must be rejected");
    assert_eq!(
        error,
        EventCatalogError::HiddenReferenceLeak {
            event_id: "event:one".to_owned(),
            reference_kind: EventSemanticReferenceKind::Page,
        }
    );
    assert!(
        !format!("{error:?}").contains("page:secret"),
        "the rejection must not disclose the hidden page identity"
    );
}

#[test]
fn visible_page_referencing_owner_only_page_is_rejected() {
    let content = full_manifest();
    let mut definition = rich_event("event:one");
    definition.pages[1].visibility = EventVisibility::OwnerOnly;
    definition.pages[0].references =
        vec![reference(EventSemanticReferenceKind::Page, "page:reward")];
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::HiddenReferenceLeak {
            event_id: "event:one".to_owned(),
            reference_kind: EventSemanticReferenceKind::Page,
        })
    );
}

#[test]
fn visible_option_referencing_hidden_option_is_rejected() {
    let content = full_manifest();
    let mut definition = rich_event("event:one");
    definition.pages[1].visibility = EventVisibility::Hidden;
    definition.options[1].visibility = EventVisibility::Hidden;
    definition.options[0].outcomes[0].follow_up = EventFollowUp::End;
    definition.options[0].references = vec![reference(
        EventSemanticReferenceKind::Option,
        "option:leave",
    )];
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::HiddenReferenceLeak {
            event_id: "event:one".to_owned(),
            reference_kind: EventSemanticReferenceKind::Option,
        })
    );
}

#[test]
fn visible_event_referencing_owner_only_event_is_rejected() {
    let content = manifest(&[("event", "event:one"), ("event", "event:linked")]);
    let mut first = simple_event("event:one");
    first.references = vec![reference(EventSemanticReferenceKind::Event, "event:linked")];
    let mut second = simple_event("event:linked");
    second.visibility = EventVisibility::OwnerOnly;
    assert_eq!(
        produce(&content, vec![first, second]),
        Err(EventCatalogError::HiddenReferenceLeak {
            event_id: "event:one".to_owned(),
            reference_kind: EventSemanticReferenceKind::Event,
        })
    );
}

fn hidden_and_empty_catalog() -> EventCatalog {
    let content = manifest(&[("event", "event:hidden"), ("event", "event:empty")]);
    let mut hidden = simple_event("event:hidden");
    hidden.eligibility[0].visibility = EventVisibility::Hidden;
    hidden.options[0].outcomes[0].visibility = EventVisibility::Hidden;
    let mut empty = simple_event("event:empty");
    empty.eligibility.clear();
    empty.options[0].outcomes.clear();
    produce(&content, vec![hidden, empty]).expect("catalog")
}

#[test]
fn withheld_collections_are_distinct_from_observed_empty() {
    let catalog = hidden_and_empty_catalog();
    let hidden = catalog
        .get(
            &event_reference(&catalog, "event:hidden"),
            EventVisibilityScope::Owner,
        )
        .expect("hidden event");
    assert_eq!(hidden.eligibility_status, EventFieldStatus::Denied);
    assert!(hidden.eligibility.is_empty());
    let empty = catalog
        .get(
            &event_reference(&catalog, "event:empty"),
            EventVisibilityScope::Owner,
        )
        .expect("empty event");
    assert_eq!(empty.eligibility_status, EventFieldStatus::Available);
    assert!(empty.eligibility.is_empty());

    let hidden_option = catalog
        .get_option(
            &option_reference(&catalog, "event:hidden", "option:leave"),
            EventVisibilityScope::Owner,
        )
        .expect("hidden option");
    assert_eq!(hidden_option.outcomes_status, EventFieldStatus::Denied);
    assert!(hidden_option.outcomes.is_empty());
    let empty_option = catalog
        .get_option(
            &option_reference(&catalog, "event:empty", "option:leave"),
            EventVisibilityScope::Owner,
        )
        .expect("empty option");
    assert_eq!(empty_option.outcomes_status, EventFieldStatus::Available);
    assert!(empty_option.outcomes.is_empty());
    assert_eq!(empty_option.costs_status, EventFieldStatus::Available);
}

#[test]
fn partially_withheld_collections_report_partial() {
    let content = manifest(&[("event", "event:partial")]);
    let mut definition = simple_event("event:partial");
    let visible_cost = cost("cost:a", EventCostKind::Gold, 10);
    let mut hidden_cost = cost("cost:b", EventCostKind::Gold, 5);
    hidden_cost.visibility = EventVisibility::Hidden;
    definition.options[0].costs = vec![visible_cost, hidden_cost];
    let catalog = produce(&content, vec![definition]).expect("catalog");
    let option = catalog
        .get_option(
            &option_reference(&catalog, "event:partial", "option:leave"),
            EventVisibilityScope::Owner,
        )
        .expect("option");
    assert_eq!(option.costs_status, EventFieldStatus::Partial);
    assert_eq!(option.costs.len(), 1);
    assert_eq!(option.costs[0].cost_id, "cost:a");
}

#[test]
fn list_summaries_distinguish_withheld_from_empty_eligibility() {
    let catalog = hidden_and_empty_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list(&list_query("en-US", EventVisibilityScope::Owner, 8))
        .expect("list");
    let hidden = page
        .entries
        .iter()
        .find(|entry| entry.reference.event_id == "event:hidden")
        .expect("hidden summary");
    assert_eq!(hidden.eligibility, EventFieldStatus::Denied);
    let empty = page
        .entries
        .iter()
        .find(|entry| entry.reference.event_id == "event:empty")
        .expect("empty summary");
    assert_eq!(empty.eligibility, EventFieldStatus::Available);
}
