// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, EVENT_MAX_DEFINITION_BYTES, EVENT_MAX_IDENTITY_BYTES, EVENT_MAX_OPTIONS,
    EVENT_MAX_REQUIREMENTS, EventCatalog, EventCatalogError, EventCatalogProducer,
    EventDefinitionInput, EventField, EventFieldStatus, EventFollowUp, EventKind, EventProbability,
    EventRequirementKind, EventSourceError, EventUnavailableReason, EventVisibility,
    EventVisibilityScope,
};

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definition: EventDefinitionInput,
) -> Result<EventCatalog, EventCatalogError> {
    EventCatalogProducer::new().produce(
        content,
        &EventSource {
            snapshot: Ok(snapshot(content, vec![definition])),
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
fn explicit_unavailable_fields_remain_distinct_from_empty_success() {
    let content = full_manifest();
    let mut definition = rich_event("event:one");
    definition.options[0].costs[0].rule_reference =
        EventField::Unavailable(EventUnavailableReason::Denied);
    definition.options[0].outcomes[0].probability =
        EventProbability::Unavailable(EventUnavailableReason::NotObserved);
    let catalog = catalog(&content, vec![definition]);
    let option = catalog
        .get_option(
            &option_reference(&catalog, "event:one", "option:offer"),
            EventVisibilityScope::Owner,
        )
        .expect("option");
    assert_eq!(
        option.costs[0].rule_reference.status(),
        EventFieldStatus::Denied
    );
    assert_eq!(option.costs[0].rule_reference.value(), None);
    assert_eq!(
        option.outcomes[0].probability,
        EventProbability::Unavailable(EventUnavailableReason::NotObserved)
    );
}

#[test]
fn multi_stage_selector_and_numeric_cost_are_inspectable() {
    let content = full_manifest();
    let catalog = catalog(&content, vec![rich_event("event:one")]);
    let definition = catalog
        .get(
            &event_reference(&catalog, "event:one"),
            EventVisibilityScope::Owner,
        )
        .expect("event");
    assert_eq!(definition.pages.len(), 2);
    let option = catalog
        .get_option(
            &option_reference(&catalog, "event:one", "option:offer"),
            EventVisibilityScope::Owner,
        )
        .expect("option");
    assert_eq!(
        option.costs[0].amount,
        sts2_game_mod::EventNumericValue::Fixed(50)
    );
    assert_eq!(
        option.outcomes[0].follow_up,
        EventFollowUp::Page("page:reward".to_owned())
    );
    assert_eq!(
        definition.pages[0].offered_options,
        ["option:offer"],
        "the start page must offer the branching option"
    );
    assert_eq!(
        definition.pages[1].page_id, "page:reward",
        "the successful branch must reach a distinct page"
    );
    assert_eq!(
        definition.pages[1].offered_options,
        ["option:leave"],
        "the reward page must offer its own follow-up choice"
    );
}

fn bulk_definition(kind_len: usize) -> EventDefinitionInput {
    let options = (0..EVENT_MAX_OPTIONS)
        .map(|option_index| {
            let mut choice = option(&format!("option:{option_index}"));
            choice.requirements = (0..EVENT_MAX_REQUIREMENTS)
                .map(|requirement_index| {
                    let mut req = requirement(
                        &format!("requirement:{option_index}:{requirement_index}"),
                        EventRequirementKind::Custom("k".repeat(kind_len)),
                    );
                    req.parameters.clear();
                    req
                })
                .collect();
            choice
        })
        .collect();
    let mut start = page("page:start");
    start.offered_options = (0..EVENT_MAX_OPTIONS)
        .map(|option_index| format!("option:{option_index}"))
        .collect();
    EventDefinitionInput {
        event_id: "event:bulk".to_owned(),
        title: text("Bulk"),
        kind: EventKind::Normal,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: EventVisibility::Visible,
        pages: vec![start],
        eligibility: vec![requirement(
            "requirement:act",
            EventRequirementKind::Progression,
        )],
        options,
        references: Vec::new(),
    }
}

fn bulk_actual(kind_len: usize) -> usize {
    let content = manifest(&[("event", "event:bulk")]);
    match produce(&content, bulk_definition(kind_len)) {
        Err(EventCatalogError::DefinitionTooLarge { actual, .. }) => actual,
        other => {
            assert!(matches!(
                other,
                Err(EventCatalogError::DefinitionTooLarge { .. })
            ));
            0
        }
    }
}

#[test]
fn definition_byte_limit_counts_nested_custom_kind_strings() {
    let long = bulk_actual(EVENT_MAX_IDENTITY_BYTES);
    let short = bulk_actual(EVENT_MAX_IDENTITY_BYTES - 6);
    assert!(long > EVENT_MAX_DEFINITION_BYTES);
    assert_eq!(
        long - short,
        EVENT_MAX_OPTIONS * EVENT_MAX_REQUIREMENTS * 6,
        "every nested custom requirement-kind byte must count toward the definition bound"
    );
}

#[test]
fn source_failures_map_to_typed_errors() {
    let content = manifest(&[]);
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Err(EventSourceError::Malformed),
            }
        ),
        Err(EventCatalogError::MalformedSource)
    );
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Err(EventSourceError::NoActiveSource),
            }
        ),
        Err(EventCatalogError::NoActiveSource)
    );
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Err(EventSourceError::AccessDenied),
            }
        ),
        Err(EventCatalogError::SourceAccessDenied)
    );
}
