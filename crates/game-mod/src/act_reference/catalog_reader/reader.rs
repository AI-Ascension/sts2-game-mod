// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::model::ACT_MAX_PAGE_ITEMS;
use super::super::{
    ActCatalogBinding, ActDefinition, ActDefinitionReference, ActFamilyState, ActFieldStatus,
    ActReferenceError, ActVisibilityScope, EncounterDefinition,
};
use super::catalog::ActCatalog;
use super::page::{
    ActContinuation, ActCursorState, ActDefinitionPage, ActDefinitionSummary,
    ActEncounterContinuation, ActEncounterDefinitionPage, ActEncounterListQuery,
    ActEncounterSummary, ActListQuery, ContinuationScope, EncounterCursorState,
};
use super::{visible_act, visible_encounter};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registries are mutable and continuations
/// are single-use. Call [`ActCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct ActCatalogReader {
    pub(super) catalog: ActCatalog,
    act_cursors: BTreeMap<String, ActCursorState>,
    encounter_cursors: BTreeMap<String, EncounterCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl ActCatalogReader {
    pub(super) fn new(catalog: ActCatalog) -> Self {
        Self {
            catalog,
            act_cursors: BTreeMap::new(),
            encounter_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &ActCatalog {
        &self.catalog
    }

    /// Lists visible act definitions in stable act-ID order.
    pub fn list(&mut self, query: &ActListQuery) -> Result<ActDefinitionPage, ActReferenceError> {
        if query.locale != self.catalog.binding.locale {
            return Err(ActReferenceError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > ACT_MAX_PAGE_ITEMS {
            return Err(ActReferenceError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.act_cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible_act(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end]
            .iter()
            .map(|definition| act_summary(definition))
            .collect::<Vec<_>>();
        let continuation = if end < total {
            let token = self.make_token("act-cursor");
            self.act_cursors.insert(
                token.clone(),
                ActCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(ActContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(ActDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible encounters for one exact act, optionally filtered by category.
    pub fn list_encounters(
        &mut self,
        query: &ActEncounterListQuery,
    ) -> Result<ActEncounterDefinitionPage, ActReferenceError> {
        if query.limit == 0 || query.limit > ACT_MAX_PAGE_ITEMS {
            return Err(ActReferenceError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.encounter_cursor_start(query)?;
        let definition = self.definition_for_reference(&query.act, query.scope)?;
        let entries_all = definition
            .encounters
            .iter()
            .filter(|encounter| visible_encounter(encounter, query.scope))
            .filter(|encounter| {
                query
                    .kind
                    .as_ref()
                    .is_none_or(|kind| *kind == encounter.kind)
            })
            .map(encounter_summary)
            .collect::<Vec<_>>();
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all[start..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("act-encounter-cursor");
            self.encounter_cursors.insert(
                token.clone(),
                EncounterCursorState {
                    binding: self.catalog.binding.clone(),
                    act_id: query.act.act_id.clone(),
                    kind: query.kind.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(ActEncounterContinuation::new(
                token,
                Arc::clone(&self.scope),
            ))
        } else {
            None
        };
        Ok(ActEncounterDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    pub(super) fn validate_family(&self) -> Result<(), ActReferenceError> {
        match self.catalog.family.state {
            ActFamilyState::Handled => Ok(()),
            ActFamilyState::Unsupported => Err(ActReferenceError::UnsupportedFamily),
            ActFamilyState::Unavailable => Err(ActReferenceError::UnavailableFamily),
        }
    }

    pub(super) fn definition_for_reference(
        &self,
        reference: &ActDefinitionReference,
        scope: ActVisibilityScope,
    ) -> Result<&ActDefinition, ActReferenceError> {
        if reference.catalog != self.catalog.binding {
            return Err(ActReferenceError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.act_id)
            .ok_or(ActReferenceError::NotFound)?;
        if !visible_act(definition, scope) {
            return Err(ActReferenceError::ExcludedByScope);
        }
        Ok(definition)
    }

    pub(super) fn definition_for_identity(
        &self,
        binding: &ActCatalogBinding,
        act_id: &str,
        scope: ActVisibilityScope,
    ) -> Result<&ActDefinition, ActReferenceError> {
        if binding != &self.catalog.binding {
            return Err(ActReferenceError::StaleReference);
        }
        let reference = ActDefinitionReference {
            catalog: self.catalog.binding.clone(),
            act_id: act_id.to_owned(),
        };
        self.definition_for_reference(&reference, scope)
    }

    fn act_cursor_start(&mut self, query: &ActListQuery) -> Result<usize, ActReferenceError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(ActReferenceError::InvalidContinuation);
        }
        let cursor = self
            .act_cursors
            .remove(continuation.token())
            .ok_or(ActReferenceError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ActReferenceError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn encounter_cursor_start(
        &mut self,
        query: &ActEncounterListQuery,
    ) -> Result<usize, ActReferenceError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(ActReferenceError::InvalidContinuation);
        }
        let cursor = self
            .encounter_cursors
            .remove(continuation.token())
            .ok_or(ActReferenceError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.act_id != query.act.act_id
            || cursor.kind != query.kind
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(ActReferenceError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

fn act_summary(definition: &ActDefinition) -> ActDefinitionSummary {
    ActDefinitionSummary {
        reference: definition.reference.clone(),
        name: definition.name.clone(),
        order: definition.order,
        unlock_state: definition.unlock_state,
        room_category_count: definition.room_categories.len(),
        encounter_count: definition.encounters.len(),
        pools: definition.pools.status(),
        constraints: definition.constraints.status(),
    }
}

fn encounter_summary(encounter: &EncounterDefinition) -> ActEncounterSummary {
    ActEncounterSummary {
        reference: encounter.reference.clone(),
        name: encounter.name.clone(),
        kind: encounter.kind.clone(),
        group_count: encounter.groups.len(),
        enemy_count: encounter
            .groups
            .iter()
            .map(|group| group.enemies.len())
            .sum(),
        eligibility: ActFieldStatus::Available,
        weight: encounter.weight.clone(),
        room_category_id: encounter.room_category_id.clone(),
    }
}
