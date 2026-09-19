// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    candidate::{SelectionCandidate, SelectionCandidateInput},
    field::{SelectionField, SelectionFieldStatus, SelectionText},
    model::{
        SelectionCandidateKind, SelectionCatalogBinding, SelectionConfirmation,
        SelectionDuplicateRule, SelectionEvidence, SelectionFamilyCoverage, SelectionKind,
        SelectionNextDomain, SelectionOrderingRule, SelectionParentOperation, SelectionReference,
        SelectionSemanticReference, SelectionVisibility, SelectionVisibilityScope,
    },
    reader::SelectionReader,
};

/// Required and bounded pick counts for one selector.
///
/// A bound the source does not state stays an explicit non-value instead of being read as zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionPickRule {
    /// Whether the caller must pick before the prompt can complete.
    pub required: bool,
    /// Minimum picks the selector requires, or an explicit non-value.
    pub minimum: SelectionField<u32>,
    /// Maximum picks the selector accepts, or an explicit non-value.
    pub maximum: SelectionField<u32>,
}

impl SelectionPickRule {
    /// Returns the minimum picks the selector requires, when the source states one.
    #[must_use]
    pub fn minimum_picks(&self) -> Option<u32> {
        self.minimum.value().copied()
    }

    /// Returns the maximum picks the selector accepts, when the source states one.
    #[must_use]
    pub fn maximum_picks(&self) -> Option<u32> {
        self.maximum.value().copied()
    }
}

/// Support state recorded for one audited selector or candidate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionCoverageState {
    /// The target has a typed record in this slice.
    Handled,
    /// The target is known but this producer has no typed record for it.
    Unsupported,
    /// The target is known but currently unavailable from this source.
    Unavailable,
}

/// Named coverage record for one host-reported selector or candidate.
///
/// Every selector and candidate the host reports is either described by a typed record or named
/// here, so a new prompt or entry is audited explicitly instead of being silently omitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCoverageRecord {
    /// Host-reported identity this record covers.
    pub target_id: String,
    /// Support state for the covered target.
    pub state: SelectionCoverageState,
    /// Localized explanation for an unsupported or unavailable target.
    pub reason: SelectionText,
}

/// Owner-supplied selection definition used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionDefinitionInput {
    /// Namespaced selection identity.
    pub selection_id: String,
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
    /// Generation of the selector this definition describes.
    pub selector_generation: u64,
    /// Required and bounded pick counts.
    pub picks: SelectionPickRule,
    /// Ordering rule the presented candidate list follows.
    pub ordering: SelectionOrderingRule,
    /// Whether one candidate identity may be picked more than once.
    pub duplicate: SelectionDuplicateRule,
    /// How the prompt completes once its picks are satisfied.
    pub confirmation: SelectionConfirmation,
    /// How the prompt may be abandoned, or an explicit non-value.
    pub cancellation: SelectionField<super::model::SelectionCancellation>,
    /// Declared steps in a multi-step sequence, or an explicit non-value.
    pub steps: SelectionField<u32>,
    /// Domain the selector presents once the current picks are complete, or an explicit non-value.
    pub next: SelectionField<SelectionNextDomain>,
    /// Candidate identities the host reports for this selector.
    ///
    /// Every reported identity must be either described by a typed candidate or named by a coverage
    /// record, so a newly audited entry cannot be silently omitted.
    pub observed_candidates: Vec<String>,
    /// Typed candidates for this selection.
    pub candidates: Vec<SelectionCandidateInput>,
    /// Named coverage records for reported selectors and candidates without a typed record.
    pub coverage: Vec<SelectionCoverageRecord>,
    /// Stable host action vocabulary token, or an explicit non-value.
    pub action_kind: SelectionField<String>,
    /// Transient legal action identity.
    ///
    /// A static slice must not carry one: the host assigns action identities per rendered prompt, so
    /// an available value here is rejected rather than copied into static reference data.
    pub action: SelectionField<super::model::SelectionActionReference>,
    /// Definitions the definition refers to.
    pub references: Vec<SelectionSemanticReference>,
}

