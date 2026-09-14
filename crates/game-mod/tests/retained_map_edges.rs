// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RetainedMapContinuation, RetainedMapNodeKind,
    RetainedMapNodeVisibility, RetainedMapReader, RetainedMapSnapshot, RetainedMapTopologyQuery,
    RetainedMapVisibilityScope,
};

fn query(limit: usize, continuation: Option<RetainedMapContinuation>) -> RetainedMapTopologyQuery {
    RetainedMapTopologyQuery {
        limit,
        continuation,
    }
}

fn public_reader(retained: RetainedMapSnapshot) -> RetainedMapReader {
    let mut reader = RetainedMapReader::new(RetainedMapVisibilityScope::Public);
    reader.replace_snapshot(retained).expect("replace");
    reader
}

#[test]
fn topology_exposes_visible_edges_in_deterministic_order() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let page = reader.topology(&query(8, None)).expect("page");
    assert_eq!(page.total, 4);
    assert_eq!(page.total_edges, 4);
    assert_eq!(page.edges.len(), 4);
    assert!(page.complete);
    assert!(
        page.edges.iter().all(|edge| edge.binding == binding(1)),
        "every edge carries the retained snapshot fence"
    );
    let keys = page
        .edges
        .iter()
        .map(|edge| (edge.from_node_id.clone(), edge.to_node_id.clone()))
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "edges are deterministically ordered");
}

#[test]
fn removing_retained_edges_changes_exposed_connectivity() {
    let mut full_reader =
        public_reader(RetainedMapSnapshot::from_input(snapshot(1)).expect("full snapshot"));
    let full_page = full_reader.topology(&query(8, None)).expect("full page");

    let mut trimmed_input = snapshot(1);
    trimmed_input
        .edges
        .retain(|edge| edge.to_node_id != "map:1:2:0");
    let mut trimmed_reader =
        public_reader(RetainedMapSnapshot::from_input(trimmed_input).expect("trimmed snapshot"));
    let trimmed_page = trimmed_reader
        .topology(&query(8, None))
        .expect("trimmed page");

    assert_eq!(full_page.total, trimmed_page.total, "nodes are unchanged");
    assert_eq!(full_page.total_edges, 4);
    assert_eq!(trimmed_page.total_edges, 2);
    assert_ne!(
        full_page.edges, trimmed_page.edges,
        "removed connectivity must not be silently identical"
    );
}

#[test]
fn edge_window_paginates_with_shared_limit() {
    let source = FixtureRetainedMapSource::new(snapshot(1));
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    let first = reader.topology(&query(2, None)).expect("first page");
    assert_eq!(first.entries.len(), 2);
    assert_eq!(first.edges.len(), 2);
    assert!(!first.complete, "an unexhausted edge window stays partial");
    let token = first.continuation.expect("continuation");

    let second = reader
        .topology(&query(2, Some(token)))
        .expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert_eq!(second.edges.len(), 2);
    assert!(second.complete);
    assert!(second.continuation.is_none());
}

#[test]
fn edges_require_both_endpoints_visible() {
    let mut input = snapshot(1);
    input.nodes.push(node(
        "map:1:3:0",
        RetainedMapNodeKind::Shop,
        RetainedMapNodeVisibility::OwnerOnly,
    ));
    input.edges.push(edge("map:1:2:0", "map:1:3:0"));
    let retained = RetainedMapSnapshot::from_input(input).expect("snapshot");

    let mut public = public_reader(retained.clone());
    let public_page = public.topology(&query(8, None)).expect("public page");
    assert_eq!(public_page.total_edges, 4);
    assert!(
        public_page
            .edges
            .iter()
            .all(|edge| edge.to_node_id != "map:1:3:0"),
        "an edge with an invisible endpoint is withheld"
    );

    let mut owner = RetainedMapReader::new(RetainedMapVisibilityScope::Owner);
    owner.replace_snapshot(retained).expect("replace");
    let owner_page = owner.topology(&query(8, None)).expect("owner page");
    assert_eq!(owner_page.total_edges, 5);
    assert!(
        owner_page
            .edges
            .iter()
            .any(|edge| edge.to_node_id == "map:1:3:0")
    );
}
