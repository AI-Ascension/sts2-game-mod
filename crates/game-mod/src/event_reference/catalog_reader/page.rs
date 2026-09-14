// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    EventCatalogBinding, EventDefinitionReference, EventFieldStatus, EventKind,
    EventOptionReference, EventText, EventVisibility, EventVisibilityScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use event-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl EventContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use option-list continuation.
///
/// Like [`EventContinuation`], a clone is rejected after the token is consumed once.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventOptionContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl EventOptionContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded event-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct EventListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: EventVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EventContinuation>,
}

/// Typed summary returned by one bounded event page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDefinitionSummary {
    /// Exact static definition reference.
    pub reference: EventDefinitionReference,
    /// Localized event title or explicit unavailable state.
    pub title: EventText,
    /// Event category.
    pub kind: EventKind,
    /// Number of visible narrative pages.
    pub page_count: usize,
    /// Number of visible options.
    pub option_count: usize,
    /// Availability of event eligibility predicates.
    pub eligibility: EventFieldStatus,
}

/// Complete or partial event definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct EventDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: EventCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<EventDefinitionSummary>,
    /// Number of visible events.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EventContinuation>,
}

/// Bounded option-list request scoped to one exact event definition.
#[derive(Debug, Eq, PartialEq)]
pub struct EventOptionListQuery {
    /// Exact event definition whose options are listed.
    pub event: EventDefinitionReference,
    /// Visibility scope.
    pub scope: EventVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EventOptionContinuation>,
}

/// Typed summary returned by one bounded option page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOptionSummary {
    /// Exact static option reference.
    pub reference: EventOptionReference,
    /// Localized option text.
    pub text: EventText,
    /// Number of visible option requirements.
    pub requirement_count: usize,
    /// Availability of option requirements after scope withholding.
    pub requirements_status: EventFieldStatus,
    /// Number of visible option costs.
    pub cost_count: usize,
    /// Availability of option costs after scope withholding.
    pub costs_status: EventFieldStatus,
    /// Number of visible option outcomes.
    pub outcome_count: usize,
    /// Availability of option outcomes after scope withholding.
    pub outcomes_status: EventFieldStatus,
    /// Visibility of the static option.
    pub visibility: EventVisibility,
}

/// Complete or partial option page.
#[derive(Debug, Eq, PartialEq)]
pub struct EventOptionPage {
    /// Catalog witness for every entry.
    pub binding: EventCatalogBinding,
    /// Deterministically ordered option summaries.
    pub entries: Vec<EventOptionSummary>,
    /// Number of visible options.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EventOptionContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct EventCursorState {
    pub(super) binding: EventCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: EventVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct OptionCursorState {
    pub(super) binding: EventCatalogBinding,
    pub(super) event_id: String,
    pub(super) scope: EventVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
