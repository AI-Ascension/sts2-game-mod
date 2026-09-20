// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::identity::is_opaque_coop_identity;
use super::super::model::COOP_MAX_PAGE_ITEMS;
use super::super::{
    CoopError, CoopFamilyState, CoopLiveFence, CoopPartyRecord, CoopPartyView, CoopPeerListQuery,
    CoopPeerPage, CoopPeerRecord, CoopPeerReference, CoopPeerView, CoopReadAuthority,
    CoopReadScope,
};
use super::catalog::CoopCatalog;
use super::page::{ContinuationScope, CoopPeerContinuation, PeerCursorState};
use super::summary::peer_summary;

/// Reader retaining one catalog while enforcing locale, party, scope and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`CoopCatalog::reader`] for an independent reader instead. It exposes no
/// entry point that acts in the party, selects the single-player save profile, or loads a save.
#[derive(Debug)]
pub struct CoopCatalogReader {
    pub(super) catalog: CoopCatalog,
    peer_cursors: BTreeMap<String, PeerCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl CoopCatalogReader {
    pub(super) fn new(catalog: CoopCatalog) -> Self {
        Self {
            catalog,
            peer_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &CoopCatalog {
        &self.catalog
    }

    /// Returns the capability this read withholds.
    #[must_use]
    pub const fn authority(&self) -> CoopReadAuthority {
        CoopReadAuthority::NotGranted
    }

    /// Reads the retained party record.
    pub fn party(&self) -> Result<CoopPartyRecord, CoopError> {
        self.validate_family()?;
        Ok(self.party_record()?.clone())
    }

    /// Reads the fenced live party.
    ///
    /// A live party belongs to the active instance, so the read requires the instance and party the
    /// observation was taken at; a fence that names another instance or party is stale rather than
    /// answered with that party's record.
    pub fn current(
        &self,
        fence: &CoopLiveFence,
        scope: CoopReadScope,
    ) -> Result<CoopPartyView, CoopError> {
        self.validate_family()?;
        Self::validate_fence(fence)?;
        let record = self.party_record()?;
        if record.party.instance_id != fence.instance_id || record.party.party_id != fence.party_id
        {
            return Err(CoopError::StaleLiveFence);
        }
        if record.party.epoch != fence.epoch {
            return Err(CoopError::EpochMismatch {
                expected: fence.epoch,
                actual: record.party.epoch,
            });
        }
        Ok(record.viewed(scope))
    }

    /// Projects one member under an explicit read scope.
    pub fn peer(
        &self,
        reference: &CoopPeerReference,
        scope: CoopReadScope,
    ) -> Result<CoopPeerView, CoopError> {
        self.validate_family()?;
        self.peer_view(reference, scope)
    }

    /// Lists the members one read scope may observe, one bounded page at a time.
    pub fn list_peers(&mut self, query: &CoopPeerListQuery) -> Result<CoopPeerPage, CoopError> {
        if query.locale != self.catalog.binding.locale {
            return Err(CoopError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > COOP_MAX_PAGE_ITEMS {
            return Err(CoopError::InvalidPageSize);
        }
        if query.live_fence.is_some() {
            return Err(CoopError::UnexpectedLiveFence);
        }
        self.validate_family()?;
        if query.party.catalog != self.catalog.binding {
            return Err(CoopError::StaleReference);
        }
        let record = self.party_record()?.clone();
        if query.party.party_id != record.party.party_id {
            return Err(CoopError::NotFound);
        }
        let start = self.peer_cursor_offset(query)?;
        let mut entries = Vec::new();
        for peer in &record.party.peers {
            if query.scope.observes_peer(peer.role) {
                entries.push(peer_summary(&peer_view_of(
                    &record,
                    &peer.peer_id,
                    query.scope,
                )?));
            }
        }
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start.min(end)..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("peer-cursor");
            self.peer_cursors.insert(
                token.clone(),
                PeerCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    party_id: query.party.party_id.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(CoopPeerContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(CoopPeerPage {
            binding: self.catalog.binding.clone(),
            authority: CoopReadAuthority::NotGranted,
            party: record.summary(query.scope),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    pub(in crate::coop_reference) fn validate_family(&self) -> Result<(), CoopError> {
        match self.catalog.family.state {
            CoopFamilyState::Handled => Ok(()),
            CoopFamilyState::Unavailable => Err(CoopError::UnavailableFamily),
        }
    }

    pub(in crate::coop_reference) fn party_record(&self) -> Result<&CoopPartyRecord, CoopError> {
        self.catalog
            .party
            .as_ref()
            .ok_or(CoopError::UnavailableFamily)
    }

    fn peer_view(
        &self,
        reference: &CoopPeerReference,
        scope: CoopReadScope,
    ) -> Result<CoopPeerView, CoopError> {
        if reference.catalog != self.catalog.binding {
            return Err(CoopError::StaleReference);
        }
        let record = self.party_record()?;
        if reference.party_id != record.party.party_id {
            return Err(CoopError::NotFound);
        }
        let peer = record
            .party
            .peers
            .iter()
            .find(|peer| peer.peer_id == reference.peer_id)
            .ok_or(CoopError::NotFound)?;
        if peer.generation != reference.generation {
            return Err(CoopError::StalePeerGeneration(reference.peer_id.clone()));
        }
        peer_view_of(record, &reference.peer_id, scope)
    }

    /// Returns whether a live fence names an instance and party this boundary may observe.
    pub(super) fn validate_fence(fence: &CoopLiveFence) -> Result<(), CoopError> {
        if fence.instance_id.is_empty() || fence.party_id.is_empty() {
            return Err(CoopError::MissingLiveFence);
        }
        if !is_opaque_coop_identity(&fence.instance_id) || !is_opaque_coop_identity(&fence.party_id)
        {
            return Err(CoopError::NonOpaqueIdentity("live_fence"));
        }
        Ok(())
    }

    fn peer_cursor_offset(&mut self, query: &CoopPeerListQuery) -> Result<usize, CoopError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(CoopError::InvalidContinuation);
        }
        let cursor = self
            .peer_cursors
            .remove(continuation.token())
            .ok_or(CoopError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.party_id != query.party.party_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(CoopError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

/// Projects one member of one party record, refusing a member this scope may not observe.
fn peer_view_of(
    record: &CoopPartyRecord,
    peer_id: &str,
    scope: CoopReadScope,
) -> Result<CoopPeerView, CoopError> {
    let peer = record
        .party
        .peers
        .iter()
        .find(|peer| peer.peer_id == peer_id)
        .ok_or(CoopError::NotFound)?;
    if !scope.observes_peer(peer.role) {
        return Err(CoopError::ExcludedByScope);
    }
    Ok(CoopPeerRecord {
        binding: record.binding.clone(),
        party_id: record.party.party_id.clone(),
        peer: peer.clone(),
    }
    .viewed(scope))
}
