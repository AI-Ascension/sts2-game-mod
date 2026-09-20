// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    cursor::ActionCursors,
    definition::{ActionCatalog, ActionDefinition},
    error::ActionError,
    explain::{ActionAvailabilityExplanation, ActionAvailabilityQuery, explain_availability},
    model::{
        ACTION_MAX_PAGE_ITEMS, ActionCatalogBinding, ActionFrameReference, ActionPreviewReference,
        ActionReference, ActionTargetReference, ActionVisibilityScope,
    },
    page::{
        ActionDefinitionPage, ActionListQuery, ActionTargetListQuery, ActionTargetPage,
        ContinuationScope,
    },
    preview::ActionPreview,
    preview_query::{ActionPreviewQuery, ActionPreviewResult, preview_of},
    projection::{
        action_summary, map_status, project_definition, target_summary, visibility_allowed,
        visible_target,
    },
    target::ActionTarget,
};

/// Reader retaining one catalog while enforcing locale, scope, generation, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`ActionCatalog::reader`] for an independent reader instead. Every method here
/// is a bounded read over an immutable snapshot: none dispatches, plays, uses, buys, ends, confirms,
/// or advances a legal action, and none reads a live frame.
#[derive(Debug)]
pub struct ActionReader<'a> {
    catalog: &'a ActionCatalog,
    cursors: ActionCursors,
    scope: ActionVisibilityScope,
    continuations: Arc<ContinuationScope>,
}

