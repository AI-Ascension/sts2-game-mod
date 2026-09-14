// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    catalog::CharacterStateCatalog,
    definition::{
        CharacterResourceDefinition, CharacterResourceDefinitionReference,
        SecondaryEntityDefinition, SecondaryEntityDefinitionReference,
    },
    error::CharacterStateCatalogError,
    model::{
        CHARACTER_STATE_MAX_DEFINITION_BYTES, CHARACTER_STATE_MAX_PAGE_ITEMS,
        CharacterStateCatalogBinding, CharacterStateVisibilityScope,
    },
    reader_support,
    sizes::{resource_definition_bytes, secondary_definition_bytes},
};

/// Static definition family selected by a bounded list query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterStateDefinitionKind {
    /// Character-specific resource definition.
    Resource,
    /// Controlled secondary-entity definition.
    SecondaryEntity,
}

/// Bounded static definition query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateListQuery {
    /// Definition family.
    pub kind: CharacterStateDefinitionKind,
    /// Optional character filter.
    pub character_id: Option<String>,
    /// Optional mode filter.
    pub mode_id: Option<String>,
    /// Visibility scope.
    pub scope: CharacterStateVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<CharacterStateContinuation>,
}

/// Summary returned by a bounded static definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateDefinitionSummary {
    /// Definition family.
    pub kind: CharacterStateDefinitionKind,
    /// Static definition identity.
    pub definition_id: String,
    /// Character identity.
    pub character_id: String,
    /// Mode identity.
    pub mode_id: String,
    /// Localized label.
    pub label: String,
}

/// Complete or partial static definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: CharacterStateCatalogBinding,
    /// Deterministically ordered entries.
    pub entries: Vec<CharacterStateDefinitionSummary>,
    /// Exact visible count.
    pub total: usize,
    /// Whether every visible entry was returned.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<CharacterStateContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use catalog continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterStateContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl CharacterStateContinuation {
    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Clone, Debug)]
