// SPDX-License-Identifier: MIT

use super::super::model::{EventNumericValue, EventSemanticReference, EventText, EventVisibility};

/// One visible parameter retained without evaluating hidden state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventParameter {
    /// Stable parameter identity.
    pub parameter_id: String,
    /// Localized/source-defined label.
    pub label: EventText,
    /// Optional unit.
    pub unit: Option<String>,
    /// Fixed, formula-backed, or unavailable value.
    pub value: EventNumericValue,
}

/// Coarse option/event requirement category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventRequirementKind {
    /// A minimum or exact resource amount (gold, HP, potion slots).
    Resource,
    /// Character or loadout identity.
    Character,
    /// Progression or act-order requirement.
    Progression,
    /// Progression/unlock requirement.
    Unlock,
    /// Content-configuration flag.
    ContentFlag,
    /// Game mode identity.
    Mode,
    /// Owner rule reference.
    Rule,
    /// Owner-defined predicate.
    Custom(String),
    /// Source could not classify the predicate.
    Unknown,
}

/// One typed requirement for an event or one of its options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRequirement {
    /// Stable requirement identity.
    pub requirement_id: String,
    /// Predicate category.
    pub kind: EventRequirementKind,
    /// Localized/source-defined requirement label.
    pub label: EventText,
    /// Typed parameters used by the requirement.
    pub parameters: Vec<EventParameter>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static requirement.
    pub visibility: EventVisibility,
}
