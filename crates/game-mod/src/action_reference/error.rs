// SPDX-License-Identifier: MIT

use super::{
    field::ActionUnavailableReason,
    kind::{ActionKind, ActionPreviewClass, ActionReferenceKind, ActionRefusalReason},
};

/// Sanitized failure reported by an owner-local action-preview source.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionSourceError {
    /// No supported legal-action registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned data this producer cannot accept.
    Malformed,
}

/// Failure that rejects an action-preview snapshot, explanation, or preview request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionError {
    /// No supported legal-action registry is active.
    NoActiveSource,
    /// The owner source denied the read.
    SourceAccessDenied,
    /// The owner source returned malformed data.
    MalformedSource,
    /// The source snapshot was produced for another content manifest.
    ManifestMismatch,
    /// The source snapshot was produced for another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The manifest does not inventory the legal-action family.
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
    /// A legal action claims a family that contradicts the target it accepts.
    ///
    /// This is what keeps a supported action from staying hard-coded unknown: an unknown action
    /// whose described target resolves to a known definition family rejects the snapshot.
    KindDisagreesWithDefinition {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Family the source reported.
        reported: ActionKind,
        /// Family the described target resolves to.
        family: String,
    },
    /// A legal-action definition repeats an observed target identity.
    DuplicateTarget {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Repeated target identity.
        target_id: String,
    },
    /// A host-reported target has neither a typed record nor a named coverage record.
    ///
    /// This is what stops a newly audited target from being silently omitted from the reference.
    UncoveredTarget {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Target the source reported but did not describe or cover.
        target_id: String,
    },
    /// A legal action this producer cannot type carries no named coverage record.
    UncoveredAction {
        /// Legal-action definition the source reported but did not cover.
        action_id: String,
    },
    /// A coverage record names neither the action nor one of its reported targets.
    InvalidCoverageRecord {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Coverage target that names nothing observed.
        target_id: String,
    },
    /// Action availability state and refusal reason contradict each other.
    InvalidEligibility {
        /// Owning legal-action definition identity.
        action_id: String,
    },
    /// Target eligibility state and reason contradict each other.
    InvalidTargetEligibility {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Target whose eligibility is rejected.
        target_id: String,
    },
    /// A refusal states no supported cause for why the action is unavailable.
    ///
    /// A refusal an agent cannot act on is the defect this slice exists to remove, so an
    /// unavailable action must name an unaffordable cost contributor or an unsatisfied matching
    /// restriction rather than only a reason token.
    InvalidRefusalSupport {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Refusal the definition states.
        reason: ActionRefusalReason,
    },
    /// A cost contributor contradicts its own required/available/affordable declaration.
    InvalidCostContributor {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Cost contributor whose declaration is rejected.
        cost_id: String,
    },
    /// A target restriction contradicts its own kind and satisfaction declaration.
    InvalidRestriction {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Restriction whose declaration is rejected.
        restriction_id: String,
    },
    /// A preview declaration is not usable as written.
    InvalidPreview {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Preview whose declaration is rejected.
        preview_id: String,
    },
    /// A preview states a consequence that would change nothing.
    InvalidPreviewChange {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Preview carrying the rejected change.
        preview_id: String,
        /// Change whose declaration is rejected.
        change_id: String,
    },
    /// A preview claims certainty while naming an omission or assumption.
    ///
    /// Random and unsupported chains must report uncertainty explicitly, so a preview carrying a
    /// named omission or assumption may not be classified `DeterministicExact`.
    UncertainPreview {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Preview whose classification is rejected.
        preview_id: String,
        /// Classification the source claimed.
        class: ActionPreviewClass,
    },
    /// A preview claims a classification its definition does not declare as supported.
    UndeclaredPreviewClass {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Preview whose classification is rejected.
        preview_id: String,
        /// Undeclared classification.
        class: ActionPreviewClass,
    },
    /// A preview was approximated by applying and undoing a real action.
    ///
    /// The read is refused by name rather than accepted as a preview, because the approximation
    /// mutates state, queues, history, and the RNG.
    SimulatedPreview {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Preview whose provenance is rejected.
        preview_id: String,
    },
    /// A preview names a target this legal action does not present.
    DanglingTarget {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Target identity absent from the presented domain.
        target_id: String,
    },
    /// A record more visible than its target would disclose a restricted definition.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: ActionReferenceKind,
    },
    /// A typed reference inside an action names an identity absent from the same snapshot.
    DanglingReference {
        /// Owning legal-action definition identity.
        action_id: String,
        /// Reference family.
        reference_kind: ActionReferenceKind,
        /// Missing identity.
        id: String,
    },
    /// An action reference was produced by another legal-action generation.
    StaleActionReference {
        /// Action whose frame was referenced.
        action_id: String,
        /// Generation the reference was bound to.
        referenced: u64,
        /// Generation the catalog currently reports.
        current: u64,
    },
    /// A static slice carries a resolved live action instance instead of an explicit non-value.
    InvalidInstanceReference {
        /// Owning legal-action definition identity.
        action_id: String,
    },
    /// A live query omitted the instance/run/epoch/snapshot fence it must bind to.
    MissingLiveFence {
        /// Legal-action definition identity the query names.
        action_id: String,
    },
    /// A static candidate query carried a live fence it must not bind to.
    UnexpectedLiveFence,
    /// No supported preview describes the requested action and target.
    ///
    /// The request is answered with an explicitly unavailable preview rather than an invented
    /// consequence, so this error is reserved for a subject the catalog cannot resolve at all.
    NoSupportedPreview {
        /// Legal-action definition identity the query names.
        action_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(ActionUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActionError {}

pub(super) fn map_source_error(error: ActionSourceError) -> ActionError {
    match error {
        ActionSourceError::NoActiveSource => ActionError::NoActiveSource,
        ActionSourceError::AccessDenied => ActionError::SourceAccessDenied,
        ActionSourceError::Malformed => ActionError::MalformedSource,
    }
}
