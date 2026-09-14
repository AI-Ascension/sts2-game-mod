// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RetainedMapError, RetainedMapFreshness, RetainedMapObservationState,
    RetainedMapReader, RetainedMapTravelActionability, RetainedMapVisibilityScope,
};

#[test]
fn reconcile_rejects_snapshot_identity_change_within_same_generation() {
    let source = FixtureRetainedMapSource::new(snapshot(4));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(4), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let mut rotated = binding(4);
    rotated.snapshot_id = "snapshot:rotated".to_owned();
    assert_eq!(reader.reconcile(&rotated), RetainedMapFreshness::Stale);
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Stale
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        )),
        "a rotated snapshot identity cannot leave old travel authorized"
    );

    assert_eq!(
        reader.reconcile(&binding(4)),
        RetainedMapFreshness::Current,
        "only the unchanged complete fence restores current authority"
    );
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Current
    );
    reader
        .authorize_travel(&travel[0])
        .expect("restored travel authority");
}

#[test]
fn rejected_closed_observation_demotes_current_authority() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let closed = FixtureRetainedMapSource::with_observation(
        snapshot(1),
        RetainedMapObservationState::Closed,
    );
    assert_eq!(
        reader.observe(&closed, &binding(1)),
        Err(RetainedMapError::MapNotObservable(
            RetainedMapObservationState::Closed
        ))
    );
    assert_eq!(reader.observation(), RetainedMapObservationState::Closed);
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Retained
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        )),
        "a rejected closed observation must revoke current travel authority"
    );
}

#[test]
fn rejected_run_mismatch_observation_marks_stale_and_refuses_old_travel() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let mut moved = snapshot(2);
    moved.binding.run_id = "run:other".to_owned();
    let other = FixtureRetainedMapSource::new(moved);
    let moved_binding = other.binding().clone();
    assert_eq!(
        reader.observe(&other, &moved_binding),
        Err(RetainedMapError::RunMismatch)
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert_eq!(reader.binding(), Some(&binding(1)));
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        )),
        "a rejected new-run observation must refuse old-run travel"
    );
}