impl<'a> ActionReader<'a> {
    pub(super) fn new(catalog: &'a ActionCatalog, scope: ActionVisibilityScope) -> Self {
        Self {
            catalog,
            cursors: ActionCursors::new(),
            scope,
            continuations: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub const fn catalog(&self) -> &ActionCatalog {
        self.catalog
    }

    /// Returns the default visibility scope this reader was created with.
    #[must_use]
    pub const fn scope(&self) -> ActionVisibilityScope {
        self.scope
    }

    /// Lists visible legal-action definitions in stable action-identity order.
    pub fn list(&mut self, query: &ActionListQuery) -> Result<ActionDefinitionPage, ActionError> {
        if query.locale != self.catalog.binding.locale {
            return Err(ActionError::LocaleMismatch);
        }
        self.check_page_size(query.limit)?;
        self.validate_family()?;
        let start = self
            .cursors
            .action_start(&self.catalog.binding, &self.continuations, query)?;
        let entries = self
            .catalog
            .definitions()
            .values()
            .filter(|definition| visibility_allowed(definition.visibility, query.scope))
            .map(|definition| action_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_action(
            &self.catalog.binding,
            &self.continuations,
            query,
            end,
            total,
        );
        Ok(ActionDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists the targets one legal-action generation currently presents.
    ///
    /// The page is bound to the generation the caller observed, so paging a frame the host has
    /// already moved past is refused rather than answered with the newer frame's targets.
    pub fn list_targets(
        &mut self,
        query: &ActionTargetListQuery,
    ) -> Result<ActionTargetPage, ActionError> {
        self.check_page_size(query.limit)?;
        self.validate_family()?;
        let definition = self.frame_of(&query.frame, query.scope)?;
        let start = self
            .cursors
            .target_start(&self.catalog.binding, &self.continuations, query)?;
        let entries = definition
            .targets
            .values()
            .filter(|target| visible_target(target, query.scope))
            .map(target_summary)
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_target(
            &self.catalog.binding,
            &self.continuations,
            query,
            end,
            total,
        );
        let available = definition
            .targets
            .values()
            .filter(|target| visible_target(target, query.scope) && target.is_available())
            .count();
        Ok(ActionTargetPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            targets_status: map_status(
                definition.targets.values(),
                definition.targets_status,
                |target| visible_target(target, query.scope),
            ),
            available,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact legal-action definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &ActionReference,
        scope: ActionVisibilityScope,
    ) -> Result<ActionDefinition, ActionError> {
        self.validate_family()?;
        Ok(project_definition(
            self.definition_of(&reference.action_id, &reference.catalog, scope)?,
            scope,
        ))
    }

    /// Performs one exact observed-target lookup under an explicit visibility scope.
    pub fn get_target(
        &self,
        reference: &ActionTargetReference,
        scope: ActionVisibilityScope,
    ) -> Result<ActionTarget, ActionError> {
        self.validate_family()?;
        Ok(self.target_of(reference, scope)?.clone())
    }

    /// Performs one exact declared-preview lookup under an explicit visibility scope.
    ///
    /// A preview reference names no generation, so the generation fence is enforced by
    /// [`Self::preview`]; this lookup answers only whether the catalog describes that preview at all.
    pub fn get_preview(
        &self,
        reference: &ActionPreviewReference,
        scope: ActionVisibilityScope,
    ) -> Result<ActionPreview, ActionError> {
        self.validate_family()?;
        let definition = self.definition_of(&reference.action_id, &reference.catalog, scope)?;
        definition
            .previews
            .get(&reference.preview_id)
            .filter(|preview| super::projection::visible_preview(preview, definition, scope))
            .map(|preview| super::projection::project_preview(preview, definition, scope))
            .ok_or(ActionError::NotFound)
    }

    /// Explains why one legal-action frame is, or is not, available right now.
    ///
    /// A frame whose generation the catalog has moved past is refused, so a stale explanation is
    /// never published as the current reason.
    pub fn explain(
        &self,
        query: &ActionAvailabilityQuery,
    ) -> Result<ActionAvailabilityExplanation, ActionError> {
        self.validate_family()?;
        let definition = self.frame_of(&query.frame, query.scope)?;
        Ok(explain_availability(definition, query.scope))
    }

    /// Previews the visible target-specific consequences of one legal-action frame.
    ///
    /// The result never authorizes dispatch, and a query that leaves the action disabled still
    /// reports what the source documents rather than withholding a documented consequence.
    pub fn preview(&self, query: &ActionPreviewQuery) -> Result<ActionPreviewResult, ActionError> {
        self.validate_family()?;
        let definition = self.frame_of(&query.frame, query.scope)?;
        preview_of(definition, query)
    }

    fn check_page_size(&self, limit: usize) -> Result<(), ActionError> {
        if limit == 0 || limit > ACTION_MAX_PAGE_ITEMS {
            return Err(ActionError::InvalidPageSize);
        }
        Ok(())
    }

    pub(super) fn validate_family(&self) -> Result<(), ActionError> {
        match self.catalog.family.state {
            super::model::ActionFamilyState::Handled => Ok(()),
            super::model::ActionFamilyState::Unsupported => Err(ActionError::UnsupportedFamily),
            super::model::ActionFamilyState::Unavailable => Err(ActionError::UnavailableFamily),
        }
    }

    /// Resolves one frame and refuses a reference bound to another legal-action generation.
    pub(super) fn frame_of(
        &self,
        frame: &ActionFrameReference,
        scope: ActionVisibilityScope,
    ) -> Result<&'a ActionDefinition, ActionError> {
        let definition =
            self.definition_of(&frame.action.action_id, &frame.action.catalog, scope)?;
        if frame.generation != definition.instance_generation {
            return Err(ActionError::StaleActionReference {
                action_id: definition.reference.action_id.clone(),
                referenced: frame.generation,
                current: definition.instance_generation,
            });
        }
        Ok(definition)
    }

    pub(super) fn definition_of(
        &self,
        action_id: &str,
        binding: &ActionCatalogBinding,
        scope: ActionVisibilityScope,
    ) -> Result<&'a ActionDefinition, ActionError> {
        if binding != &self.catalog.binding {
            return Err(ActionError::StaleReference);
        }
        let definition = self
            .catalog
            .definition(action_id)
            .ok_or(ActionError::NotFound)?;
        if !visibility_allowed(definition.visibility, scope) {
            return Err(ActionError::ExcludedByScope);
        }
        Ok(definition)
    }

    pub(super) fn target_of(
        &self,
        reference: &ActionTargetReference,
        scope: ActionVisibilityScope,
    ) -> Result<&'a ActionTarget, ActionError> {
        let definition = self.definition_of(&reference.action_id, &reference.catalog, scope)?;
        let target = definition
            .targets
            .get(&reference.target_id)
            .ok_or(ActionError::NotFound)?;
        if !visible_target(target, scope) {
            return Err(ActionError::ExcludedByScope);
        }
        Ok(target)
    }
}
