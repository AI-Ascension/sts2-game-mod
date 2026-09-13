// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use sts2_game_mod::{
    CombatBookkeepingBinding, CombatBookkeepingCapability, CombatBookkeepingError,
    CombatBookkeepingReader, CombatBookkeepingSnapshot, CombatBookkeepingSource,
    CombatCardInstanceReference, CombatCardMembership, CombatCardPosition,
    CombatCompositionCompleteness, CombatCounter, CombatCounterKind, CombatCounterProvenance,
    CombatCounterReset, CombatCounters, CombatDeckReconciliation, CombatField, CombatOrder,
    CombatPending, CombatResolutionState, CombatSnapshotInput, CombatSourceError,
    CombatTurnIdentity, CombatTurnOwner, CombatVisibilityScope, CombatZoneInput,
    CombatZoneInventory, CombatZoneKind, CombatZoneStatus, FixtureCombatBookkeepingSource,
    UnavailableCombatBookkeepingSource,
};

fn binding(epoch: u64) -> CombatBookkeepingBinding {
    CombatBookkeepingBinding::new(
        "manifest-1",
        "game-1",
        "run-1",
        "combat-1",
        format!("snapshot-{epoch}"),
        epoch,
    )
    .expect("binding")
}

fn card(
    read: &CombatBookkeepingBinding,
    instance_id: &str,
    definition_id: &str,
) -> CombatCardInstanceReference {
    CombatCardInstanceReference::new(read.clone(), instance_id, definition_id, "player-1")
        .expect("card reference")
}

fn member(
    read: &CombatBookkeepingBinding,
    instance_id: &str,
    definition_id: &str,
    position: CombatCardPosition,
) -> CombatCardMembership {
    CombatCardMembership {
        card: card(read, instance_id, definition_id),
        position,
    }
}

fn counter(
    kind: CombatCounterKind,
    value: CombatField<u64>,
    provenance: CombatCounterProvenance,
) -> CombatCounter {
    CombatCounter {
        kind,
        value,
        reset: CombatCounterReset::Combat,
        provenance,
    }
}

fn counters() -> CombatCounters {
    CombatCounters {
        cards_played: counter(
            CombatCounterKind::CardsPlayed,
            CombatField::Available(2),
            CombatCounterProvenance::HostReported,
        ),
        damage_taken: counter(
            CombatCounterKind::DamageTaken,
            CombatField::Available(4),
            CombatCounterProvenance::SemanticHistory {
                history_id: "history-1".to_owned(),
                event_count: 3,
            },
        ),
        damage_dealt: counter(
            CombatCounterKind::DamageDealt,
            CombatField::Unknown,
            CombatCounterProvenance::Unknown,
        ),
        enemies_defeated: counter(
            CombatCounterKind::EnemiesDefeated,
            CombatField::NotObserved,
            CombatCounterProvenance::NotObserved,
        ),
    }
}

fn base_input(epoch: u64) -> CombatSnapshotInput {
    let read = binding(epoch);
    let draw_cards = vec![
        member(
            &read,
            "card-a",
            "base:strike",
            CombatCardPosition::NotObserved,
        ),
        member(
            &read,
            "card-b",
            "base:strike",
            CombatCardPosition::NotObserved,
        ),
    ];
    let temporary_cards = vec![member(
        &read,
        "generated-a",
        "generated:shiv",
        CombatCardPosition::NotApplicable,
    )];
    let zones = vec![
        CombatZoneInput {
            kind: CombatZoneKind::Draw,
            total: CombatField::Available(2),
            composition: CombatField::Available(draw_cards),
            completeness: CombatCompositionCompleteness::Complete,
            ordering: CombatOrder::Unordered,
        },
        CombatZoneInput {
            kind: CombatZoneKind::Temporary,
            total: CombatField::Available(1),
            composition: CombatField::Available(temporary_cards),
            completeness: CombatCompositionCompleteness::Complete,
            ordering: CombatOrder::NotApplicable,
        },
    ];
    let mut inventory = CombatZoneInventory::new();
    inventory.set(CombatZoneKind::Draw, CombatZoneStatus::Available);
    inventory.set(CombatZoneKind::Temporary, CombatZoneStatus::Available);
    CombatSnapshotInput {
        binding: read,
        turn: CombatField::Available(CombatTurnIdentity {
            round_id: "round-2".to_owned(),
            player_turn_id: "turn-4".to_owned(),
            owner: CombatTurnOwner::Player,
        }),
        zones,
        zone_inventory: inventory,
        deck: CombatDeckReconciliation {
            permanent_deck_count: CombatField::Available(2),
            combat_card_count: CombatField::Available(3),
            temporary_card_count: CombatField::Available(1),
        },
        counters: counters(),
        resolution: CombatField::Available(CombatResolutionState::Idle),
        pending: CombatField::Available(CombatPending::None),
    }
}

