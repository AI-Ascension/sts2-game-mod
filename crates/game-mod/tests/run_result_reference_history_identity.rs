// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use manifest_fixture::manifest;
use sts2_game_mod::{RunResultError, RunResultVisibilityScope};
use support::*;

#[test]
fn a_reference_from_another_catalog_is_stale() {
    let (_, catalog) = fixture_catalog();
    let other_manifest = manifest(&[
        ("card", "card.strike"),
        ("card", "card.bash"),
        ("card", "card.defend"),
        ("relic", "relic.burning_blood"),
        ("potion", "potion.fire"),
        ("card", "card.cleave"),
    ]);
    let other = produce(&other_manifest, fixture_results(), fixture_summaries());
    assert_ne!(other.binding(), catalog.binding());
    assert_eq!(
        catalog
            .get(
                &result_reference(&other, "result.victory"),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("stale reference"),
        RunResultError::StaleReference
    );
    assert_eq!(
        catalog
            .get_summary(
                &summary_reference(&other, "summary.alpha.victory"),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("stale summary reference"),
        RunResultError::StaleReference
    );
}

#[test]
fn an_unknown_identity_is_not_found_rather_than_invented() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog
            .get(
                &result_reference(&catalog, "result.absent"),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("unknown result"),
        RunResultError::NotFound
    );
    assert_eq!(
        catalog
            .get_summary(
                &summary_reference(&catalog, "summary.absent"),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("unknown summary"),
        RunResultError::NotFound
    );
}
