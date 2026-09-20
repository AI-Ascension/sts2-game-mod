// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{
    RunResultReadAuthority, RunResultSourceError, RunResultVisibilityScope, RunSummaryListQuery,
    is_opaque_identity,
};
use support::*;

fn query(limit: usize) -> RunSummaryListQuery {
    RunSummaryListQuery {
        locale: "en-US".to_owned(),
        profile_id: sts2_game_mod::RunResultFieldValue::absent(),
        scope: RunResultVisibilityScope::Owner,
        limit,
        continuation: None,
        live_fence: None,
    }
}

fn read_everything(catalog: &sts2_game_mod::RunResultCatalog) {
    let mut reader = catalog.reader();
    let _ = reader.list(&query(sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS));
    let _ = catalog.current(
        &sts2_game_mod::RunResultLiveFence {
            instance_id: "instance.alpha".to_owned(),
            run_id: "run.alpha.1".to_owned(),
            epoch: 7,
        },
        RunResultVisibilityScope::Owner,
    );
    for result_id in [
        "result.victory",
        "result.defeat",
        "result.abandonment",
        "result.pending",
        "result.hidden",
    ] {
        let _ = catalog.get(
            &sts2_game_mod::RunResultReference {
                catalog: catalog.binding().clone(),
                result_id: result_id.to_owned(),
            },
            RunResultVisibilityScope::Owner,
        );
    }
}

#[test]
fn the_source_is_consulted_exactly_once_per_production() {
    let manifest = fixture_manifest();
    let source =
        RunResultSource::answering(snapshot(&manifest, fixture_results(), fixture_summaries()));
    assert_eq!(source.reads(), 0);
    let catalog = produce_with(&manifest, &source).expect("catalog");
    assert_eq!(source.reads(), 1);
    read_everything(&catalog);
    assert_eq!(source.reads(), 1, "every read is served from the catalog");
}

#[test]
fn reading_never_mutates_the_retained_catalog() {
    let (_, catalog) = fixture_catalog();
    let before = catalog
        .reader()
        .list(&query(sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS))
        .expect("before");
    let family_before = catalog.family().clone();
    read_everything(&catalog);
    let after = catalog
        .reader()
        .list(&query(sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS))
        .expect("after");
    assert_eq!(before, after);
    assert_eq!(family_before, *catalog.family());
}

#[test]
fn a_shared_reference_is_enough_to_read_everything() {
    // The boundary exposes no setter, no profile selection and no run start: a shared borrow is
    // enough to list, read and reconcile, which is what makes the slice read-only by construction.
    let (_, catalog) = fixture_catalog();
    let borrowed: &sts2_game_mod::RunResultCatalog = &catalog;
    read_everything(borrowed);
    let page = borrowed
        .reader()
        .list(&query(sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS))
        .expect("page");
    assert_eq!(page.total, 4);
}

#[test]
fn every_published_read_states_the_authority_it_withholds() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(catalog.authority(), RunResultReadAuthority::NotGranted);
    let reader = catalog.reader();
    assert_eq!(reader.authority(), RunResultReadAuthority::NotGranted);
    let mut reader = catalog.reader();
    let page = reader
        .list(&query(sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS))
        .expect("page");
    assert_eq!(page.authority, RunResultReadAuthority::NotGranted);
}

#[test]
fn a_read_failure_leaves_the_source_and_catalog_untouched() {
    let manifest = fixture_manifest();
    let source = RunResultSource::failing(RunResultSourceError::Malformed);
    assert!(produce_with(&manifest, &source).is_err());
    assert_eq!(source.reads(), 1);
    assert!(source.outcome.is_err());

    let (_, catalog) = fixture_catalog();
    let before = catalog.binding().clone();
    read_everything(&catalog);
    assert_eq!(catalog.binding(), &before, "reads retain no state");
}

#[test]
fn an_opaque_identity_is_path_free_or_it_is_not_an_identity() {
    assert!(is_opaque_identity("result.victory"));
    assert!(is_opaque_identity("run#1"));
    for refused in [
        "",
        "/",
        "..",
        "a/b",
        "C:\\saves",
        "profile..alpha",
        "line\nbreak",
        "caf\u{e9}",
    ] {
        assert!(!is_opaque_identity(refused), "{refused:?} is not opaque");
    }
}
