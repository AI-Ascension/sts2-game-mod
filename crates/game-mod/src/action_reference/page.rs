// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    eligibility::ActionEligibility,
    field::{ActionFieldStatus, ActionText},
    kind::{ActionKind, ActionParentOperation},
    model::{
        ActionCatalogBinding, ActionEvidence, ActionFrameReference, ActionReference,
        ActionTargetReference, ActionVisibility, ActionVisibilityScope,
    },
    target::ActionTargetKind,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use action-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ActionContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use target-list continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionTargetContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ActionTargetContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded action-definition-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct ActionListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: ActionVisibilityScope,
    /// Maximum definitions in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ActionContinuation>,
}

/// Typed summary returned by one bounded action page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDefinitionSummary {
    /// Exact static legal-action reference.
    pub reference: ActionReference,
    /// Operation whose frame owns this action.
    pub parent: ActionParentOperation,
    /// Exact-build legal-action family.
    pub kind: ActionKind,
    /// Localized action label.
    pub label: ActionText,
    /// Visibility of the definition.
    pub visibility: ActionVisibility,
    /// Evidence label for the definition.
    pub evidence: ActionEvidence,
    /// Observed legal-action generation.
    pub instance_generation: u64,
    /// Resolved action availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Number of cost contributors.
    pub cost_count: usize,
    /// Number of cost contributors that currently block the action.
    pub blocking_cost_count: usize,
    /// Number of target restrictions.
    pub restriction_count: usize,
    /// Number of restrictions that currently block the action.
    pub blocking_restriction_count: usize,
    /// Number of visible observed targets.
    pub target_count: usize,
    /// Availability of the target list after scope withholding.
    pub targets_status: ActionFieldStatus,
    /// Number of visible declared previews.
    pub preview_count: usize,
    /// Availability of the preview list after scope withholding.
    pub previews_status: ActionFieldStatus,
}

/// Complete or partial action-definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct ActionDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: ActionCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<ActionDefinitionSummary>,
    /// Number of visible legal-action definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ActionContinuation>,
}

/// Bounded target-list request scoped to one legal-action generation.
#[derive(Debug, Eq, PartialEq)]
pub struct ActionTargetListQuery {
    /// Legal-action frame whose presented targets are listed.
    pub frame: ActionFrameReference,
    /// Visibility scope.
    pub scope: ActionVisibilityScope,
    /// Maximum targets in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ActionTargetContinuation>,
}

/// Typed summary returned by one bounded target page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTargetSummary {
    /// Exact static target reference.
    pub reference: ActionTargetReference,
    /// Localized target label.
    pub label: ActionText,
    /// Family of entity the target resolves to.
    pub kind: ActionTargetKind,
    /// Resolved target availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Availability of the resolved parameter after scope withholding.
    pub detail_status: ActionFieldStatus,
    /// Visibility of the target.
    pub visibility: ActionVisibility,
}

/// Complete or partial target page.
#[derive(Debug, Eq, PartialEq)]
pub struct ActionTargetPage {
    /// Catalog witness for every entry.
    pub binding: ActionCatalogBinding,
    /// Deterministically ordered target summaries.
    pub entries: Vec<ActionTargetSummary>,
    /// Number of visible targets.
    pub total: usize,
    /// Availability of targets after scope withholding, independent of pagination exhaustion.
    pub targets_status: ActionFieldStatus,
    /// Targets the host reports as available right now.
    pub available: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ActionTargetContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct ActionCursorState {
    pub(super) binding: ActionCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: ActionVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ActionTargetCursorState {
    pub(super) binding: ActionCatalogBinding,
    pub(super) action_id: String,
    pub(super) instance_generation: u64,
    pub(super) scope: ActionVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
