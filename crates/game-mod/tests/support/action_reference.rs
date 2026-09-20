// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ACTION_REFERENCE_CARD_KIND, ACTION_REFERENCE_EFFECT_KIND, ACTION_REFERENCE_ENEMY_KIND,
    ACTION_REFERENCE_ENTITY_KIND, ACTION_REFERENCE_POTION_KIND, ACTION_REFERENCE_PRODUCER_VERSION,
    ACTION_REFERENCE_RELIC_KIND, ACTION_REFERENCE_RESOURCE_KIND, ACTION_REFERENCE_SELECTION_KIND,
    ACTION_REFERENCE_STATUS_KIND, ActionCatalog, ActionCatalogProducer, ActionCatalogSnapshot,
    ActionCatalogSource, ActionCostContributor, ActionCostKind, ActionCoverageRecord,
    ActionCoverageState, ActionDefinitionInput, ActionEffectKind, ActionEligibility,
    ActionEligibilityState, ActionError, ActionEvidence, ActionFamilyCoverage, ActionFamilyState,
    ActionField, ActionFrameReference, ActionInstanceReference, ActionKind, ActionOmissionKind,
    ActionParentOperation, ActionPreviewAssumption, ActionPreviewChange, ActionPreviewClass,
    ActionPreviewInput, ActionPreviewOmission, ActionPreviewProvenance, ActionPreviewSelection,
    ActionReference, ActionReferenceKind, ActionRefusalReason, ActionRestrictionKind,
    ActionSemanticReference, ActionSnapshotReference, ActionTargetInput, ActionTargetKind,
    ActionTargetRestriction, ActionText, ActionUnavailableReason, ActionVisibility,
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    FixtureActionSource,
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

/// Manifest declaring the action family plus the families the fixture references.
pub fn manifest(actions: &[&str]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = actions
        .iter()
        .map(|action_id| (ACTION_REFERENCE_ENTITY_KIND, *action_id))
        .collect();
    definitions.extend(item_definitions());
    manifest_of(&definitions)
}

/// Manifest that inventories the action family with no legal action at all.
pub fn manifest_without_actions() -> ContentManifest {
    manifest_of_with_kinds(&item_definitions(), &[ACTION_REFERENCE_ENTITY_KIND])
}

/// Manifest with the fixture actions plus extra content definitions, to shift the fence.
pub fn manifest_with_extra(extra: &[(&str, &str)]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = FIXTURE_IDS
        .iter()
        .map(|action_id| (ACTION_REFERENCE_ENTITY_KIND, *action_id))
        .collect();
    definitions.extend(item_definitions());
    definitions.extend(extra.iter().copied());
    manifest_of(&definitions)
}

/// The definition families every fixture reference resolves to.
pub fn item_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        (ACTION_REFERENCE_CARD_KIND, "card.strike"),
        (ACTION_REFERENCE_EFFECT_KIND, "action-effect.damage"),
        (ACTION_REFERENCE_ENEMY_KIND, "enemy.slime"),
        (ACTION_REFERENCE_ENEMY_KIND, "enemy.cultist"),
        (ACTION_REFERENCE_ENEMY_KIND, "enemy.corpse"),
        (ACTION_REFERENCE_POTION_KIND, "potion.fire"),
        (ACTION_REFERENCE_RELIC_KIND, "relic.burning_blood"),
        (ACTION_REFERENCE_RESOURCE_KIND, "resource.energy"),
        (ACTION_REFERENCE_RESOURCE_KIND, "resource.potion_charge"),
        (ACTION_REFERENCE_SELECTION_KIND, "selection.alpha"),
        (ACTION_REFERENCE_STATUS_KIND, "status.strength"),
    ]
}

/// Legal-action generation every shared fixture definition describes.
pub const GENERATION: u64 = 3;
/// Legal-action identities of the shared fixture.
pub const FIXTURE_IDS: [&str; 7] = [
    "action.alpha",
    "action.beta",
    "action.gamma",
    "action.delta",
    "action.epsilon",
    "action.zeta",
    "action.eta",
];

pub fn text(value: &str) -> ActionText {
    ActionText::Available(value.to_owned())
}

/// Localized text the source knows exists but must not reveal.
pub fn withheld_text() -> ActionText {
    ActionText::withheld()
}

/// Availability of a field the source supports but did not observe.
pub fn not_observed() -> ActionUnavailableReason {
    ActionUnavailableReason::NotObserved
}

pub fn not_applicable() -> ActionUnavailableReason {
    ActionUnavailableReason::NotApplicable
}

/// An explicit non-value, used wherever a field does not apply.
pub fn none<T>() -> ActionField<T> {
    ActionField::unavailable(not_applicable())
}

/// Localized text that does not apply, such as the reason text of an offered action.
pub fn no_text() -> ActionText {
    ActionText::Unavailable(not_applicable())
}

pub fn reference(kind: ActionReferenceKind, id: &str) -> ActionSemanticReference {
    ActionSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

/// Availability of an action the host offers now, which must carry no refusal reason.
pub fn eligible() -> ActionEligibility {
    ActionEligibility {
        state: ActionEligibilityState::Available,
        reason: none(),
        reason_text: no_text(),
        references: Vec::new(),
    }
}

/// Availability of an action the host refuses for an explicit reason.
pub fn refused(reason: ActionRefusalReason, reason_text: &str) -> ActionEligibility {
    ActionEligibility {
        state: ActionEligibilityState::Unavailable,
        reason: ActionField::available(reason),
        reason_text: text(reason_text),
        references: Vec::new(),
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<ActionDefinitionInput>,
) -> ActionCatalogSnapshot {
    ActionCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: ACTION_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: ActionFamilyCoverage {
            entity_kind: ACTION_REFERENCE_ENTITY_KIND.to_owned(),
            state: ActionFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

pub fn produce(
    manifest: &ContentManifest,
    snapshot: ActionCatalogSnapshot,
) -> Result<ActionCatalog, ActionError> {
    ActionCatalogProducer::new().produce(manifest, &FixtureActionSource::new(snapshot))
}

/// Produces through an explicit source so a test can observe the read count.
pub fn produce_with<S: ActionCatalogSource>(
    manifest: &ContentManifest,
    source: &S,
) -> Result<ActionCatalog, ActionError> {
    ActionCatalogProducer::new().produce(manifest, source)
}

/// Exact static legal-action frame reference bound to one produced catalog.
pub fn frame(catalog: &ActionCatalog, action_id: &str, generation: u64) -> ActionFrameReference {
    ActionFrameReference {
        action: ActionReference {
            catalog: catalog.binding.clone(),
            action_id: action_id.to_owned(),
        },
        generation,
    }
}

/// Transient live action-instance reference a static slice must never retain.
pub fn instance(action_instance_id: &str, generation: u64) -> ActionInstanceReference {
    ActionInstanceReference {
        snapshot: ActionSnapshotReference {
            run_id: "run.alpha".to_owned(),
            instance_id: "instance:run.alpha".to_owned(),
            epoch: 1,
            snapshot_id: "snapshot.alpha".to_owned(),
        },
        action_instance_id: action_instance_id.to_owned(),
        generation,
    }
}

/// The live fence one live preview query binds to.
pub fn fence() -> ActionSnapshotReference {
    instance("action-instance.alpha", GENERATION).snapshot
}

/// Provenance of a preview built from versioned rules.
pub fn rules() -> ActionPreviewProvenance {
    ActionPreviewProvenance::VersionedRules {
        rules_id: "rules.v1".to_owned(),
    }
}

#[path = "action_reference_fixture.rs"]
mod fixture;

pub use fixture::*;
