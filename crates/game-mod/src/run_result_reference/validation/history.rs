// SPDX-License-Identifier: MIT

//! Prior-summary validation and the profile isolation its run identities must respect.

use std::collections::BTreeMap;

use super::super::identity::validate_opaque_identity;
use super::super::{RunOutcome, RunResultError, RunSummaryInput};
use super::fields::{require_consistent, validate_identity_field};

/// Validates one source-owned prior-summary record before it enters an immutable catalog.
pub(super) fn validate_summary(input: &RunSummaryInput) -> Result<(), RunResultError> {
    validate_opaque_identity(&input.summary_id, "summary_id")?;
    validate_opaque_identity(&input.profile_id, "profile_id")?;
    validate_opaque_identity(&input.run_id, "run_id")?;
    validate_identity_field(&input.detail_result_id, "detail_result_id")?;
    validate_identity_field(&input.character_id, "summary_character_id")?;
    require_consistent(&input.act, "summary_act")?;
    require_consistent(&input.floor, "summary_floor")?;
    require_consistent(&input.duration, "summary_duration")?;
    require_consistent(&input.score_total, "summary_score_total")?;
    if matches!(input.outcome, RunOutcome::Unfinished) && input.score_total.is_present() {
        return Err(RunResultError::ScoreForUnfinishedRun);
    }
    Ok(())
}

/// Records the profile that owns one run identity, refusing a second profile for the same run.
///
/// Identity alone is not a scope: a run copied under two profiles would let one profile read the
/// other's history, so the catalog refuses the whole snapshot instead of merging them.
pub(in crate::run_result_reference) fn claim_run(
    owners: &mut BTreeMap<String, String>,
    run_id: &str,
    profile_id: &str,
) -> Result<(), RunResultError> {
    match owners.get(run_id) {
        Some(existing) if existing != profile_id => {
            Err(RunResultError::ProfileIsolationViolation(run_id.to_owned()))
        }
        Some(_) => Ok(()),
        None => {
            owners.insert(run_id.to_owned(), profile_id.to_owned());
            Ok(())
        }
    }
}
