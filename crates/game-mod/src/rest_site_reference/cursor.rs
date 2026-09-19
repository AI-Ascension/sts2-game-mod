// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    error::RestSiteError,
    model::RestCatalogBinding,
    page::{
        ContinuationScope, RestOptionContinuation, RestOptionCursorState, RestOptionListQuery,
        RestSiteContinuation, RestSiteCursorState, RestSiteListQuery,
    },
};

/// Bounded registry of single-use continuations retained by one rest-site reader.
#[derive(Debug, Default)]
pub(super) struct RestCursors {
    sites: BTreeMap<String, RestSiteCursorState>,
    options: BTreeMap<String, RestOptionCursorState>,
    next: u64,
}

impl RestCursors {
    /// Creates an empty registry.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Consumes one rest-site-list continuation and returns its offset.
    pub(super) fn site_start(
        &mut self,
        binding: &RestCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &RestSiteListQuery,
    ) -> Result<usize, RestSiteError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(RestSiteError::InvalidContinuation);
        }
        let cursor = self
            .sites
            .remove(continuation.token())
            .ok_or(RestSiteError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RestSiteError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one rest-site-list continuation when the page is partial.
    pub(super) fn next_site(
        &mut self,
        binding: &RestCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &RestSiteListQuery,
        end: usize,
        total: usize,
    ) -> Option<RestSiteContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.sites);
        let token = self.make_token("rest-site-cursor");
        self.sites.insert(
            token.clone(),
            RestSiteCursorState {
                binding: binding.clone(),
                locale: query.locale.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(RestSiteContinuation::new(token, Arc::clone(scope)))
    }

    /// Consumes one option-list continuation and returns its offset.
    pub(super) fn option_start(
        &mut self,
        binding: &RestCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &RestOptionListQuery,
    ) -> Result<usize, RestSiteError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(RestSiteError::InvalidContinuation);
        }
        let cursor = self
            .options
            .remove(continuation.token())
            .ok_or(RestSiteError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.site_id != query.site.site_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RestSiteError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one option-list continuation when the page is partial.
    pub(super) fn next_option(
        &mut self,
        binding: &RestCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &RestOptionListQuery,
        end: usize,
        total: usize,
    ) -> Option<RestOptionContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.options);
        let token = self.make_token("rest-option-cursor");
        self.options.insert(
            token.clone(),
            RestOptionCursorState {
                binding: binding.clone(),
                site_id: query.site.site_id.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(RestOptionContinuation::new(token, Arc::clone(scope)))
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next);
        self.next = self.next.saturating_add(1);
        token
    }
}

/// Bounds a retained cursor registry by dropping the oldest outstanding token.
fn prune<T>(cursors: &mut BTreeMap<String, T>) {
    if cursors.len() >= super::model::REST_MAX_CONTINUATIONS
        && let Some(oldest) = cursors.keys().next().cloned()
    {
        cursors.remove(&oldest);
    }
}
