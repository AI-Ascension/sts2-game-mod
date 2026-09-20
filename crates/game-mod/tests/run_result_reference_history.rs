// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use sts2_game_mod::{
    ResultOrigin, RunResultDetailQuery, RunResultDetailState, RunResultError, RunResultFieldValue,
    RunResultLiveFence, RunResultVisibilityScope, RunSummaryContinuation, RunSummaryListQuery,
};
use support::*;

const PAGE: usize = sts2_game_mod::RUN_RESULT_MAX_PAGE_ITEMS;

fn profile(name: &str) -> RunResultFieldValue<String> {
    RunResultFieldValue::present(name.to_owned())
}

fn query(
    scope: RunResultVisibilityScope,
    owner: RunResultFieldValue<String>,
    limit: usize,
    continuation: Option<RunSummaryContinuation>,
) -> RunSummaryListQuery {
    RunSummaryListQuery {
        locale: "en-US".to_owned(),
        profile_id: owner,
        scope,
        limit,
        continuation,
        live_fence: None,
    }
}

fn detail_query(
    catalog: &sts2_game_mod::RunResultCatalog,
    summary_id: &str,
    owner: RunResultFieldValue<String>,
) -> RunResultDetailQuery {
    RunResultDetailQuery {
        summary: summary_reference(catalog, summary_id),
        profile_id: owner,
        scope: RunResultVisibilityScope::Owner,
        live_fence: None,
    }
}

#[test]
fn prior_summaries_page_with_single_use_continuations() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&query(
            RunResultVisibilityScope::Owner,
            RunResultFieldValue::absent(),
            2,
            None,
        ))
        .expect("first page");
    assert_eq!(first.total, 4);
    assert!(!first.complete);
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.summary_id.as_str())
            .collect::<Vec<_>>(),
        ["summary.alpha.defeat", "summary.alpha.pending"]
    );
    let continuation = first.continuation.expect("continuation");
    assert_eq!(continuation.token(), "summary-cursor-00000000");

    let second = reader
        .list(&query(
            RunResultVisibilityScope::Owner,
            RunResultFieldValue::absent(),
            2,
            Some(continuation.clone()),
        ))
        .expect("second page");
    assert_eq!(second.total, 4);
    assert!(second.complete);
    assert!(second.continuation.is_none());
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.reference.summary_id.as_str())
            .collect::<Vec<_>>(),
        ["summary.alpha.victory", "summary.beta.abandon"]
    );

    assert_eq!(
        reader
            .list(&query(
                RunResultVisibilityScope::Owner,
                RunResultFieldValue::absent(),
                2,
                Some(continuation),
            ))
            .expect_err("a consumed continuation is refused"),
        RunResultError::InvalidContinuation
    );
}

#[test]
fn a_continuation_is_bound_to_the_query_that_minted_it() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&query(
            RunResultVisibilityScope::Owner,
            profile("profile.alpha"),
            2,
            None,
        ))
        .expect("first page");
    let continuation = first.continuation.expect("continuation");
    assert_eq!(
        reader
            .list(&query(
                RunResultVisibilityScope::Owner,
                profile("profile.beta"),
                2,
                Some(continuation.clone()),
            ))
            .expect_err("another profile"),
        RunResultError::InvalidContinuation
    );
    assert_eq!(
        reader
            .list(&query(
                RunResultVisibilityScope::Owner,
                profile("profile.alpha"),
                3,
                Some(continuation.clone()),
            ))
            .expect_err("another limit"),
        RunResultError::InvalidContinuation
    );
    let mut other = catalog.reader();
    assert_eq!(
        other
            .list(&query(
                RunResultVisibilityScope::Owner,
                profile("profile.alpha"),
                2,
                Some(continuation),
            ))
            .expect_err("another reader"),
        RunResultError::InvalidContinuation
    );
}

#[test]
fn a_bounded_page_refuses_a_size_it_cannot_serve() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    for limit in [0, PAGE + 1] {
        assert_eq!(
            reader
                .list(&query(
                    RunResultVisibilityScope::Owner,
                    RunResultFieldValue::absent(),
                    limit,
                    None,
                ))
                .expect_err("invalid page size"),
            RunResultError::InvalidPageSize
        );
    }
    let mut wrong_locale = query(
        RunResultVisibilityScope::Owner,
        RunResultFieldValue::absent(),
        4,
        None,
    );
    wrong_locale.locale = "de-DE".to_owned();
    assert_eq!(
        reader.list(&wrong_locale).expect_err("locale"),
        RunResultError::LocaleMismatch
    );
}

