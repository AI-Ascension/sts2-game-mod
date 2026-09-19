// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    error::SelectionError,
    model::SelectionCatalogBinding,
    page::{
        ContinuationScope, SelectionCandidateContinuation, SelectionCandidateCursorState,
        SelectionCandidateListQuery, SelectionContinuation, SelectionCursorState,
        SelectionListQuery,
    },
};

/// Bounded registry of single-use continuations retained by one selection reader.
#[derive(Debug, Default)]
pub(super) struct SelectionCursors {
    selections: BTreeMap<String, SelectionCursorState>,
    candidates: BTreeMap<String, SelectionCandidateCursorState>,
    next: u64,
}

impl SelectionCursors {
    /// Creates an empty registry.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Consumes one selection-list continuation and returns its offset.
    pub(super) fn selection_start(
        &mut self,
        binding: &SelectionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &SelectionListQuery,
    ) -> Result<usize, SelectionError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(SelectionError::InvalidContinuation);
        }
        let cursor = self
            .selections
            .remove(continuation.token())
            .ok_or(SelectionError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(SelectionError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one selection-list continuation when the page is partial.
    pub(super) fn next_selection(
        &mut self,
        binding: &SelectionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &SelectionListQuery,
        end: usize,
        total: usize,
    ) -> Option<SelectionContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.selections);
        let token = self.make_token("selection-cursor");
        self.selections.insert(
            token.clone(),
            SelectionCursorState {
                binding: binding.clone(),
                locale: query.locale.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(SelectionContinuation::new(token, Arc::clone(scope)))
    }

    /// Consumes one candidate-list continuation and returns its offset.
    ///
    /// A continuation is bound to the picks it was created for, so a page cannot be resumed against
    /// a different prompt state.
    pub(super) fn candidate_start(
        &mut self,
        binding: &SelectionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &SelectionCandidateListQuery,
        selected: &[String],
    ) -> Result<usize, SelectionError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(SelectionError::InvalidContinuation);
        }
        let cursor = self
            .candidates
            .remove(continuation.token())
            .ok_or(SelectionError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.selection_id != query.selector.selection.selection_id
            || cursor.selector_generation != query.selector.selector_generation
            || cursor.scope != query.scope
            || cursor.limit != query.limit
            || cursor.selected != selected
        {
            return Err(SelectionError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one candidate-list continuation when the page is partial.
    pub(super) fn next_candidate(
        &mut self,
        binding: &SelectionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &SelectionCandidateListQuery,
        selected: &[String],
        end: usize,
        total: usize,
    ) -> Option<SelectionCandidateContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.candidates);
        let token = self.make_token("selection-candidate-cursor");
        self.candidates.insert(
            token.clone(),
            SelectionCandidateCursorState {
                binding: binding.clone(),
                selection_id: query.selector.selection.selection_id.clone(),
                selector_generation: query.selector.selector_generation,
                scope: query.scope,
                limit: query.limit,
                offset: end,
                selected: selected.to_vec(),
            },
        );
        Some(SelectionCandidateContinuation::new(
            token,
            Arc::clone(scope),
        ))
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next);
        self.next = self.next.saturating_add(1);
        token
    }
}

/// Bounds a retained cursor registry by dropping the oldest outstanding token.
fn prune<T>(cursors: &mut BTreeMap<String, T>) {
    if cursors.len() >= super::model::SEL_MAX_CONTINUATIONS
        && let Some(oldest) = cursors.keys().next().cloned()
    {
        cursors.remove(&oldest);
    }
}