fn reader(epoch: u64) -> CombatBookkeepingReader {
    CombatBookkeepingReader::new(
        CombatBookkeepingSnapshot::from_input(base_input(epoch)).expect("snapshot"),
    )
    .expect("reader")
}

#[test]
fn public_draw_composition_is_unordered_and_permanent_deck_reconciles() {
    let reader = reader(7);
    assert_eq!(
        reader.draw_count().expect("draw count"),
        &CombatField::Available(2)
    );
    let composition = reader
        .zone_composition(CombatZoneKind::Draw)
        .expect("draw composition")
        .value()
        .expect("public composition");
    assert_eq!(composition.len(), 2);
    assert!(
        composition
            .iter()
            .all(|card| matches!(card.position, CombatCardPosition::NotObserved))
    );
    assert_eq!(
        reader
            .zone(CombatZoneKind::Temporary)
            .expect("temporary zone")
            .kind
            .code(),
        "temporary"
    );
    assert_eq!(
        reader
            .card(&composition[0].card)
            .expect("card lookup")
            .card
            .definition_id,
        "base:strike"
    );
}

#[test]
fn counters_keep_reset_provenance_and_unknown_values_explicit() {
    let reader = reader(7);
    let played = reader.counter(CombatCounterKind::CardsPlayed);
    assert_eq!(played.value, CombatField::Available(2));
    assert_eq!(played.reset, CombatCounterReset::Combat);
    assert_eq!(played.provenance, CombatCounterProvenance::HostReported);
    let damage = reader.counter(CombatCounterKind::DamageTaken);
    assert!(matches!(
        damage.provenance,
        CombatCounterProvenance::SemanticHistory { event_count: 3, .. }
    ));
    assert_eq!(
        reader.counter(CombatCounterKind::DamageDealt).value,
        CombatField::Unknown
    );
    assert_eq!(
        reader.counter(CombatCounterKind::EnemiesDefeated).value,
        CombatField::NotObserved
    );
}

#[test]
fn pending_public_selection_preserves_instance_identity() {
    let mut input = base_input(7);
    let read = input.binding.clone();
    let choice = card(&read, "card-a", "base:strike");
    input.pending = CombatField::Available(CombatPending::Selection {
        selection_id: "selection-1".to_owned(),
        kind: "discard".to_owned(),
        choices: vec![choice.clone()],
    });
    let reader = CombatBookkeepingReader::new(
        CombatBookkeepingSnapshot::from_input(input).expect("selection snapshot"),
    )
    .expect("selection reader");
    let pending = reader.pending().expect("pending").value().expect("value");
    assert!(matches!(
        pending,
        CombatPending::Selection { choices, .. } if choices == &vec![choice]
    ));
}

