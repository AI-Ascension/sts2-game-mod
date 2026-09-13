// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/character_state_support.rs"]
mod character_state_support;

use character_state_support::{
    CatalogFixture, catalog, catalog_snapshot, coverage, live_binding, manifest, owner, snapshot,
    unit,
};
use sts2_game_mod::{
    CharacterMechanicCoverage, CharacterMechanicState, CharacterResourceDefinitionInput,
    CharacterResourceKind, CharacterResourceSlot, CharacterResourceSlotContent,
    CharacterResourceValue, CharacterResourceValueDefinition, CharacterStateCatalogProducer,
    CharacterStateDefinitionKind, CharacterStateField, CharacterStateListQuery,
    CharacterStateLiveBinding, CharacterStateLiveReader, CharacterStateLiveSnapshot,
    CharacterStateLiveSnapshotInput, CharacterStateLiveSource, CharacterStateOwnerKind,
    CharacterStateSourceError, CharacterStateVisibility, CharacterStateVisibilityScope,
    SecondaryEntityInput, SecondaryEntityStatus,
};

#[test]
fn projects_typed_resource_slots_and_secondary_entity_links() {
    let catalog = catalog();
    let reader =
        CharacterStateLiveReader::new(&catalog, snapshot(&catalog, 1)).expect("live reader");
    let resources = reader
        .resource_references_for("char-a")
        .expect("resource refs");
    assert_eq!(resources.len(), 2);
    let resource = reader.get_resource(&resources[0]).expect("resource detail");
    assert_eq!(resource.definition.rule_reference, "rule:charge");
    assert_eq!(
        resource.slots.value().expect("slots")[1].content,
        CharacterStateField::Available(CharacterResourceSlotContent::Empty)
    );
    assert_eq!(
        reader
            .get_resource(&resources[1])
            .expect("ally resource")
            .owner
            .id
            .as_str(),
        "ally-1"
    );
    let entities = reader
        .secondary_entity_references_for("char-a")
        .expect("entity refs");
    let entity = reader
        .get_secondary_entity(&entities[0])
        .expect("entity detail");
    let second_entity = reader
        .get_secondary_entity(&entities[1])
        .expect("second entity");
    assert_eq!(second_entity.owner.id.as_str(), "orb-owner-2");
    assert_eq!(entity.hp, CharacterStateField::Available(12));
    assert_eq!(
        entity
            .controller
            .value()
            .expect("controller")
            .owner_id
            .as_str(),
        "player-1"
    );
    assert_eq!(
        entity.intent.value().expect("intent").amount,
        CharacterStateField::Available(5)
    );
}

