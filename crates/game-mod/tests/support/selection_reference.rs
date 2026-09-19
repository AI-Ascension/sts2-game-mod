// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    FixtureSelectionSource, SELECTION_REFERENCE_CARD_KIND, SELECTION_REFERENCE_EFFECT_KIND,
    SELECTION_REFERENCE_ENTITY_KIND, SELECTION_REFERENCE_PLAYER_KIND,
    SELECTION_REFERENCE_POTION_KIND, SELECTION_REFERENCE_PRODUCER_VERSION,
    SELECTION_REFERENCE_RELIC_KIND, SelectionActionReference, SelectionBlockReason,
    SelectionCancellation, SelectionCandidateInput, SelectionCandidateKind, SelectionCatalog,
    SelectionCatalogProducer, SelectionCatalogSnapshot, SelectionConfirmation,
    SelectionCoverageRecord, SelectionCoverageState, SelectionDefinitionInput,
    SelectionDuplicateRule, SelectionEligibility, SelectionEligibilityState, SelectionError,
    SelectionEvidence, SelectionFamilyCoverage, SelectionFamilyState, SelectionField,
    SelectionKind, SelectionNextDomain, SelectionOrderingRule, SelectionParentOperation,
    SelectionPickRule, SelectionProgressInput, SelectionProspectiveEffect,
    SelectionProspectiveEffectKind, SelectionReference, SelectionReferenceKind,
    SelectionSemanticReference, SelectionSnapshotReference, SelectionText,
    SelectionUnavailableReason, SelectionVisibility, SelectorGenerationReference,
    SelectorInstanceReference,
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

/// Manifest declaring the selection family plus the families the fixture references.
pub fn manifest(selections: &[&str]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = selections
        .iter()
        .map(|selection_id| (SELECTION_REFERENCE_ENTITY_KIND, *selection_id))
        .collect();
    definitions.extend(item_definitions());
    manifest_of(&definitions)
}

/// Manifest that inventories the selection family with no selection definition at all.
pub fn manifest_without_selections() -> ContentManifest {
    manifest_of_with_kinds(&item_definitions(), &[SELECTION_REFERENCE_ENTITY_KIND])
}

/// The definition families every fixture reference resolves to.
pub fn item_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        (SELECTION_REFERENCE_CARD_KIND, "card.strike"),
        (SELECTION_REFERENCE_CARD_KIND, "card.defend"),
        (SELECTION_REFERENCE_CARD_KIND, "card.removal"),
        (SELECTION_REFERENCE_EFFECT_KIND, "selection-effect.upgrade"),
        (SELECTION_REFERENCE_EFFECT_KIND, "selection-effect.reward"),
        (SELECTION_REFERENCE_RELIC_KIND, "relic.burning_blood"),
        (SELECTION_REFERENCE_POTION_KIND, "potion.fire"),
        (SELECTION_REFERENCE_PLAYER_KIND, COOP_PLAYER_ID),
    ]
}

/// Fixture manifest plus extra content definitions, used to shift the manifest fence.
pub fn manifest_with_extra(extra: &[(&str, &str)]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = FIXTURE_IDS
        .iter()
        .map(|selection_id| (SELECTION_REFERENCE_ENTITY_KIND, *selection_id))
        .collect();
    definitions.extend(item_definitions());
    definitions.extend(extra.iter().copied());
    manifest_of(&definitions)
}

/// Co-op player identity the shared player selection resolves to.
pub const COOP_PLAYER_ID: &str = "player.coop.one";
/// Selector generations the shared fixture declares.
pub const ALPHA_GENERATION: u64 = 3;
/// Selector generation of the second shared selector.
pub const BETA_GENERATION: u64 = 5;
/// Selector generation of the multi-step shared selector.
pub const EPSILON_GENERATION: u64 = 11;
/// Selection identities of the shared fixture.
pub const FIXTURE_IDS: [&str; 5] = [
    "selection.alpha",
    "selection.beta",
    "selection.gamma",
    "selection.delta",
    "selection.epsilon",
];

/// Transient action reference a static selection slice must never retain.
pub fn action_reference() -> SelectionActionReference {
    SelectionActionReference {
        selector: SelectorInstanceReference {
            snapshot: SelectionSnapshotReference {
                run_id: "run.alpha".to_owned(),
                instance_id: "instance:run.alpha".to_owned(),
                epoch: 1,
                snapshot_id: "snapshot.alpha".to_owned(),
            },
            selector_instance_id: "selector-instance.alpha".to_owned(),
        },
        action_kind: "selection.confirm".to_owned(),
        action_id: "action.selection.confirm".to_owned(),
    }
}

pub fn text(value: &str) -> SelectionText {
    SelectionText::Available(value.to_owned())
}

/// Localized text the source knows exists but must not reveal.
pub fn withheld_text() -> SelectionText {
    SelectionText::withheld()
}

