// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::{
    ENEMY_MAX_PAGE_ITEMS, EnemyCatalogBinding, EnemyCatalogError, EnemyDefinition,
    EnemyDefinitionReference, EnemyFamilyState, EnemyMoveDefinition, EnemyMoveReference,
    EnemyVisibilityScope,
};
use super::catalog::EnemyCatalog;
use super::page::{
    ContinuationScope, EnemyContinuation, EnemyCursorState, EnemyDefinitionPage,
    EnemyDefinitionSummary, EnemyListQuery, EnemyMoveContinuation, EnemyMoveDefinitionPage,
    EnemyMoveListQuery, EnemyMoveSummary, MoveCursorState,
};
use super::{visible_definition, visible_move};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registries are mutable and continuations
/// are single-use. Call [`EnemyCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct EnemyCatalogReader {
    catalog: EnemyCatalog,
    enemy_cursors: BTreeMap<String, EnemyCursorState>,
    move_cursors: BTreeMap<String, MoveCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl EnemyCatalogReader {
    pub(super) fn new(catalog: EnemyCatalog) -> Self {
        Self {
            catalog,
            enemy_cursors: BTreeMap::new(),
            move_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &EnemyCatalog {
        &self.catalog
    }

    /// Lists visible enemy definitions in stable enemy-ID order.
    pub fn list(
        &mut self,
        query: &EnemyListQuery,
    ) -> Result<EnemyDefinitionPage, EnemyCatalogError> {
        if query.locale != self.catalog.binding.locale {
            return Err(EnemyCatalogError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > ENEMY_MAX_PAGE_ITEMS {
            return Err(EnemyCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.enemy_cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible_definition(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end]
            .iter()
            .map(|definition| EnemyDefinitionSummary {
                reference: definition.reference.clone(),
                name: definition.name.clone(),
                kind: definition.kind.clone(),
                unlock_state: definition.unlock_state,
                move_count: definition.moves.len(),
                phase_count: definition.phases.len(),
                tag_count: definition.tags.len(),
                spawn_conditions: definition.spawn_conditions.status(),
                encounters: definition.encounters.status(),
            })
            .collect::<Vec<_>>();
        let continuation = if end < total {
            let token = self.make_token("enemy-cursor");
            self.enemy_cursors.insert(
                token.clone(),
                EnemyCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(EnemyContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(EnemyDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible moves for one exact enemy definition.
    pub fn list_moves(
        &mut self,
        query: &EnemyMoveListQuery,
    ) -> Result<EnemyMoveDefinitionPage, EnemyCatalogError> {
        if query.limit == 0 || query.limit > ENEMY_MAX_PAGE_ITEMS {
            return Err(EnemyCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.move_cursor_start(query)?;
        let page_entries_all = self
            .definition_for_reference(&query.enemy, query.scope)?
            .moves
            .iter()
            .filter(|movement| visible_move(movement, query.scope))
            .map(|movement| EnemyMoveSummary {
                reference: movement.reference.clone(),
                name: movement.name.clone(),
                effect_count: movement.effects.len(),
                phase_ids: movement.phase_ids.clone(),
                probability: movement.probability.clone(),
            })
            .collect::<Vec<_>>();
        let total = page_entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = page_entries_all[start..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("enemy-move-cursor");
            self.move_cursors.insert(
                token.clone(),
                MoveCursorState {
                    binding: self.catalog.binding.clone(),
                    enemy_id: query.enemy.enemy_id.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(EnemyMoveContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(EnemyMoveDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact enemy lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &EnemyDefinitionReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyDefinition, EnemyCatalogError> {
        self.validate_family()?;
        Ok(self.definition_for_reference(reference, scope)?.clone())
    }

    /// Performs one exact move lookup under an explicit visibility scope.
    pub fn get_move(
        &self,
        reference: &EnemyMoveReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyMoveDefinition, EnemyCatalogError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.enemy_id, scope)?;
        if let Some(movement) = definition
            .moves
            .iter()
            .find(|movement| movement.reference.move_id == reference.move_id)
        {
            if visible_move(movement, scope) {
                return Ok(movement.clone());
            }
            return Err(EnemyCatalogError::ExcludedByScope);
        }
        Err(EnemyCatalogError::NotFound)
    }

    fn validate_family(&self) -> Result<(), EnemyCatalogError> {
        match self.catalog.family.state {
            EnemyFamilyState::Handled => Ok(()),
            EnemyFamilyState::Unsupported => Err(EnemyCatalogError::UnsupportedFamily),
            EnemyFamilyState::Unavailable => Err(EnemyCatalogError::UnavailableFamily),
        }
    }

    fn definition_for_reference(
        &self,
        reference: &EnemyDefinitionReference,
        scope: EnemyVisibilityScope,
    ) -> Result<&EnemyDefinition, EnemyCatalogError> {
        self.definition_for_identity(&reference.catalog, &reference.enemy_id, scope)
    }

    fn definition_for_identity(
        &self,
        binding: &EnemyCatalogBinding,
        enemy_id: &str,
        scope: EnemyVisibilityScope,
    ) -> Result<&EnemyDefinition, EnemyCatalogError> {
        if binding != &self.catalog.binding {
            return Err(EnemyCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(enemy_id)
            .ok_or(EnemyCatalogError::NotFound)?;
        if !visible_definition(definition, scope) {
            return Err(EnemyCatalogError::ExcludedByScope);
        }
        Ok(definition)
    }

    fn enemy_cursor_start(&mut self, query: &EnemyListQuery) -> Result<usize, EnemyCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(EnemyCatalogError::InvalidContinuation);
        }
        let cursor = self
            .enemy_cursors
            .remove(continuation.token())
            .ok_or(EnemyCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(EnemyCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn move_cursor_start(
        &mut self,
        query: &EnemyMoveListQuery,
    ) -> Result<usize, EnemyCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(EnemyCatalogError::InvalidContinuation);
        }
        let cursor = self
            .move_cursors
            .remove(continuation.token())
            .ok_or(EnemyCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.enemy_id != query.enemy.enemy_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(EnemyCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}
