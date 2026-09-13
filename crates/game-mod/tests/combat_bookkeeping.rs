// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/combat_bookkeeping.rs"]
mod support;

use sts2_game_mod::{
    CombatBookkeepingBinding, CombatBookkeepingCapability, CombatBookkeepingError,
    CombatBookkeepingReader, CombatBookkeepingSnapshot, CombatBookkeepingSource,
    CombatCardPosition, CombatCompositionCompleteness, CombatCounterKind, CombatCounterProvenance,
    CombatCounterReset, CombatField, CombatOrder, CombatPending, CombatSnapshotInput,
    CombatSourceError, CombatVisibilityScope, CombatZoneKind, CombatZoneStatus,
    FixtureCombatBookkeepingSource, UnavailableCombatBookkeepingSource,
};
use support::*;

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
fn hidden_composition_is_canonicalized_independent_of_source_order() {
    let mut input = base_input(7);
    let draw = zone_mut(&mut input, CombatZoneKind::Draw);
    assert!(matches!(draw.composition, CombatField::Available(_)));
    if let CombatField::Available(cards) = &mut draw.composition {
        cards.reverse();
    }
    let snapshot = CombatBookkeepingSnapshot::from_input(input).expect("snapshot");
    let cards = snapshot
        .zone(CombatZoneKind::Draw)
        .expect("draw")
        .composition
        .value()
        .expect("composition");
    assert_eq!(
        cards
            .iter()
            .map(|card| card.card.instance_id.as_str())
            .collect::<Vec<_>>(),
        vec!["card-a", "card-b"]
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
fn pending_selection_validates_definition_and_owner_identity() {
    let mut input = base_input(7);
    let read = input.binding.clone();
    let mut choice = card(&read, "card-a", "base:strike");
    choice.definition_id = "invalid definition".to_owned();
    input.pending = CombatField::Available(CombatPending::Selection {
        selection_id: "selection-1".to_owned(),
        kind: "discard".to_owned(),
        choices: vec![choice],
    });
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::InvalidBinding(
            "choice_definition_id"
        ))
    );
}

#[test]
fn pending_effect_references_count_toward_snapshot_bound() {
    let mut input = base_input(7);
    input.pending = CombatField::Available(CombatPending::Effect {
        effect_id: "effect-1".to_owned(),
        kind: "damage".to_owned(),
        target_ids: (0..512).map(|index| format!("target-{index}")).collect(),
    });
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::InvalidInput("snapshot too large"))
    );
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
    let next_binding = next.binding.clone();
    let moved_cards = if let CombatField::Available(cards) =
        &mut zone_mut(&mut next, CombatZoneKind::Draw).composition
    {
        std::mem::take(cards)
    } else {
        Vec::new()
    };
    let draw = zone_mut(&mut next, CombatZoneKind::Draw);
    draw.total = CombatField::Available(0);
    draw.ordering = CombatOrder::NotApplicable;
    let discard = zone_mut(&mut next, CombatZoneKind::Discard);
    discard.total = CombatField::Available(2);
    discard.ordering = CombatOrder::Public;
    discard.composition = CombatField::Available(vec![
        member(
            &next_binding,
            "card-a",
            "base:strike",
            CombatCardPosition::Known(0),
        ),
        member(
            &next_binding,
            "card-b",
            "base:strike",
            CombatCardPosition::Known(1),
        ),
    ]);
    assert_eq!(moved_cards.len(), 2);
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
    let read = input.binding.clone();
    let draw = zone_mut(&mut input, CombatZoneKind::Draw);
    draw.composition = CombatField::Available(vec![member(
        &read,
        "card-a",
        "base:strike",
        CombatCardPosition::Known(0),
    )]);
    draw.completeness = CombatCompositionCompleteness::Partial;
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::SecretOrderExposed)
    );

    let mut input = base_input(7);
    let read = input.binding.clone();
    let draw = zone_mut(&mut input, CombatZoneKind::Draw);
    draw.ordering = CombatOrder::Public;
    draw.composition = CombatField::Available(vec![member(
        &read,
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

#[test]
fn unobserved_zone_cannot_be_treated_as_an_empty_reconciliation_zone() {
    let mut input = base_input(7);
    input
        .zone_inventory
        .set(CombatZoneKind::Discard, CombatZoneStatus::NotObserved);
    input
        .zones
        .retain(|zone| zone.kind != CombatZoneKind::Discard);
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::InvalidInput(
            "combat total requires complete zone inventory"
        ))
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
    let read = input.binding.clone();
    let duplicate = member(
        &read,
        "card-a",
        "base:strike",
        CombatCardPosition::NotApplicable,
    );
    zone_mut(&mut input, CombatZoneKind::Temporary).composition =
        CombatField::Available(vec![duplicate]);
    assert_eq!(
        CombatBookkeepingSnapshot::from_input(input),
        Err(CombatBookkeepingError::DuplicateCard("card-a".to_owned()))
    );
}
