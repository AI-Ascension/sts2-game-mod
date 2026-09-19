// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    FixtureRestSiteSource, REST_REFERENCE_CARD_KIND, REST_REFERENCE_EFFECT_KIND,
    REST_REFERENCE_ENTITY_KIND, REST_REFERENCE_PLAYER_KIND, REST_REFERENCE_POTION_KIND,
    REST_REFERENCE_PRODUCER_VERSION, REST_REFERENCE_RELIC_KIND, RestActionReference, RestCandidate,
    RestCost, RestCostUnit, RestCoverageRecord, RestCoverageState, RestEffect, RestEffectKind,
    RestEvidence, RestFamilyCoverage, RestFamilyState, RestField, RestHealAmount, RestHealModifier,
    RestHealModifierKind, RestLimit, RestLimitUnit, RestOptionAvailability, RestOptionInput,
    RestOptionKind, RestOptionState, RestReferenceKind, RestRequirement, RestRequirementState,
    RestRoundingMode, RestSelectionDomain, RestSelectionRequirement, RestSemanticReference,
    RestSiteCatalog, RestSiteCatalogProducer, RestSiteCatalogSnapshot, RestSiteDefinitionInput,
    RestSiteError, RestSiteKind, RestText, RestUnavailableReason, RestVisibility,
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

/// Builds a manifest from explicit `(entity_kind, id)` pairs.
pub fn manifest_of(definitions: &[(&str, &str)]) -> ContentManifest {
    manifest_of_with_kinds(definitions, &[])
}

/// Builds a manifest from explicit pairs plus families registered without a definition.
pub fn manifest_of_with_kinds(
    definitions: &[(&str, &str)],
    extra_kinds: &[&str],
) -> ContentManifest {
    let mut kinds: Vec<String> = extra_kinds.iter().map(|kind| (*kind).to_owned()).collect();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for kind in extra_kinds {
        counts.insert((*kind).to_owned(), 0);
    }
    for (entity_kind, _) in definitions {
        if !kinds.iter().any(|kind| kind == entity_kind) {
            kinds.push((*entity_kind).to_owned());
        }
        *counts.entry((*entity_kind).to_owned()).or_insert(0) += 1;
    }
    let snapshot = ContentCatalogSnapshot {
        generation_before: 11,
        generation_after: 11,
        game_build: "sts2-build:synthetic".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![ContentPackageInput {
            package_id: "base:synthetic".to_owned(),
            package_version: Some("1".to_owned()),
            order: 0,
        }],
        available_entity_kinds: kinds.clone(),
        registry_definition_counts: counts,
        definitions: definitions
            .iter()
            .map(|(entity_kind, id)| content_definition(entity_kind, id))
            .collect(),
    };
    ContentManifestProducer::new("adapter-v1", kinds)
        .expect("producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

/// Manifest declaring the rest-site family plus the families the fixture references.
pub fn manifest(sites: &[&str]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = sites
        .iter()
        .map(|site_id| (REST_REFERENCE_ENTITY_KIND, *site_id))
        .collect();
    definitions.extend(item_definitions());
    manifest_of(&definitions)
}

/// Manifest that inventories the rest-site family with no rest-site definition at all.
pub fn manifest_without_sites() -> ContentManifest {
    manifest_of_with_kinds(&item_definitions(), &[REST_REFERENCE_ENTITY_KIND])
}

/// The definition families every fixture reference resolves to.
pub fn item_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        (REST_REFERENCE_EFFECT_KIND, "rest-effect.heal"),
        (REST_REFERENCE_EFFECT_KIND, "rest-effect.smith"),
        (REST_REFERENCE_EFFECT_KIND, "rest-effect.mend"),
        (REST_REFERENCE_EFFECT_KIND, "rest-effect.lift"),
        (REST_REFERENCE_CARD_KIND, "card.strike"),
        (REST_REFERENCE_RELIC_KIND, "relic.burning_blood"),
        (REST_REFERENCE_POTION_KIND, "potion.fire"),
        (REST_REFERENCE_PLAYER_KIND, COOP_PLAYER_ID),
    ]
}

/// Fixture manifest plus extra content definitions, used to shift the manifest fence.
pub fn manifest_with_extra(extra: &[(&str, &str)]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = vec![
        (REST_REFERENCE_ENTITY_KIND, "rest.alpha"),
        (REST_REFERENCE_ENTITY_KIND, "rest.beta"),
        (REST_REFERENCE_ENTITY_KIND, "rest.gamma"),
        (REST_REFERENCE_ENTITY_KIND, "rest.delta"),
    ];
    definitions.extend(item_definitions());
    definitions.extend(extra.iter().copied());
    manifest_of(&definitions)
}

