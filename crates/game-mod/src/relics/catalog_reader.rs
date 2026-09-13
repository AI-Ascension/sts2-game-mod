// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    definition::{RelicDefinition, RelicFamilyCoverage, RelicFamilyState, RelicVisibilityScope},
    error::RelicCatalogError,
    model::{RelicCatalogBinding, RelicDefinitionReference, RelicVisibility},
};

/// Immutable relic definitions keyed by namespaced definition ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicCatalog {
    pub(super) binding: RelicCatalogBinding,
    pub(super) family: RelicFamilyCoverage,
    pub(super) definitions: BTreeMap<String, RelicDefinition>,
}

impl RelicCatalog {
    pub(super) fn from_parts(
        binding: RelicCatalogBinding,
        family: RelicFamilyCoverage,
        definitions: BTreeMap<String, RelicDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the catalog identity fence.
    #[must_use]
    pub fn binding(&self) -> &RelicCatalogBinding {
        &self.binding
    }

    /// Returns explicit relic-family support coverage.
    #[must_use]
    pub fn family(&self) -> &RelicFamilyCoverage {
        &self.family
    }

    /// Returns an immutable reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> RelicCatalogReader {
        RelicCatalogReader::new(self.clone())
    }

    /// Performs one exact definition lookup.
    pub fn get(
        &self,
        reference: &RelicDefinitionReference,
        scope: RelicVisibilityScope,
    ) -> Result<RelicDefinition, RelicCatalogError> {
        self.reader().get(reference, scope)
    }

    pub(super) fn definition(&self, relic_id: &str) -> Option<&RelicDefinition> {
        self.definitions.get(relic_id)
    }
}

/// Typed summary returned by a bounded definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicDefinitionSummary {
    /// Exact static definition reference.
    pub reference: RelicDefinitionReference,
    /// Localized title.
    pub title: String,
    /// Owner-defined rarity.
    pub rarity: super::model::RelicRarity,
    /// Owner-defined tier.
    pub tier: super::model::RelicTier,
    /// Optional acquisition pool.
    pub pool: Option<super::model::RelicPool>,
    /// Explicit unlock observation.
    pub unlock_state: crate::ContentUnlockState,
}

/// Bounded definition page request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicListQuery {
    /// Visibility scope.
    pub scope: RelicVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RelicContinuation>,
}

/// Complete or partial definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: RelicCatalogBinding,
    /// Deterministically ordered entries.
    pub entries: Vec<RelicDefinitionSummary>,
    /// Number of visible entries.
    pub total: usize,
    /// Whether more entries remain.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RelicContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use catalog continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelicContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl RelicContinuation {
    fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Clone, Debug)]
struct CursorState {
    binding: RelicCatalogBinding,
    scope: RelicVisibilityScope,
    limit: usize,
    offset: usize,
}

/// Reader retaining one catalog while enforcing scope and cursor fences.
#[derive(Clone, Debug)]
pub struct RelicCatalogReader {
    catalog: RelicCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl RelicCatalogReader {
    fn new(catalog: RelicCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Lists visible definitions in namespaced identity order.
    pub fn list(
        &mut self,
        query: &RelicListQuery,
    ) -> Result<RelicDefinitionPage, RelicCatalogError> {
        if query.limit == 0 || query.limit > super::model::RELIC_MAX_PAGE_ITEMS {
            return Err(RelicCatalogError::InvalidPageSize);
        }
        match self.catalog.family.state {
            RelicFamilyState::Handled => {}
            RelicFamilyState::Unsupported => return Err(RelicCatalogError::UnsupportedFamily),
            RelicFamilyState::Unavailable => return Err(RelicCatalogError::UnavailableFamily),
        }
        let start = self.cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible(definition, query.scope))
            .collect::<Vec<_>>();
        let end = start.saturating_add(query.limit).min(entries.len());
        let page_entries = entries[start..end]
            .iter()
            .map(|definition| RelicDefinitionSummary {
                reference: definition.reference.clone(),
                title: definition.title.clone(),
                rarity: definition.rarity.clone(),
                tier: definition.tier.clone(),
                pool: definition.pool.clone(),
                unlock_state: definition.acquisition.unlock.state,
            })
            .collect::<Vec<_>>();
        let continuation = if end < entries.len() {
            let token = format!("relic-cursor-{:08}", self.next_cursor);
            self.next_cursor = self.next_cursor.saturating_add(1);
            self.cursors.insert(
                token.clone(),
                CursorState {
                    binding: self.catalog.binding.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(RelicContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(RelicDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total: entries.len(),
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact lookup with an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RelicDefinitionReference,
        scope: RelicVisibilityScope,
    ) -> Result<RelicDefinition, RelicCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(RelicCatalogError::StaleReference);
        }
        match self.catalog.family.state {
            RelicFamilyState::Handled => {}
            RelicFamilyState::Unsupported => return Err(RelicCatalogError::UnsupportedFamily),
            RelicFamilyState::Unavailable => return Err(RelicCatalogError::UnavailableFamily),
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.relic_id)
            .ok_or(RelicCatalogError::NotFound)?;
        if !visible(definition, scope) {
            return Err(RelicCatalogError::ExcludedByScope);
        }
        if !definition_fields_visible(definition, scope) {
            return Err(RelicCatalogError::ExcludedByScope);
        }
        Ok(definition.clone())
    }

    fn cursor_start(&mut self, query: &RelicListQuery) -> Result<usize, RelicCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(RelicCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(RelicCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RelicCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }
}

fn visible(definition: &RelicDefinition, scope: RelicVisibilityScope) -> bool {
    match definition.acquisition.unlock.state {
        crate::ContentUnlockState::Unlocked => true,
        crate::ContentUnlockState::Locked => {
            matches!(
                scope,
                RelicVisibilityScope::Reference | RelicVisibilityScope::Owner
            )
        }
        crate::ContentUnlockState::Unknown => false,
    }
}

fn definition_fields_visible(definition: &RelicDefinition, scope: RelicVisibilityScope) -> bool {
    definition
        .parameters
        .iter()
        .all(|parameter| visibility_allowed(parameter.visibility, scope))
        && definition
            .counters
            .iter()
            .all(|counter| visibility_allowed(counter.visibility, scope))
        && definition
            .triggers
            .iter()
            .all(|trigger| visibility_allowed(trigger.visibility, scope))
}

fn visibility_allowed(visibility: RelicVisibility, scope: RelicVisibilityScope) -> bool {
    match visibility {
        RelicVisibility::Visible => true,
        RelicVisibility::OwnerOnly => matches!(scope, RelicVisibilityScope::Owner),
        RelicVisibility::Hidden | RelicVisibility::Unknown => false,
    }
}
