// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/relics.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, RELIC_MAX_DEFINITION_BYTES, RELIC_MAX_LIVE_DETAIL_BYTES,
    RELIC_MAX_TEXT_BYTES, RELIC_PRODUCER_VERSION, RelicAccumulatedValue, RelicActivationState,
    RelicCatalogError, RelicCatalogProducer, RelicCatalogSnapshot, RelicCondition,
    RelicCounterReset, RelicCounterState, RelicFamilyCoverage, RelicFamilyState, RelicField,
    RelicLiveError, RelicLiveReader, RelicLiveSnapshot, RelicLiveSnapshotInput,
    RelicParameterValue, RelicPendingTrigger, RelicResolvedParameter, RelicTriggerDefinition,
    RelicVisibility, RelicVisibilityScope,
};

#[test]
fn oversized_static_and_live_payloads_fail_closed() {
    let manifest = manifest();
    let mut definitions = vec![
        input(
            "mod:synthetic:badge",
            ContentUnlockState::Unlocked,
            sts2_game_mod::RelicActivationKind::Passive,
            Vec::new(),
        ),
        input(
            "mod:synthetic:charged",
            ContentUnlockState::Unlocked,
            sts2_game_mod::RelicActivationKind::Charged,
            vec![fixture::counter("charge", "Charges", "count")],
        ),
        input(
            "mod:synthetic:conditional",
            ContentUnlockState::Unlocked,
            sts2_game_mod::RelicActivationKind::Conditional,
            vec![fixture::counter("threshold", "Threshold", "count")],
        ),
        input(
            "mod:synthetic:turn",
            ContentUnlockState::Unlocked,
            sts2_game_mod::RelicActivationKind::TurnCounter,
            vec![fixture::counter("turns", "Turns", "turn")],
        ),
        input(
            "mod:synthetic:room",
            ContentUnlockState::Unlocked,
            sts2_game_mod::RelicActivationKind::RoomCounter,
            vec![fixture::counter("rooms", "Rooms", "room")],
        ),
        input(
            "mod:synthetic:multiple",
            ContentUnlockState::Locked,
            sts2_game_mod::RelicActivationKind::Multiple,
            vec![
                fixture::counter("first", "First", "count"),
                fixture::counter("second", "Second", "count"),
            ],
        ),
    ];
    definitions[0].triggers = (0..5)
        .map(|index| RelicTriggerDefinition {
            id: format!("trigger:oversized:{index}"),
            label: "fixture".to_owned(),
            condition: Some(RelicCondition {
                id: format!("condition:oversized:{index}"),
                label: "x".repeat(16_000),
            }),
            visibility: RelicVisibility::Visible,
        })
        .collect();
    let source = CatalogSource {
        snapshot: Ok(RelicCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: RELIC_PRODUCER_VERSION.to_owned(),
            family: RelicFamilyCoverage {
                entity_kind: "relic".to_owned(),
                state: RelicFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions,
        }),
    };
    assert!(matches!(
        RelicCatalogProducer::new().produce(&manifest, &source),
        Err(RelicCatalogError::DefinitionTooLarge {
            limit: RELIC_MAX_DEFINITION_BYTES,
            ..
        })
    ));

    let catalog = catalog();
    let oversized_instance = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 12),
        instances: vec![{
            let mut value = instance(
                "instance:oversized",
                "mod:synthetic:badge",
                RelicField::NotApplicable,
                RelicField::NotApplicable,
            );
            value.accumulated = RelicField::Available(
                (0..200)
                    .map(|index| RelicAccumulatedValue {
                        id: format!("value:{index}"),
                        label: "x".repeat(100),
                        unit: unit("count"),
                        reset: RelicCounterReset::Run,
                        value: RelicField::Available(index),
                    })
                    .collect(),
            );
            value
        }],
    })
    .expect("live snapshot shape");
    assert!(matches!(
        RelicLiveReader::new(&catalog, oversized_instance),
        Err(RelicLiveError::DetailTooLarge {
            limit: RELIC_MAX_LIVE_DETAIL_BYTES,
            ..
        })
    ));
}

