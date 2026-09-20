// SPDX-License-Identifier: MIT

//! The five completed-run results, five prior summaries and the source that answers with them.

use std::cell::Cell;

use sts2_game_mod::{
    ContentManifest, RUN_RESULT_REFERENCE_PRODUCER_VERSION, ResultFinalization, ResultOrigin,
    RunOutcome, RunResultCardEntry, RunResultCatalog, RunResultCatalogProducer,
    RunResultCatalogSnapshot, RunResultCatalogSource, RunResultFamilyCoverage,
    RunResultFamilyState, RunResultFieldValue, RunResultInput, RunResultItemEntry,
    RunResultItemKind, RunResultLinkage, RunResultPersistence, RunResultQuantity,
    RunResultReference, RunResultSourceError, RunResultVisibility, RunSummaryInput,
    RunSummaryReference, ScoreAuthority,
};

use crate::manifest_fixture::{
    component, duration, fixture_manifest, no_score, quantity, reconciled_score, statistics,
    total_only_score,
};

fn ending_deck() -> RunResultFieldValue<Vec<RunResultCardEntry>> {
    RunResultFieldValue::present(vec![
        RunResultCardEntry {
            card_id: "card.strike".to_owned(),
            count: 4,
            upgraded: true,
        },
        RunResultCardEntry {
            card_id: "card.bash".to_owned(),
            count: 1,
            upgraded: false,
        },
    ])
}

fn ending_inventory() -> RunResultFieldValue<Vec<RunResultItemEntry>> {
    RunResultFieldValue::present(vec![
        RunResultItemEntry {
            item_id: "relic.burning_blood".to_owned(),
            kind: RunResultItemKind::Relic,
            count: 1,
        },
        RunResultItemEntry {
            item_id: "potion.fire".to_owned(),
            kind: RunResultItemKind::Potion,
            count: 2,
        },
    ])
}

fn linkage(
    trajectory_id: &str,
    evidence: RunResultFieldValue<String>,
) -> RunResultFieldValue<RunResultLinkage> {
    RunResultFieldValue::present(RunResultLinkage {
        trajectory_id: trajectory_id.to_owned(),
        trajectory_origin: ResultOrigin::Harness,
        evidence_id: evidence,
    })
}

/// A finalized victory whose components reconcile with its displayed total.
pub fn victory_result() -> RunResultInput {
    RunResultInput {
        result_id: "result.victory".to_owned(),
        profile_id: "profile.alpha".to_owned(),
        run_id: "run.alpha.1".to_owned(),
        outcome: RunOutcome::Victory,
        finalization: ResultFinalization::Finalized,
        persistence: RunResultPersistence::Confirmed,
        character_id: RunResultFieldValue::present("character.ironclad".to_owned()),
        configuration_id: RunResultFieldValue::present("config.standard".to_owned()),
        act: RunResultFieldValue::present(3),
        floor: RunResultFieldValue::present(51),
        duration: duration(2_520),
        ending_deck: ending_deck(),
        ending_inventory: ending_inventory(),
        score: reconciled_score(vec![
            component("score.floor", "Floors", 30, ScoreAuthority::HostGameScore),
            component(
                "score.enemies",
                "Enemies",
                45,
                ScoreAuthority::HostGameScore,
            ),
        ]),
        statistics: statistics(),
        linkage: linkage(
            "trajectory.alpha.1",
            RunResultFieldValue::present("evidence.alpha.1".to_owned()),
        ),
        origin: ResultOrigin::Native,
        visibility: RunResultVisibility::Public,
    }
}

/// A finalized defeat reported in a mode that carries no components.
pub fn defeat_result() -> RunResultInput {
    RunResultInput {
        result_id: "result.defeat".to_owned(),
        profile_id: "profile.alpha".to_owned(),
        run_id: "run.alpha.2".to_owned(),
        outcome: RunOutcome::Defeat,
        finalization: ResultFinalization::Finalized,
        persistence: RunResultPersistence::Confirmed,
        character_id: RunResultFieldValue::present("character.silent".to_owned()),
        configuration_id: RunResultFieldValue::absent(),
        act: RunResultFieldValue::present(2),
        floor: RunResultFieldValue::present(33),
        duration: duration(1_200),
        ending_deck: RunResultFieldValue::absent(),
        ending_inventory: RunResultFieldValue::absent(),
        score: total_only_score(12),
        statistics: RunResultFieldValue::absent(),
        linkage: linkage("trajectory.alpha.2", RunResultFieldValue::absent()),
        origin: ResultOrigin::Native,
        visibility: RunResultVisibility::Public,
    }
}

