// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    error::ShopCatalogError,
    model::ShopCatalogBinding,
    page::{
        ContinuationScope, ShopContinuation, ShopCursorState, ShopEntryContinuation,
        ShopEntryCursorState, ShopEntryListQuery, ShopListQuery, ShopServiceContinuation,
        ShopServiceCursorState, ShopServiceListQuery,
    },
};

/// Bounded registry of single-use continuations retained by one shop reader.
#[derive(Debug, Default)]
pub(super) struct ShopCursors {
    shops: BTreeMap<String, ShopCursorState>,
    entries: BTreeMap<String, ShopEntryCursorState>,
    services: BTreeMap<String, ShopServiceCursorState>,
    next: u64,
}

impl ShopCursors {
    /// Creates an empty registry.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Consumes one shop-list continuation and returns its offset.
    pub(super) fn shop_start(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopListQuery,
    ) -> Result<usize, ShopCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        let cursor = self
            .shops
            .remove(continuation.token())
            .ok_or(ShopCatalogError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one shop-list continuation when the page is partial.
    pub(super) fn next_shop(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopListQuery,
        end: usize,
        total: usize,
    ) -> Option<ShopContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.shops);
        let token = self.make_token("shop-cursor");
        self.shops.insert(
            token.clone(),
            ShopCursorState {
                binding: binding.clone(),
                locale: query.locale.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(ShopContinuation::new(token, Arc::clone(scope)))
    }

    /// Consumes one entry-list continuation and returns its offset.
    pub(super) fn entry_start(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopEntryListQuery,
    ) -> Result<usize, ShopCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        let cursor = self
            .entries
            .remove(continuation.token())
            .ok_or(ShopCatalogError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.shop_id != query.shop.shop_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one entry-list continuation when the page is partial.
    pub(super) fn next_entry(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopEntryListQuery,
        end: usize,
        total: usize,
    ) -> Option<ShopEntryContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.entries);
        let token = self.make_token("shop-entry-cursor");
        self.entries.insert(
            token.clone(),
            ShopEntryCursorState {
                binding: binding.clone(),
                shop_id: query.shop.shop_id.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(ShopEntryContinuation::new(token, Arc::clone(scope)))
    }

    /// Consumes one service-list continuation and returns its offset.
    pub(super) fn service_start(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopServiceListQuery,
    ) -> Result<usize, ShopCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, scope) {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        let cursor = self
            .services
            .remove(continuation.token())
            .ok_or(ShopCatalogError::InvalidContinuation)?;
        if cursor.binding != *binding
            || cursor.shop_id != query.shop.shop_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ShopCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    /// Retains one service-list continuation when the page is partial.
    pub(super) fn next_service(
        &mut self,
        binding: &ShopCatalogBinding,
        scope: &Arc<ContinuationScope>,
        query: &ShopServiceListQuery,
        end: usize,
        total: usize,
    ) -> Option<ShopServiceContinuation> {
        if end >= total {
            return None;
        }
        prune(&mut self.services);
        let token = self.make_token("shop-service-cursor");
        self.services.insert(
            token.clone(),
            ShopServiceCursorState {
                binding: binding.clone(),
                shop_id: query.shop.shop_id.clone(),
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(ShopServiceContinuation::new(token, Arc::clone(scope)))
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next);
        self.next = self.next.saturating_add(1);
        token
    }
}

/// Bounds a retained cursor registry by dropping the oldest outstanding token.
fn prune<T>(cursors: &mut BTreeMap<String, T>) {
    if cursors.len() >= super::model::SHOP_MAX_CONTINUATIONS
        && let Some(oldest) = cursors.keys().next().cloned()
    {
        cursors.remove(&oldest);
    }
}
