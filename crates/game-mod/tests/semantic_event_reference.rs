// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/semantic_event_manifest.rs"]
mod manifest_fixture;
#[path = "support/semantic_event.rs"]
mod support;

use sts2_game_mod::{
    SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION, SemanticCoverageStatus, SemanticEventKind,
    SemanticEventOrigin, SemanticFamilyState, SemanticHistoryAuthority, SemanticHistoryScope,
};
use support::*;

#[test]
fn producing_binds_the_history_to_one_manifest_and_producer() {
    let (manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(
        catalog.binding().producer_version,
        SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().state, SemanticFamilyState::Handled);
    assert_eq!(catalog.family().event_count, 5);
    assert_eq!(catalog.family().gap_count, 0);
}

#[test]
fn the_inventory_is_closed_and_every_kind_has_a_stable_name() {
    let names = SemanticEventKind::ALL
        .iter()
        .map(|kind| kind.name())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 14);
    assert_eq!(names[0], "card_played");
    assert!(names.contains(&"room_transitioned"));
    let unique = names.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), names.len(), "names are unique");
    assert_eq!(SemanticEventOrigin::ALL.len(), 3);
    assert_eq!(SemanticEventOrigin::Native.name(), "native");
}

#[test]
fn every_origin_states_whether_it_admits_a_parent() {
    assert!(SemanticEventOrigin::Native.admits_stated_parent());
    assert!(SemanticEventOrigin::Derived.admits_stated_parent());
    assert!(
        !SemanticEventOrigin::Imported.admits_stated_parent(),
        "an imported event's causality was settled when it was captured"
    );
}

#[test]
fn a_read_publishes_the_capability_it_withholds() {
    let (_manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.authority(), SemanticHistoryAuthority::NotGranted);
    let view = catalog.history().expect("history");
    assert_eq!(view.authority, SemanticHistoryAuthority::NotGranted);
    assert_eq!(view.scope, SemanticHistoryScope::History);
    assert_eq!(view.event_scope, fixture_scope());
}

#[test]
fn a_gap_is_a_record_that_occupies_its_sequence_number() {
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    events[2] = gap("event.3", 3, SemanticCoverageStatus::Dropped);
    snapshot.batch.events = events;
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: SemanticCoverageStatus::Dropped,
        first_sequence: 3,
        last_sequence: 3,
    }];
    snapshot.family = fixture_family(&snapshot.batch);
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let view = catalog.history().expect("history");
    assert_eq!(view.total, 5);
    assert_eq!(view.observed().len(), 4);
    assert_eq!(view.gaps().len(), 1);
    assert_eq!(view.gaps()[0].sequence().sequence, 3);
    assert_eq!(view.observed_kinds().len(), 4);
}

#[test]
fn a_captured_record_inside_a_declared_gap_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: SemanticCoverageStatus::Unsupported,
        first_sequence: 2,
        last_sequence: 4,
    }];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(sts2_game_mod::SemanticEventError::CoverageContradiction(2))
    ));
}

#[test]
fn a_gap_outside_a_declared_interval_is_refused() {
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    events[1] = gap("event.2", 2, SemanticCoverageStatus::Dropped);
    snapshot.batch.events = events;
    snapshot.family = fixture_family(&snapshot.batch);
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(sts2_game_mod::SemanticEventError::UndeclaredGap(2))
    ));
}

#[test]
fn an_empty_family_is_explicit_rather_than_a_history_of_zero_events() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = sts2_game_mod::SemanticFamilyCoverage {
        state: SemanticFamilyState::Unavailable,
        event_count: 0,
        gap_count: 0,
    };
    snapshot.batch = sts2_game_mod::SemanticEventBatch {
        scope: sts2_game_mod::SemanticEventScope {
            run_id: String::new(),
            branch_id: String::new(),
            episode: 0,
            epoch: 0,
        },
        window: sts2_game_mod::SemanticCaptureWindow {
            capture_start_sequence: 0,
            history_before_capture: false,
            intervals: Vec::new(),
        },
        events: Vec::new(),
    };
    let catalog = produce_snapshot(snapshot).expect("catalog");
    assert_eq!(catalog.family().state, SemanticFamilyState::Unavailable);
    assert!(matches!(
        catalog.history(),
        Err(sts2_game_mod::SemanticEventError::UnavailableFamily)
    ));
}

#[test]
fn a_declared_unavailable_family_carrying_a_history_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = sts2_game_mod::SemanticFamilyCoverage {
        state: SemanticFamilyState::Unavailable,
        event_count: 5,
        gap_count: 0,
    };
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(sts2_game_mod::SemanticEventError::InvalidInput("batch"))
    ));
}

#[test]
fn the_capture_window_discloses_whether_history_predates_it() {
    let window = fixture_window();
    assert!(window.starts_at_beginning());
    let later = sts2_game_mod::SemanticCaptureWindow {
        capture_start_sequence: 9,
        history_before_capture: true,
        intervals: Vec::new(),
    };
    assert!(!later.starts_at_beginning());
    assert!(later.interval_covering(4).is_none());
}
