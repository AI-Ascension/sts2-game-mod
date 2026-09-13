// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    definition::{PowerStatusDefinition, PowerStatusFamilyCoverage, PowerStatusFamilyState},
    error::PowerStatusCatalogError,
    model::{
        POWER_STATUS_MAX_PAGE_ITEMS, PowerStatusCatalogBinding, PowerStatusDefinitionReference,
        PowerStatusVisibility, PowerStatusVisibilityScope,
    },
};

/// Immutable power/status definitions keyed by namespaced ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusCatalog {
    pub(super) binding: PowerStatusCatalogBinding,
    pub(super) family: PowerStatusFamilyCoverage,
    pub(super) definitions: BTreeMap<String, PowerStatusDefinition>,
}

impl PowerStatusCatalog {
    pub(super) fn from_parts(
        binding: PowerStatusCatalogBinding,
        family: PowerStatusFamilyCoverage,
        definitions: BTreeMap<String, PowerStatusDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the catalog identity fence.
    #[must_use]
    pub fn binding(&self) -> &PowerStatusCatalogBinding {
        &self.binding
    }

    /// Returns explicit family support coverage.
    #[must_use]
    pub fn family(&self) -> &PowerStatusFamilyCoverage {
        &self.family
    }

    /// Returns an immutable reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> PowerStatusCatalogReader {
        PowerStatusCatalogReader::new(self.clone())
    }

    /// Performs one exact definition lookup.
    pub fn get(
        &self,
        reference: &PowerStatusDefinitionReference,
        scope: PowerStatusVisibilityScope,
    ) -> Result<PowerStatusDefinition, PowerStatusCatalogError> {
        self.reader().get(reference, scope)
    }

    pub(super) fn definition(&self, definition_id: &str) -> Option<&PowerStatusDefinition> {
        self.definitions.get(definition_id)
    }
}

/// Typed summary returned by a bounded definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDefinitionSummary {
    /// Exact static definition reference.
    pub reference: PowerStatusDefinitionReference,
    /// Localized title.
    pub title: String,
    /// Power/status family.
    pub kind: super::model::PowerStatusKind,
    /// Owner-defined category.
    pub category: super::model::PowerStatusCategory,
    /// Explicit unlock observation.
    pub unlock_state: crate::ContentUnlockState,
}

/// Bounded definition page request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusListQuery {
    /// Visibility scope.
    pub scope: PowerStatusVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<PowerStatusContinuation>,
}

/// Complete or partial definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: PowerStatusCatalogBinding,
    /// Deterministically ordered entries.
    pub entries: Vec<PowerStatusDefinitionSummary>,
    /// Number of visible entries.
    pub total: usize,
    /// Whether every visible entry was returned.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<PowerStatusContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use catalog continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl PowerStatusContinuation {
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
    binding: PowerStatusCatalogBinding,
    scope: PowerStatusVisibilityScope,
    limit: usize,
    offset: usize,
}

/// Reader retaining one catalog while enforcing scope and cursor fences.
#[derive(Clone, Debug)]
pub struct PowerStatusCatalogReader {
    catalog: PowerStatusCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl PowerStatusCatalogReader {
    fn new(catalog: PowerStatusCatalog) -> Self {
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
        query: &PowerStatusListQuery,
    ) -> Result<PowerStatusDefinitionPage, PowerStatusCatalogError> {
        if query.limit == 0 || query.limit > POWER_STATUS_MAX_PAGE_ITEMS {
            return Err(PowerStatusCatalogError::InvalidPageSize);
        }
        match self.catalog.family.state {
            PowerStatusFamilyState::Handled => {}
            PowerStatusFamilyState::Unsupported => {
                return Err(PowerStatusCatalogError::UnsupportedFamily);
            }
            PowerStatusFamilyState::Unavailable => {
                return Err(PowerStatusCatalogError::UnavailableFamily);
            }
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
            .map(|definition| PowerStatusDefinitionSummary {
                reference: definition.reference.clone(),
                title: definition.title.clone(),
                kind: definition.kind.clone(),
                category: definition.category.clone(),
                unlock_state: definition.unlock_state,
            })
            .collect::<Vec<_>>();
        let continuation = if end < entries.len() {
            let token = format!("power-status-cursor-{:08}", self.next_cursor);
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
            Some(PowerStatusContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(PowerStatusDefinitionPage {
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
        reference: &PowerStatusDefinitionReference,
        scope: PowerStatusVisibilityScope,
    ) -> Result<PowerStatusDefinition, PowerStatusCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(PowerStatusCatalogError::StaleReference);
        }
        match self.catalog.family.state {
            PowerStatusFamilyState::Handled => {}
            PowerStatusFamilyState::Unsupported => {
                return Err(PowerStatusCatalogError::UnsupportedFamily);
            }
            PowerStatusFamilyState::Unavailable => {
                return Err(PowerStatusCatalogError::UnavailableFamily);
            }
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.definition_id)
            .ok_or(PowerStatusCatalogError::NotFound)?;
        if !visible(definition, scope) {
            return Err(PowerStatusCatalogError::ExcludedByScope);
        }
        Ok(definition.clone())
    }

    fn cursor_start(
        &mut self,
        query: &PowerStatusListQuery,
    ) -> Result<usize, PowerStatusCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(PowerStatusCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(PowerStatusCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(PowerStatusCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }
}

fn visible(definition: &PowerStatusDefinition, scope: PowerStatusVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        crate::ContentUnlockState::Unlocked => true,
        crate::ContentUnlockState::Locked => {
            matches!(scope, PowerStatusVisibilityScope::Owner)
        }
        crate::ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visibility_allowed(
    visibility: PowerStatusVisibility,
    scope: PowerStatusVisibilityScope,
) -> bool {
    match visibility {
        PowerStatusVisibility::Visible => true,
        PowerStatusVisibility::OwnerOnly => matches!(scope, PowerStatusVisibilityScope::Owner),
        PowerStatusVisibility::Hidden | PowerStatusVisibility::Unknown => false,
    }
}
