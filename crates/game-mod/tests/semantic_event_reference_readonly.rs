// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/semantic_event_manifest.rs"]
mod manifest_fixture;
#[path = "support/semantic_event.rs"]
mod support;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{SemanticEventError, SemanticHistoryAuthority, SemanticHistoryScope};
use support::*;

#[test]
fn a_matching_fence_reads_the_history_it_names() {
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog
        .current(&fence(), SemanticHistoryScope::History)
        .expect("current");
    assert_eq!(view.event_scope, fixture_scope());
    assert_eq!(view.observed().len(), 5);
}

#[test]
fn a_fence_naming_another_epoch_is_refused_rather_than_answered() {
    let (_manifest, catalog) = fixture_catalog();
    let mut stale = fence();
    stale.epoch = 13;
    assert!(matches!(
        catalog.current(&stale, SemanticHistoryScope::History),
        Err(SemanticEventError::EpochMismatch {
            expected: 13,
            actual: 12
        })
    ));
}

#[test]
fn a_fence_naming_another_place_is_refused_as_stale() {
    let (_manifest, catalog) = fixture_catalog();
    let mut elsewhere = fence();
    elsewhere.branch_id = "branch.other".to_owned();
    assert!(matches!(
        catalog.current(&elsewhere, SemanticHistoryScope::History),
        Err(SemanticEventError::StaleHistoryFence)
    ));
}

#[test]
fn an_empty_or_imprecise_fence_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut empty = fence();
    empty.run_id = String::new();
    assert!(matches!(
        catalog.current(&empty, SemanticHistoryScope::History),
        Err(SemanticEventError::StaleHistoryFence)
    ));
    let mut pathlike = fence();
    pathlike.run_id = "../run".to_owned();
    assert!(matches!(
        catalog.current(&pathlike, SemanticHistoryScope::History),
        Err(SemanticEventError::NonOpaqueIdentity("live_fence"))
    ));
}

#[test]
fn a_scope_that_does_not_cover_the_history_it_names_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut elsewhere = fence();
    elsewhere.episode = 2;
    assert!(matches!(
        catalog.current(&elsewhere, SemanticHistoryScope::History),
        Err(SemanticEventError::StaleHistoryFence)
    ));
}

#[test]
fn a_reference_carries_the_binding_it_was_produced_for() {
    let (_manifest, catalog) = fixture_catalog();
    let reference = event_reference(&catalog, "event.2");
    assert_eq!(reference.catalog, *catalog.binding());
    let view = catalog
        .event(&reference, SemanticHistoryScope::History)
        .expect("event");
    assert_eq!(view.total, 5);
}

#[test]
fn a_reference_from_another_catalog_is_refused_as_stale() {
    let (_manifest, catalog) = fixture_catalog();
    let mut foreign = event_reference(&catalog, "event.2");
    foreign.catalog.producer_version = "someone-else-v1".to_owned();
    assert!(matches!(
        catalog.event(&foreign, SemanticHistoryScope::History),
        Err(SemanticEventError::StaleReference)
    ));
}

#[test]
fn an_unknown_event_identity_is_not_found_rather_than_absent() {
    let (_manifest, catalog) = fixture_catalog();
    let missing = event_reference(&catalog, "event.absent");
    assert!(matches!(
        catalog.event(&missing, SemanticHistoryScope::History),
        Err(SemanticEventError::NotFound)
    ));
}

#[test]
fn listing_is_bounded_and_continues_with_a_single_use_token() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let aliased = reader.list_events(&list_query(&catalog, 2)).expect("page");
    assert_eq!(aliased.entries.len(), 2);
    assert_eq!(aliased.total, 5);
    assert!(!aliased.complete);
    let continuation = aliased.continuation.expect("continuation");

    let mut query = list_query(&catalog, 2);
    query.continuation = Some(continuation.clone());
    let second = reader.list_events(&query).expect("second page");
    assert_eq!(second.entries[0].sequence, 3);
    assert_eq!(
        second.entries[0].kind,
        Some(sts2_game_mod::SemanticEventKind::Block)
    );
    assert_eq!(second.authority, SemanticHistoryAuthority::NotGranted);

    let mut reused = list_query(&catalog, 2);
    reused.continuation = Some(continuation);
    assert!(matches!(
        reader.list_events(&reused),
        Err(SemanticEventError::InvalidContinuation)
    ));
}

#[test]
fn a_page_size_of_zero_or_above_its_bound_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    assert!(matches!(
        catalog.list_events(&list_query(&catalog, 0)),
        Err(SemanticEventError::InvalidPageSize)
    ));
    assert!(matches!(
        catalog.list_events(&list_query(&catalog, 4096)),
        Err(SemanticEventError::InvalidPageSize)
    ));
}

#[test]
fn listing_that_carries_a_live_fence_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut query = list_query(&catalog, 2);
    query.live_fence = Some(fence());
    assert!(matches!(
        catalog.list_events(&query),
        Err(SemanticEventError::ExcludedByScope)
    ));
}

#[test]
fn listing_that_names_another_history_is_refused_as_stale() {
    let (_manifest, catalog) = fixture_catalog();
    let mut query = list_query(&catalog, 2);
    query.reference.producer_version = "someone-else-v1".to_owned();
    assert!(matches!(
        catalog.list_events(&query),
        Err(SemanticEventError::StaleReference)
    ));
}

#[test]
fn listing_a_scope_that_does_not_cover_the_history_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut query = list_query(&catalog, 2);
    query.event_scope.epoch = 13;
    assert!(matches!(
        catalog.list_events(&query),
        Err(SemanticEventError::ExcludedByScope)
    ));
}

#[test]
fn a_summary_of_a_gap_carries_no_kind_or_origin() {
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    events[2] = gap("event.3", 3, sts2_game_mod::SemanticCoverageStatus::Dropped);
    snapshot.batch.events = events;
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: sts2_game_mod::SemanticCoverageStatus::Dropped,
        first_sequence: 3,
        last_sequence: 3,
    }];
    snapshot.family = fixture_family(&snapshot.batch);
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let page = catalog.list_events(&list_query(&catalog, 8)).expect("page");
    let gap_summary = page
        .entries
        .iter()
        .find(|entry| entry.sequence == 3)
        .expect("gap row");
    assert!(gap_summary.kind.is_none());
    assert!(gap_summary.origin.is_none());
    assert!(gap_summary.roles.is_empty());
}

#[test]
fn a_source_is_read_once_and_the_boundary_offers_no_mutating_seam() {
    let manifest = fixture_manifest();
    let fixture = source(fixture_snapshot());
    let catalog = sts2_game_mod::SemanticEventCatalogProducer::new()
        .produce(&manifest, &fixture)
        .expect("catalog");
    assert_eq!(fixture.reads.get(), 1);
    assert_eq!(catalog.authority(), SemanticHistoryAuthority::NotGranted);
    assert_eq!(
        catalog.reader().authority(),
        SemanticHistoryAuthority::NotGranted
    );
}
