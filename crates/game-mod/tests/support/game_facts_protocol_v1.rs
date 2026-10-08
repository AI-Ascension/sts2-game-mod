// SPDX-License-Identifier: MIT

//! Synthetic caller-supplied values for GameFactsReferenceV1Adapter conformance.

use sts2_game_mod::{
    ContentDefinition, ContentFamily, ContentManifest, ContentPackage, FactsEvidenceStatus,
    FactsInputAvailability, FactsInputSource, FactsInventory, FactsRepresentation, FactsRuleEntry,
    FactsRuleInput, FactsSourceKind, FactsUnsupportedCombination, GameFactsError,
};
use sts2_protocol::game_facts_reference_v1::{
    AuthenticatedScope, Binding, BindingMode, EntityKind, GAME_FACTS_REFERENCE_V1_ARTIFACT,
    GAME_FACTS_REFERENCE_V1_PROFILE, GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST, GameFactsProvenance,
    InstanceRef, Message, MessageKind, ParentObservation, Query, SnapshotRef,
};

/// Publicly constructed synthetic manifest; it is not ContentManifestProducer evidence.
pub fn manifest() -> ContentManifest {
    ContentManifest {
        game_build: "build-fixture".to_owned(),
        adapter_compatibility: "adapter-fixture".to_owned(),
        catalog_generation: 7,
        locale: "en".to_owned(),
        packages: vec![ContentPackage {
            package_id: "package-fixture".to_owned(),
            package_version: Some("1".to_owned()),
            order: 0,
        }],
        families: vec![ContentFamily {
            entity_kind: "card".to_owned(),
            handled: true,
            definition_count: 1,
        }],
        definitions: vec![ContentDefinition {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.fixture".to_owned(),
            handled: true,
            origin: sts2_game_mod::ContentOriginInput {
                package_id: Some("package-fixture".to_owned()),
                package_version: Some("1".to_owned()),
            },
            override_chain: Vec::new(),
            semantic_revision: "semantic-fixture".to_owned(),
            localized_text_revision: "text-fixture".to_owned(),
        }],
        content_set_revision: "content-fixture".to_owned(),
        localized_text_revision: "locale-fixture".to_owned(),
        inventory_revision: "inventory-fixture".to_owned(),
    }
}

pub fn representation() -> FactsRepresentation {
    FactsRepresentation {
        version: "facts-v1".to_owned(),
        encoding: "structured-facts".to_owned(),
    }
}

pub fn input(name: &str, unit: &str, reference: Option<&str>) -> FactsRuleInput {
    input_with_kind(name, unit, FactsSourceKind::GameMod, reference)
}

pub fn input_with_kind(
    name: &str,
    unit: &str,
    kind: FactsSourceKind,
    reference: Option<&str>,
) -> FactsRuleInput {
    input_with_availability(
        name,
        unit,
        kind,
        FactsInputAvailability::Required,
        reference,
    )
}

pub fn input_with_availability(
    name: &str,
    unit: &str,
    kind: FactsSourceKind,
    availability: FactsInputAvailability,
    reference: Option<&str>,
) -> FactsRuleInput {
    FactsRuleInput {
        name: name.to_owned(),
        unit: unit.to_owned(),
        availability,
        source: reference.map(|reference| FactsInputSource {
            kind,
            reference: reference.to_owned(),
        }),
    }
}

pub fn rule(id: &str, status: FactsEvidenceStatus, inputs: Vec<FactsRuleInput>) -> FactsRuleEntry {
    FactsRuleEntry {
        rule_id: id.to_owned(),
        evidence: status,
        inputs,
    }
}

pub fn inventory(
    manifest: &ContentManifest,
    rules: Vec<FactsRuleEntry>,
    unsupported: Vec<FactsUnsupportedCombination>,
) -> Result<FactsInventory, GameFactsError> {
    FactsInventory::new_for_manifest(manifest, "standard", representation(), rules, unsupported)
}

pub fn query(rule_ids: &[&str], content_manifest_id: &str, mode: BindingMode) -> Message {
    let (binding, scope, parent_observation) = if mode == BindingMode::Static {
        (
            Binding {
                mode,
                content_manifest_id: content_manifest_id.to_owned(),
                locale: "en".to_owned(),
                visibility_scope: None,
                instance_ref: None,
                snapshot_ref: None,
            },
            None,
            None,
        )
    } else {
        let instance_ref = InstanceRef {
            instance_id: "instance-fixture".to_owned(),
            run_id: "run-fixture".to_owned(),
            epoch: 1,
            entity_kind: EntityKind::Character,
            entity_id: "player-fixture".to_owned(),
        };
        let snapshot_ref = SnapshotRef {
            snapshot_id: "snapshot-fixture".to_owned(),
            instance_ref: instance_ref.clone(),
            state_generation: 1,
        };
        (
            Binding {
                mode,
                content_manifest_id: content_manifest_id.to_owned(),
                locale: "en".to_owned(),
                visibility_scope: Some("owner".to_owned()),
                instance_ref: Some(instance_ref.clone()),
                snapshot_ref: Some(snapshot_ref.clone()),
            },
            Some(AuthenticatedScope {
                instance_id: instance_ref.instance_id.clone(),
                run_id: instance_ref.run_id.clone(),
                authority_epoch: 1,
                content_manifest_id: content_manifest_id.to_owned(),
                locale: "en".to_owned(),
            }),
            Some(ParentObservation {
                instance_ref,
                snapshot_ref,
                state_generation: 1,
            }),
        )
    };
    Message {
        protocol_version: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
        schema_digest: GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST.to_owned(),
        provenance: GameFactsProvenance {
            artifact: GAME_FACTS_REFERENCE_V1_ARTIFACT.to_owned(),
            source: "schemas/game-facts-reference-v1.schema.json".to_owned(),
            generator: "hand-authored".to_owned(),
        },
        correlation_id: "game-facts-test-1".to_owned(),
        kind: MessageKind::QueryRequest,
        query: Some(Query {
            rules_reference_version: 2,
            rule_ids: rule_ids.iter().map(|id| (*id).to_owned()).collect(),
            binding,
            scope,
            parent_observation,
        }),
        result: None,
        capabilities: None,
        error: None,
    }
}