pub fn reference(kind: SelectionReferenceKind, id: &str) -> SelectionSemanticReference {
    SelectionSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

/// Availability of a field the source supports but did not observe.
pub fn not_observed() -> SelectionUnavailableReason {
    SelectionUnavailableReason::NotObserved
}

pub fn picks(required: bool, minimum: Option<u32>, maximum: Option<u32>) -> SelectionPickRule {
    let field = |value: Option<u32>| match value {
        Some(value) => SelectionField::available(value),
        None => SelectionField::unavailable(SelectionUnavailableReason::NotObserved),
    };
    SelectionPickRule {
        required,
        minimum: field(minimum),
        maximum: field(maximum),
    }
}

/// Eligibility of a candidate the host offers now, which must carry no refusal reason.
pub fn eligible() -> SelectionEligibility {
    SelectionEligibility {
        state: SelectionEligibilityState::Eligible,
        reason: SelectionField::unavailable(SelectionUnavailableReason::NotObserved),
        references: Vec::new(),
    }
}

/// Eligibility of a candidate the host refuses for an explicit reason.
pub fn refused(reason: SelectionBlockReason) -> SelectionEligibility {
    SelectionEligibility {
        state: SelectionEligibilityState::Ineligible,
        reason: SelectionField::available(reason),
        references: Vec::new(),
    }
}

/// One documented prospective effect with both sides stated.
pub fn effect(
    effect_id: &str,
    kind: SelectionProspectiveEffectKind,
    before: &str,
    after: &str,
) -> SelectionProspectiveEffect {
    SelectionProspectiveEffect {
        effect_id: effect_id.to_owned(),
        label: text(effect_id),
        kind,
        target: SelectionField::unavailable(SelectionUnavailableReason::NotApplicable),
        before: SelectionField::available(before.to_owned()),
        after: SelectionField::available(after.to_owned()),
        references: Vec::new(),
    }
}

pub fn candidate(
    candidate_id: &str,
    kind: SelectionCandidateKind,
    definition: SelectionSemanticReference,
) -> SelectionCandidateInput {
    SelectionCandidateInput {
        candidate_id: candidate_id.to_owned(),
        label: text(candidate_id),
        kind,
        definition,
        eligibility: eligible(),
        detail: SelectionField::available(candidate_id.to_owned()),
        prospective: Vec::new(),
        visibility: SelectionVisibility::Visible,
        evidence: SelectionEvidence::SourceDerived,
        references: Vec::new(),
    }
}

pub fn coverage(target_id: &str, state: SelectionCoverageState) -> SelectionCoverageRecord {
    SelectionCoverageRecord {
        target_id: target_id.to_owned(),
        state,
        reason: text("no typed record for this selection target"),
    }
}

/// Base definition with no candidates, coverage, or references.
pub fn definition(selection_id: &str, generation: u64) -> SelectionDefinitionInput {
    SelectionDefinitionInput {
        selection_id: selection_id.to_owned(),
        parent: SelectionParentOperation::CombatCardPlay,
        kind: SelectionKind::Card,
        prompt: text(selection_id),
        visibility: SelectionVisibility::Visible,
        evidence: SelectionEvidence::SourceDerived,
        selector_generation: generation,
        picks: picks(true, Some(1), Some(1)),
        ordering: SelectionOrderingRule::Stable,
        duplicate: SelectionDuplicateRule::Distinct,
        confirmation: SelectionConfirmation::AutomaticClose,
        cancellation: SelectionField::unavailable(SelectionUnavailableReason::NotApplicable),
        steps: SelectionField::unavailable(SelectionUnavailableReason::NotApplicable),
        next: SelectionField::unavailable(SelectionUnavailableReason::NotApplicable),
        observed_candidates: Vec::new(),
        candidates: Vec::new(),
        coverage: Vec::new(),
        action_kind: SelectionField::unavailable(SelectionUnavailableReason::NotIntegrated),
        action: SelectionField::unavailable(SelectionUnavailableReason::NotIntegrated),
        references: Vec::new(),
    }
}

/// Definition hidden from every scope, which requires explicitly withheld text.
pub fn hidden_definition(selection_id: &str, generation: u64) -> SelectionDefinitionInput {
    SelectionDefinitionInput {
        prompt: withheld_text(),
        visibility: SelectionVisibility::Hidden,
        ..definition(selection_id, generation)
    }
}

/// Exact selector generation reference bound to one produced catalog.
pub fn selector(
    catalog: &SelectionCatalog,
    selection_id: &str,
    generation: u64,
) -> SelectorGenerationReference {
    SelectorGenerationReference {
        selection: SelectionReference {
            catalog: catalog.binding.clone(),
            selection_id: selection_id.to_owned(),
        },
        selector_generation: generation,
    }
}

/// Picks a caller reports against one selector generation of one catalog.
pub fn progress(
    catalog: &SelectionCatalog,
    selection_id: &str,
    generation: u64,
    selected: &[&str],
) -> SelectionProgressInput {
    SelectionProgressInput {
        selector: selector(catalog, selection_id, generation),
        selected: selected.iter().map(|id| (*id).to_owned()).collect(),
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<SelectionDefinitionInput>,
) -> SelectionCatalogSnapshot {
    SelectionCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: SELECTION_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: SelectionFamilyCoverage {
            entity_kind: SELECTION_REFERENCE_ENTITY_KIND.to_owned(),
            state: SelectionFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

pub fn produce(
    manifest: &ContentManifest,
    snapshot: SelectionCatalogSnapshot,
) -> Result<SelectionCatalog, SelectionError> {
    SelectionCatalogProducer::new().produce(manifest, &FixtureSelectionSource::new(snapshot))
}

/// Produces through an explicit source so a test can observe the read count.
pub fn produce_with<S: sts2_game_mod::SelectionCatalogSource>(
    manifest: &ContentManifest,
    source: &S,
) -> Result<SelectionCatalog, SelectionError> {
    SelectionCatalogProducer::new().produce(manifest, source)
}

#[path = "selection_reference_fixture.rs"]
mod fixture;

pub use fixture::*;
