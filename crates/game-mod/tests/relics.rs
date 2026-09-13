// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/relics.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, RELIC_PRODUCER_VERSION, RelicAccumulatedValue, RelicActivationState,
    RelicCatalogError, RelicCatalogProducer, RelicCatalogSnapshot, RelicCounterReset,
    RelicCounterState, RelicFamilyCoverage, RelicFamilyState, RelicField, RelicFieldStatus,
    RelicListQuery, RelicLiveError, RelicLiveReader, RelicLiveSnapshot, RelicLiveSnapshotInput,
    RelicParameterValue, RelicPendingTrigger, RelicResolvedParameter, RelicSemanticReferenceKind,
    RelicVisibilityScope,
};

#[test]
fn definitions_keep_static_rules_and_reference_fences() {
    let catalog = catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&RelicListQuery {
            scope: RelicVisibilityScope::Public,
            limit: 3,
            continuation: None,
        })
        .expect("page");
    assert_eq!(first.total, 5);
    assert!(!first.complete);
    assert_eq!(first.entries[0].reference.relic_id, "mod:synthetic:badge");
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&RelicListQuery {
            scope: RelicVisibilityScope::Public,
            limit: 3,
            continuation: Some(continuation.clone()),
        })
        .expect("second page");
    assert!(second.complete);
    assert_eq!(
        reader.list(&RelicListQuery {
            scope: RelicVisibilityScope::Public,
            limit: 3,
            continuation: Some(continuation),
        }),
        Err(RelicCatalogError::InvalidContinuation)
    );
    let locked = catalog
        .reader()
        .list(&RelicListQuery {
            scope: RelicVisibilityScope::Reference,
            limit: 8,
            continuation: None,
        })
        .expect("reference page");
    assert_eq!(locked.total, 6);
    let locked_reference = locked
        .entries
        .iter()
        .find(|entry| entry.unlock_state == ContentUnlockState::Locked)
        .expect("locked reference")
        .reference
        .clone();
    let detail = catalog
        .get(&locked_reference, RelicVisibilityScope::Reference)
        .expect("detail");
    assert_eq!(detail.acquisition.rules[0].kind, "reward");
    assert_eq!(detail.variants[0].id, "default");
    assert!(detail.description.contains("Static rule"));
    assert_eq!(
        detail.references[0].kind,
        RelicSemanticReferenceKind::Effect
    );
    let mut stale = detail.reference;
    stale.catalog.manifest.catalog_generation += 1;
    assert_eq!(
        catalog.get(&stale, RelicVisibilityScope::Reference),
        Err(RelicCatalogError::StaleReference)
    );
}

#[test]
fn live_instances_cover_counter_families_and_resolved_parameters() {
    let catalog = catalog();
    let mut live_input = RelicLiveSnapshotInput {
        binding: binding(&catalog, 7),
        instances: vec![
            instance(
                "instance:passive",
                "mod:synthetic:badge",
                RelicField::NotApplicable,
                RelicField::NotApplicable,
            ),
            instance(
                "instance:charged",
                "mod:synthetic:charged",
                RelicField::Available(vec![RelicCounterState {
                    id: "charge".to_owned(),
                    value: RelicField::Available(2),
                }]),
                RelicField::Available(RelicActivationState {
                    active: RelicField::Available(true),
                    used: RelicField::Available(false),
                }),
            ),
            instance(
                "instance:turn",
                "mod:synthetic:turn",
                RelicField::Available(vec![RelicCounterState {
                    id: "turns".to_owned(),
                    value: RelicField::Available(3),
                }]),
                RelicField::Available(RelicActivationState {
                    active: RelicField::Available(false),
                    used: RelicField::Available(true),
                }),
            ),
            instance(
                "instance:room",
                "mod:synthetic:room",
                RelicField::Available(vec![RelicCounterState {
                    id: "rooms".to_owned(),
                    value: RelicField::Available(1),
                }]),
                RelicField::Unavailable,
            ),
            instance(
                "instance:multiple",
                "mod:synthetic:multiple",
                RelicField::Available(vec![
                    RelicCounterState {
                        id: "first".to_owned(),
                        value: RelicField::Available(4),
                    },
                    RelicCounterState {
                        id: "second".to_owned(),
                        value: RelicField::Available(0),
                    },
                ]),
                RelicField::Available(RelicActivationState {
                    active: RelicField::Available(true),
                    used: RelicField::Available(false),
                }),
            ),
            instance(
                "instance:conditional",
                "mod:synthetic:conditional",
                RelicField::Available(vec![RelicCounterState {
                    id: "threshold".to_owned(),
                    value: RelicField::Unavailable,
                }]),
                RelicField::Available(RelicActivationState {
                    active: RelicField::Available(false),
                    used: RelicField::Available(false),
                }),
            ),
        ],
    };
    let charged = live_input
        .instances
        .iter_mut()
        .find(|instance| instance.instance_id == "instance:charged")
        .expect("charged instance");
    charged.resolved_parameters = RelicField::Available(vec![RelicResolvedParameter {
        id: "amount".to_owned(),
        unit: unit("count"),
        value: RelicParameterValue::Integer(3),
    }]);
    let multiple = live_input
        .instances
        .iter_mut()
        .find(|instance| instance.instance_id == "instance:multiple")
        .expect("multiple instance");
    multiple.accumulated = RelicField::Available(vec![RelicAccumulatedValue {
        id: "earned".to_owned(),
        label: "Earned".to_owned(),
        unit: unit("count"),
        reset: RelicCounterReset::Run,
        value: RelicField::Available(8),
    }]);
    multiple.pending_triggers = RelicField::Available(vec![RelicPendingTrigger {
        id: "trigger:fixture".to_owned(),
        label: "Fixture trigger".to_owned(),
        condition: None,
        parameters: Vec::new(),
    }]);
    let live = RelicLiveSnapshot::from_input(live_input).expect("live snapshot");
    let reader = RelicLiveReader::new(&catalog, live).expect("reader");
    assert_eq!(reader.references().len(), 6);
    let passive_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:passive")
        .expect("passive reference");
    let passive = reader.get(&passive_reference).expect("passive");
    assert_eq!(passive.counters.status(), RelicFieldStatus::NotApplicable);
    assert!(passive.counters.value().is_none());
    let room_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:room")
        .expect("room reference");
    let room = reader.get(&room_reference).expect("room");
    assert_eq!(room.activation.status(), RelicFieldStatus::Unavailable);
    let multiple_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:multiple")
        .expect("multiple reference");
    let multiple = reader.get(&multiple_reference).expect("multiple");
    assert_eq!(
        multiple.counters.value().expect("counters")[1]
            .value
            .value(),
        Some(&0)
    );
    assert_eq!(
        multiple.accumulated.value().expect("accumulated")[0]
            .value
            .value(),
        Some(&8)
    );
    assert_eq!(
        multiple.pending_triggers.value().expect("triggers")[0].id,
        "trigger:fixture"
    );
    let charged_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:charged")
        .expect("charged reference");
    let charged = reader.get(&charged_reference).expect("charged");
    assert_eq!(
        charged.resolved_parameters.value().expect("parameter")[0].value,
        RelicParameterValue::Integer(3)
    );
    assert_eq!(
        reader.get(&charged_reference).expect("repeat read"),
        charged
    );
}

