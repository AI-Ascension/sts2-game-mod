// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::catalog::{
    GlossaryCatalog, GlossaryTerm, GlossaryTermDetail, GlossaryTermReference, GlossaryTermSummary,
};
use super::model::GlossaryQueryScope;
use super::model::validate_text;
use super::projection::detail_bytes;
use super::query::{
    GlossaryCompleteness, GlossaryContinuation, GlossaryContinuationScope, GlossaryListPage,
    GlossaryListQuery, GlossaryQueryBinding, GlossaryQueryOperation, GlossarySearchPage,
    GlossarySearchQuery, GlossarySearchResult,
};
use super::{GLOSSARY_MAX_PAGE_ITEMS, GlossaryCatalogError};

/// Mutable cursor state over an otherwise immutable glossary catalog.
#[derive(Debug)]
pub struct GlossaryReader {
    catalog: GlossaryCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<GlossaryContinuationScope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CursorState {
    binding: GlossaryQueryBinding,
    offset: usize,
}

impl GlossaryReader {
    pub(super) fn new(catalog: GlossaryCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(GlossaryContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &GlossaryCatalog {
        &self.catalog
    }

    /// Replaces the catalog and expires outstanding continuations.
    pub fn replace_catalog(&mut self, catalog: GlossaryCatalog) {
        if self.catalog != catalog {
            self.cursors.clear();
        }
        self.catalog = catalog;
    }

    /// Lists visible term summaries in stable term-ID order.
    pub fn list(
        &mut self,
        query: &GlossaryListQuery,
    ) -> Result<GlossaryListPage, GlossaryCatalogError> {
        self.validate_common(query.locale.as_str(), query.limit)?;
        let binding = GlossaryQueryBinding {
            operation: GlossaryQueryOperation::List,
            catalog: self.catalog.binding().clone(),
            locale: query.locale.clone(),
            scope: query.scope,
            literal: String::new(),
            limit: query.limit,
        };
        let start = self.cursor_start(query.continuation.as_ref(), &binding)?;
        let candidates = self.visible_terms(query.scope);
        let end = start.saturating_add(query.limit).min(candidates.len());
        let entries = candidates[start..end]
            .iter()
            .map(|term| GlossaryTermSummary::from_term(term))
            .collect::<Vec<_>>();
        let total = candidates.len();
        drop(candidates);
        let continuation = self.make_continuation(&binding, end, total);
        Ok(GlossaryListPage {
            binding,
            entries,
            total,
            completeness: if continuation.is_some() {
                GlossaryCompleteness::Partial
            } else {
                GlossaryCompleteness::Complete
            },
            continuation,
        })
    }

    /// Searches visible localized text with literal, deterministic ranking.
    pub fn search(
        &mut self,
        query: &GlossarySearchQuery,
    ) -> Result<GlossarySearchPage, GlossaryCatalogError> {
        self.validate_common(query.locale.as_str(), query.limit)?;
        validate_text(&query.literal)?;
        let binding = GlossaryQueryBinding {
            operation: GlossaryQueryOperation::Search,
            catalog: self.catalog.binding().clone(),
            locale: query.locale.clone(),
            scope: query.scope,
            literal: query.literal.clone(),
            limit: query.limit,
        };
        let start = self.cursor_start(query.continuation.as_ref(), &binding)?;
        let mut matches = self
            .visible_terms(query.scope)
            .into_iter()
            .filter_map(|term| search_rank(term, &query.literal).map(|rank| (rank, term)))
            .collect::<Vec<_>>();
        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left.reference.term_id.cmp(&right.reference.term_id))
        });
        let end = start.saturating_add(query.limit).min(matches.len());
        let entries = matches[start..end]
            .iter()
            .map(|(rank, term)| GlossarySearchResult {
                summary: GlossaryTermSummary::from_term(term),
                rank: *rank,
            })
            .collect::<Vec<_>>();
        let total = matches.len();
        drop(matches);
        let continuation = self.make_continuation(&binding, end, total);
        Ok(GlossarySearchPage {
            binding,
            entries,
            total,
            completeness: if continuation.is_some() {
                GlossaryCompleteness::Partial
            } else {
                GlossaryCompleteness::Complete
            },
            continuation,
        })
    }

