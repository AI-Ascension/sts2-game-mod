// SPDX-License-Identifier: MIT

use super::{SelectionKind, SelectionReferenceKind, SelectionUnavailableReason};

/// Failure before an owned selection snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionSourceError {
    /// No supported selector registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for SelectionSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SelectionSourceError {}

/// Sanitized failures while producing or reading source-only selection reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The manifest does not inventory a selection family.
    MissingFamily,
    /// The source advertises a family it cannot project.
    UnsupportedFamily,
    /// The source family is temporarily unavailable.
    UnavailableFamily,
    /// The family identity in the source snapshot is wrong.
    FamilyIdentityMismatch,
    /// Source and manifest family counts disagree.
    FamilyCountMismatch,
    /// A source definition is absent from the manifest.
    UnknownDefinition(String),
    /// A manifest definition is absent from the source snapshot.
    MissingDefinition(String),
    /// A source definition repeats an identity.
    DuplicateDefinition(String),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A definition exceeds the aggregate byte bound.
    DefinitionTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A typed reference is absent from the manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// A selector claims a family that contradicts the candidates it presents.
    ///
    /// This is what keeps a supported selector from staying hard-coded unknown: an unknown selector
    /// whose described candidate resolves to a known definition family rejects the snapshot.
    KindDisagreesWithDefinition {
        /// Owning selection definition identity.
        selection_id: String,
        /// Family the source reported.
        reported: SelectionKind,
        /// Family the described candidate resolves to.
        family: String,
    },
    /// A selection definition repeats a candidate identity.
    DuplicateCandidate {
        /// Owning selection definition identity.
        selection_id: String,
        /// Repeated candidate identity.
        candidate_id: String,
    },
    /// A host-reported candidate has neither a typed record nor a named coverage record.
    ///
    /// This is what stops a newly audited candidate from being silently omitted from the reference.
    UncoveredCandidate {
        /// Owning selection definition identity.
        selection_id: String,
        /// Candidate the source reported but did not describe or cover.
        candidate_id: String,
    },
    /// A selector this producer cannot type carries no named coverage record.
    UncoveredSelection {
        /// Selection definition the source reported but did not cover.
        selection_id: String,
    },
    /// A coverage record names neither the selector nor one of its reported candidates.
    InvalidCoverageRecord {
        /// Owning selection definition identity.
        selection_id: String,
        /// Coverage target that names nothing observed.
        target_id: String,
    },
    /// A selection declares an impossible required/minimum/maximum pick rule.
    InvalidPickRule {
        /// Owning selection definition identity.
        selection_id: String,
    },
    /// Confirmation and cancellation semantics contradict each other.
    InvalidConfirmation {
        /// Owning selection definition identity.
        selection_id: String,
    },
    /// A multi-step declaration contradicts the next domain it names.
    InvalidStepDeclaration {
        /// Owning selection definition identity.
        selection_id: String,
    },
    /// Candidate eligibility state and reason contradict each other.
    InvalidEligibility {
        /// Owning selection definition identity.
        selection_id: String,
        /// Candidate whose eligibility is rejected.
        candidate_id: String,
    },
    /// A prospective effect claims a change that changes nothing.
    InvalidProspectiveEffect {
        /// Owning selection definition identity.
        selection_id: String,
        /// Candidate whose effect is rejected.
        candidate_id: String,
        /// Effect whose declaration is rejected.
        effect_id: String,
    },
    /// A static slice carries a resolved transient action instead of an explicit non-value.
    InvalidSelectionAction {
        /// Owning selection definition identity.
        selection_id: String,
    },
    /// A record more visible than its target would disclose a restricted definition.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning selection definition identity.
        selection_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: SelectionReferenceKind,
    },
    /// A typed reference inside a selection names an identity absent from the same snapshot.
    DanglingReference {
        /// Owning selection definition identity.
        selection_id: String,
        /// Reference family.
        reference_kind: SelectionReferenceKind,
        /// Missing identity.
        id: String,
    },
    /// A selector reference was produced by another selector generation.
    StaleSelectorReference {
        /// Selection whose selector was referenced.
        selection_id: String,
        /// Generation the reference was bound to.
        referenced: u64,
        /// Generation the catalog currently reports.
        current: u64,
    },
    /// A caller picked one candidate identity twice where repeats are not allowed.
    DuplicateChoice {
        /// Owning selection definition identity.
        selection_id: String,
        /// Candidate identity picked more than once.
        candidate_id: String,
    },
    /// A caller picked a candidate this selection does not present.
    UnknownCandidate {
        /// Owning selection definition identity.
        selection_id: String,
        /// Candidate identity absent from the presented domain.
        candidate_id: String,
    },
    /// A caller asked to confirm before the required picks were made.
    ///
    /// The current state is refused rather than reported as a legal confirmation.
    PrematureConfirmation {
        /// Owning selection definition identity.
        selection_id: String,
        /// Minimum picks the selector still requires.
        required: u32,
        /// Picks the caller had made.
        observed: u32,
    },
    /// A caller asked to advance a multi-step selection before its picks were complete.
    IncompleteSelection {
        /// Owning selection definition identity.
        selection_id: String,
        /// Minimum picks the selector still requires.
        required: u32,
        /// Picks the caller had made.
        observed: u32,
    },
    /// A caller picked more candidates than the selector accepts.
    ExcessPicks {
        /// Owning selection definition identity.
        selection_id: String,
        /// Maximum picks the selector accepts.
        maximum: u32,
        /// Picks the caller had made.
        observed: u32,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(SelectionUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SelectionError {}

pub(super) fn map_source_error(error: SelectionSourceError) -> SelectionError {
    match error {
        SelectionSourceError::NoActiveSource => SelectionError::NoActiveSource,
        SelectionSourceError::AccessDenied => SelectionError::SourceAccessDenied,
        SelectionSourceError::Malformed => SelectionError::MalformedSource,
    }
}
