// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/powers.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, PowerStatusAmount, PowerStatusCatalogError, PowerStatusDurationState,
    PowerStatusField, PowerStatusFieldStatus, PowerStatusLiveError, PowerStatusLiveReader,
    PowerStatusLiveSnapshot, PowerStatusLiveSnapshotInput, PowerStatusSourceError,
    PowerStatusVisibilityScope,
};

#[test]
fn definitions_preserve_timing_stacking_caps_and_reference_fences() {
    let catalog = catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Public,
            limit: 2,
            continuation: None,
        })
        .expect("first page");
    assert_eq!(first.total, 5);
    assert!(!first.complete);
    assert_eq!(
        first.entries[0].reference.definition_id,
        "mod:synthetic:marker"
    );
    assert_eq!(
        catalog
            .get(
                &first.entries[0].reference,
                PowerStatusVisibilityScope::Public
            )
            .expect("marker")
            .stacking
            .policy,
        sts2_game_mod::PowerStatusStackingPolicy::NonStacking
    );
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation.clone()),
        })
        .expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert!(!second.complete);
    let third = reader
        .list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Public,
            limit: 2,
            continuation: second.continuation.clone(),
        })
        .expect("third page");
    assert!(third.complete);
    assert_eq!(third.entries.len(), 1);
    assert_eq!(
        reader.list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation),
        }),
        Err(PowerStatusCatalogError::InvalidContinuation)
    );

    let strength = catalog
        .get(
            &first
                .entries
                .iter()
                .chain(second.entries.iter())
                .chain(third.entries.iter())
                .find(|entry| entry.reference.definition_id.ends_with("strength"))
                .expect("strength summary")
                .reference,
            PowerStatusVisibilityScope::Public,
        )
        .expect("strength");
    assert_eq!(
        strength.description,
        "Static timing and stacking rules remain separate from live values."
    );
    assert_eq!(
        strength.stacking.policy,
        sts2_game_mod::PowerStatusStackingPolicy::Additive
    );
    assert_eq!(
        strength.stacking.cap.as_ref().expect("cap").maximum,
        Some(99)
    );
    assert_eq!(
        strength.duration.rule,
        sts2_game_mod::PowerStatusDurationRule::Counter { unit: unit("turn") }
    );
    assert_eq!(strength.references.len(), 3);
    let poison = catalog
        .get(
            &second
                .entries
                .iter()
                .find(|entry| entry.reference.definition_id.ends_with("poison"))
                .expect("poison summary")
                .reference,
            PowerStatusVisibilityScope::Public,
        )
        .expect("poison");
    assert_eq!(
        poison.decay.rule,
        sts2_game_mod::PowerStatusDecayRule::By {
            amount: 1,
            unit: unit("damage")
        }
    );

    let owner_page = catalog
        .reader()
        .list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        })
        .expect("owner page");
    assert_eq!(owner_page.total, 6);
    let hidden = owner_page
        .entries
        .iter()
        .find(|entry| entry.reference.definition_id.ends_with("hidden"))
        .expect("owner hidden reference");
    let hidden_detail = catalog
        .get(&hidden.reference, PowerStatusVisibilityScope::Owner)
        .expect("owner hidden detail");
    assert_eq!(hidden_detail.unlock_state, ContentUnlockState::Locked);
    let mut stale = hidden_detail.reference;
    stale.catalog.manifest.catalog_generation += 1;
    assert_eq!(
        catalog.get(&stale, PowerStatusVisibilityScope::Owner),
        Err(PowerStatusCatalogError::StaleReference)
    );
}

