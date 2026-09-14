// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    ContentUnlockState, EVENT_REFERENCE_ENTITY_KIND, EVENT_REFERENCE_PRODUCER_VERSION,
    EventCatalog, EventCatalogProducer, EventCatalogSnapshot, EventCatalogSource, EventCost,
    EventCostKind, EventDefinitionInput, EventDefinitionReference, EventEffect, EventEffectKind,
    EventEvidence, EventFamilyCoverage, EventFamilyState, EventField, EventFollowUp,
    EventNarrativePage, EventNumericValue, EventOptionInput, EventOptionReference,
    EventOutcomeInput, EventPageReference, EventParameter, EventProbability, EventRequirement,
    EventRequirementKind, EventSemanticReference, EventSemanticReferenceKind, EventSourceError,
    EventText, EventVisibility, EventVisibilityScope,
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

pub fn manifest_source(entries: &[(&str, &str)]) -> ManifestSource {
    let definitions = entries
        .iter()
        .map(|(kind, id)| content_definition(kind, id))
        .collect();
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
    if !available_entity_kinds.iter().any(|kind| kind == "event") {
        available_entity_kinds.push("event".to_owned());
        available_entity_kinds.sort();
    }
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 7,
            generation_after: 7,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            available_entity_kinds,
            definitions,
        },
    }
}

pub fn manifest(entries: &[(&str, &str)]) -> ContentManifest {
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.push("event".to_owned());
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
    ContentManifestProducer::new("adapter-v1", available_entity_kinds)
        .expect("producer")
        .produce(&manifest_source(entries))
        .expect("manifest")
}

pub fn text(value: &str) -> EventText {
    EventText::available(value).expect("text")
}

