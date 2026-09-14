// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RetainedMapCapability, RetainedMapError, RetainedMapFreshness, RetainedMapLiveBinding,
    RetainedMapObservationState, RetainedMapReader, RetainedMapSnapshotInput, RetainedMapSource,
    RetainedMapSourceError, RetainedMapTravelActionability, RetainedMapUnavailableReason,
    RetainedMapVisibilityScope, UnavailableRetainedMapSource,
};

/// Source whose capability and surface state can be switched between observations.
struct SurfaceSource {
    snapshot: RetainedMapSnapshotInput,
    observation: RetainedMapObservationState,
    capability: RetainedMapCapability,
}

impl SurfaceSource {
    fn available(observation: RetainedMapObservationState) -> Self {
        Self {
            snapshot: snapshot(1),
            observation,
            capability: RetainedMapCapability::SyntheticFixtureOnly,
        }
    }

    fn unavailable(observation: RetainedMapObservationState) -> Self {
        Self {
            snapshot: snapshot(1),
            observation,
            capability: RetainedMapCapability::Unavailable(
                RetainedMapUnavailableReason::NoActiveSource,
            ),
        }
    }
}

impl RetainedMapSource for SurfaceSource {
    fn capability(&self) -> RetainedMapCapability {
        self.capability
    }

    fn observation_state(&self) -> RetainedMapObservationState {
        self.observation
    }

    fn read_snapshot(
        &self,
        expected: &RetainedMapLiveBinding,
        _scope: RetainedMapVisibilityScope,
    ) -> Result<RetainedMapSnapshotInput, RetainedMapSourceError> {
        if &self.snapshot.binding != expected {
            return Err(RetainedMapSourceError::Stale);
        }
        Ok(self.snapshot.clone())
    }
}

#[test]
fn capability_loss_revokes_current_authority() {
    let source = SurfaceSource::available(RetainedMapObservationState::Observable);
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let lost = SurfaceSource::unavailable(RetainedMapObservationState::Observable);
    assert_eq!(
        reader.observe(&lost, &binding(1)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::NoActiveSource
        )),
        "a capability loss is reported through the unavailable path"
    );
    assert_eq!(
        reader.freshness(),
        RetainedMapFreshness::Retained,
        "a capability loss must demote current authority even when the surface looks observable"
    );
    assert_ne!(reader.freshness(), RetainedMapFreshness::Current);
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Retained
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        )),
        "a capability loss must refuse old travel"
    );
}

#[test]
fn capability_unavailable_with_non_observable_surface_reports_surface_state() {
    let source = SurfaceSource::available(RetainedMapObservationState::Observable);
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let closed = SurfaceSource::unavailable(RetainedMapObservationState::Closed);
    assert_eq!(
        reader.observe(&closed, &binding(1)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::NoActiveSource
        ))
    );
    assert_eq!(
        reader.observation(),
        RetainedMapObservationState::Closed,
        "the reported non-observable surface state must be adopted"
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        )),
        "capability loss plus closure must refuse old travel"
    );
}

#[test]
fn builtin_unavailable_source_revokes_current_authority() {
    let source = SurfaceSource::available(RetainedMapObservationState::Observable);
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    let unavailable = UnavailableRetainedMapSource;
    assert_eq!(
        reader.observe(&unavailable, &binding(1)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::ExactHostEvidenceRequired
        ))
    );
    assert_eq!(
        reader.observation(),
        RetainedMapObservationState::Unsupported
    );
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
    assert_ne!(reader.freshness(), RetainedMapFreshness::Current);
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        )),
        "the built-in unavailable source must refuse old travel on a current reader"
    );
}
