// SPDX-License-Identifier: MIT

use super::{
    field::{SelectionField, SelectionText},
    model::{
        SelectionCandidateKind, SelectionCandidateReference, SelectionEvidence,
        SelectionSemanticReference, SelectionVisibility,
    },
};

/// Resolved eligibility of one candidate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionEligibilityState {
    /// The candidate may be picked now.
    Eligible,
    /// The candidate exists but is currently refused for a stated reason.
    Ineligible,
    /// The host could not resolve the candidate's eligibility.
    Unknown,
}

/// Reason one candidate is refused.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionBlockReason {
    /// A requirement is unmet.
    RequirementUnsatisfied,
    /// A limit is exhausted.
    LimitReached,
    /// A cost cannot currently be paid.
    CostUnaffordable,
    /// The candidate does not apply to the current mode.
    WrongMode,
    /// The candidate was already picked and this selector does not allow repeats.
    AlreadySelected,
    /// The reason exists but must not be revealed.
    Withheld,
    /// No supported extractor describes the refusal.
    Unsupported,
    /// Owner-defined refusal reason.
    Custom(String),
    /// Source could not classify the refusal.
    Unknown,
}

/// Eligibility of one candidate with an explicit refusal reason.
///
/// A refused candidate must carry a reason and an eligible candidate must not, so a disabled entry
/// is always explained and an enabled one never carries a stale refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionEligibility {
    /// Resolved eligibility state.
    pub state: SelectionEligibilityState,
    /// Explicit refusal reason, present only for a refused candidate.
    pub reason: SelectionField<SelectionBlockReason>,
    /// Definitions the eligibility depends on.
    pub references: Vec<SelectionSemanticReference>,
}

/// Kind of prospective effect one candidate would produce.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionProspectiveEffectKind {
    /// Damage would be dealt.
    DamageDealt,
    /// Block would be gained.
    BlockGained,
    /// Health would be restored.
    HealthRestored,
    /// A card would be upgraded.
    CardUpgraded,
    /// A card would be removed.
    CardRemoved,
    /// A card would be added.
    CardAdded,
    /// A relic would be granted.
    RelicGranted,
    /// A potion would be granted.
    PotionGranted,
    /// A player would be targeted.
    PlayerTargeted,
    /// Owner-defined effect kind.
    Custom(String),
    /// Source could not classify the effect.
    Unknown,
}

/// One documented prospective effect of picking a candidate.
///
/// The values stay as the source states them; a documented effect never folds several contributors
/// into an invented total, and an effect that would change nothing is refused rather than published
/// as a change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionProspectiveEffect {
    /// Stable effect identity within its candidate.
    pub effect_id: String,
    /// Localized effect text.
    pub label: SelectionText,
    /// Kind of prospective effect.
    pub kind: SelectionProspectiveEffectKind,
    /// Definition the effect targets, or an explicit non-value.
    pub target: SelectionField<SelectionSemanticReference>,
    /// Documented value before the pick, or an explicit non-value.
    pub before: SelectionField<String>,
    /// Documented value after the pick, or an explicit non-value.
    pub after: SelectionField<String>,
    /// Definitions this effect refers to.
    pub references: Vec<SelectionSemanticReference>,
}

/// Owner-supplied candidate used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCandidateInput {
    /// Stable candidate identity within its selection.
    pub candidate_id: String,
    /// Localized candidate label.
    pub label: SelectionText,
    /// Family of entity the candidate resolves to.
    pub kind: SelectionCandidateKind,
    /// Definition the candidate resolves to.
    pub definition: SelectionSemanticReference,
    /// Resolved eligibility and refusal reason.
    pub eligibility: SelectionEligibility,
    /// Resolved public parameter, or an explicit non-value.
    pub detail: SelectionField<String>,
    /// Documented prospective effects of picking this candidate.
    pub prospective: Vec<SelectionProspectiveEffect>,
    /// Visibility of the candidate.
    pub visibility: SelectionVisibility,
    /// Evidence label for the candidate.
    pub evidence: SelectionEvidence,
    /// Definitions the candidate refers to.
    pub references: Vec<SelectionSemanticReference>,
}

/// Immutable candidate with a bounded, deterministic effect list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCandidate {
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
    /// Resolved public parameter, or an explicit non-value.
    pub detail: SelectionField<String>,
    /// Documented prospective effects of picking this candidate.
    pub prospective: Vec<SelectionProspectiveEffect>,
    /// Visibility of the candidate.
    pub visibility: SelectionVisibility,
    /// Evidence label for the candidate.
    pub evidence: SelectionEvidence,
    /// Definitions the candidate refers to.
    pub references: Vec<SelectionSemanticReference>,
}

impl SelectionCandidate {
    /// Binds a validated candidate input to one exact reference.
    pub(super) fn from_input(
        reference: SelectionCandidateReference,
        input: SelectionCandidateInput,
    ) -> Self {
        Self {
            reference,
            label: input.label,
            kind: input.kind,
            definition: input.definition,
            eligibility: input.eligibility,
            detail: input.detail,
            prospective: input.prospective,
            visibility: input.visibility,
            evidence: input.evidence,
            references: input.references,
        }
    }

    /// Returns whether this candidate may be picked now.
    #[must_use]
    pub fn is_eligible(&self) -> bool {
        self.eligibility.state == SelectionEligibilityState::Eligible
    }
}
