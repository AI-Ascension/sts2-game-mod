// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    ProgressionCatalogBinding, ProgressionDomain, ProgressionEntryReference, ProgressionFieldValue,
    ProgressionProfileQuery, ProgressionQuantity, ProgressionReadAuthority, ProgressionReadState,
    ProgressionRevision, ProgressionText, ProgressionVisibility, ProgressionVisibilityScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use progression continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProgressionContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ProgressionContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded, filtered, revision-fenced progression list request.
///
/// The request names the profile revision the caller believes it is reading: a snapshot taken at
/// another revision is refused rather than answered from stale data.
#[derive(Debug, Eq, PartialEq)]
pub struct ProgressionListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Profile the caller believes it is reading.
    pub profile: ProgressionProfileQuery,
    /// Requested visibility scope.
    pub scope: ProgressionVisibilityScope,
    /// Profile revision the caller believes it is reading, when it states one.
    pub revision: Option<ProgressionRevision>,
    /// Domain filter; absent lists every readable domain.
    pub domain: Option<ProgressionDomain>,
    /// Read-state filter; absent lists every state the source reports.
    pub state: Option<ProgressionReadState>,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ProgressionContinuation>,
}

/// Typed entry returned by one bounded page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionEntrySummary {
    /// Exact entry reference, carrying its catalog witness.
    pub reference: ProgressionEntryReference,
    /// Documented freshness of the read this entry came from.
    pub revision: ProgressionRevision,
    /// Domain this entry belongs to.
    pub domain: ProgressionDomain,
    /// What the source reports about the entry's state for this profile.
    pub read_state: ProgressionReadState,
    /// Localized entry title.
    pub title: String,
    /// Localized entry description or an explicit non-value.
    pub description: ProgressionText,
    /// Progress toward the entry, or its stated absence.
    pub progress: ProgressionFieldValue<ProgressionQuantity>,
    /// Best recorded value for the entry, or its stated absence.
    pub best: ProgressionFieldValue<ProgressionQuantity>,
    /// Number of requirements the source stated for this entry.
    pub stated_requirements: usize,
    /// Owner-defined visibility.
    pub visibility: ProgressionVisibility,
}

/// Complete or explicitly partial progression page.
#[derive(Debug, Eq, PartialEq)]
pub struct ProgressionEntryPage {
    /// Catalog witness for every entry.
    pub binding: ProgressionCatalogBinding,
    /// Documented freshness of this read.
    pub revision: ProgressionRevision,
    /// Capability this read does not grant.
    pub authority: ProgressionReadAuthority,
    /// Deterministically ordered entries.
    pub entries: Vec<ProgressionEntrySummary>,
    /// Number of visible entries matching the filter.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ProgressionContinuation>,
}

impl ProgressionEntryPage {
    /// Returns whether this page is partial and must be continued to be complete.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        !self.complete
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CursorState {
    pub(super) binding: ProgressionCatalogBinding,
    pub(super) locale: String,
    pub(super) profile: ProgressionProfileQuery,
    pub(super) scope: ProgressionVisibilityScope,
    pub(super) domain: Option<ProgressionDomain>,
    pub(super) state: Option<ProgressionReadState>,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
