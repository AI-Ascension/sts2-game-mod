// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::model::REWARD_MAX_PAGE_ITEMS;
use super::super::{
    RewardCatalogError, RewardDefinitionReference, RewardFamilyState, RewardItem,
    RewardItemReference, RewardOfferDefinition, RewardVisibilityScope,
};
use super::catalog::RewardCatalog;
use super::page::{
    ContinuationScope, ItemCursorState, RewardContinuation, RewardCursorState,
    RewardDefinitionPage, RewardItemContinuation, RewardItemListQuery, RewardItemPage,
    RewardListQuery,
};
use super::summary::{item_summary, reward_summary};
use super::{collection_status, visible_item, visible_reward};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registries are mutable and continuations are
/// single-use. Call [`RewardCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct RewardCatalogReader {
    pub(super) catalog: RewardCatalog,
    reward_cursors: BTreeMap<String, RewardCursorState>,
    item_cursors: BTreeMap<String, ItemCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl RewardCatalogReader {
    pub(super) fn new(catalog: RewardCatalog) -> Self {
        Self {
            catalog,
            reward_cursors: BTreeMap::new(),
            item_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &RewardCatalog {
        &self.catalog
    }

    /// Lists visible reward definitions in stable reward-ID order.
    pub fn list(
        &mut self,
        query: &RewardListQuery,
    ) -> Result<RewardDefinitionPage, RewardCatalogError> {
        if query.locale != self.catalog.binding.locale {
            return Err(RewardCatalogError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > REWARD_MAX_PAGE_ITEMS {
            return Err(RewardCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.reward_cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible_reward(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end]
            .iter()
            .map(|definition| reward_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let continuation = if end < total {
            let token = self.make_token("reward-cursor");
            self.reward_cursors.insert(
                token.clone(),
                RewardCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(RewardContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(RewardDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible offered items for one exact reward.
    pub fn list_items(
        &mut self,
        query: &RewardItemListQuery,
    ) -> Result<RewardItemPage, RewardCatalogError> {
        if query.limit == 0 || query.limit > REWARD_MAX_PAGE_ITEMS {
            return Err(RewardCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.item_cursor_start(query)?;
        let definition = self.definition_for_reference(&query.reward, query.scope)?;
        let entries_all = definition
            .items
            .iter()
            .filter(|item| visible_item(item, query.scope))
            .map(item_summary)
            .collect::<Vec<_>>();
        let items_status = collection_status(&definition.items, query.scope, visible_item);
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all[start..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("reward-item-cursor");
            self.item_cursors.insert(
                token.clone(),
                ItemCursorState {
                    binding: self.catalog.binding.clone(),
                    reward_id: query.reward.reward_id.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(RewardItemContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(RewardItemPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            items_status,
            complete: continuation.is_none(),
            continuation,
        })
    }

    pub(super) fn validate_family(&self) -> Result<(), RewardCatalogError> {
        match self.catalog.family.state {
            RewardFamilyState::Handled => Ok(()),
            RewardFamilyState::Unsupported => Err(RewardCatalogError::UnsupportedFamily),
            RewardFamilyState::Unavailable => Err(RewardCatalogError::UnavailableFamily),
        }
    }

    pub(super) fn definition_for_reference(
        &self,
        reference: &RewardDefinitionReference,
        scope: RewardVisibilityScope,
    ) -> Result<&RewardOfferDefinition, RewardCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(RewardCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.reward_id)
            .ok_or(RewardCatalogError::NotFound)?;
        if !visible_reward(definition, scope) {
            return Err(RewardCatalogError::ExcludedByScope);
        }
        Ok(definition)
    }

    pub(super) fn item_for_reference(
        &self,
        reference: &RewardItemReference,
        scope: RewardVisibilityScope,
    ) -> Result<&RewardItem, RewardCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(RewardCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.reward_id)
            .ok_or(RewardCatalogError::NotFound)?;
        if !visible_reward(definition, scope) {
            return Err(RewardCatalogError::ExcludedByScope);
        }
        let item = definition
            .items
            .iter()
            .find(|item| item.reference.item_id == reference.item_id)
            .ok_or(RewardCatalogError::NotFound)?;
        if !visible_item(item, scope) {
            return Err(RewardCatalogError::ExcludedByScope);
        }
        Ok(item)
    }

    fn reward_cursor_start(
        &mut self,
        query: &RewardListQuery,
    ) -> Result<usize, RewardCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(RewardCatalogError::InvalidContinuation);
        }
        let cursor = self
            .reward_cursors
            .remove(continuation.token())
            .ok_or(RewardCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RewardCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn item_cursor_start(
        &mut self,
        query: &RewardItemListQuery,
    ) -> Result<usize, RewardCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(RewardCatalogError::InvalidContinuation);
        }
        let cursor = self
            .item_cursors
            .remove(continuation.token())
            .ok_or(RewardCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.reward_id != query.reward.reward_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RewardCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}