/// A finalized abandonment whose mode reports no score at all.
pub fn abandonment_result() -> RunResultInput {
    RunResultInput {
        result_id: "result.abandonment".to_owned(),
        profile_id: "profile.beta".to_owned(),
        run_id: "run.beta.1".to_owned(),
        outcome: RunOutcome::Abandonment,
        finalization: ResultFinalization::Finalized,
        persistence: RunResultPersistence::Confirmed,
        character_id: RunResultFieldValue::absent(),
        configuration_id: RunResultFieldValue::absent(),
        act: RunResultFieldValue::unsupported(),
        floor: RunResultFieldValue::present(7),
        duration: RunResultFieldValue::absent(),
        ending_deck: RunResultFieldValue::absent(),
        ending_inventory: RunResultFieldValue::absent(),
        score: no_score(),
        statistics: RunResultFieldValue::absent(),
        linkage: RunResultFieldValue::absent(),
        origin: ResultOrigin::Native,
        visibility: RunResultVisibility::Public,
    }
}

/// A terminal outcome observed before the host confirms it persisted anything.
pub fn pending_result() -> RunResultInput {
    RunResultInput {
        result_id: "result.pending".to_owned(),
        profile_id: "profile.alpha".to_owned(),
        run_id: "run.alpha.3".to_owned(),
        outcome: RunOutcome::Victory,
        finalization: ResultFinalization::PendingPersistence,
        persistence: RunResultPersistence::Unknown,
        character_id: RunResultFieldValue::present("character.ironclad".to_owned()),
        configuration_id: RunResultFieldValue::absent(),
        act: RunResultFieldValue::present(1),
        floor: RunResultFieldValue::present(9),
        duration: RunResultFieldValue::absent(),
        ending_deck: RunResultFieldValue::absent(),
        ending_inventory: RunResultFieldValue::absent(),
        score: reconciled_score(vec![component(
            "score.floor",
            "Floors",
            9,
            ScoreAuthority::HostGameScore,
        )]),
        statistics: RunResultFieldValue::absent(),
        linkage: RunResultFieldValue::absent(),
        origin: ResultOrigin::Native,
        visibility: RunResultVisibility::OwnerOnly,
    }
}

/// A withheld result that no visibility scope may observe.
pub fn hidden_result() -> RunResultInput {
    RunResultInput {
        result_id: "result.hidden".to_owned(),
        profile_id: "profile.gamma".to_owned(),
        run_id: "run.gamma.1".to_owned(),
        outcome: RunOutcome::Victory,
        finalization: ResultFinalization::Finalized,
        persistence: RunResultPersistence::Confirmed,
        character_id: RunResultFieldValue::absent(),
        configuration_id: RunResultFieldValue::absent(),
        act: RunResultFieldValue::absent(),
        floor: RunResultFieldValue::absent(),
        duration: RunResultFieldValue::absent(),
        ending_deck: RunResultFieldValue::absent(),
        ending_inventory: RunResultFieldValue::absent(),
        score: total_only_score(5),
        statistics: RunResultFieldValue::absent(),
        linkage: RunResultFieldValue::absent(),
        origin: ResultOrigin::Native,
        visibility: RunResultVisibility::Hidden,
    }
}

pub fn fixture_results() -> Vec<RunResultInput> {
    vec![
        victory_result(),
        defeat_result(),
        abandonment_result(),
        pending_result(),
        hidden_result(),
    ]
}

fn summary(
    summary_id: &str,
    profile_id: &str,
    run_id: &str,
    detail: RunResultFieldValue<String>,
    outcome: RunOutcome,
    score_total: RunResultFieldValue<RunResultQuantity>,
    visibility: RunResultVisibility,
) -> RunSummaryInput {
    RunSummaryInput {
        summary_id: summary_id.to_owned(),
        profile_id: profile_id.to_owned(),
        run_id: run_id.to_owned(),
        detail_result_id: detail,
        outcome,
        character_id: RunResultFieldValue::present("character.ironclad".to_owned()),
        act: RunResultFieldValue::present(3),
        floor: RunResultFieldValue::present(51),
        duration: duration(2_520),
        score_total,
        origin: ResultOrigin::Native,
        visibility,
    }
}

