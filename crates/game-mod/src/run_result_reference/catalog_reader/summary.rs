// SPDX-License-Identifier: MIT

use super::super::{RunResultRecord, RunSummaryRecord, RunSummaryReference};
use super::page::RunSummarySummary;

/// Builds one bounded summary entry that preserves every field's stated availability.
pub(super) fn summary_entry(record: &RunSummaryRecord) -> RunSummarySummary {
    RunSummarySummary {
        reference: RunSummaryReference {
            catalog: record.binding.clone(),
            summary_id: record.summary.summary_id.clone(),
        },
        profile_id: record.summary.profile_id.clone(),
        run_id: record.summary.run_id.clone(),
        outcome: record.summary.outcome,
        character_id: record.summary.character_id.clone(),
        act: record.summary.act.clone(),
        floor: record.summary.floor.clone(),
        duration: record.summary.duration.clone(),
        score_total: record.summary.score_total.clone(),
        detail_state: record.detail_state(),
        origin: record.summary.origin,
        visibility: record.summary.visibility,
    }
}

/// Returns whether one result record carries the run a fence names.
pub(super) fn carries_fenced_run(record: &RunResultRecord, run_id: &str) -> bool {
    record.result.run_id == run_id
}