struct CursorState {
    binding: CharacterStateCatalogBinding,
    query: CharacterStateListQueryKey,
    offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CharacterStateListQueryKey {
    kind: CharacterStateDefinitionKind,
    character_id: Option<String>,
    mode_id: Option<String>,
    scope: CharacterStateVisibilityScope,
}

/// Reader retaining one catalog while enforcing scope and cursor fences.
#[derive(Clone, Debug)]
pub struct CharacterStateCatalogReader {
    catalog: CharacterStateCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl CharacterStateCatalogReader {
    pub(super) fn new(catalog: CharacterStateCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Lists visible static definitions in deterministic identity order.
    pub fn list(
        &mut self,
        query: &CharacterStateListQuery,
    ) -> Result<CharacterStateDefinitionPage, CharacterStateCatalogError> {
        if query.limit == 0 || query.limit > CHARACTER_STATE_MAX_PAGE_ITEMS {
            return Err(CharacterStateCatalogError::InvalidPageSize);
        }
        reader_support::ensure_query_coverage(&self.catalog, query)?;
        let key = CharacterStateListQueryKey {
            kind: query.kind,
            character_id: query.character_id.clone(),
            mode_id: query.mode_id.clone(),
            scope: query.scope,
        };
        let start = self.cursor_start(query.continuation.as_ref(), &key)?;
        let mut entries = Vec::new();
        match query.kind {
            CharacterStateDefinitionKind::Resource => {
                for definition in self.catalog.resources.values() {
                    if reader_support::matches_filter(
                        &definition.character_id,
                        &definition.mode_id,
                        query.character_id.as_deref(),
                        query.mode_id.as_deref(),
                    ) && reader_support::visible(definition.visibility, query.scope)
                    {
                        entries.push(CharacterStateDefinitionSummary {
                            kind: query.kind,
                            definition_id: definition.reference.definition_id.clone(),
                            character_id: definition.character_id.clone(),
                            mode_id: definition.mode_id.clone(),
                            label: definition.label.clone(),
                        });
                    }
                }
            }
            CharacterStateDefinitionKind::SecondaryEntity => {
                for definition in self.catalog.entities.values() {
                    if reader_support::matches_filter(
                        &definition.character_id,
                        &definition.mode_id,
                        query.character_id.as_deref(),
                        query.mode_id.as_deref(),
                    ) && reader_support::visible(definition.visibility, query.scope)
                    {
                        entries.push(CharacterStateDefinitionSummary {
                            kind: query.kind,
                            definition_id: definition.reference.definition_id.clone(),
                            character_id: definition.character_id.clone(),
                            mode_id: definition.mode_id.clone(),
                            label: definition.label.clone(),
                        });
                    }
                }
            }
        }
        let end = start.saturating_add(query.limit).min(entries.len());
        let continuation = if end < entries.len() {
            let token = format!("character-state-cursor-{:08}", self.next_cursor);
            self.next_cursor = self.next_cursor.saturating_add(1);
            self.cursors.insert(
                token.clone(),
                CursorState {
                    binding: self.catalog.binding.clone(),
                    query: key,
                    offset: end,
                },
            );
            Some(CharacterStateContinuation {
                token,
                scope: Arc::clone(&self.scope),
            })
        } else {
            None
        };
        Ok(CharacterStateDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: entries[start..end].to_vec(),
            total: entries.len(),
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact resource-definition lookup with explicit visibility scope.
    pub fn resource(
        &self,
        reference: &CharacterResourceDefinitionReference,
        scope: CharacterStateVisibilityScope,
    ) -> Result<CharacterResourceDefinition, CharacterStateCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(CharacterStateCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .resources
            .get(&reference.definition_id)
            .ok_or(CharacterStateCatalogError::NotFound)?;
        reader_support::ensure_supported(self.catalog.mechanic_state(
            &definition.character_id,
            &definition.mode_id,
            CharacterStateDefinitionKind::Resource,
        )?)?;
        if !reader_support::visible(definition.visibility, scope) {
            return Err(CharacterStateCatalogError::ExcludedByScope);
        }
        let detail_bytes = resource_definition_bytes(definition);
        if detail_bytes > CHARACTER_STATE_MAX_DEFINITION_BYTES {
            return Err(CharacterStateCatalogError::DefinitionTooLarge {
                limit: CHARACTER_STATE_MAX_DEFINITION_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(definition.clone())
    }

    /// Performs one exact secondary-entity lookup with explicit visibility scope.
    pub fn secondary_entity(
        &self,
        reference: &SecondaryEntityDefinitionReference,
        scope: CharacterStateVisibilityScope,
    ) -> Result<SecondaryEntityDefinition, CharacterStateCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(CharacterStateCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .entities
            .get(&reference.definition_id)
            .ok_or(CharacterStateCatalogError::NotFound)?;
        reader_support::ensure_supported(self.catalog.mechanic_state(
            &definition.character_id,
            &definition.mode_id,
            CharacterStateDefinitionKind::SecondaryEntity,
        )?)?;
        if !reader_support::visible(definition.visibility, scope) {
            return Err(CharacterStateCatalogError::ExcludedByScope);
        }
        let detail_bytes = secondary_definition_bytes(definition);
        if detail_bytes > CHARACTER_STATE_MAX_DEFINITION_BYTES {
            return Err(CharacterStateCatalogError::DefinitionTooLarge {
                limit: CHARACTER_STATE_MAX_DEFINITION_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(definition.clone())
    }

    fn cursor_start(
        &mut self,
        continuation: Option<&CharacterStateContinuation>,
        query: &CharacterStateListQueryKey,
    ) -> Result<usize, CharacterStateCatalogError> {
        let Some(continuation) = continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(CharacterStateCatalogError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(CharacterStateCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding || cursor.query != *query {
            return Err(CharacterStateCatalogError::StaleReference);
        }
        Ok(cursor.offset)
    }
}
