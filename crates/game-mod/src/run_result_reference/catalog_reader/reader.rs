// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::identity::is_opaque_identity;
use super::super::model::RUN_RESULT_MAX_PAGE_ITEMS;
use super::super::{
    RunResultError, RunResultFamilyState, RunResultFieldValue, RunResultLiveFence,
    RunResultReadAuthority, RunResultRecord, RunResultReference, RunResultVisibilityScope,
    RunSummaryRecord, RunSummaryReference,
};
use super::catalog::RunResultCatalog;
use super::page::{
    ContinuationScope, RunSummaryContinuation, RunSummaryListQuery, RunSummaryPage,
    SummaryCursorState,
};
use super::summary::summary_entry;

/// Reader retaining one catalog while enforcing locale, scope, profile and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`RunResultCatalog::reader`] for an independent reader instead. It exposes no
/// selection, save-loading or run-start entry point, so a history read cannot alter the active run.
#[derive(Debug)]
pub struct RunResultCatalogReader {
    pub(super) catalog: RunResultCatalog,
    summary_cursors: BTreeMap<String, SummaryCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl RunResultCatalogReader {
    pub(super) fn new(catalog: RunResultCatalog) -> Self {
        Self {
            catalog,
            summary_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &RunResultCatalog {
        &self.catalog
    }

    /// Returns the capability this read withholds.
    #[must_use]
    pub const fn authority(&self) -> RunResultReadAuthority {
        RunResultReadAuthority::NotGranted
    }

    /// Lists visible native prior summaries in stable summary-identity order.
    pub fn list(&mut self, query: &RunSummaryListQuery) -> Result<RunSummaryPage, RunResultError> {
        if query.locale != self.catalog.binding.locale {
            return Err(RunResultError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > RUN_RESULT_MAX_PAGE_ITEMS {
            return Err(RunResultError::InvalidPageSize);
        }
        if query.live_fence.is_some() {
            return Err(RunResultError::UnexpectedLiveFence);
        }
        if !query.profile_id.is_consistent() {
            return Err(RunResultError::InconsistentField("profile_id"));
        }
        if query.profile_id.is_present() && query.scope == RunResultVisibilityScope::Anonymous {
            return Err(RunResultError::ProfileNotObserved);
        }
        self.validate_family()?;
        let start = self.summary_cursor_offset(query)?;
        let entries = self
            .catalog
            .summaries
            .values()
            .filter(|record| observable(record, query))
            .map(summary_entry)
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start.min(end)..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("summary-cursor");
            self.summary_cursors.insert(
                token.clone(),
                SummaryCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    profile_id: query.profile_id.value().cloned(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(RunSummaryContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(RunSummaryPage {
            binding: self.catalog.binding.clone(),
            authority: RunResultReadAuthority::NotGranted,
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    pub(super) fn validate_family(&self) -> Result<(), RunResultError> {
        match self.catalog.family.state {
            RunResultFamilyState::Handled => Ok(()),
            RunResultFamilyState::Unavailable => Err(RunResultError::UnavailableFamily),
        }
    }

    pub(super) fn result_for_reference(
        &self,
        reference: &RunResultReference,
        scope: RunResultVisibilityScope,
    ) -> Result<&RunResultRecord, RunResultError> {
        if reference.catalog != self.catalog.binding {
            return Err(RunResultError::StaleReference);
        }
        let record = self
            .catalog
            .results
            .get(&reference.result_id)
            .ok_or(RunResultError::NotFound)?;
        if !scope.observes(record.result.visibility) {
            return Err(RunResultError::ExcludedByScope);
        }
        Ok(record)
    }

    pub(super) fn summary_for_reference(
        &self,
        reference: &RunSummaryReference,
        scope: RunResultVisibilityScope,
        profile_id: &RunResultFieldValue<String>,
    ) -> Result<&RunSummaryRecord, RunResultError> {
        if reference.catalog != self.catalog.binding {
            return Err(RunResultError::StaleReference);
        }
        if profile_id.is_present() && scope == RunResultVisibilityScope::Anonymous {
            return Err(RunResultError::ProfileNotObserved);
        }
        let record = self
            .catalog
            .summaries
            .get(&reference.summary_id)
            .ok_or(RunResultError::NotFound)?;
        if !scope.observes(record.summary.visibility) {
            return Err(RunResultError::ExcludedByScope);
        }
        if let Some(profile) = profile_id.value()
            && record.summary.profile_id != *profile
        {
            return Err(RunResultError::ProfileNotObserved);
        }
        Ok(record)
    }

    /// Returns whether a live fence names an instance and run this boundary may observe.
    pub(super) fn validate_fence(fence: &RunResultLiveFence) -> Result<(), RunResultError> {
        if fence.instance_id.is_empty() || fence.run_id.is_empty() {
            return Err(RunResultError::MissingLiveFence);
        }
        if !is_opaque_identity(&fence.instance_id) || !is_opaque_identity(&fence.run_id) {
            return Err(RunResultError::NonOpaqueIdentity("live_fence"));
        }
        Ok(())
    }

    fn summary_cursor_offset(
        &mut self,
        query: &RunSummaryListQuery,
    ) -> Result<usize, RunResultError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(RunResultError::InvalidContinuation);
        }
        let cursor = self
            .summary_cursors
            .remove(continuation.token())
            .ok_or(RunResultError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.profile_id.as_deref() != query.profile_id.value().map(String::as_str)
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RunResultError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

fn observable(record: &RunSummaryRecord, query: &RunSummaryListQuery) -> bool {
    query.scope.observes(record.summary.visibility)
        && query
            .profile_id
            .value()
            .is_none_or(|profile| record.summary.profile_id == *profile)
}
