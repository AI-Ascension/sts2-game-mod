// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/event_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    EventCatalog, EventCatalogError, EventCatalogProducer, EventCostKind, EventDefinitionInput,
    EventEffectKind, EventEvidence, EventField, EventFollowUp, EventNumericValue, EventProbability,
    EventSemanticReferenceKind, EventVisibility,
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
fn dangling_manifest_references_are_rejected() {
    let content = full_manifest();

    let mut unknown_enemy = rich_event("event:one");
    unknown_enemy.references = vec![reference(
        EventSemanticReferenceKind::Enemy,
        "enemy:missing",
    )];
    assert_eq!(
        produce(&content, unknown_enemy),
        Err(EventCatalogError::UnknownManifestReference {
            entity_kind: "enemy".to_owned(),
            namespaced_id: "enemy:missing".to_owned(),
        })
    );

    let mut unknown_relic = rich_event("event:one");
    unknown_relic.options[0].outcomes[0].effects[0].references = vec![reference(
        EventSemanticReferenceKind::Relic,
        "relic:missing",
    )];
    assert_eq!(
        produce(&content, unknown_relic),
        Err(EventCatalogError::UnknownManifestReference {
            entity_kind: "relic".to_owned(),
            namespaced_id: "relic:missing".to_owned(),
        })
    );
}

#[test]
fn dangling_intra_event_references_are_rejected() {
    let content = full_manifest();

    let mut unknown_page = rich_event("event:one");
    unknown_page.options[0].outcomes[0].follow_up = EventFollowUp::Page("page:missing".to_owned());
    assert_eq!(
        produce(&content, unknown_page),
        Err(EventCatalogError::UnknownPageReference {
            event_id: "event:one".to_owned(),
            page_id: "page:missing".to_owned(),
        })
    );

    let mut unknown_page_reference = rich_event("event:one");
    unknown_page_reference.pages[0].references =
        vec![reference(EventSemanticReferenceKind::Page, "page:missing")];
    assert_eq!(
        produce(&content, unknown_page_reference),
        Err(EventCatalogError::UnknownPageReference {
            event_id: "event:one".to_owned(),
            page_id: "page:missing".to_owned(),
        })
    );

    let mut unknown_option_reference = rich_event("event:one");
    unknown_option_reference.options[0].references = vec![reference(
        EventSemanticReferenceKind::Option,
        "option:missing",
    )];
    assert_eq!(
        produce(&content, unknown_option_reference),
        Err(EventCatalogError::UnknownOptionReference {
            event_id: "event:one".to_owned(),
            option_id: "option:missing".to_owned(),
        })
    );
}

#[test]
fn manifest_event_references_resolve_across_definitions() {
    let content = manifest(&[("event", "event:one"), ("event", "event:linked")]);
    let mut first = simple_event("event:one");
    first.references = vec![reference(EventSemanticReferenceKind::Event, "event:linked")];
    let catalog = fixture::catalog(&content, vec![first, simple_event("event:linked")]);
    assert_eq!(catalog.family().definition_count, 2);
    let definition = catalog
        .get(
            &event_reference(&catalog, "event:one"),
            sts2_game_mod::EventVisibilityScope::Owner,
        )
        .expect("event");
    assert_eq!(definition.references.len(), 1);

    let content = manifest(&[("event", "event:one")]);
    let mut dangling = simple_event("event:one");
    dangling.references = vec![reference(
        EventSemanticReferenceKind::Event,
        "event:missing",
    )];
    assert_eq!(
        produce(&content, dangling),
        Err(EventCatalogError::UnknownManifestReference {
            entity_kind: "event".to_owned(),
            namespaced_id: "event:missing".to_owned(),
        })
    );
}

#[test]
fn hidden_future_leak_is_rejected() {
    let content = full_manifest();
    let mut definition = rich_event("event:one");
    let mut secret = page("page:secret");
    secret.visibility = EventVisibility::Hidden;
    definition.pages.push(secret);
    definition.options[0].outcomes[0].follow_up = EventFollowUp::Page("page:secret".to_owned());
    assert_eq!(
        produce(&content, definition),
        Err(EventCatalogError::HiddenFutureLeak {
            event_id: "event:one".to_owned(),
            page_id: "page:secret".to_owned(),
        })
    );
}

