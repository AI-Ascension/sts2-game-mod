// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RetainedMapCapability, RetainedMapError, RetainedMapFreshness,
    RetainedMapLiveBinding, RetainedMapObservationState, RetainedMapReader,
    RetainedMapSnapshotInput, RetainedMapSource, RetainedMapSourceError,
    RetainedMapTravelActionability, RetainedMapVisibilityScope,
};

/// Test source that reports an observable preflight but fails the copying read with one error.
struct ReadErrorSource {
    observation: RetainedMapObservationState,
    error: RetainedMapSourceError,
}

impl RetainedMapSource for ReadErrorSource {
    fn capability(&self) -> RetainedMapCapability {
        RetainedMapCapability::SyntheticFixtureOnly
    }

    fn observation_state(&self) -> RetainedMapObservationState {
        self.observation
    }

    fn read_snapshot(
        &self,
        _expected: &RetainedMapLiveBinding,
        _scope: RetainedMapVisibilityScope,
    ) -> Result<RetainedMapSnapshotInput, RetainedMapSourceError> {
        Err(self.error)
    }
}

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

#[test]
fn source_reported_stale_read_revokes_current_authority() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let rotated = FixtureRetainedMapSource::new(snapshot(2));
    assert_eq!(
        reader.observe(&rotated, &binding(1)),
        Err(RetainedMapError::SourceStale),
        "a source that changed during copying reports a stale read"
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Stale,
        "a stale source read must mark the surface closed as well as stale"
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        )),
        "a stale source read must refuse old travel"
    );
}

#[test]
fn read_reported_not_observable_demotes_current_authority() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let closed = ReadErrorSource {
        observation: RetainedMapObservationState::Observable,
        error: RetainedMapSourceError::NotObservable,
    };
    assert_eq!(
        reader.observe(&closed, &binding(1)),
        Err(RetainedMapError::MapNotObservable(
            RetainedMapObservationState::Closed
        )),
        "an observable preflight cannot mask a closed copying surface"
    );
    assert_eq!(reader.observation(), RetainedMapObservationState::Closed);
    assert_eq!(
        reader.freshness(),
        RetainedMapFreshness::Retained,
        "read-time closure demotes current knowledge to retained"
    );
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Retained
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        )),
        "read-time closure must refuse old travel"
    );
}

#[test]
fn malformed_new_run_topology_still_invalidates_identity() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let mut malformed = snapshot(2);
    malformed.binding.run_id = "run:other".to_owned();
    malformed.nodes.push(malformed.nodes[0].clone());
    let moved = FixtureRetainedMapSource::new(malformed);
    let moved_binding = moved.binding().clone();
    assert_eq!(
        reader.observe(&moved, &moved_binding),
        Err(RetainedMapError::RunMismatch),
        "identity change must be reported before malformed topology"
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert_ne!(reader.freshness(), RetainedMapFreshness::Current);
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        )),
        "malformed new-run topology must still refuse old-run travel"
    );
}

#[test]
fn malformed_newer_generation_topology_revokes_authority() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let mut malformed = snapshot(2);
    malformed.nodes.push(malformed.nodes[0].clone());
    let replacement = FixtureRetainedMapSource::new(malformed);
    let replacement_binding = replacement.binding().clone();
    assert_eq!(
        reader.observe(&replacement, &replacement_binding),
        Err(RetainedMapError::DuplicateNode("map:1:0:0".to_owned())),
        "a malformed newer generation still reports its topology error"
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert_ne!(reader.freshness(), RetainedMapFreshness::Current);
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Stale,
        "a malformed replacement after identity validation revokes current authority"
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        )),
        "malformed newer-generation topology must refuse old travel"
    );

    let mut first_malformed = snapshot(1);
    first_malformed.nodes.push(first_malformed.nodes[0].clone());
    let first = FixtureRetainedMapSource::new(first_malformed);
    let first_binding = first.binding().clone();
    let mut first_reader = RetainedMapReader::new(RetainedMapVisibilityScope::Public);
    assert_eq!(
        first_reader.observe(&first, &first_binding),
        Err(RetainedMapError::DuplicateNode("map:1:0:0".to_owned()))
    );
    assert_eq!(
        first_reader.freshness(),
        RetainedMapFreshness::NeverObserved,
        "a first malformed observation must not invent stale authority"
    );
}

#[test]
fn transient_read_errors_preserve_current_authority() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");

    for error in [
        RetainedMapSourceError::Busy,
        RetainedMapSourceError::NoActiveSource,
        RetainedMapSourceError::AccessDenied,
        RetainedMapSourceError::Malformed,
    ] {
        let failing = ReadErrorSource {
            observation: RetainedMapObservationState::Observable,
            error,
        };
        assert!(
            reader.observe(&failing, &binding(1)).is_err(),
            "a transient {error:?} read still reports failure"
        );
        assert_eq!(
            reader.freshness(),
            RetainedMapFreshness::Current,
            "a transient {error:?} read must not revoke current authority"
        );
        assert_eq!(
            reader.observation(),
            RetainedMapObservationState::Observable
        );
        assert_eq!(
            reader.travel_actionability(),
            RetainedMapTravelActionability::Current
        );
        reader
            .authorize_travel(&travel[0])
            .expect("current travel survives a transient read failure");
    }
}
