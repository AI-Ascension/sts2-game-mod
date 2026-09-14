// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use crate::ContentUnlockState;

use super::{
    ENEMY_MAX_PAGE_ITEMS, EnemyCatalogBinding, EnemyCatalogError, EnemyDefinition,
    EnemyDefinitionReference, EnemyFamilyCoverage, EnemyFamilyState, EnemyFieldStatus,
    EnemyMoveDefinition, EnemyMoveReference, EnemyVisibility, EnemyVisibilityScope,
};

/// Immutable enemy definitions keyed by namespaced enemy identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyCatalog {
    pub(super) binding: EnemyCatalogBinding,
    pub(super) family: EnemyFamilyCoverage,
    pub(super) definitions: BTreeMap<String, EnemyDefinition>,
}

impl EnemyCatalog {
    pub(super) fn from_parts(
        binding: EnemyCatalogBinding,
        family: EnemyFamilyCoverage,
        definitions: BTreeMap<String, EnemyDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &EnemyCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the enemy family.
    #[must_use]
    pub fn family(&self) -> &EnemyFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> EnemyCatalogReader {
        EnemyCatalogReader::new(self.clone())
    }

    /// Performs one exact enemy lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &EnemyDefinitionReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyDefinition, EnemyCatalogError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact move lookup under an explicit visibility scope.
    pub fn get_move(
        &self,
        reference: &EnemyMoveReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyMoveDefinition, EnemyCatalogError> {
        self.reader().get_move(reference, scope)
    }
}

/// Typed summary returned by one bounded enemy definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinitionSummary {
    /// Exact static definition reference.
    pub reference: EnemyDefinitionReference,
    /// Localized enemy name or explicit unavailable state.
    pub name: super::EnemyText,
    /// Enemy role.
    pub kind: super::EnemyKind,
    /// Explicit unlock state.
    pub unlock_state: ContentUnlockState,
    /// Number of source-owned moves.
    pub move_count: usize,
    /// Number of behavior phases.
    pub phase_count: usize,
    /// Number of stable tags.
    pub tag_count: usize,
    /// Availability of spawn conditions.
    pub spawn_conditions: EnemyFieldStatus,
    /// Availability of encounter references.
    pub encounters: EnemyFieldStatus,
}

/// Bounded enemy page request.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: EnemyVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EnemyContinuation>,
}

/// Complete or partial enemy definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: EnemyCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<EnemyDefinitionSummary>,
    /// Number of visible definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EnemyContinuation>,
}

/// Bounded move page request scoped to one exact enemy definition.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyMoveListQuery {
    /// Exact enemy definition whose moves are listed.
    pub enemy: EnemyDefinitionReference,
    /// Visibility scope.
    pub scope: EnemyVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EnemyMoveContinuation>,
}

/// Summary returned by one bounded move page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyMoveSummary {
    /// Exact static move reference.
    pub reference: EnemyMoveReference,
    /// Localized move name.
    pub name: super::EnemyText,
    /// Number of ordered effects.
    pub effect_count: usize,
    /// Phase identities in which the move is available.
    pub phase_ids: Vec<String>,
    /// Selection probability/weight with explicit evidence.
    pub probability: super::EnemyProbability,
}

/// Complete or partial move page.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyMoveDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: EnemyCatalogBinding,
    /// Deterministically ordered move summaries.
    pub entries: Vec<EnemyMoveSummary>,
    /// Number of visible moves.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EnemyMoveContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ContinuationScope;

/// Opaque single-use enemy continuation.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl EnemyContinuation {
    fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use move continuation.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyMoveContinuation {
    token: String,
    scope: Arc<ContinuationScope>,
}

impl EnemyMoveContinuation {
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
struct EnemyCursorState {
    binding: EnemyCatalogBinding,
    locale: String,
    scope: EnemyVisibilityScope,
    limit: usize,
    offset: usize,
}

#[derive(Clone, Debug)]
struct MoveCursorState {
    binding: EnemyCatalogBinding,
    enemy_id: String,
    scope: EnemyVisibilityScope,
    limit: usize,
    offset: usize,
}

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
    fn new(catalog: EnemyCatalog) -> Self {
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

fn visible_definition(definition: &EnemyDefinition, scope: EnemyVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => {
            matches!(
                scope,
                EnemyVisibilityScope::Reference | EnemyVisibilityScope::Owner
            )
        }
        ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visible_move(movement: &EnemyMoveDefinition, scope: EnemyVisibilityScope) -> bool {
    visibility_allowed(movement.visibility, scope)
}

fn visibility_allowed(visibility: EnemyVisibility, scope: EnemyVisibilityScope) -> bool {
    match visibility {
        EnemyVisibility::Visible => true,
        EnemyVisibility::OwnerOnly => matches!(scope, EnemyVisibilityScope::Owner),
        EnemyVisibility::Hidden | EnemyVisibility::Unknown => false,
    }
}
