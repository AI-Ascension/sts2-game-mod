// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    EVENT_MAX_OPTIONS, EVENT_MAX_PAGES, EVENT_MAX_TEXT_BYTES, EventCatalog, EventCatalogError,
    EventCatalogProducer, EventCostKind, EventDefinitionInput, EventEffectKind, EventFamilyState,
    EventFollowUp, EventRequirementKind, EventSemanticReferenceKind, EventText,
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
fn manifest_locale_and_producer_fences_are_rejected() {
    let content = full_manifest();
    let other = manifest(&[
        ("event", "event:two"),
        ("enemy", "enemy:slime"),
        ("relic", "relic:shrine"),
    ]);

    let mut wrong_manifest = snapshot(&content, vec![rich_event("event:one")]);
    wrong_manifest.manifest = other.cursor_binding();
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(wrong_manifest),
            }
        ),
        Err(EventCatalogError::ManifestMismatch)
    );

    let mut wrong_locale = snapshot(&content, vec![rich_event("event:one")]);
    wrong_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(wrong_locale),
            }
        ),
        Err(EventCatalogError::LocaleMismatch)
    );

    let mut wrong_producer = snapshot(&content, vec![rich_event("event:one")]);
    wrong_producer.producer_version = "game-event-reference-producer-v9".to_owned();
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(wrong_producer),
            }
        ),
        Err(EventCatalogError::ProducerVersionMismatch)
    );

    let mut wrong_family = snapshot(&content, vec![rich_event("event:one")]);
    wrong_family.family.entity_kind = "enemy".to_owned();
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(wrong_family),
            }
        ),
        Err(EventCatalogError::FamilyIdentityMismatch)
    );
}

#[test]
fn unsupported_and_unavailable_families_fail_closed() {
    let content = manifest(&[]);
    let mut unsupported = snapshot(&content, Vec::new());
    unsupported.family.state = EventFamilyState::Unsupported;
    let catalog = EventCatalogProducer::new()
        .produce(
            &content,
            &EventSource {
                snapshot: Ok(unsupported),
            },
        )
        .expect("unsupported catalog");
    assert_eq!(catalog.family().state, EventFamilyState::Unsupported);
    assert_eq!(
        catalog.reader().list(&list_query(
            "en-US",
            sts2_game_mod::EventVisibilityScope::Owner,
            8
        )),
        Err(EventCatalogError::UnsupportedFamily)
    );
    assert_eq!(
        catalog.get(
            &event_reference(&catalog, "event:x"),
            sts2_game_mod::EventVisibilityScope::Owner
        ),
        Err(EventCatalogError::UnsupportedFamily)
    );

    let mut unavailable = snapshot(&content, Vec::new());
    unavailable.family.state = EventFamilyState::Unavailable;
    let catalog = EventCatalogProducer::new()
        .produce(
            &content,
            &EventSource {
                snapshot: Ok(unavailable),
            },
        )
        .expect("unavailable catalog");
    assert_eq!(catalog.family().state, EventFamilyState::Unavailable);

    let content = manifest(&[("event", "event:one")]);
    let mut carrying = snapshot(&content, vec![simple_event("event:one")]);
    carrying.family.state = EventFamilyState::Unsupported;
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(carrying),
            }
        ),
        Err(EventCatalogError::UnknownDefinition("event:one".to_owned()))
    );
}

#[test]
fn missing_family_and_family_count_are_rejected() {
    let mut source = manifest_source(&[("enemy", "enemy:slime")]);
    source
        .snapshot
        .available_entity_kinds
        .retain(|kind| kind != "event");
    source.snapshot.registry_definition_counts.remove("event");
    let no_event = sts2_game_mod::ContentManifestProducer::new("adapter-v1", ["enemy".to_owned()])
        .expect("producer")
        .produce(&source)
        .expect("manifest");
    assert_eq!(
        EventCatalogProducer::new().produce(
            &no_event,
            &EventSource {
                snapshot: Ok(snapshot(&no_event, Vec::new())),
            }
        ),
        Err(EventCatalogError::MissingFamily)
    );

    let content = manifest(&[("event", "event:one")]);
    let mut wrong_count = snapshot(&content, vec![simple_event("event:one")]);
    wrong_count.family.definition_count = 5;
    assert_eq!(
        EventCatalogProducer::new().produce(
            &content,
            &EventSource {
                snapshot: Ok(wrong_count),
            }
        ),
        Err(EventCatalogError::FamilyCountMismatch)
    );
}

