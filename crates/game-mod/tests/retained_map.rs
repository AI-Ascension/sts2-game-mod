// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RETAINED_MAP_PRODUCER_VERSION, RUNTIME_MAP_V1_PROTOCOL_VERSION,
    RetainedMapContents, RetainedMapError, RetainedMapFieldStatus, RetainedMapFreshness,
    RetainedMapNodeKind, RetainedMapNodeReference, RetainedMapNodeVisibility,
    RetainedMapObservationState, RetainedMapReader, RetainedMapSnapshot, RetainedMapTopologyQuery,
    RetainedMapTravelActionability, RetainedMapTravelReference, RetainedMapUnavailableReason,
    RetainedMapVisibilityScope, UnavailableRetainedMapSource, verify_runtime_map_artifact,
};

fn query(limit: usize) -> RetainedMapTopologyQuery {
    RetainedMapTopologyQuery {
        limit,
        continuation: None,
    }
}

fn reader_with(
    scope: RetainedMapVisibilityScope,
    retained: RetainedMapSnapshot,
) -> RetainedMapReader {
    let mut reader = RetainedMapReader::new(scope);
    reader.replace_snapshot(retained).expect("replace");
    reader
}

#[test]
fn observe_then_close_serves_known_topology_with_honest_freshness() {
    let source = FixtureRetainedMapSource::new(snapshot(4));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(4), RetainedMapVisibilityScope::Public)
            .expect("reader");
    assert_eq!(reader.freshness(), RetainedMapFreshness::Current);
    assert_eq!(
        reader.observation(),
        RetainedMapObservationState::Observable
    );

    reader.close_screen();
    assert_eq!(reader.observation(), RetainedMapObservationState::Closed);
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
    assert_eq!(reader.freshness().code(), "retained");
    assert!(!reader.freshness().is_current());

    let page = reader.topology(&query(8)).expect("retained topology");
    assert_eq!(page.freshness, RetainedMapFreshness::Retained);
    assert_eq!(page.total, 4);
    assert!(page.complete, "retained topology is fully enumerated");
    assert!(page.continuation.is_none());
    assert_eq!(page.entries.len(), 4);
    assert!(page.binding.is_some());

    let first = page.entries[0].reference.clone();
    let node = reader.node(&first).expect("node");
    assert_eq!(node.kind, RetainedMapNodeKind::Start);
    assert_eq!(
        node.label.status(),
        RetainedMapFieldStatus::Available,
        "an already-public label stays readable while closed"
    );

    let closed_source = FixtureRetainedMapSource::with_observation(
        snapshot(4),
        RetainedMapObservationState::Closed,
    );
    assert_eq!(
        reader.observe(&closed_source, &binding(4)),
        Err(RetainedMapError::MapNotObservable(
            RetainedMapObservationState::Closed
        )),
        "a read must never open the closed screen"
    );
    assert_eq!(reader.observation(), RetainedMapObservationState::Closed);
    assert_eq!(reader.freshness(), RetainedMapFreshness::Retained);
}

#[test]
fn pre_observation_reads_stay_unavailable_without_forbidden_data() {
    let mut reader = RetainedMapReader::new(RetainedMapVisibilityScope::Public);
    assert_eq!(reader.freshness(), RetainedMapFreshness::NeverObserved);
    assert_eq!(reader.binding(), None);
    assert_eq!(
        reader.topology(&query(8)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::NeverObserved
        ))
    );
    assert!(reader.travel_references().is_empty());
    assert_eq!(
        reader.authorize_travel(&RetainedMapTravelReference {
            binding: binding(1),
            from_node_id: "map:1:0:0".to_owned(),
            to_node_id: "map:1:1:0".to_owned(),
            action_id: "select-map-node:1:left".to_owned(),
            actionability: RetainedMapTravelActionability::Current,
        }),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::NeverObserved
        ))
    );

    let forbidden = FixtureRetainedMapSource::with_observation(
        snapshot(1),
        RetainedMapObservationState::Forbidden,
    );
    assert_eq!(
        reader.observe(&forbidden, &binding(1)),
        Err(RetainedMapError::MapNotObservable(
            RetainedMapObservationState::Forbidden
        ))
    );

    let unavailable = UnavailableRetainedMapSource;
    assert_eq!(
        reader.observe(&unavailable, &binding(1)),
        Err(RetainedMapError::Unavailable(
            RetainedMapUnavailableReason::ExactHostEvidenceRequired
        ))
    );
}

#[test]
fn act_transition_restore_and_new_run_invalidate_retained_knowledge() {
    let source = FixtureRetainedMapSource::new(snapshot(3));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(3), RetainedMapVisibilityScope::Public)
            .expect("reader");
    reader.close_screen();
    assert_eq!(
        reader.reconcile(&binding(3)),
        RetainedMapFreshness::Retained
    );

    let mut transitioned = binding(4);
    transitioned.act_id = "act:2".to_owned();
    assert_eq!(reader.reconcile(&transitioned), RetainedMapFreshness::Stale);
    let page = reader
        .topology(&query(8))
        .expect("stale topology is disclosed");
    assert_eq!(page.freshness, RetainedMapFreshness::Stale);
    assert!(!page.complete, "stale knowledge is never complete");
    assert_eq!(
        reader.authorize_travel(&reader.travel_references()[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        ))
    );

    let mut run_changed = snapshot(4);
    run_changed.binding.run_id = "run:other".to_owned();
    assert_eq!(
        reader.replace_snapshot(RetainedMapSnapshot::from_input(run_changed).expect("run")),
        Err(RetainedMapError::RunMismatch)
    );
    let mut act_changed = snapshot(4);
    act_changed.binding.act_id = "act:other".to_owned();
    assert_eq!(
        reader.replace_snapshot(RetainedMapSnapshot::from_input(act_changed).expect("act")),
        Err(RetainedMapError::ActMismatch)
    );
    let mut mode_changed = snapshot(4);
    mode_changed.binding.mode_id = "mode:custom".to_owned();
    assert_eq!(
        reader.replace_snapshot(RetainedMapSnapshot::from_input(mode_changed).expect("mode")),
        Err(RetainedMapError::ModeMismatch)
    );
    let mut map_changed = snapshot(4);
    map_changed.binding.map_instance_id = "map-instance:2".to_owned();
    assert_eq!(
        reader.replace_snapshot(RetainedMapSnapshot::from_input(map_changed).expect("map")),
        Err(RetainedMapError::MapInstanceMismatch)
    );

    let restore = RetainedMapSnapshot::from_input(snapshot(2)).expect("restore");
    assert_eq!(
        reader.replace_snapshot(restore),
        Err(RetainedMapError::NonMonotonicEpoch {
            current: 3,
            supplied: 2
        })
    );
}

