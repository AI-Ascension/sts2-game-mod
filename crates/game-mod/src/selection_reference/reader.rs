// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    candidate::SelectionCandidate,
    cursor::SelectionCursors,
    definition::{SelectionCatalog, SelectionDefinition},
    error::SelectionError,
    model::{
        SEL_MAX_PAGE_ITEMS, SelectionCandidateReference, SelectionCatalogBinding,
        SelectionFamilyState, SelectionReference, SelectionVisibilityScope,
    },
    page::{
        ContinuationScope, SelectionCandidateListQuery, SelectionCandidatePage,
        SelectionDefinitionPage, SelectionListQuery,
    },
    progress::remaining_picks,
    projection::{
        candidate_summary, map_status, project_definition, selectable_candidate, selection_summary,
        visibility_allowed, visible_candidate,
    },
};

/// Reader retaining one catalog while enforcing locale, scope, generation, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`SelectionCatalog::reader`] for an independent reader instead. Every method
/// here is a bounded read: none clicks, confirms, cancels, or advances a prompt.
#[derive(Debug)]
pub struct SelectionReader<'a> {
    catalog: &'a SelectionCatalog,
    cursors: SelectionCursors,
    scope: SelectionVisibilityScope,
    continuations: Arc<ContinuationScope>,
}

impl<'a> SelectionReader<'a> {
    pub(super) fn new(catalog: &'a SelectionCatalog, scope: SelectionVisibilityScope) -> Self {
        Self {
            catalog,
            cursors: SelectionCursors::new(),
            scope,
            continuations: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub const fn catalog(&self) -> &SelectionCatalog {
        self.catalog
    }

    /// Returns the default visibility scope this reader was created with.
    #[must_use]
    pub const fn scope(&self) -> SelectionVisibilityScope {
        self.scope
    }

    /// Lists visible selection definitions in stable selection-identity order.
    pub fn list(
        &mut self,
        query: &SelectionListQuery,
    ) -> Result<SelectionDefinitionPage, SelectionError> {
        if query.locale != self.catalog.binding.locale {
            return Err(SelectionError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > SEL_MAX_PAGE_ITEMS {
            return Err(SelectionError::InvalidPageSize);
        }
        self.validate_family()?;
        let start =
            self.cursors
                .selection_start(&self.catalog.binding, &self.continuations, query)?;
        let entries = self
            .catalog
            .definitions()
            .values()
            .filter(|definition| visibility_allowed(definition.visibility, query.scope))
            .map(|definition| selection_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_selection(
            &self.catalog.binding,
            &self.continuations,
            query,
            end,
            total,
        );
        Ok(SelectionDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists the candidates still selectable for one selector generation.
    ///
    /// When the caller reports its picks, the page excludes candidates it already picked under a
    /// no-repeat rule and reports the remaining count for those picks, so a two-pick prompt shows
    /// the second pick's real options rather than the first pick's list.
    pub fn list_candidates(
        &mut self,
        query: &SelectionCandidateListQuery,
    ) -> Result<SelectionCandidatePage, SelectionError> {
        if query.limit == 0 || query.limit > SEL_MAX_PAGE_ITEMS {
            return Err(SelectionError::InvalidPageSize);
        }
        self.validate_family()?;
        let selected = self.selected_ids(query.progress.as_ref(), query.scope)?;
        let definition = self.generation_of(&query.selector, query.scope)?;
        let start = self.cursors.candidate_start(
            &self.catalog.binding,
            &self.continuations,
            query,
            &selected,
        )?;
        let candidates_status = map_status(
            definition.candidates.values(),
            query.scope,
            visible_candidate,
        );
        let entries = definition
            .candidates
            .values()
            .filter(|candidate| {
                selectable_candidate(candidate, query.scope, &selected, definition.duplicate)
            })
            .map(candidate_summary)
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_candidate(
            &self.catalog.binding,
            &self.continuations,
            query,
            &selected,
            end,
            total,
        );
        Ok(SelectionCandidatePage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            candidates_status,
            picked: selected.len(),
            remaining: remaining_picks(&definition.picks, selected.len()),
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact selection definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &SelectionReference,
        scope: SelectionVisibilityScope,
    ) -> Result<SelectionDefinition, SelectionError> {
        self.validate_family()?;
        Ok(project_definition(
            self.definition_of(&reference.selection_id, &reference.catalog, scope)?,
            scope,
        ))
    }

    /// Performs one exact candidate lookup under an explicit visibility scope.
    pub fn get_candidate(
        &self,
        reference: &SelectionCandidateReference,
        scope: SelectionVisibilityScope,
    ) -> Result<SelectionCandidate, SelectionError> {
        self.validate_family()?;
        let definition = self.definition_of(&reference.selection_id, &reference.catalog, scope)?;
        let candidate = definition
            .candidates
            .get(&reference.candidate_id)
            .ok_or(SelectionError::NotFound)?;
        if !visible_candidate(candidate, scope) {
            return Err(SelectionError::ExcludedByScope);
        }
        Ok(candidate.clone())
    }

    pub(super) fn validate_family(&self) -> Result<(), SelectionError> {
        match self.catalog.family.state {
            SelectionFamilyState::Handled => Ok(()),
            SelectionFamilyState::Unsupported => Err(SelectionError::UnsupportedFamily),
            SelectionFamilyState::Unavailable => Err(SelectionError::UnavailableFamily),
        }
    }

    pub(super) fn definition_of(
        &self,
        selection_id: &str,
        binding: &SelectionCatalogBinding,
        scope: SelectionVisibilityScope,
    ) -> Result<&'a SelectionDefinition, SelectionError> {
        if binding != &self.catalog.binding {
            return Err(SelectionError::StaleReference);
        }
        let definition = self
            .catalog
            .definition(selection_id)
            .ok_or(SelectionError::NotFound)?;
        if !visibility_allowed(definition.visibility, scope) {
            return Err(SelectionError::ExcludedByScope);
        }
        Ok(definition)
    }
}