/// Immutable selection definition with bounded, deterministic candidate collections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionDefinition {
    /// Exact static definition reference.
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
    /// Generation of the selector this definition describes.
    pub selector_generation: u64,
    /// Required and bounded pick counts.
    pub picks: SelectionPickRule,
    /// Ordering rule the presented candidate list follows.
    pub ordering: SelectionOrderingRule,
    /// Whether one candidate identity may be picked more than once.
    pub duplicate: SelectionDuplicateRule,
    /// How the prompt completes once its picks are satisfied.
    pub confirmation: SelectionConfirmation,
    /// How the prompt may be abandoned, or an explicit non-value.
    pub cancellation: SelectionField<super::model::SelectionCancellation>,
    /// Declared steps in a multi-step sequence, or an explicit non-value.
    pub steps: SelectionField<u32>,
    /// Domain the selector presents once the current picks are complete, or an explicit non-value.
    pub next: SelectionField<SelectionNextDomain>,
    /// Number of candidate identities the host reported for this selector.
    pub observed_candidate_count: usize,
    /// Typed candidates keyed by candidate identity.
    pub candidates: BTreeMap<String, SelectionCandidate>,
    /// Named coverage records for audited targets without a typed record.
    pub coverage: Vec<SelectionCoverageRecord>,
    /// Availability of the candidate list after scope withholding.
    pub candidates_status: SelectionFieldStatus,
    /// Availability of the coverage list after scope withholding.
    pub coverage_status: SelectionFieldStatus,
    /// Stable host action vocabulary token, or an explicit non-value.
    pub action_kind: SelectionField<String>,
    /// Definitions the definition refers to.
    pub references: Vec<SelectionSemanticReference>,
}

impl SelectionDefinition {
    /// Returns the candidate family this selector presents most often, when one is described.
    ///
    /// A selector with no described candidate reports no family rather than an assumed one.
    #[must_use]
    pub fn candidate_kind(&self) -> Option<&SelectionCandidateKind> {
        self.candidates
            .values()
            .map(|candidate| &candidate.kind)
            .next()
    }

    /// Returns whether this selector presents a single step.
    #[must_use]
    pub fn is_single_step(&self) -> bool {
        !self.next.is_available() && self.steps.value().copied().unwrap_or(1) <= 1
    }
}

/// Immutable, read-only selection reference catalog.
///
/// The catalog owns no setter, no click, and no confirmation: it is a bounded copy of static
/// reference data fenced by one content manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCatalog {
    /// Static identity shared by every record in this catalog.
    pub binding: SelectionCatalogBinding,
    /// Explicit support state for the selection family.
    pub family: SelectionFamilyCoverage,
    definitions: BTreeMap<String, SelectionDefinition>,
}

impl SelectionCatalog {
    /// Binds validated definitions to one catalog identity.
    pub(super) fn from_parts(
        binding: SelectionCatalogBinding,
        family: SelectionFamilyCoverage,
        definitions: BTreeMap<String, SelectionDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the locale every localized value was copied for.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the number of selection definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether the catalog carries no selection definition.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns one selection definition by identity.
    #[must_use]
    pub fn definition(&self, selection_id: &str) -> Option<&SelectionDefinition> {
        self.definitions.get(selection_id)
    }

    /// Returns one candidate by its selection and candidate identity.
    #[must_use]
    pub fn candidate(&self, selection_id: &str, candidate_id: &str) -> Option<&SelectionCandidate> {
        self.definitions
            .get(selection_id)
            .and_then(|definition| definition.candidates.get(candidate_id))
    }

    /// Returns every bound definition for same-catalog resolution.
    #[must_use]
    pub(super) fn definitions(&self) -> &BTreeMap<String, SelectionDefinition> {
        &self.definitions
    }

    /// Returns an independent reader over this catalog under one visibility scope.
    #[must_use]
    pub fn reader(&self, scope: SelectionVisibilityScope) -> SelectionReader<'_> {
        SelectionReader::new(self, scope)
    }
}
