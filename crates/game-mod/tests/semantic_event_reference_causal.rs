// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/semantic_event_manifest.rs"]
mod manifest_fixture;
#[path = "support/semantic_event.rs"]
mod support;

use sts2_game_mod::{
    SemanticCausalParent, SemanticCausalProvenance, SemanticEventError, SemanticEventKind,
    SemanticEventOrigin,
};
use support::*;

#[test]
fn a_stated_parent_and_its_child_are_both_visible_in_causal_order() {
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog.history().expect("history");
    assert!(view.causal_pair_visible("event.1", "event.2"));
    assert!(!view.causal_pair_visible("event.2", "event.1"));
}

#[test]
fn no_parent_named_is_a_disclosure_rather_than_an_inference() {
    let parent = SemanticCausalParent::not_stated();
    assert_eq!(parent.provenance, SemanticCausalProvenance::NotStated);
    assert_eq!(parent.stated_parent(), None);
    assert_eq!(parent.parent_event_id, None);
    let stated = SemanticCausalParent::stated("event.1");
    assert_eq!(stated.provenance, SemanticCausalProvenance::Stated);
    assert_eq!(stated.stated_parent(), Some("event.1"));
}

#[test]
fn a_stated_parent_that_does_not_exist_in_the_history_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].causal_parent = Some(SemanticCausalParent::stated("event.absent"));
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::StatedParentUnknown(id)) if id == "event.absent"
    ));
}

#[test]
fn a_stated_parent_that_does_not_precede_its_child_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].causal_parent = Some(SemanticCausalParent::stated("event.3"));
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::StatedParentNotBefore(id)) if id == "event.3"
    ));
}

#[test]
fn a_parent_named_without_the_stated_flag_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].causal_parent = Some(SemanticCausalParent {
        parent_event_id: Some("event.1".to_owned()),
        provenance: SemanticCausalProvenance::NotStated,
    });
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::CausalProvenanceMismatch(id)) if id == "event.2"
    ));
}

#[test]
fn the_stated_flag_without_a_named_parent_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].causal_parent = Some(SemanticCausalParent {
        parent_event_id: None,
        provenance: SemanticCausalProvenance::Stated,
    });
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::CausalProvenanceMismatch(id)) if id == "event.2"
    ));
}

#[test]
fn a_kind_that_admits_a_cause_must_state_one_or_its_absence() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].causal_parent = None;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::MissingCausalParent(id)) if id == "event.3"
    ));
}

#[test]
fn a_kind_that_admits_no_cause_omits_it_rather_than_stating_its_absence() {
    assert!(!SemanticEventKind::CardPlayed.admits_cause());
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog.history().expect("history");
    let card = view
        .events
        .iter()
        .find(|event| event.event_id() == "event.1")
        .expect("card play");
    assert!(card.event.causal_parent.is_none());
}

#[test]
fn a_transition_carries_no_cause_because_it_has_none_to_state() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[4].causal_parent = Some(SemanticCausalParent::not_stated());
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::CausalityNotAdmitted(id)) if id == "event.5"
    ));
}

#[test]
fn an_imported_event_never_states_a_parent() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].origin = Some(SemanticEventOrigin::Imported);
    snapshot.batch.events[1].causal_parent = Some(SemanticCausalParent::stated("event.1"));
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::CausalityNotAdmitted(id)) if id == "event.2"
    ));
    let mut again = fixture_snapshot();
    again.batch.events[1].origin = Some(SemanticEventOrigin::Imported);
    again.batch.events[1].causal_parent = Some(SemanticCausalParent::not_stated());
    let catalog = produce_snapshot(again).expect("catalog");
    let view = catalog.history().expect("history");
    let damage = view
        .events
        .iter()
        .find(|event| event.event_id() == "event.2")
        .expect("damage");
    assert_eq!(damage.event.origin, Some(SemanticEventOrigin::Imported));
}

#[test]
fn a_parent_that_is_only_a_disclosed_gap_is_not_a_stated_cause() {
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    events[0] = gap("event.1", 1, sts2_game_mod::SemanticCoverageStatus::Dropped);
    snapshot.batch.events = events;
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: sts2_game_mod::SemanticCoverageStatus::Dropped,
        first_sequence: 1,
        last_sequence: 1,
    }];
    snapshot.family = fixture_family(&snapshot.batch);
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::StatedParentUnknown(id)) if id == "event.1"
    ));
}

#[test]
fn only_effect_kinds_admit_a_cause() {
    assert!(SemanticEventKind::Damage.admits_cause());
    assert!(SemanticEventKind::PileMoved.admits_cause());
    assert!(!SemanticEventKind::RoomTransitioned.admits_cause());
    assert!(!SemanticEventKind::OfferPresented.admits_cause());
}
