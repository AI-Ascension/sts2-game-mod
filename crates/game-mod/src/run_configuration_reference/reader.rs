// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::{
    binding::{RunConfigurationCacheKey, RunConfigurationLiveBinding},
    definition::{
        RunConfigurationCatalog, RunConfigurationDefinition, RunConfigurationDefinitionReference,
    },
    error::RunConfigurationError,
    model::{
        RUN_CONFIGURATION_MAX_CONTINUATIONS, RUN_CONFIGURATION_MAX_PAGE_ITEMS,
        RunConfigurationFamilyState, RunFieldKind, RunFieldRecord, RunVisibilityScope,
    },
    page::{
        ContinuationScope, RunConfigurationContinuation, RunConfigurationCursorState,
        RunConfigurationListQuery, RunConfigurationPage,
    },
    projection::{
        definition_visible, field_visible, mode_of, project, reject_seed_blind, summarize,
    },
};

/// Reader retaining one catalog while enforcing locale, scope, revision, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`RunConfigurationCatalog::reader`] for an independent reader instead.
#[derive(Debug)]
pub struct RunConfigurationReader {
    pub(super) catalog: RunConfigurationCatalog,
    cursors: BTreeMap<String, RunConfigurationCursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl RunConfigurationReader {
    pub(super) fn new(catalog: RunConfigurationCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &RunConfigurationCatalog {
        &self.catalog
    }

    /// Lists visible run configurations in stable run-identity order.
    pub fn list(
        &mut self,
        query: &RunConfigurationListQuery,
    ) -> Result<RunConfigurationPage, RunConfigurationError> {
        if query.locale != self.catalog.binding.locale {
            return Err(RunConfigurationError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > RUN_CONFIGURATION_MAX_PAGE_ITEMS {
            return Err(RunConfigurationError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self.cursor_start(query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| definition_visible(definition, query.scope))
            .filter(|definition| {
                query
                    .revision
                    .is_none_or(|revision| definition.live.revision == revision)
            })
            .filter(|definition| {
                query
                    .mode
                    .is_none_or(|mode| mode_of(definition) == Some(mode))
            })
            .map(summarize)
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end].to_vec();
        let continuation = self.next_continuation(query, end, total);
        Ok(RunConfigurationPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact run-configuration lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RunConfigurationDefinitionReference,
        scope: RunVisibilityScope,
    ) -> Result<RunConfigurationDefinition, RunConfigurationError> {
        if reference.catalog != self.catalog.binding {
            return Err(RunConfigurationError::StaleReference);
        }
        let definition = self.definition(reference, scope)?;
        Ok(project(definition, scope))
    }

    /// Returns the settled field record for one kind under an explicit visibility scope.
    pub fn settled(
        &self,
        run_id: &str,
        kind: RunFieldKind,
        scope: RunVisibilityScope,
    ) -> Result<RunFieldRecord, RunConfigurationError> {
        reject_seed_blind(kind, scope)?;
        let definition = self.definition_for_run(run_id, scope)?;
        let record = definition
            .field(kind)
            .ok_or(RunConfigurationError::NotFound(kind))?;
        if !field_visible(record, scope) {
            return Err(RunConfigurationError::ExcludedByScope);
        }
        Ok(record.clone())
    }

    /// Returns the settled field record for one kind, rejecting a stale run identity or revision.
    pub fn settled_for(
        &self,
        live: &RunConfigurationLiveBinding,
        kind: RunFieldKind,
        scope: RunVisibilityScope,
    ) -> Result<RunFieldRecord, RunConfigurationError> {
        let definition = self.definition_for_run(&live.run_id, scope)?;
        if definition.live.instance_id != live.instance_id || definition.live.epoch != live.epoch {
            return Err(RunConfigurationError::StaleRunIdentity);
        }
        if definition.live.revision != live.revision {
            return Err(RunConfigurationError::StaleRevision {
                expected: definition.live.revision,
                actual: live.revision,
            });
        }
        self.settled(&live.run_id, kind, scope)
    }

    /// Returns the cache key a preview entry must be fenced by for one run.
    pub fn cache_key(
        &self,
        run_id: &str,
        scope: RunVisibilityScope,
    ) -> Result<RunConfigurationCacheKey, RunConfigurationError> {
        let definition = self.definition_for_run(run_id, scope)?;
        Ok(match scope {
            RunVisibilityScope::SeedBlind => definition.seed_blind_cache.clone(),
            RunVisibilityScope::Public | RunVisibilityScope::Owner => definition.cache.clone(),
        })
    }

    fn validate_family(&self) -> Result<(), RunConfigurationError> {
        match self.catalog.family.state {
            RunConfigurationFamilyState::Handled => Ok(()),
            RunConfigurationFamilyState::Unsupported => {
                Err(RunConfigurationError::UnsupportedFamily)
            }
            RunConfigurationFamilyState::Unavailable => {
                Err(RunConfigurationError::UnavailableFamily)
            }
        }
    }

    fn definition_for_run(
        &self,
        run_id: &str,
        scope: RunVisibilityScope,
    ) -> Result<&RunConfigurationDefinition, RunConfigurationError> {
        self.validate_family()?;
        let definition = self
            .catalog
            .definitions
            .get(run_id)
            .ok_or_else(|| RunConfigurationError::UnknownRun(run_id.to_owned()))?;
        if !definition_visible(definition, scope) {
            return Err(RunConfigurationError::ExcludedByScope);
        }
        Ok(definition)
    }

    fn definition(
        &self,
        reference: &RunConfigurationDefinitionReference,
        scope: RunVisibilityScope,
    ) -> Result<&RunConfigurationDefinition, RunConfigurationError> {
        let definition = self.definition_for_run(&reference.run_id, scope)?;
        if reference.revision != definition.live.revision {
            return Err(RunConfigurationError::StaleRevision {
                expected: definition.live.revision,
                actual: reference.revision,
            });
        }
        Ok(definition)
    }

    fn next_continuation(
        &mut self,
        query: &RunConfigurationListQuery,
        end: usize,
        total: usize,
    ) -> Option<RunConfigurationContinuation> {
        if end >= total {
            return None;
        }
        if self.cursors.len() >= RUN_CONFIGURATION_MAX_CONTINUATIONS
            && let Some(oldest) = self.cursors.keys().next().cloned()
        {
            self.cursors.remove(&oldest);
        }
        let token = self.make_token();
        self.cursors.insert(
            token.clone(),
            RunConfigurationCursorState {
                binding: self.catalog.binding.clone(),
                locale: query.locale.clone(),
                revision: query.revision,
                mode: query.mode,
                scope: query.scope,
                limit: query.limit,
                offset: end,
            },
        );
        Some(RunConfigurationContinuation::new(
            token,
            Arc::clone(&self.scope),
        ))
    }

    fn cursor_start(
        &mut self,
        query: &RunConfigurationListQuery,
    ) -> Result<usize, RunConfigurationError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(RunConfigurationError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(RunConfigurationError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.revision != query.revision
            || cursor.mode != query.mode
            || cursor.scope != query.scope
            || cursor.limit != query.limit
        {
            return Err(RunConfigurationError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self) -> String {
        let token = format!("run-configuration-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}
