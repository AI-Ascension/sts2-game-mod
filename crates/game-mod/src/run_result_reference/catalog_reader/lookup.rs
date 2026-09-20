// SPDX-License-Identifier: MIT

use super::super::{
    RunResultError, RunResultFieldValue, RunResultLiveFence, RunResultRecord, RunResultReference,
    RunResultVisibilityScope, RunSummaryRecord, RunSummaryReference,
};
use super::page::RunResultDetailQuery;
use super::reader::RunResultCatalogReader;
use super::summary::carries_fenced_run;

impl RunResultCatalogReader {
    /// Reads the fenced current terminal result.
    ///
    /// A current result belongs to the active run, so the read requires the instance and run it was
    /// observed at; a fence that names no carried run is stale rather than answered with another
    /// run's result or with an empty success.
    pub fn current(
        &self,
        fence: &RunResultLiveFence,
        scope: RunResultVisibilityScope,
    ) -> Result<RunResultRecord, RunResultError> {
        self.validate_family()?;
        Self::validate_fence(fence)?;
        let record = self
            .catalog
            .results
            .values()
            .find(|record| carries_fenced_run(record, &fence.run_id))
            .ok_or(RunResultError::StaleLiveFence)?;
        if !scope.observes(record.result.visibility) {
            return Err(RunResultError::ExcludedByScope);
        }
        Ok(record.clone())
    }

    /// Performs one exact retained-result lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RunResultReference,
        scope: RunResultVisibilityScope,
    ) -> Result<RunResultRecord, RunResultError> {
        self.validate_family()?;
        Ok(self.result_for_reference(reference, scope)?.clone())
    }

    /// Performs one exact prior-summary lookup under an explicit visibility scope.
    pub fn get_summary(
        &self,
        reference: &RunSummaryReference,
        scope: RunResultVisibilityScope,
    ) -> Result<RunSummaryRecord, RunResultError> {
        self.validate_family()?;
        let unconstrained = RunResultFieldValue::absent();
        Ok(self
            .summary_for_reference(reference, scope, &unconstrained)?
            .clone())
    }

    /// Reads the detail result one summary names, without selecting a profile or loading a save.
    pub fn detail(&self, query: &RunResultDetailQuery) -> Result<RunResultRecord, RunResultError> {
        self.validate_family()?;
        if query.live_fence.is_some() {
            return Err(RunResultError::UnexpectedLiveFence);
        }
        if !query.profile_id.is_consistent() {
            return Err(RunResultError::InconsistentField("profile_id"));
        }
        let summary = self.summary_for_reference(&query.summary, query.scope, &query.profile_id)?;
        let Some(detail_id) = summary.summary.detail_result_id.value() else {
            return Err(RunResultError::DetailUnavailable(
                summary.summary.summary_id.clone(),
            ));
        };
        let record = self
            .catalog
            .results
            .get(detail_id)
            .ok_or_else(|| RunResultError::DetailUnavailable(detail_id.clone()))?;
        if !query.scope.observes(record.result.visibility) {
            return Err(RunResultError::ExcludedByScope);
        }
        Ok(record.clone())
    }
}
