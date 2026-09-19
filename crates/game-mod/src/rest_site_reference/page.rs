// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    field::{RestFieldStatus, RestText},
    model::{
        RestCatalogBinding, RestEvidence, RestOptionKind, RestOptionReference,
        RestSemanticReference, RestSiteDefinitionReference, RestVisibility, RestVisibilityScope,
    },
    option::RestOptionAvailability,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use rest-site-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestSiteContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RestSiteContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use rest-option-list continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestOptionContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RestOptionContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded rest-site-definition-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct RestSiteListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: RestVisibilityScope,
    /// Maximum definitions in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RestSiteContinuation>,
}

/// Typed summary returned by one bounded rest-site page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSiteDefinitionSummary {
    /// Exact static definition reference.
    pub reference: RestSiteDefinitionReference,
    /// Localized site label or explicit unavailable state.
    pub label: RestText,
    /// Visibility of the definition.
    pub visibility: RestVisibility,
    /// Evidence label for the definition.
    pub evidence: RestEvidence,
    /// Observed option-set generation.
    pub option_set_generation: u64,
    /// Number of visible rest options.
    pub option_count: usize,
    /// Availability of options after scope withholding.
    pub options_status: RestFieldStatus,
    /// Number of visible coverage records.
    pub coverage_count: usize,
    /// Availability of coverage records after scope withholding.
    pub coverage_status: RestFieldStatus,
}

/// Complete or partial rest-site-definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct RestSiteDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: RestCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<RestSiteDefinitionSummary>,
    /// Number of visible rest-site definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RestSiteContinuation>,
}

/// Bounded option-list request scoped to one exact rest-site definition.
#[derive(Debug, Eq, PartialEq)]
pub struct RestOptionListQuery {
    /// Exact rest-site definition whose options are listed.
    pub site: RestSiteDefinitionReference,
    /// Visibility scope.
    pub scope: RestVisibilityScope,
    /// Maximum options in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RestOptionContinuation>,
}

/// Typed summary returned by one bounded rest-option page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestOptionSummary {
    /// Exact static option reference.
    pub reference: RestOptionReference,
    /// Localized option label.
    pub label: RestText,
    /// Reported type of the option.
    pub kind: RestOptionKind,
    /// Definition the option resolves to.
    pub definition: RestSemanticReference,
    /// Resolved availability and refusal reason.
    pub availability: RestOptionAvailability,
    /// Number of requirements the option imposes.
    pub requirement_count: usize,
    /// Number of effects the option would produce.
    pub effect_count: usize,
    /// Visibility of the option.
    pub visibility: RestVisibility,
}

/// Complete or partial rest-option page.
#[derive(Debug, Eq, PartialEq)]
pub struct RestOptionPage {
    /// Catalog witness for every entry.
    pub binding: RestCatalogBinding,
    /// Deterministically ordered option summaries.
    pub entries: Vec<RestOptionSummary>,
    /// Number of visible options.
    pub total: usize,
    /// Availability of options after scope withholding, independent of pagination exhaustion.
    pub options_status: RestFieldStatus,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RestOptionContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct RestSiteCursorState {
    pub(super) binding: RestCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: RestVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct RestOptionCursorState {
    pub(super) binding: RestCatalogBinding,
    pub(super) site_id: String,
    pub(super) scope: RestVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