    /// Performs one exact lookup with explicit visibility and detail bounds.
    pub fn get(
        &self,
        reference: &GlossaryTermReference,
        scope: GlossaryQueryScope,
    ) -> Result<GlossaryTermDetail, GlossaryCatalogError> {
        if reference.catalog != *self.catalog.binding() {
            return Err(GlossaryCatalogError::StaleReference);
        }
        let term = self
            .catalog
            .term(&reference.term_id)
            .ok_or(GlossaryCatalogError::NotFound)?;
        if !self.catalog.visible(term, scope) {
            return Err(GlossaryCatalogError::ExcludedByScope);
        }
        detail_bytes(term)?;
        Ok(term.clone())
    }

    fn validate_common(&self, locale: &str, limit: usize) -> Result<(), GlossaryCatalogError> {
        if locale != self.catalog.locale() {
            return Err(GlossaryCatalogError::LocaleMismatch);
        }
        if limit == 0 || limit > GLOSSARY_MAX_PAGE_ITEMS {
            return Err(GlossaryCatalogError::InvalidPageSize);
        }
        Ok(())
    }

    fn visible_terms(&self, scope: GlossaryQueryScope) -> Vec<&GlossaryTerm> {
        self.catalog
            .terms
            .values()
            .filter(|term| self.catalog.visible(term, scope))
            .collect()
    }

    fn cursor_start(
        &mut self,
        continuation: Option<&GlossaryContinuation>,
        binding: &GlossaryQueryBinding,
    ) -> Result<usize, GlossaryCatalogError> {
        let Some(continuation) = continuation else {
            return Ok(0);
        };
        if !continuation.scope_matches(&self.scope) {
            return Err(GlossaryCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(GlossaryCatalogError::InvalidContinuation)?;
        if cursor.binding != *binding {
            return Err(GlossaryCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_continuation(
        &mut self,
        binding: &GlossaryQueryBinding,
        end: usize,
        total: usize,
    ) -> Option<GlossaryContinuation> {
        if end >= total {
            return None;
        }
        let token = format!("glossary-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        self.cursors.insert(
            token.clone(),
            CursorState {
                binding: binding.clone(),
                offset: end,
            },
        );
        Some(GlossaryContinuation::scoped(token, Arc::clone(&self.scope)))
    }
}

fn search_rank(term: &GlossaryTerm, literal: &str) -> Option<u8> {
    if literal.is_empty() {
        return Some(0);
    }
    let query = fold(literal);
    let mut rank = match_text(&term.display_name, &query, 0);
    for alias in &term.aliases {
        rank = merge_rank(rank, match_text(alias, &query, 3));
    }
    if let super::GlossaryDefinitionText::Available(definition) = &term.definition {
        rank = merge_rank(rank, match_text(definition, &query, 6));
    }
    for placeholder in &term.parameter_placeholders {
        rank = merge_rank(rank, match_text(placeholder, &query, 9));
    }
    for rule_reference in &term.rule_references {
        rank = merge_rank(rank, match_text(rule_reference, &query, 12));
    }
    rank
}

fn merge_rank(current: Option<u8>, candidate: Option<u8>) -> Option<u8> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (None, Some(candidate)) => Some(candidate),
        (current, None) => current,
    }
}

fn match_text(value: &str, query: &str, base: u8) -> Option<u8> {
    let folded = fold(value);
    if folded == query {
        return Some(base);
    }
    if folded.starts_with(query) {
        return Some(base.saturating_add(1));
    }
    folded.contains(query).then_some(base.saturating_add(2))
}

fn fold(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}
