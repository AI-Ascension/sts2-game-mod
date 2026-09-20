// SPDX-License-Identifier: MIT

//! Completion-outcome, finalization and persistence coherence.

use super::super::{
    ResultFinalization, RunOutcome, RunResultError, RunResultInput, RunResultPersistence,
    ScoreAuthority, ScoreMode,
};
use super::scores::score_reports_a_total;

/// Refuses a result whose finalization, persistence and outcome contradict one another.
pub(super) fn validate_completion(input: &RunResultInput) -> Result<(), RunResultError> {
    let terminal = !matches!(input.outcome, RunOutcome::Unfinished);
    match input.finalization {
        ResultFinalization::Finalized => {
            if input.persistence != RunResultPersistence::Confirmed {
                return Err(RunResultError::UnconfirmedFinalization);
            }
            if !terminal {
                return Err(RunResultError::UnfinishedResultPublishedAsFinalized);
            }
        }
        ResultFinalization::PendingPersistence | ResultFinalization::Unsettled => {
            if input.persistence == RunResultPersistence::Confirmed {
                return Err(RunResultError::UnconfirmedFinalization);
            }
        }
        ResultFinalization::Unavailable => {
            if terminal {
                return Err(RunResultError::UnfinishedResultPublishedAsFinalized);
            }
        }
    }
    if !terminal && score_reports_a_total(&input.score) {
        return Err(RunResultError::ScoreForUnfinishedRun);
    }
    if input.score.authority == ScoreAuthority::NoScore
        && (input.score.mode != ScoreMode::Unsupported || score_reports_a_total(&input.score))
    {
        return Err(RunResultError::ScoreModeConflict);
    }
    if terminal
        && input.score.mode == ScoreMode::Componentized
        && !score_reports_a_total(&input.score)
    {
        return Err(RunResultError::OutcomeWithoutScore);
    }
    Ok(())
}
