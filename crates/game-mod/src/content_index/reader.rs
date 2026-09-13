// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::engine::ContentIndex;
use super::errors::validate_text;
use super::helpers::{identity, matches_filters, search_rank, summary, validate_optional_identity};
use super::query::ContentContinuationScope;
use super::{
    ContentCompleteness, ContentContinuation, ContentDefinition, ContentDefinitionReference,
    ContentFilterKind, ContentIndexError, ContentListPage, ContentListQuery, ContentQueryBinding,
    ContentQueryFilters, ContentQueryOperation, ContentQueryScope, ContentSearchPage,
    ContentSearchQuery, ContentSearchResult,
};

/// Mutable cursor state over an otherwise immutable index.
#[derive(Debug)]
pub struct ContentIndexReader {
    index: ContentIndex,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContentContinuationScope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CursorState {
    binding: ContentQueryBinding,
    offset: usize,
}

impl ContentIndexReader {
    /// Creates a reader with a private continuation scope.
    #[must_use]
    pub fn new(index: ContentIndex) -> Self {
        Self {
            index,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContentContinuationScope),
        }
    }

    /// Returns the current immutable index.
    #[must_use]
    pub fn index(&self) -> &ContentIndex {
        &self.index
    }

    /// Replaces the index and expires every outstanding continuation.
    pub fn replace_index(&mut self, index: ContentIndex) {
        if self.index != index {
            self.cursors.clear();
        }
        self.index = index;
    }

    /// Enumerates visible summaries in deterministic family/ID order.
    pub fn list(&mut self, query: &ContentListQuery) -> Result<ContentListPage, ContentIndexError> {
        self.validate_common(query.locale.as_str(), &query.filters, query.limit)?;
        let binding = ContentQueryBinding {
            operation: ContentQueryOperation::List,
            manifest: self.index.manifest.clone(),
            locale: query.locale.clone(),
            scope: query.scope,
            filters: query.filters.clone(),
            literal: String::new(),
            limit: query.limit,
        };
        let start = self.cursor_start(query.continuation.as_ref(), &binding)?;
        let (entries, total, end) = {
            let candidates = self.filtered_definitions(&query.filters, query.scope);
            let end = start.saturating_add(query.limit).min(candidates.len());
            let entries = candidates[start..end]
                .iter()
                .map(|definition| summary(definition))
                .collect::<Vec<_>>();
            (entries, candidates.len(), end)
        };
        let continuation = self.make_continuation(&binding, end, total);
        Ok(ContentListPage {
            binding,
            entries,
            total,
            completeness: if continuation.is_some() {
                ContentCompleteness::Partial
            } else {
                ContentCompleteness::Complete
            },
            continuation,
        })
    }

    /// Searches names, aliases, and descriptions by a literal locale-bound substring.
    pub fn search(
        &mut self,
        query: &ContentSearchQuery,
    ) -> Result<ContentSearchPage, ContentIndexError> {
        self.validate_common(query.locale.as_str(), &query.filters, query.limit)?;
        validate_text(&query.literal)?;
        let binding = ContentQueryBinding {
            operation: ContentQueryOperation::Search,
            manifest: self.index.manifest.clone(),
            locale: query.locale.clone(),
            scope: query.scope,
            filters: query.filters.clone(),
            literal: query.literal.clone(),
            limit: query.limit,
        };
        let start = self.cursor_start(query.continuation.as_ref(), &binding)?;
        let mut matches = self
            .filtered_definitions(&query.filters, query.scope)
            .into_iter()
            .filter_map(|definition| {
                search_rank(definition, &query.literal).map(|rank| (rank, definition))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| identity(left).cmp(&identity(right)))
        });
        let (entries, total, end) = {
            let end = start.saturating_add(query.limit).min(matches.len());
            let entries = matches[start..end]
                .iter()
                .map(|(rank, definition)| ContentSearchResult {
                    summary: summary(definition),
                    rank: *rank,
                })
                .collect::<Vec<_>>();
            (entries, matches.len(), end)
        };
        let continuation = self.make_continuation(&binding, end, total);
        Ok(ContentSearchPage {
            binding,
            entries,
            total,
            completeness: if continuation.is_some() {
                ContentCompleteness::Partial
            } else {
                ContentCompleteness::Complete
            },
            continuation,
        })
    }

    /// Performs an exact lookup while retaining this reader's immutable index.
    pub fn get(
        &self,
        reference: &ContentDefinitionReference,
        scope: ContentQueryScope,
    ) -> Result<ContentDefinition, ContentIndexError> {
        self.index.get_internal(reference, scope)
    }

    fn validate_common(
        &self,
        locale: &str,
        filters: &ContentQueryFilters,
        limit: usize,
    ) -> Result<(), ContentIndexError> {
        if locale != self.index.locale {
            return Err(ContentIndexError::LocaleMismatch);
        }
        if limit == 0 || limit > super::CONTENT_INDEX_MAX_PAGE_ITEMS {
            return Err(ContentIndexError::InvalidPageSize);
        }
        self.index
            .registry
            .validate_kind(filters.entity_kind.as_deref())?;
        for filter in [
            filters
                .character_or_pool
                .as_ref()
                .map(|_| ContentFilterKind::CharacterOrPool),
            filters.rarity.as_ref().map(|_| ContentFilterKind::Rarity),
            filters.unlock_state.map(|_| ContentFilterKind::UnlockState),
        ]
        .into_iter()
        .flatten()
        {
            self.index
                .registry
                .validate_filter(filters.entity_kind.as_deref(), filter)?;
        }
        validate_optional_identity(filters.entity_kind.as_deref(), "entity_kind")?;
        validate_optional_identity(filters.origin_package.as_deref(), "origin_package")?;
        validate_optional_identity(filters.character_or_pool.as_deref(), "character_or_pool")?;
        Ok(())
    }

    fn filtered_definitions(
        &self,
        filters: &ContentQueryFilters,
        scope: ContentQueryScope,
    ) -> Vec<&ContentDefinition> {
        self.index
            .definitions
            .values()
            .filter(|definition| self.index.is_visible(definition, scope))
            .filter(|definition| matches_filters(definition, filters))
            .collect()
    }

    fn cursor_start(
        &mut self,
        continuation: Option<&ContentContinuation>,
        binding: &ContentQueryBinding,
    ) -> Result<usize, ContentIndexError> {
        let Some(continuation) = continuation else {
            return Ok(0);
        };
        if !continuation.scope_matches(&self.scope) {
            return Err(ContentIndexError::InvalidContinuation);
        }
        let Some(cursor) = self.cursors.remove(continuation.token()) else {
            return Err(ContentIndexError::InvalidContinuation);
        };
        if cursor.binding != *binding {
            return Err(ContentIndexError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_continuation(
        &mut self,
        binding: &ContentQueryBinding,
        end: usize,
        total: usize,
    ) -> Option<ContentContinuation> {
        if end >= total {
            return None;
        }
        let token = format!("content-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        self.cursors.insert(
            token.clone(),
            CursorState {
                binding: binding.clone(),
                offset: end,
            },
        );
        Some(ContentContinuation::scoped(token, Arc::clone(&self.scope)))
    }
}
