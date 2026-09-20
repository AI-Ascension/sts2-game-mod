// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    error::ActionError,
    model::ActionCatalogBinding,
    page::{
        ActionContinuation, ActionCursorState, ActionListQuery, ActionTargetContinuation,
        ActionTargetCursorState, ActionTargetListQuery, ContinuationScope,
    },
};

/// Bounded registry of single-use continuations retained by one action reader.
#[derive(Debug, Default)]
pub(super) struct ActionCursors {
    actions: BTreeMap<String, ActionCursorState>,
    targets: BTreeMap<String, ActionTargetCursorState>,
    next: u64,
}

impl ActionCursors {
    /// Creates an empty registry.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Consumes one action-list continuation and returns its offset.
    pub(super) fn action_start(
        &mut self,
        binding: &ActionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ActionListQuery,
    ) -> Result<usize, ActionError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(ActionError::InvalidContinuation);
        }
        let cursor = self
            .actions
            .remove(continuation.token())
            .ok_or(ActionError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ActionError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one action-list continuation when the page is partial.
    pub(super) fn next_action(
        &mut self,
        binding: &ActionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ActionListQuery,
        end: usize,
        total: usize,
    ) -> Option<ActionContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.actions);
        let token = self.make_token("action-cursor");
        self.actions.insert(
            token.clone(),
            ActionCursorState {
                binding: binding.clone(),
                locale: query.locale.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(ActionContinuation::new(token, Arc::clone(scope)))
    }

    /// Consumes one target-list continuation and returns its offset.
    ///
    /// A continuation is bound to the generation it was created for, so a page cannot be resumed
    /// against a frame the host has since moved past.
    pub(super) fn target_start(
        &mut self,
        binding: &ActionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ActionTargetListQuery,
    ) -> Result<usize, ActionError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(ActionError::InvalidContinuation);
        }
        let cursor = self
            .targets
            .remove(continuation.token())
            .ok_or(ActionError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.action_id != query.frame.action.action_id
            || cursor.instance_generation != query.frame.generation
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ActionError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one target-list continuation when the page is partial.
    pub(super) fn next_target(
        &mut self,
        binding: &ActionCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ActionTargetListQuery,
        end: usize,
        total: usize,
    ) -> Option<ActionTargetContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.targets);
        let token = self.make_token("action-target-cursor");
        self.targets.insert(
            token.clone(),
            ActionTargetCursorState {
                binding: binding.clone(),
                action_id: query.frame.action.action_id.clone(),
                instance_generation: query.frame.generation,
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(ActionTargetContinuation::new(token, Arc::clone(scope)))
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next);
        self.next = self.next.saturating_add(1);
        token
    }
}

/// Bounds a retained cursor registry by dropping the oldest outstanding token.
fn prune<T>(cursors: &mut BTreeMap<String, T>) {
    if cursors.len() >= super::model::ACTION_MAX_CONTINUATIONS
        && let Some(oldest) = cursors.keys().next().cloned()
    {
        cursors.remove(&oldest);
    }
}
