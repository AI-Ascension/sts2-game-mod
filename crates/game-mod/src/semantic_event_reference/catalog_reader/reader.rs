// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::identity::is_opaque_semantic_identity;
use super::super::model::SEMANTIC_MAX_PAGE_ITEMS;
use super::super::{
    SemanticEventBatch, SemanticEventError, SemanticEventListQuery, SemanticEventPage,
    SemanticEventRecord, SemanticEventReference, SemanticEventSummary, SemanticFamilyState,
    SemanticHistoryAuthority, SemanticHistoryFence, SemanticHistoryScope, SemanticHistoryView,
};
use super::catalog::SemanticHistoryCatalog;
use super::page::{ContinuationScope, EventCursorState, SemanticEventContinuation};

/// Reader retaining one catalog while enforcing scope, identity and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`SemanticHistoryCatalog::reader`] for an independent reader instead. It exposes
/// no entry point that replays, rewinds or re-runs history, and none that acts in the run.
#[derive(Debug)]
pub struct SemanticEventReader {
    pub(super) catalog: SemanticHistoryCatalog,
    event_cursors: BTreeMap<String, EventCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl SemanticEventReader {
    pub(super) fn new(catalog: SemanticHistoryCatalog) -> Self {
        Self {
            catalog,
            event_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &SemanticHistoryCatalog {
        &self.catalog
    }

    /// Returns the capability this read withholds.
    #[must_use]
    pub const fn authority(&self) -> SemanticHistoryAuthority {
        SemanticHistoryAuthority::NotGranted
    }

    /// Reads the retained history at its own scope.
    pub fn history(&self) -> Result<SemanticHistoryView, SemanticEventError> {
        self.validate_family()?;
        let batch = self.batch()?;
        Ok(self.view(batch, SemanticHistoryScope::History, batch.events.clone()))
    }

    /// Reads the fenced live history.
    ///
    /// A history belongs to the run and branch it was observed in, so the read requires the place and
    /// the epoch the observation was taken at; a fence naming another place is stale, and another
    /// epoch is refused rather than answered with this history.
    pub fn current(
        &self,
        fence: &SemanticHistoryFence,
        scope: SemanticHistoryScope,
    ) -> Result<SemanticHistoryView, SemanticEventError> {
        self.validate_family()?;
        validate_fence(fence)?;
        let batch = self.batch()?;
        if batch.scope.run_id != fence.run_id
            || batch.scope.branch_id != fence.branch_id
            || batch.scope.episode != fence.episode
        {
            return Err(SemanticEventError::StaleHistoryFence);
        }
        if batch.scope.epoch != fence.epoch {
            return Err(SemanticEventError::EpochMismatch {
                expected: fence.epoch,
                actual: batch.scope.epoch,
            });
        }
        let asked = super::super::SemanticEventScope {
            run_id: fence.run_id.clone(),
            branch_id: fence.branch_id.clone(),
            episode: fence.episode,
            epoch: fence.epoch,
        };
        if !scope.observes(&asked, &batch.scope) {
            return Err(SemanticEventError::ExcludedByScope);
        }
        Ok(self.view(batch, scope, batch.events.clone()))
    }

    /// Reads one event by exact reference and returns the history it sits in.
    pub fn event(
        &self,
        reference: &SemanticEventReference,
        scope: SemanticHistoryScope,
    ) -> Result<SemanticHistoryView, SemanticEventError> {
        self.validate_family()?;
        if reference.catalog != self.catalog.binding {
            return Err(SemanticEventError::StaleReference);
        }
        let batch = self.batch()?;
        if !batch
            .events
            .iter()
            .any(|event| event.event_id == reference.event_id)
        {
            return Err(SemanticEventError::NotFound);
        }
        Ok(self.view(batch, scope, batch.events.clone()))
    }

    /// Lists the events one read scope may observe, one bounded page at a time.
    pub fn list_events(
        &mut self,
        query: &SemanticEventListQuery,
    ) -> Result<SemanticEventPage, SemanticEventError> {
        if query.limit == 0 || query.limit > SEMANTIC_MAX_PAGE_ITEMS {
            return Err(SemanticEventError::InvalidPageSize);
        }
        if query.live_fence.is_some() {
            return Err(SemanticEventError::ExcludedByScope);
        }
        self.validate_family()?;
        if query.reference != self.catalog.binding {
            return Err(SemanticEventError::StaleReference);
        }
        let batch = self.batch()?.clone();
        if !query.scope.observes(&query.event_scope, &batch.scope) {
            return Err(SemanticEventError::ExcludedByScope);
        }
        let start = self.cursor_offset(query)?;
        let entries = batch
            .events
            .iter()
            .map(|event| self.summary(&batch, event))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start.min(end)..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("event-cursor");
            self.event_cursors.insert(
                token.clone(),
                EventCursorState {
                    binding: self.catalog.binding.clone(),
                    scope: query.scope,
                    event_scope: query.event_scope.clone(),
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(SemanticEventContinuation::new(
                token,
                Arc::clone(&self.scope),
            ))
        } else {
            None
        };
        Ok(SemanticEventPage {
            binding: self.catalog.binding.clone(),
            authority: SemanticHistoryAuthority::NotGranted,
            family: self.catalog.family.clone(),
            event_scope: batch.scope.clone(),
            scope: query.scope,
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    fn validate_family(&self) -> Result<(), SemanticEventError> {
        match self.catalog.family.state {
            SemanticFamilyState::Handled => Ok(()),
            SemanticFamilyState::Unavailable => Err(SemanticEventError::UnavailableFamily),
        }
    }

    fn batch(&self) -> Result<&SemanticEventBatch, SemanticEventError> {
        self.catalog
            .batch
            .as_ref()
            .ok_or(SemanticEventError::UnavailableFamily)
    }

    /// Builds one bounded view: every record, gaps included, under the caller's scope.
    fn view(
        &self,
        batch: &SemanticEventBatch,
        scope: SemanticHistoryScope,
        events: Vec<super::super::SemanticEventInput>,
    ) -> SemanticHistoryView {
        let records = events
            .into_iter()
            .map(|event| SemanticEventRecord {
                binding: self.catalog.binding.clone(),
                scope: batch.scope.clone(),
                event,
            })
            .collect::<Vec<_>>();
        SemanticHistoryView {
            binding: self.catalog.binding.clone(),
            authority: SemanticHistoryAuthority::NotGranted,
            scope,
            family: self.catalog.family.clone(),
            event_scope: batch.scope.clone(),
            window: batch.window.clone(),
            total: records.len(),
            events: records,
        }
    }

    fn summary(
        &self,
        _batch: &SemanticEventBatch,
        event: &super::super::SemanticEventInput,
    ) -> SemanticEventSummary {
        SemanticEventSummary {
            reference: SemanticEventReference {
                catalog: self.catalog.binding.clone(),
                event_id: event.event_id.clone(),
            },
            sequence: event.sequence,
            coverage: event.coverage.clone(),
            kind: event.kind,
            origin: event.origin,
            roles: event.subjects.iter().map(|subject| subject.role).collect(),
        }
    }

    fn cursor_offset(
        &mut self,
        query: &SemanticEventListQuery,
    ) -> Result<usize, SemanticEventError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(SemanticEventError::InvalidContinuation);
        }
        let cursor = self
            .event_cursors
            .remove(continuation.token())
            .ok_or(SemanticEventError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.scope != query.scope
            || cursor.event_scope != query.event_scope
            || cursor.limit != query.limit
        {
            return Err(SemanticEventError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

/// Returns whether a live fence names a run and branch this boundary may observe.
pub(super) fn validate_fence(fence: &SemanticHistoryFence) -> Result<(), SemanticEventError> {
    if fence.run_id.is_empty() || fence.branch_id.is_empty() {
        return Err(SemanticEventError::StaleHistoryFence);
    }
    if !is_opaque_semantic_identity(&fence.run_id) || !is_opaque_semantic_identity(&fence.branch_id)
    {
        return Err(SemanticEventError::NonOpaqueIdentity("live_fence"));
    }
    Ok(())
}
