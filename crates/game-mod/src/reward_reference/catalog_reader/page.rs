// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    RewardCatalogBinding, RewardDefinitionReference, RewardFieldStatus, RewardItemReference,
    RewardKind, RewardQuantity, RewardSemanticReference, RewardText, RewardVisibility,
    RewardVisibilityScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use reward-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RewardContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use offered-item-list continuation.
///
/// Like [`RewardContinuation`], a clone is rejected after the token is consumed once.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardItemContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RewardItemContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded reward-definition-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct RewardListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: RewardVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RewardContinuation>,
}

/// Typed summary returned by one bounded reward page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardDefinitionSummary {
    /// Exact static definition reference.
    pub reference: RewardDefinitionReference,
    /// Localized reward label or explicit unavailable state.
    pub label: RewardText,
    /// Reward category.
    pub kind: RewardKind,
    /// Number of visible offered items.
    pub item_count: usize,
    /// Availability of offered items after scope withholding.
    pub items_status: RewardFieldStatus,
    /// Number of visible generation rules.
    pub rule_count: usize,
    /// Availability of generation rules after scope withholding.
    pub generation_status: RewardFieldStatus,
    /// Availability of the selection group's legal actions.
    pub legal_actions_status: RewardFieldStatus,
}

/// Complete or partial reward definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct RewardDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: RewardCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<RewardDefinitionSummary>,
    /// Number of visible rewards.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RewardContinuation>,
}

/// Bounded offered-item-list request scoped to one exact reward definition.
#[derive(Debug, Eq, PartialEq)]
pub struct RewardItemListQuery {
    /// Exact reward definition whose items are listed.
    pub reward: RewardDefinitionReference,
    /// Visibility scope.
    pub scope: RewardVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RewardItemContinuation>,
}

/// Typed summary returned by one bounded offered-item page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardItemSummary {
    /// Exact static item reference.
    pub reference: RewardItemReference,
    /// Localized/source-defined item label.
    pub label: RewardText,
    /// Typed content definition reference for this item.
    pub definition: RewardSemanticReference,
    /// Exact typed quantity.
    pub quantity: RewardQuantity,
    /// Availability of the item's currency/owner unit.
    pub unit_status: RewardFieldStatus,
    /// Availability of the live item instance reference.
    pub instance_status: RewardFieldStatus,
    /// Visibility of the static item.
    pub visibility: RewardVisibility,
}

/// Complete or partial offered-item page.
#[derive(Debug, Eq, PartialEq)]
pub struct RewardItemPage {
    /// Catalog witness for every entry.
    pub binding: RewardCatalogBinding,
    /// Deterministically ordered item summaries.
    pub entries: Vec<RewardItemSummary>,
    /// Number of visible items.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RewardItemContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct RewardCursorState {
    pub(super) binding: RewardCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: RewardVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ItemCursorState {
    pub(super) binding: RewardCatalogBinding,
    pub(super) reward_id: String,
    pub(super) scope: RewardVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
