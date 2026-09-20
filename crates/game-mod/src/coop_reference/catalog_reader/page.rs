// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    CoopCatalogBinding, CoopFieldValue, CoopLiveFence, CoopPartyReference, CoopPartySummary,
    CoopPeerFreshness, CoopPeerMembership, CoopPeerReference, CoopPeerRole, CoopReadAuthority,
    CoopReadScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use member-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopPeerContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl CoopPeerContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque continuation token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded, scoped member-list request for one party.
///
/// Listing the members of a party selects no save profile and starts nothing, so the request states
/// the live fence it does not hold. Supplying one is refused rather than ignored, because a caller
/// that believes it is holding a live observation is reading the wrong boundary.
#[derive(Debug, Eq, PartialEq)]
pub struct CoopPeerListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Exact party whose members are listed.
    pub party: CoopPartyReference,
    /// Scope the caller observes from.
    pub scope: CoopReadScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<CoopPeerContinuation>,
    /// A retained read holds no live fence; a supplied fence is refused.
    pub live_fence: Option<CoopLiveFence>,
}

/// Typed member returned by one bounded member page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPeerSummary {
    /// Exact static member reference.
    pub reference: CoopPeerReference,
    /// Whether this member is local or an ally.
    pub role: CoopPeerRole,
    /// Membership state.
    pub membership: CoopPeerMembership,
    /// Freshness of the reported data.
    pub freshness: CoopPeerFreshness,
    /// Character this member plays, or the reason no value is observed.
    pub character_id: CoopFieldValue<String>,
}

/// Complete or partial member page of one party.
#[derive(Debug, Eq, PartialEq)]
pub struct CoopPeerPage {
    /// Catalog witness for every entry.
    pub binding: CoopCatalogBinding,
    /// Capability this read does not grant.
    pub authority: CoopReadAuthority,
    /// The party these members belong to, stated with the counts each scope observes.
    pub party: CoopPartySummary,
    /// Deterministically ordered members.
    pub entries: Vec<CoopPeerSummary>,
    /// Number of members this scope observes.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<CoopPeerContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct PeerCursorState {
    pub(super) binding: CoopCatalogBinding,
    pub(super) locale: String,
    pub(super) party_id: String,
    pub(super) scope: CoopReadScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