pub fn reference(kind: EventSemanticReferenceKind, id: &str) -> EventSemanticReference {
    EventSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn parameter(parameter_id: &str, value: i64) -> EventParameter {
    EventParameter {
        parameter_id: parameter_id.to_owned(),
        label: text(parameter_id),
        unit: Some("count".to_owned()),
        value: EventNumericValue::Fixed(value),
    }
}

pub fn requirement(requirement_id: &str, kind: EventRequirementKind) -> EventRequirement {
    EventRequirement {
        requirement_id: requirement_id.to_owned(),
        kind,
        label: text(requirement_id),
        parameters: vec![parameter("min", 1)],
        references: Vec::new(),
        visibility: EventVisibility::Visible,
    }
}

pub fn cost(cost_id: &str, kind: EventCostKind, amount: i64) -> EventCost {
    EventCost {
        cost_id: cost_id.to_owned(),
        kind,
        label: text(cost_id),
        amount: EventNumericValue::Fixed(amount),
        resource: EventField::Available("resource:gold".to_owned()),
        rule_reference: EventField::Available("rule:cost".to_owned()),
        references: Vec::new(),
        evidence: EventEvidence::SourceDerived,
        visibility: EventVisibility::Visible,
    }
}

pub fn effect(effect_id: &str, kind: EventEffectKind) -> EventEffect {
    EventEffect {
        effect_id: effect_id.to_owned(),
        kind,
        label: text(effect_id),
        amount: EventNumericValue::Fixed(1),
        target: EventField::Available("target:player".to_owned()),
        rule_reference: EventField::Available("rule:effect".to_owned()),
        references: Vec::new(),
        evidence: EventEvidence::SourceDerived,
        visibility: EventVisibility::Visible,
    }
}

pub fn exact_probability(numerator: u32, denominator: u32) -> EventProbability {
    EventProbability::Exact {
        numerator,
        denominator,
        evidence: EventEvidence::SourceDerived,
    }
}

pub fn rule_probability(rule_reference: &str) -> EventProbability {
    EventProbability::Rule {
        rule_reference: rule_reference.to_owned(),
        evidence: EventEvidence::IndependentlyAuthored,
    }
}

pub fn outcome(outcome_id: &str, follow_up: EventFollowUp) -> EventOutcomeInput {
    EventOutcomeInput {
        outcome_id: outcome_id.to_owned(),
        label: text(outcome_id),
        probability: exact_probability(1, 1),
        effects: vec![effect("effect:one", EventEffectKind::GainGold)],
        follow_up,
        references: Vec::new(),
        visibility: EventVisibility::Visible,
    }
}

pub fn option(option_id: &str) -> EventOptionInput {
    EventOptionInput {
        option_id: option_id.to_owned(),
        text: text(option_id),
        requirements: Vec::new(),
        costs: Vec::new(),
        outcomes: vec![outcome("outcome:one", EventFollowUp::End)],
        references: Vec::new(),
        visibility: EventVisibility::Visible,
    }
}

pub fn page(page_id: &str) -> EventNarrativePage {
    EventNarrativePage {
        page_id: page_id.to_owned(),
        narrative: text(page_id),
        references: Vec::new(),
        visibility: EventVisibility::Visible,
    }
}

pub fn simple_event(event_id: &str) -> EventDefinitionInput {
    EventDefinitionInput {
        event_id: event_id.to_owned(),
        title: text(event_id),
        kind: sts2_game_mod::EventKind::Normal,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: EventVisibility::Visible,
        pages: vec![page("page:start")],
        eligibility: vec![requirement(
            "requirement:act",
            EventRequirementKind::Progression,
        )],
        options: vec![option("option:leave")],
        references: Vec::new(),
    }
}

pub fn rich_event(event_id: &str) -> EventDefinitionInput {
    let mut offer = option("option:offer");
    offer.requirements = vec![requirement(
        "requirement:gold",
        EventRequirementKind::Resource,
    )];
    offer.costs = vec![cost("cost:gold", EventCostKind::Gold, 50)];
    offer.outcomes = vec![
        outcome(
            "outcome:success",
            EventFollowUp::Page("page:reward".to_owned()),
        ),
        outcome("outcome:fail", EventFollowUp::End),
    ];
    offer.outcomes[0].probability = exact_probability(1, 2);
    offer.outcomes[0].effects = vec![
        effect("effect:relic", EventEffectKind::GainRelic),
        effect("effect:max-hp", EventEffectKind::MaxHpChange),
    ];
    offer.outcomes[0].effects[0].references =
        vec![reference(EventSemanticReferenceKind::Relic, "relic:shrine")];
    offer.references = vec![reference(
        EventSemanticReferenceKind::Option,
        "option:leave",
    )];
    let leave = option("option:leave");
    EventDefinitionInput {
        event_id: event_id.to_owned(),
        title: text("Synthetic Event"),
        kind: sts2_game_mod::EventKind::Normal,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: EventVisibility::Visible,
        pages: vec![page("page:start"), page("page:reward")],
        eligibility: vec![requirement(
            "requirement:act",
            EventRequirementKind::Progression,
        )],
        options: vec![offer, leave],
        references: vec![reference(EventSemanticReferenceKind::Enemy, "enemy:slime")],
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<EventDefinitionInput>,
) -> EventCatalogSnapshot {
    EventCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: EVENT_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: EventFamilyCoverage {
            entity_kind: EVENT_REFERENCE_ENTITY_KIND.to_owned(),
            state: EventFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSource {
    pub snapshot: Result<EventCatalogSnapshot, EventSourceError>,
}

impl EventCatalogSource for EventSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<EventCatalogSnapshot, EventSourceError> {
        self.snapshot.clone()
    }
}

pub fn catalog(manifest: &ContentManifest, definitions: Vec<EventDefinitionInput>) -> EventCatalog {
    EventCatalogProducer::new()
        .produce(
            manifest,
            &EventSource {
                snapshot: Ok(snapshot(manifest, definitions)),
            },
        )
        .expect("catalog")
}

pub fn event_reference(catalog: &EventCatalog, event_id: &str) -> EventDefinitionReference {
    EventDefinitionReference {
        catalog: catalog.binding().clone(),
        event_id: event_id.to_owned(),
    }
}

pub fn page_reference(catalog: &EventCatalog, event_id: &str, page_id: &str) -> EventPageReference {
    EventPageReference {
        catalog: catalog.binding().clone(),
        event_id: event_id.to_owned(),
        page_id: page_id.to_owned(),
    }
}

pub fn option_reference(
    catalog: &EventCatalog,
    event_id: &str,
    option_id: &str,
) -> EventOptionReference {
    EventOptionReference {
        catalog: catalog.binding().clone(),
        event_id: event_id.to_owned(),
        option_id: option_id.to_owned(),
    }
}

pub fn list_query(
    locale: &str,
    scope: EventVisibilityScope,
    limit: usize,
) -> sts2_game_mod::EventListQuery {
    sts2_game_mod::EventListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}