#[test]
fn live_instances_cover_owners_sources_typed_amounts_and_expiry() {
    let catalog = catalog();
    let mut multi = instance(
        "instance:multi",
        "mod:synthetic:multi",
        sts2_game_mod::PowerStatusOwnerKind::Secondary,
        PowerStatusField::Available(PowerStatusAmount::Counters(vec![
            counter("stacks", 3, "count"),
            counter("turns", 2, "turn"),
        ])),
        PowerStatusField::Available(duration(4, "turn")),
    );
    multi.source = PowerStatusField::Available(source(
        sts2_game_mod::PowerStatusSourceKind::Relic,
        "relic:fixture",
    ));
    multi.pending_expiry = PowerStatusField::Available(expiry(1, "turn"));
    let input = PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 7),
        instances: vec![
            instance(
                "instance:strength:player",
                "mod:synthetic:strength",
                sts2_game_mod::PowerStatusOwnerKind::Player,
                PowerStatusField::Available(integer(2, "count")),
                PowerStatusField::Available(duration(2, "turn")),
            ),
            instance(
                "instance:strength:enemy",
                "mod:synthetic:strength",
                sts2_game_mod::PowerStatusOwnerKind::Enemy,
                PowerStatusField::Available(integer(4, "count")),
                PowerStatusField::Available(duration(1, "turn")),
            ),
            instance(
                "instance:vulnerable",
                "mod:synthetic:vulnerable",
                sts2_game_mod::PowerStatusOwnerKind::Ally,
                PowerStatusField::NotApplicable,
                PowerStatusField::Available(duration(1, "turn")),
            ),
            instance(
                "instance:poison",
                "mod:synthetic:poison",
                sts2_game_mod::PowerStatusOwnerKind::Enemy,
                PowerStatusField::Available(PowerStatusAmount::Decimal {
                    value: 25,
                    scale: 1,
                    unit: unit("damage"),
                }),
                PowerStatusField::Available(duration(3, "round")),
            ),
            multi,
            instance(
                "instance:marker",
                "mod:synthetic:marker",
                sts2_game_mod::PowerStatusOwnerKind::Player,
                PowerStatusField::Available(PowerStatusAmount::Boolean {
                    value: true,
                    unit: unit("active"),
                }),
                PowerStatusField::Available(PowerStatusDurationState::Permanent),
            ),
        ],
    };
    let snapshot = PowerStatusLiveSnapshot::from_input(input).expect("snapshot");
    let reader = PowerStatusLiveReader::new(&catalog, snapshot).expect("reader");
    assert_eq!(reader.references().len(), 6);
    let multi_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:multi")
        .expect("multi reference");
    let multi_detail = reader.get(&multi_reference).expect("multi detail");
    assert_eq!(
        multi_detail.owner.kind,
        sts2_game_mod::PowerStatusOwnerKind::Secondary
    );
    assert_eq!(
        multi_detail.source.value().expect("source").kind,
        sts2_game_mod::PowerStatusSourceKind::Relic
    );
    assert_eq!(
        multi_detail.amount.value().expect("amount"),
        &PowerStatusAmount::Counters(vec![
            counter("stacks", 3, "count"),
            counter("turns", 2, "turn"),
        ])
    );
    assert_eq!(
        multi_detail.pending_expiry.status(),
        PowerStatusFieldStatus::Available
    );
    let duplicate_definition_refs = reader
        .references()
        .into_iter()
        .filter(|reference| reference.definition_id.ends_with("strength"))
        .count();
    assert_eq!(duplicate_definition_refs, 2);
}

