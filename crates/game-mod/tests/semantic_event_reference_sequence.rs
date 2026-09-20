// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/semantic_event_manifest.rs"]
mod manifest_fixture;
#[path = "support/semantic_event.rs"]
mod support;

use sts2_game_mod::{SemanticCoverageStatus, SemanticEventError};
use support::*;

#[test]
fn sequence_order_is_monotonic_and_contiguous_from_capture() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[3].sequence = 7;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::NonMonotonicSequence(7))
    ));
}

#[test]
fn a_repeated_sequence_number_is_refused_rather_than_renumbered() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[3].sequence = 3;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::NonMonotonicSequence(3))
    ));
}

#[test]
fn a_history_that_does_not_begin_where_capture_began_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.window.capture_start_sequence = 2;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::InvalidSequenceStart(2))
    ));
    let mut later = fixture_snapshot();
    later.batch.window.capture_start_sequence = 3;
    later.batch.window.history_before_capture = true;
    assert!(matches!(
        produce_snapshot(later),
        Err(SemanticEventError::InvalidSequenceStart(1))
    ));
}

#[test]
fn a_window_that_claims_nothing_preceded_a_capture_that_did_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.window.capture_start_sequence = 4;
    snapshot.batch.window.history_before_capture = false;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::InvalidSequenceStart(4))
    ));
}

#[test]
fn a_window_claiming_nothing_preceded_a_zero_start_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.window.history_before_capture = true;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::InvalidSequenceStart(1))
    ));
}

#[test]
fn declared_intervals_must_be_incomplete_ordered_and_disjoint() {
    let mut captured_span = fixture_snapshot();
    captured_span.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: SemanticCoverageStatus::Captured,
        first_sequence: 2,
        last_sequence: 3,
    }];
    assert!(matches!(
        produce_snapshot(captured_span),
        Err(SemanticEventError::InvalidCoverageInterval(2))
    ));

    let mut overlapping = fixture_snapshot();
    let mut events = fixture_events();
    events[1] = gap("event.2", 2, SemanticCoverageStatus::Dropped);
    events[2] = gap("event.3", 3, SemanticCoverageStatus::Dropped);
    overlapping.batch.events = events;
    overlapping.batch.window.intervals = vec![
        sts2_game_mod::SemanticCoverageInterval {
            status: SemanticCoverageStatus::Dropped,
            first_sequence: 2,
            last_sequence: 3,
        },
        sts2_game_mod::SemanticCoverageInterval {
            status: SemanticCoverageStatus::Dropped,
            first_sequence: 3,
            last_sequence: 3,
        },
    ];
    overlapping.family = fixture_family(&overlapping.batch);
    assert!(matches!(
        produce_snapshot(overlapping),
        Err(SemanticEventError::InvalidCoverageInterval(3))
    ));

    let mut reversed = fixture_snapshot();
    reversed.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: SemanticCoverageStatus::Dropped,
        first_sequence: 4,
        last_sequence: 2,
    }];
    assert!(matches!(
        produce_snapshot(reversed),
        Err(SemanticEventError::InvalidCoverageInterval(4))
    ));
}

#[test]
fn sequence_order_only_means_something_inside_one_scope() {
    let here = sts2_game_mod::SemanticEventSequence::new(fixture_scope(), 3);
    let mut elsewhere = fixture_scope();
    elsewhere.epoch = 13;
    let other = sts2_game_mod::SemanticEventSequence::new(elsewhere, 4);
    assert!(!here.shares_scope(&other));
    assert!(
        !here.precedes(&other),
        "two positions from different epochs are not one order"
    );
    let later = sts2_game_mod::SemanticEventSequence::new(fixture_scope(), 9);
    assert!(here.precedes(&later));
    assert!(!later.precedes(&here));
}

#[test]
fn two_histories_of_one_place_differ_only_by_epoch() {
    let mut other = fixture_scope();
    other.epoch = 99;
    assert!(fixture_scope().same_place(&other));
    let mut moved = fixture_scope();
    moved.branch_id = "branch.other".to_owned();
    assert!(!fixture_scope().same_place(&moved));
}

#[test]
fn an_unsupported_gap_and_a_dropped_gap_stay_distinguishable() {
    assert!(!SemanticCoverageStatus::Dropped.is_observed());
    assert!(!SemanticCoverageStatus::Unsupported.is_observed());
    assert!(SemanticCoverageStatus::Captured.is_observed());
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    events[1] = gap("event.2", 2, SemanticCoverageStatus::Unsupported);
    snapshot.batch.events = events;
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: SemanticCoverageStatus::Unsupported,
        first_sequence: 2,
        last_sequence: 2,
    }];
    snapshot.family = fixture_family(&snapshot.batch);
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let view = catalog.history().expect("history");
    assert_eq!(
        view.gaps()[0].event.coverage.status,
        SemanticCoverageStatus::Unsupported
    );
}
