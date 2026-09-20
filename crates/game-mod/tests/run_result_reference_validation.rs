// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_result_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/run_result_reference.rs"]
mod support;

use manifest_fixture::{
    component, fixture_manifest, manifest, no_score, quantity, reconciled_score,
};
use sts2_game_mod::{
    ResultFinalization, RunOutcome, RunResultCardEntry, RunResultError, RunResultFamilyState,
    RunResultFieldValue, RunResultPersistence, RunResultScore, RunResultVisibilityScope,
    RunStatistic, RunStatisticKind, ScoreAuthority, ScoreMode,
};
use support::*;

fn refuse(
    results: Vec<sts2_game_mod::RunResultInput>,
    summaries: Vec<sts2_game_mod::RunSummaryInput>,
) -> RunResultError {
    let manifest = fixture_manifest();
    produce_with(
        &manifest,
        &RunResultSource::answering(snapshot(&manifest, results, summaries)),
    )
    .expect_err("must be refused")
}

fn refuse_result(result: sts2_game_mod::RunResultInput) -> RunResultError {
    refuse(vec![result], Vec::new())
}

#[test]
fn a_snapshot_for_another_manifest_locale_or_producer_is_refused() {
    let manifest = fixture_manifest();
    let cases: [fn(&mut sts2_game_mod::RunResultCatalogSnapshot); 3] = [
        |declared| declared.manifest = other_of(),
        |declared| declared.locale = "de-DE".to_owned(),
        |declared| declared.producer_version = "game-run-result-reference-producer-v0".to_owned(),
    ];
    let expected = [
        RunResultError::ManifestMismatch,
        RunResultError::LocaleMismatch,
        RunResultError::ProducerVersionMismatch,
    ];
    for (case, expected) in cases.into_iter().zip(expected) {
        let mut declared = snapshot(&manifest, vec![victory_result()], Vec::new());
        case(&mut declared);
        let error = produce_with(&manifest, &RunResultSource::answering(declared))
            .expect_err("fence mismatch");
        assert_eq!(error, expected);
    }
}

/// Returns another manifest's binding, so a mismatched fence can be injected.
fn other_of() -> sts2_game_mod::ContentCursorBinding {
    manifest(&[("card", "card.strike")]).cursor_binding()
}

#[test]
fn declared_coverage_must_match_the_records_the_source_reports() {
    let manifest = fixture_manifest();
    let mut declared = snapshot(&manifest, vec![victory_result()], Vec::new());
    declared.family.result_count = 2;
    assert_eq!(
        produce_with(&manifest, &RunResultSource::answering(declared)).expect_err("count mismatch"),
        RunResultError::FamilyCountMismatch
    );
}

#[test]
fn a_record_published_while_the_family_is_unavailable_is_refused() {
    let manifest = fixture_manifest();
    let mut declared = snapshot(&manifest, vec![victory_result()], Vec::new());
    declared.family.state = RunResultFamilyState::Unavailable;
    assert_eq!(
        produce_with(&manifest, &RunResultSource::answering(declared))
            .expect_err("unavailable family"),
        RunResultError::UnknownResult("result.victory".to_owned())
    );
}

#[test]
fn duplicate_identities_and_duplicate_runs_are_refused() {
    assert_eq!(
        refuse_result_pair(victory_result(), victory_result()),
        RunResultError::DuplicateResult("result.victory".to_owned())
    );
    let mut same_run = defeat_result();
    same_run.result_id = "result.defeat.two".to_owned();
    same_run.run_id = "run.alpha.1".to_owned();
    assert_eq!(
        refuse_result_pair(victory_result(), same_run),
        RunResultError::DuplicateRun("run.alpha.1".to_owned())
    );
    let mut other_profile = defeat_result();
    other_profile.result_id = "result.defeat.two".to_owned();
    other_profile.run_id = "run.alpha.1".to_owned();
    other_profile.profile_id = "profile.beta".to_owned();
    assert_eq!(
        refuse_result_pair(victory_result(), other_profile),
        RunResultError::ProfileIsolationViolation("run.alpha.1".to_owned())
    );

    let mut repeated = summary_without_detail();
    repeated.summary_id = "summary.alpha.victory".to_owned();
    assert_eq!(
        refuse(
            vec![victory_result()],
            vec![summary_victory(), repeated.clone()]
        ),
        RunResultError::DuplicateSummary("summary.alpha.victory".to_owned())
    );
    let mut same_summary_run = summary_without_detail();
    same_summary_run.run_id = "run.alpha.1".to_owned();
    same_summary_run.profile_id = "profile.alpha".to_owned();
    assert_eq!(
        refuse(
            vec![victory_result()],
            vec![summary_victory(), same_summary_run]
        ),
        RunResultError::DuplicateRun("run.alpha.1".to_owned())
    );
}

