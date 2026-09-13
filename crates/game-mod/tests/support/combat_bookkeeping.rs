// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    CombatBookkeepingBinding, CombatBookkeepingReader, CombatBookkeepingSnapshot,
    CombatCardInstanceReference, CombatCardMembership, CombatCardPosition,
    CombatCompositionCompleteness, CombatCounter, CombatCounterKind, CombatCounterProvenance,
    CombatCounterReset, CombatCounters, CombatDeckReconciliation, CombatField, CombatOrder,
    CombatPending, CombatResolutionState, CombatSnapshotInput, CombatTurnIdentity, CombatTurnOwner,
    CombatZoneInput, CombatZoneInventory, CombatZoneKind, CombatZoneStatus,
};

pub fn binding(epoch: u64) -> CombatBookkeepingBinding {
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

pub fn card(
    read: &CombatBookkeepingBinding,
    instance_id: &str,
    definition_id: &str,
) -> CombatCardInstanceReference {
    CombatCardInstanceReference::new(read.clone(), instance_id, definition_id, "player-1")
        .expect("card reference")
}

pub fn member(
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

pub fn base_input(epoch: u64) -> CombatSnapshotInput {
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
    let mut zones = CombatZoneKind::all()
        .iter()
        .copied()
        .map(|kind| CombatZoneInput {
            kind,
            total: CombatField::Available(0),
            composition: CombatField::Available(Vec::new()),
            completeness: CombatCompositionCompleteness::Complete,
            ordering: CombatOrder::NotApplicable,
        })
        .collect::<Vec<_>>();
    let draw = zones
        .iter_mut()
        .find(|zone| zone.kind == CombatZoneKind::Draw)
        .expect("draw zone");
    draw.total = CombatField::Available(2);
    draw.composition = CombatField::Available(draw_cards);
    draw.ordering = CombatOrder::Unordered;
    let temporary = zones
        .iter_mut()
        .find(|zone| zone.kind == CombatZoneKind::Temporary)
        .expect("temporary zone");
    temporary.total = CombatField::Available(1);
    temporary.composition = CombatField::Available(temporary_cards);
    let mut inventory = CombatZoneInventory::new();
    for zone in CombatZoneKind::all() {
        inventory.set(*zone, CombatZoneStatus::Available);
    }
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

pub fn zone_mut(input: &mut CombatSnapshotInput, kind: CombatZoneKind) -> &mut CombatZoneInput {
    input
        .zones
        .iter_mut()
        .find(|zone| zone.kind == kind)
        .expect("zone")
}

pub fn reader(epoch: u64) -> CombatBookkeepingReader {
    CombatBookkeepingReader::new(
        CombatBookkeepingSnapshot::from_input(base_input(epoch)).expect("snapshot"),
    )
    .expect("reader")
}
