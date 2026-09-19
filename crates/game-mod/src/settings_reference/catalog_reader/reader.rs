// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::super::model::SETTINGS_MAX_PAGE_ITEMS;
use super::super::{
    SettingDefinition, SettingsDefinitionReference, SettingsFamilyState, SettingsReferenceError,
    SettingsVisibilityScope,
};
use super::catalog::SettingsCatalog;
use super::page::{
    ContinuationScope, SettingsContinuation, SettingsCursorState, SettingsDefinitionPage,
    SettingsDefinitionSummary, SettingsListQuery,
};
use super::visible_setting;

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`SettingsCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct SettingsCatalogReader {
    pub(super) catalog: SettingsCatalog,
    cursors: BTreeMap<String, SettingsCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl SettingsCatalogReader {
    pub(super) fn new(catalog: SettingsCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &SettingsCatalog {
        &self.catalog
    }

    /// Lists visible setting definitions in stable setting-ID order.
    pub fn list(
        &mut self,
        query: &SettingsListQuery,
    ) -> Result<SettingsDefinitionPage, SettingsReferenceError> {
        if query.locale != self.catalog.binding.locale {
            return Err(SettingsReferenceError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > SETTINGS_MAX_PAGE_ITEMS {
            return Err(SettingsReferenceError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visible_setting(definition, query.scope))
            .filter(|definition| {
                query
                    .category
                    .is_none_or(|category| category == definition.category)
            })
            .filter(|definition| query.level.is_none_or(|level| level == definition.level))
            .map(setting_summary)
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end].to_vec();
        let continuation = self.next_continuation(query, end, total);
        Ok(SettingsDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact setting lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &SettingsDefinitionReference,
        scope: SettingsVisibilityScope,
    ) -> Result<SettingDefinition, SettingsReferenceError> {
        self.definition_for_reference(reference, scope).cloned()
    }

    pub(super) fn validate_family(&self) -> Result<(), SettingsReferenceError> {
        match self.catalog.family.state {
            SettingsFamilyState::Handled => Ok(()),
            SettingsFamilyState::Unsupported => Err(SettingsReferenceError::UnsupportedFamily),
            SettingsFamilyState::Unavailable => Err(SettingsReferenceError::UnavailableFamily),
        }
    }

    pub(super) fn definition_for_reference(
        &self,
        reference: &SettingsDefinitionReference,
        scope: SettingsVisibilityScope,
    ) -> Result<&SettingDefinition, SettingsReferenceError> {
        if reference.catalog != self.catalog.binding {
            return Err(SettingsReferenceError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(&reference.setting_id)
            .ok_or(SettingsReferenceError::NotFound)?;
        if !visible_setting(definition, scope) {
            return Err(SettingsReferenceError::ExcludedByScope);
        }
        Ok(definition)
    }

    fn next_continuation(
        &mut self,
        query: &SettingsListQuery,
        end: usize,
        total: usize,
    ) -> Option<SettingsContinuation> {
        if end >= total {
            return None;
        }
        let token = self.make_token();
        self.cursors.insert(
            token.clone(),
            SettingsCursorState {
                binding: self.catalog.binding.clone(),
                locale: query.locale.clone(),
                category: query.category,
                level: query.level,
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(SettingsContinuation::new(token, Arc::clone(&self.scope)))
    }

    fn cursor_start(&mut self, query: &SettingsListQuery) -> Result<usize, SettingsReferenceError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(SettingsReferenceError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(SettingsReferenceError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.category != query.category
            || cursor.level != query.level
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(SettingsReferenceError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self) -> String {
        let token = format!("settings-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

fn setting_summary(definition: &SettingDefinition) -> SettingsDefinitionSummary {
    SettingsDefinitionSummary {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        category: definition.category,
        level: definition.level,
        value_type: definition.value_type,
        sensitivity: definition.sensitivity,
        visibility: definition.visibility,
        default_value: definition.default_value.status(),
        stored_value: definition.stored_value.status(),
        effective_value: definition.effective_value.status(),
        run_link: definition.run_link.status(),
    }
}
