// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use sts2_game_mod::{
    ResultFinalization, RunResultError, RunResultLiveFence, RunResultVisibilityScope,
};
use support::*;

fn fence(instance_id: &str, run_id: &str, epoch: u64) -> RunResultLiveFence {
    RunResultLiveFence {
        instance_id: instance_id.to_owned(),
        run_id: run_id.to_owned(),
        epoch,
    }
}

#[test]
fn a_current_result_read_requires_a_live_fence() {
    let (_, catalog) = fixture_catalog();
    for incomplete in [fence("", "run.alpha.1", 4), fence("instance.alpha", "", 4)] {
        assert_eq!(
            catalog
                .current(&incomplete, RunResultVisibilityScope::Owner)
                .expect_err("incomplete fence"),
            RunResultError::MissingLiveFence
        );
    }
}

#[test]
fn a_current_result_read_answers_the_fenced_run() {
    let (_, catalog) = fixture_catalog();
    let record = catalog
        .current(
            &fence("instance.alpha", "run.alpha.1", 4),
            RunResultVisibilityScope::Owner,
        )
        .expect("current result");
    assert_eq!(record.result_id(), "result.victory");
    assert_eq!(record.result.run_id, "run.alpha.1");
    assert_eq!(record.result.finalization, ResultFinalization::Finalized);
}

#[test]
fn a_fence_that_names_no_carried_run_is_stale() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog
            .current(
                &fence("instance.alpha", "run.alpha.99", 4),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("stale fence"),
        RunResultError::StaleLiveFence
    );
}

#[test]
fn a_fence_that_carries_a_path_is_refused_rather_than_answered() {
    let (_, catalog) = fixture_catalog();
    let path_like = fence("C:\\saves\\profile", "run.alpha.1", 4);
    assert_eq!(
        catalog
            .current(&path_like, RunResultVisibilityScope::Owner)
            .expect_err("path-shaped fence"),
        RunResultError::NonOpaqueIdentity("live_fence")
    );
}

#[test]
fn an_owner_only_current_result_is_not_returned_to_an_anonymous_read() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog
            .current(
                &fence("instance.alpha", "run.alpha.3", 9),
                RunResultVisibilityScope::Anonymous,
            )
            .expect_err("owner-only"),
        RunResultError::ExcludedByScope
    );
    let owner = catalog
        .current(
            &fence("instance.alpha", "run.alpha.3", 9),
            RunResultVisibilityScope::Owner,
        )
        .expect("owner observes its own result");
    assert_eq!(owner.result_id(), "result.pending");
    assert_eq!(
        owner.detail_state(),
        sts2_game_mod::RunResultDetailState::Unavailable
    );
}

#[test]
fn a_withheld_current_result_is_observable_in_no_scope() {
    let (_, catalog) = fixture_catalog();
    for scope in [
        RunResultVisibilityScope::Owner,
        RunResultVisibilityScope::Anonymous,
    ] {
        assert_eq!(
            catalog
                .current(&fence("instance.gamma", "run.gamma.1", 2), scope)
                .expect_err("hidden"),
            RunResultError::ExcludedByScope
        );
    }
}
