// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    EventCatalog, EventCatalogError, EventCatalogProducer, EventDefinitionInput,
    EventSemanticReferenceKind, EventVisibility, EventVisibilityScope,
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

fn event_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("event", "event:one")])
}

#[test]
fn page_offering_unknown_option_is_rejected() {
    let content = event_manifest();
    let mut definition = simple_event("event:one");
    definition.pages[0]
        .offered_options
        .push("option:ghost".to_owned());
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::UnknownOptionReference {
            event_id: "event:one".to_owned(),
            option_id: "option:ghost".to_owned(),
        })
    );
}

#[test]
fn uncovered_option_is_rejected() {
    let content = event_manifest();
    let mut definition = simple_event("event:one");
    definition.pages[0].offered_options.clear();
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::UncoveredOption {
            event_id: "event:one".to_owned(),
            option_id: "option:leave".to_owned(),
        })
    );
}

#[test]
fn option_offered_by_two_pages_is_rejected() {
    let content = event_manifest();
    let mut definition = simple_event("event:one");
    definition
        .pages
        .push(page_offering("page:second", &["option:leave"]));
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::DuplicateOptionMembership {
            event_id: "event:one".to_owned(),
            option_id: "option:leave".to_owned(),
        })
    );
}

#[test]
fn option_offered_twice_on_one_page_is_rejected() {
    let content = event_manifest();
    let mut definition = simple_event("event:one");
    definition.pages[0].offered_options =
        vec!["option:leave".to_owned(), "option:leave".to_owned()];
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::InvalidInput("duplicate_page_option"))
    );
}

#[test]
fn page_more_visible_than_offered_option_is_rejected() {
    let content = event_manifest();
    let mut definition = simple_event("event:one");
    definition.options[0].visibility = EventVisibility::OwnerOnly;
    assert_eq!(
        produce(&content, vec![definition]),
        Err(EventCatalogError::HiddenReferenceLeak {
            event_id: "event:one".to_owned(),
            reference_kind: EventSemanticReferenceKind::Option,
        })
    );
}

#[test]
fn page_option_membership_is_readable() {
    let content = manifest(&[
        ("event", "event:one"),
        ("enemy", "enemy:slime"),
        ("relic", "relic:shrine"),
    ]);
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    let definition = catalog
        .get(
            &event_reference(&catalog, "event:one"),
            EventVisibilityScope::Owner,
        )
        .expect("event");
    assert_eq!(
        definition.pages[0].offered_options,
        ["option:offer"],
        "the start page offers the branching option"
    );
    assert_eq!(definition.pages[1].page_id, "page:reward");
    assert_eq!(
        definition.pages[1].offered_options,
        ["option:leave"],
        "the reward page offers its own follow-up option"
    );
}
