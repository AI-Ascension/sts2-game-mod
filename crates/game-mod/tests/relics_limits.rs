// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/relics.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, RELIC_MAX_DEFINITION_BYTES, RELIC_MAX_LIVE_DETAIL_BYTES,
    RELIC_PRODUCER_VERSION, RelicAccumulatedValue, RelicCatalogError, RelicCatalogProducer,
    RelicCatalogSnapshot, RelicCounterReset, RelicFamilyCoverage, RelicFamilyState, RelicField,
    RelicLiveError, RelicLiveReader, RelicLiveSnapshot, RelicLiveSnapshotInput,
    RelicSemanticReference, RelicSemanticReferenceKind,
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
    definitions[0].references = (0..5)
        .map(|index| RelicSemanticReference {
            kind: RelicSemanticReferenceKind::Effect,
            id: format!("effect:oversized:{index}"),
            label: "x".repeat(16_000),
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
