// SPDX-License-Identifier: MIT

use super::super::model::{
    EventEvidence, EventField, EventNumericValue, EventSemanticReference, EventText,
    EventVisibility,
};

/// Coarse structured-cost category.
///
/// Costs distinguish HP loss, max-HP change, gold, and item removal from owner-defined
/// resources, and never collapse an unknown resource into a named one. Every named category is a
/// non-negative magnitude: a cost states how much is given up, never a signed delta.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventCostKind {
    /// Lose current HP.
    HpLoss,
    /// Change maximum HP.
    MaxHpChange,
    /// Spend or lose gold.
    Gold,
    /// Remove a card.
    CardRemoval,
    /// Remove or consume a potion.
    PotionRemoval,
    /// Remove or lose a relic.
    RelicRemoval,
    /// Owner-defined item or resource removal.
    ItemRemoval,
    /// Owner rule reference.
    Rule,
    /// Owner-defined cost.
    Custom(String),
    /// A cost is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the cost.
    Unknown,
}

/// One structured cost for an option, with a rule reference and evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCost {
    /// Stable cost identity scoped by the option.
    pub cost_id: String,
    /// Cost category.
    pub kind: EventCostKind,
    /// Localized/source-defined cost label.
    pub label: EventText,
    /// Fixed, formula-backed, or unavailable amount.
    pub amount: EventNumericValue,
    /// Optional resource identity the amount applies to.
    pub resource: EventField<String>,
    /// Owner rule reference, preserving unavailable/empty distinctions.
    pub rule_reference: EventField<String>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Evidence label for this cost.
    pub evidence: EventEvidence,
    /// Visibility of the static cost.
    pub visibility: EventVisibility,
}
