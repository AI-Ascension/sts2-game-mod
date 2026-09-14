// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/retained_map.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    FixtureRetainedMapSource, RETAINED_MAP_MAX_DETAIL_BYTES, RETAINED_MAP_MAX_NODES,
    RETAINED_MAP_MAX_PAGE_ITEMS, RETAINED_MAP_MAX_SNAPSHOT_BYTES, RETAINED_MAP_MAX_TEXT_BYTES,
    RetainedMapContinuation, RetainedMapError, RetainedMapFreshness, RetainedMapNodeKind,
    RetainedMapNodeVisibility, RetainedMapReader, RetainedMapSnapshot, RetainedMapTopologyQuery,
    RetainedMapVisibilityScope,
};

fn source(epoch: u64, count: usize) -> FixtureRetainedMapSource {
    FixtureRetainedMapSource::new(page_snapshot(epoch, count))
}

fn query(limit: usize, continuation: Option<RetainedMapContinuation>) -> RetainedMapTopologyQuery {
    RetainedMapTopologyQuery {
        limit,
        continuation,
    }
}

#[test]
fn topology_pagination_is_bounded_and_continuations_single_use() {
    let source = source(1, 5);
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");

    let first = reader.topology(&query(2, None)).expect("first page");
    assert_eq!(first.total, 5);
    assert_eq!(first.entries.len(), 2);
    assert!(!first.complete);
    let token = first.continuation.expect("continuation");

    let reused = token.clone();
    let second = reader
        .topology(&query(2, Some(token)))
        .expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert_eq!(
        reader.topology(&query(2, Some(reused))),
        Err(RetainedMapError::InvalidContinuation),
        "a consumed continuation is single-use"
    );

    let fresh = reader.topology(&query(2, None)).expect("fresh page");
    let wrong_token = fresh.continuation.expect("fresh continuation");
    assert_eq!(
        reader.topology(&query(3, Some(wrong_token))),
        Err(RetainedMapError::InvalidContinuation),
        "a continuation is bound to its original limit"
    );

    let third_token = second.continuation.expect("third continuation");
    let third = reader
        .topology(&query(2, Some(third_token)))
        .expect("third page");
    assert_eq!(third.entries.len(), 1);
    assert!(third.complete);
    assert!(third.continuation.is_none());

    assert_eq!(
        reader.topology(&query(0, None)),
        Err(RetainedMapError::InvalidPageSize)
    );
    assert_eq!(
        reader.topology(&query(RETAINED_MAP_MAX_PAGE_ITEMS + 1, None)),
        Err(RetainedMapError::InvalidPageSize)
    );

    let mut other =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("other reader");
    let foreign = other.topology(&query(2, None)).expect("foreign page");
    let foreign_token = foreign.continuation.expect("foreign continuation");
    let mut unrelated =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("unrelated reader");
    assert_eq!(
        unrelated.topology(&query(2, Some(foreign_token))),
        Err(RetainedMapError::InvalidContinuation)
    );
}

#[test]
fn stale_pages_are_never_reported_complete() {
    let source = source(1, 3);
    let mut reader =
        RetainedMapReader::from_source(&source, &binding(1), RetainedMapVisibilityScope::Public)
            .expect("reader");
    reader.close_screen();
    assert_eq!(reader.reconcile(&binding(2)), RetainedMapFreshness::Stale);
    let page = reader.topology(&query(8, None)).expect("page");
    assert_eq!(page.total, 3);
    assert!(!page.complete, "stale knowledge is never complete");
    assert_eq!(page.freshness, RetainedMapFreshness::Stale);
}

#[test]
fn detail_and_snapshot_bounds_count_every_nested_string() {
    let long = detail_actual(RETAINED_MAP_MAX_TEXT_BYTES, RETAINED_MAP_MAX_TEXT_BYTES);
    let short = detail_actual(
        RETAINED_MAP_MAX_TEXT_BYTES - 10,
        RETAINED_MAP_MAX_TEXT_BYTES,
    );
    assert!(long > RETAINED_MAP_MAX_DETAIL_BYTES);
    assert_eq!(
        long - short,
        10,
        "every nested custom-kind byte counts toward the detail bound"
    );

    let labels = bulk_snapshot(1, 100, 3 * 1024);
    assert!(matches!(
        RetainedMapSnapshot::from_input(labels),
        Err(RetainedMapError::DetailTooLarge {
            limit: RETAINED_MAP_MAX_SNAPSHOT_BYTES,
            ..
        })
    ));
}

fn detail_actual(kind_len: usize, label_len: usize) -> usize {
    match RetainedMapSnapshot::from_input(oversized_node_snapshot(1, kind_len, label_len)) {
        Err(RetainedMapError::DetailTooLarge { actual, .. }) => actual,
        other => {
            assert!(other.is_err(), "expected an oversized detail");
            0
        }
    }
}

#[test]
fn malformed_or_ambiguous_topology_is_rejected_before_publication() {
    let duplicate = {
        let mut input = snapshot(1);
        input.nodes.push(node(
            "map:1:0:0",
            RetainedMapNodeKind::Combat,
            RetainedMapNodeVisibility::Public,
        ));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(duplicate),
        Err(RetainedMapError::DuplicateNode("map:1:0:0".to_owned()))
    );

    let duplicate_edge = {
        let mut input = snapshot(1);
        input.edges.push(edge("map:1:0:0", "map:1:1:0"));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(duplicate_edge),
        Err(RetainedMapError::DuplicateEdge {
            from: "map:1:0:0".to_owned(),
            to: "map:1:1:0".to_owned(),
        })
    );

    let unknown = {
        let mut input = snapshot(1);
        input.edges.push(edge("map:1:0:0", "map:missing"));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(unknown),
        Err(RetainedMapError::UnknownNode("map:missing".to_owned()))
    );

    let self_edge = {
        let mut input = snapshot(1);
        input.nodes.push(node(
            "map:1:4:0",
            RetainedMapNodeKind::Rest,
            RetainedMapNodeVisibility::Public,
        ));
        input.edges.push(edge("map:1:4:0", "map:1:4:0"));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(self_edge),
        Err(RetainedMapError::InvalidInput("self_edge"))
    );

    let duplicate_travel = {
        let mut input = snapshot(1);
        input
            .travel
            .push(travel("map:1:0:0", "map:1:1:0", "select-map-node:1:left"));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(duplicate_travel),
        Err(RetainedMapError::DuplicateTravel(
            "select-map-node:1:left".to_owned()
        ))
    );

    let ambiguous = {
        let mut input = snapshot(1);
        input
            .travel
            .push(travel("map:1:0:0", "map:1:1:0", "map:1:0:0"));
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(ambiguous),
        Err(RetainedMapError::AmbiguousIdentity("action_id"))
    );

    let wrong_producer = {
        let mut input = snapshot(1);
        input.binding.catalog.producer_version = "other-producer".to_owned();
        input
    };
    assert_eq!(
        RetainedMapSnapshot::from_input(wrong_producer),
        Err(RetainedMapError::InvalidBinding("producer_version"))
    );

    assert_eq!(
        RetainedMapSnapshot::from_input(page_snapshot(1, RETAINED_MAP_MAX_NODES + 1)),
        Err(RetainedMapError::InvalidInput("nodes"))
    );

    let stale_source = source(1, 2);
    assert!(matches!(
        RetainedMapReader::from_source(
            &stale_source,
            &binding(9),
            RetainedMapVisibilityScope::Public
        ),
        Err(RetainedMapError::SourceStale)
    ));
}
