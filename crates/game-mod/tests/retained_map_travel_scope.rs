// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RetainedMapError, RetainedMapNodeKind, RetainedMapNodeVisibility,
    RetainedMapReader, RetainedMapSnapshotInput, RetainedMapTravelActionability,
    RetainedMapTravelReference, RetainedMapVisibilityScope,
};

const OWNER_ACTION: &str = "select-map-node:1:owner";

fn owner_only_destination() -> RetainedMapSnapshotInput {
    let mut input = snapshot(1);
    input.nodes.push(node(
        "map:1:3:0",
        RetainedMapNodeKind::Shop,
        RetainedMapNodeVisibility::OwnerOnly,
    ));
    input.edges.push(edge("map:1:2:0", "map:1:3:0"));
    input
        .travel
        .push(travel("map:1:0:0", "map:1:3:0", OWNER_ACTION));
    input
}

#[test]
fn public_reader_omits_and_refuses_owner_only_travel() {
    let source = FixtureRetainedMapSource::new(owner_only_destination());
    let reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    assert_eq!(travel.len(), 2, "owner-only destination is not disclosed");
    assert!(
        travel
            .iter()
            .all(|reference| reference.to_node_id != "map:1:3:0"),
        "no disclosed reference may reach the owner-only node"
    );

    let forged = RetainedMapTravelReference {
        binding: binding(1),
        from_node_id: "map:1:0:0".to_owned(),
        to_node_id: "map:1:3:0".to_owned(),
        action_id: OWNER_ACTION.to_owned(),
        actionability: RetainedMapTravelActionability::Current,
    };
    assert_eq!(
        reader.authorize_travel(&forged),
        Err(RetainedMapError::ScopeDenied("travel")),
        "authorization must enforce endpoint visibility"
    );
}

#[test]
fn owner_reader_sees_and_authorizes_owner_only_travel() {
    let source = FixtureRetainedMapSource::new(owner_only_destination());
    let reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Owner)
            .expect("reader");
    let travel = reader.travel_references().expect("travel");
    assert_eq!(travel.len(), 3);
    let owner = travel
        .iter()
        .find(|reference| reference.to_node_id == "map:1:3:0")
        .expect("owner-only travel");
    reader
        .authorize_travel(owner)
        .expect("owner-authorized travel");
}
