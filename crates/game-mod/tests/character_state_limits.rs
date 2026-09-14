// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/character_state_support.rs"]
mod character_state_support;

use character_state_support::{
    CatalogFixture, catalog, catalog_snapshot, live_binding, manifest, owner, unit,
};
use sts2_game_mod::{
    CharacterMechanicCoverage, CharacterMechanicState, CharacterResourceSlot,
    CharacterResourceSlotContent, CharacterResourceValue, CharacterResourceValueDefinition,
    CharacterStateCatalogProducer, CharacterStateDefinitionKind, CharacterStateField,
    CharacterStateListQuery, CharacterStateLiveReader, CharacterStateLiveSnapshot,
    CharacterStateLiveSnapshotInput, CharacterStateOwnerKind, CharacterStateVisibility,
    CharacterStateVisibilityScope, SecondaryEntityInput, SecondaryEntityStatus,
};

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
fn static_list_fails_closed_on_mixed_unsupported_coverage() {
    let manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source.coverage.push(CharacterMechanicCoverage {
        character_id: "char-b".to_owned(),
        mode_id: "standard".to_owned(),
        resources: CharacterMechanicState::Unsupported,
        secondary_entities: CharacterMechanicState::NotApplicable,
    });
    let catalog = CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("catalog");
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&CharacterStateListQuery {
            kind: CharacterStateDefinitionKind::Resource,
            character_id: None,
            mode_id: None,
            scope: CharacterStateVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(sts2_game_mod::CharacterStateCatalogError::UnsupportedMechanic),
        "an unfiltered query that also touches unsupported coverage must not report a partial success"
    );
    assert_eq!(
        reader.list(&CharacterStateListQuery {
            kind: CharacterStateDefinitionKind::SecondaryEntity,
            character_id: Some("char-b".to_owned()),
            mode_id: Some("standard".to_owned()),
            scope: CharacterStateVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(sts2_game_mod::CharacterStateCatalogError::NotApplicableMechanic),
        "a not-applicable mechanic must be disclosed instead of returning an empty page"
    );
    let supported = reader
        .list(&CharacterStateListQuery {
            kind: CharacterStateDefinitionKind::Resource,
            character_id: Some("char-a".to_owned()),
            mode_id: Some("standard".to_owned()),
            scope: CharacterStateVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("supported slice");
    assert!(supported.complete);
    assert_eq!(supported.total, 1);
}

#[test]
fn detail_limit_counts_definition_binding_and_custom_slot_kinds() {
    let manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source.resource_definitions[0].label = "L".repeat(16_000);
    source.resource_definitions[0].value = CharacterResourceValueDefinition::Text;
    source.resource_definitions[0].maximum = None;
    let catalog = CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("catalog");
    let build = |kind_len: usize| {
        CharacterStateLiveSnapshot::from_input(CharacterStateLiveSnapshotInput {
            binding: live_binding(&catalog, 1),
            resources: vec![sts2_game_mod::CharacterResourceInput {
                instance_id: "resource-live-custom".to_owned(),
                definition_id: "resource:charge".to_owned(),
                owner: owner(CharacterStateOwnerKind::Player, "player-1"),
                current: CharacterStateField::Available(CharacterResourceValue::Text(
                    "v".repeat(16_000),
                )),
                maximum: CharacterStateField::NotObserved,
                slots: CharacterStateField::Available(vec![CharacterResourceSlot {
                    slot_id: "slot-0".to_owned(),
                    position: 0,
                    content: CharacterStateField::Available(CharacterResourceSlotContent::Custom {
                        kind: "k".repeat(kind_len),
                        value: "s".repeat(8_000),
                    }),
                }]),
                active: CharacterStateField::Available(true),
            }],
            secondary_entities: Vec::new(),
        })
        .expect("snapshot shape")
    };
    let actual = |kind_len: usize| {
        let error = CharacterStateLiveReader::new(&catalog, build(kind_len))
            .expect_err("detail exceeds bound");
        match error {
            sts2_game_mod::CharacterStateLiveError::DetailTooLarge { actual, .. } => actual,
            other => {
                assert!(matches!(
                    other,
                    sts2_game_mod::CharacterStateLiveError::DetailTooLarge { .. }
                ));
                0
            }
        }
    };
    assert_eq!(
        actual(128) - actual(8),
        120,
        "the detail limit must count every custom slot kind byte"
    );
}