#[test]
fn epoch_game_run_and_source_fences_reject_stale_reads() {
    let catalog = catalog();
    let snapshot = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 1),
        instances: vec![instance(
            "instance:strength",
            "mod:synthetic:strength",
            sts2_game_mod::PowerStatusOwnerKind::Player,
            PowerStatusField::Available(integer(1, "count")),
            PowerStatusField::Available(duration(2, "turn")),
        )],
    })
    .expect("snapshot");
    let mut reader = PowerStatusLiveReader::new(&catalog, snapshot).expect("reader");
    let old_reference = reader.references().pop().expect("reference");
    let next = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 2),
        instances: vec![instance(
            "instance:strength",
            "mod:synthetic:strength",
            sts2_game_mod::PowerStatusOwnerKind::Player,
            PowerStatusField::Available(integer(2, "count")),
            PowerStatusField::Available(duration(1, "turn")),
        )],
    })
    .expect("next snapshot");
    reader.replace_snapshot(next).expect("replace");
    assert_eq!(
        reader.get(&old_reference),
        Err(PowerStatusLiveError::StaleReference)
    );

    let non_monotonic = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 2),
        instances: Vec::new(),
    })
    .expect("same epoch");
    assert_eq!(
        reader.replace_snapshot(non_monotonic),
        Err(PowerStatusLiveError::NonMonotonicEpoch {
            current: 2,
            supplied: 2
        })
    );

    let mut wrong_game = binding(&catalog, 3);
    wrong_game.game_instance_id = "game:other".to_owned();
    assert_eq!(
        reader.replace_snapshot(
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: wrong_game,
                instances: Vec::new(),
            })
            .expect("wrong game snapshot"),
        ),
        Err(PowerStatusLiveError::GameInstanceMismatch)
    );

    let expected = binding(&catalog, 4);
    let source = FixtureLiveSource {
        snapshot: Ok(PowerStatusLiveSnapshotInput {
            binding: binding(&catalog, 5),
            instances: Vec::new(),
        }),
    };
    assert!(matches!(
        PowerStatusLiveReader::from_source(&catalog, &expected, &source),
        Err(PowerStatusLiveError::StaleReference)
    ));
    let unavailable = FixtureLiveSource {
        snapshot: Err(PowerStatusSourceError::NoActiveSource),
    };
    assert!(matches!(
        PowerStatusLiveReader::from_source(&catalog, &expected, &unavailable),
        Err(PowerStatusLiveError::NoActiveSource)
    ));
}

#[test]
fn invalid_shapes_and_hidden_values_fail_closed_without_zero_defaults() {
    let catalog = catalog();
    let mut invalid = instance(
        "instance:bad",
        "mod:synthetic:vulnerable",
        sts2_game_mod::PowerStatusOwnerKind::Player,
        PowerStatusField::Available(integer(0, "count")),
        PowerStatusField::Available(duration(1, "turn")),
    );
    assert!(matches!(
        PowerStatusLiveReader::new(
            &catalog,
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: binding(&catalog, 1),
                instances: vec![invalid.clone()],
            })
            .expect("shape snapshot"),
        ),
        Err(PowerStatusLiveError::AmountShapeMismatch("amountless"))
    ));

    invalid.definition_id = "mod:synthetic:missing".to_owned();
    assert!(matches!(
        PowerStatusLiveReader::new(
            &catalog,
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: binding(&catalog, 2),
                instances: vec![invalid],
            })
            .expect("unknown snapshot"),
        ),
        Err(PowerStatusLiveError::UnknownDefinition(_))
    ));

    let mut missing_counter = instance(
        "instance:counter",
        "mod:synthetic:multi",
        sts2_game_mod::PowerStatusOwnerKind::Enemy,
        PowerStatusField::Available(PowerStatusAmount::Counters(vec![counter(
            "stacks", 2, "count",
        )])),
        PowerStatusField::Available(duration(1, "turn")),
    );
    assert!(matches!(
        PowerStatusLiveReader::new(
            &catalog,
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: binding(&catalog, 3),
                instances: vec![missing_counter.clone()],
            })
            .expect("missing counter snapshot"),
        ),
        Err(PowerStatusLiveError::MissingCounter { counter_id, .. }) if counter_id == "turns"
    ));
    missing_counter.amount = PowerStatusField::Denied;
    let reader = PowerStatusLiveReader::new(
        &catalog,
        PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
            binding: binding(&catalog, 4),
            instances: vec![missing_counter],
        })
        .expect("denied amount snapshot"),
    )
    .expect("denied amount reader");
    let detail = reader.get(&reader.references()[0]).expect("denied detail");
    assert_eq!(detail.amount.status(), PowerStatusFieldStatus::Denied);
    assert!(detail.amount.value().is_none());
}

#[derive(Clone, Debug)]
struct FixtureLiveSource {
    snapshot: Result<PowerStatusLiveSnapshotInput, PowerStatusSourceError>,
}

impl sts2_game_mod::PowerStatusLiveSource for FixtureLiveSource {
    fn read_live(
        &self,
        _expected: &sts2_game_mod::PowerStatusLiveBinding,
    ) -> Result<PowerStatusLiveSnapshotInput, PowerStatusSourceError> {
        self.snapshot.clone()
    }
}
