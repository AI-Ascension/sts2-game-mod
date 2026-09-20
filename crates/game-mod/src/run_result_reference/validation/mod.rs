// SPDX-License-Identifier: MIT

//! Fail-closed validation of completed-run results, score reconciliation and prior summaries.

mod fields;
mod history;
mod manifest;
mod outcomes;
mod references;
mod scores;

use crate::ContentManifest;

use self::fields::{require_consistent, validate_identity_field};
use self::history::validate_summary;
use self::outcomes::validate_completion;
use self::references::validate_run_records;
use self::scores::validate_score;

pub(super) use self::history::claim_run;

use super::identity::validate_opaque_identity;
use super::model::{RUN_RESULT_MAX_RESULT_BYTES, RUN_RESULT_MAX_SUMMARY_BYTES};
use super::{
    RunResultError, RunResultFieldValue, RunResultInput, RunResultItemKind, RunResultQuantity,
    RunResultScore, RunSummaryInput,
};

/// Validates one source-owned completed-run record before it enters an immutable catalog.
pub(super) fn validate_result(
    input: &RunResultInput,
    manifest: &ContentManifest,
) -> Result<(), RunResultError> {
    validate_opaque_identity(&input.result_id, "result_id")?;
    validate_opaque_identity(&input.profile_id, "profile_id")?;
    validate_opaque_identity(&input.run_id, "run_id")?;
    validate_identity_field(&input.character_id, "character_id")?;
    validate_identity_field(&input.configuration_id, "configuration_id")?;
    require_consistent(&input.act, "act")?;
    require_consistent(&input.floor, "floor")?;
    require_consistent(&input.duration, "duration")?;
    validate_completion(input)?;
    validate_score(&input.score, input.origin)?;
    validate_run_records(input, manifest)?;
    require_within(result_bytes(input), RUN_RESULT_MAX_RESULT_BYTES)
}

/// Validates one prior summary and binds its opaque identity, outcome and profile.
pub(super) fn validate_history(input: &RunSummaryInput) -> Result<(), RunResultError> {
    validate_summary(input)?;
    require_within(summary_bytes(input), RUN_RESULT_MAX_SUMMARY_BYTES)
}

fn require_within(actual: usize, limit: usize) -> Result<(), RunResultError> {
    if actual > limit {
        return Err(RunResultError::ResultTooLarge { limit, actual });
    }
    Ok(())
}

fn text_bytes(field: &RunResultFieldValue<String>) -> usize {
    field.value().map_or(0, String::len)
}

fn quantity_bytes(field: &RunResultFieldValue<RunResultQuantity>) -> usize {
    field.value().map_or(0, |quantity| quantity.unit.unit.len())
}

fn score_bytes(score: &RunResultScore) -> usize {
    let components = score.components.value().map_or(0, |components| {
        components
            .iter()
            .map(|component| {
                component.component_id.len()
                    + text_bytes(&component.label)
                    + component.contribution.unit.unit.len()
            })
            .sum()
    });
    quantity_bytes(&score.total) + components
}

/// Returns the aggregate owner-defined text bytes one result record would publish.
fn result_bytes(input: &RunResultInput) -> usize {
    let deck = input.ending_deck.value().map_or(0, |entries| {
        entries.iter().map(|entry| entry.card_id.len()).sum()
    });
    let inventory = input.ending_inventory.value().map_or(0, |entries| {
        entries
            .iter()
            .map(|entry| {
                entry.item_id.len()
                    + match &entry.kind {
                        RunResultItemKind::Other { entity_kind } => entity_kind.len(),
                        RunResultItemKind::Relic | RunResultItemKind::Potion => 0,
                    }
            })
            .sum()
    });
    let statistics = input.statistics.value().map_or(0, |entries| {
        entries
            .iter()
            .map(|entry| {
                entry.statistic_id.len() + text_bytes(&entry.label) + quantity_bytes(&entry.value)
            })
            .sum()
    });
    let linkage = input.linkage.value().map_or(0, |linkage| {
        linkage.trajectory_id.len() + text_bytes(&linkage.evidence_id)
    });
    input.result_id.len()
        + input.profile_id.len()
        + input.run_id.len()
        + text_bytes(&input.character_id)
        + text_bytes(&input.configuration_id)
        + deck
        + inventory
        + statistics
        + linkage
        + score_bytes(&input.score)
}

/// Returns the aggregate owner-defined text bytes one prior summary would publish.
fn summary_bytes(input: &RunSummaryInput) -> usize {
    input.summary_id.len()
        + input.profile_id.len()
        + input.run_id.len()
        + text_bytes(&input.detail_result_id)
        + text_bytes(&input.character_id)
        + quantity_bytes(&input.score_total)
}
