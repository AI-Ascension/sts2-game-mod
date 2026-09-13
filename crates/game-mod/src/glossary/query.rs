// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::ContentQueryLocale;

use super::catalog::GlossaryTermSummary;
use super::model::GlossaryCatalogBinding;
use super::model::GlossaryQueryScope;

/// Query operation bound into a continuation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryQueryOperation {
    /// Enumerate visible term summaries.
    List,
    /// Search localized names, aliases, definitions, placeholders, and rule IDs.
    Search,
}

/// A deterministic page query for glossary terms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryListQuery {
    /// Exact locale bound to the page and continuation.
    pub locale: ContentQueryLocale,
    /// Visibility scope.
    pub scope: GlossaryQueryScope,
    /// Maximum summaries in this page.
    pub limit: usize,
    /// Opaque single-use continuation.
    pub continuation: Option<GlossaryContinuation>,
}

/// A deterministic literal localized-text search query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossarySearchQuery {
    /// Exact locale bound to the page and continuation.
    pub locale: ContentQueryLocale,
    /// Literal UTF-8 search text; empty returns identity order.
    pub literal: String,
    /// Visibility scope.
    pub scope: GlossaryQueryScope,
    /// Maximum results in this page.
    pub limit: usize,
    /// Opaque single-use continuation.
    pub continuation: Option<GlossaryContinuation>,
}

/// Manifest/locale/scope/query witness attached to a page cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryQueryBinding {
    /// Operation that produced the page.
    pub operation: GlossaryQueryOperation,
    /// Immutable glossary catalog witness.
    pub catalog: GlossaryCatalogBinding,
    /// Exact query locale.
    pub locale: ContentQueryLocale,
    /// Visibility scope.
    pub scope: GlossaryQueryScope,
    /// Literal search text, empty for list.
    pub literal: String,
    /// Page size.
    pub limit: usize,
}

/// Whether more entries remain for this bound query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryCompleteness {
    /// No entries remain.
    Complete,
    /// A continuation contains the remaining entries.
    Partial,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct GlossaryContinuationScope;

/// Opaque single-use continuation retained by one reader.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GlossaryContinuation {
    pub(crate) token: String,
    pub(crate) scope: Arc<GlossaryContinuationScope>,
}

impl GlossaryContinuation {
    pub(crate) fn scoped(token: impl Into<String>, scope: Arc<GlossaryContinuationScope>) -> Self {
        Self {
            token: token.into(),
            scope,
        }
    }

    pub(crate) fn scope_matches(&self, scope: &Arc<GlossaryContinuationScope>) -> bool {
        Arc::ptr_eq(&self.scope, scope)
    }

    /// Returns the opaque token for deterministic diagnostics.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// One ranked literal search match.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossarySearchResult {
    /// Matched term summary.
    pub summary: GlossaryTermSummary,
    /// Lower ranks sort first; ties use term ID.
    pub rank: u8,
}

/// One bounded list page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryListPage {
    /// Query identity and locale fence.
    pub binding: GlossaryQueryBinding,
    /// Deterministically ordered term summaries.
    pub entries: Vec<GlossaryTermSummary>,
    /// Total visible entries for this immutable query.
    pub total: usize,
    /// Complete or partial page state.
    pub completeness: GlossaryCompleteness,
    /// Continuation when the page is partial.
    pub continuation: Option<GlossaryContinuation>,
}

/// One bounded search page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossarySearchPage {
    /// Query identity and locale fence.
    pub binding: GlossaryQueryBinding,
    /// Deterministically ranked term summaries.
    pub entries: Vec<GlossarySearchResult>,
    /// Total visible matches for this immutable query.
    pub total: usize,
    /// Complete or partial page state.
    pub completeness: GlossaryCompleteness,
    /// Continuation when the page is partial.
    pub continuation: Option<GlossaryContinuation>,
}
