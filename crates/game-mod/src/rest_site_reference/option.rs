// SPDX-License-Identifier: MIT

use super::{
    effect::{RestEffect, RestProspectiveComparison},
    field::{RestField, RestFieldStatus, RestText},
    model::{
        RestActionReference, RestEvidence, RestOptionKind, RestOptionReference,
        RestSemanticReference, RestVisibility,
    },
    requirement::{RestCost, RestLimit, RestRequirement, RestSelectionRequirement},
};

/// Resolved availability of one rest option.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestOptionState {
    /// The option can be selected now.
    Available,
    /// The option exists but is currently refused for a stated reason.
    Disabled,
    /// The host does not offer the option for this option set.
    NotOffered,
    /// The host supports the option but could not project it.
    Unavailable,
    /// Source could not classify the option state.
    Unknown,
}

/// Reason a rest option is refused.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestBlockReason {
    /// A requirement is unmet.
    RequirementUnsatisfied,
    /// A limit is exhausted.
    LimitReached,
    /// A cost cannot currently be paid.
    CostUnaffordable,
    /// The option does not apply to the current mode.
    WrongMode,
    /// The reason exists but must not be revealed.
    Withheld,
    /// No supported extractor describes the refusal.
    Unsupported,
    /// Owner-defined refusal reason.
    Custom(String),
    /// Source could not classify the refusal.
    Unknown,
}

/// Availability of one rest option with an explicit refusal reason.
///
/// A refused option must carry a reason and an available option must not, so a disabled button is
/// always explained and an enabled one never carries a stale refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestOptionAvailability {
    /// Resolved availability state.
    pub state: RestOptionState,
    /// Explicit refusal reason, present only for a refused option.
    pub reason: RestField<RestBlockReason>,
    /// Definitions the availability depends on.
    pub references: Vec<RestSemanticReference>,
}

/// Support state recorded for one audited host option.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestCoverageState {
    /// The option has a typed record in this slice.
    Handled,
    /// The option is known but this producer has no typed record for it.
    Unsupported,
    /// The option is known but currently unavailable from this source.
    Unavailable,
}

/// Named coverage record for one host-reported rest option.
///
/// Every option the host reports is either described by a typed record or named here, so a new
/// button is audited explicitly instead of being silently omitted from the reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestCoverageRecord {
    /// Host-reported option identity this record covers.
    pub option_id: String,
    /// Support state for the covered option.
    pub state: RestCoverageState,
    /// Localized explanation for an unsupported or unavailable option.
    pub reason: RestText,
}

/// Owner-supplied rest option used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestOptionInput {
    /// Stable option identity within its rest site.
    pub option_id: String,
    /// Localized option label.
    pub label: RestText,
    /// Localized option description.
    pub description: RestText,
    /// Reported type of the option.
    pub kind: RestOptionKind,
    /// Definition the option resolves to.
    pub definition: RestSemanticReference,
    /// Resolved availability and refusal reason.
    pub availability: RestOptionAvailability,
    /// Requirements the option imposes.
    pub requirements: Vec<RestRequirement>,
    /// Costs the option charges.
    pub costs: Vec<RestCost>,
    /// Limits that bound the option.
    pub limits: Vec<RestLimit>,
    /// Effects the option would produce.
    pub effects: Vec<RestEffect>,
    /// Selection the option requires.
    pub selection: RestSelectionRequirement,
    /// Documented prospective comparison, when the source supplies one.
    pub comparison: Option<RestProspectiveComparison>,
    /// Stable host action vocabulary token, or an explicit non-value.
    pub action_kind: RestField<String>,
    /// Transient host action identity.
    ///
    /// A static slice must not carry one: the host assigns action identities per rendered menu, so
    /// an available value here is rejected rather than copied into static reference data.
    pub action: RestField<RestActionReference>,
    /// Visibility of the option.
    pub visibility: RestVisibility,
    /// Evidence label for the option.
    pub evidence: RestEvidence,
    /// Definitions the option refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Immutable rest option with bounded, deterministic sub-collections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestOption {
    /// Exact static option reference.
    pub reference: RestOptionReference,
    /// Localized option label.
    pub label: RestText,
    /// Localized option description.
    pub description: RestText,
    /// Reported type of the option.
    pub kind: RestOptionKind,
    /// Definition the option resolves to.
    pub definition: RestSemanticReference,
    /// Resolved availability and refusal reason.
    pub availability: RestOptionAvailability,
    /// Requirements the option imposes.
    pub requirements: Vec<RestRequirement>,
    /// Costs the option charges.
    pub costs: Vec<RestCost>,
    /// Limits that bound the option.
    pub limits: Vec<RestLimit>,
    /// Effects the option would produce.
    pub effects: Vec<RestEffect>,
    /// Selection the option requires.
    pub selection: RestSelectionRequirement,
    /// Documented prospective comparison, when the source supplies one.
    pub comparison: Option<RestProspectiveComparison>,
    /// Stable host action vocabulary token, or an explicit non-value.
    pub action_kind: RestField<String>,
    /// Visibility of the option.
    pub visibility: RestVisibility,
    /// Evidence label for the option.
    pub evidence: RestEvidence,
    /// Definitions the option refers to.
    pub references: Vec<RestSemanticReference>,
    /// Availability of the requirement list after scope withholding.
    pub requirements_status: RestFieldStatus,
    /// Availability of the effect list after scope withholding.
    pub effects_status: RestFieldStatus,
}

impl RestOption {
    /// Binds one validated input to its static option reference.
    pub(super) fn from_input(reference: RestOptionReference, input: RestOptionInput) -> Self {
        Self {
            reference,
            label: input.label,
            description: input.description,
            kind: input.kind,
            definition: input.definition,
            availability: input.availability,
            requirements: input.requirements,
            costs: input.costs,
            limits: input.limits,
            effects: input.effects,
            selection: input.selection,
            comparison: input.comparison,
            action_kind: input.action_kind,
            visibility: input.visibility,
            evidence: input.evidence,
            references: input.references,
            requirements_status: RestFieldStatus::Available,
            effects_status: RestFieldStatus::Available,
        }
    }
}