fn refuse_result_pair(
    first: sts2_game_mod::RunResultInput,
    second: sts2_game_mod::RunResultInput,
) -> RunResultError {
    refuse(vec![first, second], Vec::new())
}

#[test]
fn a_summary_detail_must_exist_and_must_be_finalized() {
    let mut absent = summary_without_detail();
    absent.detail_result_id = RunResultFieldValue::present("result.absent".to_owned());
    assert_eq!(
        refuse(vec![victory_result()], vec![absent]),
        RunResultError::MissingResultDetail("result.absent".to_owned())
    );
    let mut pending = summary_owner_only();
    pending.detail_result_id = RunResultFieldValue::present("result.pending".to_owned());
    assert_eq!(
        refuse(vec![pending_result()], vec![pending]),
        RunResultError::PendingDetailPublished
    );
}

#[test]
fn finalization_persistence_and_outcome_must_agree() {
    let mut unconfirmed = victory_result();
    unconfirmed.persistence = RunResultPersistence::Unknown;
    assert_eq!(
        refuse_result(unconfirmed),
        RunResultError::UnconfirmedFinalization
    );

    let mut unfinished = victory_result();
    unfinished.outcome = RunOutcome::Unfinished;
    unfinished.score = no_score();
    assert_eq!(
        refuse_result(unfinished),
        RunResultError::UnfinishedResultPublishedAsFinalized
    );

    let mut scored_unfinished = victory_result();
    scored_unfinished.outcome = RunOutcome::Unfinished;
    scored_unfinished.finalization = ResultFinalization::PendingPersistence;
    scored_unfinished.persistence = RunResultPersistence::Unknown;
    assert_eq!(
        refuse_result(scored_unfinished),
        RunResultError::ScoreForUnfinishedRun
    );
}

#[test]
fn a_componentized_terminal_result_must_report_a_reconciled_total() {
    let mut no_total = victory_result();
    no_total.score.total = RunResultFieldValue::absent();
    no_total.score.components = RunResultFieldValue::absent();
    assert_eq!(refuse_result(no_total), RunResultError::OutcomeWithoutScore);

    let mut mismatched = victory_result();
    mismatched.score.total = RunResultFieldValue::present(quantity("points", 99));
    assert_eq!(
        refuse_result(mismatched),
        RunResultError::ScoreTotalMismatch {
            displayed: 99,
            summed: 75,
        }
    );

    let mut unreconciled = victory_result();
    unreconciled.score.components = RunResultFieldValue::absent();
    assert_eq!(
        refuse_result(unreconciled),
        RunResultError::UnreconciledScore
    );

    let mut empty = victory_result();
    empty.score.components = RunResultFieldValue::present(Vec::new());
    assert_eq!(
        refuse_result(empty),
        RunResultError::EmptyPresentCollection("score_components")
    );
}

#[test]
fn score_authority_must_be_honest_about_its_own_components() {
    let mut mismatched = victory_result();
    mismatched.score = reconciled_score(vec![component(
        "score.floor",
        "Floors",
        75,
        ScoreAuthority::SyntheticMetric,
    )]);
    assert_eq!(
        refuse_result(mismatched),
        RunResultError::ComponentAuthorityMismatch("score.floor".to_owned())
    );

    let mut duplicated = victory_result();
    duplicated.score = reconciled_score(vec![
        component("score.floor", "Floors", 30, ScoreAuthority::HostGameScore),
        component("score.floor", "Floors", 45, ScoreAuthority::HostGameScore),
    ]);
    assert_eq!(
        refuse_result(duplicated),
        RunResultError::DuplicateScoreComponent("score.floor".to_owned())
    );

    let mut conflicted = victory_result();
    conflicted.score = RunResultScore {
        authority: ScoreAuthority::NoScore,
        mode: ScoreMode::Componentized,
        total: RunResultFieldValue::absent(),
        components: RunResultFieldValue::absent(),
    };
    assert_eq!(refuse_result(conflicted), RunResultError::ScoreModeConflict);
}