#[test]
fn stale_snapshot_epoch_and_wrong_identity_fail_closed() {
    let catalog = catalog();
    let mut reader =
        CharacterStateLiveReader::new(&catalog, snapshot(&catalog, 1)).expect("reader");
    let old = reader
        .resource_references_for("char-a")
        .expect("refs")
        .remove(0);
    assert_eq!(
        reader.replace_snapshot(snapshot(&catalog, 1)),
        Err(sts2_game_mod::CharacterStateLiveError::NonMonotonicEpoch {
            current: 1,
            supplied: 1,
        })
    );
    reader
        .replace_snapshot(snapshot(&catalog, 2))
        .expect("advanced epoch");
    assert_eq!(
        reader.get_resource(&old),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
    let mut wrong = live_binding(&catalog, 3);
    wrong.run_id = "other-run".to_owned();
    let input = CharacterStateLiveSnapshotInput {
        binding: wrong,
        resources: Vec::new(),
        secondary_entities: Vec::new(),
    };
    let replacement = CharacterStateLiveSnapshot::from_input(input).expect("wrong snapshot");
    assert_eq!(
        reader.replace_snapshot(replacement),
        Err(sts2_game_mod::CharacterStateLiveError::RunMismatch)
    );
}

#[test]
fn explicit_unsupported_and_not_applicable_coverage_is_not_an_empty_success() {
    let mut manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source.coverage = vec![
        coverage("char-a"),
        CharacterMechanicCoverage {
            character_id: "char-b".to_owned(),
            mode_id: "standard".to_owned(),
            resources: CharacterMechanicState::NotApplicable,
            secondary_entities: CharacterMechanicState::Unsupported,
        },
    ];
    let catalog = CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("catalog");
    let reader = CharacterStateLiveReader::new(
        &catalog,
        CharacterStateLiveSnapshot::from_input(CharacterStateLiveSnapshotInput {
            binding: live_binding(&catalog, 1),
            resources: Vec::new(),
            secondary_entities: Vec::new(),
        })
        .expect("empty coherent snapshot"),
    )
    .expect("reader");
    assert_eq!(
        reader.resource_references_for("char-b"),
        Err(sts2_game_mod::CharacterStateLiveError::ResourcesNotApplicable)
    );
    assert_eq!(
        reader.secondary_entity_references_for("char-b"),
        Err(sts2_game_mod::CharacterStateLiveError::UnsupportedSecondaryEntities)
    );
    manifest.locale = "fr".to_owned();
    let mut stale = catalog_snapshot(&manifest);
    stale.manifest.catalog_generation = 6;
    stale.coverage = vec![coverage("char-a")];
    assert_eq!(
        CharacterStateCatalogProducer::new().produce(&manifest, &CatalogFixture(stale)),
        Err(sts2_game_mod::CharacterStateCatalogError::ManifestMismatch)
    );
}

#[test]
fn malformed_slots_visibility_and_oversized_detail_are_rejected() {
    let catalog = catalog();
    let mut source = CharacterStateLiveSnapshotInput {
        binding: live_binding(&catalog, 1),
        resources: vec![sts2_game_mod::CharacterResourceInput {
            instance_id: "resource-live-1".to_owned(),
            definition_id: "resource:charge".to_owned(),
            owner: owner(CharacterStateOwnerKind::Player, "player-1"),
            current: CharacterStateField::Available(CharacterResourceValue::Integer {
                value: 2,
                unit: unit("wrong-unit"),
            }),
            maximum: CharacterStateField::NotObserved,
            slots: CharacterStateField::Available(vec![
                CharacterResourceSlot {
                    slot_id: "slot-1".to_owned(),
                    position: 0,
                    content: CharacterStateField::Available(CharacterResourceSlotContent::Empty),
                },
                CharacterResourceSlot {
                    slot_id: "slot-0".to_owned(),
                    position: 1,
                    content: CharacterStateField::Available(CharacterResourceSlotContent::Empty),
                },
            ]),
            active: CharacterStateField::Available(true),
        }],
        secondary_entities: Vec::new(),
    };
    assert!(matches!(
        CharacterStateLiveReader::new(
            &catalog,
            CharacterStateLiveSnapshot::from_input(source.clone()).expect("shape"),
        ),
        Err(sts2_game_mod::CharacterStateLiveError::ValueShapeMismatch(
            "resource_value"
        ))
    ));
    source.resources[0].current = CharacterStateField::Available(CharacterResourceValue::Integer {
        value: 2,
        unit: unit("charge"),
    });
    source.resources[0].slots = CharacterStateField::Available(vec![
        CharacterResourceSlot {
            slot_id: "slot-1".to_owned(),
            position: 1,
            content: CharacterStateField::Available(CharacterResourceSlotContent::Empty),
        },
        CharacterResourceSlot {
            slot_id: "slot-0".to_owned(),
            position: 0,
            content: CharacterStateField::Available(CharacterResourceSlotContent::Empty),
        },
    ]);
    assert_eq!(
        CharacterStateLiveSnapshot::from_input(source),
        Err(sts2_game_mod::CharacterStateLiveError::InvalidSlotOrder)
    );
    let statuses = (0..64)
        .map(|index| SecondaryEntityStatus {
            instance_id: format!("status-{index}"),
            definition_id: "status:charged".to_owned(),
            amount: CharacterStateField::Available(1),
            visibility: CharacterStateVisibility::Visible,
            label: CharacterStateField::Available("x".repeat(600)),
        })
        .collect::<Vec<_>>();
    let reader = CharacterStateLiveReader::new(
        &catalog,
        CharacterStateLiveSnapshot::from_input(CharacterStateLiveSnapshotInput {
            binding: live_binding(&catalog, 3),
            resources: Vec::new(),
            secondary_entities: vec![SecondaryEntityInput {
                instance_id: "entity-live-oversized".to_owned(),
                definition_id: "entity:orb".to_owned(),
                owner: owner(CharacterStateOwnerKind::Secondary, "orb-owner"),
                controller: CharacterStateField::NotObserved,
                hp: CharacterStateField::Available(1),
                maximum_hp: CharacterStateField::Available(1),
                block: CharacterStateField::Available(0),
                statuses: CharacterStateField::Available(statuses),
                intent: CharacterStateField::NotApplicable,
                active: CharacterStateField::Available(true),
            }],
        })
        .expect("snapshot"),
    );
    assert!(matches!(
        reader,
        Err(sts2_game_mod::CharacterStateLiveError::DetailTooLarge { .. })
    ));
}

#[test]
fn static_reader_binds_pages_and_visibility_to_the_catalog() {
    let manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source
        .resource_definitions
        .push(CharacterResourceDefinitionInput {
            definition_id: "resource:reserve".to_owned(),
            character_id: "char-a".to_owned(),
            mode_id: "standard".to_owned(),
            kind: CharacterResourceKind::Counter,
            label: "Reserve".to_owned(),
            rule_reference: "rule:reserve".to_owned(),
            value: CharacterResourceValueDefinition::Integer {
                unit: unit("reserve"),
            },
            maximum: Some(9),
            visibility: CharacterStateVisibility::Visible,
            slots_visibility: CharacterStateVisibility::Visible,
        });
    let catalog = CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("catalog");
    let mut reader = catalog.reader();
    let query = CharacterStateListQuery {
        kind: CharacterStateDefinitionKind::Resource,
        character_id: Some("char-a".to_owned()),
        mode_id: Some("standard".to_owned()),
        scope: CharacterStateVisibilityScope::Public,
        limit: 1,
        continuation: None,
    };
    let first = reader.list(&query).expect("first page");
    assert_eq!(first.binding, *catalog.binding());
    assert_eq!(first.total, 2);
    assert!(!first.complete);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&CharacterStateListQuery {
            continuation: Some(continuation.clone()),
            ..query.clone()
        })
        .expect("second page");
    assert!(second.complete);
    assert_eq!(second.entries.len(), 1);
    assert_eq!(
        reader.list(&CharacterStateListQuery {
            continuation: Some(continuation),
            ..query
        }),
        Err(sts2_game_mod::CharacterStateCatalogError::InvalidContinuation)
    );
}