#[test]
fn hidden_future_contents_are_never_revealed() {
    let mut input = snapshot(4);
    input.nodes.push(hidden_node("map:1:9:9"));
    let retained = RetainedMapSnapshot::from_input(input).expect("snapshot");

    for scope in [
        RetainedMapVisibilityScope::Public,
        RetainedMapVisibilityScope::Owner,
    ] {
        let reader = reader_with(scope, retained.clone());
        assert_eq!(
            reader.node(&RetainedMapNodeReference {
                binding: binding(4),
                node_id: "map:1:9:9".to_owned(),
            }),
            Err(RetainedMapError::ScopeDenied("node"))
        );
    }

    let mut leaked_label = hidden_node("map:1:9:9");
    leaked_label.label = label("secret event");
    let mut input = snapshot(4);
    input.nodes.push(leaked_label);
    assert_eq!(
        RetainedMapSnapshot::from_input(input),
        Err(RetainedMapError::InvalidInput("hidden node detail"))
    );

    let mut leaked_contents = hidden_node("map:1:9:9");
    leaked_contents.contents = RetainedMapContents::PublicCategory;
    let mut input = snapshot(4);
    input.nodes.push(leaked_contents);
    assert_eq!(
        RetainedMapSnapshot::from_input(input),
        Err(RetainedMapError::InvalidInput("hidden node detail"))
    );

    let mut owner_only = snapshot(4);
    owner_only.nodes.push(node(
        "map:1:3:0",
        RetainedMapNodeKind::Shop,
        RetainedMapNodeVisibility::OwnerOnly,
    ));
    owner_only.edges.push(edge("map:1:2:0", "map:1:3:0"));
    let retained = RetainedMapSnapshot::from_input(owner_only).expect("snapshot");

    let mut public = reader_with(RetainedMapVisibilityScope::Public, retained.clone());
    assert_eq!(public.topology(&query(8)).expect("page").total, 4);

    let mut owner = reader_with(RetainedMapVisibilityScope::Owner, retained);
    assert_eq!(owner.topology(&query(8)).expect("page").total, 5);
}

#[test]
fn retained_reads_cannot_produce_actionable_stale_travel_references() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let live_travel = reader.travel_references();
    assert_eq!(live_travel.len(), 2);
    assert_eq!(
        live_travel[0].actionability,
        RetainedMapTravelActionability::Current
    );
    reader
        .authorize_travel(&live_travel[0])
        .expect("current travel");

    reader.close_screen();
    assert_eq!(
        reader.travel_actionability(),
        RetainedMapTravelActionability::Retained
    );
    assert_eq!(
        reader.authorize_travel(&live_travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Retained
        ))
    );

    assert_eq!(reader.reconcile(&binding(2)), RetainedMapFreshness::Stale);
    assert_eq!(
        reader.authorize_travel(&live_travel[0]),
        Err(RetainedMapError::TravelNotActionable(
            RetainedMapTravelActionability::Stale
        ))
    );

    reader.open_screen();
    let replacing = RetainedMapSnapshot::from_input(snapshot(2)).expect("snapshot");
    reader.replace_snapshot(replacing).expect("replace");
    assert_eq!(
        reader.authorize_travel(&live_travel[0]),
        Err(RetainedMapError::StaleReference),
        "an old-generation reference cannot become actionable again"
    );
}

#[test]
fn reveal_policy_change_withholds_retained_knowledge() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references();
    reader.withhold();
    assert_eq!(reader.freshness(), RetainedMapFreshness::Withheld);
    assert!(!reader.freshness().trusts_topology());
    let page = reader.topology(&query(8)).expect("withheld topology");
    assert_eq!(page.freshness, RetainedMapFreshness::Withheld);
    assert!(!page.complete, "withheld knowledge is never complete");
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
}

#[test]
fn producer_is_distinct_from_and_does_not_alter_runtime_map_v1() {
    assert_ne!(
        RETAINED_MAP_PRODUCER_VERSION,
        RUNTIME_MAP_V1_PROTOCOL_VERSION
    );
    assert_eq!(verify_runtime_map_artifact(), Ok(()));
    let retained = RetainedMapSnapshot::from_input(snapshot(1)).expect("snapshot");
    assert_eq!(retained.binding().catalog.locale, "en-US");
    assert_eq!(
        retained.binding().catalog.producer_version,
        RETAINED_MAP_PRODUCER_VERSION
    );
    assert!(
        retained
            .nodes()
            .values()
            .all(|node| node.node_id.starts_with("map:"))
    );
    let fixture_source = FixtureRetainedMapSource::new(snapshot(1));
    assert!(sts2_game_mod::RetainedMapSource::capability(&fixture_source).is_available());
}