#[test]
fn live_fences_reject_stale_identity_and_invalid_counter_shapes() {
    let catalog = catalog();
    let snapshot = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 1),
        instances: vec![instance(
            "instance:charged",
            "mod:synthetic:charged",
            RelicField::Available(vec![RelicCounterState {
                id: "charge".to_owned(),
                value: RelicField::Available(0),
            }]),
            RelicField::Available(RelicActivationState {
                active: RelicField::Available(false),
                used: RelicField::Available(false),
            }),
        )],
    })
    .expect("snapshot");
    let mut reader = RelicLiveReader::new(&catalog, snapshot).expect("reader");
    let reference = reader.references().pop().expect("reference");
    let next = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 2),
        instances: vec![instance(
            "instance:charged",
            "mod:synthetic:charged",
            RelicField::Unavailable,
            RelicField::Unavailable,
        )],
    })
    .expect("next");
    reader.replace_snapshot(next).expect("replace");
    assert_eq!(reader.get(&reference), Err(RelicLiveError::StaleReference));
    let invalid = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 3),
        instances: vec![instance(
            "instance:passive",
            "mod:synthetic:badge",
            RelicField::Available(Vec::new()),
            RelicField::NotApplicable,
        )],
    })
    .expect("snapshot shape");
    assert!(matches!(
        RelicLiveReader::new(&catalog, invalid),
        Err(RelicLiveError::UnknownCounter {
            relic_id,
            counter_id
        }) if relic_id == "mod:synthetic:badge" && counter_id == "none"
    ));
}

#[test]
fn catalog_fences_and_unhandled_family_fail_closed() {
    let manifest = manifest();
    let mut source = CatalogSource {
        snapshot: Ok(RelicCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: RELIC_PRODUCER_VERSION.to_owned(),
            family: RelicFamilyCoverage {
                entity_kind: "relic".to_owned(),
                state: RelicFamilyState::Unsupported,
                definition_count: 6,
            },
            definitions: Vec::new(),
        }),
    };
    let unsupported = RelicCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("explicit unsupported catalog");
    assert_eq!(unsupported.family().state, RelicFamilyState::Unsupported);
    assert_eq!(
        unsupported.reader().list(&RelicListQuery {
            scope: RelicVisibilityScope::Reference,
            limit: 8,
            continuation: None,
        }),
        Err(RelicCatalogError::UnsupportedFamily)
    );
    let mut wrong_version = source.snapshot.clone().expect("snapshot");
    wrong_version.producer_version = "game-relics-producer-v0".to_owned();
    source.snapshot = Ok(wrong_version);
    assert_eq!(
        RelicCatalogProducer::new().produce(&manifest, &source),
        Err(RelicCatalogError::ProducerVersionMismatch)
    );
}

#[test]
fn source_fences_reject_mismatch_before_publishing_data() {
    let manifest = manifest();
    let mut snapshot = CatalogSource {
        snapshot: Ok(sts2_game_mod::RelicCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: RELIC_PRODUCER_VERSION.to_owned(),
            family: RelicFamilyCoverage {
                entity_kind: "relic".to_owned(),
                state: RelicFamilyState::Handled,
                definition_count: 6,
            },
            definitions: Vec::new(),
        }),
    };
    let mut wrong = snapshot.snapshot.clone().expect("snapshot");
    wrong.manifest.catalog_generation += 1;
    snapshot.snapshot = Ok(wrong);
    assert_eq!(
        RelicCatalogProducer::new().produce(&manifest, &snapshot),
        Err(RelicCatalogError::ManifestMismatch)
    );
}
