// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::ContentCursorBinding;

use super::errors::validate_text;
use super::model::{
    ContentDefinitionSummary, ContentIndexInputError, ContentQueryLocale, ContentRarity,
    ContentUnlockState, validate_identity,
};

pub use super::errors::ContentIndexError;
pub use super::model::ContentIndexDefinitionInput;

/// The two local query operations whose bindings are cursor-scoped.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentQueryOperation {
    /// Enumerate visible definition summaries.
    List,
    /// Search visible localized text literally.
    Search,
}

/// Visibility requested by a local caller.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentQueryScope {
    /// Only definitions observed as unlocked are visible.
    Public,
    /// References may include locked definitions when policy allows them.
    Reference,
}

/// Producer policy for references to locked definitions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentReferenceVisibilityPolicy {
    /// Locked and unknown definitions remain excluded in every scope.
    UnlockedOnly,
    /// Locked definitions may be returned in reference scope; unknown remains excluded.
    AllowLockedReferences,
}

/// Filters recognized by the local query engine.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentFilterKind {
    /// Filter by owner-defined family.
    Kind,
    /// Filter by manifest origin package.
    OriginPackage,
    /// Filter by character or pool when the adapter defines it.
    CharacterOrPool,
    /// Filter by owner-defined rarity when the adapter defines it.
    Rarity,
    /// Filter by observed unlock state when the adapter defines it.
    UnlockState,
}

/// Typed filter values shared by list and search.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContentQueryFilters {
    /// Optional family filter.
    pub entity_kind: Option<String>,
    /// Optional manifest origin package filter.
    pub origin_package: Option<String>,
    /// Optional character/pool filter.
    pub character_or_pool: Option<String>,
    /// Optional owner-defined rarity filter.
    pub rarity: Option<ContentRarity>,
    /// Optional unlock-state filter.
    pub unlock_state: Option<ContentUnlockState>,
}

/// A page query for deterministic enumeration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentListQuery {
    /// Exact locale bound to the page and its continuation.
    pub locale: ContentQueryLocale,
    /// Visibility scope.
    pub scope: ContentQueryScope,
    /// Typed filters.
    pub filters: ContentQueryFilters,
    /// Maximum summaries in the page.
    pub limit: usize,
    /// Opaque continuation returned by an earlier page.
    pub continuation: Option<ContentContinuation>,
}

/// A page query for literal localized-text search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentSearchQuery {
    /// Exact locale bound to the page and its continuation.
    pub locale: ContentQueryLocale,
    /// Literal UTF-8 query; empty returns all visible summaries in identity order.
    pub literal: String,
    /// Visibility scope.
    pub scope: ContentQueryScope,
    /// Typed filters.
    pub filters: ContentQueryFilters,
    /// Maximum results in the page.
    pub limit: usize,
    /// Opaque continuation returned by an earlier page.
    pub continuation: Option<ContentContinuation>,
}

/// A manifest/query/locale/scope witness attached to a page cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentQueryBinding {
    /// Operation that produced the page.
    pub operation: ContentQueryOperation,
    /// Immutable manifest witness.
    pub manifest: ContentCursorBinding,
    /// Exact selected locale.
    pub locale: ContentQueryLocale,
    /// Visibility scope.
    pub scope: ContentQueryScope,
    /// Filters used for the page.
    pub filters: ContentQueryFilters,
    /// Literal query (empty for list).
    pub literal: String,
    /// Page size; changing it cannot reuse the cursor.
    pub limit: usize,
}

/// Whether one page is final or has an explicit continuation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentCompleteness {
    /// No entries remain for this bound query.
    Complete,
    /// More entries remain and a continuation is present.
    Partial,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ContentContinuationScope;

/// Opaque single-use local continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentContinuation {
    pub(crate) token: String,
    pub(crate) scope: Arc<ContentContinuationScope>,
}

impl ContentContinuation {
    pub(crate) fn scoped(token: impl Into<String>, scope: Arc<ContentContinuationScope>) -> Self {
        Self {
            token: token.into(),
            scope,
        }
    }

    pub(crate) fn scope_matches(&self, scope: &Arc<ContentContinuationScope>) -> bool {
        Arc::ptr_eq(&self.scope, scope)
    }

    /// Returns the opaque token for diagnostics and deterministic fixture assertions.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// A bounded list page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentListPage {
    /// Query binding for this page.
    pub binding: ContentQueryBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<ContentDefinitionSummary>,
    /// Total visible entries when known from the immutable index.
    pub total: usize,
    /// Complete or partial page state.
    pub completeness: ContentCompleteness,
    /// Present only when the page is partial.
    pub continuation: Option<ContentContinuation>,
}

/// A search match with its deterministic rank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentSearchResult {
    /// Matched summary.
    pub summary: ContentDefinitionSummary,
    /// Lower ranks sort first; ties use kind then namespaced ID.
    pub rank: u8,
}

/// A bounded search page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentSearchPage {
    /// Query binding for this page.
    pub binding: ContentQueryBinding,
    /// Deterministically ranked matches.
    pub entries: Vec<ContentSearchResult>,
    /// Total visible matches when known from the immutable index.
    pub total: usize,
    /// Complete or partial page state.
    pub completeness: ContentCompleteness,
    /// Present only when the page is partial.
    pub continuation: Option<ContentContinuation>,
}

impl ContentIndexDefinitionInput {
    /// Validates a source record before it is merged with a manifest definition.
    pub(crate) fn validate(&self) -> Result<(), ContentIndexError> {
        validate_identity(&self.entity_kind, "entity_kind").map_err(|error| match error {
            ContentIndexInputError::InvalidIdentity(field) => {
                ContentIndexError::InvalidIdentity(field)
            }
        })?;
        validate_identity(&self.namespaced_id, "namespaced_id").map_err(|error| match error {
            ContentIndexInputError::InvalidIdentity(field) => {
                ContentIndexError::InvalidIdentity(field)
            }
        })?;
        if let Some(display_name) = &self.display_name {
            validate_text(display_name)?;
        }
        let mut aliases = std::collections::BTreeSet::new();
        for alias in &self.aliases {
            if alias.is_empty() {
                return Err(ContentIndexError::InvalidText);
            }
            validate_text(alias)?;
            if !aliases.insert(alias) {
                return Err(ContentIndexError::DuplicateAlias);
            }
        }
        if let Some(description) = &self.rendered_description {
            validate_text(description)?;
        }
        if let Some(character_or_pool) = &self.character_or_pool {
            validate_identity(character_or_pool, "character_or_pool").map_err(
                |error| match error {
                    ContentIndexInputError::InvalidIdentity(field) => {
                        ContentIndexError::InvalidIdentity(field)
                    }
                },
            )?;
        }
        Ok(())
    }
}

impl ContentQueryLocale {
    /// Creates a list query with no filters and no continuation.
    pub fn list_query(self, scope: ContentQueryScope, limit: usize) -> ContentListQuery {
        ContentListQuery {
            locale: self,
            scope,
            filters: ContentQueryFilters::default(),
            limit,
            continuation: None,
        }
    }

    /// Creates a literal search query with no filters and no continuation.
    pub fn search_query(
        self,
        literal: impl Into<String>,
        scope: ContentQueryScope,
        limit: usize,
    ) -> ContentSearchQuery {
        ContentSearchQuery {
            locale: self,
            literal: literal.into(),
            scope,
            filters: ContentQueryFilters::default(),
            limit,
            continuation: None,
        }
    }
}
