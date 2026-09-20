// SPDX-License-Identifier: MIT

use super::{
    field::{ActionField, ActionText},
    kind::{ActionCostKind, ActionRefusalReason, ActionRestrictionKind},
    model::ActionSemanticReference,
};

/// One bounded resource an action draws on, with the exact required and observed amounts.
///
/// The values stay as the source states them: a contributor never folds several costs into an
/// invented total, and a contributor whose required or available amount is unobserved must not
/// claim affordability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCostContributor {
    /// Stable cost identity within its action.
    pub cost_id: String,
    /// Localized cost description.
    pub label: ActionText,
    /// Bounded resource family the cost draws on.
    pub kind: ActionCostKind,
    /// Localized unit the required and available amounts are stated in, or an explicit non-value.
    pub unit: ActionText,
    /// Definition of the resource the cost draws on, or an explicit non-value.
    pub resource: ActionField<ActionSemanticReference>,
    /// Amount the action requires, or an explicit non-value.
    pub required: ActionField<i64>,
    /// Amount the caller currently has, or an explicit non-value.
    pub available: ActionField<i64>,
    /// Whether this contributor currently permits the action.
    ///
    /// A contributor whose required or available amount was not observed reports `false` rather
    /// than an assumed affordability.
    pub affordable: bool,
    /// Definitions this contributor refers to.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionCostContributor {
    /// Returns whether both amounts needed to settle affordability were observed.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.required.is_available() && self.available.is_available()
    }

    /// Returns the shortfall this contributor currently leaves, when both amounts were observed.
    #[must_use]
    pub fn shortfall(&self) -> Option<i64> {
        let required = *self.required.value()?;
        let available = *self.available.value()?;
        Some(required.saturating_sub(available).max(0))
    }
}

/// One restriction an action imposes on the target or state it accepts.
///
/// A restriction explains what an agent must change before the action becomes available, so an
/// unsatisfied restriction names the refusal it produces instead of only reporting that the action
/// is disabled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTargetRestriction {
    /// Stable restriction identity within its action.
    pub restriction_id: String,
    /// Localized restriction description.
    pub label: ActionText,
    /// Coarse class of the restriction.
    pub kind: ActionRestrictionKind,
    /// Target identity the restriction applies to, or an explicit non-value for the whole action.
    pub target: ActionField<String>,
    /// Whether the restriction currently admits the action.
    pub satisfied: bool,
    /// Refusal this restriction produces when it is unsatisfied, or an explicit non-value.
    pub reason: ActionField<ActionRefusalReason>,
    /// Definitions this restriction refers to.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionTargetRestriction {
    /// Returns whether this restriction currently admits the action.
    #[must_use]
    pub const fn is_satisfied(&self) -> bool {
        self.satisfied
    }
}