#[test]
fn duplicate_identities_are_rejected() {
    let content = manifest(&[("event", "event:dup")]);
    let mut two = snapshot(&content, vec![simple_event("event:dup")]);
    two.definitions.push(simple_event("event:dup"));
    assert_eq!(
        EventCatalogProducer::new().produce(&content, &EventSource { snapshot: Ok(two) }),
        Err(EventCatalogError::DuplicateDefinition(
            "event:dup".to_owned()
        ))
    );

    let mut duplicate_page = simple_event("event:dup");
    duplicate_page.pages.push(page("page:start"));
    assert_eq!(
        produce(&content, duplicate_page),
        Err(EventCatalogError::InvalidInput("duplicate_page"))
    );

    let mut duplicate_option = simple_event("event:dup");
    duplicate_option.options.push(option("option:leave"));
    assert_eq!(
        produce(&content, duplicate_option),
        Err(EventCatalogError::InvalidInput("duplicate_option"))
    );

    let mut duplicate_requirement = simple_event("event:dup");
    duplicate_requirement.eligibility.push(requirement(
        "requirement:act",
        EventRequirementKind::Progression,
    ));
    assert_eq!(
        produce(&content, duplicate_requirement),
        Err(EventCatalogError::InvalidInput("duplicate_requirement"))
    );

    let mut duplicate_cost = simple_event("event:dup");
    duplicate_cost.options[0]
        .costs
        .push(cost("cost:gold", EventCostKind::Gold, 10));
    duplicate_cost.options[0]
        .costs
        .push(cost("cost:gold", EventCostKind::Gold, 10));
    assert_eq!(
        produce(&content, duplicate_cost),
        Err(EventCatalogError::InvalidInput("duplicate_cost"))
    );

    let mut duplicate_outcome = simple_event("event:dup");
    duplicate_outcome.options[0]
        .outcomes
        .push(outcome("outcome:one", EventFollowUp::End));
    assert_eq!(
        produce(&content, duplicate_outcome),
        Err(EventCatalogError::InvalidInput("duplicate_outcome"))
    );

    let mut duplicate_effect = simple_event("event:dup");
    duplicate_effect.options[0].outcomes[0]
        .effects
        .push(effect("effect:one", EventEffectKind::GainGold));
    assert_eq!(
        produce(&content, duplicate_effect),
        Err(EventCatalogError::InvalidInput("duplicate_effect"))
    );

    let mut duplicate_reference = simple_event("event:dup");
    duplicate_reference.references = vec![
        reference(EventSemanticReferenceKind::Enemy, "enemy:slime"),
        reference(EventSemanticReferenceKind::Enemy, "enemy:slime"),
    ];
    assert_eq!(
        produce(&content, duplicate_reference),
        Err(EventCatalogError::InvalidInput("duplicate_reference"))
    );
}

#[test]
fn malformed_identities_and_collection_bounds_are_rejected() {
    let content = manifest(&[("event", "event:ok")]);

    let mut bad_identity = simple_event("event:ok");
    bad_identity.event_id = "event:bad\u{1}".to_owned();
    assert_eq!(
        produce(&content, bad_identity),
        Err(EventCatalogError::InvalidInput("event_id"))
    );

    let mut empty_title = simple_event("event:ok");
    empty_title.title = EventText::Available(String::new());
    assert_eq!(
        produce(&content, empty_title),
        Err(EventCatalogError::InvalidInput("title"))
    );

    let mut oversized_title = simple_event("event:ok");
    oversized_title.title = EventText::Available("x".repeat(EVENT_MAX_TEXT_BYTES + 1));
    assert_eq!(
        produce(&content, oversized_title),
        Err(EventCatalogError::InvalidInput("title"))
    );

    let mut too_many_pages = simple_event("event:ok");
    too_many_pages.pages = (0..EVENT_MAX_PAGES + 1)
        .map(|index| page(&format!("page:{index}")))
        .collect();
    assert_eq!(
        produce(&content, too_many_pages),
        Err(EventCatalogError::InvalidInput("pages"))
    );

    let mut too_many_options = simple_event("event:ok");
    too_many_options.options = (0..EVENT_MAX_OPTIONS + 1)
        .map(|index| option(&format!("option:{index}")))
        .collect();
    assert_eq!(
        produce(&content, too_many_options),
        Err(EventCatalogError::InvalidInput("options"))
    );
}
