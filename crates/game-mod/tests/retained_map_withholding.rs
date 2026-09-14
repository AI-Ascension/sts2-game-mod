// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RetainedMapError, RetainedMapFreshness, RetainedMapNodeReference,
    RetainedMapObservationState, RetainedMapReader, RetainedMapTopologyQuery,
    RetainedMapTravelActionability, RetainedMapUnavailableReason, RetainedMapVisibilityScope,
};

fn query(limit: usize) -> RetainedMapTopologyQuery {
    RetainedMapTopologyQuery {
        limit,
        continuation: None,
    }
}

#[test]
fn reveal_policy_withholding_is_enforced_and_survives_reconcile() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let node_reference = RetainedMapNodeReference {
        binding: binding(1),
        node_id: "map:1:0:0".to_owned(),
    };
    let travel = reader.travel_references().expect("travel");
    assert!(reader.node(&node_reference).is_ok());

    reader.withhold();
    assert!(reader.is_withheld());
    assert_eq!(reader.freshness(), RetainedMapFreshness::Withheld);
    assert!(!reader.freshness().trusts_topology());
    assert_eq!(
        reader.topology(&query(8)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::PolicyWithheld
        )),
        "withheld topology is refused, not served"
    );
    assert_eq!(
        reader.node(&node_reference),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::PolicyWithheld
        )),
        "withheld node detail is refused, not served"
    );
    assert_eq!(
        reader.travel_references(),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::PolicyWithheld
        )),
        "withheld travel references are refused, not served"
    );
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Withheld
    );
    assert_eq!(
        reader.authorize_travel(&travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Withheld
        ))
    );

    assert_eq!(
        reader.reconcile(&binding(1)),
        RetainedMapFreshness::Withheld,
        "a same-generation reconcile preserves the withholding"
    );
    assert_eq!(
        reader.reconcile(&binding(2)),
        RetainedMapFreshness::Withheld,
        "a generation-changing reconcile preserves the withholding"
    );
    assert!(reader.is_withheld());

    reader.unwithhold();
    assert!(!reader.is_withheld());
    assert_eq!(reader.freshness(), RetainedMapFreshness::Stale);
    assert!(reader.topology(&query(8)).is_ok());
}

#[test]
fn closing_and_reopening_without_reconcile_does_not_reauthorize_travel() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    reader.authorize_travel(&travel[0]).expect("current travel");

    reader.close_screen();
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
    reader.open_screen();
    assert_eq!(
        reader.observation(),
        RetainedMapObservationState::Observable
    );
    assert_eq!(
        reader.freshness(),
        RetainedMapFreshness::Retained,
        "opening the surface never promotes retained back to current"
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
        "reopening the surface without a fresh observation or reconcile must not re-arm travel"
    );
    assert_eq!(
        reader.travel_references().expect("travel")[0].actionability,
        RetainedMapTravelActionability::Retained
    );

    assert_eq!(
        reader.reconcile(&binding(1)),
        RetainedMapFreshness::Current,
        "an explicit reconcile while the surface is open re-verifies the map"
    );
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Current
    );
    reader
        .authorize_travel(&travel[0])
        .expect("reconciled travel");
}
