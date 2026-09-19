// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    candidate::SelectionEligibility,
    definition::SelectionPickRule,
    field::{SelectionField, SelectionFieldStatus, SelectionText},
    model::{
        SelectionCandidateKind, SelectionCandidateReference, SelectionCatalogBinding,
        SelectionEvidence, SelectionKind, SelectionParentOperation, SelectionReference,
        SelectionSemanticReference, SelectionVisibility, SelectionVisibilityScope,
        SelectorGenerationReference,
    },
    progress::SelectionProgressInput,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use selection-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl SelectionContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use candidate-list continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionCandidateContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl SelectionCandidateContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded selection-definition-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct SelectionListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: SelectionVisibilityScope,
    /// Maximum definitions in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<SelectionContinuation>,
}

/// Typed summary returned by one bounded selection page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionDefinitionSummary {
    /// Exact static selection reference.
    pub reference: SelectionReference,
    /// Operation whose prompt owns this selector.
    pub parent: SelectionParentOperation,
    /// Exact-build selection family.
    pub kind: SelectionKind,
    /// Localized prompt text.
    pub prompt: SelectionText,
    /// Visibility of the definition.
    pub visibility: SelectionVisibility,
    /// Evidence label for the definition.
    pub evidence: SelectionEvidence,
    /// Observed selector generation.
    pub selector_generation: u64,
    /// Required and bounded pick counts.
    pub picks: SelectionPickRule,
    /// Number of visible candidates.
    pub candidate_count: usize,
    /// Availability of candidates after scope withholding.
    pub candidates_status: SelectionFieldStatus,
    /// Number of visible coverage records.
    pub coverage_count: usize,
    /// Availability of coverage records after scope withholding.
    pub coverage_status: SelectionFieldStatus,
}

/// Complete or partial selection-definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct SelectionDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: SelectionCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<SelectionDefinitionSummary>,
    /// Number of visible selection definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<SelectionContinuation>,
}

/// Bounded candidate-list request scoped to one selector generation.
#[derive(Debug, Eq, PartialEq)]
pub struct SelectionCandidateListQuery {
    /// Selector generation whose candidates are listed.
    pub selector: SelectorGenerationReference,
    /// Visibility scope.
    pub scope: SelectionVisibilityScope,
    /// Maximum candidates in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<SelectionCandidateContinuation>,
    /// Picks the caller already made, when the caller reports them.
    ///
    /// Supplying them makes the page reflect the current prompt: a candidate the caller already
    /// picked is no longer offered when this selector does not allow repeats, and the remaining
    /// count is computed for the picks made rather than for an untouched prompt.
    pub progress: Option<SelectionProgressInput>,
}

/// Typed summary returned by one bounded candidate page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCandidateSummary {
    /// Exact static candidate reference.
    pub reference: SelectionCandidateReference,
    /// Localized candidate label.
    pub label: SelectionText,
    /// Family of entity the candidate resolves to.
    pub kind: SelectionCandidateKind,
    /// Definition the candidate resolves to.
    pub definition: SelectionSemanticReference,
    /// Resolved eligibility and refusal reason.
    pub eligibility: SelectionEligibility,
    /// Availability of the resolved parameter after scope withholding.
    pub detail_status: SelectionFieldStatus,
    /// Number of documented prospective effects.
    pub effect_count: usize,
    /// Visibility of the candidate.
    pub visibility: SelectionVisibility,
}

/// Complete or partial candidate page, carrying the reconciled pick state.
#[derive(Debug, Eq, PartialEq)]
pub struct SelectionCandidatePage {
    /// Catalog witness for every entry.
    pub binding: SelectionCatalogBinding,
    /// Deterministically ordered candidate summaries.
    pub entries: Vec<SelectionCandidateSummary>,
    /// Number of candidates still selectable under the duplicate rule and scope.
    pub total: usize,
    /// Availability of candidates after scope withholding, independent of pagination exhaustion.
    pub candidates_status: SelectionFieldStatus,
    /// Picks the caller had made.
    pub picked: usize,
    /// Picks still available, or an explicit non-value when the maximum is unknown.
    pub remaining: SelectionField<u32>,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<SelectionCandidateContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct SelectionCursorState {
    pub(super) binding: SelectionCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: SelectionVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct SelectionCandidateCursorState {
    pub(super) binding: SelectionCatalogBinding,
    pub(super) selection_id: String,
    pub(super) selector_generation: u64,
    pub(super) scope: SelectionVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
    pub(super) selected: Vec<String>,
}