#[test]
fn owner_visibility_requires_an_explicit_owner_scope() {
    let manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source.resource_definitions[0].visibility = CharacterStateVisibility::OwnerOnly;
    source.resource_definitions[0].slots_visibility = CharacterStateVisibility::OwnerOnly;
    let catalog = CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("catalog");
    let mut reader = catalog.reader();
    let public = reader
        .list(&CharacterStateListQuery {
            kind: CharacterStateDefinitionKind::Resource,
            character_id: Some("char-a".to_owned()),
            mode_id: Some("standard".to_owned()),
            scope: CharacterStateVisibilityScope::Public,
            limit: 4,
            continuation: None,
        })
        .expect("public page");
    assert!(public.entries.is_empty());
    let owner_page = reader
        .list(&CharacterStateListQuery {
            kind: CharacterStateDefinitionKind::Resource,
            character_id: Some("char-a".to_owned()),
            mode_id: Some("standard".to_owned()),
            scope: CharacterStateVisibilityScope::Owner,
            limit: 4,
            continuation: None,
        })
        .expect("owner page");
    assert_eq!(owner_page.entries.len(), 1);

    assert!(matches!(
        CharacterStateLiveReader::new(&catalog, snapshot(&catalog, 1)),
        Err(sts2_game_mod::CharacterStateLiveError::VisibilityDenied(
            "resource"
        ))
    ));
    CharacterStateLiveReader::new_with_scope(
        &catalog,
        snapshot(&catalog, 2),
        CharacterStateVisibilityScope::Owner,
    )
    .expect("owner live scope");
}

struct NoLiveSource;

impl CharacterStateLiveSource for NoLiveSource {
    fn read_live(
        &self,
        _expected: &CharacterStateLiveBinding,
    ) -> Result<CharacterStateLiveSnapshotInput, CharacterStateSourceError> {
        Err(CharacterStateSourceError::NoActiveSource)
    }
}

struct MismatchedLiveSource {
    input: CharacterStateLiveSnapshotInput,
}

impl CharacterStateLiveSource for MismatchedLiveSource {
    fn read_live(
        &self,
        _expected: &CharacterStateLiveBinding,
    ) -> Result<CharacterStateLiveSnapshotInput, CharacterStateSourceError> {
        Ok(self.input.clone())
    }
}

#[test]
fn source_errors_and_binding_mismatches_fail_closed() {
    let catalog = catalog();
    let expected = live_binding(&catalog, 1);
    assert!(matches!(
        CharacterStateLiveReader::from_source(&catalog, &expected, &NoLiveSource),
        Err(sts2_game_mod::CharacterStateLiveError::NoActiveSource)
    ));
    let mut mismatched = expected.clone();
    mismatched.epoch = 2;
    mismatched.snapshot_id = "snapshot-2".to_owned();
    assert!(matches!(
        CharacterStateLiveReader::from_source(
            &catalog,
            &expected,
            &MismatchedLiveSource {
                input: CharacterStateLiveSnapshotInput {
                    binding: mismatched,
                    resources: Vec::new(),
                    secondary_entities: Vec::new(),
                },
            },
        ),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    ));
}
