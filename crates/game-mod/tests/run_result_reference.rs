// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{
    ResultFinalization, RunOutcome, RunResultDetailState, RunResultFamilyState,
    RunResultFieldStatus, RunResultPersistence, RunResultVisibilityScope, ScoreAuthority,
    ScoreMode,
};
use support::*;

fn result(
    catalog: &sts2_game_mod::RunResultCatalog,
    result_id: &str,
) -> sts2_game_mod::RunResultRecord {
    catalog
        .get(
            &result_reference(catalog, result_id),
            RunResultVisibilityScope::Owner,
        )
        .expect("result")
}

#[test]
fn producing_binds_every_record_to_one_manifest_and_locale() {
    let (manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        sts2_game_mod::RUN_RESULT_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().state, RunResultFamilyState::Handled);
    assert_eq!(catalog.family().result_count, 5);
    assert_eq!(catalog.family().summary_count, 5);
}

#[test]
fn victory_defeat_abandonment_and_no_score_stay_distinct() {
    let (_, catalog) = fixture_catalog();
    let victory = result(&catalog, "result.victory");
    let defeat = result(&catalog, "result.defeat");
    let abandonment = result(&catalog, "result.abandonment");

    assert_eq!(victory.result.outcome, RunOutcome::Victory);
    assert_eq!(defeat.result.outcome, RunOutcome::Defeat);
    assert_eq!(abandonment.result.outcome, RunOutcome::Abandonment);
    assert_ne!(victory.result.outcome, RunOutcome::Unfinished);

    assert_eq!(victory.result.score.mode, ScoreMode::Componentized);
    assert_eq!(defeat.result.score.mode, ScoreMode::TotalOnly);
    assert_eq!(abandonment.result.score.mode, ScoreMode::Unsupported);
    assert_eq!(abandonment.result.score.authority, ScoreAuthority::NoScore);
    assert!(!abandonment.result.score.total.is_present());
    assert!(!abandonment.result.score.components.is_present());
}

#[test]
fn score_components_reconcile_with_the_displayed_total() {
    let (_, catalog) = fixture_catalog();
    let victory = result(&catalog, "result.victory");
    let components = victory.result.score.components.value().expect("components");
    let summed: i64 = components
        .iter()
        .map(|component| component.contribution.amount)
        .sum();
    assert_eq!(summed, 75);
    assert_eq!(
        victory.result.score.total.value().expect("total").amount,
        summed
    );
    assert_eq!(components.len(), 2);
    assert!(
        components
            .iter()
            .all(|component| component.authority == ScoreAuthority::HostGameScore)
    );
}

#[test]
fn a_reported_result_carries_its_own_character_configuration_and_position() {
    let (_, catalog) = fixture_catalog();
    let victory = result(&catalog, "result.victory");
    assert_eq!(
        victory.result.character_id.value().map(String::as_str),
        Some("character.ironclad")
    );
    assert_eq!(
        victory.result.configuration_id.value().map(String::as_str),
        Some("config.standard")
    );
    assert_eq!(victory.result.act.value(), Some(&3));
    assert_eq!(victory.result.floor.value(), Some(&51));
    assert_eq!(
        victory.result.duration.value().map(|value| value.seconds),
        Some(2_520)
    );
    assert_eq!(
        victory.result.ending_deck.value().map(Vec::len),
        Some(2),
        "the ending deck keeps every retained entry"
    );
    assert_eq!(
        victory.result.ending_inventory.value().map(Vec::len),
        Some(2)
    );
    assert_eq!(victory.result.statistics.value().map(Vec::len), Some(1));
}

#[test]
fn an_absent_or_unsupported_field_states_itself_instead_of_becoming_zero() {
    let (_, catalog) = fixture_catalog();
    let defeat = result(&catalog, "result.defeat");
    assert_eq!(
        defeat.result.configuration_id.status(),
        RunResultFieldStatus::Absent
    );
    assert!(defeat.result.configuration_id.value().is_none());
    assert!(defeat.result.ending_deck.value().is_none());

    let abandonment = result(&catalog, "result.abandonment");
    assert_eq!(
        abandonment.result.act.status(),
        RunResultFieldStatus::Unsupported
    );
    assert!(abandonment.result.act.value().is_none());
    assert_eq!(
        abandonment.result.floor.value(),
        Some(&7),
        "a floor the host did report keeps its value"
    );
}

#[test]
fn a_terminal_presentation_is_not_a_persisted_result() {
    let (_, catalog) = fixture_catalog();
    let pending = result(&catalog, "result.pending");
    assert_eq!(pending.result.outcome, RunOutcome::Victory);
    assert_eq!(
        pending.result.finalization,
        ResultFinalization::PendingPersistence
    );
    assert_eq!(pending.result.persistence, RunResultPersistence::Unknown);
    assert_ne!(pending.result.finalization, ResultFinalization::Finalized);
    assert_eq!(pending.detail_state(), RunResultDetailState::Unavailable);

    let victory = result(&catalog, "result.victory");
    assert_eq!(victory.detail_state(), RunResultDetailState::Available);
}

#[test]
fn a_family_that_reports_no_results_produces_an_explicitly_empty_catalog() {
    let manifest = fixture_manifest();
    let mut declared = snapshot(&manifest, Vec::new(), Vec::new());
    declared.family.state = RunResultFamilyState::Unavailable;
    let catalog = produce_with(&manifest, &RunResultSource::answering(declared.clone()))
        .expect("empty catalog");
    assert_eq!(catalog.family().state, RunResultFamilyState::Unavailable);
    assert_eq!(catalog.family().result_count, 0);
    assert!(declared.results.is_empty() && declared.summaries.is_empty());
    assert_eq!(
        catalog
            .get(
                &result_reference(&catalog, "result.victory"),
                RunResultVisibilityScope::Owner,
            )
            .expect_err("unavailable family"),
        sts2_game_mod::RunResultError::UnavailableFamily
    );
}

#[test]
fn a_source_failure_maps_to_one_sanitized_error() {
    let manifest = fixture_manifest();
    for (failure, expected) in [
        (
            sts2_game_mod::RunResultSourceError::NoActiveSource,
            sts2_game_mod::RunResultError::NoActiveSource,
        ),
        (
            sts2_game_mod::RunResultSourceError::AccessDenied,
            sts2_game_mod::RunResultError::SourceAccessDenied,
        ),
        (
            sts2_game_mod::RunResultSourceError::Malformed,
            sts2_game_mod::RunResultError::MalformedSource,
        ),
    ] {
        let source = RunResultSource::failing(failure);
        let error = produce_with(&manifest, &source).expect_err("must fail closed");
        assert_eq!(error, expected);
        assert_eq!(source.reads(), 1);
    }
}

#[test]
fn an_imported_or_harness_record_never_claims_the_host_score() {
    let manifest = fixture_manifest();
    let mut imported = victory_result();
    imported.origin = sts2_game_mod::ResultOrigin::Harness;
    let error = produce_with(
        &manifest,
        &RunResultSource::answering(snapshot(&manifest, vec![imported], Vec::new())),
    )
    .expect_err("harness origin cannot claim the host score");
    assert_eq!(
        error,
        sts2_game_mod::RunResultError::EvaluatorScoreAsHostScore
    );
}