#[test]
fn invalid_probability_costs_and_effects_are_rejected() {
    let content = full_manifest();

    let mut bad_probability = rich_event("event:one");
    bad_probability.options[0].outcomes[0].probability = EventProbability::Exact {
        numerator: 1,
        denominator: 0,
        evidence: EventEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&content, bad_probability),
        Err(EventCatalogError::InvalidInput("probability"))
    );

    let mut bad_rule = rich_event("event:one");
    bad_rule.options[0].outcomes[0].probability = EventProbability::Rule {
        rule_reference: String::new(),
        evidence: EventEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&content, bad_rule),
        Err(EventCatalogError::InvalidInput("probability_rule"))
    );

    let mut bad_resource = rich_event("event:one");
    bad_resource.options[0].costs[0].resource = EventField::Available(String::new());
    assert_eq!(
        produce(&content, bad_resource),
        Err(EventCatalogError::InvalidInput("cost_resource"))
    );

    let mut bad_effect_kind = rich_event("event:one");
    bad_effect_kind.options[0].outcomes[0].effects[0].kind = EventEffectKind::Custom(String::new());
    assert_eq!(
        produce(&content, bad_effect_kind),
        Err(EventCatalogError::InvalidInput("effect_kind"))
    );

    let mut bad_visibility = rich_event("event:one");
    bad_visibility.options[0].outcomes[0].visibility = EventVisibility::Unknown;
    assert_eq!(
        produce(&content, bad_visibility),
        Err(EventCatalogError::InvalidInput("visibility"))
    );
}

#[test]
fn invalid_amount_signs_are_rejected_per_kind() {
    let content = full_manifest();

    let mut negative_gold_cost = rich_event("event:one");
    negative_gold_cost.options[0].costs[0].amount = EventNumericValue::Fixed(-50);
    assert_eq!(
        produce(&content, negative_gold_cost),
        Err(EventCatalogError::InvalidInput("cost_amount"))
    );

    let mut negative_hp_cost = rich_event("event:one");
    negative_hp_cost.options[0].costs[0].kind = EventCostKind::HpLoss;
    negative_hp_cost.options[0].costs[0].amount = EventNumericValue::Fixed(-1);
    assert_eq!(
        produce(&content, negative_hp_cost),
        Err(EventCatalogError::InvalidInput("cost_amount"))
    );

    let mut negative_gain = rich_event("event:one");
    negative_gain.options[0].outcomes[0].effects[0].amount = EventNumericValue::Fixed(-1);
    assert_eq!(
        produce(&content, negative_gain),
        Err(EventCatalogError::InvalidInput("effect_amount"))
    );

    let mut negative_heal = rich_event("event:one");
    negative_heal.options[0].outcomes[0].effects[0].kind = EventEffectKind::Heal;
    negative_heal.options[0].outcomes[0].effects[0].amount = EventNumericValue::Fixed(-1);
    assert_eq!(
        produce(&content, negative_heal),
        Err(EventCatalogError::InvalidInput("effect_amount"))
    );
}

#[test]
fn signed_and_unspecified_amounts_accept_negative_values() {
    let content = full_manifest();

    let mut max_hp = rich_event("event:one");
    max_hp.options[0].outcomes[0].effects[1].kind = EventEffectKind::MaxHpChange;
    max_hp.options[0].outcomes[0].effects[1].amount = EventNumericValue::Fixed(-5);
    assert!(produce(&content, max_hp).is_ok());

    let mut custom_effect = rich_event("event:one");
    custom_effect.options[0].outcomes[0].effects[0].kind =
        EventEffectKind::Custom("owner".to_owned());
    custom_effect.options[0].outcomes[0].effects[0].amount = EventNumericValue::Fixed(-5);
    assert!(produce(&content, custom_effect).is_ok());

    let mut custom_cost = rich_event("event:one");
    custom_cost.options[0].costs[0].kind = EventCostKind::Custom("owner".to_owned());
    custom_cost.options[0].costs[0].amount = EventNumericValue::Fixed(-5);
    assert!(produce(&content, custom_cost).is_ok());
}