#[test]
fn nested_live_strings_are_validated_and_counted() {
    let catalog = catalog();
    let mut text_instance = instance(
        "instance:text",
        "mod:synthetic:charged",
        RelicField::Available(vec![RelicCounterState {
            id: "charge".to_owned(),
            value: RelicField::Available(1),
        }]),
        RelicField::Available(RelicActivationState {
            active: RelicField::Available(true),
            used: RelicField::Available(false),
        }),
    );
    text_instance.resolved_parameters = RelicField::Available(vec![RelicResolvedParameter {
        id: "amount".to_owned(),
        unit: unit("count"),
        value: RelicParameterValue::Text("x".repeat(RELIC_MAX_TEXT_BYTES)),
    }]);
    let text_snapshot = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 20),
        instances: vec![text_instance],
    })
    .expect("text snapshot");
    assert!(matches!(
        RelicLiveReader::new(&catalog, text_snapshot),
        Err(RelicLiveError::DetailTooLarge { .. })
    ));

    let mut trigger_instance = instance(
        "instance:trigger",
        "mod:synthetic:multiple",
        RelicField::Available(vec![
            RelicCounterState {
                id: "first".to_owned(),
                value: RelicField::Available(1),
            },
            RelicCounterState {
                id: "second".to_owned(),
                value: RelicField::Available(1),
            },
        ]),
        RelicField::Available(RelicActivationState {
            active: RelicField::Available(true),
            used: RelicField::Available(false),
        }),
    );
    trigger_instance.pending_triggers = RelicField::Available(vec![RelicPendingTrigger {
        id: "trigger:fixture".to_owned(),
        label: "Fixture trigger".to_owned(),
        condition: Some(RelicCondition {
            id: "condition:fixture".to_owned(),
            label: "invalid\ncondition".to_owned(),
        }),
        parameters: Vec::new(),
    }]);
    let trigger_snapshot = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&catalog, 21),
        instances: vec![trigger_instance],
    })
    .expect("trigger snapshot");
    assert!(matches!(
        RelicLiveReader::new(&catalog, trigger_snapshot),
        Err(RelicLiveError::InvalidInput("trigger_condition_label"))
    ));
}

#[test]
fn hidden_fields_are_rejected_and_owner_only_fields_require_scope() {
    let owner_catalog = catalog_with_parameter_visibility(RelicVisibility::OwnerOnly);
    let owner_reference = owner_catalog
        .reader()
        .list(&sts2_game_mod::RelicListQuery {
            scope: RelicVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("summary")
        .entries
        .into_iter()
        .find(|entry| entry.reference.relic_id == "mod:synthetic:charged")
        .expect("charged summary")
        .reference;
    assert_eq!(
        owner_catalog.get(&owner_reference, RelicVisibilityScope::Public),
        Err(RelicCatalogError::ExcludedByScope)
    );
    assert!(
        owner_catalog
            .get(&owner_reference, RelicVisibilityScope::Owner)
            .is_ok()
    );

    let mut live_instance = instance(
        "instance:owner",
        "mod:synthetic:charged",
        RelicField::Available(vec![RelicCounterState {
            id: "charge".to_owned(),
            value: RelicField::Available(1),
        }]),
        RelicField::Available(RelicActivationState {
            active: RelicField::Available(true),
            used: RelicField::Available(false),
        }),
    );
    live_instance.resolved_parameters = RelicField::Available(vec![RelicResolvedParameter {
        id: "amount".to_owned(),
        unit: unit("count"),
        value: RelicParameterValue::Integer(3),
    }]);
    let snapshot = RelicLiveSnapshot::from_input(RelicLiveSnapshotInput {
        binding: binding(&owner_catalog, 22),
        instances: vec![live_instance],
    })
    .expect("owner snapshot");
    let owner_snapshot = snapshot.clone();
    assert!(matches!(
        RelicLiveReader::new(&owner_catalog, snapshot),
        Err(RelicLiveError::InvalidState("parameter_visibility"))
    ));
    assert!(
        RelicLiveReader::new_with_scope(
            &owner_catalog,
            owner_snapshot,
            RelicVisibilityScope::Owner
        )
        .is_ok()
    );

    let hidden_catalog = catalog_with_parameter_visibility(RelicVisibility::Hidden);
    let hidden_reference = hidden_catalog
        .reader()
        .list(&sts2_game_mod::RelicListQuery {
            scope: RelicVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("hidden summary")
        .entries
        .into_iter()
        .find(|entry| entry.reference.relic_id == "mod:synthetic:charged")
        .expect("hidden charged summary")
        .reference;
    assert_eq!(
        hidden_catalog.get(&hidden_reference, RelicVisibilityScope::Owner),
        Err(RelicCatalogError::ExcludedByScope)
    );
}

fn catalog_with_parameter_visibility(visibility: RelicVisibility) -> sts2_game_mod::RelicCatalog {
    let manifest = manifest();
    let mut definitions = definitions();
    definitions
        .iter_mut()
        .find(|definition| definition.relic_id == "mod:synthetic:charged")
        .expect("charged definition")
        .parameters[0]
        .visibility = visibility;
    let source = CatalogSource {
        snapshot: Ok(RelicCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: RELIC_PRODUCER_VERSION.to_owned(),
            family: RelicFamilyCoverage {
                entity_kind: "relic".to_owned(),
                state: RelicFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions,
        }),
    };
    RelicCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("visibility catalog")
}