/// Co-op player identity the shared-rest selection resolves to.
pub const COOP_PLAYER_ID: &str = "player.coop.one";

pub fn text(value: &str) -> RestText {
    RestText::Available(value.to_owned())
}

/// Localized text the source knows exists but must not reveal.
pub fn withheld_text() -> RestText {
    RestText::Unavailable(RestUnavailableReason::Withheld)
}

pub fn reference(kind: RestReferenceKind, id: &str) -> RestSemanticReference {
    RestSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn option_reference(option_id: &str) -> RestSemanticReference {
    reference(RestReferenceKind::Option, option_id)
}

/// Transient host action reference the fixture proves cannot enter the static slice.
pub fn action_reference() -> RestActionReference {
    RestActionReference {
        snapshot: sts2_game_mod::RestSnapshotReference {
            run_id: "run.alpha".to_owned(),
            instance_id: "instance:run.alpha".to_owned(),
            epoch: 1,
            snapshot_id: "snapshot.alpha".to_owned(),
        },
        action_id: "action.rest".to_owned(),
    }
}

/// Availability of an option the host offers now, which must carry no refusal reason.
pub fn available_now() -> RestOptionAvailability {
    RestOptionAvailability {
        state: RestOptionState::Available,
        reason: RestField::unavailable(RestUnavailableReason::NotObserved),
        references: Vec::new(),
    }
}

/// Availability of an option the host refuses for an explicit reason.
pub fn disabled_by(reason: sts2_game_mod::RestBlockReason) -> RestOptionAvailability {
    RestOptionAvailability {
        state: RestOptionState::Disabled,
        reason: RestField::available(reason),
        references: Vec::new(),
    }
}

pub fn no_selection() -> RestSelectionRequirement {
    RestSelectionRequirement {
        domain: RestSelectionDomain::None,
        required: false,
        minimum: RestField::unavailable(RestUnavailableReason::NotApplicable),
        maximum: RestField::unavailable(RestUnavailableReason::NotApplicable),
        candidates: Vec::new(),
        references: Vec::new(),
    }
}

pub fn candidate(candidate_id: &str, reference: RestSemanticReference) -> RestCandidate {
    RestCandidate {
        candidate_id: candidate_id.to_owned(),
        label: text(candidate_id),
        reference,
        eligibility: RestRequirementState::Satisfied,
        detail: RestField::available(candidate_id.to_owned()),
        references: Vec::new(),
    }
}

pub fn selection(
    domain: RestSelectionDomain,
    candidates: Vec<RestCandidate>,
) -> RestSelectionRequirement {
    RestSelectionRequirement {
        domain,
        required: true,
        minimum: RestField::available(1),
        maximum: RestField::available(1),
        candidates,
        references: Vec::new(),
    }
}

pub fn requirement(requirement_id: &str, state: RestRequirementState) -> RestRequirement {
    RestRequirement {
        requirement_id: requirement_id.to_owned(),
        label: text(requirement_id),
        state,
        detail: RestField::unavailable(RestUnavailableReason::NotObserved),
        references: Vec::new(),
    }
}

pub fn gold_cost(cost_id: &str, amount: i64) -> RestCost {
    RestCost {
        cost_id: cost_id.to_owned(),
        label: text(cost_id),
        unit: RestCostUnit::Gold,
        amount: RestField::available(amount),
        references: Vec::new(),
    }
}

pub fn per_run_limit(limit_id: &str, remaining: u32) -> RestLimit {
    RestLimit {
        limit_id: limit_id.to_owned(),
        label: text(limit_id),
        unit: RestLimitUnit::PerRun,
        remaining: RestField::available(remaining),
        references: Vec::new(),
    }
}

pub fn heal_modifier(
    modifier_id: &str,
    kind: RestHealModifierKind,
    percent: i32,
) -> RestHealModifier {
    RestHealModifier {
        modifier_id: modifier_id.to_owned(),
        label: text(modifier_id),
        kind,
        percent: RestField::available(percent),
        references: Vec::new(),
    }
}

/// Documented healing amount whose contributors stay separate from its resolved total.
pub fn heal_amount(
    base_percent: u32,
    modifiers: Vec<RestHealModifier>,
    resolved_percent: u32,
) -> RestHealAmount {
    RestHealAmount {
        base_percent: RestField::available(base_percent),
        flat_bonus: RestField::unavailable(RestUnavailableReason::NotObserved),
        modifiers,
        rounding: RestRoundingMode::Down,
        resolved_percent: RestField::available(resolved_percent),
    }
}

pub fn heal_effect(effect_id: &str, healing: RestHealAmount) -> RestEffect {
    RestEffect {
        effect_id: effect_id.to_owned(),
        label: text(effect_id),
        kind: RestEffectKind::Heal,
        target: RestField::unavailable(RestUnavailableReason::NotApplicable),
        healing: Some(healing),
        references: Vec::new(),
    }
}

pub fn card_effect(effect_id: &str, kind: RestEffectKind) -> RestEffect {
    RestEffect {
        effect_id: effect_id.to_owned(),
        label: text(effect_id),
        kind,
        target: RestField::available(reference(RestReferenceKind::Card, "card.strike")),
        healing: None,
        references: Vec::new(),
    }
}

pub fn coverage(option_id: &str, state: RestCoverageState) -> RestCoverageRecord {
    RestCoverageRecord {
        option_id: option_id.to_owned(),
        state,
        reason: text("no typed record for this option"),
    }
}

pub fn untyped_option(
    option_id: &str,
    kind: RestOptionKind,
    availability: RestOptionAvailability,
) -> RestOptionInput {
    RestOptionInput {
        kind,
        availability,
        definition: option_reference(option_id),
        ..option(option_id, RestOptionKind::Custom("placeholder".to_owned()))
    }
}

/// Base option with no requirements, costs, limits, effects, or selection.
pub fn option(option_id: &str, kind: RestOptionKind) -> RestOptionInput {
    RestOptionInput {
        option_id: option_id.to_owned(),
        label: text(option_id),
        description: text(option_id),
        kind,
        definition: reference(RestReferenceKind::Effect, "rest-effect.heal"),
        availability: available_now(),
        requirements: Vec::new(),
        costs: Vec::new(),
        limits: Vec::new(),
        effects: Vec::new(),
        selection: no_selection(),
        comparison: None,
        action_kind: RestField::unavailable(RestUnavailableReason::NotIntegrated),
        action: RestField::unavailable(RestUnavailableReason::NotIntegrated),
        visibility: RestVisibility::Visible,
        evidence: RestEvidence::SourceDerived,
        references: Vec::new(),
    }
}

/// Base definition with no options, coverage records, or references.
pub fn definition(site_id: &str, generation: u64) -> RestSiteDefinitionInput {
    RestSiteDefinitionInput {
        site_id: site_id.to_owned(),
        label: text(site_id),
        description: text(site_id),
        kind: RestSiteKind::RestSite,
        visibility: RestVisibility::Visible,
        evidence: RestEvidence::SourceDerived,
        option_set_generation: generation,
        observed_options: Vec::new(),
        options: Vec::new(),
        coverage: Vec::new(),
        references: Vec::new(),
    }
}

/// Definition hidden from every scope, which requires explicitly withheld text.
pub fn hidden_definition(site_id: &str, generation: u64) -> RestSiteDefinitionInput {
    RestSiteDefinitionInput {
        label: withheld_text(),
        description: withheld_text(),
        visibility: RestVisibility::Hidden,
        ..definition(site_id, generation)
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<RestSiteDefinitionInput>,
) -> RestSiteCatalogSnapshot {
    RestSiteCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: REST_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: RestFamilyCoverage {
            entity_kind: REST_REFERENCE_ENTITY_KIND.to_owned(),
            state: RestFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

pub fn produce(
    manifest: &ContentManifest,
    snapshot: RestSiteCatalogSnapshot,
) -> Result<RestSiteCatalog, RestSiteError> {
    RestSiteCatalogProducer::new().produce(manifest, &FixtureRestSiteSource::new(snapshot))
}

/// Produces through an explicit source so a test can observe the read count.
pub fn produce_with<S: sts2_game_mod::RestSiteCatalogSource>(
    manifest: &ContentManifest,
    source: &S,
) -> Result<RestSiteCatalog, RestSiteError> {
    RestSiteCatalogProducer::new().produce(manifest, source)
}

#[path = "rest_site_reference_fixture.rs"]
mod fixture;

pub use fixture::*;
