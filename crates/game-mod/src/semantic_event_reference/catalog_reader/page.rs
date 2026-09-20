// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    SemanticCatalogBinding, SemanticEventKind, SemanticEventOrigin, SemanticEventReference,
    SemanticEventScope, SemanticFamilyCoverage, SemanticHistoryAuthority, SemanticHistoryFence,
    SemanticHistoryScope, SemanticSubjectRole,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use event-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticEventContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl SemanticEventContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque continuation token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded event-list request for one history.
///
/// Listing what happened selects nothing and starts nothing, so the request states the live fence it
/// does not hold. Supplying one is refused rather than ignored, because a caller that believes it is
/// holding a live observation is reading the wrong boundary.
#[derive(Debug, Eq, PartialEq)]
pub struct SemanticEventListQuery {
    /// Exact history whose events are listed.
    pub reference: SemanticCatalogBinding,
    /// Scope the caller observes from.
    pub scope: SemanticHistoryScope,
    /// The run, branch, episode and epoch the caller expects.
    pub event_scope: SemanticEventScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<SemanticEventContinuation>,
    /// A retained read holds no live fence; a supplied fence is refused.
    pub live_fence: Option<SemanticHistoryFence>,
}

/// Typed entry returned by one bounded event page.
///
/// A gap is summarised by its coverage alone: it has no kind, no origin, no subject and no value, so
/// a consumer can never read a gap as an event that happened to be empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventSummary {
    /// Exact static event reference.
    pub reference: SemanticEventReference,
    /// Sequence number this record occupies.
    pub sequence: u64,
    /// Whether this record was observed, dropped or is unsupported here.
    pub coverage: super::super::SemanticEventCoverage,
    /// Kind of event; present only when observed.
    pub kind: Option<SemanticEventKind>,
    /// Where the event came from; present only when observed.
    pub origin: Option<SemanticEventOrigin>,
    /// Subject roles this record names.
    pub roles: Vec<SemanticSubjectRole>,
}

/// Complete or partial event page of one history.
#[derive(Debug, Eq, PartialEq)]
pub struct SemanticEventPage {
    /// Catalog witness for every entry.
    pub binding: SemanticCatalogBinding,
    /// Capability this read does not grant.
    pub authority: SemanticHistoryAuthority,
    /// The history these events belong to, stated with the counts each scope observes.
    pub family: SemanticFamilyCoverage,
    /// The run, branch, episode and epoch these events belong to.
    pub event_scope: SemanticEventScope,
    /// Scope the caller observed from.
    pub scope: SemanticHistoryScope,
    /// Deterministically ordered entries.
    pub entries: Vec<SemanticEventSummary>,
    /// Number of records this scope observes, gaps included.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<SemanticEventContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct EventCursorState {
    pub(super) binding: SemanticCatalogBinding,
    pub(super) scope: SemanticHistoryScope,
    pub(super) event_scope: SemanticEventScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
