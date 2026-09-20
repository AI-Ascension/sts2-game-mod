// SPDX-License-Identifier: MIT

use super::{
    eligibility::ActionEligibility,
    field::{ActionField, ActionText},
    model::{ActionEvidence, ActionSemanticReference, ActionTargetReference, ActionVisibility},
};

/// Family of entity one observed target resolves to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionTargetKind {
    /// An enemy instance.
    Enemy,
    /// A co-op player instance.
    Player,
    /// A card instance.
    Card,
    /// A potion instance.
    Potion,
    /// A relic instance.
    Relic,
    /// A shop item or service.
    ShopItem,
    /// A bounded destination such as a pile or slot.
    Destination,
    /// Owner-defined target family.
    Custom(String),
    /// Source could not classify the target family.
    Unknown,
}

/// Owner-supplied observed target used to construct one immutable catalog.
///
/// A target is what the host presents for one action, not a candidate the caller invented: the
/// source reports which identities the action currently accepts, and every identity it reports is
/// either described here or named by a coverage record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTargetInput {
    /// Stable target identity within its action.
    pub target_id: String,
    /// Localized target label.
    pub label: ActionText,
    /// Family of entity the target resolves to.
    pub kind: ActionTargetKind,
    /// Definition the target resolves to.
    pub definition: ActionSemanticReference,
    /// Resolved target availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Resolved public parameter, or an explicit non-value.
    pub detail: ActionField<String>,
    /// Visibility of the target.
    pub visibility: ActionVisibility,
    /// Evidence label for the target.
    pub evidence: ActionEvidence,
    /// Definitions the target refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// Immutable observed target bound to one exact legal-action definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTarget {
    /// Exact static target reference.
    pub reference: ActionTargetReference,
    /// Localized target label.
    pub label: ActionText,
    /// Family of entity the target resolves to.
    pub kind: ActionTargetKind,
    /// Definition the target resolves to.
    pub definition: ActionSemanticReference,
    /// Resolved target availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Resolved public parameter, or an explicit non-value.
    pub detail: ActionField<String>,
    /// Visibility of the target.
    pub visibility: ActionVisibility,
    /// Evidence label for the target.
    pub evidence: ActionEvidence,
    /// Definitions the target refers to.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionTarget {
    /// Binds a validated target input to one exact reference.
    pub(super) fn from_input(reference: ActionTargetReference, input: ActionTargetInput) -> Self {
        Self {
            reference,
            label: input.label,
            kind: input.kind,
            definition: input.definition,
            eligibility: input.eligibility,
            detail: input.detail,
            visibility: input.visibility,
            evidence: input.evidence,
            references: input.references,
        }
    }

    /// Returns whether this target may be acted on now.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.eligibility.is_available()
    }
}
