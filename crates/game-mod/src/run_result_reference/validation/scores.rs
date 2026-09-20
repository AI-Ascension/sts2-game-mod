// SPDX-License-Identifier: MIT

//! Score reconciliation: components must sum to the displayed total, and authority must be honest.

use std::collections::BTreeSet;

use super::super::{
    RUN_RESULT_MAX_SCORE_COMPONENTS, ResultOrigin, RunResultError, RunResultScore, ScoreAuthority,
    ScoreMode,
};
use super::fields::{require_bounded, require_consistent, validate_text_field};

/// Returns whether the score publishes a displayed total.
pub(super) fn score_reports_a_total(score: &RunResultScore) -> bool {
    score.total.is_present()
}

/// Reconciles a score against its own components and its record's origin.
pub(super) fn validate_score(
    score: &RunResultScore,
    origin: ResultOrigin,
) -> Result<(), RunResultError> {
    require_consistent(&score.total, "score_total")?;
    require_consistent(&score.components, "score_components")?;
    if score.authority == ScoreAuthority::HostGameScore
        && matches!(origin, ResultOrigin::Harness | ResultOrigin::Imported)
    {
        return Err(RunResultError::EvaluatorScoreAsHostScore);
    }
    let components = match score.components.value() {
        Some(components) => components,
        None => {
            if score.mode == ScoreMode::Componentized && score.total.is_present() {
                return Err(RunResultError::UnreconciledScore);
            }
            return Ok(());
        }
    };
    require_bounded(
        components,
        RUN_RESULT_MAX_SCORE_COMPONENTS,
        "score_components",
    )?;
    if components.is_empty() {
        return Err(RunResultError::EmptyPresentCollection("score_components"));
    }
    let mut seen = BTreeSet::new();
    let mut summed: i64 = 0;
    for component in components {
        if !seen.insert(component.component_id.clone()) {
            return Err(RunResultError::DuplicateScoreComponent(
                component.component_id.clone(),
            ));
        }
        if component.authority != score.authority {
            return Err(RunResultError::ComponentAuthorityMismatch(
                component.component_id.clone(),
            ));
        }
        validate_text_field(&component.label, "score_component_label")?;
        summed = summed.saturating_add(component.contribution.amount);
    }
    let Some(total) = score.total.value() else {
        return Err(RunResultError::UnreconciledScore);
    };
    if total.amount != summed {
        return Err(RunResultError::ScoreTotalMismatch {
            displayed: total.amount,
            summed,
        });
    }
    Ok(())
}
