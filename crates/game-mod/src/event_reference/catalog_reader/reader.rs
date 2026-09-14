// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::model::EVENT_MAX_PAGE_ITEMS;
use super::super::{
    EventCatalogBinding, EventCatalogError, EventDefinition, EventDefinitionReference,
    EventFamilyState, EventFieldStatus, EventVisibilityScope,
};
use super::catalog::EventCatalog;
use super::page::{
    ContinuationScope, EventContinuation, EventCursorState, EventDefinitionPage,
    EventDefinitionSummary, EventListQuery, EventOptionContinuation, EventOptionListQuery,
    EventOptionPage, EventOptionSummary, OptionCursorState,
};
use super::{
    visible_cost, visible_event, visible_option, visible_outcome, visible_page, visible_requirement,
};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registries are mutable and continuations are
/// single-use. Call [`EventCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct EventCatalogReader {
    pub(super) catalog: EventCatalog,
    event_cursors: BTreeMap<String, EventCursorState>,
    option_cursors: BTreeMap<String, OptionCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl EventCatalogReader {
    pub(super) fn new(catalog: EventCatalog) -> Self {
        Self {
            catalog,
            event_cursors: BTreeMap::new(),
            option_cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &EventCatalog {
        &self.catalog
    }

    /// Lists visible event definitions in stable event-ID order.
    pub fn list(
        &mut self,
        query: &EventListQuery,
    ) -> Result<EventDefinitionPage, EventCatalogError> {
        if query.locale != self.catalog.binding.locale {
            return Err(EventCatalogError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > EVENT_MAX_PAGE_ITEMS {
            return Err(EventCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.event_cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible_event(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end]
            .iter()
            .map(|definition| event_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let continuation = if end < total {
            let token = self.make_token("event-cursor");
            self.event_cursors.insert(
                token.clone(),
                EventCursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(EventContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(EventDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible options for one exact event.
    pub fn list_options(
        &mut self,
        query: &EventOptionListQuery,
    ) -> Result<EventOptionPage, EventCatalogError> {
        if query.limit == 0 || query.limit > EVENT_MAX_PAGE_ITEMS {
            return Err(EventCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.option_cursor_start(query)?;
        let definition = self.definition_for_reference(&query.event, query.scope)?;
        let entries_all = definition
            .options
            .iter()
            .filter(|option| visible_option(option, query.scope))
            .map(|option| option_summary(option, query.scope))
            .collect::<Vec<_>>();
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all[start..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token("event-option-cursor");
            self.option_cursors.insert(
                token.clone(),
                OptionCursorState {
                    binding: self.catalog.binding.clone(),
                    event_id: query.event.event_id.clone(),
                    scope: query.scope,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(EventOptionContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(EventOptionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    pub(super) fn validate_family(&self) -> Result<(), EventCatalogError> {
        match self.catalog.family.state {
            EventFamilyState::Handled => Ok(()),
            EventFamilyState::Unsupported => Err(EventCatalogError::UnsupportedFamily),
            EventFamilyState::Unavailable => Err(EventCatalogError::UnavailableFamily),
        }
    }

    pub(super) fn definition_for_reference(
        &self,
        reference: &EventDefinitionReference,
        scope: EventVisibilityScope,
    ) -> Result<&EventDefinition, EventCatalogError> {
        if reference.catalog != self.catalog.binding {
            return Err(EventCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.event_id)
            .ok_or(EventCatalogError::NotFound)?;
        if !visible_event(definition, scope) {
            return Err(EventCatalogError::ExcludedByScope);
        }
        Ok(definition)
    }

    pub(super) fn definition_for_identity(
        &self,
        binding: &EventCatalogBinding,
        event_id: &str,
        scope: EventVisibilityScope,
    ) -> Result<&EventDefinition, EventCatalogError> {
        if binding != &self.catalog.binding {
            return Err(EventCatalogError::StaleReference);
        }
        let reference = EventDefinitionReference {
            catalog: self.catalog.binding.clone(),
            event_id: event_id.to_owned(),
        };
        self.definition_for_reference(&reference, scope)
    }

    fn event_cursor_start(&mut self, query: &EventListQuery) -> Result<usize, EventCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(EventCatalogError::InvalidContinuation);
        }
        let cursor = self
            .event_cursors
            .remove(continuation.token())
            .ok_or(EventCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(EventCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn option_cursor_start(
        &mut self,
        query: &EventOptionListQuery,
    ) -> Result<usize, EventCatalogError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(EventCatalogError::InvalidContinuation);
        }
        let cursor = self
            .option_cursors
            .remove(continuation.token())
            .ok_or(EventCatalogError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.event_id != query.event.event_id
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(EventCatalogError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self, prefix: &str) -> String {
        let token = format!("{prefix}-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

fn event_summary(
    definition: &EventDefinition,
    scope: EventVisibilityScope,
) -> EventDefinitionSummary {
    EventDefinitionSummary {
        reference: definition.reference.clone(),
        title: definition.title.clone(),
        kind: definition.kind.clone(),
        page_count: definition
            .pages
            .iter()
            .filter(|page| visible_page(page, scope))
            .count(),
        option_count: definition
            .options
            .iter()
            .filter(|option| visible_option(option, scope))
            .count(),
        eligibility: EventFieldStatus::Available,
    }
}

fn option_summary(
    option: &super::super::EventOption,
    scope: EventVisibilityScope,
) -> EventOptionSummary {
    EventOptionSummary {
        reference: option.reference.clone(),
        text: option.text.clone(),
        requirement_count: option
            .requirements
            .iter()
            .filter(|requirement| visible_requirement(requirement, scope))
            .count(),
        cost_count: option
            .costs
            .iter()
            .filter(|cost| visible_cost(cost, scope))
            .count(),
        outcome_count: option
            .outcomes
            .iter()
            .filter(|outcome| visible_outcome(outcome, scope))
            .count(),
        visibility: option.visibility,
    }
}