#[test]
fn a_history_read_carries_no_live_fence() {
    let (_, catalog) = fixture_catalog();
    let fence = RunResultLiveFence {
        instance_id: "instance.alpha".to_owned(),
        run_id: "run.alpha.1".to_owned(),
        epoch: 3,
    };
    let mut fenced = query(
        RunResultVisibilityScope::Owner,
        RunResultFieldValue::absent(),
        4,
        None,
    );
    fenced.live_fence = Some(fence.clone());
    assert_eq!(
        catalog
            .reader()
            .list(&fenced)
            .expect_err("a list holds no live fence"),
        RunResultError::UnexpectedLiveFence
    );

    let mut detail = detail_query(
        &catalog,
        "summary.alpha.victory",
        RunResultFieldValue::absent(),
    );
    detail.live_fence = Some(fence);
    assert_eq!(
        catalog
            .reader()
            .detail(&detail)
            .expect_err("a detail read holds no live fence"),
        RunResultError::UnexpectedLiveFence
    );
}

#[test]
fn a_detail_read_resolves_the_finalized_result_it_names() {
    let (_, catalog) = fixture_catalog();
    let detail = catalog
        .reader()
        .detail(&detail_query(
            &catalog,
            "summary.alpha.victory",
            RunResultFieldValue::absent(),
        ))
        .expect("detail");
    assert_eq!(detail.result_id(), "result.victory");
    assert_eq!(detail.result.outcome, sts2_game_mod::RunOutcome::Victory);

    let summary = catalog
        .get_summary(
            &summary_reference(&catalog, "summary.alpha.victory"),
            RunResultVisibilityScope::Owner,
        )
        .expect("summary");
    assert_eq!(summary.detail_state(), RunResultDetailState::Available);
}

#[test]
fn a_summary_without_a_retained_detail_stays_unavailable_rather_than_reconstructed() {
    let (_, catalog) = fixture_catalog();
    let summary = catalog
        .get_summary(
            &summary_reference(&catalog, "summary.beta.abandon"),
            RunResultVisibilityScope::Owner,
        )
        .expect("summary");
    assert_eq!(summary.detail_state(), RunResultDetailState::Unavailable);
    assert!(!summary.summary.detail_result_id.is_present());
    assert_eq!(
        catalog
            .reader()
            .detail(&detail_query(
                &catalog,
                "summary.beta.abandon",
                RunResultFieldValue::absent(),
            ))
            .expect_err("no detail to reconstruct"),
        RunResultError::DetailUnavailable("summary.beta.abandon".to_owned())
    );
}

#[test]
fn one_run_identity_belongs_to_one_profile_only() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog
            .reader()
            .detail(&detail_query(
                &catalog,
                "summary.beta.abandon",
                profile("profile.alpha"),
            ))
            .expect_err("another profile's summary"),
        RunResultError::ProfileNotObserved
    );
    let mut anonymous = query(
        RunResultVisibilityScope::Anonymous,
        profile("profile.alpha"),
        4,
        None,
    );
    anonymous.profile_id = profile("profile.alpha");
    assert_eq!(
        catalog
            .reader()
            .list(&anonymous)
            .expect_err("an anonymous read names no owner"),
        RunResultError::ProfileNotObserved
    );
    let own = catalog
        .reader()
        .list(&query(
            RunResultVisibilityScope::Owner,
            profile("profile.alpha"),
            PAGE,
            None,
        ))
        .expect("own history");
    assert!(
        own.entries
            .iter()
            .all(|entry| entry.profile_id == "profile.alpha")
    );
    assert_eq!(own.total, 3);
}

#[test]
fn a_withheld_or_owner_only_summary_is_never_listed_outside_its_scope() {
    let (_, catalog) = fixture_catalog();
    let anonymous = catalog
        .reader()
        .list(&query(
            RunResultVisibilityScope::Anonymous,
            RunResultFieldValue::absent(),
            PAGE,
            None,
        ))
        .expect("anonymous history");
    let ids = anonymous
        .entries
        .iter()
        .map(|entry| entry.reference.summary_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(anonymous.total, 3);
    assert!(!ids.contains(&"summary.alpha.pending"), "owner-only");
    assert!(!ids.contains(&"summary.gamma.hidden"), "hidden");
}

#[test]
fn origin_and_source_linkage_survive_the_read() {
    let (_, catalog) = fixture_catalog();
    let summary = catalog
        .get_summary(
            &summary_reference(&catalog, "summary.alpha.victory"),
            RunResultVisibilityScope::Owner,
        )
        .expect("summary");
    assert_eq!(summary.summary.origin, ResultOrigin::Native);
    assert_eq!(summary.summary.run_id, "run.alpha.1");

    let detail = catalog
        .reader()
        .detail(&detail_query(
            &catalog,
            "summary.alpha.victory",
            RunResultFieldValue::absent(),
        ))
        .expect("detail");
    let linkage = detail.result.linkage.value().expect("linkage");
    assert_eq!(linkage.trajectory_id, "trajectory.alpha.1");
    assert_eq!(linkage.trajectory_origin, ResultOrigin::Harness);
    assert_eq!(
        linkage.evidence_id.value().map(String::as_str),
        Some("evidence.alpha.1")
    );
}
