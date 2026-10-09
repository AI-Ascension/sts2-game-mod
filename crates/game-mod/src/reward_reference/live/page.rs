// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::RewardVisibilityScope;
use super::identity::{ReaderScope, RewardLiveOfferReference, RewardLiveSnapshotReference};
use super::snapshot::RewardLiveOfferDisposition;

/// One bounded live offer listing request.
#[derive(Debug)]
pub struct RewardLiveOfferListQuery {
    /// Snapshot captured by the same reader.
    pub snapshot: RewardLiveSnapshotReference,
    /// Locale must match the retained static catalog binding.
    pub locale: String,
    /// Existing static visibility scope applied to nested static references.
    pub scope: RewardVisibilityScope,
    /// Requested number of entries, bounded by the live reader.
    pub limit: usize,
    /// Move-only continuation returned by the immediately preceding page.
    pub continuation: Option<RewardLiveContinuation>,
}

/// Single-use continuation bound to a reader, snapshot, locale, scope, and page size.
#[derive(Debug)]
pub struct RewardLiveContinuation {
    pub(super) scope_owner: Arc<ReaderScope>,
    pub(super) snapshot: RewardLiveSnapshotReference,
    pub(super) locale: String,
    pub(super) visibility: RewardVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

/// One listing row with an exact reader-scoped live offer reference.
#[derive(Clone, Debug)]
pub struct RewardLiveOfferEntry {
    /// Current live offer identity and full snapshot fence.
    pub reference: RewardLiveOfferReference,
    /// Source-observed category and state.
    pub kind: super::super::RewardKind,
    /// Current state or its explicit unavailable reason.
    pub state: super::super::RewardField<super::super::RewardOfferState>,
    /// Exact source visibility label for this row.
    pub visibility: super::super::RewardVisibility,
    /// Visible counts and their completeness states.
    pub groups_status: super::super::RewardFieldStatus,
    /// Number of visible groups in this page's source snapshot.
    pub groups_count: usize,
    /// Item collection completeness.
    pub items_status: super::super::RewardFieldStatus,
    /// Number of visible live items.
    pub items_count: usize,
    /// Source projection of current selectability.
    pub disposition: RewardLiveOfferDisposition,
}

/// One bounded page of current live offers.
#[derive(Debug)]
pub struct RewardLiveOfferPage {
    /// Snapshot that owns these entries and continuation.
    pub snapshot: RewardLiveSnapshotReference,
    /// Returned source-owned offer rows.
    pub entries: Vec<RewardLiveOfferEntry>,
    /// Number of visible offers in the retained snapshot.
    pub total: usize,
    /// Whether the source offer collection is complete for this scope.
    pub status: super::super::RewardFieldStatus,
    /// Move-only continuation, if more visible offers remain.
    pub continuation: Option<RewardLiveContinuation>,
}