pub fn summary_victory() -> RunSummaryInput {
    summary(
        "summary.alpha.victory",
        "profile.alpha",
        "run.alpha.1",
        RunResultFieldValue::present("result.victory".to_owned()),
        RunOutcome::Victory,
        RunResultFieldValue::present(quantity("points", 75)),
        RunResultVisibility::Public,
    )
}

pub fn summary_defeat() -> RunSummaryInput {
    summary(
        "summary.alpha.defeat",
        "profile.alpha",
        "run.alpha.2",
        RunResultFieldValue::present("result.defeat".to_owned()),
        RunOutcome::Defeat,
        RunResultFieldValue::present(quantity("points", 12)),
        RunResultVisibility::Public,
    )
}

/// A summary whose older detail the catalog does not carry, stated rather than reconstructed.
pub fn summary_without_detail() -> RunSummaryInput {
    summary(
        "summary.beta.abandon",
        "profile.beta",
        "run.beta.1",
        RunResultFieldValue::absent(),
        RunOutcome::Abandonment,
        RunResultFieldValue::absent(),
        RunResultVisibility::Public,
    )
}

pub fn summary_owner_only() -> RunSummaryInput {
    summary(
        "summary.alpha.pending",
        "profile.alpha",
        "run.alpha.3",
        RunResultFieldValue::absent(),
        RunOutcome::Victory,
        RunResultFieldValue::present(quantity("points", 9)),
        RunResultVisibility::OwnerOnly,
    )
}

pub fn summary_hidden() -> RunSummaryInput {
    summary(
        "summary.gamma.hidden",
        "profile.gamma",
        "run.gamma.1",
        RunResultFieldValue::absent(),
        RunOutcome::Victory,
        RunResultFieldValue::present(quantity("points", 5)),
        RunResultVisibility::Hidden,
    )
}

pub fn fixture_summaries() -> Vec<RunSummaryInput> {
    vec![
        summary_victory(),
        summary_defeat(),
        summary_without_detail(),
        summary_owner_only(),
        summary_hidden(),
    ]
}

pub fn snapshot(
    manifest: &ContentManifest,
    results: Vec<RunResultInput>,
    summaries: Vec<RunSummaryInput>,
) -> RunResultCatalogSnapshot {
    RunResultCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: RUN_RESULT_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: RunResultFamilyCoverage {
            state: RunResultFamilyState::Handled,
            result_count: results.len(),
            summary_count: summaries.len(),
        },
        results,
        summaries,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultSource {
    pub outcome: Result<RunResultCatalogSnapshot, RunResultSourceError>,
    reads: Cell<usize>,
}

impl RunResultSource {
    pub fn answering(snapshot: RunResultCatalogSnapshot) -> Self {
        Self {
            outcome: Ok(snapshot),
            reads: Cell::new(0),
        }
    }

    pub fn failing(error: RunResultSourceError) -> Self {
        Self {
            outcome: Err(error),
            reads: Cell::new(0),
        }
    }

    pub fn reads(&self) -> usize {
        self.reads.get()
    }
}

impl RunResultCatalogSource for RunResultSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RunResultCatalogSnapshot, RunResultSourceError> {
        self.reads.set(self.reads.get() + 1);
        self.outcome.clone()
    }
}

pub fn produce_with(
    manifest: &ContentManifest,
    source: &RunResultSource,
) -> Result<RunResultCatalog, sts2_game_mod::RunResultError> {
    RunResultCatalogProducer::new().produce(manifest, source)
}

pub fn produce(
    manifest: &ContentManifest,
    results: Vec<RunResultInput>,
    summaries: Vec<RunSummaryInput>,
) -> RunResultCatalog {
    produce_with(
        manifest,
        &RunResultSource::answering(snapshot(manifest, results, summaries)),
    )
    .expect("catalog")
}

/// The shared catalog: five results and five summaries over the fixture manifest.
pub fn fixture_catalog() -> (ContentManifest, RunResultCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, fixture_results(), fixture_summaries());
    (manifest, catalog)
}

/// Binds one catalog generation and result identity into a published reference.
pub fn result_reference(catalog: &RunResultCatalog, result_id: &str) -> RunResultReference {
    RunResultReference {
        catalog: catalog.binding().clone(),
        result_id: result_id.to_owned(),
    }
}

/// Binds one catalog generation and summary identity into a published reference.
pub fn summary_reference(catalog: &RunResultCatalog, summary_id: &str) -> RunSummaryReference {
    RunSummaryReference {
        catalog: catalog.binding().clone(),
        summary_id: summary_id.to_owned(),
    }
}
