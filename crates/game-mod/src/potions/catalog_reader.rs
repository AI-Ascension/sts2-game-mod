// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    definition::{PotionDefinition, PotionFamilyCoverage, PotionFamilyState},
    error::PotionCatalogError,
    model::{
        POTION_MAX_PAGE_ITEMS, PotionCatalogBinding, PotionDefinitionReference, PotionTargetMode,
        PotionUseRule, PotionVisibility, PotionVisibilityScope,
    },
};

/// Immutable potion definitions keyed by namespaced definition ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionCatalog {
    pub(super) binding: PotionCatalogBinding,
    pub(super) family: PotionFamilyCoverage,
    pub(super) definitions: BTreeMap<String, PotionDefinition>,
}

impl PotionCatalog {
    pub(super) fn from_parts(
        binding: PotionCatalogBinding,
        family: PotionFamilyCoverage,
        definitions: BTreeMap<String, PotionDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the catalog identity fence.
    #[must_use]
    pub fn binding(&self) -> &PotionCatalogBinding {
        &self.binding
    }

    /// Returns explicit potion-family support coverage.
    #[must_use]
    pub fn family(&self) -> &PotionFamilyCoverage {
        &self.family
    }

    /// Returns an immutable reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> PotionCatalogReader {
        PotionCatalogReader::new(self.clone())
    }

    /// Performs one exact definition lookup.
    pub fn get(
        &self,
        reference: &PotionDefinitionReference,
        scope: PotionVisibilityScope,
    ) -> Result<PotionDefinition, PotionCatalogError> {
        self.reader().get(reference, scope)
    }

    pub(super) fn definition(&self, potion_id: &str) -> Option<&PotionDefinition> {
        self.definitions.get(potion_id)
    }
}

/// Typed summary returned by a bounded definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionDefinitionSummary {
    /// Exact static definition reference.
    pub reference: PotionDefinitionReference,
    /// Localized title.
    pub title: String,
    /// Owner-defined rarity.
    pub rarity: super::model::PotionRarity,
    /// Optional acquisition pool.
    pub pool: Option<super::model::PotionPool>,
    /// Static target mode.
    pub target_mode: PotionTargetMode,
    /// Static use rule.
    pub use_rule: PotionUseRule,
    /// Explicit acquisition/unlock observation.
    pub unlock_state: crate::ContentUnlockState,
}

/// Bounded definition page request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionListQuery {
    /// Visibility scope.
    pub scope: PotionVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<PotionContinuation>,
}

/// Complete or partial definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: PotionCatalogBinding,
    /// Deterministically ordered entries.
    pub entries: Vec<PotionDefinitionSummary>,
    /// Number of visible entries.
    pub total: usize,
    /// Whether more entries remain.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<PotionContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use catalog continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl PotionContinuation {
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
    binding: PotionCatalogBinding,
    scope: PotionVisibilityScope,
    limit: usize,
    offset: usize,
}

/// Reader retaining one catalog while enforcing scope and cursor fences.
#[derive(Clone, Debug)]
pub struct PotionCatalogReader {
    catalog: PotionCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl PotionCatalogReader {
    fn new(catalog: PotionCatalog) -> Self {
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
        query: &PotionListQuery,
    ) -> Result<PotionDefinitionPage, PotionCatalogError> {
        if query.limit == 0 || query.limit > POTION_MAX_PAGE_ITEMS {
            return Err(PotionCatalogError::InvalidPageSize);
        }
        match self.catalog.family.state {
            PotionFamilyState::Handled => {}
            PotionFamilyState::Unsupported => return Err(PotionCatalogError::UnsupportedFamily),
            PotionFamilyState::Unavailable => return Err(PotionCatalogError::UnavailableFamily),
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
            .map(|definition| PotionDefinitionSummary {
                reference: definition.reference.clone(),
                title: definition.title.clone(),
                rarity: definition.rarity.clone(),
                pool: definition.pool.clone(),
                target_mode: definition.target_mode,
                use_rule: definition.use_rule.clone(),
                unlock_state: definition.acquisition.unlock.state,
            })
            .collect::<Vec<_>>();
        let continuation = if end < entries.len() {
            let token = format!("potion-cursor-{:08}", self.next_cursor);
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
            Some(PotionContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(PotionDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total: entries.len(),
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact lookup with an explicit visibility authorization.
    pub fn get(
        &self,
        reference: &PotionDefinitionReference,
        scope: PotionVisibilityScope,
    ) -> Result<PotionDefinition, PotionCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(PotionCatalogError::StaleReference);
        }
        match self.catalog.family.state {
            PotionFamilyState::Handled => {}
            PotionFamilyState::Unsupported => return Err(PotionCatalogError::UnsupportedFamily),
            PotionFamilyState::Unavailable => return Err(PotionCatalogError::UnavailableFamily),
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.potion_id)
            .ok_or(PotionCatalogError::NotFound)?;
        if !visible(definition, scope) || !definition_fields_visible(definition, scope) {
            return Err(PotionCatalogError::ExcludedByScope);
        }
        Ok(definition.clone())
    }

    fn cursor_start(&mut self, query: &PotionListQuery) -> Result<usize, PotionCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(PotionCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(PotionCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(PotionCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }
}

fn visible(definition: &PotionDefinition, scope: PotionVisibilityScope) -> bool {
    match definition.acquisition.unlock.state {
        crate::ContentUnlockState::Unlocked => true,
        crate::ContentUnlockState::Locked => {
            matches!(
                scope,
                PotionVisibilityScope::Reference | PotionVisibilityScope::Owner
            )
        }
        crate::ContentUnlockState::Unknown => false,
    }
}

fn definition_fields_visible(definition: &PotionDefinition, scope: PotionVisibilityScope) -> bool {
    definition
        .effects
        .iter()
        .all(|effect| visibility_allowed(effect.visibility, scope))
        && definition
            .parameters
            .iter()
            .all(|parameter| visibility_allowed(parameter.visibility, scope))
}

fn visibility_allowed(visibility: PotionVisibility, scope: PotionVisibilityScope) -> bool {
    match visibility {
        PotionVisibility::Visible => true,
        PotionVisibility::OwnerOnly => matches!(scope, PotionVisibilityScope::Owner),
        PotionVisibility::Hidden | PotionVisibility::Unknown => false,
    }
}
