// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    ResultOrigin, RunOutcome, RunResultCatalogBinding, RunResultDetailState, RunResultDuration,
    RunResultFieldValue, RunResultLiveFence, RunResultQuantity, RunResultReadAuthority,
    RunResultVisibility, RunResultVisibilityScope, RunSummaryReference,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use run-summary continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunSummaryContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RunSummaryContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded, scoped native run-summary list request.
///
/// A summary list is a history read: it selects no profile, loads no save and starts no run, so the
/// request states the live fence it does not hold. Supplying one is refused rather than ignored,
/// because a caller that believes it is holding a live observation is reading the wrong boundary.
#[derive(Debug, Eq, PartialEq)]
pub struct RunSummaryListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Opaque profile whose summaries are listed; absent lists every profile the scope observes.
    pub profile_id: RunResultFieldValue<String>,
    /// Visibility scope.
    pub scope: RunResultVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RunSummaryContinuation>,
    /// A history read holds no live fence; a supplied fence is refused.
    pub live_fence: Option<RunResultLiveFence>,
}

/// Bounded, scoped request for one summary's detail result.
#[derive(Debug, Eq, PartialEq)]
pub struct RunResultDetailQuery {
    /// Exact summary whose detail is requested.
    pub summary: RunSummaryReference,
    /// Opaque profile expected to own the summary; absent accepts the record's own owner.
    pub profile_id: RunResultFieldValue<String>,
    /// Visibility scope.
    pub scope: RunResultVisibilityScope,
    /// A history read holds no live fence; a supplied fence is refused.
    pub live_fence: Option<RunResultLiveFence>,
}

/// Typed summary returned by one bounded summary page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSummarySummary {
    /// Exact static summary reference.
    pub reference: RunSummaryReference,
    /// Opaque owning profile identity.
    pub profile_id: String,
    /// Opaque run identity this summary belongs to.
    pub run_id: String,
    /// Reported completion outcome.
    pub outcome: RunOutcome,
    /// Character the run was played with, or its stated absence.
    pub character_id: RunResultFieldValue<String>,
    /// Furthest act reached, or its stated absence.
    pub act: RunResultFieldValue<u32>,
    /// Furthest floor reached, or its stated absence.
    pub floor: RunResultFieldValue<u32>,
    /// Run duration with its clock kind, or its stated absence.
    pub duration: RunResultFieldValue<RunResultDuration>,
    /// Displayed score total, or its stated absence.
    pub score_total: RunResultFieldValue<RunResultQuantity>,
    /// Whether this summary's detail can be read.
    pub detail_state: RunResultDetailState,
    /// Origin of this record.
    pub origin: ResultOrigin,
    /// Visibility class of this record.
    pub visibility: RunResultVisibility,
}

/// Complete or partial native run-summary page.
#[derive(Debug, Eq, PartialEq)]
pub struct RunSummaryPage {
    /// Catalog witness for every entry.
    pub binding: RunResultCatalogBinding,
    /// Capability this read does not grant.
    pub authority: RunResultReadAuthority,
    /// Deterministically ordered summaries.
    pub entries: Vec<RunSummarySummary>,
    /// Number of visible summaries.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RunSummaryContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct SummaryCursorState {
    pub(super) binding: RunResultCatalogBinding,
    pub(super) locale: String,
    pub(super) profile_id: Option<String>,
    pub(super) scope: RunResultVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
