// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use crate::ContentUnlockState;

use super::{
    CHARACTER_MAX_PAGE_ITEMS, CharacterCatalogBinding, CharacterCatalogError, CharacterDefinition,
    CharacterDefinitionReference, CharacterFamilyCoverage, CharacterFamilyState, CharacterField,
    CharacterFieldStatus, CharacterVisibilityScope,
};

/// Immutable character definitions keyed by namespaced character identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterCatalog {
    pub(super) binding: CharacterCatalogBinding,
    pub(super) family: CharacterFamilyCoverage,
    pub(super) definitions: BTreeMap<String, CharacterDefinition>,
}

impl CharacterCatalog {
    pub(super) fn from_parts(
        binding: CharacterCatalogBinding,
        family: CharacterFamilyCoverage,
        definitions: BTreeMap<String, CharacterDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &CharacterCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the character family.
    #[must_use]
    pub fn family(&self) -> &CharacterFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> CharacterCatalogReader {
        CharacterCatalogReader::new(self.clone())
    }

    /// Performs one exact definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &CharacterDefinitionReference,
        scope: CharacterVisibilityScope,
    ) -> Result<CharacterDefinition, CharacterCatalogError> {
        self.reader().get(reference, scope)
    }
}

/// Typed summary returned by one bounded page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterDefinitionSummary {
    /// Exact static definition reference.
    pub reference: CharacterDefinitionReference,
    /// Localized character name or explicit unavailable state.
    pub name: super::CharacterText,
    /// Explicit unlock state.
    pub unlock_state: ContentUnlockState,
    /// Availability of the unlock record itself.
    pub unlock_availability: CharacterFieldStatus,
    /// Number of mode/loadout variants.
    pub loadout_count: usize,
    /// Number of variants the source marked available.
    pub available_loadout_count: usize,
}

/// Bounded character page request.
#[derive(Debug, Eq, PartialEq)]
pub struct CharacterListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: CharacterVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<CharacterContinuation>,
}

/// Complete or partial character definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct CharacterDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: CharacterCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<CharacterDefinitionSummary>,
    /// Number of visible definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<CharacterContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use catalog continuation.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl CharacterContinuation {
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
    binding: CharacterCatalogBinding,
    locale: String,
    scope: CharacterVisibilityScope,
    limit: usize,
    offset: usize,
}

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`CharacterCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct CharacterCatalogReader {
    catalog: CharacterCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl CharacterCatalogReader {
    pub(super) fn new(catalog: CharacterCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &CharacterCatalog {
        &self.catalog
    }

    /// Lists visible definitions in stable character-ID order.
    pub fn list(
        &mut self,
        query: &CharacterListQuery,
    ) -> Result<CharacterDefinitionPage, CharacterCatalogError> {
        if query.locale != self.catalog.binding.locale {
            return Err(CharacterCatalogError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > CHARACTER_MAX_PAGE_ITEMS {
            return Err(CharacterCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
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
            .map(|definition| CharacterDefinitionSummary {
                reference: definition.reference.clone(),
                name: definition.name.clone(),
                unlock_state: unlock_state(definition),
                unlock_availability: unlock_availability(definition),
                loadout_count: definition.loadouts.len(),
                available_loadout_count: definition
                    .loadouts
                    .iter()
                    .filter(|loadout| {
                        matches!(
                            loadout.availability,
                            super::CharacterLoadoutAvailability::Available
                        )
                    })
                    .count(),
            })
            .collect::<Vec<_>>();
        let total = entries.len();
        let continuation = if end < total {
            let token = format!("character-cursor-{:08}", self.next_cursor);
            self.next_cursor = self.next_cursor.saturating_add(1);
            self.cursors.insert(
                token.clone(),
                CursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(CharacterContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(CharacterDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &CharacterDefinitionReference,
        scope: CharacterVisibilityScope,
    ) -> Result<CharacterDefinition, CharacterCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(CharacterCatalogError::StaleReference);
        }
        self.validate_family()?;
        let definition = self
            .catalog
            .definitions
            .get(&reference.character_id)
            .ok_or(CharacterCatalogError::NotFound)?;
        if !visible(definition, scope) {
            return Err(CharacterCatalogError::ExcludedByScope);
        }
        Ok(definition.clone())
    }

    fn validate_family(&self) -> Result<(), CharacterCatalogError> {
        match self.catalog.family.state {
            CharacterFamilyState::Handled => Ok(()),
            CharacterFamilyState::Unsupported => Err(CharacterCatalogError::UnsupportedFamily),
            CharacterFamilyState::Unavailable => Err(CharacterCatalogError::UnavailableFamily),
        }
    }

    fn cursor_start(&mut self, query: &CharacterListQuery) -> Result<usize, CharacterCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(CharacterCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(CharacterCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(CharacterCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }
}

fn visible(definition: &CharacterDefinition, scope: CharacterVisibilityScope) -> bool {
    let CharacterField::Available(unlock) = &definition.unlock else {
        return false;
    };
    match unlock.state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => matches!(
            scope,
            CharacterVisibilityScope::Reference | CharacterVisibilityScope::Owner
        ),
        ContentUnlockState::Unknown => false,
    }
}

fn unlock_state(definition: &CharacterDefinition) -> ContentUnlockState {
    match &definition.unlock {
        CharacterField::Available(unlock) => unlock.state,
        CharacterField::Unavailable(_) => ContentUnlockState::Unknown,
    }
}

fn unlock_availability(definition: &CharacterDefinition) -> CharacterFieldStatus {
    definition.unlock.status()
}

#[cfg(test)]
#[path = "catalog_reader_tests.rs"]
mod tests;