#[test]
fn ending_entries_must_be_unique_non_zero_and_resolve_in_the_manifest() {
    let mut duplicated = victory_result();
    duplicated.ending_deck = RunResultFieldValue::present(vec![
        RunResultCardEntry {
            card_id: "card.strike".to_owned(),
            count: 1,
            upgraded: false,
        },
        RunResultCardEntry {
            card_id: "card.strike".to_owned(),
            count: 2,
            upgraded: true,
        },
    ]);
    assert_eq!(
        refuse_result(duplicated),
        RunResultError::DuplicateEndingEntry("card.strike".to_owned())
    );

    let mut zero = victory_result();
    zero.ending_deck = RunResultFieldValue::present(vec![RunResultCardEntry {
        card_id: "card.strike".to_owned(),
        count: 0,
        upgraded: false,
    }]);
    assert_eq!(
        refuse_result(zero),
        RunResultError::ZeroCountEntry("card.strike".to_owned())
    );

    let mut unknown = victory_result();
    unknown.ending_inventory =
        RunResultFieldValue::present(vec![sts2_game_mod::RunResultItemEntry {
            item_id: "relic.ghost".to_owned(),
            kind: sts2_game_mod::RunResultItemKind::Relic,
            count: 1,
        }]);
    assert_eq!(
        refuse_result(unknown),
        RunResultError::UnknownManifestReference {
            entity_kind: "relic".to_owned(),
            namespaced_id: "relic.ghost".to_owned(),
        }
    );

    let mut empty = victory_result();
    empty.ending_deck = RunResultFieldValue::present(Vec::new());
    assert_eq!(
        refuse_result(empty),
        RunResultError::EmptyPresentCollection("ending_deck")
    );
}

#[test]
fn statistics_must_be_unique_and_bounded() {
    let mut duplicated = victory_result();
    duplicated.statistics = RunResultFieldValue::present(vec![
        RunStatistic {
            statistic_id: "stat.turns".to_owned(),
            kind: RunStatisticKind::CombatTurns,
            label: RunResultFieldValue::present("Turns".to_owned()),
            value: RunResultFieldValue::present(quantity("turns", 24)),
        },
        RunStatistic {
            statistic_id: "stat.turns".to_owned(),
            kind: RunStatisticKind::Other,
            label: RunResultFieldValue::present("Turns".to_owned()),
            value: RunResultFieldValue::present(quantity("turns", 25)),
        },
    ]);
    assert_eq!(
        refuse_result(duplicated),
        RunResultError::DuplicateStatistic("stat.turns".to_owned())
    );

    let mut control = victory_result();
    control.statistics = RunResultFieldValue::present(vec![RunStatistic {
        statistic_id: "stat.turns".to_owned(),
        kind: RunStatisticKind::CombatTurns,
        label: RunResultFieldValue::present("Tu\u{1}rns".to_owned()),
        value: RunResultFieldValue::present(quantity("turns", 24)),
    }]);
    assert_eq!(
        refuse_result(control),
        RunResultError::InvalidInput("statistic_label")
    );
}

#[test]
fn a_path_shaped_identity_is_refused_rather_than_sanitized() {
    for identity in [
        "C:\\saves\\profile.alpha",
        "profiles/alpha",
        "..\\..\\alpha",
    ] {
        let mut result = victory_result();
        result.profile_id = identity.to_owned();
        assert_eq!(
            refuse_result(result),
            RunResultError::NonOpaqueIdentity("profile_id")
        );
    }
}

#[test]
fn an_aggregate_record_bound_is_enforced() {
    let mut oversized = victory_result();
    let label = "a".repeat(16 * 1024);
    oversized.score = reconciled_score(
        (0..9)
            .map(|index| {
                component(
                    &format!("score.component{index}"),
                    &label,
                    1,
                    ScoreAuthority::HostGameScore,
                )
            })
            .collect(),
    );
    let error = refuse_result(oversized);
    assert!(
        matches!(error, RunResultError::ResultTooLarge { .. }),
        "an oversized record is refused rather than truncated: {error:?}"
    );
}

#[test]
fn more_records_than_the_local_bound_are_refused() {
    let results = (0..=sts2_game_mod::RUN_RESULT_MAX_RESULTS)
        .map(|index| {
            let mut result = victory_result();
            result.result_id = format!("result.{index}");
            result.run_id = format!("run.{index}");
            result
        })
        .collect::<Vec<_>>();
    assert_eq!(
        refuse(results, Vec::new()),
        RunResultError::InvalidInput("records")
    );
}

#[test]
fn a_valid_snapshot_still_produces_the_shared_catalog() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(catalog.family().result_count, 5);
    assert_eq!(
        catalog
            .get(
                &sts2_game_mod::RunResultReference {
                    catalog: catalog.binding().clone(),
                    result_id: "result.abandonment".to_owned(),
                },
                RunResultVisibilityScope::Owner,
            )
            .expect("abandonment")
            .result
            .outcome,
        RunOutcome::Abandonment
    );
}