#[test]
fn replacement_epoch_fences_moved_cards_and_wrong_combat() {
    let mut reader = reader(7);
    let old = reader
        .zone_composition(CombatZoneKind::Draw)
        .expect("draw")
        .value()
        .expect("draw values")[0]
        .card
        .clone();
    let mut next = base_input(8);
    next.zones[0].kind = CombatZoneKind::Discard;
    next.zones[0].ordering = CombatOrder::Public;
    next.zones[0].composition = CombatField::Available(vec![
        member(
            &next.binding,
            "card-a",
            "base:strike",
            CombatCardPosition::Known(0),
        ),
        member(
            &next.binding,
            "card-b",
            "base:strike",
            CombatCardPosition::Known(1),
        ),
    ]);
    next.zone_inventory
        .set(CombatZoneKind::Draw, CombatZoneStatus::NotObserved);
    next.zone_inventory
        .set(CombatZoneKind::Discard, CombatZoneStatus::Available);
    let next_snapshot = CombatBookkeepingSnapshot::from_input(next).expect("next snapshot");
    reader.replace_snapshot(next_snapshot).expect("replacement");
    assert_eq!(
        reader.card(&old),
        Err(CombatBookkeepingError::StaleReference)
    );
    assert_eq!(
        reader.replace_snapshot(
            CombatBookkeepingSnapshot::from_input(base_input(8)).expect("same epoch")
        ),
        Err(CombatBookkeepingError::NonMonotonicEpoch {
            current: 8,
            supplied: 8
        })
    );
    let mut wrong = base_input(9);
    wrong.binding.combat_id = "combat-2".to_owned();
    for zone in &mut wrong.zones {
        if let CombatField::Available(cards) = &mut zone.composition {
            for card in cards {
                card.card.binding = wrong.binding.clone();
            }
        }
    }
    let wrong = CombatBookkeepingSnapshot::from_input(wrong).expect("different combat snapshot");
    assert_eq!(
        reader.replace_snapshot(wrong),
        Err(CombatBookkeepingError::CombatMismatch)
    );
}

#[test]
fn secret_draw_order_and_count_mismatch_fail_closed() {
    let mut input = base_input(7);
    input.zones[0].composition = CombatField::Available(vec![member(
        &input.binding,
        "card-a",
        "base:strike",
        CombatCardPosition::Known(0),
    )]);
    input.zones[0].completeness = CombatCompositionCompleteness::Partial;
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::SecretOrderExposed)
    );

    let mut input = base_input(7);
    input.zones[0].ordering = CombatOrder::Public;
    input.zones[0].composition = CombatField::Available(vec![member(
        &input.binding,
        "card-a",
        "base:strike",
        CombatCardPosition::Known(0),
    )]);
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::CountMismatch(CombatZoneKind::Draw))
    );
}

#[test]
fn unsupported_zone_and_owner_only_fields_are_not_empty_successes() {
    let mut input = base_input(7);
    input
        .zone_inventory
        .set(CombatZoneKind::Draw, CombatZoneStatus::Unsupported);
    assert!(matches!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::InvalidInput(
            "zone has no available inventory entry"
        ))
    ));

    let mut input = base_input(7);
    input.pending = CombatField::OwnerOnly;
    let reader = CombatBookkeepingReader::new(
        CombatBookkeepingSnapshot::from_input(input).expect("owner-only snapshot"),
    )
    .expect("public reader");
    assert_eq!(
        reader.pending(),
        Err(CombatBookkeepingError::VisibilityDenied("pending"))
    );
}

struct BusySource;

impl CombatBookkeepingSource for BusySource {
    fn capability(&self) -> CombatBookkeepingCapability {
        CombatBookkeepingCapability::SyntheticFixtureOnly
    }

    fn read_snapshot(
        &self,
        _expected: &CombatBookkeepingBinding,
        _scope: CombatVisibilityScope,
    ) -> Result<CombatSnapshotInput, CombatSourceError> {
        Err(CombatSourceError::Busy)
    }
}

#[test]
fn source_busy_stale_and_unavailable_states_are_typed() {
    let expected = binding(7);
    assert!(matches!(
        CombatBookkeepingReader::from_source(&BusySource, &expected),
        Err(CombatBookkeepingError::Busy)
    ));

    let fixture = FixtureCombatBookkeepingSource::new(base_input(7));
    assert!(matches!(
        CombatBookkeepingReader::from_source(&fixture, &binding(8)),
        Err(CombatBookkeepingError::SourceStale)
    ));
    assert!(matches!(
        CombatBookkeepingReader::from_source(&UnavailableCombatBookkeepingSource, &expected),
        Err(CombatBookkeepingError::Unavailable(
            sts2_game_mod::CombatUnavailableReason::ExactHostEvidenceRequired
        ))
    ));
}

#[test]
fn duplicate_live_instances_are_rejected_across_zones() {
    let mut input = base_input(7);
    let duplicate = member(
        &input.binding,
        "card-a",
        "base:strike",
        CombatCardPosition::NotApplicable,
    );
    input.zones[1].composition = CombatField::Available(vec![duplicate]);
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::DuplicateCard("card-a".to_owned()))
    );
}
